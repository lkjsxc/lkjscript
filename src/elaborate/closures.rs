//! Statically resolved callable values are ordinary nominal environments and K
//! functions. Capture requirements come from the operations the ordinary body
//! elaborator emits; the final functions receive no unchecked access privilege.
use super::{Bindings, Builder, Compiler, Facts, Result, a, err, is_copy, k, place_type};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Kind {
    Shared,
    Mutable,
    Once,
}
impl Kind {
    pub(super) fn named(name: &str) -> Option<Self> {
        match name {
            "Fn" => Some(Self::Shared),
            "FnMut" => Some(Self::Mutable),
            "FnOnce" => Some(Self::Once),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Shared => "Fn",
            Self::Mutable => "FnMut",
            Self::Once => "FnOnce",
        }
    }
}
#[derive(Clone)]
pub(super) struct Signature {
    pub(super) kind: Kind,
    pub(super) arguments: Vec<usize>,
    pub(super) result: usize,
}
#[derive(Clone)]
pub(super) struct VariableCapture {
    pub(super) name: String,
    pub(super) ty: usize,
    pub(super) uses: Vec<a::ExprId>,
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct CapturePlan {
    source: usize,
    projections: Vec<k::Projection>,
    mode: Kind,
}
#[derive(Clone, Copy)]
struct CallSelection {
    kind: Kind,
    dereferences: usize,
}
#[derive(Clone, Default)]
pub(super) struct Evidence {
    plans: BTreeMap<a::ExprId, (Kind, Vec<CapturePlan>)>,
    calls: BTreeMap<a::ExprId, CallSelection>,
}
#[derive(Clone)]
struct Closure {
    expression: a::ExprId,
    source_function: usize,
    substitutions: BTreeMap<String, usize>,
    variables: Vec<VariableCapture>,
    signature: Signature,
    plans: Option<Vec<CapturePlan>>,
    function: Option<usize>,
}
#[derive(Clone)]
pub(super) struct RegionBound {
    pub(super) scope: usize,
    pub(super) arguments: Vec<a::TypeId>,
}
type ClosureKey = (a::ExprId, Vec<usize>, Vec<(String, usize)>);
#[derive(Default)]
pub(super) struct Catalog {
    keys: BTreeMap<ClosureKey, usize>,
    entries: BTreeMap<usize, Closure>,
    pub(super) assumptions: BTreeMap<usize, Signature>,
    pub(super) assumed_functions: BTreeMap<usize, usize>,
    pub(super) region_bounds: BTreeMap<usize, Vec<RegionBound>>,
    pub(super) wrappers: BTreeMap<(usize, Kind), usize>,
    pub(super) checked_functions: BTreeSet<usize>,
    pub(super) evidence: Evidence,
}

fn prefix(a: &[k::Projection], b: &[k::Projection]) -> bool {
    a.len() <= b.len() && a.iter().zip(b).all(|(x, y)| x == y)
}
fn operation_place(operation: &k::Op) -> Option<(&k::Place, Kind)> {
    match operation {
        k::Op::Read { place, moving, .. } => {
            Some((place, if *moving { Kind::Once } else { Kind::Shared }))
        }
        k::Op::Borrow { place, mutable, .. }
        | k::Op::SliceBorrow { place, mutable, .. }
        | k::Op::ViewRange { place, mutable, .. } => Some((
            place,
            if *mutable {
                Kind::Mutable
            } else {
                Kind::Shared
            },
        )),
        k::Op::Assign { place, .. } | k::Op::Drop { place } | k::Op::DropIf { place } => {
            Some((place, Kind::Mutable))
        }
        k::Op::TagTest { place, .. } => Some((place, Kind::Shared)),
        _ => None,
    }
}
fn operation_place_mut(operation: &mut k::Op) -> Option<&mut k::Place> {
    match operation {
        k::Op::Read { place, .. }
        | k::Op::Borrow { place, .. }
        | k::Op::SliceBorrow { place, .. }
        | k::Op::ViewRange { place, .. }
        | k::Op::Assign { place, .. }
        | k::Op::Drop { place }
        | k::Op::DropIf { place }
        | k::Op::TagTest { place, .. } => Some(place),
        _ => None,
    }
}

impl Compiler<'_> {
    pub(super) fn closure_type(
        &mut self,
        expression: a::ExprId,
        source_function: usize,
        substitutions: &BTreeMap<String, usize>,
        variables: Vec<VariableCapture>,
        arguments: Vec<usize>,
        result: usize,
    ) -> usize {
        let mut types: Vec<_> = variables.iter().map(|capture| capture.ty).collect();
        types.extend_from_slice(&arguments);
        types.push(result);
        let key = (
            expression,
            types,
            substitutions
                .iter()
                .map(|(name, ty)| (name.clone(), *ty))
                .collect(),
        );
        if let Some(ty) = self.closures.keys.get(&key) {
            return *ty;
        }
        let ty = self.types.len();
        // The nominal ID exists during inference. Its fields are finalized from
        // the body before a value or callable contract using it is emitted.
        self.types.push(k::Type {
            name: format!("$closure:{expression}:{ty}"),
            kind: k::TypeKind::Struct {
                fields: variables
                    .iter()
                    .map(|capture| k::Field {
                        name: capture.name.clone(),
                        ty: capture.ty,
                    })
                    .collect(),
                copy: false,
            },
        });
        self.closures.keys.insert(key, ty);
        self.closures.entries.insert(
            ty,
            Closure {
                expression,
                source_function,
                substitutions: substitutions.clone(),
                variables,
                signature: Signature {
                    kind: Kind::Once,
                    arguments,
                    result,
                },
                plans: None,
                function: None,
            },
        );
        ty
    }

