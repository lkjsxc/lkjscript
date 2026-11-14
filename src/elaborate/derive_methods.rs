//! Executable builtin trait operations use ordinary, checked IR functions.
//!
//! Headers are interned before their bodies enter the shared compilation queue.
//! Recursive nominal layouts therefore reuse an existing function identity;
//! neither helper discovery nor field type traversal recurses on the host stack.
use super::generic_check::BuiltinMethod;
use super::{Builder, Compiler, PendingFunction, Result, a, err, ir, is_copy};
use crate::generics::TraitIdentity;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(super) struct DerivedInstance {
    receiver: usize,
    method: BuiltinMethod,
    span: a::Span,
}

impl Compiler<'_> {
    /// Select the exact builtin trait, including explicit source implementations.
    /// A same-spelled inherent method is never a candidate for a field operation.
    pub(super) fn builtin_operation(
        &mut self,
        receiver: usize,
        method: BuiltinMethod,
        span: a::Span,
    ) -> Result<usize> {
        let identity = TraitIdentity::Builtin(method.required().to_string());
        if !self.satisfies_identity(receiver, &identity) {
            return Err(err(
                span,
                "E_TRAIT",
                &format!(
                    "{} requires the builtin {} trait",
                    method.name(),
                    method.required(),
                ),
            ));
        }
        let rigid_receiver = self
            .contracts
            .as_ref()
            .is_some_and(|c| c.rigid.contains_key(&receiver));
        if rigid_receiver {
            return self.builtin_method_contract(receiver, method, span);
        }
        if let Some(witness) = self.implementation_witness(receiver, &identity) {
            let mut selected = None;
            for index in &self.source.impls[witness.implementation].methods {
                if self.source.functions[*index].name.as_str() == method.name() {
                    selected = Some(*index);
                    break;
                }
            }
            if let Some(source) = selected {
                if self.source.functions[source].body.is_none() {
                    return Err(err(
                        span,
                        "E_UNSUPPORTED",
                        "selected builtin method has no executable body",
                    ));
                }
                let mut substitutions = witness.substitutions;
                substitutions.insert("Self".to_string(), receiver);
                let parameters = self.source.impls[witness.implementation].generics.clone();
                self.bounds(&parameters, &substitutions)?;
                return self.instantiate(source, substitutions);
            }
        }
        if let ir::TypeKind::Vec { inner } = self.types[receiver].kind {
            let helper = match method {
                BuiltinMethod::Clone => Some("__lkjscript_vec_clone"),
                BuiltinMethod::Eq => Some("__lkjscript_vec_equal"),
                BuiltinMethod::Order => Some("__lkjscript_vec_order"),
                _ => None,
            };
            if let Some(helper) = helper {
                return self.sequence_instance(helper, inner, span);
            }
        }
        if let ir::TypeKind::Slice { inner, .. } = self.types[receiver].kind {
            let helper = match method {
                BuiltinMethod::Eq => Some("__lkjscript_slice_equal"),
                BuiltinMethod::Order => Some("__lkjscript_slice_order"),
                _ => None,
            };
            if let Some(helper) = helper {
                return self.sequence_instance(helper, inner, span);
            }
        }
        if matches!(self.types[receiver].kind, ir::TypeKind::Str) {
            let helper = match method {
                BuiltinMethod::Eq => Some(("__lkjscript_eq", ir::BOOL)),
                BuiltinMethod::Order => Some(("__lkjscript_compare", ir::I32)),
                _ => None,
            };
            if let Some((name, result)) = helper {
                let source = self.names.library_function("str", name, span)?;
                let function = self.instantiate(source, BTreeMap::new())?;
                let reference = self.reference(receiver, false);
                let signature = &self.functions[function];
                if signature.params != 2
                    || signature.result != result
                    || signature.locals[0].ty != reference
                    || signature.locals[1].ty != reference
                {
                    return Err(err(
                        span,
                        "E_TYPE",
                        "source text comparison signature differs from its operation",
                    ));
                }
                return Ok(function);
            }
        }
        if method == BuiltinMethod::CloneFrom
            && matches!(
                self.types[receiver].kind,
                ir::TypeKind::Box { .. } | ir::TypeKind::Array { .. } | ir::TypeKind::Vec { .. }
            )
        {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "clone_from for Box, arrays, and Vec requires the container element update implementation",
            ));
        }
        let key = (receiver, method);
        if let Some(index) = self.derived_functions.get(&key) {
            return Ok(*index);
        }
        if method == BuiltinMethod::CloneFrom && self.has_view(receiver) {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "clone_from for view-bearing values requires a provenance-preserving mutation contract",
            ));
        }
        let reference = self.reference(receiver, method == BuiltinMethod::CloneFrom);
        let mut locals = vec![ir::Local {
            name: "self".to_string(),
            ty: reference,
            mutable: false,
        }];
        if method.arguments() != 0 {
            let shared = self.reference(receiver, false);
            locals.push(ir::Local {
                name: "other".to_string(),
                ty: shared,
                mutable: false,
            });
        }
        let result = method.result(receiver);
        let return_source = if method == BuiltinMethod::Clone && self.has_view(receiver) {
            Some(ir::ReturnSource::Parameter {
                index: 0,
                projections: vec![ir::Projection::Deref],
            })
        } else {
            None
        };
        let index = self.functions.len();
        self.functions.push(ir::Function {
            file: span.file,
            name: format!("$builtin:{}:t{receiver}", method.name()),
            params: locals.len(),
            result,
            return_source,
            locals,
            body: Vec::new(),
            test: false,
        });
        self.derived_functions.insert(key, index);
        self.pending_functions.push((
            index,
            PendingFunction::Derived(DerivedInstance {
                receiver,
                method,
                span,
            }),
        ));
        Ok(index)
    }

    /// Rust's Copy shortcut applies to a derived nominal declaration without
    /// type parameters. Conditional generic derives clone each field even when
    /// a particular application is Copy. Explicit Clone was selected above.
    fn derived_copy_clone(&self, receiver: usize) -> bool {
        if !is_copy(&self.types, receiver) {
            return false;
        }
        let Some((name, _)) = self.applications.get(&receiver) else {
            return false;
        };
        let Some(declaration) = self.declaration_names.get(name) else {
            return false;
        };
        let (parameters, attrs) = match self.source.declarations[*declaration].item {
            a::Item::Struct(index) => {
                let item = &self.source.structs[index];
                (&item.generics, &item.attrs)
            }
            a::Item::Enum(index) => {
                let item = &self.source.enums[index];
                (&item.generics, &item.attrs)
            }
            _ => return false,
        };
        parameters.iter().all(|parameter| parameter.lifetime)
            && attrs
                .iter()
                .any(|attr| attr.name == "derive" && attr.args.iter().any(|name| name == "Copy"))
    }

    pub(super) fn compile_derived(
        &mut self,
        index: usize,
        instance: &DerivedInstance,
    ) -> Result<ir::Function> {
        let function = self.functions[index].clone();
        let live = vec![true; function.params];
        let mut body = Generated {
            compiler: self,
            function,
            live,
            span: instance.span,
        };
        match instance.method {
            BuiltinMethod::Clone => body.clone_value(instance.receiver)?,
            BuiltinMethod::Eq => body.equal_value(instance.receiver)?,
            BuiltinMethod::Order => body.order_value(instance.receiver)?,
            BuiltinMethod::Lt | BuiltinMethod::Le | BuiltinMethod::Gt | BuiltinMethod::Ge => {
                body.relation_value(instance.receiver, instance.method)?;
            }
            BuiltinMethod::Ne => {
                let equal = body.compiler.builtin_operation(
                    instance.receiver,
                    BuiltinMethod::Eq,
                    instance.span,
                )?;
                let compared = body.call(equal, vec![0, 1]);
                let result = body.local(ir::BOOL);
                body.emit(ir::Op::Unary {
                    dst: result,
                    op: "!".to_string(),
                    src: compared,
                });
                body.finish(result);
            }
            BuiltinMethod::CloneFrom => {
                let clone = body.compiler.builtin_operation(
                    instance.receiver,
                    BuiltinMethod::Clone,
                    instance.span,
                )?;
                let cloned = body.call(clone, vec![1]);
                let old = body.local(instance.receiver);
                body.emit(ir::Op::Intrinsic {
                    dst: old,
                    name: "replace".to_string(),
                    args: vec![0, cloned],
                });
                body.emit(ir::Op::Drop {
                    place: ir::Place::local(old),
                });
                let result = body.local(ir::UNIT);
                body.emit(ir::Op::Unit { dst: result });
                body.finish(result);
            }
        }
        Ok(body.function)
    }
}

