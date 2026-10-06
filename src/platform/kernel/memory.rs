//! Direct owned-memory flow, independent of capability-resource provenance.
#[cfg(test)]
#[path = "memory_borrow_result_tests.rs"]
mod borrow_result_tests;
#[cfg(test)]
#[path = "memory_borrow_tests.rs"]
mod borrow_tests;
#[cfg(test)]
#[path = "memory_sequence_tests.rs"]
mod sequence_tests;
#[cfg(test)]
#[path = "memory_work_tests.rs"]
mod work_tests;
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
    direct_in(read, read.package_id(), ty)
}

/// A type parameter's constraint belongs to its defining package, not its caller.
pub(crate) fn direct_in(
    read: &(impl ExpressionRead + ?Sized),
    package: PackageId,
    ty: TypeObjectDigest,
) -> Result<bool, Diagnostic> {
    Ok(
        match read
            .type_object(ty)?
            .ok_or_else(|| reject("missing memory type"))?
            .form
        {
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. } => true,
            TypeForm::TypeParameter { parameter } => {
                if package == read.package_id() {
                    matches!(read.owner(OwnerKey::TypeParameter(parameter))?,
                        Some(OwnerRecord::TypeParameter(p)) if p.constraints.has_owned())
                } else {
                    matches!(read.package_interface_owner(package, OwnerKey::TypeParameter(parameter))?,
                        Some(PackageInterfaceRecord::TypeParameter(p)) if p.constraints.has_owned())
                }
            }
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
    // Current derived code retains every annotation root. Missing type metadata
    // cannot turn an explicit product annotation into ordinary data.
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
    result_borrow: Option<crate::platform::semantic_id::ParameterId>,
    pure: bool,
    generation: u16,
}
fn signature(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
) -> Result<Option<Signature>, Diagnostic> {
    let (type_parameters, parameters, result, result_borrow, pure, generation) =
        if d.package == read.package_id() {
            match read.owner(OwnerKey::Declaration(d.declaration))? {
                Some(OwnerRecord::Declaration(r)) => match r.payload {
                    DeclarationPayload::Function(f) => (
                        f.type_parameters,
                        f.parameters,
                        f.result,
                        f.result_borrow,
                        matches!(f.effect, FunctionEffect::Pure),
                        r.header.contract_version,
                    ),
                    DeclarationPayload::External(f) => (
                        f.type_parameters,
                        f.parameters,
                        f.result,
                        None,
                        true,
                        r.header.contract_version,
                    ),
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
                        f.result_borrow,
                        matches!(f.effect, FunctionEffect::Pure),
                        r.header.contract_version,
                    ),
                    PackageInterfaceDeclarationPayload::External(f) => (
                        f.type_parameters,
                        f.parameters,
                        f.result,
                        None,
                        true,
                        r.header.contract_version,
                    ),
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
        result_borrow,
        pure,
        generation,
    }))
}

fn instantiate(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
    arguments: &[TypeObjectDigest],
    s: &mut Signature,
    scope: Option<crate::platform::semantic_id::DeclarationId>,
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
        if p.constraints.has_owned() {
            if !direct(read, *ty)? {
                return Err(reject("owned substitution must be direct memory"));
            }
        } else if contains(read, *ty)? {
            return Err(reject("ordinary generic argument contains owned memory"));
        }
        if p.constraints.requires_transfer() {
            super::transfer::admit(read, *ty, scope)?;
        }
        if p.constraints.requires_share() {
            super::share::admit(read, *ty, scope)?;
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
        if direct(read, p.ty)? {
            if capability {
                return Err(reject("owned memory must precede the resource suffix"));
            }
            memory = true;
            if p.use_mode == ParameterUse::Unrestricted || p.resource_requirement.is_some() {
                return Err(reject(
                    "owned parameters require an explicit use and no requirement",
                ));
            }
            if !s.pure
                && p.use_mode == ParameterUse::Borrow
                && (s.generation < contract::SHARE_GRAPH_CONTRACT_VERSION
                    || p.header.contract_version < contract::SHARE_GRAPH_CONTRACT_VERSION)
            {
                return Err(reject("borrowed task memory parameters require Graph 30"));
            }
        } else if matches!(
            read.type_object(p.ty)?.map(|t| t.form),
            Some(TypeForm::CapabilityResource { .. })
        ) {
            // Resource provenance, effects and use modes retain their independent
            // affine admission; a memory owner never supplies resource authority.
            capability = true;
        } else {
            if contains(read, p.ty)? {
                return Err(reject(
                    "owned memory cannot occur in a parameter container or descriptor",
                ));
            }
            // Resource-only signatures keep their independent affine diagnostics.
            // This checker owns ordering only once an owned-memory input occurs.
            if memory {
                return Err(reject(
                    "ordinary parameters must precede owned memory and resources",
                ));
            }
        }
    }
    let result = direct(read, s.result)?;
    if let Some(source) = s.result_borrow {
        let parameter = s
            .parameters
            .iter()
            .find(|p| p.header.owner == OwnerKey::Parameter(source))
            .ok_or_else(|| {
                reject("borrowed result source is outside the exact parameter inventory")
            })?;
        if !s.pure
            || !result
            || !direct(read, parameter.ty)?
            || parameter.use_mode != ParameterUse::Borrow
        {
            return Err(reject(
                "borrowed results require a pure function, direct owned result and exact borrowed source",
            ));
        }
    }
    if !result && contains(read, s.result)? {
        return Err(reject("only a direct owned result can transfer memory"));
    }
    if s.pure && capability && (memory || result) {
        return Err(reject(
            "pure memory helpers cannot carry capability resources",
        ));
    }
    Ok(())
}
#[derive(Clone, Copy, Eq, PartialEq)]
struct Slot {
    borrowed: bool,
    live: bool,
    /// Lexical parent custody for a projected read view.
    parent: Option<LocalValueReference>,
    /// Active lexical scopes freeze every source in their provenance chain.
    loans: u32,
}
impl Slot {
    fn owner(borrowed: bool) -> Self {
        Self {
            borrowed,
            live: true,
            parent: None,
            loans: 0,
        }
    }
    fn view(parent: LocalValueReference) -> Self {
        Self {
            borrowed: true,
            live: true,
            parent: Some(parent),
            loans: 0,
        }
    }
}
type State = BTreeMap<LocalValueReference, Slot>;
#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Unrestricted,
    Borrow,
    Consume,
    /// Returning a view requires exact semantic provenance, not allocation identity.
    ReturnBorrow(LocalValueReference),
}
impl From<ParameterUse> for Mode {
    fn from(value: ParameterUse) -> Self {
        match value {
            ParameterUse::Unrestricted => Self::Unrestricted,
            ParameterUse::Borrow => Self::Borrow,
            ParameterUse::Consume => Self::Consume,
        }
    }
}