    pub(super) fn closure_reference_fields(&self, ty: usize) -> Vec<usize> {
        if !self.closures.entries.contains_key(&ty) {
            return Vec::new();
        }
        let k::TypeKind::Struct { fields, .. } = &self.types[ty].kind else {
            return Vec::new();
        };
        let mut references = Vec::new();
        for (index, field) in fields.iter().enumerate() {
            if matches!(self.types[field.ty].kind, k::TypeKind::Ref { .. }) {
                references.push(index);
            } else if self.has_view(field.ty) {
                return Vec::new();
            }
        }
        references
    }

    pub(super) fn callable_signature(&self, ty: usize) -> Option<Signature> {
        if let k::TypeKind::Ref { inner, mutable } = self.types[ty].kind {
            let signature = self.callable_signature(inner)?;
            if signature.kind == Kind::Once || (!mutable && signature.kind != Kind::Shared) {
                return None;
            }
            return Some(signature);
        }
        self.closures
            .entries
            .get(&ty)
            .filter(|entry| entry.plans.is_some())
            .map(|entry| entry.signature.clone())
            .or_else(|| self.closures.assumptions.get(&ty).cloned())
    }

    fn callable_type_signature(&self, mut ty: usize) -> Option<Signature> {
        while let k::TypeKind::Ref { inner, .. } = self.types[ty].kind {
            ty = inner;
        }
        self.closures
            .entries
            .get(&ty)
            .map(|entry| entry.signature.clone())
            .or_else(|| self.closures.assumptions.get(&ty).cloned())
    }