/// This builder tracks only its own explicit temporaries for cleanup. All types,
/// ownership, tag proofs and provenance are independently checked by IR.
struct Generated<'a, 'b> {
    compiler: &'a mut Compiler<'b>,
    function: ir::Function,
    live: Vec<bool>,
    span: a::Span,
}

impl Generated<'_, '_> {
    fn local(&mut self, ty: usize) -> usize {
        let local = self.function.locals.len();
        self.function.locals.push(ir::Local {
            name: format!("v{local}"),
            ty,
            mutable: false,
        });
        self.live.push(false);
        local
    }

    fn emit(&mut self, op: ir::Op) {
        match &op {
            ir::Op::Read { dst, .. }
            | ir::Op::Borrow { dst, .. }
            | ir::Op::SliceBorrow { dst, .. }
            | ir::Op::Int { dst, .. }
            | ir::Op::Unit { dst }
            | ir::Op::TagTest { dst, .. } => self.live[*dst] = true,
            ir::Op::Call { dst, args, .. } | ir::Op::Intrinsic { dst, args, .. } => {
                for arg in args {
                    self.live[*arg] = false;
                }
                self.live[*dst] = true;
            }
            ir::Op::Struct { dst, fields }
            | ir::Op::Variant { dst, fields, .. }
            | ir::Op::Array {
                dst,
                elements: fields,
            } => {
                for field in fields {
                    self.live[*field] = false;
                }
                self.live[*dst] = true;
            }
            ir::Op::Binary {
                dst, left, right, ..
            } => {
                self.live[*left] = false;
                self.live[*right] = false;
                self.live[*dst] = true;
            }
            ir::Op::Unary { dst, src, .. } => {
                self.live[*src] = false;
                self.live[*dst] = true;
            }
            ir::Op::Assign { place, src } => {
                self.live[*src] = false;
                if place.projections.is_empty() {
                    self.live[place.local] = true;
                }
            }
            ir::Op::Drop { place } | ir::Op::DropIf { place } => {
                if place.projections.is_empty() {
                    self.live[place.local] = false;
                }
            }
            ir::Op::If { condition } => self.live[*condition] = false,
            ir::Op::Return { src } => self.live[*src] = false,
            _ => {}
        }
        self.function.body.push(ir::Instruction {
            op,
            line: self.span.line,
            column: self.span.column.max(1),
        });
    }

