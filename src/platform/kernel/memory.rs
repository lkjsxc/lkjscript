//! Direct owned-memory flow, independent of capability-resource provenance.
use super::infer::ExpressionRead;
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::ExpressionId;
use std::collections::{BTreeMap, BTreeSet};

fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "kernel_buffer_ownership",
        message,
    )
}
pub(crate) fn direct(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
) -> Result<bool, Diagnostic> {
    Ok(
        match read
            .type_object(ty)?
            .ok_or_else(|| reject("missing memory type"))?
            .form
        {
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => true,
            TypeForm::TypeParameter { parameter } => matches!(
                read.owner(OwnerKey::TypeParameter(parameter))?,
                Some(OwnerRecord::TypeParameter(p)) if p.constraints == TypeParameterConstraints::Owned
            ),
            _ => false,
        },
    )
}
fn owned_annotation(
    read: &(impl ExpressionRead + ?Sized),
    annotation: Option<TypeObjectDigest>,
) -> Result<bool, Diagnostic> {
    let Some(annotation) = annotation else {
        return Ok(false);
    };
    static BUFFER: std::sync::OnceLock<Result<TypeObjectDigest, Diagnostic>> =
        std::sync::OnceLock::new();
    let buffer = BUFFER
        .get_or_init(|| {
            TypeObject::new(TypeForm::ByteBuffer)
                .and_then(|object| encode_type_object(&object).map(|(digest, _)| digest))
        })
        .as_ref()
        .map_err(Clone::clone)?;
    // Artifacts may omit ordinary annotation types reconstructed by expression
    // inference. Only the exact concrete buffer annotation requests ownership;
    // that type must still be present, and its initializer is checked separately.
    if annotation != *buffer && read.type_object(annotation)?.is_none() {
        let cell = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell)?)?.0;
        if annotation != cell {
            return Ok(false);
        }
    }
    direct(read, annotation)
}
fn owner(
    read: &(impl ExpressionRead + ?Sized),
    package: PackageId,
    key: OwnerKey,
) -> Result<OwnerRecord, Diagnostic> {
    if package == read.package_id() {
        return read
            .owner(key)?
            .ok_or_else(|| reject("missing memory contract owner"));
    }
    let record = read
        .package_interface_owner(package, key)?
        .ok_or_else(|| reject("missing imported memory contract"))?;
    Ok(match record {
        PackageInterfaceRecord::Parameter(p) => OwnerRecord::Parameter(p),
        PackageInterfaceRecord::Field(p) => OwnerRecord::Field(p),
        PackageInterfaceRecord::Case(p) => OwnerRecord::Case(p),
        PackageInterfaceRecord::TypeParameter(p) => OwnerRecord::TypeParameter(p),
        _ => return Err(reject("unexpected memory contract owner kind")),
    })
}
pub(crate) fn contains(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
) -> Result<bool, Diagnostic> {
    let mut pending = vec![ty];
    let mut seen = BTreeSet::new();
    let mut declarations = BTreeSet::new();
    while let Some(ty) = pending.pop() {
        read.validation_work()?;
        if !seen.insert(ty) {
            continue;
        }
        let object = read
            .type_object(ty)?
            .ok_or_else(|| reject("missing memory containment type"))?;
        if direct(read, ty)? {
            return Ok(true);
        }
        pending.extend(object.child_types());
        let declaration = match object.form {
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                Some(declaration)
            }
            _ => None,
        };
        if let Some(d) = declaration.filter(|d| declarations.insert(*d)) {
            let (fields, cases) = if d.package == read.package_id() {
                match read.owner(OwnerKey::Declaration(d.declaration))? {
                    Some(OwnerRecord::Declaration(p)) => match p.payload {
                        DeclarationPayload::Record { fields, .. } => (fields, vec![]),
                        DeclarationPayload::Variant { cases, .. } => (vec![], cases),
                        _ => (vec![], vec![]),
                    },
                    _ => (vec![], vec![]),
                }
            } else {
                match read
                    .package_interface_owner(d.package, OwnerKey::Declaration(d.declaration))?
                {
                    Some(PackageInterfaceRecord::Declaration(p)) => match p.payload {
                        PackageInterfaceDeclarationPayload::Record { fields, .. } => {
                            (fields, vec![])
                        }
                        PackageInterfaceDeclarationPayload::Variant { cases, .. } => {
                            (vec![], cases)
                        }
                        _ => (vec![], vec![]),
                    },
                    _ => (vec![], vec![]),
                }
            };
            for f in fields {
                if let OwnerRecord::Field(f) = owner(read, d.package, OwnerKey::Field(f))? {
                    pending.push(f.ty);
                }
            }
            for c in cases {
                if let OwnerRecord::Case(c) = owner(read, d.package, OwnerKey::Case(c))? {
                    pending.extend(c.payload);
                }
            }
        }
    }
    Ok(false)
}
struct Signature {
    type_parameters: Vec<crate::platform::semantic_id::TypeParameterId>,
    parameters: Vec<ParameterRecord>,
    result: TypeObjectDigest,
    pure: bool,
}
fn signature(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
) -> Result<Option<Signature>, Diagnostic> {
    let (type_parameters, parameters, result, pure) = if d.package == read.package_id() {
        match read.owner(OwnerKey::Declaration(d.declaration))? {
            Some(OwnerRecord::Declaration(r)) => match r.payload {
                DeclarationPayload::Function(f) => (
                    f.type_parameters,
                    f.parameters,
                    f.result,
                    matches!(f.effect, FunctionEffect::Pure),
                ),
                DeclarationPayload::External(f) => {
                    (f.type_parameters, f.parameters, f.result, true)
                }
                _ => return Ok(None),
            },
            _ => return Err(reject("missing direct memory callee")),
        }
    } else {
        match read.package_interface_owner(d.package, OwnerKey::Declaration(d.declaration))? {
            Some(PackageInterfaceRecord::Declaration(r)) => match r.payload {
                PackageInterfaceDeclarationPayload::Function(f) => (
                    f.type_parameters,
                    f.parameters,
                    f.result,
                    matches!(f.effect, FunctionEffect::Pure),
                ),
                PackageInterfaceDeclarationPayload::External(f) => {
                    (f.type_parameters, f.parameters, f.result, true)
                }
                _ => return Ok(None),
            },
            _ => return Err(reject("missing imported memory callee")),
        }
    };
    let parameters = parameters
        .into_iter()
        .map(
            |id| match owner(read, d.package, OwnerKey::Parameter(id))? {
                OwnerRecord::Parameter(p) => Ok(p),
                _ => Err(reject("wrong memory parameter kind")),
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Signature {
        type_parameters,
        parameters,
        result,
        pure,
    }))
}

