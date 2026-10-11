//! Direct owned-memory flow, independent of capability-resource provenance.
#[cfg(test)]
#[path = "memory_associated_tests.rs"]
pub(crate) mod associated_tests;
#[cfg(test)]
#[path = "memory_borrow_result_tests.rs"]
mod borrow_result_tests;
#[cfg(test)]
#[path = "memory_borrow_tests.rs"]
mod borrow_tests;
#[cfg(test)]
#[path = "generalized_sequence_memory_tests.rs"]
mod generalized_sequence_tests;
#[cfg(test)]
#[path = "memory_sequence_tests.rs"]
mod sequence_tests;
#[cfg(test)]
#[path = "memory_work_tests.rs"]
mod work_tests;
use super::infer::ExpressionRead;
use super::owned_associated::{
    OwnedAssociatedAdmission, OwnedTypeOccurrence, TypeParameterContext,
};
use super::owned_contract::FunctionArguments;
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
            TypeForm::Named { .. } | TypeForm::Applied { .. } => {
                super::owned_record::is_owned(read, package, ty)?
            }
            _ => false,
        },
    )
}

/// Category admission retains the exact occurrence and authored reader.
pub(crate) fn direct_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    super::owned_product::validate_owning_type_depth(read, ty)?;
    associated.validate_occurrence_at(read, &[ty], occurrence)?;
    let object = read
        .type_object(ty)?
        .ok_or_else(|| reject("missing memory type"))?;
    match object.form {
        TypeForm::AssociatedContractMember { .. }
        | TypeForm::AssociatedWitnessProjection { .. } => Ok(true),
        TypeForm::TypeParameter { parameter } => {
            let record = match occurrence.parameters {
                TypeParameterContext::Authored(scope) => {
                    super::owned_contract::type_parameter_at(read, scope, parameter)?
                }
                TypeParameterContext::NominalRepresentation(scope) => {
                    super::owned_record::parameter(read, scope, parameter)?
                }
                TypeParameterContext::Closed => {
                    return Err(reject(
                        "memory parameter requires an exact occurrence scope",
                    ));
                }
            };
            if record.constraints.has_owned() {
                associated.validate_owned_parameter_at(read, parameter, occurrence.parameters)?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => direct_in(
            read,
            occurrence
                .parameters
                .scope()
                .map_or(read.package_id(), |scope| scope.package),
            ty,
        ),
    }
}
fn owned_annotation_at(
    read: &(impl ExpressionRead + ?Sized),
    annotation: Option<TypeObjectDigest>,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    // Missing metadata cannot erase the category of an explicit annotation.
    annotation.map_or(Ok(false), |ty| direct_at(read, ty, occurrence, associated))
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
    contains_inner(read, ty, None, &OwnedAssociatedAdmission::new())
}

pub(crate) fn contains_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    super::owned_product::validate_owning_type_depth(read, ty)?;
    associated.validate_occurrence_at(read, &[ty], occurrence)?;
    contains_inner(read, ty, Some(occurrence), associated)
}

fn contains_inner(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: Option<OwnedTypeOccurrence>,
    associated: &OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    let mut pending = vec![(ty, occurrence)];
    let mut seen = BTreeSet::new();
    let mut declarations = BTreeSet::new();
    while let Some((ty, occurrence)) = pending.pop() {
        read.validation_work()?;
        if !seen.insert((ty, occurrence)) {
            continue;
        }
        let object = read
            .type_object(ty)?
            .ok_or_else(|| reject("missing memory containment type"))?;
        let owned = if let Some(occurrence) = occurrence {
            direct_at(read, ty, occurrence, associated)?
        } else {
            direct(read, ty)?
        };
        if owned {
            return Ok(true);
        }
        pending.extend(object.child_types().into_iter().map(|ty| (ty, occurrence)));
        let declaration = match object.form {
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                Some(declaration)
            }
            _ => None,
        };
        if let Some(d) = declaration.filter(|d| declarations.insert(*d)) {
            let property = super::owned_record::property_definition(read, d)?;
            for id in property.fields {
                let field = super::owned_record::field(
                    read,
                    FieldReference {
                        package: d.package,
                        field: id,
                    },
                )?;
                if field.declaration != d.declaration {
                    return Err(reject(
                        "memory property field belongs to another declaration",
                    ));
                }
                pending.push((
                    field.ty,
                    Some(OwnedTypeOccurrence::representation(
                        d,
                        field.header.contract_version,
                    )),
                ));
            }
            for id in property.cases {
                let case = super::owned_record::case(
                    read,
                    CaseReference {
                        package: d.package,
                        case: id,
                    },
                )?;
                if case.declaration != d.declaration {
                    return Err(reject(
                        "memory property case belongs to another declaration",
                    ));
                }
                if let Some(ty) = case.payload {
                    pending.push((
                        ty,
                        Some(OwnedTypeOccurrence::representation(
                            d,
                            case.header.contract_version,
                        )),
                    ));
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
    occurrence: OwnedTypeOccurrence,
    external: bool,
    materialized: bool,
}
fn signature(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
) -> Result<Option<Signature>, Diagnostic> {
    let (type_parameters, parameters, result, result_borrow, pure, generation, external) =
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
                        false,
                    ),
                    DeclarationPayload::External(f) => (
                        f.type_parameters,
                        f.parameters,
                        f.result,
                        None,
                        true,
                        r.header.contract_version,
                        true,
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
                        false,
                    ),
                    PackageInterfaceDeclarationPayload::External(f) => (
                        f.type_parameters,
                        f.parameters,
                        f.result,
                        None,
                        true,
                        r.header.contract_version,
                        true,
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
        occurrence: OwnedTypeOccurrence {
            parameters: TypeParameterContext::Authored(d),
            associated: if external {
                super::owned_associated::AssociatedTypeContext::Forbidden
            } else {
                super::owned_associated::AssociatedTypeContext::Lexical(d)
            },
            consumer_generation: generation,
        },
        external,
        materialized: false,
    }))
}