    fn restored(&mut self, mut live: Vec<bool>) {
        let length = self.function.locals.len();
        while live.len() < length {
            live.push(false);
        }
        while live.len() > length {
            live.pop();
        }
        self.live = live;
    }

    fn referent(parameter: usize) -> ir::Place {
        ir::Place {
            local: parameter,
            projections: vec![ir::Projection::Deref],
        }
    }

    fn projected(parameter: usize, projection: ir::Projection) -> ir::Place {
        ir::Place {
            local: parameter,
            projections: vec![ir::Projection::Deref, projection],
        }
    }

    fn borrow(&mut self, place: ir::Place, ty: usize) -> usize {
        let reference = self.compiler.reference(ty, false);
        let dst = self.local(reference);
        self.emit(ir::Op::Borrow {
            dst,
            place,
            mutable: false,
        });
        dst
    }

    fn read(&mut self, place: ir::Place, ty: usize) -> usize {
        let dst = self.local(ty);
        self.emit(ir::Op::Read {
            dst,
            place,
            moving: false,
        });
        dst
    }

    fn call(&mut self, function: usize, args: Vec<usize>) -> usize {
        let result = self.compiler.functions[function].result;
        let dst = self.local(result);
        self.emit(ir::Op::Call {
            dst,
            function,
            args,
        });
        dst
    }

