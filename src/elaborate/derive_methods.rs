//! Executable builtin trait operations use ordinary, checked K functions.
//!
//! Headers are interned before their bodies enter the shared compilation queue.
//! Recursive nominal layouts therefore reuse an existing function identity;
//! neither helper discovery nor field type traversal recurses on the host stack.
use super::generic_check::BuiltinMethod;
use super::{Builder, Compiler, PendingFunction, Result, a, err, is_copy, k};
use crate::generics::{TraitIdentity, implementation_witness};
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
        if self
            .contracts
            .as_ref()
            .is_some_and(|c| c.rigid.contains_key(&receiver))
        {
            return self.builtin_method_contract(receiver, method, span);
        }
        let empty = BTreeMap::new();
        let assumptions = self.contracts.as_ref().map_or(&empty, |c| &c.rigid);
        if let Some(witness) = implementation_witness(
            self.source,
            &self.implementations,
            &self.types,
            &self.applications,
            receiver,
            &identity,
            assumptions,
        ) {
            let implementation = &self.source.impls[witness.implementation];
            if let Some(source) = implementation
                .methods
                .iter()
                .copied()
                .find(|index| self.source.functions[*index].name == method.name())
            {
                if self.source.functions[source].body.is_none() {
                    return Err(err(
                        span,
                        "E_UNSUPPORTED",
                        "selected builtin method has no executable body",
                    ));
                }
                let mut substitutions = witness.substitutions;
                substitutions.insert("Self".to_string(), receiver);
                self.bounds(&implementation.generics, &substitutions)?;
                return self.instantiate(source, substitutions);
            }
        }
        if let k::TypeKind::Vec { inner } = self.types[receiver].kind {
            let helper = match method {
                BuiltinMethod::Clone => Some("__lkj_vec_clone"),
                BuiltinMethod::Eq => Some("__lkj_vec_equal"),
                _ => None,
            };
            if let Some(helper) = helper {
                return self.sequence_instance(helper, inner, span);
            }
        }
        if method == BuiltinMethod::CloneFrom
            && matches!(
                self.types[receiver].kind,
                k::TypeKind::Box { .. } | k::TypeKind::Array { .. } | k::TypeKind::Vec { .. }
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
        let mut locals = vec![k::Local {
            name: "self".to_string(),
            ty: reference,
            mutable: false,
        }];
        if method.arguments() != 0 {
            let shared = self.reference(receiver, false);
            locals.push(k::Local {
                name: "other".to_string(),
                ty: shared,
                mutable: false,
            });
        }
        let result = method.result(receiver);
        let return_source = if method == BuiltinMethod::Clone && self.has_view(receiver) {
            Some(k::ReturnSource::Parameter {
                index: 0,
                projections: vec![k::Projection::Deref],
            })
        } else {
            None
        };
        let index = self.functions.len();
        self.functions.push(k::Function {
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
    ) -> Result<k::Function> {
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
            BuiltinMethod::Ne => {
                let equal = body.compiler.builtin_operation(
                    instance.receiver,
                    BuiltinMethod::Eq,
                    instance.span,
                )?;
                let compared = body.call(equal, vec![0, 1]);
                let result = body.local(k::BOOL);
                body.emit(k::Op::Unary {
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
                body.emit(k::Op::Intrinsic {
                    dst: old,
                    name: "replace".to_string(),
                    args: vec![0, cloned],
                });
                body.emit(k::Op::Drop {
                    place: k::Place::local(old),
                });
                let result = body.local(k::UNIT);
                body.emit(k::Op::Unit { dst: result });
                body.finish(result);
            }
        }
        Ok(body.function)
    }
}

/// This builder tracks only its own explicit temporaries for cleanup. All types,
/// ownership, tag proofs and provenance are independently checked by K.
struct Generated<'a, 'b> {
    compiler: &'a mut Compiler<'b>,
    function: k::Function,
    live: Vec<bool>,
    span: a::Span,
}

impl Generated<'_, '_> {
    fn local(&mut self, ty: usize) -> usize {
        let local = self.function.locals.len();
        self.function.locals.push(k::Local {
            name: format!("v{local}"),
            ty,
            mutable: false,
        });
        self.live.push(false);
        local
    }

    fn emit(&mut self, op: k::Op) {
        match &op {
            k::Op::Read { dst, .. }
            | k::Op::Borrow { dst, .. }
            | k::Op::Int { dst, .. }
            | k::Op::Unit { dst }
            | k::Op::TagTest { dst, .. } => self.live[*dst] = true,
            k::Op::Call { dst, args, .. } | k::Op::Intrinsic { dst, args, .. } => {
                for arg in args {
                    self.live[*arg] = false;
                }
                self.live[*dst] = true;
            }
            k::Op::Struct { dst, fields }
            | k::Op::Variant { dst, fields, .. }
            | k::Op::Array {
                dst,
                elements: fields,
            } => {
                for field in fields {
                    self.live[*field] = false;
                }
                self.live[*dst] = true;
            }
            k::Op::Binary {
                dst, left, right, ..
            } => {
                self.live[*left] = false;
                self.live[*right] = false;
                self.live[*dst] = true;
            }
            k::Op::Unary { dst, src, .. } => {
                self.live[*src] = false;
                self.live[*dst] = true;
            }
            k::Op::Drop { place } | k::Op::DropIf { place } => {
                if place.projections.is_empty() {
                    self.live[place.local] = false;
                }
            }
            k::Op::If { condition } => self.live[*condition] = false,
            k::Op::Return { src } => self.live[*src] = false,
            _ => {}
        }
        self.function.body.push(k::Instruction {
            op,
            line: self.span.line,
            column: self.span.column.max(1),
        });
    }

    fn restored(&mut self, mut live: Vec<bool>) {
        live.resize(self.function.locals.len(), false);
        self.live = live;
    }

    fn referent(parameter: usize) -> k::Place {
        k::Place {
            local: parameter,
            projections: vec![k::Projection::Deref],
        }
    }

    fn projected(parameter: usize, projection: k::Projection) -> k::Place {
        k::Place {
            local: parameter,
            projections: vec![k::Projection::Deref, projection],
        }
    }

    fn borrow(&mut self, place: k::Place, ty: usize) -> usize {
        let reference = self.compiler.reference(ty, false);
        let dst = self.local(reference);
        self.emit(k::Op::Borrow {
            dst,
            place,
            mutable: false,
        });
        dst
    }

    fn read(&mut self, place: k::Place, ty: usize) -> usize {
        let dst = self.local(ty);
        self.emit(k::Op::Read {
            dst,
            place,
            moving: false,
        });
        dst
    }

    fn call(&mut self, function: usize, args: Vec<usize>) -> usize {
        let dst = self.local(self.compiler.functions[function].result);
        self.emit(k::Op::Call {
            dst,
            function,
            args,
        });
        dst
    }

    fn finish(&mut self, result: usize) {
        for local in (0..self.live.len()).rev() {
            if local != result && self.live[local] {
                self.emit(k::Op::Drop {
                    place: k::Place::local(local),
                });
            }
        }
        self.emit(k::Op::Return { src: result });
    }

    fn boolean(&mut self, value: bool) -> usize {
        let dst = self.local(k::BOOL);
        self.emit(k::Op::Int {
            dst,
            value: i64::from(value),
        });
        dst
    }

    fn finish_bool(&mut self, value: bool) {
        let result = self.boolean(value);
        self.finish(result);
    }

    fn index(&mut self, value: usize) -> usize {
        let dst = self.local(k::U64);
        self.emit(k::Op::Int {
            dst,
            value: value as i64,
        });
        dst
    }

    fn clone_place(&mut self, place: k::Place, ty: usize) -> Result<usize> {
        let function = self
            .compiler
            .builtin_operation(ty, BuiltinMethod::Clone, self.span)?;
        let borrowed = self.borrow(place, ty);
        Ok(self.call(function, vec![borrowed]))
    }

    fn clone_fields(&mut self, fields: &[k::Field], variant: Option<usize>) -> Result<Vec<usize>> {
        let mut values = Vec::new();
        for (index, field) in fields.iter().enumerate() {
            let projection = variant.map_or(k::Projection::Field(index), |variant| {
                k::Projection::VariantField(variant, index)
            });
            values.push(self.clone_place(Self::projected(0, projection), field.ty)?);
        }
        Ok(values)
    }

    fn clone_value(&mut self, ty: usize) -> Result<()> {
        let kind = self.compiler.types[ty].kind.clone();
        if matches!(
            kind,
            k::TypeKind::Unit
                | k::TypeKind::Bool
                | k::TypeKind::Int { .. }
                | k::TypeKind::F64
                | k::TypeKind::Ref { mutable: false, .. }
        ) || self.compiler.derived_copy_clone(ty)
        {
            let result = self.read(Self::referent(0), ty);
            self.finish(result);
            return Ok(());
        }
        match kind {
            k::TypeKind::Struct { fields, .. } => {
                let fields = self.clone_fields(&fields, None)?;
                let result = self.local(ty);
                self.emit(k::Op::Struct {
                    dst: result,
                    fields,
                });
                self.finish(result);
            }
            k::TypeKind::Enum { variants, .. } => {
                for (variant, value) in variants.iter().enumerate() {
                    let condition = self.local(k::BOOL);
                    self.emit(k::Op::TagTest {
                        dst: condition,
                        place: k::Place::local(0),
                        variant,
                    });
                    self.emit(k::Op::If { condition });
                    let entry = self.live.clone();
                    let fields = self.clone_fields(&value.fields, Some(variant))?;
                    let result = self.local(ty);
                    self.emit(k::Op::Variant {
                        dst: result,
                        variant,
                        fields,
                    });
                    self.finish(result);
                    self.emit(k::Op::End);
                    self.restored(entry);
                }
                self.emit(k::Op::Trap {
                    message: "invalid enum discriminant".to_string(),
                });
            }
            k::TypeKind::Box { inner } => {
                let result = self.clone_place(Self::projected(0, k::Projection::Deref), inner)?;
                let boxed = self.local(ty);
                self.emit(k::Op::Intrinsic {
                    dst: boxed,
                    name: "box_new".to_string(),
                    args: vec![result],
                });
                self.finish(boxed);
            }
            k::TypeKind::Array { inner, len } => {
                let mut elements = Vec::new();
                for index in 0..len {
                    let at = self.index(index);
                    elements.push(
                        self.clone_place(Self::projected(0, k::Projection::Index(at)), inner)?,
                    );
                    self.emit(k::Op::Drop {
                        place: k::Place::local(at),
                    });
                }
                let result = self.local(ty);
                self.emit(k::Op::Array {
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

    fn equal_places(&mut self, left: k::Place, right: k::Place, ty: usize) -> Result<usize> {
        let function = self
            .compiler
            .builtin_operation(ty, BuiltinMethod::Eq, self.span)?;
        let left = self.borrow(left, ty);
        let right = self.borrow(right, ty);
        Ok(self.call(function, vec![left, right]))
    }

    fn require_equal(&mut self, compared: usize) {
        self.emit(k::Op::If {
            condition: compared,
        });
        let entry = self.live.clone();
        self.emit(k::Op::Else);
        self.finish_bool(false);
        self.emit(k::Op::End);
        self.restored(entry);
    }

    fn equal_fields(&mut self, fields: &[k::Field], variant: Option<usize>) -> Result<()> {
        for (index, field) in fields.iter().enumerate() {
            let projection = variant.map_or(k::Projection::Field(index), |variant| {
                k::Projection::VariantField(variant, index)
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
            k::TypeKind::Unit => self.finish_bool(true),
            k::TypeKind::Bool | k::TypeKind::Int { .. } | k::TypeKind::F64 => {
                let left = self.read(Self::referent(0), ty);
                let right = self.read(Self::referent(1), ty);
                let result = self.local(k::BOOL);
                self.emit(k::Op::Binary {
                    dst: result,
                    op: "==".to_string(),
                    left,
                    right,
                });
                self.finish(result);
            }
            k::TypeKind::Struct { fields, .. } => {
                self.equal_fields(&fields, None)?;
                self.finish_bool(true);
            }
            k::TypeKind::Enum { variants, .. } => {
                for (variant, value) in variants.iter().enumerate() {
                    let condition = self.local(k::BOOL);
                    self.emit(k::Op::TagTest {
                        dst: condition,
                        place: k::Place::local(0),
                        variant,
                    });
                    self.emit(k::Op::If { condition });
                    let outer = self.live.clone();
                    let condition = self.local(k::BOOL);
                    self.emit(k::Op::TagTest {
                        dst: condition,
                        place: k::Place::local(1),
                        variant,
                    });
                    self.emit(k::Op::If { condition });
                    let inner = self.live.clone();
                    self.equal_fields(&value.fields, Some(variant))?;
                    self.finish_bool(true);
                    self.emit(k::Op::Else);
                    self.restored(inner);
                    self.finish_bool(false);
                    self.emit(k::Op::End);
                    self.emit(k::Op::End);
                    self.restored(outer);
                }
                self.emit(k::Op::Trap {
                    message: "invalid enum discriminant".to_string(),
                });
            }
            k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } => {
                let compared = self.equal_places(
                    Self::projected(0, k::Projection::Deref),
                    Self::projected(1, k::Projection::Deref),
                    inner,
                )?;
                self.finish(compared);
            }
            k::TypeKind::Array { inner, len } => {
                for index in 0..len {
                    let at = self.index(index);
                    let compared = self.equal_places(
                        Self::projected(0, k::Projection::Index(at)),
                        Self::projected(1, k::Projection::Index(at)),
                        inner,
                    )?;
                    self.emit(k::Op::Drop {
                        place: k::Place::local(at),
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
        let required = if matches!(name, "clone" | "clone_from") {
            "Clone"
        } else {
            "PartialEq"
        };
        let builtin = self.compiler.satisfies_bound(receiver, required);
        let mut declared_trait = false;
        for implementation in self.compiler.source.impls.clone() {
            if !implementation
                .methods
                .iter()
                .any(|method| self.compiler.source.functions[*method].name == name)
            {
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
                || !self.trait_available(&implementation, receiver)
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
            let identity =
                implementation
                    .trait_ty
                    .and_then(|id| match &self.compiler.source.types[id].kind {
                        a::Type::Path(path) => {
                            Some(crate::generics::bound_identity(self.compiler.source, path))
                        }
                        _ => None,
                    });
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
        target: &k::Place,
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
            k::Op::Call {
                dst,
                function,
                args: values,
            },
            span.line,
        );
        Ok((dst, result))
    }
}