fn instantiate(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
    application_arguments: FunctionArguments<'_>,
    s: &mut Signature,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<BTreeMap<TypeObjectDigest, TypeObject>, Diagnostic> {
    let FunctionArguments {
        types: arguments,
        implementations,
        effects: effect_arguments,
        requirements: requirement_arguments,
    } = application_arguments;
    // Definitions and their parameter owners retain independent generations.
    admit_signature(read, s, associated)?;
    if !s.external {
        let mut application = super::owned_contract::admit_function_application(
            read,
            d,
            arguments,
            implementations,
            occurrence,
            associated,
        )?;
        s.parameters = std::mem::take(&mut application.parameters);
        let overlay = super::owned_contract::AppliedTypeRead {
            read,
            types: &application.types,
        };
        if application.signature.effect_parameters.len() != effect_arguments.len()
            || application.signature.requirement_parameters.len() != requirement_arguments.len()
        {
            return Err(reject(
                "memory call requires exact effect and requirement argument arities",
            ));
        }
        associated.reserve::<(EffectParameterReference, EffectRow)>(effect_arguments.len())?;
        for row in effect_arguments {
            associated.reserve::<RequirementOperand>(row.requirements.len())?;
            associated.reserve::<EffectParameterReference>(row.parameters.len())?;
        }
        associated.reserve::<(RequirementParameterReference, RequirementOperand)>(
            requirement_arguments.len(),
        )?;
        let effects = application
            .signature
            .effect_parameters
            .iter()
            .zip(effect_arguments)
            .map(|(parameter, row)| {
                (
                    EffectParameterReference {
                        package: d.package,
                        parameter: *parameter,
                    },
                    row.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let requirements = application
            .signature
            .requirement_parameters
            .iter()
            .zip(requirement_arguments)
            .map(|(parameter, operand)| {
                (
                    RequirementParameterReference {
                        package: d.package,
                        parameter: *parameter,
                    },
                    *operand,
                )
            })
            .collect::<RequirementSubstitution>();
        // Rewrite source templates before inserting caller actuals. Rewriting
        // afterwards would capture identically numbered caller effect formals.
        let mut effect_applied =
            super::parallel_types::EffectAppliedTypes::new(&overlay, &effects, &requirements);
        for parameter in &mut s.parameters {
            parameter.ty = effect_applied.substitute(parameter.ty, 0)?;
        }
        let result_template = effect_applied.substitute(application.signature.result, 0)?;
        let mut applied = super::parallel_types::WitnessAppliedTypes::with_admission(
            &effect_applied,
            application.environment(),
            associated,
        );
        for parameter in &mut s.parameters {
            parameter.ty = applied.substitute(parameter.ty, 0)?;
        }
        s.result = applied.substitute(result_template, 0)?;
        let witness_types = applied.into_types();
        let effect_types = effect_applied.into_types();
        let mut types = BTreeMap::new();
        let mut storage = super::owned_record::LayoutBudget::default();
        for source in [witness_types, effect_types, application.types] {
            merge_types(read, &mut storage, &mut types, source)?;
        }
        s.occurrence = occurrence;
        s.materialized = true;
        let overlay = super::owned_contract::AppliedTypeRead {
            read,
            types: &types,
        };
        admit_signature(&overlay, s, associated)?;
        return Ok(types);
    }
    // External declarations have ordinary formals but no lexical witness scope.
    if !implementations.is_empty()
        || !effect_arguments.is_empty()
        || !requirement_arguments.is_empty()
    {
        return Err(reject(
            "external memory calls cannot carry implementation witnesses",
        ));
    }
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
        super::owned_contract::validate_type_argument_constraints_at(
            read,
            p.constraints,
            *ty,
            occurrence,
            associated,
        )?;
        substitutions.insert(*id, *ty);
    }
    let mut applied = super::parallel_types::AppliedTypes::new(read);
    for p in &mut s.parameters {
        p.ty = applied.substitute(p.ty, &substitutions, 0)?;
    }
    s.result = applied.substitute(s.result, &substitutions, 0)?;
    s.occurrence = occurrence;
    s.materialized = true;
    let types = applied.into_types();
    let overlay = super::owned_contract::AppliedTypeRead {
        read,
        types: &types,
    };
    admit_signature(&overlay, s, associated)?;
    Ok(types)
}

fn merge_types(
    read: &(impl ExpressionRead + ?Sized),
    storage: &mut super::owned_record::LayoutBudget,
    target: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    source: BTreeMap<TypeObjectDigest, TypeObject>,
) -> Result<(), Diagnostic> {
    // Admission precedes each merged map node, and duplicate digests retain one
    // exact canonical object rather than overriding another derivation phase.
    for (digest, object) in &source {
        read.validation_work()?;
        if target
            .get(digest)
            .is_some_and(|existing| existing != object)
            || read
                .type_object(*digest)?
                .is_some_and(|existing| existing != *object)
        {
            return Err(reject(
                "derived memory type digest disagrees across application overlays",
            ));
        }
    }
    storage.extend_types(target, source)
}

fn admit_signature(
    read: &(impl ExpressionRead + ?Sized),
    s: &Signature,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    let mut memory = false;
    let mut capability = false;
    for p in &s.parameters {
        let occurrence = s.occurrence;
        if !s.materialized {
            associated.validate_occurrence_at(
                read,
                &[p.ty],
                OwnedTypeOccurrence {
                    consumer_generation: p.header.contract_version,
                    ..occurrence
                },
            )?;
        }
        if direct_at(read, p.ty, occurrence, associated)? {
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
            if contains_at(read, p.ty, occurrence, associated)? {
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
    let result = direct_at(read, s.result, s.occurrence, associated)?;
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
            || !direct_at(read, parameter.ty, s.occurrence, associated)?
            || parameter.use_mode != ParameterUse::Borrow
        {
            return Err(reject(
                "borrowed results require a pure function, direct owned result and exact borrowed source",
            ));
        }
    }
    if !result && contains_at(read, s.result, s.occurrence, associated)? {
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
    borrow_invocation_at(read, call, scope, &OwnedAssociatedAdmission::new())
}

pub(crate) fn borrow_invocation_at(
    read: &(impl ExpressionRead + ?Sized),
    call: ExpressionId,
    scope: Option<crate::platform::semantic_id::DeclarationId>,
    associated: &OwnedAssociatedAdmission,
) -> Result<BorrowInvocation, Diagnostic> {
    let Some(OwnerRecord::Expression(expression)) = read.owner(OwnerKey::Expression(call))? else {
        return Err(reject("missing borrowed invocation"));
    };
    super::owned_contract::require_expression_generation(read, &expression)?;
    let occurrence = OwnedTypeOccurrence::authored(
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
        expression.contract_version,
    );
    let (arguments, parameters, result, source_position, types) = match expression.operation {
        ExpressionOperation::Call {
            function,
            type_arguments,
            arguments,
            effect_arguments,
            requirement_arguments,
        } => borrowed_function_application(
            read,
            function,
            FunctionArguments {
                types: &type_arguments,
                implementations: &[],
                effects: &effect_arguments,
                requirements: &requirement_arguments,
            },
            arguments,
            occurrence,
            associated,
        )?,
        ExpressionOperation::ImplementationCall {
            function,
            type_arguments,
            arguments,
            effect_arguments,
            requirement_arguments,
            implementations,
        } => borrowed_function_application(
            read,
            function,
            FunctionArguments {
                types: &type_arguments,
                implementations: &implementations,
                effects: &effect_arguments,
                requirements: &requirement_arguments,
            },
            arguments,
            occurrence,
            associated,
        )?,
        ExpressionOperation::MethodCall {
            witness,
            contract,
            method,
            arguments,
        } => {
            let application = super::owned_contract::admit_method_application(
                read, &witness, contract, method, occurrence, associated,
            )?;
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
                    direct_at(&overlay, parameter.ty, occurrence, associated)?,
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

type BorrowedApplication = (
    Vec<ExpressionId>,
    Vec<(TypeObjectDigest, ParameterUse, bool)>,
    TypeObjectDigest,
    usize,
    BTreeMap<TypeObjectDigest, TypeObject>,
);

fn borrowed_function_application(
    read: &(impl ExpressionRead + ?Sized),
    function: DeclarationReference,
    application_arguments: FunctionArguments<'_>,
    arguments: Vec<ExpressionId>,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<BorrowedApplication, Diagnostic> {
    let mut signature = signature(read, function)?
        .ok_or_else(|| reject("borrow-call requires a named graph function"))?;
    let source = signature
        .result_borrow
        .ok_or_else(|| reject("borrow-call requires a declared borrowed result"))?;
    let position = signature
        .parameters
        .iter()
        .position(|parameter| parameter.header.owner == OwnerKey::Parameter(source))
        .ok_or_else(|| reject("borrowed invocation source is not a parameter"))?;
    let types = instantiate(
        read,
        function,
        application_arguments,
        &mut signature,
        occurrence,
        associated,
    )?;
    let overlay = super::owned_contract::AppliedTypeRead {
        read,
        types: &types,
    };
    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| {
            Ok((
                parameter.ty,
                parameter.use_mode,
                direct_at(&overlay, parameter.ty, occurrence, associated)?,
            ))
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    Ok((arguments, parameters, signature.result, position, types))
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
    associated: &'a OwnedAssociatedAdmission,
    types: std::cell::RefCell<BTreeMap<TypeObjectDigest, TypeObject>>,
}
impl<R: ExpressionRead + ?Sized> ExpressionRead for Check<'_, R> {
    fn package_id(&self) -> PackageId {
        self.read.package_id()
    }
    fn owner(&self, key: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.read.owner(key)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        if let Some(object) = self.types.borrow().get(&ty) {
            return Ok(Some(object.clone()));
        }
        self.read.type_object(ty)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        key: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_interface_owner(package, key)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_dependency(package)
    }
    fn package_representation_owner(
        &self,
        package: PackageId,
        key: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_representation_owner(package, key)
    }
    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_representation_dependency(package)
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.read.validation_work()
    }
}
impl<R: ExpressionRead + ?Sized> Check<'_, R> {
    fn occurrence(&self, generation: u16) -> OwnedTypeOccurrence {
        OwnedTypeOccurrence::authored(
            self.scope.map(|declaration| DeclarationReference {
                package: self.package_id(),
                declaration,
            }),
            generation,
        )
    }

    fn retain_types(
        &self,
        types: BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<(), Diagnostic> {
        for (digest, object) in types {
            self.validation_work()?;
            if let Some(existing) = self.type_object(digest)? {
                if existing != object {
                    return Err(reject(
                        "derived memory type digest disagrees with retained object",
                    ));
                }
                continue;
            }
            if self.types.borrow().len() >= contract::MAXIMUM_VALIDATION_WORK {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_memory_types",
                    "derived memory type inventory exceeds finite preparation",
                ));
            }
            self.associated
                .reserve::<(TypeObjectDigest, TypeObject)>(1)?;
            self.associated
                .reserve::<TypeObjectDigest>(object.child_type_count())?;
            match &object.form {
                TypeForm::StructuralRecord { fields }
                | TypeForm::OwnedProduct { fields }
                | TypeForm::OwnedChoice { cases: fields } => {
                    self.associated
                        .reserve::<StructuralTypeField>(fields.len())?;
                    for field in fields {
                        self.associated.reserve_bytes(field.name.as_str().len())?;
                    }
                }
                TypeForm::TaskFunction { effect, .. } => {
                    self.associated
                        .reserve::<RequirementOperand>(effect.requirements.len())?;
                    self.associated
                        .reserve::<EffectParameterReference>(effect.parameters.len())?;
                }
                _ => {}
            }
            self.types.borrow_mut().insert(digest, object);
        }
        Ok(())
    }

    fn record_layout(
        &self,
        ty: TypeObjectDigest,
        occurrence: OwnedTypeOccurrence,
    ) -> Result<super::owned_record::AppliedOwnedRecord, Diagnostic> {
        let scope = self.scope.map(|declaration| DeclarationReference {
            package: self.package_id(),
            declaration,
        });
        super::owned_record::require_access(self, ty, scope)?;
        super::owned_record::validate_application_at(self, ty, occurrence, self.associated)?;
        let mut layout = super::owned_record::layout(self, ty)?;
        self.retain_types(std::mem::take(&mut layout.types))?;
        Ok(layout)
    }
    fn sequence_item(
        &self,
        ty: TypeObjectDigest,
        occurrence: OwnedTypeOccurrence,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        super::owned_product::validate_at(self, ty, occurrence, self.associated)?;
        let TypeForm::OwnedSequence { item } = self
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
        })) = self.owner(OwnerKey::Expression(expression))?
        else {
            return Err(reject("child reads require an exact local source"));
        };
        let actual = self.local_type(value)?;
        if actual != Some(ty) || !state.contains_key(&value) {
            return Err(reject(
                "child read source requires an exact typed live owner or view",
            ));
        }
        self.eval(expression, state, ParameterUse::Borrow, depth)?;
        Ok(value)
    }

    fn local_type(
        &self,
        value: LocalValueReference,
    ) -> Result<Option<TypeObjectDigest>, Diagnostic> {
        let (ty, generation) = match value {
            LocalValueReference::FunctionParameter(parameter) => {
                match self.owner(OwnerKey::Parameter(parameter))? {
                    Some(OwnerRecord::Parameter(record)) => {
                        (Some(record.ty), record.header.contract_version)
                    }
                    _ => return Err(reject("missing exact memory parameter")),
                }
            }
            LocalValueReference::LexicalBinding(binding) => {
                match self.owner(OwnerKey::Binding(binding))? {
                    Some(OwnerRecord::Binding(record)) => {
                        (record.declared_type, record.header.contract_version)
                    }
                    _ => return Err(reject("missing exact memory binding")),
                }
            }
            _ => return Ok(None),
        };
        if let Some(ty) = ty {
            self.associated
                .validate_occurrence_at(self, &[ty], self.occurrence(generation))?;
        }
        Ok(ty)
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
            self.validation_work()?;
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
        let Some(OwnerRecord::Binding(record)) = self.owner(OwnerKey::Binding(binding))? else {
            return Err(reject("missing child read binding"));
        };
        if record.kind != BindingKind::OwnedBorrow
            || record.value.is_some()
            || record.declared_type != Some(ty)
        {
            return Err(reject("child read binding contract mismatch"));
        }
        self.associated.validate_occurrence_at(
            self,
            &[ty],
            self.occurrence(record.header.contract_version),
        )?;
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
            self.validation_work()?;
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

    fn function_call(
        &self,
        function: DeclarationReference,
        application_arguments: FunctionArguments<'_>,
        arguments: Vec<ExpressionId>,
        occurrence: OwnedTypeOccurrence,
        state: &mut State,
        depth: usize,
    ) -> Result<bool, Diagnostic> {
        let Some(mut signature) = signature(self, function)? else {
            for argument in arguments {
                self.eval(argument, state, ParameterUse::Unrestricted, depth)?;
            }
            return Ok(false);
        };
        let types = instantiate(
            self,
            function,
            application_arguments,
            &mut signature,
            occurrence,
            self.associated,
        )?;
        self.retain_types(types)?;
        if signature.result_borrow.is_some() {
            return Err(reject(
                "borrowed result calls require a lexical borrow-call",
            ));
        }
        if signature.parameters.len() != arguments.len() {
            return Err(reject("memory call arity mismatch"));
        }
        let mut uses = BTreeMap::new();
        let mut loans = Vec::new();
        for (parameter, argument) in signature.parameters.iter().zip(arguments) {
            if direct_at(self, parameter.ty, occurrence, self.associated)? {
                let local = self.source_local(argument, parameter.ty, state, depth)?;
                if let Some(prior) = uses.insert(local, parameter.use_mode)
                    && (prior == ParameterUse::Consume
                        || parameter.use_mode == ParameterUse::Consume)
                {
                    return Err(reject("consuming memory argument aliases another argument"));
                }
                self.eval(argument, state, parameter.use_mode, depth)?;
                if parameter.use_mode == ParameterUse::Borrow {
                    self.loan(local, state, true)?;
                    loans.push(local);
                }
            } else {
                self.eval(argument, state, ParameterUse::Unrestricted, depth)?;
            }
        }
        for source in loans.into_iter().rev() {
            self.loan(source, state, false)?;
        }
        direct_at(self, signature.result, occurrence, self.associated)
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
        self.validation_work()?;
        let Some(OwnerRecord::Expression(e)) = self.owner(OwnerKey::Expression(id))? else {
            return Err(reject("missing memory expression"));
        };
        super::owned_contract::require_expression_generation(self, &e)?;
        let occurrence = self.occurrence(e.contract_version);
        self.associated.validate_occurrence_at(
            self,
            &OwnerRecord::Expression(e.clone()).type_roots(),
            occurrence,
        )?;
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
                    | ExpressionOperation::BorrowOwnedRecordField { .. }
                    | ExpressionOperation::MatchBorrowedOwned { .. }
                    | ExpressionOperation::MatchOwned { .. }
                    | ExpressionOperation::UnpackOwned { .. }
                    | ExpressionOperation::UnpackOwnedRecord { .. }
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
                let mut invocation = borrow_invocation_at(self, call, self.scope, self.associated)?;
                self.retain_types(std::mem::take(&mut invocation.types))?;
                self.associated.validate_occurrence_at(
                    self,
                    &[invocation.result, invocation.source_type],
                    occurrence,
                )?;
                self.borrow_binding(binding, invocation.result)?;
                let mut uses = BTreeMap::new();
                let mut loans = Vec::new();
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
                        if *use_mode == ParameterUse::Borrow {
                            self.loan(local, state, true)?;
                            loans.push(local);
                        }
                    } else {
                        plain(*argument, state)?;
                    }
                }
                let source =
                    self.source_local(invocation.source, invocation.source_type, state, next)?;
                self.loan(source, state, true)?;
                for source in loans.into_iter().rev() {
                    self.loan(source, state, false)?;
                }
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
                self.sequence_item(sequence_type, occurrence)?;
                true
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                self.sequence_item(sequence_type, occurrence)?;
                self.source_local(source, sequence_type, state, next)?;
                false
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                let item = self.sequence_item(sequence_type, occurrence)?;
                // Authored evaluation order consumes the element before the sequence.
                if direct_at(self, item, occurrence, self.associated)? {
                    self.consume_local(value, item, state, next)?;
                } else {
                    plain(value, state)?;
                }
                self.consume_local(source, sequence_type, state, next)?;
                true
            }
            ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index,
            } => {
                let item = self.sequence_item(sequence_type, occurrence)?;
                if direct_at(self, item, occurrence, self.associated)? {
                    return Err(reject("sequence get requires an ordinary element type"));
                }
                plain(index, state)?;
                self.source_local(source, sequence_type, state, next)?;
                false
            }
            ExpressionOperation::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            } => {
                let item = self.sequence_item(sequence_type, occurrence)?;
                super::owned_product::validate_at(self, result_type, occurrence, self.associated)?;
                plain(index, state)?;
                if direct_at(self, item, occurrence, self.associated)? {
                    self.consume_local(value, item, state, next)?;
                } else {
                    plain(value, state)?;
                }
                self.consume_local(source, sequence_type, state, next)?;
                true
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                self.sequence_item(sequence_type, occurrence)?;
                super::owned_product::validate_at(self, result_type, occurrence, self.associated)?;
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
                let item = self.sequence_item(sequence_type, occurrence)?;
                if !direct_at(self, item, occurrence, self.associated)? {
                    return Err(reject(
                        "owned item borrowing requires an owned element type",
                    ));
                }
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
            ExpressionOperation::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            } => {
                let layout = self.record_layout(record_type, occurrence)?;
                let (_, selected) = layout
                    .fields
                    .iter()
                    .find(|(reference, _)| *reference == field)
                    .ok_or_else(|| reject("unknown nominal owned child field"))?;
                let overlay = super::owned_contract::AppliedTypeRead {
                    read: self,
                    types: &layout.types,
                };
                if !direct_at(&overlay, *selected, occurrence, self.associated)? {
                    return Err(reject("nominal child read requires an Owned field"));
                }
                self.borrow_binding(binding, *selected)?;
                let source = self.source_local(source, record_type, state, next)?;
                self.loan(source, state, true)?;
                let local = LocalValueReference::LexicalBinding(binding);
                if state.insert(local, Slot::view(source)).is_some() {
                    return Err(reject("duplicate nominal child read binding"));
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
                super::owned_product::validate_at(self, product_type, occurrence, self.associated)?;
                let TypeForm::OwnedProduct { fields } = self
                    .type_object(product_type)?
                    .ok_or_else(|| reject("missing child read product type"))?
                    .form
                else {
                    return Err(reject("field read requires an owned product"));
                };
                let mut selected = None;
                for candidate in fields {
                    self.validation_work()?;
                    if candidate.name == field {
                        selected = Some(candidate.ty);
                    }
                }
                let selected = selected.ok_or_else(|| reject("unknown child read field"))?;
                if !direct_at(self, selected, occurrence, self.associated)? {
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
                super::owned_product::validate_at(self, choice_type, occurrence, self.associated)?;
                let TypeForm::OwnedChoice { cases } = self
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
                    self.validation_work()?;
                    if arm.name != case.name {
                        return Err(reject("borrowed match case mismatch"));
                    }
                    self.borrow_binding(arm.binding, case.ty)?;
                    let mut branch = fork(self, state)?;
                    let local = LocalValueReference::LexicalBinding(arm.binding);
                    if direct_at(self, case.ty, occurrence, self.associated)?
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
                        join(self, prior, &branch)?;
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
                super::owned_product::validate_at(self, choice_type, occurrence, self.associated)?;
                let TypeForm::OwnedChoice { cases } = self
                    .type_object(choice_type)?
                    .ok_or_else(|| reject("missing owned choice type"))?
                    .form
                else {
                    return Err(reject("choice construction requires an owned choice"));
                };
                let mut payload = None;
                for candidate in cases {
                    self.validation_work()?;
                    if candidate.name == case {
                        payload = Some(candidate.ty);
                    }
                }
                let payload = payload.ok_or_else(|| reject("unknown owned choice case"))?;
                if direct_at(self, payload, occurrence, self.associated)? {
                    let Some(OwnerRecord::Expression(ExpressionRecord {
                        operation: ExpressionOperation::Local { value: local },
                        ..
                    })) = self.owner(OwnerKey::Expression(value))?
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
                super::owned_product::validate_at(self, choice_type, occurrence, self.associated)?;
                let Some(OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::Local { value },
                    ..
                })) = self.owner(OwnerKey::Expression(source))?
                else {
                    return Err(reject("owned match requires an exact local operand"));
                };
                if !state.contains_key(&value)
                    || !self.eval(source, state, ParameterUse::Consume, next)?
                {
                    return Err(reject("owned match requires a live owner"));
                }
                let TypeForm::OwnedChoice { cases } = self
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
                    self.validation_work()?;
                    let Some(OwnerRecord::Binding(b)) =
                        self.owner(OwnerKey::Binding(arm.binding))?
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
                    self.associated.validate_occurrence_at(
                        self,
                        &[case.ty],
                        self.occurrence(b.header.contract_version),
                    )?;
                    let mut branch = fork(self, state)?;
                    let local = LocalValueReference::LexicalBinding(arm.binding);
                    if direct_at(self, case.ty, occurrence, self.associated)?
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
                        join(self, prior, &branch)?;
                    } else {
                        joined = Some((branch, result));
                    }
                }
                let (joined, result) = joined.ok_or_else(|| reject("empty owned match"))?;
                *state = joined;
                result
            }
            ExpressionOperation::PackOwnedRecord {
                record_type,
                fields,
            } => {
                let layout = self.record_layout(record_type, occurrence)?;
                if fields.len() != layout.fields.len() {
                    return Err(reject("incomplete nominal owned construction"));
                }
                let overlay = super::owned_contract::AppliedTypeRead {
                    read: self,
                    types: &layout.types,
                };
                let mut seen = BTreeSet::new();
                for field in fields {
                    let (_, ty) = layout
                        .fields
                        .iter()
                        .find(|(reference, _)| *reference == field.field)
                        .ok_or_else(|| reject("unknown nominal owned construction field"))?;
                    if !seen.insert(field.field) {
                        return Err(reject("duplicate nominal owned construction field"));
                    }
                    if direct_at(&overlay, *ty, occurrence, self.associated)? {
                        self.consume_local(field.value, *ty, state, next)?;
                    } else {
                        plain(field.value, state)?;
                    }
                }
                true
            }
            ExpressionOperation::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            } => {
                let layout = self.record_layout(record_type, occurrence)?;
                self.consume_local(source, record_type, state, next)?;
                if fields.len() != layout.fields.len() {
                    return Err(reject("incomplete nominal owned decomposition"));
                }
                let overlay = super::owned_contract::AppliedTypeRead {
                    read: self,
                    types: &layout.types,
                };
                let mut scoped = Vec::new();
                for (field, (reference, ty)) in fields.iter().zip(layout.fields.iter()) {
                    let Some(OwnerRecord::Binding(binding)) =
                        self.owner(OwnerKey::Binding(field.binding))?
                    else {
                        return Err(reject("missing nominal decomposition binding"));
                    };
                    if field.field != *reference
                        || binding.declared_type != Some(*ty)
                        || binding.kind != BindingKind::OwnedUnpack
                        || binding.value.is_some()
                    {
                        return Err(reject("nominal decomposition binding contract mismatch"));
                    }
                    self.associated.validate_occurrence_at(
                        self,
                        &[*ty],
                        self.occurrence(binding.header.contract_version),
                    )?;
                    if direct_at(&overlay, *ty, occurrence, self.associated)? {
                        let local = LocalValueReference::LexicalBinding(field.binding);
                        if state.insert(local, Slot::owner(false)).is_some() {
                            return Err(reject("duplicate nominal decomposition local"));
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
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                super::owned_product::validate_at(self, product_type, occurrence, self.associated)?;
                let TypeForm::OwnedProduct { fields: contract } = self
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
                    if direct_at(self, expected.ty, occurrence, self.associated)? {
                        let Some(OwnerRecord::Expression(ExpressionRecord {
                            operation: ExpressionOperation::Local { value },
                            ..
                        })) = self.owner(OwnerKey::Expression(field.value))?
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
                super::owned_product::validate_at(self, product_type, occurrence, self.associated)?;
                let Some(OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::Local { value },
                    ..
                })) = self.owner(OwnerKey::Expression(source))?
                else {
                    return Err(reject("unpack requires an exact local operand"));
                };
                if !state.contains_key(&value)
                    || !self.eval(source, state, ParameterUse::Consume, next)?
                {
                    return Err(reject("unpack requires a live owner"));
                }
                let TypeForm::OwnedProduct { fields: expected } = self
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
                        self.owner(OwnerKey::Binding(field.binding))?
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
                    self.associated.validate_occurrence_at(
                        self,
                        &[expected.ty],
                        self.occurrence(b.header.contract_version),
                    )?;
                    if direct_at(self, expected.ty, occurrence, self.associated)? {
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
                if let Some(ty) = self.local_type(value)? {
                    // A Local has no explicit roots, yet consumes its owner's type.
                    direct_at(self, ty, occurrence, self.associated)?;
                }
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
                        self.validation_work()?;
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
                    let Some(OwnerRecord::Binding(b)) = self.owner(OwnerKey::Binding(b))? else {
                        return Err(reject("missing memory binding"));
                    };
                    let value = b
                        .value
                        .ok_or_else(|| reject("missing memory binding initializer"))?;
                    if let Some(ty) = b.declared_type {
                        self.associated
                            .validate_occurrence_at(self, &[ty], occurrence)?;
                    }
                    // Annotations are semantic contracts, not ownership certificates.
                    let is_buffer = owned_annotation_at(
                        self,
                        b.declared_type,
                        self.occurrence(b.header.contract_version),
                        self.associated,
                    )?;
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
                let mut a = fork(self, state)?;
                let mut b = fork(self, state)?;
                let av = self.eval(when_true, &mut a, mode, next)?;
                let bv = self.eval(when_false, &mut b, mode, next)?;
                if av != bv {
                    return Err(reject("buffer result ownership must agree at branch join"));
                }
                join(self, &mut a, &b)?;
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
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
                effect_arguments,
                requirement_arguments,
            } => self.function_call(
                function,
                FunctionArguments {
                    types: &type_arguments,
                    implementations: &[],
                    effects: &effect_arguments,
                    requirements: &requirement_arguments,
                },
                arguments,
                occurrence,
                state,
                next,
            )?,
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                arguments,
                effect_arguments,
                requirement_arguments,
                implementations,
            } => self.function_call(
                function,
                FunctionArguments {
                    types: &type_arguments,
                    implementations: &implementations,
                    effects: &effect_arguments,
                    requirements: &requirement_arguments,
                },
                arguments,
                occurrence,
                state,
                next,
            )?,
            ExpressionOperation::Parallel { left, right } => {
                let mut left =
                    super::parallel::admit_call_at(self, left, self.scope, self.associated)?;
                self.retain_types(std::mem::take(&mut left.types))?;
                let mut right =
                    super::parallel::admit_call_at(self, right, self.scope, self.associated)?;
                self.retain_types(std::mem::take(&mut right.types))?;
                let consumed = left
                    .parameters
                    .iter()
                    .chain(&right.parameters)
                    .map(|parameter| parameter.ty)
                    .chain([left.result, right.result])
                    .collect::<Vec<_>>();
                self.associated
                    .validate_occurrence_at(self, &consumed, occurrence)?;
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
                let mut application = super::owned_contract::admit_method_application(
                    self,
                    &witness,
                    contract,
                    method,
                    occurrence,
                    self.associated,
                )?;
                self.retain_types(std::mem::take(&mut application.types))?;
                let signature = application.method;
                let overlay = super::owned_contract::AppliedTypeRead {
                    read: self,
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
                let mut loans = Vec::new();
                for (p, a) in signature.parameters.iter().zip(arguments) {
                    if direct_at(&overlay, p.ty, occurrence, self.associated)? {
                        let value = self.source_local(a, p.ty, state, next)?;
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
                        if p.use_mode == ParameterUse::Borrow {
                            self.loan(value, state, true)?;
                            loans.push(value);
                        }
                    } else {
                        plain(a, state)?;
                    }
                }
                for source in loans.into_iter().rev() {
                    self.loan(source, state, false)?;
                }
                direct_at(&overlay, signature.result, occurrence, self.associated)?
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                for t in type_arguments {
                    if contains_at(self, t, occurrence, self.associated)? {
                        return Err(reject("buffer generic descriptor argument"));
                    }
                }
                if let Some(s) = signature(self, function)? {
                    admit_signature(self, &s, self.associated)?;
                    let mut memory = contains_at(self, s.result, s.occurrence, self.associated)?;
                    for p in s.parameters {
                        memory |= contains_at(self, p.ty, s.occurrence, self.associated)?;
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
                })) = self.owner(OwnerKey::Expression(value))?
                    && state.contains_key(&local)
                {
                    let ty = self
                        .local_type(local)?
                        .ok_or_else(|| reject("product metadata requires an exact typed local"))?;
                    self.associated
                        .validate_occurrence_at(self, &[ty], occurrence)?;
                    let object = self
                        .type_object(ty)?
                        .ok_or_else(|| reject("missing product metadata source type"))?;
                    let selected = match (object.form, selector) {
                        (TypeForm::OwnedProduct { fields }, FieldSelector::Structural(name)) => {
                            let ty = fields
                                .into_iter()
                                .find(|field| field.name == name)
                                .map(|field| field.ty)
                                .ok_or_else(|| reject("unknown product metadata field"))?;
                            if direct_at(self, ty, occurrence, self.associated)?
                                || !super::transfer::ordinary(self, ty, self.scope)?
                            {
                                return Err(reject(
                                    "product field reads cannot expose owned children",
                                ));
                            }
                            ty
                        }
                        (
                            TypeForm::Named { .. } | TypeForm::Applied { .. },
                            FieldSelector::Nominal(reference),
                        ) => {
                            let layout = self.record_layout(ty, occurrence)?;
                            let (_, selected) = layout
                                .fields
                                .iter()
                                .find(|(field, _)| *field == reference)
                                .ok_or_else(|| reject("unknown nominal metadata field"))?;
                            let overlay = super::owned_contract::AppliedTypeRead {
                                read: self,
                                types: &layout.types,
                            };
                            if direct_at(&overlay, *selected, occurrence, self.associated)?
                                || !super::transfer::ordinary(&overlay, *selected, self.scope)?
                            {
                                return Err(reject(
                                    "nominal field reads cannot expose owned children",
                                ));
                            }
                            *selected
                        }
                        _ => {
                            return Err(reject(
                                "metadata field must belong to the exact owned source",
                            ));
                        }
                    };
                    let _ = selected;
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
                let before = fork(self, state)?;
                let mut joined = None;
                for arm in arms {
                    let mut branch = fork(self, &before)?;
                    let result = self.eval(arm.body, &mut branch, mode, next)?;
                    if let Some((prior, v)) = &mut joined {
                        if *v != result {
                            return Err(reject("buffer match result ownership join"));
                        }
                        join(self, prior, &branch)?;
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
fn declaration_occurrence(
    reference: DeclarationReference,
    payload: &DeclarationPayload,
    generation: u16,
) -> OwnedTypeOccurrence {
    match payload {
        DeclarationPayload::Function(_) | DeclarationPayload::OwnedImplementation(_) => {
            OwnedTypeOccurrence::authored(Some(reference), generation)
        }
        DeclarationPayload::OwnedContract(_) => {
            OwnedTypeOccurrence::contract(reference, generation)
        }
        DeclarationPayload::Record { .. }
        | DeclarationPayload::Variant { .. }
        | DeclarationPayload::OwnedRecord { .. } => {
            OwnedTypeOccurrence::representation(reference, generation)
        }
        DeclarationPayload::External(_) => OwnedTypeOccurrence {
            parameters: TypeParameterContext::Authored(reference),
            associated: super::owned_associated::AssociatedTypeContext::Forbidden,
            consumer_generation: generation,
        },
        _ => OwnedTypeOccurrence::authored(None, generation),
    }
}

fn owner_occurrence(
    read: &(impl ExpressionRead + ?Sized),
    key: OwnerKey,
    record: &OwnerRecord,
) -> Result<Option<OwnedTypeOccurrence>, Diagnostic> {
    let generation = record.header().contract_version;
    let reference = |declaration| DeclarationReference {
        package: read.package_id(),
        declaration,
    };
    let declaration = match record {
        OwnerRecord::Declaration(declaration) => {
            let OwnerKey::Declaration(id) = key else {
                return Err(reject("wrong memory declaration identity"));
            };
            return Ok(Some(declaration_occurrence(
                reference(id),
                &declaration.payload,
                generation,
            )));
        }
        OwnerRecord::TypeParameter(parameter) => Some(parameter.declaration),
        OwnerRecord::Parameter(parameter) => match parameter.parent {
            ParameterParent::Function(id) => Some(id),
            ParameterParent::Operation(_) => None,
        },
        OwnerRecord::Field(field) => {
            return Ok(Some(OwnedTypeOccurrence::representation(
                reference(field.declaration),
                generation,
            )));
        }
        OwnerRecord::Case(case) => {
            return Ok(Some(OwnedTypeOccurrence::representation(
                reference(case.declaration),
                generation,
            )));
        }
        // These records acquire their lexical context only through the exact
        // containing declaration walk; neither carries a parent declaration.
        OwnerRecord::Expression(_) | OwnerRecord::Binding(_) => return Ok(None),
        _ => None,
    };
    if let Some(id) = declaration {
        let Some(OwnerRecord::Declaration(declaration)) = read.owner(OwnerKey::Declaration(id))?
        else {
            return Err(reject("missing exact memory occurrence declaration"));
        };
        Ok(Some(declaration_occurrence(
            reference(id),
            &declaration.payload,
            generation,
        )))
    } else {
        Ok(Some(OwnedTypeOccurrence::authored(None, generation)))
    }
}

pub(crate) fn validate_owner(
    read: &(impl ExpressionRead + ?Sized),
    key: OwnerKey,
    record: &OwnerRecord,
) -> Result<(), Diagnostic> {
    validate_owner_at(read, key, record, &OwnedAssociatedAdmission::new())
}

pub(crate) fn validate_owner_at(
    read: &(impl ExpressionRead + ?Sized),
    key: OwnerKey,
    record: &OwnerRecord,
    associated: &OwnedAssociatedAdmission,
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
    if let OwnerRecord::Declaration(declaration) = record {
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
    let generation = record.header().contract_version;
    let occurrence = owner_occurrence(read, key, record)?;
    if let Some(occurrence) = occurrence {
        for ty in &roots {
            super::owned_product::validate_owning_type_depth(read, *ty)?;
        }
        associated.validate_occurrence_at(read, &roots, occurrence)?;
    }
    super::owned_product::require_generation(read, roots, generation)?;
    let occurrence = occurrence.unwrap_or_else(|| OwnedTypeOccurrence::authored(None, generation));
    match record {
        OwnerRecord::Expression(expression) => {
            super::owned_contract::require_expression_generation(read, expression)?;
        }
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
                    DeclarationPayload::OwnedRecord {
                        type_parameters, ..
                    } => {
                        matches!(key, OwnerKey::TypeParameter(id) if type_parameters.contains(&id))
                    }
                    _ => false,
                },
                _ => false,
            };
            if !allowed {
                return Err(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    "kernel_owned_parameter_owner",
                    "Owned requires an exact graph-function, owned-contract, implementation-scheme or owned-record parameter",
                ));
            }
        }
        OwnerRecord::Parameter(p) if direct_at(read, p.ty, occurrence, associated)? => {
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
            admit_signature(read, &s, associated)?;
        }
        OwnerRecord::Port(p) => {
            if contains_at(read, p.function_type, occurrence, associated)? {
                return Err(reject("buffer adapter port is forbidden"));
            }
            if let PortImplementation::Expression(e) = p.implementation {
                Check {
                    read,
                    associated,
                    types: std::cell::RefCell::new(BTreeMap::new()),
                    scope: Some(p.declaration),
                }
                .eval(e, &mut State::new(), ParameterUse::Unrestricted, 0)?;
            }
        }
        OwnerRecord::Field(f) if contains_at(read, f.ty, occurrence, associated)? => {
            let allowed = matches!(read.owner(OwnerKey::Declaration(f.declaration))?,
                Some(OwnerRecord::Declaration(record)) if matches!(record.payload,
                    DeclarationPayload::OwnedRecord { ref fields, .. } if matches!(key, OwnerKey::Field(id) if fields.contains(&id))));
            if !allowed {
                return Err(reject(
                    "owned fields require an exact owned-record representation",
                ));
            }
        }
        OwnerRecord::Case(c)
            if c.payload
                .map(|t| contains_at(read, t, occurrence, associated))
                .transpose()?
                .unwrap_or(false) =>
        {
            return Err(reject("buffer variant payload is forbidden"));
        }
        OwnerRecord::Operation(o) if contains_at(read, o.result, occurrence, associated)? => {
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
            DeclarationPayload::OwnedRecord { .. } => {
                let OwnerKey::Declaration(declaration) = key else {
                    return Err(reject("owned record requires a declaration identity"));
                };
                super::owned_record::validate_definition(
                    read,
                    DeclarationReference {
                        package: read.package_id(),
                        declaration,
                    },
                )?;
            }
            DeclarationPayload::Constant { ty, value } => {
                if contains_at(read, *ty, occurrence, associated)? {
                    return Err(reject("buffer constants are forbidden"));
                }
                Check {
                    read,
                    associated,
                    types: std::cell::RefCell::new(BTreeMap::new()),
                    scope: Some(match key {
                        OwnerKey::Declaration(id) => id,
                        _ => return Err(reject("constant requires an exact declaration")),
                    }),
                }
                .eval(*value, &mut State::new(), ParameterUse::Unrestricted, 0)?;
            }
            DeclarationPayload::Test {
                actual, expected, ..
            } => {
                Check {
                    read,
                    associated,
                    types: std::cell::RefCell::new(BTreeMap::new()),
                    scope: Some(match key {
                        OwnerKey::Declaration(id) => id,
                        _ => return Err(reject("test requires an exact declaration")),
                    }),
                }
                .eval(*actual, &mut State::new(), ParameterUse::Unrestricted, 0)?;
                Check {
                    read,
                    associated,
                    types: std::cell::RefCell::new(BTreeMap::new()),
                    scope: Some(match key {
                        OwnerKey::Declaration(id) => id,
                        _ => return Err(reject("test requires an exact declaration")),
                    }),
                }
                .eval(
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
                admit_signature(read, &s, associated)?;
                let mut state = State::new();
                for p in &s.parameters {
                    if direct_at(read, p.ty, occurrence, associated)? {
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
                    associated,
                    types: std::cell::RefCell::new(BTreeMap::new()),
                    scope: Some(id),
                }
                .eval(
                    f.body,
                    &mut state,
                    if let Some(source) = f.result_borrow {
                        Mode::ReturnBorrow(LocalValueReference::FunctionParameter(source))
                    } else if direct_at(read, f.result, occurrence, associated)? {
                        Mode::Consume
                    } else {
                        Mode::Unrestricted
                    },
                    0,
                )?;
                if result
                    != (direct_at(read, f.result, occurrence, associated)?
                        && f.result_borrow.is_none())
                {
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
                admit_signature(read, &s, associated)?;
                let mut memory = direct_at(read, f.result, occurrence, associated)?;
                for p in &s.parameters {
                    memory |= direct_at(read, p.ty, occurrence, associated)?;
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