    fn finish(&mut self, result: usize) {
        let mut local = self.live.len();
        while local > 0 {
            local -= 1;
            if local != result && self.live[local] {
                self.emit(ir::Op::Drop {
                    place: ir::Place::local(local),
                });
            }
        }
        self.emit(ir::Op::Return { src: result });
    }

    fn boolean(&mut self, value: bool) -> usize {
        let dst = self.local(ir::BOOL);
        self.emit(ir::Op::Int {
            dst,
            value: if value { 1 } else { 0 },
        });
        dst
    }

    fn finish_bool(&mut self, value: bool) {
        let result = self.boolean(value);
        self.finish(result);
    }

    fn index(&mut self, value: usize) -> usize {
        let dst = self.local(ir::U64);
        self.emit(ir::Op::Int {
            dst,
            value: value as i64,
        });
        dst
    }

    fn clone_place(&mut self, place: ir::Place, ty: usize) -> Result<usize> {
        let function = self
            .compiler
            .builtin_operation(ty, BuiltinMethod::Clone, self.span)?;
        let borrowed = self.borrow(place, ty);
        Ok(self.call(function, vec![borrowed]))
    }

    fn clone_fields(&mut self, fields: &[ir::Field], variant: Option<usize>) -> Result<Vec<usize>> {
        let mut values = Vec::new();
        for (index, field) in fields.iter().enumerate() {
            let projection = variant.map_or(ir::Projection::Field(index), |variant| {
                ir::Projection::VariantField(variant, index)
            });
            values.push(self.clone_place(Self::projected(0, projection), field.ty)?);
        }
        Ok(values)
    }

    fn clone_value(&mut self, ty: usize) -> Result<()> {
        let kind = self.compiler.types[ty].kind.clone();
        if matches!(
            kind,
            ir::TypeKind::Unit
                | ir::TypeKind::Bool
                | ir::TypeKind::Char
                | ir::TypeKind::Int { .. }
                | ir::TypeKind::F64
                | ir::TypeKind::Ref { mutable: false, .. }
        ) || self.compiler.derived_copy_clone(ty)
        {
            let result = self.read(Self::referent(0), ty);
            self.finish(result);
            return Ok(());
        }
        match kind {
            ir::TypeKind::Struct { fields, .. } => {
                let fields = self.clone_fields(&fields, None)?;
                let result = self.local(ty);
                self.emit(ir::Op::Struct {
                    dst: result,
                    fields,
                });
                self.finish(result);
            }
            ir::TypeKind::Enum { variants, .. } => {
                for (variant, value) in variants.iter().enumerate() {
                    let condition = self.local(ir::BOOL);
                    self.emit(ir::Op::TagTest {
                        dst: condition,
                        place: ir::Place::local(0),
                        variant,
                    });
                    self.emit(ir::Op::If { condition });
                    let entry = self.live.clone();
                    let fields = self.clone_fields(&value.fields, Some(variant))?;
                    let result = self.local(ty);
                    self.emit(ir::Op::Variant {
                        dst: result,
                        variant,
                        fields,
                    });
                    self.finish(result);
                    self.emit(ir::Op::End);
                    self.restored(entry);
                }
                self.emit(ir::Op::Trap {
                    message: "invalid enum discriminant".to_string(),
                });
            }
            ir::TypeKind::Box { inner } => {
                let result = self.clone_place(Self::projected(0, ir::Projection::Deref), inner)?;
                let boxed = self.local(ty);
                self.emit(ir::Op::Intrinsic {
                    dst: boxed,
                    name: "box_new".to_string(),
                    args: vec![result],
                });
                self.finish(boxed);
            }
            ir::TypeKind::Array { inner, len } => {
                let mut elements = Vec::new();
                for index in 0..len {
                    let at = self.index(index);
                    elements.push(
                        self.clone_place(Self::projected(0, ir::Projection::Index(at)), inner)?,
                    );
                    self.emit(ir::Op::Drop {
                        place: ir::Place::local(at),
                    });
                }
                let result = self.local(ty);
                self.emit(ir::Op::Array {
                    dst: result,
                    elements,
                });
                self.finish(result);
            }
            _ => {
                return Err(err(
                    self.span,
                    "E_UNSUPPORTED",
                    "Clone has no executable implementation for this storage type",
                ));
            }
        }
        Ok(())
    }