fn instantiate(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
    arguments: &[TypeObjectDigest],
    s: &mut Signature,
) -> Result<(), Diagnostic> {
    if s.type_parameters.len() != arguments.len() {
        return Err(reject("memory type argument arity"));
    }
    let mut substitutions = BTreeMap::new();
    for (id, ty) in s.type_parameters.iter().zip(arguments) {
        let OwnerRecord::TypeParameter(p) = owner(read, d.package, OwnerKey::TypeParameter(*id))?
        else {
            return Err(reject("missing generic memory parameter"));
        };
        if p.declaration != d.declaration {
            return Err(reject("foreign generic memory parameter"));
        }
        if p.constraints == TypeParameterConstraints::Owned {
            if !direct(read, *ty)? {
                return Err(reject("owned substitution must be direct memory"));
            }
        } else if contains(read, *ty)? {
            return Err(reject("ordinary generic argument contains owned memory"));
        }
        substitutions.insert(*id, *ty);
    }
    let substitute = |ty: TypeObjectDigest| -> Result<TypeObjectDigest, Diagnostic> {
        match read
            .type_object(ty)?
            .ok_or_else(|| reject("missing memory signature type"))?
            .form
        {
            TypeForm::TypeParameter { parameter } => {
                Ok(substitutions.get(&parameter).copied().unwrap_or(ty))
            }
            _ => Ok(ty),
        }
    };
    for p in &mut s.parameters {
        p.ty = substitute(p.ty)?;
    }
    s.result = substitute(s.result)?;
    Ok(())
}
fn admit_signature(read: &(impl ExpressionRead + ?Sized), s: &Signature) -> Result<(), Diagnostic> {
    let mut memory = false;
    let mut capability = false;
    for p in &s.parameters {
        capability |= p.resource_requirement.is_some();
        if direct(read, p.ty)? {
            memory = true;
            if !s.pure
                || p.use_mode == ParameterUse::Unrestricted
                || p.resource_requirement.is_some()
            {
                return Err(reject(
                    "ByteBuffer parameters require pure borrow/consume with no requirement",
                ));
            }
        } else {
            if contains(read, p.ty)? {
                return Err(reject(
                    "ByteBuffer cannot occur in a parameter container or descriptor",
                ));
            }
            if memory {
                return Err(reject("memory parameters must form a final suffix"));
            }
        }
    }
    let result = direct(read, s.result)?;
    if (!result && contains(read, s.result)?) || (result && !s.pure) {
        return Err(reject("only a direct pure result can transfer ByteBuffer"));
    }
    if capability && (memory || result) {
        return Err(reject(
            "mixed capability/memory function signatures are unsupported",
        ));
    }
    Ok(())
}
#[derive(Clone, Copy, Eq, PartialEq)]
struct Slot {
    borrowed: bool,
    live: bool,
}
type State = BTreeMap<LocalValueReference, Slot>;
fn join(a: &mut State, b: &State) -> Result<(), Diagnostic> {
    if a.len() != b.len() {
        return Err(reject("memory scope disagreement at join"));
    }
    for (key, slot) in a {
        let other = b
            .get(key)
            .ok_or_else(|| reject("memory scope disagreement at join"))?;
        if slot.borrowed != other.borrowed {
            return Err(reject("memory borrow disagreement at join"));
        }
        slot.live &= other.live;
    }
    Ok(())
}
struct Check<'a, R: ?Sized> {
    read: &'a R,
    scope: Option<crate::platform::semantic_id::DeclarationId>,
}
impl<R: ExpressionRead + ?Sized> Check<'_, R> {
    fn eval(
        &self,
        id: ExpressionId,
        state: &mut State,
        mode: ParameterUse,
        depth: usize,
    ) -> Result<bool, Diagnostic> {
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(reject("memory expression depth exceeded"));
        }
        self.read.validation_work()?;
        let Some(OwnerRecord::Expression(e)) = self.read.owner(OwnerKey::Expression(id))? else {
            return Err(reject("missing memory expression"));
        };
        let next = depth + 1;
        let plain = |id, state: &mut State| {
            self.eval(id, state, ParameterUse::Unrestricted, next)
                .map(|_| ())
        };
        let owned = match e.operation {
            ExpressionOperation::Local { value } => {
                if let Some(slot) = state.get_mut(&value) {
                    if !slot.live
                        || mode == ParameterUse::Unrestricted
                        || (slot.borrowed && mode == ParameterUse::Consume)
                    {
                        return Err(reject(
                            "buffer copied, consumed twice, or borrowed parameter consumed/returned",
                        ));
                    }
                    if mode == ParameterUse::Consume {
                        slot.live = false;
                    }
                    return Ok(mode == ParameterUse::Consume);
                }
                false
            }
            ExpressionOperation::Let { bindings, body } => {
                let mut scoped = vec![];
                for b in bindings {
                    let Some(OwnerRecord::Binding(b)) = self.read.owner(OwnerKey::Binding(b))?
                    else {
                        return Err(reject("missing memory binding"));
                    };
                    let value = b
                        .value
                        .ok_or_else(|| reject("missing memory binding initializer"))?;
                    // Annotations are semantic contracts, not ownership certificates.
                    let is_buffer = owned_annotation(self.read, b.declared_type)?;
                    let acquired = self.eval(
                        value,
                        state,
                        if is_buffer {
                            ParameterUse::Consume
                        } else {
                            ParameterUse::Unrestricted
                        },
                        next,
                    )?;
                    if acquired {
                        let local = LocalValueReference::LexicalBinding(match b.header.owner {
                            OwnerKey::Binding(id) => id,
                            _ => return Err(reject("wrong binding identity")),
                        });
                        if state
                            .insert(
                                local,
                                Slot {
                                    borrowed: false,
                                    live: true,
                                },
                            )
                            .is_some()
                        {
                            return Err(reject("duplicate buffer binding"));
                        }
                        scoped.push(local);
                    }
                }
                let result = self.eval(body, state, mode, next)?;
                for b in scoped {
                    state.remove(&b);
                }
                result
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                plain(condition, state)?;
                let mut a = state.clone();
                let mut b = state.clone();
                let av = self.eval(when_true, &mut a, mode, next)?;
                let bv = self.eval(when_false, &mut b, mode, next)?;
                if av != bv {
                    return Err(reject("buffer result ownership must agree at branch join"));
                }
                join(&mut a, &b)?;
                *state = a;
                av
            }
            ExpressionOperation::Sequence { items } => {
                let count = items.len();
                let mut result = false;
                for (i, item) in items.into_iter().enumerate() {
                    result = self.eval(
                        item,
                        state,
                        if i + 1 == count {
                            mode
                        } else {
                            ParameterUse::Consume
                        },
                        next,
                    )?;
                }
                result
            }
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                arguments,
                ..
            }
            | ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
                ..
            } => {
                let Some(mut s) = signature(self.read, function)? else {
                    for a in arguments {
                        plain(a, state)?;
                    }
                    return Ok(false);
                };
                instantiate(self.read, function, &type_arguments, &mut s)?;
                admit_signature(self.read, &s)?;
                if s.parameters.len() != arguments.len() {
                    return Err(reject("memory call arity mismatch"));
                }
                let mut uses = BTreeMap::new();
                for (p, a) in s.parameters.iter().zip(arguments) {
                    if direct(self.read, p.ty)? {
                        let Some(OwnerRecord::Expression(e)) =
                            self.read.owner(OwnerKey::Expression(a))?
                        else {
                            return Err(reject("missing memory argument"));
                        };
                        let ExpressionOperation::Local { value } = e.operation else {
                            return Err(reject("memory call suffix requires exact locals"));
                        };
                        if let Some(prior) = uses.insert(value, p.use_mode)
                            && (prior == ParameterUse::Consume
                                || p.use_mode == ParameterUse::Consume)
                        {
                            return Err(reject(
                                "consuming memory argument aliases another argument",
                            ));
                        }
                        if !state.contains_key(&value) {
                            return Err(reject("fabricated buffer argument"));
                        }
                        self.eval(a, state, p.use_mode, next)?;
                    } else {
                        plain(a, state)?;
                    }
                }
                direct(self.read, s.result)?
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
                let signature = super::owned_contract::method_signature(
                    self.read, witness, contract, method, self.scope,
                )?;
                if signature.parameters.len() != arguments.len() {
                    return Err(reject("owned method arity mismatch"));
                }
                let mut uses = BTreeMap::new();
                for (p, a) in signature.parameters.iter().zip(arguments) {
                    if direct(self.read, p.ty)? {
                        let Some(OwnerRecord::Expression(e)) =
                            self.read.owner(OwnerKey::Expression(a))?
                        else {
                            return Err(reject("missing owned method argument"));
                        };
                        let ExpressionOperation::Local { value } = e.operation else {
                            return Err(reject(
                                "owned methods require exact local memory arguments",
                            ));
                        };
                        if let Some(prior) = uses.insert(value, p.use_mode)
                            && (prior == ParameterUse::Consume
                                || p.use_mode == ParameterUse::Consume)
                        {
                            return Err(reject("owned method consuming alias"));
                        }
                        if !state.contains_key(&value) {
                            return Err(reject("fabricated owned method argument"));
                        }
                        self.eval(a, state, p.use_mode, next)?;
                    } else {
                        plain(a, state)?;
                    }
                }
                direct(self.read, signature.result)?
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                for t in type_arguments {
                    if contains(self.read, t)? {
                        return Err(reject("buffer generic descriptor argument"));
                    }
                }
                if let Some(s) = signature(self.read, function)? {
                    let mut memory = contains(self.read, s.result)?;
                    for p in s.parameters {
                        memory |= contains(self.read, p.ty)?;
                    }
                    if memory {
                        return Err(reject(
                            "memory function cannot become an indirect descriptor",
                        ));
                    }
                }
                false
            }
            ExpressionOperation::Invoke { callee, arguments }
            | ExpressionOperation::Bind { callee, arguments } => {
                plain(callee, state)?;
                for a in arguments {
                    plain(a, state)?;
                }
                false
            }
            ExpressionOperation::Record { fields, .. } => {
                for f in fields {
                    plain(f.value, state)?;
                }
                false
            }
            ExpressionOperation::Variant { payload, .. } => {
                if let Some(p) = payload {
                    plain(p, state)?;
                }
                false
            }
            ExpressionOperation::Field { value, .. } => {
                plain(value, state)?;
                false
            }
            ExpressionOperation::List { items, .. } => {
                for i in items {
                    plain(i, state)?;
                }
                false
            }
            ExpressionOperation::Map { entries, .. } => {
                for e in entries {
                    plain(e.key, state)?;
                    plain(e.value, state)?;
                }
                false
            }
            ExpressionOperation::Match { value, arms } => {
                plain(value, state)?;
                let before = state.clone();
                let mut joined = None;
                for arm in arms {
                    let mut branch = before.clone();
                    let result = self.eval(arm.body, &mut branch, mode, next)?;
                    if let Some((prior, v)) = &mut joined {
                        if *v != result {
                            return Err(reject("buffer match result ownership join"));
                        }
                        join(prior, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                if let Some((branch, result)) = joined {
                    *state = branch;
                    result
                } else {
                    false
                }
            }
            ExpressionOperation::CapabilityCall { arguments, .. } => {
                for a in arguments {
                    plain(a, state)?;
                }
                false
            }
            ExpressionOperation::Transaction { body, .. }
            | ExpressionOperation::TransactionOutcome { body, .. } => {
                self.eval(body, state, mode, next)?
            }
            _ => false,
        };
        if owned && mode != ParameterUse::Consume {
            return Err(reject(
                "owned buffer requires a consuming result or annotated local",
            ));
        }
        Ok(owned)
    }
}
pub(crate) fn validate_owner(
    read: &(impl ExpressionRead + ?Sized),
    key: OwnerKey,
    record: &OwnerRecord,
) -> Result<(), Diagnostic> {
    match record {
        OwnerRecord::TypeParameter(p) if p.constraints == TypeParameterConstraints::Owned => {
            let allowed = match read.owner(OwnerKey::Declaration(p.declaration))? {
                Some(OwnerRecord::Declaration(d)) => match d.payload {
                    DeclarationPayload::OwnedContract(c) => {
                        c.self_parameter
                            == match key {
                                OwnerKey::TypeParameter(id) => id,
                                _ => return Err(reject("invalid Owned parameter identity")),
                            }
                    }
                    DeclarationPayload::Function(f) => {
                        matches!(f.effect, FunctionEffect::Pure)
                            && f.effect_parameters.is_empty()
                            && f.requirement_parameters.is_empty()
                            && matches!(key, OwnerKey::TypeParameter(id) if f.type_parameters.contains(&id))
                    }
                    _ => false,
                },
                _ => false,
            };
            if !allowed {
                return Err(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    "kernel_owned_parameter_owner",
                    "Owned requires an exact pure graph-function parameter or owned contract Self",
                ));
            }
        }
        OwnerRecord::Parameter(p) if direct(read, p.ty)? => {
            let ParameterParent::Function(d) = p.parent else {
                return Err(reject("capability operations cannot carry ByteBuffer"));
            };
            let s = signature(
                read,
                DeclarationReference {
                    package: read.package_id(),
                    declaration: d,
                },
            )?
            .ok_or_else(|| reject("missing buffer signature"))?;
            admit_signature(read, &s)?;
        }
        OwnerRecord::Port(p) => {
            if contains(read, p.function_type)? {
                return Err(reject("buffer adapter port is forbidden"));
            }
            if let PortImplementation::Expression(e) = p.implementation {
                Check { read, scope: None }.eval(
                    e,
                    &mut State::new(),
                    ParameterUse::Unrestricted,
                    0,
                )?;
            }
        }
        OwnerRecord::Field(f) if contains(read, f.ty)? => {
            return Err(reject("buffer field is forbidden"));
        }
        OwnerRecord::Case(c)
            if c.payload
                .map(|t| contains(read, t))
                .transpose()?
                .unwrap_or(false) =>
        {
            return Err(reject("buffer variant payload is forbidden"));
        }
        OwnerRecord::Operation(o) if contains(read, o.result)? => {
            return Err(reject("buffer adapter result is forbidden"));
        }
        OwnerRecord::Declaration(d) => match &d.payload {
            DeclarationPayload::OwnedContract(c) => {
                super::owned_contract::validate_contract(read, key, c)?
            }
            DeclarationPayload::OwnedImplementation(i) => {
                super::owned_contract::validate_implementation(read, i)?
            }
            DeclarationPayload::Constant { ty, value } => {
                if contains(read, *ty)? {
                    return Err(reject("buffer constants are forbidden"));
                }
                Check { read, scope: None }.eval(
                    *value,
                    &mut State::new(),
                    ParameterUse::Unrestricted,
                    0,
                )?;
            }
            DeclarationPayload::Test {
                actual, expected, ..
            } => {
                Check { read, scope: None }.eval(
                    *actual,
                    &mut State::new(),
                    ParameterUse::Unrestricted,
                    0,
                )?;
                Check { read, scope: None }.eval(
                    *expected,
                    &mut State::new(),
                    ParameterUse::Unrestricted,
                    0,
                )?;
            }
            DeclarationPayload::Function(f) => {
                let OwnerKey::Declaration(id) = key else {
                    return Err(reject("wrong memory function owner"));
                };
                super::owned_contract::validate_parameters(read, id, f)?;
                let s = signature(
                    read,
                    DeclarationReference {
                        package: read.package_id(),
                        declaration: id,
                    },
                )?
                .ok_or_else(|| reject("missing function signature"))?;
                admit_signature(read, &s)?;
                let mut state = State::new();
                for p in &s.parameters {
                    if direct(read, p.ty)? {
                        let OwnerKey::Parameter(id) = p.header.owner else {
                            return Err(reject("wrong parameter identity"));
                        };
                        state.insert(
                            LocalValueReference::FunctionParameter(id),
                            Slot {
                                borrowed: p.use_mode == ParameterUse::Borrow,
                                live: true,
                            },
                        );
                    }
                }
                let result = Check {
                    read,
                    scope: Some(id),
                }
                .eval(
                    f.body,
                    &mut state,
                    if direct(read, f.result)? {
                        ParameterUse::Consume
                    } else {
                        ParameterUse::Unrestricted
                    },
                    0,
                )?;
                if result != direct(read, f.result)? {
                    return Err(reject("memory result ownership disagrees with declaration"));
                }
            }
            DeclarationPayload::External(f) => {
                let OwnerKey::Declaration(id) = key else {
                    return Err(reject("wrong external identity"));
                };
                let s = signature(
                    read,
                    DeclarationReference {
                        package: read.package_id(),
                        declaration: id,
                    },
                )?
                .ok_or_else(|| reject("missing external signature"))?;
                admit_signature(read, &s)?;
                let mut memory = direct(read, f.result)?;
                for p in &s.parameters {
                    memory |= direct(read, p.ty)?;
                }
                if memory
                    && !f.implementation.as_str().starts_with("core.buffer.")
                    && !f.implementation.as_str().starts_with("core.cell.")
                {
                    return Err(reject(
                        "memory externals require the closed buffer inventory",
                    ));
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_buffer_metering_remains_exhaustion_with_exact_work_boundaries() {
        let source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author("").unwrap();
        let roots = source.owners.keys().copied().collect::<Vec<_>>();
        let mut steps = 0;
        let mut diagnostics = vec![];
        validate_affine_roots_with_limits(
            &source,
            roots.clone(),
            &mut diagnostics,
            &mut steps,
            ExpressionValidationLimits {
                maximum_steps: contract::MAXIMUM_VALIDATION_WORK,
                maximum_diagnostics: 1000,
            },
        )
        .unwrap();
        assert!(diagnostics.is_empty());
        assert!(steps > 0);
        for maximum in [0, 1, steps - 1, steps] {
            let mut work = 0;
            let mut diagnostics = vec![];
            let result = validate_affine_roots_with_limits(
                &source,
                roots.clone(),
                &mut diagnostics,
                &mut work,
                ExpressionValidationLimits {
                    maximum_steps: maximum,
                    maximum_diagnostics: 1000,
                },
            );
            assert_eq!(work, maximum);
            assert!(diagnostics.is_empty());
            assert_eq!(result.is_ok(), maximum == steps);
        }
        let failure = Diagnostic::new(
            DiagnosticClass::Resource,
            "kernel_affine_work",
            "injected exhausted metadata read",
        );
        let checked = super::super::infer::CheckedExpressionRead {
            read: &source,
            checkpoint: &|| Err(failure.clone()),
        };
        for (key, owner) in &source.owners {
            if matches!(owner,OwnerRecord::Declaration(d)if matches!(d.payload,DeclarationPayload::Function(_)|DeclarationPayload::External(_)))
            {
                assert_eq!(validate_owner(&checked, *key, owner).unwrap_err(), failure);
            }
        }
    }
}