    /// A callable bound determines types even when its output parameter does
    /// not occur in the receiver or a value argument. Binding these equations
    /// confers no callable/ownership authority; bounds and concrete K still
    /// validate the selected signature and body independently.
    pub(super) fn prepare_callable_arguments(
        &mut self,
        parameters: &[a::GenericParam],
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<()> {
        loop {
            let before = substitutions.len();
            for parameter in parameters {
                let Some(ty) = substitutions.get(&parameter.name).copied() else {
                    continue;
                };
                let Some(signature) = self.callable_type_signature(ty) else {
                    continue;
                };
                for bound in &parameter.bounds {
                    if !self.is_callable_bound(*bound) {
                        continue;
                    }
                    let a::Type::Path(path) = self.source.types[*bound].kind.clone() else {
                        unreachable!()
                    };
                    if path.args.len() != 2 {
                        continue;
                    }
                    let a::Type::Tuple(arguments) = self.source.types[path.args[0]].kind.clone()
                    else {
                        continue;
                    };
                    if arguments.len() != signature.arguments.len() {
                        continue;
                    }
                    let pairs = std::iter::once((path.args[1], signature.result)).chain(
                        arguments
                            .into_iter()
                            .zip(signature.arguments.iter().copied()),
                    );
                    for (pattern, actual) in pairs {
                        if !self.infer_type_bindings(pattern, actual, substitutions)? {
                            return Err(err(
                                self.source.types[*bound].span,
                                "E_TYPE",
                                "callable bound type arguments disagree",
                            ));
                        }
                    }
                }
            }
            if substitutions.len() == before {
                return Ok(());
            }
        }
    }

    pub(super) fn callable_region_bounds(&self, mut ty: usize) -> Vec<RegionBound> {
        loop {
            if let Some(bounds) = self.closures.region_bounds.get(&ty) {
                return bounds.clone();
            }
            if let Some(closure) = self.closures.entries.get(&ty) {
                let a::Expr::Closure { params, .. } = &self.source.exprs[closure.expression].kind
                else {
                    unreachable!()
                };
                return vec![RegionBound {
                    scope: closure.source_function,
                    arguments: params.iter().map(|parameter| parameter.ty).collect(),
                }];
            }
            if let k::TypeKind::Ref { inner, .. } = self.types[ty].kind {
                ty = inner;
            } else {
                return Vec::new();
            }
        }
    }
    fn check_fresh_callable_inputs(
        &self,
        ty: usize,
        parameters: &[a::TypeId],
        span: a::Span,
    ) -> Result<()> {
        use super::origins::Label;
        let Some(signature) = self.callable_signature(ty) else {
            return Ok(());
        };
        for bound in self.callable_region_bounds(ty) {
            for ((formal, actual), concrete) in parameters
                .iter()
                .zip(&bound.arguments)
                .zip(&signature.arguments)
            {
                let actual: BTreeMap<_, _> = self
                    .region_leaves(bound.scope, *actual, *concrete)
                    .into_iter()
                    .map(|leaf| (leaf.path, leaf.label))
                    .collect();
                for leaf in self.region_leaves(bound.scope, *formal, *concrete) {
                    if matches!(leaf.label, Label::Elided { .. })
                        && actual.get(&leaf.path).is_some_and(|label| {
                            !matches!(label, Label::Elided { .. } | Label::Unknown)
                        })
                    {
                        return Err(err(
                            span,
                            "E_TRAIT",
                            "fixed-region callable does not establish a fresh higher-ranked input",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn is_callable_bound(&self, bound: a::TypeId) -> bool {
        matches!(&self.source.types[bound].kind, a::Type::Path(path)
            if path.declaration.is_none() && Kind::named(&path.name).is_some())
    }

    pub(super) fn callable_bound(
        &mut self,
        ty: usize,
        path: &a::Path,
        substitutions: &BTreeMap<String, usize>,
        span: a::Span,
    ) -> Result<bool> {
        let Some(required) = Kind::named(&path.name).filter(|_| path.declaration.is_none()) else {
            return Ok(false);
        };
        if path.args.len() != 2 {
            return Err(err(
                span,
                "E_TYPE",
                "callable bound requires an argument tuple and result",
            ));
        }
        let a::Type::Tuple(parameters) = self.source.types[path.args[0]].kind.clone() else {
            return Err(err(span, "E_TYPE", "callable arguments require a tuple"));
        };
        let arguments = parameters
            .iter()
            .map(|parameter| self.resolve(*parameter, substitutions))
            .collect::<Result<Vec<_>>>()?;
        let result = self.resolve(path.args[1], substitutions)?;
        let signature = self.callable_signature(ty).ok_or_else(|| {
            err(
                span,
                "E_TRAIT",
                &format!(
                    "{} does not implement {}",
                    self.types[ty].name,
                    required.name()
                ),
            )
        })?;
        if signature.kind > required
            || signature.arguments != arguments
            || signature.result != result
        {
            return Err(err(
                span,
                "E_TRAIT",
                &format!(
                    "{} does not satisfy the declared {} signature",
                    self.types[ty].name,
                    required.name()
                ),
            ));
        }
        self.check_fresh_callable_inputs(ty, &parameters, span)?;
        Ok(true)
    }

    pub(super) fn assume_callable(
        &mut self,
        ty: usize,
        path: &a::Path,
        substitutions: &BTreeMap<String, usize>,
        span: a::Span,
    ) -> Result<bool> {
        let Some(kind) = Kind::named(&path.name).filter(|_| path.declaration.is_none()) else {
            return Ok(false);
        };
        if path.args.len() != 2 {
            return Err(err(
                span,
                "E_TYPE",
                "callable bound requires an argument tuple and result",
            ));
        }
        let a::Type::Tuple(parameters) = self.source.types[path.args[0]].kind.clone() else {
            return Err(err(span, "E_TYPE", "callable arguments require a tuple"));
        };
        if let Some(contracts) = &self.contracts {
            self.closures
                .region_bounds
                .entry(ty)
                .or_default()
                .push(RegionBound {
                    scope: contracts.active_scope,
                    arguments: parameters.clone(),
                });
        }
        let arguments = parameters
            .into_iter()
            .map(|parameter| self.resolve(parameter, substitutions))
            .collect::<Result<Vec<_>>>()?;
        let result = self.resolve(path.args[1], substitutions)?;
        if let Some(prior) = self.closures.assumptions.get_mut(&ty) {
            if prior.arguments != arguments || prior.result != result {
                return Err(err(
                    span,
                    "E_TRAIT",
                    "callable bounds require different signatures",
                ));
            }
            prior.kind = prior.kind.min(kind);
        } else {
            self.closures.assumptions.insert(
                ty,
                Signature {
                    kind,
                    arguments,
                    result,
                },
            );
        }
        self.assume_builtin(ty, kind.name());
        Ok(true)
    }

    fn callable_return_source(
        &self,
        function: &k::Function,
        span: a::Span,
    ) -> Result<Option<k::ReturnSource>> {
        let outputs = crate::check::reference_paths(&self.types, function.result);
        if outputs.is_empty() {
            return Ok(None);
        }
        let mut inputs = BTreeMap::<k::InputLeaf, bool>::new();
        for (index, parameter) in function.locals[..function.params].iter().enumerate() {
            let mut ty = parameter.ty;
            let mut path = Vec::new();
            if index == 0 {
                while let k::TypeKind::Ref { inner, .. } = self.types[ty].kind {
                    ty = inner;
                    path.push(k::Projection::Deref);
                }
            }
            for (leaf, mutable) in crate::check::reference_paths(&self.types, ty) {
                if mutable && !path.is_empty() {
                    continue;
                }
                let mut projections = path.clone();
                projections.extend(leaf);
                inputs.insert(k::InputLeaf { index, projections }, mutable);
            }
        }
        // Stored references reached through captured descriptors have their own
        // origins. Only source places actually traversed by this body add such
        // selectors, so recursive nominal types need no eager unfolding.
        for instruction in &function.body {
            let Some((place, _)) = operation_place(&instruction.op) else {
                continue;
            };
            if place.local >= function.params {
                continue;
            }
            let mut current = k::Place::local(place.local);
            let mut ty = function.locals[place.local].ty;
            let mut borrowed = false;
            let mut in_capture = place.local != 0;
            for projection in &place.projections {
                if matches!(projection, k::Projection::Index(_)) {
                    break;
                }
                if matches!(projection, k::Projection::Deref)
                    && matches!(self.types[ty].kind, k::TypeKind::Ref { .. })
                {
                    borrowed = true;
                }
                if !matches!(projection, k::Projection::Deref) {
                    in_capture = true;
                }
                current.projections.push(projection.clone());
                let Some(next) = place_type(&self.types, function, &current) else {
                    break;
                };
                ty = next;
                if let k::TypeKind::Ref { mutable, .. } = self.types[ty].kind {
                    if in_capture && !(borrowed && mutable) {
                        inputs.insert(
                            k::InputLeaf {
                                index: place.local,
                                projections: current.projections.clone(),
                            },
                            mutable,
                        );
                    }
                }
            }
        }
        let mut leaves = Vec::new();
        for (projections, mutable) in outputs {
            let candidates = inputs
                .iter()
                .filter(|(_, input_mut)| !mutable || **input_mut)
                .map(|(input, _)| input.clone())
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                if mutable {
                    return Err(err(
                        span,
                        "E_BORROW",
                        "callable result has no captured or argument origin",
                    ));
                }
                leaves.push(k::ReturnLeaf {
                    projections,
                    origin: k::LeafOrigin::Static,
                });
            } else {
                leaves.push(k::ReturnLeaf {
                    projections,
                    origin: k::LeafOrigin::Inputs {
                        inputs: candidates,
                        exact: false,
                    },
                });
            }
        }
        Ok(Some(k::ReturnSource::Leaves(leaves)))
    }

    fn forwarded_callable_result(
        &self,
        function: usize,
        receiver: &k::Place,
    ) -> Result<Option<k::ReturnSource>> {
        let Some(k::ReturnSource::Leaves(mut leaves)) =
            self.functions[function].return_source.clone()
        else {
            return Ok(self.functions[function].return_source.clone());
        };
        for leaf in &mut leaves {
            if let k::LeafOrigin::Inputs { inputs, .. } = &mut leaf.origin {
                for input in inputs {
                    if input.index == 0 {
                        if input.projections.first() != Some(&k::Projection::Deref) {
                            return Err(err(
                                a::Span::default(),
                                "E_BORROW",
                                "borrowed callable forwarder needs a receiver reference selector",
                            ));
                        }
                        let mut projections = receiver.projections.clone();
                        projections.extend_from_slice(&input.projections[1..]);
                        input.projections = projections;
                    }
                }
            }
        }
        Ok(Some(k::ReturnSource::Leaves(leaves)))
    }

    fn assumed_callable(&mut self, ty: usize, span: a::Span) -> Result<usize> {
        if let Some(index) = self.closures.assumed_functions.get(&ty) {
            return Ok(*index);
        }
        let signature = self.closures.assumptions[&ty].clone();
        // Opaque F currently has no hidden-leaf K inventory. A signature that
        // returns an explicit reference from F cannot pretend that F is static.
        if self.has_view(signature.result) {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "reference results of an opaque callable need its hidden origin inventory",
            ));
        }
        let receiver = match signature.kind {
            Kind::Shared => self.reference(ty, false),
            Kind::Mutable => self.reference(ty, true),
            Kind::Once => ty,
        };
        let mut locals = vec![k::Local {
            name: "$self".to_string(),
            ty: receiver,
            mutable: false,
        }];
        locals.extend(
            signature
                .arguments
                .iter()
                .enumerate()
                .map(|(i, ty)| k::Local {
                    name: format!("$arg{i}"),
                    ty: *ty,
                    mutable: false,
                }),
        );
        let index = self.functions.len();
        self.functions.push(k::Function {
            file: span.file,
            name: format!("$callable:{ty}"),
            params: locals.len(),
            locals,
            result: signature.result,
            return_source: None,
            body: Vec::new(),
            test: false,
        });
        self.closures.assumed_functions.insert(ty, index);
        Ok(index)
    }

    pub(super) fn assumed_callable_functions(&self) -> BTreeSet<usize> {
        self.closures.assumed_functions.values().copied().collect()
    }

    fn closure_entry(&mut self, ty: usize, requested: Kind, span: a::Span) -> Result<usize> {
        if let k::TypeKind::Ref { inner, .. } = self.types[ty].kind {
            return self.reference_callable_entry(ty, inner, requested, span);
        }
        if self.closures.assumptions.contains_key(&ty) {
            return self.assumed_callable(ty, span);
        }
        let entry = self.closures.entries[&ty].clone();
        if requested < entry.signature.kind {
            return Err(err(
                span,
                "E_TRAIT",
                "closure does not provide the requested call contract",
            ));
        }
        let function = entry
            .function
            .ok_or_else(|| err(span, "E_TYPE", "closure environment is not constructed"))?;
        if requested == entry.signature.kind {
            return Ok(function);
        }
        if let Some(function) = self.closures.wrappers.get(&(ty, requested)) {
            return Ok(*function);
        }
        let receiver = if requested == Kind::Once {
            ty
        } else {
            self.reference(ty, true)
        };
        let mut locals = vec![k::Local {
            name: "$self".to_string(),
            ty: receiver,
            mutable: true,
        }];
        locals.extend(
            entry
                .signature
                .arguments
                .iter()
                .enumerate()
                .map(|(i, ty)| k::Local {
                    name: format!("$arg{i}"),
                    ty: *ty,
                    mutable: false,
                }),
        );
        let params = locals.len();
        let borrowed_ty = self.functions[function].locals[0].ty;
        let borrowed = locals.len();
        locals.push(k::Local {
            name: "$borrow".to_string(),
            ty: borrowed_ty,
            mutable: false,
        });
        let output = locals.len();
        locals.push(k::Local {
            name: "$result".to_string(),
            ty: entry.signature.result,
            mutable: false,
        });
        let place = k::Place {
            local: 0,
            projections: if requested == Kind::Once {
                Vec::new()
            } else {
                vec![k::Projection::Deref]
            },
        };
        let mut args = vec![borrowed];
        args.extend(1..params);
        let operations = vec![
            k::Op::Borrow {
                dst: borrowed,
                place: place.clone(),
                mutable: entry.signature.kind == Kind::Mutable,
            },
            k::Op::Call {
                dst: output,
                function,
                args,
            },
            k::Op::Drop {
                place: k::Place::local(0),
            },
            k::Op::Return { src: output },
        ];
        let return_source = self.forwarded_callable_result(function, &place)?;
        let index = self.functions.len();
        self.functions.push(k::Function {
            file: span.file,
            name: format!("$closure:{}:{ty}:{}", entry.expression, requested.name()),
            params,
            locals,
            result: entry.signature.result,
            return_source,
            body: operations
                .into_iter()
                .map(|op| k::Instruction {
                    op,
                    line: span.line,
                    column: span.column,
                })
                .collect(),
            test: false,
        });
        self.closures.wrappers.insert((ty, requested), index);
        self.closures.checked_functions.insert(index);
        Ok(index)
    }
    fn reference_callable_entry(
        &mut self,
        ty: usize,
        inner: usize,
        requested: Kind,
        span: a::Span,
    ) -> Result<usize> {
        if let Some(index) = self.closures.wrappers.get(&(ty, requested)) {
            return Ok(*index);
        }
        let signature = self
            .callable_signature(ty)
            .ok_or_else(|| err(span, "E_TRAIT", "reference has no callable contract"))?;
        if requested < signature.kind {
            return Err(err(
                span,
                "E_TRAIT",
                "reference does not provide the requested call contract",
            ));
        }
        let target = self.closure_entry(inner, signature.kind, span)?;
        let receiver = match requested {
            Kind::Shared => self.reference(ty, false),
            Kind::Mutable => self.reference(ty, true),
            Kind::Once => ty,
        };
        let mut locals = vec![k::Local {
            name: "$self".to_string(),
            ty: receiver,
            mutable: false,
        }];
        locals.extend(
            signature
                .arguments
                .iter()
                .enumerate()
                .map(|(index, ty)| k::Local {
                    name: format!("$arg{index}"),
                    ty: *ty,
                    mutable: false,
                }),
        );
        let params = locals.len();
        let borrowed = locals.len();
        locals.push(k::Local {
            name: "$borrow".to_string(),
            ty: self.functions[target].locals[0].ty,
            mutable: false,
        });
        let output = locals.len();
        locals.push(k::Local {
            name: "$result".to_string(),
            ty: signature.result,
            mutable: false,
        });
        let mut projections = vec![k::Projection::Deref];
        if requested != Kind::Once {
            projections.push(k::Projection::Deref);
        }
        let place = k::Place {
            local: 0,
            projections,
        };
        let mut args = vec![borrowed];
        args.extend(1..params);
        let return_source = self.forwarded_callable_result(target, &place)?;
        let operations = vec![
            k::Op::Borrow {
                dst: borrowed,
                place,
                mutable: signature.kind == Kind::Mutable,
            },
            k::Op::Call {
                dst: output,
                function: target,
                args,
            },
            k::Op::Drop {
                place: k::Place::local(0),
            },
            k::Op::Return { src: output },
        ];
        let index = self.functions.len();
        self.functions.push(k::Function {
            file: span.file,
            name: format!("$reference-call:{ty}:{}", requested.name()),
            params,
            locals,
            result: signature.result,
            return_source,
            body: operations
                .into_iter()
                .map(|op| k::Instruction {
                    op,
                    line: span.line,
                    column: span.column,
                })
                .collect(),
            test: false,
        });
        self.closures.checked_functions.insert(index);
        self.closures.wrappers.insert((ty, requested), index);
        Ok(index)
    }
}

impl Builder<'_, '_> {
    pub(super) fn closure_expression(
        &mut self,
        expression: a::ExprId,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let ty = self
            .inferred
            .expressions
            .get(&expression)
            .copied()
            .ok_or_else(|| {
                err(
                    span,
                    "E_TYPE",
                    "closure parameter or result type needs context",
                )
            })?;
        self.prepare_closure(ty, span)?;
        let entry = self.compiler.closures.entries[&ty].clone();
        let plans = entry.plans.as_ref().unwrap();
        let mut fields = Vec::new();
        for plan in plans {
            let variable = &entry.variables[plan.source];
            let mut place = self.place(variable.uses[0])?;
            place.projections.extend_from_slice(&plan.projections);
            let value = match plan.mode {
                Kind::Shared => self.borrow(place, false, span.line)?,
                Kind::Mutable => {
                    let source_ty =
                        place_type(&self.compiler.types, self.function, &place).unwrap();
                    if !crate::check::place_writable(&self.compiler.types, self.function, &place)
                        && matches!(
                            self.compiler.types[source_ty].kind,
                            k::TypeKind::Ref { mutable: true, .. }
                        )
                    {
                        // A unique immutable capture may read the descriptor and
                        // mutate its referent, but may not assign the source
                        // binding. A fresh reborrow descriptor represents those
                        // existing authorities without making that binding mut.
                        let mut referent = place;
                        referent.projections.push(k::Projection::Deref);
                        let reborrow = self.borrow(referent, true, span.line)?;
                        let descriptor = self.local(source_ty, "@capture", true);
                        self.emit(
                            k::Op::Assign {
                                place: k::Place::local(descriptor),
                                src: reborrow.0,
                            },
                            span.line,
                        );
                        self.borrow(k::Place::local(descriptor), true, span.line)?
                    } else {
                        self.borrow(place, true, span.line)?
                    }
                }
                Kind::Once => self.read(place, span.line)?,
            };
            fields.push(value.0);
        }
        let dst = self.local(ty, "$", true);
        self.emit(k::Op::Struct { dst, fields }, span.line);
        let function = self.compiler.closures.entries[&ty].function.unwrap();
        self.scoped_body
            .constructions
            .entry(self.function.body.len())
            .or_default()
            .push(crate::check::scoped::Construction {
                function,
                place: k::Place::local(dst),
            });
        Ok((dst, ty))
    }

    fn prepare_closure(&mut self, ty: usize, span: a::Span) -> Result<()> {
        let entry = self.compiler.closures.entries[&ty].clone();
        if entry.plans.is_some() {
            return Ok(());
        }
        let a::Expr::Closure {
            moved,
            params,
            patterns,
            result: declared_result,
            body,
        } = self.compiler.source.exprs[entry.expression].kind.clone()
        else {
            unreachable!()
        };
        let mut locals = vec![k::Local {
            name: "$self".to_string(),
            ty,
            mutable: true,
        }];
        locals.extend(
            params
                .iter()
                .zip(&entry.signature.arguments)
                .map(|(parameter, ty)| k::Local {
                    name: parameter.name.clone(),
                    ty: *ty,
                    mutable: parameter.mutable,
                }),
        );
        let param_count = locals.len();
        let mut aliases = BTreeMap::new();
        let mut capture_locals = BTreeSet::new();
        for variable in &entry.variables {
            let original = self.place(variable.uses[0])?;
            let mutable =
                crate::check::place_writable(&self.compiler.types, self.function, &original);
            let local = locals.len();
            locals.push(k::Local {
                name: variable.name.clone(),
                ty: variable.ty,
                mutable,
            });
            capture_locals.insert(local);
            for expression in &variable.uses {
                aliases.insert(*expression, k::Place::local(local));
            }
        }
        let mut function = k::Function {
            file: span.file,
            name: format!("$closure:{}:{ty}", entry.expression),
            params: param_count,
            locals,
            result: entry.signature.result,
            return_source: None,
            body: Vec::new(),
            test: false,
        };
        let live = Facts::filled(function.locals.len(), true);
        let references = Facts::new(live.len());
        let bindings = function.locals[1..param_count]
            .iter()
            .enumerate()
            .map(|(i, local)| (local.name.clone(), i + 1))
            .collect::<Bindings>();
        let mut scoped_body;
        {
            let mut builder = Builder {
                source_function: entry.source_function,
                scope: self.scope,
                file: span.file,
                compiler: self.compiler,
                function: &mut function,
                return_annotated: declared_result.is_some(),
                scoped_body: super::scoped_locals::Body::default(),
                typed_bindings: Vec::new(),
                sub: entry.substitutions.clone(),
                bindings,
                guard_aliases: BTreeMap::new(),
                guard_shells: Vec::new(),
                capture_aliases: aliases,
                capture_locals,
                active_span: span,
                reference_updates: references.clone(),
                reference_schedules: BTreeMap::new(),
                references,
                carriers: BTreeMap::new(),
                dependents: BTreeMap::new(),
                assignment_types: BTreeMap::new(),
                range_borrows: BTreeMap::new(),
                inferred: self.inferred.clone(),
                written: live.clone(),
                definite_live: live.clone(),
                live,
                loops: Vec::new(),
                reachable: true,
            };
            for (index, parameter) in params.iter().enumerate() {
                let annotations =
                    builder.fixed_annotation(parameter.ty, &k::Place::local(index + 1))?;
                builder.scoped_body.parameters.extend(annotations);
            }
            if let Some(syntax) = declared_result {
                builder.scoped_body.results =
                    builder.fixed_regions(syntax, entry.signature.result)?;
            }
            for (index, pattern) in patterns.iter().enumerate() {
                let mut plan = super::patterns::PatternPlan::default();
                let ty = entry.signature.arguments[index];
                let root = builder.prepare_pattern(&mut plan, *pattern, ty, None)?;
                if !builder.patterns_exhaustive(&plan, &[root], ty)? {
                    return Err(err(
                        builder.compiler.source.patterns[*pattern].span,
                        "E_TYPE",
                        "closure parameter requires an irrefutable pattern",
                    ));
                }
                builder.bind_pattern_plan(&plan, root, k::Place::local(index + 1), span.line)?;
            }
            let result = builder.return_value(body)?;
            if builder.reachable {
                if result.1 != entry.signature.result {
                    return Err(err(span, "E_TYPE", "closure result type differs"));
                }
                builder.cleanup(0, Some(result.0), span.line);
                builder.emit(k::Op::Return { src: result.0 }, span.line);
            }
            scoped_body = std::mem::take(&mut builder.scoped_body);
        }
        let (mut kind, mut plans) = (Kind::Shared, Vec::<CapturePlan>::new());
        for instruction in &function.body {
            let Some((place, mode)) = operation_place(&instruction.op) else {
                continue;
            };
            if place.local < param_count || place.local >= param_count + entry.variables.len() {
                continue;
            }
            if mode == Kind::Mutable
                && !crate::check::place_writable(&self.compiler.types, &function, place)
            {
                return Err(err(
                    a::Span {
                        file: span.file,
                        line: instruction.line,
                        column: instruction.column,
                        ..span
                    },
                    "E_BORROW",
                    "closure writes through an immutable captured place",
                ));
            }
            kind = kind.max(mode);
            let source = place.local - param_count;
            let mut projections = Vec::new();
            for projection in &place.projections {
                if matches!(
                    projection,
                    k::Projection::Index(_) | k::Projection::VariantField(_, _)
                ) || (moved && matches!(projection, k::Projection::Deref))
                {
                    break;
                }
                projections.push(projection.clone());
            }
            let storage = if moved { Kind::Once } else { mode };
            if let Some(existing) = plans
                .iter_mut()
                .find(|plan| plan.source == source && prefix(&plan.projections, &projections))
            {
                existing.mode = existing.mode.max(storage);
            } else {
                let position = plans
                    .iter()
                    .position(|plan| {
                        plan.source == source && prefix(&projections, &plan.projections)
                    })
                    .unwrap_or(plans.len());
                let mut combined = storage;
                plans.retain(|plan| {
                    if plan.source == source && prefix(&projections, &plan.projections) {
                        combined = combined.max(plan.mode);
                        false
                    } else {
                        true
                    }
                });
                plans.insert(
                    position,
                    CapturePlan {
                        source,
                        projections,
                        mode: combined,
                    },
                );
            }
        }
        // Capture roots follow their first source use. Disjoint projections of
        // one root follow that root's declaration order, independently of the
        // order in which operand evaluation emitted its K loads and borrows.
        plans.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then_with(|| left.projections.cmp(&right.projections))
        });
        if let Some((proved_kind, proved_plans)) =
            self.compiler.closures.evidence.plans.get(&entry.expression)
        {
            kind = *proved_kind;
            plans = proved_plans.clone();
        } else if self.compiler.contracts.is_some() {
            self.compiler
                .closures
                .evidence
                .plans
                .insert(entry.expression, (kind, plans.clone()));
        }
        let mut fields = Vec::new();
        for (index, plan) in plans.iter().enumerate() {
            let original = k::Place {
                local: param_count + plan.source,
                projections: plan.projections.clone(),
            };
            let source_ty = place_type(&self.compiler.types, &function, &original)
                .ok_or_else(|| err(span, "E_TYPE", "invalid closure capture projection"))?;
            let field_ty = match plan.mode {
                Kind::Shared => self.compiler.reference(source_ty, false),
                Kind::Mutable => self.compiler.reference(source_ty, true),
                Kind::Once => source_ty,
            };
            fields.push(k::Field {
                name: format!("capture{index}"),
                ty: field_ty,
            });
        }
        let copy = fields
            .iter()
            .all(|field| is_copy(&self.compiler.types, field.ty));
        self.compiler.types[ty].kind = k::TypeKind::Struct { fields, copy };
        function.locals[0].ty = match kind {
            Kind::Shared => self.compiler.reference(ty, false),
            Kind::Mutable => self.compiler.reference(ty, true),
            Kind::Once => ty,
        };
        let remap = |place: &mut k::Place| -> Result<()> {
            if place.local < param_count || place.local >= param_count + entry.variables.len() {
                return Ok(());
            }
            let source = place.local - param_count;
            let (field, plan) = plans
                .iter()
                .enumerate()
                .find(|(_, plan)| {
                    plan.source == source && prefix(&plan.projections, &place.projections)
                })
                .ok_or_else(|| err(span, "E_TYPE", "capture access has no environment field"))?;
            let mut projections = Vec::new();
            if kind != Kind::Once {
                projections.push(k::Projection::Deref);
            }
            projections.push(k::Projection::Field(field));
            if plan.mode != Kind::Once {
                projections.push(k::Projection::Deref);
            }
            projections.extend_from_slice(&place.projections[plan.projections.len()..]);
            *place = k::Place {
                local: 0,
                projections,
            };
            Ok(())
        };
        for instruction in &mut function.body {
            if let Some(place) = operation_place_mut(&mut instruction.op) {
                remap(place)?;
            }
        }
        for place in scoped_body.places_mut() {
            remap(place)?;
        }
        scoped_body.environment = Some(crate::check::scoped::Environment {
            prefix: if kind == Kind::Once {
                Vec::new()
            } else {
                vec![k::Projection::Deref]
            },
        });
        scoped_body.source_scope = Some(entry.source_function);
        function.return_source = self.compiler.callable_return_source(&function, span)?;
        let index = self.compiler.functions.len();
        self.compiler.functions.push(function);
        self.compiler.scoped_bodies.insert(index, scoped_body);
        self.compiler.closures.checked_functions.insert(index);
        let prepared = self.compiler.closures.entries.get_mut(&ty).unwrap();
        prepared.signature.kind = kind;
        prepared.plans = Some(plans);
        prepared.function = Some(index);
        Ok(())
    }

    pub(super) fn callable_call(
        &mut self,
        callee: a::ExprId,
        args: &[a::ExprId],
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let mut place = self.addressable(callee)?;
        let mut ty = place_type(&self.compiler.types, self.function, &place)
            .ok_or_else(|| err(span, "E_TYPE", "invalid callable place"))?;
        let proved = self.compiler.closures.evidence.calls.get(&callee).copied();
        let mut dereferences = 0;
        while let k::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
            if proved.is_some_and(|selection| selection.dereferences == dereferences) {
                break;
            }
            place.projections.push(k::Projection::Deref);
            ty = inner;
            dereferences += 1;
        }
        let signature = self
            .compiler
            .callable_signature(ty)
            .ok_or_else(|| err(span, "E_TYPE", "value has no checked callable contract"))?;
        let requested = proved
            .map(|selection| selection.kind)
            .unwrap_or(signature.kind);
        if self.compiler.contracts.is_some() {
            self.compiler.closures.evidence.calls.insert(
                callee,
                CallSelection {
                    kind: requested,
                    dereferences,
                },
            );
        }
        if args.len() != signature.arguments.len() {
            return Err(err(span, "E_TYPE", "callable argument count differs"));
        }
        if expected.is_some_and(|ty| ty != signature.result) {
            return Err(err(span, "E_TYPE", "callable result type differs"));
        }
        let receiver = match requested {
            Kind::Shared => self.borrow(place, false, span.line)?,
            Kind::Mutable => self.borrow(place, true, span.line)?,
            Kind::Once => self.read(place, span.line)?,
        };
        let mut values = vec![receiver.0];
        for (argument, ty) in args.iter().zip(&signature.arguments) {
            values.push(self.expr(*argument, Some(*ty))?.0);
        }
        let function = self.compiler.closure_entry(ty, requested, span)?;
        let dst = self.temp(signature.result);
        self.emit(
            k::Op::Call {
                dst,
                function,
                args: values,
            },
            span.line,
        );
        Ok((dst, signature.result))
    }
}