    fn integer(&mut self, value: i32) -> usize {
        let result = self.local(ir::I32);
        self.emit(ir::Op::Int {
            dst: result,
            value: value as i64,
        });
        result
    }

    fn finish_order(&mut self, value: i32) {
        let result = self.integer(value);
        self.finish(result);
    }

    fn binary(&mut self, operation: &str, left: usize, right: usize) -> usize {
        let result = self.local(ir::BOOL);
        self.emit(ir::Op::Binary {
            dst: result,
            op: operation.to_string(),
            left,
            right,
        });
        result
    }

    fn return_order_if(&mut self, condition: usize, value: i32) {
        self.emit(ir::Op::If { condition });
        let entry = self.live.clone();
        self.finish_order(value);
        self.emit(ir::Op::End);
        self.restored(entry);
    }

    fn order_places(&mut self, left: ir::Place, right: ir::Place, ty: usize) -> Result<usize> {
        let function = self
            .compiler
            .builtin_operation(ty, BuiltinMethod::Order, self.span)?;
        let left = self.borrow(left, ty);
        let right = self.borrow(right, ty);
        Ok(self.call(function, vec![left, right]))
    }

    // Return the first non-equal classification, including unordered. Equality
    // is an ordering result here; a custom PartialEq implementation is irrelevant.
    fn decisive_order(&mut self, compared: usize) {
        let inspect = self.read(ir::Place::local(compared), ir::I32);
        let zero = self.integer(0);
        let condition = self.binary("!=", inspect, zero);
        self.emit(ir::Op::If { condition });
        let entry = self.live.clone();
        self.finish(compared);
        self.emit(ir::Op::End);
        self.restored(entry);
        self.emit(ir::Op::Drop {
            place: ir::Place::local(compared),
        });
    }

    fn order_fields(&mut self, fields: &[ir::Field], variant: Option<usize>) -> Result<()> {
        for (index, field) in fields.iter().enumerate() {
            let projection = variant.map_or(ir::Projection::Field(index), |variant| {
                ir::Projection::VariantField(variant, index)
            });
            let compared = self.order_places(
                Self::projected(0, projection.clone()),
                Self::projected(1, projection),
                field.ty,
            )?;
            self.decisive_order(compared);
        }
        Ok(())
    }

    fn enum_rank(&mut self, parameter: usize, variants: usize) -> usize {
        let rank = self.index(0);
        self.function.locals[rank].mutable = true;
        for variant in 1..variants {
            let condition = self.local(ir::BOOL);
            self.emit(ir::Op::TagTest {
                dst: condition,
                place: ir::Place::local(parameter),
                variant,
            });
            self.emit(ir::Op::If { condition });
            let entry = self.live.clone();
            let value = self.index(variant);
            self.emit(ir::Op::Assign {
                place: ir::Place::local(rank),
                src: value,
            });
            self.emit(ir::Op::End);
            self.restored(entry);
        }
        rank
    }