/// Exact, substituted declaration metadata for the lexical borrowed-call boundary.
/// Type inference consumes the same boundary shape; custody is checked separately below.
pub(crate) struct BorrowInvocation {
    pub(crate) result: TypeObjectDigest,
    pub(crate) source: ExpressionId,
    pub(crate) source_type: TypeObjectDigest,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
    arguments: Vec<ExpressionId>,
    parameters: Vec<(TypeObjectDigest, ParameterUse, bool)>,
}
pub(crate) fn borrow_invocation(
    read: &(impl ExpressionRead + ?Sized),
    call: ExpressionId,
    scope: Option<crate::platform::semantic_id::DeclarationId>,
) -> Result<BorrowInvocation, Diagnostic> {
    let Some(OwnerRecord::Expression(expression)) = read.owner(OwnerKey::Expression(call))? else {
        return Err(reject("missing borrowed invocation"));
    };
    let (arguments, parameters, result, source_position, types) = match expression.operation {
        ExpressionOperation::Call {
            function,
            type_arguments,
            arguments,
            ..
        }
        | ExpressionOperation::ImplementationCall {
            function,
            type_arguments,
            arguments,
            ..
        } => {
            let mut signature = signature(read, function)?
                .ok_or_else(|| reject("borrow-call requires a named graph function"))?;
            let source = signature
                .result_borrow
                .ok_or_else(|| reject("borrow-call requires a declared borrowed result"))?;
            let position = signature
                .parameters
                .iter()
                .position(|p| p.header.owner == OwnerKey::Parameter(source))
                .ok_or_else(|| reject("borrowed invocation source is not a parameter"))?;
            let original_parameters = signature.parameters.clone();
            let original_result = signature.result;
            instantiate(read, function, &type_arguments, &mut signature, scope)?;
            admit_signature(read, &signature)?;
            let substitutions = signature
                .type_parameters
                .iter()
                .copied()
                .zip(type_arguments.iter().copied())
                .collect::<BTreeMap<_, _>>();
            let mut derived = super::parallel_types::AppliedTypes::new(read);
            let mut parameters = Vec::new();
            for parameter in &original_parameters {
                let ty = derived.substitute(parameter.ty, &substitutions, 0)?;
                parameters.push((ty, parameter.use_mode, direct(&derived, ty)?));
            }
            let result = derived.substitute(original_result, &substitutions, 0)?;
            (
                arguments,
                parameters,
                result,
                position,
                derived.into_types(),
            )
        }
        ExpressionOperation::MethodCall {
            witness,
            contract,
            method,
            arguments,
        } => {
            let application =
                super::owned_contract::method_signature(read, &witness, contract, method, scope)?;
            let signature = application.method;
            let position = signature
                .result_borrow
                .ok_or_else(|| reject("borrow-call requires a declared borrowed method result"))?
                as usize;
            let overlay = super::owned_contract::AppliedTypeRead {
                read,
                types: &application.types,
            };
            let mut parameters = Vec::new();
            for parameter in &signature.parameters {
                parameters.push((
                    parameter.ty,
                    parameter.use_mode,
                    direct(&overlay, parameter.ty)?,
                ));
            }
            (
                arguments,
                parameters,
                signature.result,
                position,
                application.types,
            )
        }
        _ => {
            return Err(reject(
                "borrow-call requires an exact call, implementation-call or method-call",
            ));
        }
    };
    if arguments.len() != parameters.len() {
        return Err(reject("borrowed invocation argument arity mismatch"));
    }
    let source = *arguments
        .get(source_position)
        .ok_or_else(|| reject("borrowed invocation source is out of range"))?;
    let (source_type, use_mode, owned) = parameters[source_position];
    if !owned || use_mode != ParameterUse::Borrow {
        return Err(reject(
            "borrowed invocation source requires an exact borrowed owned parameter",
        ));
    }
    Ok(BorrowInvocation {
        result,
        source,
        source_type,
        types,
        arguments,
        parameters,
    })
}
fn fork(read: &(impl ExpressionRead + ?Sized), state: &State) -> Result<State, Diagnostic> {
    // Admit the complete fixed-size slot inventory before cloning its map nodes.
    for _ in 0..state.len() {
        read.validation_work()?;
    }
    Ok(state.clone())
}
fn join(read: &(impl ExpressionRead + ?Sized), a: &mut State, b: &State) -> Result<(), Diagnostic> {
    if a.len() != b.len() {
        return Err(reject("memory scope disagreement at join"));
    }
    for (key, slot) in a {
        read.validation_work()?;
        let other = b
            .get(key)
            .ok_or_else(|| reject("memory scope disagreement at join"))?;
        if slot.borrowed != other.borrowed
            || slot.parent != other.parent
            || slot.loans != other.loans
        {
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
    fn sequence_item(&self, ty: TypeObjectDigest) -> Result<TypeObjectDigest, Diagnostic> {
        super::owned_product::validate(self.read, ty, self.scope)?;
        let TypeForm::OwnedSequence { item } = self
            .read
            .type_object(ty)?
            .ok_or_else(|| reject("missing owned sequence type"))?
            .form
        else {
            return Err(reject("sequence operation requires an owned sequence type"));
        };
        Ok(item)
    }

    fn consume_local(
        &self,
        expression: ExpressionId,
        ty: TypeObjectDigest,
        state: &mut State,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        // Admission proves exact local identity and type before changing custody.
        self.source_local(expression, ty, state, depth)?;
        if !self.eval(expression, state, ParameterUse::Consume, depth)? {
            return Err(reject("sequence operand requires an exact live owner"));
        }
        Ok(())
    }

    fn source_local(
        &self,
        expression: ExpressionId,
        ty: TypeObjectDigest,
        state: &mut State,
        depth: usize,
    ) -> Result<LocalValueReference, Diagnostic> {
        let Some(OwnerRecord::Expression(ExpressionRecord {
            operation: ExpressionOperation::Local { value },
            ..
        })) = self.read.owner(OwnerKey::Expression(expression))?
        else {
            return Err(reject("child reads require an exact local source"));
        };
        let actual = match value {
            LocalValueReference::FunctionParameter(parameter) => {
                match self.read.owner(OwnerKey::Parameter(parameter))? {
                    Some(OwnerRecord::Parameter(record)) => Some(record.ty),
                    _ => None,
                }
            }
            LocalValueReference::LexicalBinding(binding) => {
                match self.read.owner(OwnerKey::Binding(binding))? {
                    Some(OwnerRecord::Binding(record)) => record.declared_type,
                    _ => None,
                }
            }
            _ => None,
        };
        if actual != Some(ty) || !state.contains_key(&value) {
            return Err(reject(
                "child read source requires an exact typed live owner or view",
            ));
        }
        self.eval(expression, state, ParameterUse::Borrow, depth)?;
        Ok(value)
    }

    fn loan(
        &self,
        source: LocalValueReference,
        state: &mut State,
        acquire: bool,
    ) -> Result<(), Diagnostic> {
        let mut current = Some(source);
        let mut remaining = state.len();
        while let Some(local) = current {
            self.read.validation_work()?;
            if remaining == 0 {
                return Err(reject("cyclic child read provenance"));
            }
            remaining -= 1;
            let slot = state
                .get_mut(&local)
                .ok_or_else(|| reject("missing child read ancestor"))?;
            if !slot.live {
                return Err(reject("child read ancestor is no longer live"));
            }
            slot.loans = if acquire {
                slot.loans
                    .checked_add(1)
                    .ok_or_else(|| reject("child read loan overflow"))?
            } else {
                slot.loans
                    .checked_sub(1)
                    .ok_or_else(|| reject("missing child read loan"))?
            };
            current = slot.parent;
        }
        Ok(())
    }

    fn borrow_binding(
        &self,
        binding: crate::platform::semantic_id::BindingId,
        ty: TypeObjectDigest,
    ) -> Result<(), Diagnostic> {
        let Some(OwnerRecord::Binding(record)) = self.read.owner(OwnerKey::Binding(binding))?
        else {
            return Err(reject("missing child read binding"));
        };
        if record.kind != BindingKind::OwnedBorrow
            || record.value.is_some()
            || record.declared_type != Some(ty)
        {
            return Err(reject("child read binding contract mismatch"));
        }
        Ok(())
    }

    fn parallel_arguments(
        &self,
        call: &super::parallel::ParallelCall,
        state: &mut State,
        loans: &mut Vec<LocalValueReference>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        for (parameter, argument) in call.parameters.iter().zip(&call.arguments) {
            self.read.validation_work()?;
            match parameter.use_mode {
                ParameterUse::Borrow => {
                    let source = self.source_local(*argument, parameter.ty, state, depth + 1)?;
                    // A child is not executed during preparation. Its loan must
                    // already freeze every ancestor while later arguments run.
                    self.loan(source, state, true)?;
                    loans.push(source);
                }
                ParameterUse::Consume => {
                    self.consume_local(*argument, parameter.ty, state, depth + 1)?;
                }
                ParameterUse::Unrestricted => {
                    self.eval(*argument, state, ParameterUse::Unrestricted, depth + 1)?;
                }
            }
        }
        Ok(())
    }

    fn eval(
        &self,
        id: ExpressionId,
        state: &mut State,
        mode: impl Into<Mode>,
        depth: usize,
    ) -> Result<bool, Diagnostic> {
        let mode = mode.into();
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(reject("memory expression depth exceeded"));
        }
        self.read.validation_work()?;
        let Some(OwnerRecord::Expression(e)) = self.read.owner(OwnerKey::Expression(id))? else {
            return Err(reject("missing memory expression"));
        };
        if matches!(mode, Mode::ReturnBorrow(_))
            && !matches!(
                e.operation,
                ExpressionOperation::Local { .. }
                    | ExpressionOperation::Let { .. }
                    | ExpressionOperation::If { .. }
                    | ExpressionOperation::Sequence { .. }
                    | ExpressionOperation::BorrowCall { .. }
                    | ExpressionOperation::BorrowOwnedItem { .. }
                    | ExpressionOperation::BorrowOwnedField { .. }
                    | ExpressionOperation::MatchBorrowedOwned { .. }
                    | ExpressionOperation::MatchOwned { .. }
                    | ExpressionOperation::UnpackOwned { .. }
                    | ExpressionOperation::Match { .. }
            )
        {
            return Err(reject(
                "borrowed results require an exact borrowed local, optionally selected through lexical scopes and branches",
            ));
        }
        let next = depth + 1;
        let plain = |id, state: &mut State| {
            self.eval(id, state, ParameterUse::Unrestricted, next)
                .map(|_| ())
        };
        let owned = match e.operation {
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                let invocation = borrow_invocation(self.read, call, self.scope)?;
                self.borrow_binding(binding, invocation.result)?;
                let mut uses = BTreeMap::new();
                for ((ty, use_mode, owned), argument) in
                    invocation.parameters.iter().zip(&invocation.arguments)
                {
                    if *owned {
                        let local = self.source_local(*argument, *ty, state, next)?;
                        if let Some(prior) = uses.insert(local, *use_mode)
                            && (prior == ParameterUse::Consume
                                || *use_mode == ParameterUse::Consume)
                        {
                            return Err(reject(
                                "consuming borrowed invocation argument aliases another argument",
                            ));
                        }
                        self.eval(*argument, state, *use_mode, next)?;
                    } else {
                        plain(*argument, state)?;
                    }
                }
                let source =
                    self.source_local(invocation.source, invocation.source_type, state, next)?;
                self.loan(source, state, true)?;
                let local = LocalValueReference::LexicalBinding(binding);
                if state.insert(local, Slot::view(source)).is_some() {
                    return Err(reject("duplicate borrowed invocation binding"));
                }
                let result = self.eval(body, state, mode, next)?;
                state.remove(&local);
                self.loan(source, state, false)?;
                result
            }
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                self.sequence_item(sequence_type)?;
                true
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                self.sequence_item(sequence_type)?;
                self.source_local(source, sequence_type, state, next)?;
                false
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                let item = self.sequence_item(sequence_type)?;
                // Authored evaluation order consumes the element before the sequence.
                self.consume_local(value, item, state, next)?;
                self.consume_local(source, sequence_type, state, next)?;
                true
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                self.sequence_item(sequence_type)?;
                super::owned_product::validate(self.read, result_type, self.scope)?;
                self.consume_local(source, sequence_type, state, next)?;
                true
            }
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                let item = self.sequence_item(sequence_type)?;
                self.borrow_binding(binding, item)?;
                // An effectful index can move the source. Check its liveness afterwards.
                plain(index, state)?;
                let source = self.source_local(source, sequence_type, state, next)?;
                self.loan(source, state, true)?;
                let local = LocalValueReference::LexicalBinding(binding);
                if state.insert(local, Slot::view(source)).is_some() {
                    return Err(reject("duplicate sequence read binding"));
                }
                let result = self.eval(body, state, mode, next)?;
                state.remove(&local);
                self.loan(source, state, false)?;
                result
            }
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => {
                super::owned_product::validate(self.read, product_type, self.scope)?;
                let TypeForm::OwnedProduct { fields } = self
                    .read
                    .type_object(product_type)?
                    .ok_or_else(|| reject("missing child read product type"))?
                    .form
                else {
                    return Err(reject("field read requires an owned product"));
                };
                let mut selected = None;
                for candidate in fields {
                    self.read.validation_work()?;
                    if candidate.name == field {
                        selected = Some(candidate.ty);
                    }
                }
                let selected = selected.ok_or_else(|| reject("unknown child read field"))?;
                if !direct(self.read, selected)? {
                    return Err(reject("field read binding requires a direct owned child"));
                }
                self.borrow_binding(binding, selected)?;
                let source = self.source_local(source, product_type, state, next)?;
                self.loan(source, state, true)?;
                let local = LocalValueReference::LexicalBinding(binding);
                if state.insert(local, Slot::view(source)).is_some() {
                    return Err(reject("duplicate child read binding"));
                }
                let result = self.eval(body, state, mode, next)?;
                state.remove(&local);
                self.loan(source, state, false)?;
                result
            }
            ExpressionOperation::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                super::owned_product::validate(self.read, choice_type, self.scope)?;
                let TypeForm::OwnedChoice { cases } = self
                    .read
                    .type_object(choice_type)?
                    .ok_or_else(|| reject("missing child read choice type"))?
                    .form
                else {
                    return Err(reject("borrowed match requires an owned choice"));
                };
                if cases.len() != arms.len() || arms.is_empty() {
                    return Err(reject("borrowed match must cover every case exactly once"));
                }
                let source = self.source_local(source, choice_type, state, next)?;
                self.loan(source, state, true)?;
                let mut joined: Option<(State, bool)> = None;
                for (arm, case) in arms.into_iter().zip(cases) {
                    self.read.validation_work()?;
                    if arm.name != case.name {
                        return Err(reject("borrowed match case mismatch"));
                    }
                    self.borrow_binding(arm.binding, case.ty)?;
                    let mut branch = fork(self.read, state)?;
                    let local = LocalValueReference::LexicalBinding(arm.binding);
                    if direct(self.read, case.ty)?
                        && branch.insert(local, Slot::view(source)).is_some()
                    {
                        return Err(reject("duplicate borrowed match binding"));
                    }
                    let result = self.eval(arm.body, &mut branch, mode, next)?;
                    branch.remove(&local);
                    if let Some((prior, prior_result)) = &mut joined {
                        if *prior_result != result {
                            return Err(reject("borrowed match result ownership disagreement"));
                        }
                        join(self.read, prior, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                let (joined, result) = joined.ok_or_else(|| reject("empty borrowed match"))?;
                *state = joined;
                self.loan(source, state, false)?;
                result
            }
            ExpressionOperation::ChooseOwned {
                choice_type,
                case,
                value,
            } => {
                super::owned_product::validate(self.read, choice_type, self.scope)?;
                let TypeForm::OwnedChoice { cases } = self
                    .read
                    .type_object(choice_type)?
                    .ok_or_else(|| reject("missing owned choice type"))?
                    .form
                else {
                    return Err(reject("choice construction requires an owned choice"));
                };
                let mut payload = None;
                for candidate in cases {
                    self.read.validation_work()?;
                    if candidate.name == case {
                        payload = Some(candidate.ty);
                    }
                }
                let payload = payload.ok_or_else(|| reject("unknown owned choice case"))?;
                if direct(self.read, payload)? {
                    let Some(OwnerRecord::Expression(ExpressionRecord {
                        operation: ExpressionOperation::Local { value: local },
                        ..
                    })) = self.read.owner(OwnerKey::Expression(value))?
                    else {
                        return Err(reject(
                            "owned choice payload requires an exact local operand",
                        ));
                    };
                    if !state.contains_key(&local)
                        || !self.eval(value, state, ParameterUse::Consume, next)?
                    {
                        return Err(reject("owned choice payload has no live owner"));
                    }
                } else {
                    plain(value, state)?;
                }
                true
            }
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            } => {
                super::owned_product::validate(self.read, choice_type, self.scope)?;
                let Some(OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::Local { value },
                    ..
                })) = self.read.owner(OwnerKey::Expression(source))?
                else {
                    return Err(reject("owned match requires an exact local operand"));
                };
                if !state.contains_key(&value)
                    || !self.eval(source, state, ParameterUse::Consume, next)?
                {
                    return Err(reject("owned match requires a live owner"));
                }
                let TypeForm::OwnedChoice { cases } = self
                    .read
                    .type_object(choice_type)?
                    .ok_or_else(|| reject("missing owned choice type"))?
                    .form
                else {
                    return Err(reject("owned match source is not a choice"));
                };
                if cases.len() != arms.len() || arms.is_empty() {
                    return Err(reject("owned match must cover every case exactly once"));
                }
                let mut joined: Option<(State, bool)> = None;
                for (arm, case) in arms.into_iter().zip(cases) {
                    self.read.validation_work()?;
                    let Some(OwnerRecord::Binding(b)) =
                        self.read.owner(OwnerKey::Binding(arm.binding))?
                    else {
                        return Err(reject("missing owned choice binding"));
                    };
                    if arm.name != case.name
                        || b.declared_type != Some(case.ty)
                        || b.kind != BindingKind::OwnedChoicePayload
                        || b.value.is_some()
                    {
                        return Err(reject("owned choice binding contract mismatch"));
                    }
                    let mut branch = fork(self.read, state)?;
                    let local = LocalValueReference::LexicalBinding(arm.binding);
                    if direct(self.read, case.ty)?
                        && branch.insert(local, Slot::owner(false)).is_some()
                    {
                        return Err(reject("duplicate owned choice local"));
                    }
                    let result = self.eval(arm.body, &mut branch, mode, next)?;
                    branch.remove(&local);
                    if let Some((prior, prior_result)) = &mut joined {
                        if *prior_result != result {
                            return Err(reject(
                                "owned match result ownership disagrees between cases",
                            ));
                        }
                        join(self.read, prior, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                let (joined, result) = joined.ok_or_else(|| reject("empty owned match"))?;
                *state = joined;
                result
            }
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                super::owned_product::validate(self.read, product_type, self.scope)?;
                let TypeForm::OwnedProduct { fields: contract } = self
                    .read
                    .type_object(product_type)?
                    .ok_or_else(|| reject("missing product type"))?
                    .form
                else {
                    return Err(reject("pack operand is not a product"));
                };
                if fields.len() != contract.len() {
                    return Err(reject("incomplete product construction"));
                }
                let mut names = BTreeSet::new();
                for field in fields {
                    let expected = contract
                        .iter()
                        .find(|f| f.name == field.name)
                        .ok_or_else(|| reject("unknown product field"))?;
                    if !names.insert(field.name) {
                        return Err(reject("duplicate product field"));
                    }
                    if direct(self.read, expected.ty)? {
                        let Some(OwnerRecord::Expression(ExpressionRecord {
                            operation: ExpressionOperation::Local { value },
                            ..
                        })) = self.read.owner(OwnerKey::Expression(field.value))?
                        else {
                            return Err(reject(
                                "owned product fields require exact local operands",
                            ));
                        };
                        if !state.contains_key(&value)
                            || !self.eval(field.value, state, ParameterUse::Consume, next)?
                        {
                            return Err(reject("product field has no live owner"));
                        }
                    } else {
                        plain(field.value, state)?;
                    }
                }
                true
            }
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                super::owned_product::validate(self.read, product_type, self.scope)?;
                let Some(OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::Local { value },
                    ..
                })) = self.read.owner(OwnerKey::Expression(source))?
                else {
                    return Err(reject("unpack requires an exact local operand"));
                };
                if !state.contains_key(&value)
                    || !self.eval(source, state, ParameterUse::Consume, next)?
                {
                    return Err(reject("unpack requires a live owner"));
                }
                let TypeForm::OwnedProduct { fields: expected } = self
                    .read
                    .type_object(product_type)?
                    .ok_or_else(|| reject("missing product type"))?
                    .form
                else {
                    return Err(reject("unpack operand is not a product"));
                };
                if expected.len() != fields.len() {
                    return Err(reject("incomplete unpack"));
                }
                let mut scoped = Vec::new();
                for (field, expected) in fields.iter().zip(expected) {
                    let Some(OwnerRecord::Binding(b)) =
                        self.read.owner(OwnerKey::Binding(field.binding))?
                    else {
                        return Err(reject("missing unpack binding"));
                    };
                    if field.name != expected.name
                        || b.declared_type != Some(expected.ty)
                        || b.kind != BindingKind::OwnedUnpack
                        || b.value.is_some()
                    {
                        return Err(reject("unpack binding contract mismatch"));
                    }
                    if direct(self.read, expected.ty)? {
                        let local = LocalValueReference::LexicalBinding(field.binding);
                        if state.insert(local, Slot::owner(false)).is_some() {
                            return Err(reject("duplicate unpack local"));
                        }
                        scoped.push(local);
                    }
                }
                let result = self.eval(body, state, mode, next)?;
                for local in scoped {
                    state.remove(&local);
                }
                result
            }
            ExpressionOperation::Local { value } => {
                if let Mode::ReturnBorrow(expected) = mode {
                    let slot = state.get(&value).ok_or_else(|| {
                        reject("borrowed result requires a live owned local view")
                    })?;
                    if !slot.live || !slot.borrowed {
                        return Err(reject("borrowed result cannot expose a local owner"));
                    }
                    let mut root = value;
                    let mut remaining = state.len();
                    loop {
                        self.read.validation_work()?;
                        if remaining == 0 {
                            return Err(reject("cyclic borrowed result provenance"));
                        }
                        remaining -= 1;
                        let ancestor = state
                            .get(&root)
                            .ok_or_else(|| reject("missing borrowed result ancestor"))?;
                        if !ancestor.live {
                            return Err(reject("borrowed result ancestor is no longer live"));
                        }
                        if let Some(parent) = ancestor.parent {
                            root = parent;
                        } else {
                            break;
                        }
                    }
                    if root != expected {
                        return Err(reject(
                            "borrowed result originates from a different source parameter",
                        ));
                    }
                    // The view crosses the boundary without transferring owning rights.
                    return Ok(false);
                }
                if let Some(slot) = state.get_mut(&value) {
                    if !slot.live
                        || mode == Mode::Unrestricted
                        || ((slot.borrowed || slot.loans != 0) && mode == Mode::Consume)
                    {
                        return Err(reject(
                            "buffer copied, consumed twice, or borrowed parameter consumed/returned",
                        ));
                    }
                    if mode == Mode::Consume {
                        slot.live = false;
                    }
                    return Ok(mode == Mode::Consume);
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
                            Mode::Consume
                        } else {
                            Mode::Unrestricted
                        },
                        next,
                    )?;
                    if acquired {
                        let local = LocalValueReference::LexicalBinding(match b.header.owner {
                            OwnerKey::Binding(id) => id,
                            _ => return Err(reject("wrong binding identity")),
                        });
                        if state.insert(local, Slot::owner(false)).is_some() {
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
                let mut a = fork(self.read, state)?;
                let mut b = fork(self.read, state)?;
                let av = self.eval(when_true, &mut a, mode, next)?;
                let bv = self.eval(when_false, &mut b, mode, next)?;
                if av != bv {
                    return Err(reject("buffer result ownership must agree at branch join"));
                }
                join(self.read, &mut a, &b)?;
                *state = a;
                av
            }
            ExpressionOperation::Sequence { items } => {
                if items.is_empty() && matches!(mode, Mode::ReturnBorrow(_)) {
                    return Err(reject("a borrowed result sequence must return a view"));
                }
                let count = items.len();
                let mut result = false;
                for (i, item) in items.into_iter().enumerate() {
                    result = self.eval(
                        item,
                        state,
                        if i + 1 == count { mode } else { Mode::Consume },
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
                instantiate(self.read, function, &type_arguments, &mut s, self.scope)?;
                admit_signature(self.read, &s)?;
                if s.result_borrow.is_some() {
                    return Err(reject(
                        "borrowed result calls require a lexical borrow-call",
                    ));
                }
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
            ExpressionOperation::Parallel { left, right } => {
                let left = super::parallel::admit_call(self.read, left, self.scope)?;
                let right = super::parallel::admit_call(self.read, right, self.scope)?;
                if left
                    .parameters
                    .iter()
                    .chain(&right.parameters)
                    .any(|parameter| parameter.use_mode == ParameterUse::Borrow)
                    && e.contract_version < contract::SHARE_GRAPH_CONTRACT_VERSION
                {
                    return Err(reject("parallel read captures require Graph 30"));
                }
                // Both inventories prepare in one state, with read custody
                // spanning all later argument trees and the joined child group.
                let mut loans = Vec::new();
                self.parallel_arguments(&left, state, &mut loans, next)?;
                self.parallel_arguments(&right, state, &mut loans, next)?;
                for source in loans.into_iter().rev() {
                    self.loan(source, state, false)?;
                }
                left.result_owned || right.result_owned
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
                let application = super::owned_contract::method_signature(
                    self.read, &witness, contract, method, self.scope,
                )?;
                let signature = application.method;
                let overlay = super::owned_contract::AppliedTypeRead {
                    read: self.read,
                    types: &application.types,
                };
                if signature.result_borrow.is_some() {
                    return Err(reject(
                        "borrowed method results require a lexical borrow-call",
                    ));
                }
                if signature.parameters.len() != arguments.len() {
                    return Err(reject("owned method arity mismatch"));
                }
                let mut uses = BTreeMap::new();
                for (p, a) in signature.parameters.iter().zip(arguments) {
                    if direct(&overlay, p.ty)? {
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
                direct(&overlay, signature.result)?
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
            ExpressionOperation::Field { value, selector } => {
                if let Some(OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::Local { value: local },
                    ..
                })) = self.read.owner(OwnerKey::Expression(value))?
                    && state.contains_key(&local)
                {
                    let ty = match local {
                        LocalValueReference::FunctionParameter(p) => {
                            match self.read.owner(OwnerKey::Parameter(p))? {
                                Some(OwnerRecord::Parameter(p)) => Some(p.ty),
                                _ => None,
                            }
                        }
                        LocalValueReference::LexicalBinding(b) => {
                            match self.read.owner(OwnerKey::Binding(b))? {
                                Some(OwnerRecord::Binding(b)) => b.declared_type,
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                    .ok_or_else(|| reject("product metadata requires an exact typed local"))?;
                    let object = self
                        .read
                        .type_object(ty)?
                        .ok_or_else(|| reject("missing product metadata source type"))?;
                    let (TypeForm::OwnedProduct { fields }, FieldSelector::Structural(name)) =
                        (object.form, selector)
                    else {
                        return Err(reject("only structural product metadata can be read"));
                    };
                    let mut selected = None;
                    for field in fields {
                        self.read.validation_work()?;
                        if field.name == name {
                            selected = Some(field.ty);
                            break;
                        }
                    }
                    let selected =
                        selected.ok_or_else(|| reject("unknown product metadata field"))?;
                    if !super::transfer::ordinary(self.read, selected, self.scope)? {
                        return Err(reject("product field reads cannot expose owned children"));
                    }
                    self.eval(value, state, ParameterUse::Borrow, next)?;
                } else {
                    // In particular, an owned temporary cannot silently become a loan.
                    plain(value, state)?;
                }
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
                let before = fork(self.read, state)?;
                let mut joined = None;
                for arm in arms {
                    let mut branch = fork(self.read, &before)?;
                    let result = self.eval(arm.body, &mut branch, mode, next)?;
                    if let Some((prior, v)) = &mut joined {
                        if *v != result {
                            return Err(reject("buffer match result ownership join"));
                        }
                        join(self.read, prior, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                if let Some((branch, result)) = joined {
                    *state = branch;
                    result
                } else {
                    if matches!(mode, Mode::ReturnBorrow(_)) {
                        return Err(reject(
                            "a borrowed result match requires a nonempty branch inventory",
                        ));
                    }
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
        if owned && mode != Mode::Consume {
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
    if record.header().contract_version < 26 {
        let extended = match record {
            OwnerRecord::Declaration(d) => match &d.payload {
                DeclarationPayload::OwnedContract(c) => {
                    super::owned_contract::requires_parameterized_generation(
                        read,
                        read.package_id(),
                        c,
                    )?
                }
                DeclarationPayload::OwnedImplementation(i) => !i.type_arguments.is_empty(),
                DeclarationPayload::Function(f) => f
                    .implementation_parameters
                    .iter()
                    .any(|p| !p.type_arguments.is_empty()),
                _ => false,
            },
            OwnerRecord::TypeParameter(p) => {
                let OwnerKey::TypeParameter(id) = key else {
                    return Err(reject("invalid contract parameter identity"));
                };
                match read.owner(OwnerKey::Declaration(p.declaration))? {
                    Some(OwnerRecord::Declaration(d)) => matches!(d.payload,
                        DeclarationPayload::OwnedContract(c) if c.self_parameter != id && c.type_parameters.contains(&id)),
                    _ => false,
                }
            }
            _ => false,
        };
        if extended {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_parameterized_contract_generation",
                "parameterized and structured owned contract meaning requires Graph 26",
            ));
        }
    }
    let mut roots = record.type_roots();
    if record.header().contract_version < 25
        && let OwnerRecord::Declaration(declaration) = record
    {
        let parameters = match &declaration.payload {
            DeclarationPayload::Function(function) => function.parameters.as_slice(),
            DeclarationPayload::External(external) => external.parameters.as_slice(),
            _ => &[],
        };
        // Parameter owners have their own headers. A newer parameter cannot
        // grant its enclosing older declaration authority for a new type.
        for parameter in parameters {
            read.validation_work()?;
            let OwnerRecord::Parameter(parameter) =
                owner(read, read.package_id(), OwnerKey::Parameter(*parameter))?
            else {
                return Err(reject("wrong generation-bound signature parameter kind"));
            };
            roots.push(parameter.ty);
        }
    }
    super::owned_product::require_generation(read, roots, record.header().contract_version)?;
    match record {
        OwnerRecord::TypeParameter(p) if p.constraints.has_owned() => {
            let allowed = match read.owner(OwnerKey::Declaration(p.declaration))? {
                Some(OwnerRecord::Declaration(d)) => match d.payload {
                    DeclarationPayload::OwnedContract(c) => {
                        matches!(key, OwnerKey::TypeParameter(id)
                            if c.self_parameter == id || c.type_parameters.contains(&id))
                    }
                    DeclarationPayload::Function(f) => {
                        matches!(key, OwnerKey::TypeParameter(id) if f.type_parameters.contains(&id))
                    }
                    DeclarationPayload::OwnedImplementation(i) => {
                        p.header.contract_version >= 28
                            && matches!(key, OwnerKey::TypeParameter(id)
                                if i.type_parameters.contains(&id))
                    }
                    _ => false,
                },
                _ => false,
            };
            if !allowed {
                return Err(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    "kernel_owned_parameter_owner",
                    "Owned requires an exact graph-function, owned-contract or implementation-scheme parameter",
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
                let OwnerKey::Declaration(declaration) = key else {
                    return Err(reject("implementation requires a declaration identity"));
                };
                super::owned_contract::validate_implementation_at(
                    read,
                    DeclarationReference {
                        package: read.package_id(),
                        declaration,
                    },
                    i,
                )?
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
                            Slot::owner(p.use_mode == ParameterUse::Borrow),
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
                    if let Some(source) = f.result_borrow {
                        Mode::ReturnBorrow(LocalValueReference::FunctionParameter(source))
                    } else if direct(read, f.result)? {
                        Mode::Consume
                    } else {
                        Mode::Unrestricted
                    },
                    0,
                )?;
                if result != (direct(read, f.result)? && f.result_borrow.is_none()) {
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
