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
use super::owned_associated::read_view_admission::{self as views, ScopeAuthority};
use super::owned_associated::{
    OwnedAssociatedAdmission, OwnedTypeOccurrence, TypeParameterContext,
};
use super::owned_contract::{FunctionArguments, ScopeApplication};
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
    if views::direct(&object.form) {
        return Ok(true);
    }
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
/// The separate positive affine view category never confers Owned authority.
pub(crate) fn view_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    associated.step(read)?;
    let object = read
        .type_object(ty)?
        .ok_or_else(|| reject("missing exact view category type"))?;
    if !views::direct(&object.form) {
        return Ok(false);
    }
    associated.validate_occurrence_at(read, &[ty], occurrence)?;
    Ok(true)
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
    result: CallableResult<crate::platform::semantic_id::ParameterId>,
    read_scope_sources: Vec<ReadScopeSource<crate::platform::semantic_id::ParameterId>>,
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
    let (type_parameters, parameters, result, read_scope_sources, pure, generation, external) =
        if d.package == read.package_id() {
            match read.owner(OwnerKey::Declaration(d.declaration))? {
                Some(OwnerRecord::Declaration(r)) => match r.payload {
                    DeclarationPayload::Function(f) => (
                        f.type_parameters,
                        f.parameters,
                        f.result,
                        f.read_scope_sources,
                        matches!(f.effect, FunctionEffect::Pure),
                        r.header.contract_version,
                        false,
                    ),
                    DeclarationPayload::External(f) => (
                        f.type_parameters,
                        f.parameters,
                        CallableResult {
                            ty: f.result,
                            relation: ResultRelation::Value,
                        },
                        Vec::new(),
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
                        f.read_scope_sources,
                        matches!(f.effect, FunctionEffect::Pure),
                        r.header.contract_version,
                        false,
                    ),
                    PackageInterfaceDeclarationPayload::External(f) => (
                        f.type_parameters,
                        f.parameters,
                        CallableResult {
                            ty: f.result,
                            relation: ResultRelation::Value,
                        },
                        Vec::new(),
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
        read_scope_sources,
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

/// Written function and read-scope inputs; live authority remains a later check.
#[derive(Clone, Copy)]
struct ScopedFunctionArguments<'a> {
    application: FunctionArguments<'a>,
    scopes: &'a [ReadScopeArgument],
}

fn instantiate(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
    application_arguments: FunctionArguments<'_>,
    s: &mut Signature,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<BTreeMap<TypeObjectDigest, TypeObject>, Diagnostic> {
    let loans = BTreeSet::new();
    instantiate_with_scopes(
        read,
        d,
        application_arguments,
        ScopeApplication {
            arguments: &[],
            authority: ScopeAuthority {
                binder: views::binder(occurrence.associated),
                lexical: views::lexical(occurrence.associated),
                live_loans: &loans,
            },
        },
        s,
        occurrence,
        associated,
    )
}

fn instantiate_with_scopes(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
    application_arguments: FunctionArguments<'_>,
    scope_application: ScopeApplication<'_>,
    s: &mut Signature,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<BTreeMap<TypeObjectDigest, TypeObject>, Diagnostic> {
    let ScopeApplication {
        arguments: scopes,
        authority,
    } = scope_application;
    let FunctionArguments {
        types: arguments,
        implementations,
        effects: effect_arguments,
        requirements: requirement_arguments,
    } = application_arguments;
    // Definitions and their parameter owners retain independent generations.
    admit_signature(read, s, associated)?;
    if !s.external {
        let mut application = super::owned_contract::admit_function_application_with_scopes(
            read,
            d,
            arguments,
            implementations,
            super::owned_contract::ScopeApplication {
                arguments: scopes,
                authority,
            },
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
        let result_template = effect_applied.substitute(application.signature.result.ty, 0)?;
        let mut applied = super::parallel_types::WitnessAppliedTypes::with_admission(
            &effect_applied,
            application.environment(),
            associated,
        );
        for parameter in &mut s.parameters {
            parameter.ty = applied.substitute(parameter.ty, 0)?;
        }
        s.result.ty = applied.substitute(result_template, 0)?;
        if let ResultRelation::ViewFrom { scope, .. } = &mut s.result.relation {
            *scope = applied.substitute_scope(*scope)?;
        }
        for source in &mut s.read_scope_sources {
            source.scope = applied.substitute_scope(source.scope)?;
        }
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
        || !scopes.is_empty()
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
    s.result.ty = applied.substitute(s.result.ty, &substitutions, 0)?;
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
    let mut read_view = false;
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
        let view = view_at(read, p.ty, occurrence, associated)?;
        if direct_at(read, p.ty, occurrence, associated)? || view {
            if view && memory {
                return Err(reject(
                    "borrowed views must precede the actual Owned suffix",
                ));
            }
            if capability {
                return Err(reject("owned memory must precede the resource suffix"));
            }
            memory |= !view;
            read_view |= view;
            if view && p.use_mode != ParameterUse::Borrow {
                return Err(reject(
                    "read-view parameters require Borrow and cannot transfer handles",
                ));
            }
            if p.use_mode == ParameterUse::Unrestricted || p.resource_requirement.is_some() {
                return Err(reject(
                    "owned parameters require an explicit use and no requirement",
                ));
            }
            if !s.pure
                && p.use_mode == ParameterUse::Borrow
                && (s.generation != contract::GRAPH_CONTRACT_VERSION
                    || p.header.contract_version != contract::GRAPH_CONTRACT_VERSION)
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
    let result = direct_at(read, s.result.ty, s.occurrence, associated)?;
    let view_result = view_at(read, s.result.ty, s.occurrence, associated)?;
    associated.reserve::<(
        crate::platform::semantic_id::ParameterId,
        TypeObjectDigest,
        ParameterUse,
    )>(s.parameters.len())?;
    let mut sources = Vec::with_capacity(s.parameters.len());
    for parameter in &s.parameters {
        associated.step(read)?;
        let OwnerKey::Parameter(id) = parameter.header.owner else {
            return Err(reject("wrong exact callable parameter identity"));
        };
        sources.push((id, parameter.ty, parameter.use_mode));
    }
    for relationship in &s.read_scope_sources {
        associated.step(read)?;
        let mut source = None;
        for parameter in &s.parameters {
            associated.step(read)?;
            if parameter.header.owner == OwnerKey::Parameter(relationship.source) {
                source = Some(parameter);
            }
        }
        let source = source
            .ok_or_else(|| reject("scope source is outside complete exact parameter inventory"))?;
        if source.use_mode != ParameterUse::Borrow
            || (!direct_at(read, source.ty, s.occurrence, associated)?
                && !view_at(read, source.ty, s.occurrence, associated)?)
        {
            return Err(reject(
                "read-scope source requires an exact borrowed Owned or view parameter",
            ));
        }
    }
    views::validate_result(
        read,
        &s.result,
        &sources,
        &s.read_scope_sources,
        s.pure && !s.external,
        s.occurrence,
        associated,
    )?;
    if !result && !view_result && contains_at(read, s.result.ty, s.occurrence, associated)? {
        return Err(reject("only a direct owned result can transfer memory"));
    }
    if s.pure && capability && (memory || read_view || result || view_result) {
        return Err(reject(
            "pure memory helpers cannot carry capability resources",
        ));
    }
    Ok(())
}
#[derive(Clone, Copy, Eq, PartialEq)]
struct Slot {
    birth_order: usize,
    read_view: bool,
    scope: Option<ReadScopeArgument>,
    /// An explicit reference-field edge grants access. Passive view parent
    /// custody never supplies an Owned readable path.
    reference: bool,
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
            birth_order: 0,
            read_view: false,
            scope: None,
            reference: false,
            borrowed,
            live: true,
            parent: None,
            loans: 0,
        }
    }
    fn view(parent: LocalValueReference) -> Self {
        Self {
            birth_order: 0,
            read_view: false,
            scope: None,
            reference: false,
            borrowed: true,
            live: true,
            parent: Some(parent),
            loans: 0,
        }
    }
    fn read_view(
        parent: Option<LocalValueReference>,
        scope: ReadScopeArgument,
        borrowed: bool,
    ) -> Self {
        Self {
            birth_order: 0,
            read_view: true,
            scope: Some(scope),
            reference: false,
            borrowed,
            live: true,
            parent,
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
    ReturnView {
        source: LocalValueReference,
        scope: ReadScopeArgument,
    },
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
    pub(crate) result: CallableResult<ExpressionId>,
    pub(crate) source: ExpressionId,
    pub(crate) source_type: TypeObjectDigest,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
    pub(crate) scope_sources: Vec<ReadScopeSource<ExpressionId>>,
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
    let loans = BTreeSet::new();
    let caller = scope.map(|declaration| DeclarationReference {
        package: read.package_id(),
        declaration,
    });
    borrow_invocation_at_with_scope_authority(
        read,
        call,
        scope,
        ScopeAuthority {
            binder: caller.map(ReadScopeBinder::Declaration),
            lexical: caller,
            live_loans: &loans,
        },
        associated,
    )
}

pub(crate) fn borrow_invocation_at_with_scope_authority(
    read: &(impl ExpressionRead + ?Sized),
    call: ExpressionId,
    scope: Option<crate::platform::semantic_id::DeclarationId>,
    authority: ScopeAuthority<'_>,
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
    let (arguments, parameters, result, source_position, types, scope_sources) =
        match expression.operation {
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
                effect_arguments,
                requirement_arguments,
                read_scope_arguments,
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
                ScopeApplication {
                    arguments: &read_scope_arguments,
                    authority,
                },
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
                read_scope_arguments,
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
                ScopeApplication {
                    arguments: &read_scope_arguments,
                    authority,
                },
                occurrence,
                associated,
            )?,
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
                read_scope_arguments,
            } => {
                let application = super::owned_contract::admit_method_application_with_scopes(
                    read,
                    &witness,
                    contract,
                    method,
                    super::owned_contract::ScopeApplication {
                        arguments: &read_scope_arguments,
                        authority,
                    },
                    occurrence,
                    associated,
                )?;
                let signature = application.method;
                let position = signature.result.relation.source().copied().ok_or_else(|| {
                    reject("borrow-call requires a declared borrowed method result")
                })? as usize;
                let overlay = super::owned_contract::AppliedTypeRead {
                    read,
                    types: &application.types,
                };
                let mut parameters = Vec::new();
                associated.reserve::<(TypeObjectDigest, ParameterUse, bool)>(
                    signature.parameters.len(),
                )?;
                for parameter in &signature.parameters {
                    parameters.push((
                        parameter.ty,
                        parameter.use_mode,
                        direct_at(&overlay, parameter.ty, occurrence, associated)?
                            || view_at(&overlay, parameter.ty, occurrence, associated)?,
                    ));
                }
                associated.reserve::<ReadScopeSource<usize>>(signature.read_scope_sources.len())?;
                let mut scope_sources = Vec::with_capacity(signature.read_scope_sources.len());
                for source in signature.read_scope_sources {
                    associated.step(read)?;
                    scope_sources.push(source.try_map_source(|position| Ok(position as usize))?);
                }
                (
                    arguments,
                    parameters,
                    signature
                        .result
                        .try_map_source(|position| Ok(position as usize))?,
                    position,
                    application.types,
                    scope_sources,
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
    associated.reserve::<ReadScopeSource<ExpressionId>>(scope_sources.len())?;
    let mut actual_scope_sources = Vec::with_capacity(scope_sources.len());
    for source in scope_sources {
        associated.step(read)?;
        actual_scope_sources.push(source.try_map_source(|position| {
            arguments
                .get(position)
                .copied()
                .ok_or_else(|| reject("scope source is outside exact ordered arguments"))
        })?);
    }
    Ok(BorrowInvocation {
        result: result.try_map_source(|position| {
            arguments.get(position).copied().ok_or_else(|| {
                reject("borrowed result source is outside actual argument inventory")
            })
        })?,
        source,
        source_type,
        types,
        scope_sources: actual_scope_sources,
        arguments,
        parameters,
    })
}

type BorrowedApplication = (
    Vec<ExpressionId>,
    Vec<(TypeObjectDigest, ParameterUse, bool)>,
    CallableResult<usize>,
    usize,
    BTreeMap<TypeObjectDigest, TypeObject>,
    Vec<ReadScopeSource<usize>>,
);

fn borrowed_function_application(
    read: &(impl ExpressionRead + ?Sized),
    function: DeclarationReference,
    application_arguments: FunctionArguments<'_>,
    arguments: Vec<ExpressionId>,
    scope_application: ScopeApplication<'_>,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<BorrowedApplication, Diagnostic> {
    let ScopeApplication {
        arguments: scope_arguments,
        authority,
    } = scope_application;
    let mut signature = signature(read, function)?
        .ok_or_else(|| reject("borrow-call requires a named graph function"))?;
    let source = signature
        .result
        .relation
        .source()
        .copied()
        .ok_or_else(|| reject("borrow-call requires a declared borrowed result"))?;
    let mut position = None;
    for (index, parameter) in signature.parameters.iter().enumerate() {
        associated.step(read)?;
        if parameter.header.owner == OwnerKey::Parameter(source) {
            position = Some(index);
        }
    }
    let position =
        position.ok_or_else(|| reject("borrowed invocation source is not a parameter"))?;
    let types = instantiate_with_scopes(
        read,
        function,
        application_arguments,
        ScopeApplication {
            arguments: scope_arguments,
            authority,
        },
        &mut signature,
        occurrence,
        associated,
    )?;
    let overlay = super::owned_contract::AppliedTypeRead {
        read,
        types: &types,
    };
    associated.reserve::<(TypeObjectDigest, ParameterUse, bool)>(signature.parameters.len())?;
    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| {
            Ok((
                parameter.ty,
                parameter.use_mode,
                direct_at(&overlay, parameter.ty, occurrence, associated)?
                    || view_at(&overlay, parameter.ty, occurrence, associated)?,
            ))
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    let result = signature.result.try_map_source(|source| {
        let mut found = None;
        for (position, parameter) in signature.parameters.iter().enumerate() {
            associated.step(read)?;
            if parameter.header.owner == OwnerKey::Parameter(source) {
                if found.is_some() {
                    return Err(reject("duplicate exact invocation source"));
                }
                found = Some(position);
            }
        }
        found.ok_or_else(|| reject("invocation result source is outside exact ordered parameters"))
    })?;
    associated.reserve::<ReadScopeSource<usize>>(signature.read_scope_sources.len())?;
    let mut scope_sources = Vec::with_capacity(signature.read_scope_sources.len());
    for source in signature.read_scope_sources {
        associated.step(read)?;
        scope_sources.push(source.try_map_source(|source| {
            let mut found = None;
            for (position, parameter) in signature.parameters.iter().enumerate() {
                associated.step(read)?;
                if parameter.header.owner == OwnerKey::Parameter(source) {
                    found = Some(position);
                }
            }
            found.ok_or_else(|| reject("scope source is outside exact ordered parameters"))
        })?);
    }
    Ok((
        arguments,
        parameters,
        result,
        position,
        types,
        scope_sources,
    ))
}
fn fork(
    read: &(impl ExpressionRead + ?Sized),
    state: &State,
    associated: &OwnedAssociatedAdmission,
) -> Result<State, Diagnostic> {
    // Admit the complete fixed-size slot inventory before cloning its map nodes.
    associated.reserve::<(LocalValueReference, Slot)>(state.len())?;
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
            || slot.birth_order != other.birth_order
            || slot.read_view != other.read_view
            || slot.scope != other.scope
            || slot.reference != other.reference
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
    /// Immutable dead descriptor ancestry survives an ended local handle. It is
    /// never a source of live access; only active descendants retain custody.
    ended_views: std::cell::RefCell<BTreeMap<LocalValueReference, Slot>>,
    produced_view: std::cell::Cell<Option<Slot>>,
    scope_order: std::cell::RefCell<Vec<crate::platform::semantic_id::BindingId>>,
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
        self.associated.step(self.read)
    }
}
impl<R: ExpressionRead + ?Sized> Check<'_, R> {
    fn live_scope_loans(
        &self,
        state: &State,
    ) -> Result<BTreeSet<crate::platform::semantic_id::BindingId>, Diagnostic> {
        self.associated
            .reserve::<crate::platform::semantic_id::BindingId>(state.len())?;
        let mut loans = BTreeSet::new();
        for (value, slot) in state {
            self.associated.step(self)?;
            if slot.live
                && slot.borrowed
                && let LocalValueReference::LexicalBinding(binding) = value
            {
                loans.insert(*binding);
            }
        }
        self.associated
            .reserve::<crate::platform::semantic_id::BindingId>(self.scope_order.borrow().len())?;
        for binding in self.scope_order.borrow().iter() {
            self.associated.step(self)?;
            loans.insert(*binding);
        }
        Ok(loans)
    }

    fn scope_authority<'a>(
        &self,
        loans: &'a BTreeSet<crate::platform::semantic_id::BindingId>,
    ) -> ScopeAuthority<'a> {
        let callable = self.scope.map(|declaration| DeclarationReference {
            package: self.package_id(),
            declaration,
        });
        ScopeAuthority {
            binder: callable.map(ReadScopeBinder::Declaration),
            lexical: callable,
            live_loans: loans,
        }
    }

    fn ancestry_slot(&self, state: &State, value: LocalValueReference) -> Result<Slot, Diagnostic> {
        self.associated.step(self)?;
        state
            .get(&value)
            .copied()
            .or_else(|| self.ended_views.borrow().get(&value).copied())
            .ok_or_else(|| reject("missing exact read-view custody ancestor"))
    }

    fn path_to(
        &self,
        state: &State,
        mut value: LocalValueReference,
        source: LocalValueReference,
        readable: bool,
    ) -> Result<(), Diagnostic> {
        let mut remaining = state
            .len()
            .checked_add(self.ended_views.borrow().len())
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| reject("view custody ancestry size overflow"))?;
        let mut reference = false;
        loop {
            if remaining == 0 {
                return Err(reject("cyclic view custody ancestry"));
            }
            remaining -= 1;
            let slot = self.ancestry_slot(state, value)?;
            if !slot.live && !slot.read_view && !slot.borrowed {
                return Err(reject("read-view source owner is no longer live"));
            }
            if value == source {
                return Ok(());
            }
            reference |= slot.reference;
            if readable && slot.read_view && !reference {
                return Err(reject(
                    "passive view custody supplies no readable source path",
                ));
            }
            value = slot
                .parent
                .ok_or_else(|| reject("read originates from a different exact selected source"))?;
        }
    }

    fn finish_view(&self, state: &State, slot: Slot, mode: Mode) -> Result<bool, Diagnostic> {
        match mode {
            Mode::Consume => {}
            Mode::ReturnView { source, scope } => {
                if slot.scope != Some(scope) {
                    return Err(reject("returned view changes its declared source scope"));
                }
                self.path_to(
                    state,
                    slot.parent
                        .ok_or_else(|| reject("returned view lacks selected-source custody"))?,
                    source,
                    false,
                )?;
            }
            _ => {
                return Err(reject(
                    "view handles require explicit affine construction, move or reborrow",
                ));
            }
        }
        self.produced_view.set(Some(slot));
        // A source-tied return resolves the affine result inside its exact
        // boundary. Consume still exposes an affine initializer to its caller.
        Ok(mode == Mode::Consume)
    }

    fn end_view_local(
        &self,
        state: &mut State,
        value: LocalValueReference,
    ) -> Result<(), Diagnostic> {
        let Some(mut slot) = state.remove(&value) else {
            return Ok(());
        };
        if !slot.read_view && (!slot.borrowed || slot.parent.is_none()) {
            return Ok(());
        }
        if slot.live
            && !slot.borrowed
            && let Some(parent) = slot.parent
        {
            self.loan(parent, state, false)?;
        }
        slot.live = false;
        self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
        if self
            .ended_views
            .borrow()
            .get(&value)
            .is_some_and(|prior| prior.parent != slot.parent || prior.scope != slot.scope)
        {
            return Err(reject("view ancestry disagrees across complete branches"));
        }
        self.ended_views.borrow_mut().insert(value, slot);
        Ok(())
    }

    fn view_scope(&self, ty: TypeObjectDigest) -> Result<ReadScopeArgument, Diagnostic> {
        self.associated.step(self)?;
        match self
            .type_object(ty)?
            .ok_or_else(|| reject("missing view scope type"))?
            .form
        {
            TypeForm::ReadViewApplied { read_scope, .. }
            | TypeForm::AssociatedReadViewMember { read_scope, .. }
            | TypeForm::AssociatedReadViewProjection { read_scope, .. } => Ok(read_scope),
            _ => Err(reject("local has no positive read-view scope")),
        }
    }

    fn live_source(
        &self,
        value: LocalValueReference,
        state: &State,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let slot = state
            .get(&value)
            .ok_or_else(|| reject("missing exact view source local"))?;
        if !slot.live {
            return Err(reject("view source local handle is no longer live"));
        }
        if let Some(scope) = slot.scope {
            let loans = self.live_scope_loans(state)?;
            self.scope_authority(&loans)
                .validate(self, scope, self.associated)?;
        }
        self.local_type(value)?
            .ok_or_else(|| reject("view source requires an exact type annotation"))
    }

    fn push_scope(
        &self,
        binding: crate::platform::semantic_id::BindingId,
    ) -> Result<(), Diagnostic> {
        self.associated.step(self)?;
        self.associated
            .reserve::<crate::platform::semantic_id::BindingId>(1)?;
        self.scope_order.borrow_mut().push(binding);
        Ok(())
    }

    fn selected_scope(
        &self,
        value: LocalValueReference,
        scope: ReadScopeArgument,
        state: &State,
    ) -> Result<(), Diagnostic> {
        let slot = state
            .get(&value)
            .ok_or_else(|| reject("missing exact selected view source"))?;
        let loans = self.live_scope_loans(state)?;
        let authority = self.scope_authority(&loans);
        authority.validate(self, scope, self.associated)?;
        if slot.read_view || slot.scope.is_some() {
            return views::prove_reborrow_scope(
                self,
                slot.scope
                    .ok_or_else(|| reject("view source lacks its exact scope"))?,
                scope,
                authority,
                &self.scope_order.borrow(),
                self.associated,
            );
        }
        match scope {
            ReadScopeArgument::Lexical { binding, .. } => {
                let mut root = value;
                let mut remaining = state.len() + self.ended_views.borrow().len() + 1;
                let root_slot = loop {
                    if remaining == 0 {
                        return Err(reject("cyclic selected source scope ancestry"));
                    }
                    remaining -= 1;
                    let slot = self.ancestry_slot(state, root)?;
                    if let Some(parent) = slot.parent {
                        root = parent;
                    } else {
                        break slot;
                    }
                };
                let mut position = None;
                for (index, entry) in self.scope_order.borrow().iter().enumerate() {
                    self.associated.step(self)?;
                    if *entry == binding {
                        position = Some(index);
                    }
                }
                if position.is_none_or(|position| position < root_slot.birth_order) {
                    return Err(reject(
                        "local Owned source cannot widen to an older enclosing loan scope",
                    ));
                }
            }
            ReadScopeArgument::Parameter(_) => {
                let declaration = self
                    .scope
                    .ok_or_else(|| reject("view source has no exact callable"))?;
                let Some(OwnerRecord::Declaration(record)) =
                    self.owner(OwnerKey::Declaration(declaration))?
                else {
                    return Err(reject("missing selected-source callable"));
                };
                let DeclarationPayload::Function(function) = record.payload else {
                    return Err(reject("selected scope is not a graph function"));
                };
                let mut found = false;
                for entry in &function.read_scope_sources {
                    self.associated.step(self)?;
                    if entry.scope == scope
                        && self
                            .path_to(
                                state,
                                value,
                                LocalValueReference::FunctionParameter(entry.source),
                                true,
                            )
                            .is_ok()
                    {
                        found = true;
                    }
                }
                if !found {
                    return Err(reject(
                        "Owned source lacks its exact declared scope relationship",
                    ));
                }
            }
        }
        Ok(())
    }

    fn view_layout(
        &self,
        value: LocalValueReference,
        state: &State,
        field: ReadViewFieldReference,
    ) -> Result<ReadViewField, Diagnostic> {
        let ty = self.live_source(value, state)?;
        let occurrence = self.occurrence(contract::GRAPH_CONTRACT_VERSION);
        if !view_at(self, ty, occurrence, self.associated)? {
            return Err(reject("view field source is not a positive ReadView"));
        }
        let caller = self
            .scope
            .map(|declaration| DeclarationReference {
                package: self.package_id(),
                declaration,
            })
            .ok_or_else(|| reject("view field access has no exact callable scope"))?;
        views::require_access(self, field.declaration, caller, self.associated)?;
        let mut layout = views::layout(self, ty, occurrence, self.associated)?;
        if layout.declaration != field.declaration {
            return Err(reject("view field names another exact nominal declaration"));
        }
        self.retain_types(std::mem::take(&mut layout.types))?;
        for entry in layout.fields {
            self.associated.step(self)?;
            if entry.id == field.field {
                return Ok(entry);
            }
        }
        Err(reject(
            "view field is absent from its exact declaration inventory",
        ))
    }

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
        let mut remaining = state.len() + self.ended_views.borrow().len();
        while let Some(local) = current {
            self.validation_work()?;
            if remaining == 0 {
                return Err(reject("cyclic child read provenance"));
            }
            remaining -= 1;
            if !state.contains_key(&local) {
                let mut ended = self.ended_views.borrow_mut();
                let slot = ended
                    .get_mut(&local)
                    .ok_or_else(|| reject("missing child read ancestor"))?;
                if !slot.read_view && !slot.borrowed {
                    return Err(reject("ended ancestry supplies no owning source authority"));
                }
                slot.loans = if acquire {
                    slot.loans.checked_add(1)
                } else {
                    slot.loans.checked_sub(1)
                }
                .ok_or_else(|| reject("ended view custody count mismatch"))?;
                current = slot.parent;
                continue;
            }
            let slot = state
                .get_mut(&local)
                .ok_or_else(|| reject("missing child read ancestor"))?;
            if !slot.live && !slot.read_view {
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
        application: ScopedFunctionArguments<'_>,
        arguments: Vec<ExpressionId>,
        occurrence: OwnedTypeOccurrence,
        state: &mut State,
        depth: usize,
    ) -> Result<bool, Diagnostic> {
        let ScopedFunctionArguments {
            application: application_arguments,
            scopes: scope_arguments,
        } = application;
        let Some(mut signature) = signature(self, function)? else {
            for argument in arguments {
                self.eval(argument, state, ParameterUse::Unrestricted, depth)?;
            }
            return Ok(false);
        };
        let loans = self.live_scope_loans(state)?;
        let types = instantiate_with_scopes(
            self,
            function,
            application_arguments,
            ScopeApplication {
                arguments: scope_arguments,
                authority: self.scope_authority(&loans),
            },
            &mut signature,
            occurrence,
            self.associated,
        )?;
        self.retain_types(types)?;
        if !matches!(signature.result.relation, ResultRelation::Value) {
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
            if direct_at(self, parameter.ty, occurrence, self.associated)?
                || view_at(self, parameter.ty, occurrence, self.associated)?
            {
                let local = self.source_local(argument, parameter.ty, state, depth)?;
                if let Some(prior) = uses.insert(local, parameter.use_mode)
                    && (prior == ParameterUse::Consume
                        || parameter.use_mode == ParameterUse::Consume)
                {
                    return Err(reject("consuming memory argument aliases another argument"));
                }
                self.eval(argument, state, parameter.use_mode, depth)?;
                if parameter.use_mode == ParameterUse::Borrow {
                    for relationship in &signature.read_scope_sources {
                        self.associated.step(self)?;
                        if parameter.header.owner == OwnerKey::Parameter(relationship.source) {
                            self.selected_scope(local, relationship.scope, state)?;
                        }
                    }
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
        direct_at(self, signature.result.ty, occurrence, self.associated)
    }

    fn eval(
        &self,
        id: ExpressionId,
        state: &mut State,
        mode: impl Into<Mode>,
        depth: usize,
    ) -> Result<bool, Diagnostic> {
        let mode = mode.into();
        // Each subtree owns its result proof; a discarded earlier descriptor
        // cannot classify a later ordinary or Owned result as a view.
        self.produced_view.set(None);
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
                    | ExpressionOperation::BorrowReadView { .. }
                    | ExpressionOperation::BorrowReadViewField { .. }
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
            ExpressionOperation::CreateReadView {
                declaration,
                type_arguments,
                read_scope,
                source,
                fields,
            } => {
                views::require_generation(e.contract_version)?;
                let source_type = self.live_source(source, state)?;
                if !direct_at(self, source_type, occurrence, self.associated)?
                    && !view_at(self, source_type, occurrence, self.associated)?
                {
                    return Err(reject(
                        "view construction requires an exact live Owned or view source",
                    ));
                }
                self.selected_scope(source, read_scope, state)?;
                let caller = self
                    .scope
                    .map(|declaration| DeclarationReference {
                        package: self.package_id(),
                        declaration,
                    })
                    .ok_or_else(|| reject("view construction has no defining callable"))?;
                views::require_access(self, declaration, caller, self.associated)?;
                self.associated
                    .reserve::<TypeObjectDigest>(type_arguments.len())?;
                self.associated
                    .reserve::<(TypeObjectDigest, TypeObject)>(1)?;
                let object = TypeObject::new(TypeForm::ReadViewApplied {
                    declaration,
                    arguments: type_arguments,
                    read_scope,
                })?;
                let ty = encode_type_object(&object)?.0;
                self.retain_types(BTreeMap::from([(ty, object)]))?;
                views::validate_nominal_at(self, ty, occurrence, self.associated)?;
                let mut layout = views::layout(self, ty, occurrence, self.associated)?;
                self.retain_types(std::mem::take(&mut layout.types))?;
                if fields.len() != layout.fields.len() {
                    return Err(reject("view construction field inventory is incomplete"));
                }
                self.associated
                    .reserve::<crate::platform::semantic_id::FieldId>(fields.len())?;
                let mut seen = BTreeSet::new();
                // Custody is mandatory even when there are zero reference fields.
                self.loan(source, state, true)?;
                for initializer in fields {
                    self.associated.step(self)?;
                    if !seen.insert(initializer.field) {
                        return Err(reject("duplicate view field initializer"));
                    }
                    let mut selected = None;
                    for field in &layout.fields {
                        self.associated.step(self)?;
                        if field.id == initializer.field {
                            selected = Some(field);
                        }
                    }
                    let field = selected
                        .ok_or_else(|| reject("view initializer names a foreign inline field"))?;
                    match field.kind {
                        ReadViewFieldKind::Ordinary => plain(initializer.value, state)?,
                        ReadViewFieldKind::ReadReference { .. } => {
                            let local =
                                self.source_local(initializer.value, field.ty, state, next)?;
                            self.path_to(state, local, source, true)?;
                        }
                    }
                }
                self.finish_view(
                    state,
                    Slot::read_view(Some(source), read_scope, false),
                    mode,
                )?
            }
            ExpressionOperation::ReborrowReadView { value, read_scope } => {
                views::require_generation(e.contract_version)?;
                let ty = self.live_source(value, state)?;
                if !view_at(self, ty, occurrence, self.associated)? {
                    return Err(reject("reborrow needs a positive ReadView source"));
                }
                self.selected_scope(value, read_scope, state)?;
                self.loan(value, state, true)?;
                self.finish_view(state, Slot::read_view(Some(value), read_scope, false), mode)?
            }
            ExpressionOperation::MoveReadView { value } => {
                views::require_generation(e.contract_version)?;
                self.live_source(value, state)?;
                let slot = state
                    .get_mut(&value)
                    .ok_or_else(|| reject("missing moved view handle"))?;
                if !slot.read_view || slot.borrowed {
                    return Err(reject("a borrowed view parameter cannot transfer a handle"));
                }
                let moved = *slot;
                slot.live = false;
                // Transfer the one existing lease; moving does not acquire one.
                self.finish_view(state, moved, mode)?
            }
            ExpressionOperation::DropReadView { value } => {
                views::require_generation(e.contract_version)?;
                self.live_source(value, state)?;
                let slot = state
                    .get_mut(&value)
                    .ok_or_else(|| reject("missing dropped view handle"))?;
                if !slot.read_view || slot.borrowed {
                    return Err(reject("a borrowed view handle cannot be dropped"));
                }
                let parent = slot.parent;
                slot.live = false;
                if let Some(parent) = parent {
                    self.loan(parent, state, false)?;
                }
                false
            }
            ExpressionOperation::BorrowReadView {
                value,
                binding,
                body,
            } => {
                views::require_generation(e.contract_version)?;
                let source_type = self.live_source(value, state)?;
                if !view_at(self, source_type, occurrence, self.associated)? {
                    return Err(reject("view borrow needs a positive ReadView source"));
                }
                let Some(OwnerRecord::Binding(record)) = self.owner(OwnerKey::Binding(binding))?
                else {
                    return Err(reject("missing view borrow binding"));
                };
                if record.kind != BindingKind::ReadViewBorrow || record.value.is_some() {
                    return Err(reject(
                        "view borrow requires its exact borrowed binding role",
                    ));
                }
                let target_type = record
                    .declared_type
                    .ok_or_else(|| reject("view borrow requires its exact annotation"))?;
                if !view_at(
                    self,
                    target_type,
                    self.occurrence(record.header.contract_version),
                    self.associated,
                )? {
                    return Err(reject(
                        "view borrow annotation has no positive ReadView category",
                    ));
                }
                let target_scope = self.view_scope(target_type)?;
                self.push_scope(binding)?;
                let mut loans = self.live_scope_loans(state)?;
                self.associated
                    .reserve::<crate::platform::semantic_id::BindingId>(1)?;
                loans.insert(binding);
                views::prove_reborrow_scope(
                    self,
                    self.view_scope(source_type)?,
                    target_scope,
                    self.scope_authority(&loans),
                    &self.scope_order.borrow(),
                    self.associated,
                )?;
                self.loan(value, state, true)?;
                let local = LocalValueReference::LexicalBinding(binding);
                self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
                if state
                    .insert(local, Slot::read_view(Some(value), target_scope, true))
                    .is_some()
                {
                    return Err(reject("duplicate read-view borrow binding"));
                }
                let result = self.eval(body, state, mode, next)?;
                self.end_view_local(state, local)?;
                self.loan(value, state, false)?;
                self.scope_order.borrow_mut().pop();
                result
            }
            ExpressionOperation::ReadViewDataField { value, field } => {
                views::require_generation(e.contract_version)?;
                let field = self.view_layout(value, state, field)?;
                if field.kind != ReadViewFieldKind::Ordinary {
                    return Err(reject(
                        "reference field cannot be read as ordinary metadata",
                    ));
                }
                false
            }
            ExpressionOperation::BorrowReadViewField {
                value,
                field,
                binding,
                body,
            } => {
                views::require_generation(e.contract_version)?;
                let field = self.view_layout(value, state, field)?;
                let ReadViewFieldKind::ReadReference { read_scope } = field.kind else {
                    return Err(reject(
                        "ordinary metadata cannot supply a read-reference lease",
                    ));
                };
                let read_view = view_at(self, field.ty, occurrence, self.associated)?;
                let Some(OwnerRecord::Binding(record)) = self.owner(OwnerKey::Binding(binding))?
                else {
                    return Err(reject("missing view reference binding"));
                };
                if record.declared_type != Some(field.ty)
                    || record.value.is_some()
                    || record.kind
                        != if read_view {
                            BindingKind::ReadViewBorrow
                        } else {
                            BindingKind::OwnedBorrow
                        }
                {
                    return Err(reject(
                        "view reference binding category or exact referent type mismatch",
                    ));
                }
                self.loan(value, state, true)?;
                let mut slot = if read_view {
                    Slot::read_view(Some(value), read_scope, true)
                } else {
                    Slot::view(value)
                };
                slot.reference = true;
                slot.scope = Some(read_scope);
                let local = LocalValueReference::LexicalBinding(binding);
                self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
                if state.insert(local, slot).is_some() {
                    return Err(reject("duplicate view reference binding"));
                }
                self.push_scope(binding)?;
                let result = self.eval(body, state, mode, next)?;
                self.end_view_local(state, local)?;
                self.loan(value, state, false)?;
                self.scope_order.borrow_mut().pop();
                result
            }
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                self.push_scope(binding)?;
                let mut live_loans = self.live_scope_loans(state)?;
                self.associated
                    .reserve::<crate::platform::semantic_id::BindingId>(1)?;
                live_loans.insert(binding);
                let mut invocation = borrow_invocation_at_with_scope_authority(
                    self,
                    call,
                    self.scope,
                    self.scope_authority(&live_loans),
                    self.associated,
                )?;
                self.retain_types(std::mem::take(&mut invocation.types))?;
                self.associated.validate_occurrence_at(
                    self,
                    &[invocation.result.ty, invocation.source_type],
                    occurrence,
                )?;
                let returned_view =
                    matches!(invocation.result.relation, ResultRelation::ViewFrom { .. });
                if returned_view {
                    let Some(OwnerRecord::Binding(record)) =
                        self.owner(OwnerKey::Binding(binding))?
                    else {
                        return Err(reject("missing view-result binding"));
                    };
                    if record.kind != BindingKind::ReadView
                        || record.value.is_some()
                        || record.declared_type != Some(invocation.result.ty)
                    {
                        return Err(reject(
                            "view-result requires its exact owning binding contract",
                        ));
                    }
                    views::require_generation(record.header.contract_version)?;
                } else {
                    self.borrow_binding(binding, invocation.result.ty)?;
                }
                // The prospective result scope authenticates invocation metadata,
                // while each argument subtree retains its original live scopes.
                self.scope_order.borrow_mut().pop();
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
                for relationship in &invocation.scope_sources {
                    self.associated.step(self)?;
                    let mut parameter_type = None;
                    for (argument, (ty, _, _)) in
                        invocation.arguments.iter().zip(&invocation.parameters)
                    {
                        self.associated.step(self)?;
                        if *argument == relationship.source {
                            parameter_type = Some(*ty);
                        }
                    }
                    let ty = parameter_type
                        .ok_or_else(|| reject("scope relationship has no exact actual argument"))?;
                    let local = self.source_local(relationship.source, ty, state, next)?;
                    // This prospective scope is used only for exact metadata.
                    self.push_scope(binding)?;
                    let relation = self.selected_scope(local, relationship.scope, state);
                    self.scope_order.borrow_mut().pop();
                    relation?;
                }
                let source =
                    self.source_local(invocation.source, invocation.source_type, state, next)?;
                self.push_scope(binding)?;
                if let ResultRelation::ViewFrom { scope, .. } = invocation.result.relation {
                    self.selected_scope(source, scope, state)?;
                }
                self.loan(source, state, true)?;
                for source in loans.into_iter().rev() {
                    self.loan(source, state, false)?;
                }
                let local = LocalValueReference::LexicalBinding(binding);
                let mut slot =
                    if let ResultRelation::ViewFrom { scope, .. } = invocation.result.relation {
                        Slot::read_view(Some(source), scope, false)
                    } else {
                        Slot::view(source)
                    };
                if !returned_view {
                    slot.reference = state.get(&source).is_some_and(|source| source.read_view);
                }
                if !returned_view {
                    slot.scope = state.get(&source).and_then(|source| source.scope);
                }
                self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
                if state.insert(local, slot).is_some() {
                    return Err(reject("duplicate borrowed invocation binding"));
                }
                let result = self.eval(body, state, mode, next)?;
                if returned_view {
                    self.end_view_local(state, local)?;
                } else {
                    self.end_view_local(state, local)?;
                    self.loan(source, state, false)?;
                }
                self.scope_order.borrow_mut().pop();
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
                let mut slot = Slot::view(source);
                slot.scope = state.get(&source).and_then(|source| source.scope);
                self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
                if state.insert(local, slot).is_some() {
                    return Err(reject("duplicate sequence read binding"));
                }
                self.push_scope(binding)?;
                let result = self.eval(body, state, mode, next)?;
                self.end_view_local(state, local)?;
                self.loan(source, state, false)?;
                self.scope_order.borrow_mut().pop();
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
                let mut slot = Slot::view(source);
                slot.scope = state.get(&source).and_then(|source| source.scope);
                self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
                if state.insert(local, slot).is_some() {
                    return Err(reject("duplicate nominal child read binding"));
                }
                self.push_scope(binding)?;
                let result = self.eval(body, state, mode, next)?;
                self.end_view_local(state, local)?;
                self.loan(source, state, false)?;
                self.scope_order.borrow_mut().pop();
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
                let mut slot = Slot::view(source);
                slot.scope = state.get(&source).and_then(|source| source.scope);
                self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
                if state.insert(local, slot).is_some() {
                    return Err(reject("duplicate child read binding"));
                }
                self.push_scope(binding)?;
                let result = self.eval(body, state, mode, next)?;
                self.end_view_local(state, local)?;
                self.loan(source, state, false)?;
                self.scope_order.borrow_mut().pop();
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
                    let mut branch = fork(self, state, self.associated)?;
                    let local = LocalValueReference::LexicalBinding(arm.binding);
                    if direct_at(self, case.ty, occurrence, self.associated)? {
                        let mut slot = Slot::view(source);
                        slot.scope = state.get(&source).and_then(|source| source.scope);
                        if branch.insert(local, slot).is_some() {
                            return Err(reject("duplicate borrowed match binding"));
                        }
                    }
                    self.push_scope(arm.binding)?;
                    let result = self.eval(arm.body, &mut branch, mode, next)?;
                    self.end_view_local(&mut branch, local)?;
                    self.scope_order.borrow_mut().pop();
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
                    let mut branch = fork(self, state, self.associated)?;
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
                if matches!(mode, Mode::ReturnView { .. }) {
                    return Err(reject(
                        "view results require explicit construction, move or reborrow",
                    ));
                }
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
                        let ancestor = self.ancestry_slot(state, root)?;
                        if !ancestor.live && !ancestor.read_view && !ancestor.borrowed {
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
                    self.path_to(state, value, expected, true)?;
                    // The view crosses the boundary without transferring owning rights.
                    return Ok(false);
                }
                if let Some(slot) = state.get_mut(&value) {
                    if slot.read_view && mode != Mode::Borrow {
                        return Err(reject(
                            "view locals require explicit MoveReadView or ReborrowReadView",
                        ));
                    }
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
                let order_before = self.scope_order.borrow().len();
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
                    let is_view = if let Some(ty) = b.declared_type {
                        view_at(
                            self,
                            ty,
                            self.occurrence(b.header.contract_version),
                            self.associated,
                        )?
                    } else {
                        false
                    };
                    if is_view != (b.kind == BindingKind::ReadView) {
                        return Err(reject(
                            "owning view initializer requires its exact positive binding category",
                        ));
                    }
                    let binding_id = match b.header.owner {
                        OwnerKey::Binding(id) => id,
                        _ => return Err(reject("wrong binding identity")),
                    };
                    let prospective = is_view
                        && matches!(b.declared_type.map(|ty| self.view_scope(ty)).transpose()?, Some(ReadScopeArgument::Lexical { callable, binding }) if Some(callable.declaration) == self.scope && callable.package == self.package_id() && binding == binding_id);
                    if prospective {
                        self.push_scope(binding_id)?;
                    }
                    let is_buffer = is_view
                        || owned_annotation_at(
                            self,
                            b.declared_type,
                            self.occurrence(b.header.contract_version),
                            self.associated,
                        )?;
                    self.produced_view.set(None);
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
                    if is_buffer && !acquired {
                        return Err(reject(
                            "affine initializer did not produce its declared handle category",
                        ));
                    }
                    if acquired {
                        let local = LocalValueReference::LexicalBinding(match b.header.owner {
                            OwnerKey::Binding(id) => id,
                            _ => return Err(reject("wrong binding identity")),
                        });
                        let slot = if is_view {
                            let slot = self.produced_view.take().ok_or_else(|| {
                                reject("view initializer lost its exact source-custody proof")
                            })?;
                            let loans = self.live_scope_loans(state)?;
                            self.scope_authority(&loans).validate(
                                self,
                                slot.scope
                                    .ok_or_else(|| reject("view initializer lost its scope"))?,
                                self.associated,
                            )?;
                            slot
                        } else {
                            let mut slot = Slot::owner(false);
                            slot.birth_order = self.scope_order.borrow().len();
                            slot
                        };
                        self.associated.reserve::<(LocalValueReference, Slot)>(1)?;
                        if state.insert(local, slot).is_some() {
                            return Err(reject("duplicate buffer binding"));
                        }
                        scoped.push(local);
                        if is_view && !prospective {
                            self.push_scope(binding_id)?;
                        }
                    }
                }
                let result = self.eval(body, state, mode, next)?;
                for b in scoped.into_iter().rev() {
                    if state.get(&b).is_some_and(|slot| slot.read_view) {
                        self.end_view_local(state, b)?;
                    } else {
                        state.remove(&b);
                    }
                }
                self.scope_order.borrow_mut().truncate(order_before);
                result
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                plain(condition, state)?;
                let mut a = fork(self, state, self.associated)?;
                let mut b = fork(self, state, self.associated)?;
                self.produced_view.set(None);
                let av = self.eval(when_true, &mut a, mode, next)?;
                let a_view = self.produced_view.take();
                let bv = self.eval(when_false, &mut b, mode, next)?;
                let b_view = self.produced_view.take();
                if a_view != b_view {
                    return Err(reject("view source/scope proof disagrees at branch join"));
                }
                self.produced_view.set(a_view);
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
                    self.produced_view.set(None);
                    result = self.eval(
                        item,
                        state,
                        if i + 1 == count { mode } else { Mode::Consume },
                        next,
                    )?;
                    if i + 1 != count
                        && let Some(slot) = self.produced_view.take()
                        && let Some(parent) = slot.parent
                    {
                        self.loan(parent, state, false)?;
                    }
                }
                result
            }
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
                effect_arguments,
                requirement_arguments,
                read_scope_arguments,
            } => self.function_call(
                function,
                ScopedFunctionArguments {
                    application: FunctionArguments {
                        types: &type_arguments,
                        implementations: &[],
                        effects: &effect_arguments,
                        requirements: &requirement_arguments,
                    },
                    scopes: &read_scope_arguments,
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
                read_scope_arguments,
            } => self.function_call(
                function,
                ScopedFunctionArguments {
                    application: FunctionArguments {
                        types: &type_arguments,
                        implementations: &implementations,
                        effects: &effect_arguments,
                        requirements: &requirement_arguments,
                    },
                    scopes: &read_scope_arguments,
                },
                arguments,
                occurrence,
                state,
                next,
            )?,
            ExpressionOperation::Parallel { left, right } => {
                let loans = self.live_scope_loans(state)?;
                let mut left = super::parallel::admit_call_at_with_scope_authority(
                    self,
                    left,
                    self.scope,
                    self.scope_authority(&loans),
                    self.associated,
                )?;
                self.retain_types(std::mem::take(&mut left.types))?;
                let mut right = super::parallel::admit_call_at_with_scope_authority(
                    self,
                    right,
                    self.scope,
                    self.scope_authority(&loans),
                    self.associated,
                )?;
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
                    && e.contract_version != contract::GRAPH_CONTRACT_VERSION
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
                read_scope_arguments,
            } => {
                let loans = self.live_scope_loans(state)?;
                let mut application = super::owned_contract::admit_method_application_with_scopes(
                    self,
                    &witness,
                    contract,
                    method,
                    super::owned_contract::ScopeApplication {
                        arguments: &read_scope_arguments,
                        authority: self.scope_authority(&loans),
                    },
                    occurrence,
                    self.associated,
                )?;
                self.retain_types(std::mem::take(&mut application.types))?;
                let signature = application.method;
                let overlay = super::owned_contract::AppliedTypeRead {
                    read: self,
                    types: &application.types,
                };
                if !matches!(signature.result.relation, ResultRelation::Value) {
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
                    if direct_at(&overlay, p.ty, occurrence, self.associated)?
                        || view_at(&overlay, p.ty, occurrence, self.associated)?
                    {
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
                direct_at(&overlay, signature.result.ty, occurrence, self.associated)?
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
                    let mut memory = contains_at(self, s.result.ty, s.occurrence, self.associated)?;
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
                let before = fork(self, state, self.associated)?;
                let mut joined = None;
                for arm in arms {
                    let mut branch = fork(self, &before, self.associated)?;
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
        if matches!(mode, Mode::ReturnView { .. }) && self.produced_view.get().is_none() {
            return Err(reject(
                "view result lacks its exact positive source-custody proof",
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
        DeclarationPayload::ReadView(_) => OwnedTypeOccurrence {
            parameters: TypeParameterContext::Authored(reference),
            associated: super::owned_associated::AssociatedTypeContext::ReadViewDeclaration(
                reference,
            ),
            consumer_generation: generation,
        },
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

/// Derive type authority from an exact canonical owner, retaining its generation.
/// Parentless expression/binding records require their containing declaration walk.
pub(crate) fn owner_occurrence(
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
    if record.header().contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH {
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
                "parameterized and structured owned contract meaning requires the current schema epoch",
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
        if matches!(record, OwnerRecord::Declaration(declaration) if matches!(declaration.payload, DeclarationPayload::OwnedContract(_) | DeclarationPayload::OwnedImplementation(_)))
        {
            let OwnerKey::Declaration(declaration) = key else {
                return Err(reject(
                    "aggregate scope requires an exact declaration identity",
                ));
            };
            associated.validate_aggregate_type_at(
                read,
                DeclarationReference {
                    package: read.package_id(),
                    declaration,
                },
                record,
                &roots,
            )?;
        } else {
            associated.validate_occurrence_at(read, &roots, occurrence)?;
        }
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
                        p.header.contract_version == crate::platform::canonical_schema::SCHEMA_EPOCH
                            && matches!(key, OwnerKey::TypeParameter(id)
                                if i.type_parameters.contains(&id))
                    }
                    DeclarationPayload::OwnedRecord {
                        type_parameters, ..
                    } => {
                        matches!(key, OwnerKey::TypeParameter(id) if type_parameters.contains(&id))
                    }
                    DeclarationPayload::ReadView(view) => {
                        matches!(key, OwnerKey::TypeParameter(id) if view.type_parameters.contains(&id))
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
        OwnerRecord::Parameter(p)
            if direct_at(read, p.ty, occurrence, associated)?
                || view_at(read, p.ty, occurrence, associated)? =>
        {
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
                    ended_views: std::cell::RefCell::new(BTreeMap::new()),
                    produced_view: std::cell::Cell::new(None),
                    scope_order: std::cell::RefCell::new(Vec::new()),
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
                super::owned_contract::validate_implementation_at_with_admission(
                    read,
                    DeclarationReference {
                        package: read.package_id(),
                        declaration,
                    },
                    i,
                    associated,
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
                    ended_views: std::cell::RefCell::new(BTreeMap::new()),
                    produced_view: std::cell::Cell::new(None),
                    scope_order: std::cell::RefCell::new(Vec::new()),
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
                    ended_views: std::cell::RefCell::new(BTreeMap::new()),
                    produced_view: std::cell::Cell::new(None),
                    scope_order: std::cell::RefCell::new(Vec::new()),
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
                    ended_views: std::cell::RefCell::new(BTreeMap::new()),
                    produced_view: std::cell::Cell::new(None),
                    scope_order: std::cell::RefCell::new(Vec::new()),
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
            DeclarationPayload::ReadView(_) => {
                let OwnerKey::Declaration(id) = key else {
                    return Err(reject("wrong view declaration identity"));
                };
                views::validate_declaration_at(
                    read,
                    DeclarationReference {
                        package: read.package_id(),
                        declaration: id,
                    },
                    associated,
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
                    let read_view = view_at(read, p.ty, occurrence, associated)?;
                    if direct_at(read, p.ty, occurrence, associated)? || read_view {
                        let OwnerKey::Parameter(id) = p.header.owner else {
                            return Err(reject("wrong parameter identity"));
                        };
                        state.insert(
                            LocalValueReference::FunctionParameter(id),
                            if read_view {
                                let scope = match read
                                    .type_object(p.ty)?
                                    .ok_or_else(|| reject("missing view parameter type"))?
                                    .form
                                {
                                    TypeForm::ReadViewApplied { read_scope, .. }
                                    | TypeForm::AssociatedReadViewMember { read_scope, .. }
                                    | TypeForm::AssociatedReadViewProjection {
                                        read_scope, ..
                                    } => read_scope,
                                    _ => {
                                        return Err(reject(
                                            "view parameter lost its positive category",
                                        ));
                                    }
                                };
                                Slot::read_view(None, scope, true)
                            } else {
                                Slot::owner(p.use_mode == ParameterUse::Borrow)
                            },
                        );
                    }
                }
                let check = Check {
                    read,
                    associated,
                    types: std::cell::RefCell::new(BTreeMap::new()),
                    ended_views: std::cell::RefCell::new(BTreeMap::new()),
                    produced_view: std::cell::Cell::new(None),
                    scope_order: std::cell::RefCell::new(Vec::new()),
                    scope: Some(id),
                };
                let result = check.eval(
                    f.body,
                    &mut state,
                    match f.result.relation {
                        ResultRelation::ReadFrom { source } => {
                            Mode::ReturnBorrow(LocalValueReference::FunctionParameter(source))
                        }
                        ResultRelation::ViewFrom { source, scope } => Mode::ReturnView {
                            source: LocalValueReference::FunctionParameter(source),
                            scope,
                        },
                        ResultRelation::Value
                            if direct_at(read, f.result.ty, occurrence, associated)? =>
                        {
                            Mode::Consume
                        }
                        ResultRelation::Value => Mode::Unrestricted,
                    },
                    0,
                )?;
                let returns_view = matches!(f.result.relation, ResultRelation::ViewFrom { .. });
                if returns_view != check.produced_view.get().is_some()
                    || result
                        != (direct_at(read, f.result.ty, occurrence, associated)?
                            && matches!(f.result.relation, ResultRelation::Value))
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