    fn order_value(&mut self, ty: usize) -> Result<()> {
        match self.compiler.types[ty].kind.clone() {
            ir::TypeKind::Unit => self.finish_order(0),
            ir::TypeKind::Bool => {
                let left = self.read(Self::referent(0), ty);
                let right = self.read(Self::referent(1), ty);
                let equal = self.binary("==", left, right);
                self.return_order_if(equal, 0);
                let left = self.read(Self::referent(0), ty);
                self.return_order_if(left, 1);
                self.finish_order(-1);
            }
            ir::TypeKind::Char | ir::TypeKind::Int { .. } | ir::TypeKind::F64 => {
                for ordering in 0..2 {
                    let operation = if ordering == 0 { "<" } else { ">" };
                    let value = if ordering == 0 { -1 } else { 1 };
                    let left = self.read(Self::referent(0), ty);
                    let right = self.read(Self::referent(1), ty);
                    let condition = self.binary(operation, left, right);
                    self.return_order_if(condition, value);
                }
                if ty == ir::F64 {
                    let left = self.read(Self::referent(0), ty);
                    let right = self.read(Self::referent(1), ty);
                    let equal = self.binary("==", left, right);
                    self.return_order_if(equal, 0);
                    self.finish_order(2);
                } else {
                    self.finish_order(0);
                }
            }
            ir::TypeKind::Struct { fields, .. } => {
                self.order_fields(&fields, None)?;
                self.finish_order(0);
            }
            ir::TypeKind::Enum { variants, .. } => {
                let left = self.enum_rank(0, variants.len());
                let right = self.enum_rank(1, variants.len());
                for ordering in 0..2 {
                    let operation = if ordering == 0 { "<" } else { ">" };
                    let value = if ordering == 0 { -1 } else { 1 };
                    let left_value = self.read(ir::Place::local(left), ir::U64);
                    let right_value = self.read(ir::Place::local(right), ir::U64);
                    let condition = self.binary(operation, left_value, right_value);
                    self.return_order_if(condition, value);
                }
                self.emit(ir::Op::Drop {
                    place: ir::Place::local(left),
                });
                self.emit(ir::Op::Drop {
                    place: ir::Place::local(right),
                });
                for (variant, value) in variants.iter().enumerate() {
                    let condition = self.local(ir::BOOL);
                    self.emit(ir::Op::TagTest {
                        dst: condition,
                        place: ir::Place::local(0),
                        variant,
                    });
                    self.emit(ir::Op::If { condition });
                    let outer = self.live.clone();
                    let condition = self.local(ir::BOOL);
                    self.emit(ir::Op::TagTest {
                        dst: condition,
                        place: ir::Place::local(1),
                        variant,
                    });
                    self.emit(ir::Op::If { condition });
                    self.order_fields(&value.fields, Some(variant))?;
                    self.finish_order(0);
                    self.emit(ir::Op::End);
                    self.emit(ir::Op::End);
                    self.restored(outer);
                }
                self.emit(ir::Op::Trap {
                    message: "invalid enum discriminant".to_string(),
                });
            }
            ir::TypeKind::Ref { inner, .. } | ir::TypeKind::Box { inner } => {
                let compared = self.order_places(
                    Self::projected(0, ir::Projection::Deref),
                    Self::projected(1, ir::Projection::Deref),
                    inner,
                )?;
                self.finish(compared);
            }
            ir::TypeKind::Array { inner, .. } => {
                let slice = self.compiler.intern(
                    String::new(),
                    ir::TypeKind::Slice {
                        inner,
                        mutable: false,
                    },
                );
                let reference = self.compiler.reference(slice, false);
                let left = self.local(reference);
                self.emit(ir::Op::SliceBorrow {
                    dst: left,
                    place: Self::referent(0),
                    mutable: false,
                });
                let right = self.local(reference);
                self.emit(ir::Op::SliceBorrow {
                    dst: right,
                    place: Self::referent(1),
                    mutable: false,
                });
                let function =
                    self.compiler
                        .sequence_instance("__lkjscript_slice_order", inner, self.span)?;
                let compared = self.call(function, vec![left, right]);
                self.finish(compared);
            }
            _ => {
                return Err(err(
                    self.span,
                    "E_UNSUPPORTED",
                    "PartialOrd has no executable implementation for this storage type",
                ));
            }
        }
        Ok(())
    }

    fn relation_value(&mut self, ty: usize, method: BuiltinMethod) -> Result<()> {
        let order = self
            .compiler
            .builtin_operation(ty, BuiltinMethod::Order, self.span)?;
        let compared = self.call(order, vec![0, 1]);
        let (operation, constant) = match method {
            BuiltinMethod::Lt => ("==", -1),
            BuiltinMethod::Le => ("<=", 0),
            BuiltinMethod::Gt => ("==", 1),
            BuiltinMethod::Ge => ("!=", -1),
            _ => unreachable!(),
        };
        // Unordered is false for every relation, including >=.
        let inspect = self.read(ir::Place::local(compared), ir::I32);
        let unordered = self.integer(2);
        let condition = self.binary("==", inspect, unordered);
        self.emit(ir::Op::If { condition });
        let entry = self.live.clone();
        self.finish_bool(false);
        self.emit(ir::Op::End);
        self.restored(entry);
        let value = self.integer(constant);
        let result = self.binary(operation, compared, value);
        self.finish(result);
        Ok(())
    }

    fn equal_places(&mut self, left: ir::Place, right: ir::Place, ty: usize) -> Result<usize> {
        let function = self
            .compiler
            .builtin_operation(ty, BuiltinMethod::Eq, self.span)?;
        let left = self.borrow(left, ty);
        let right = self.borrow(right, ty);
        Ok(self.call(function, vec![left, right]))
    }

    fn require_equal(&mut self, compared: usize) {
        self.emit(ir::Op::If {
            condition: compared,
        });
        let entry = self.live.clone();
        self.emit(ir::Op::Else);
        self.finish_bool(false);
        self.emit(ir::Op::End);
        self.restored(entry);
    }

    fn equal_fields(&mut self, fields: &[ir::Field], variant: Option<usize>) -> Result<()> {
        for (index, field) in fields.iter().enumerate() {
            let projection = variant.map_or(ir::Projection::Field(index), |variant| {
                ir::Projection::VariantField(variant, index)
            });
            let compared = self.equal_places(
                Self::projected(0, projection.clone()),
                Self::projected(1, projection),
                field.ty,
            )?;
            self.require_equal(compared);
        }
        Ok(())
    }

    fn equal_value(&mut self, ty: usize) -> Result<()> {
        match self.compiler.types[ty].kind.clone() {
            ir::TypeKind::Unit => self.finish_bool(true),
            ir::TypeKind::Bool
            | ir::TypeKind::Char
            | ir::TypeKind::Int { .. }
            | ir::TypeKind::F64 => {
                let left = self.read(Self::referent(0), ty);
                let right = self.read(Self::referent(1), ty);
                let result = self.local(ir::BOOL);
                self.emit(ir::Op::Binary {
                    dst: result,
                    op: "==".to_string(),
                    left,
                    right,
                });
                self.finish(result);
            }
            ir::TypeKind::Struct { fields, .. } => {
                self.equal_fields(&fields, None)?;
                self.finish_bool(true);
            }
            ir::TypeKind::Enum { variants, .. } => {
                for (variant, value) in variants.iter().enumerate() {
                    let condition = self.local(ir::BOOL);
                    self.emit(ir::Op::TagTest {
                        dst: condition,
                        place: ir::Place::local(0),
                        variant,
                    });
                    self.emit(ir::Op::If { condition });
                    let outer = self.live.clone();
                    let condition = self.local(ir::BOOL);
                    self.emit(ir::Op::TagTest {
                        dst: condition,
                        place: ir::Place::local(1),
                        variant,
                    });
                    self.emit(ir::Op::If { condition });
                    let inner = self.live.clone();
                    self.equal_fields(&value.fields, Some(variant))?;
                    self.finish_bool(true);
                    self.emit(ir::Op::Else);
                    self.restored(inner);
                    self.finish_bool(false);
                    self.emit(ir::Op::End);
                    self.emit(ir::Op::End);
                    self.restored(outer);
                }
                self.emit(ir::Op::Trap {
                    message: "invalid enum discriminant".to_string(),
                });
            }
            ir::TypeKind::Ref { inner, .. } | ir::TypeKind::Box { inner } => {
                let compared = self.equal_places(
                    Self::projected(0, ir::Projection::Deref),
                    Self::projected(1, ir::Projection::Deref),
                    inner,
                )?;
                self.finish(compared);
            }
            ir::TypeKind::Array { inner, len } => {
                for index in 0..len {
                    let at = self.index(index);
                    let compared = self.equal_places(
                        Self::projected(0, ir::Projection::Index(at)),
                        Self::projected(1, ir::Projection::Index(at)),
                        inner,
                    )?;
                    self.emit(ir::Op::Drop {
                        place: ir::Place::local(at),
                    });
                    self.require_equal(compared);
                }
                self.finish_bool(true);
            }
            _ => {
                return Err(err(
                    self.span,
                    "E_UNSUPPORTED",
                    "PartialEq has no executable implementation for this storage type",
                ));
            }
        }
        Ok(())
    }
}

impl Builder<'_, '_> {
    /// Ordinary lookup includes the builtin trait implementation alongside
    /// visible source traits. An inherent method still has ordinary priority.
    pub(super) fn builtin_method_available(
        &mut self,
        receiver: usize,
        name: &str,
        span: a::Span,
    ) -> Result<bool> {
        let required = BuiltinMethod::named(name)
            .ok_or_else(|| err(span, "E_TYPE", "unknown builtin trait method"))?
            .required();
        let builtin = self.compiler.satisfies_bound(receiver, required);
        let mut declared_trait = false;
        for implementation_index in 0..self.compiler.source.impls.len() {
            let implementation = self.compiler.source.impls[implementation_index].clone();
            let mut named_method = false;
            for method in &implementation.methods {
                if self.compiler.source.functions[*method].name.as_str() == name {
                    named_method = true;
                    break;
                }
            }
            if !named_method {
                continue;
            }
            let mut substitutions = BTreeMap::new();
            if !self.infer_type_matches(implementation.target, receiver, &mut substitutions)? {
                continue;
            }
            substitutions.insert("Self".to_string(), receiver);
            if self
                .compiler
                .resolve(implementation.target, &substitutions)
                .ok()
                != Some(receiver)
                || !self.trait_available(&implementation, receiver, span)
                || self
                    .compiler
                    .bounds(&implementation.generics, &substitutions)
                    .is_err()
            {
                continue;
            }
            if implementation.trait_ty.is_none() {
                return Ok(false);
            }
            let identity = if let Some(id) = implementation.trait_ty {
                match &self.compiler.source.types[id].kind {
                    a::Type::Path(path) => {
                        Some(crate::generics::bound_identity(self.compiler.source, path))
                    }
                    _ => None,
                }
            } else {
                None
            };
            if identity
                != Some(crate::generics::TraitIdentity::Builtin(
                    required.to_string(),
                ))
            {
                declared_trait = true;
            }
        }
        if declared_trait && builtin {
            return Err(err(
                span,
                "E_TRAIT",
                &format!(
                    "method {name} is ambiguous between builtin and source trait implementations"
                ),
            ));
        }
        Ok(!declared_trait)
    }

    pub(super) fn builtin_trait_method(
        &mut self,
        target: &ir::Place,
        receiver: usize,
        name: &str,
        types: &[usize],
        arguments: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let method = BuiltinMethod::named(name)
            .ok_or_else(|| err(span, "E_TYPE", "unknown builtin trait method"))?;
        if !types.is_empty() {
            return Err(err(
                span,
                "E_TYPE",
                "builtin trait methods take no generic arguments",
            ));
        }
        if arguments.len() != method.arguments() {
            return Err(err(
                span,
                "E_TYPE",
                "builtin trait method argument count differs from its contract",
            ));
        }
        let function = self.compiler.builtin_operation(receiver, method, span)?;
        let borrowed = self
            .borrow(
                target.clone(),
                method == BuiltinMethod::CloneFrom,
                span.line,
            )?
            .0;
        let mut values = vec![borrowed];
        if let Some(argument) = arguments.first() {
            let shared = self.compiler.reference(receiver, false);
            let value = self.expr(*argument, Some(shared))?;
            if value.1 != shared {
                return Err(err(
                    span,
                    "E_TYPE",
                    "builtin trait method argument must borrow the exact receiver type",
                ));
            }
            values.push(value.0);
        }
        let result = method.result(receiver);
        let dst = self.temp(result);
        self.emit(
            ir::Op::Call {
                dst,
                function,
                args: values,
            },
            span.line,
        );
        Ok((dst, result))
    }
}
