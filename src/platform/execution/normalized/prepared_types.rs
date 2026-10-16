//! Disposable type/layout closure of concrete compiled roots and their static substitutions.
//! Composite substitutions need not already occur as stored type objects. They
//! are derived here without changing type identities or the artifact encoding.

use super::prepare::{
    ClosedTypeEnvironment, NormalizedCode, NormalizedEntryPoint, NormalizedFunctionBody,
    NormalizedInstruction, NormalizedProgram, NormalizedRecordLayout,
    NormalizedTransactionRequirement, NormalizedVariantLayout,
};
use super::value::FunctionIndex;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ReadScopeArgument, ReadScopeBinder, ReadScopeReference, ReadViewFieldKind,
};
use crate::platform::kernel::{TypeForm, TypeObject, TypeObjectDigest, encode_type_object};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::{BTreeMap, BTreeSet};

#[path = "prepared_implementations.rs"]
mod implementations;
#[cfg(test)]
pub(super) use implementations::associated_tests::prepared_associated_model;
#[path = "prepared_product_depth.rs"]
mod product_depth;
#[path = "prepared_shared_code.rs"]
mod shared_code;
#[path = "prepared_type_lookup.rs"]
mod type_lookup;
pub(super) use type_lookup::PreparedTypeLookup;

#[cfg(test)]
#[path = "prepared_associated_type_tests.rs"]
mod associated_tests;

#[cfg(test)]
#[path = "prepared_read_view_tests.rs"]
mod read_view_tests;

type Context = (FunctionIndex, Vec<TypeObjectDigest>);
pub(super) type ScopeBindings = BTreeMap<ReadScopeReference, ReadScopeArgument>;
/// Source selection is borrowed from an exact closed DAG. Only executable roots
/// carry a function index; intermediate implementation mappings cannot populate
/// a different caller application's lookup.
struct TypeApplication<'a> {
    bindings: &'a BTreeMap<TypeParameterId, TypeObjectDigest>,
    associated: Option<&'a ClosedTypeEnvironment<'a>>,
    function: Option<FunctionIndex>,
    scopes: Option<&'a ScopeBindings>,
}

impl<'a> TypeApplication<'a> {
    fn ordinary(bindings: &'a BTreeMap<TypeParameterId, TypeObjectDigest>) -> Self {
        Self {
            bindings,
            associated: None,
            function: None,
            scopes: None,
        }
    }
}

type EffectBindings =
    BTreeMap<crate::platform::kernel::EffectParameterReference, crate::platform::kernel::EffectRow>;
type EffectApplication = (
    FunctionIndex,
    Vec<crate::platform::kernel::EffectRow>,
    Vec<crate::platform::kernel::RequirementReference>,
);
const MAXIMUM_WORK: usize = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
// Finite preparation storage, independent of invocation-lifetime allocation accounting.
const MAXIMUM_METADATA_BYTES: usize = 256 * 1024 * 1024;

pub(super) struct Budget<'a> {
    steps: usize,
    bytes: usize,
    control: &'a crate::platform::execution::ExecutionControl,
    type_lookup: type_lookup::TypeLookupBuilder,
}

impl<'a> Budget<'a> {
    pub(super) fn new(control: &'a crate::platform::execution::ExecutionControl) -> Self {
        Self {
            steps: 0,
            bytes: 0,
            control,
            type_lookup: type_lookup::TypeLookupBuilder::default(),
        }
    }
    #[cfg(test)]
    pub(super) fn work_for_test(&self) -> (usize, usize) {
        (self.steps, self.bytes)
    }

    #[cfg(test)]
    pub(super) fn with_work_for_test(
        control: &'a crate::platform::execution::ExecutionControl,
        steps: usize,
        bytes: usize,
    ) -> Self {
        assert!(steps <= MAXIMUM_WORK && bytes <= MAXIMUM_METADATA_BYTES);
        Self {
            steps,
            bytes,
            control,
            type_lookup: type_lookup::TypeLookupBuilder::default(),
        }
    }

    pub(super) fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        reserve_metadata(&mut self.bytes, count, std::mem::size_of::<T>())
    }

    /// Advisory analyses may decline storage without consuming the ledger or
    /// turning their own capacity limit into rejection of admitted meaning.
    pub(super) fn try_reserve<T>(&mut self, count: usize) -> bool {
        let Some(total) = count
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|added| self.bytes.checked_add(added))
            .filter(|total| *total <= MAXIMUM_METADATA_BYTES)
        else {
            return false;
        };
        self.bytes = total;
        true
    }

    pub(super) fn step(&mut self) -> Result<(), Diagnostic> {
        step(self)
    }

    fn node<T>(&mut self) -> Result<(), Diagnostic> {
        self.reserve::<T>(1)?;
        self.reserve::<usize>(3)
    }

    fn cloned_effect(
        &mut self,
        effect: &crate::platform::kernel::FunctionEffect,
    ) -> Result<(), Diagnostic> {
        if let crate::platform::kernel::FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = effect
        {
            self.reserve::<crate::platform::kernel::RequirementOperand>(requirements.len())?;
            self.reserve::<crate::platform::kernel::EffectParameterReference>(
                effect_parameters.len(),
            )?;
        }
        Ok(())
    }

    fn cloned_type(&mut self, object: &TypeObject) -> Result<(), Diagnostic> {
        // Charge variable storage before cloning the canonical expression of a type.
        match &object.form {
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                self.reserve::<crate::platform::kernel::StructuralTypeField>(fields.len())?;
                for field in fields {
                    step(self)?;
                    self.reserve::<u8>(field.name.as_str().len())?;
                }
            }
            TypeForm::Applied { arguments, .. } | TypeForm::ReadViewApplied { arguments, .. } => {
                self.reserve::<TypeObjectDigest>(arguments.len())?
            }
            TypeForm::TaskFunction {
                parameters, effect, ..
            } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?;
                self.reserve::<crate::platform::kernel::RequirementOperand>(
                    effect.requirements.len(),
                )?;
                self.reserve::<crate::platform::kernel::EffectParameterReference>(
                    effect.parameters.len(),
                )?;
            }
            TypeForm::Function { parameters, .. } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?
            }
            _ => {}
        }
        Ok(())
    }
}

fn step(work: &mut Budget<'_>) -> Result<(), Diagnostic> {
    work.control
        .check()
        .map_err(|error| Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message))?;
    work.steps = work
        .steps
        .checked_add(1)
        .filter(|n| *n <= MAXIMUM_WORK)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "normalized_instantiation_work",
                "prepared type closure exceeds its bounded work inventory",
            )
        })?;
    Ok(())
}

fn reserve_metadata(bytes: &mut usize, count: usize, size: usize) -> Result<(), Diagnostic> {
    *bytes = count
        .checked_mul(size)
        .and_then(|added| bytes.checked_add(added))
        .filter(|total| *total <= MAXIMUM_METADATA_BYTES)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "normalized_instantiation_storage",
                "nominal layout metadata exceeds the existing allocation limit",
            )
        })?;
    Ok(())
}

#[cfg(test)]
pub(super) fn complete(program: &mut NormalizedProgram) -> Result<(), Diagnostic> {
    complete_controlled(
        program,
        &crate::platform::execution::ExecutionControl::uncancelled(),
    )
}

pub(super) fn complete_with_implementations(
    program: &mut NormalizedProgram,
    units: &BTreeMap<
        (
            crate::platform::kernel::PackageId,
            crate::platform::kernel::OwnerKey,
        ),
        crate::platform::compiler::CompilationUnit,
    >,
    runtime_owners: &BTreeMap<
        (
            crate::platform::kernel::PackageId,
            crate::platform::kernel::OwnerKey,
        ),
        crate::platform::kernel::OwnerRecord,
    >,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    // Raw signature categories and all subsequent closure stages share this
    // operation's ledger and control; no phase creates a replacement allowance.
    close_effect_applications(program, work)?;
    implementations::close(program, units, runtime_owners, work)?;
    complete_budgeted(program, work, true)
}

pub(super) fn share_code(
    program: &mut NormalizedProgram,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), Diagnostic> {
    let mut work = Budget {
        steps: usize::try_from(program.work.type_derivation_steps).map_err(|_| missing())?,
        bytes: usize::try_from(program.work.type_metadata_bytes).map_err(|_| missing())?,
        control,
        type_lookup: type_lookup::TypeLookupBuilder::default(),
    };
    shared_code::share(program, &mut work)?;
    program.capture_proof_bytes = work.bytes;
    program.work.type_derivation_steps = work.steps as u64;
    program.work.type_metadata_bytes = work.bytes as u64;
    Ok(())
}

#[cfg(test)]
pub(super) fn complete_controlled(
    program: &mut NormalizedProgram,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), Diagnostic> {
    let mut work = Budget::new(control);
    close_effect_applications(program, &mut work)?;
    complete_budgeted(program, &mut work, false)
}

fn complete_budgeted(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
    body_scopes_applied: bool,
) -> Result<(), Diagnostic> {
    let mut pending = BTreeSet::new();
    // Derive dispatch only after each exact effect application and its callees are closed.
    super::prepare::derive_tail_dispatch(std::sync::Arc::make_mut(&mut program.functions), work)?;
    for (index, function) in program.functions.iter().enumerate() {
        if function.type_parameters.is_empty()
            && function.effect_parameters.is_empty()
            && function.requirement_parameters.is_empty()
            && (function.read_scope_parameters.is_empty()
                || function.read_scope_parameters.len() == function.read_scope_arguments.len())
        {
            step(work)?;
            work.node::<Context>()?;
            pending.insert((
                FunctionIndex(
                    u32::try_from(index).map_err(|_| missing())?,
                    program.value_origin,
                ),
                Vec::new(),
            ));
        }
    }
    let empty = BTreeMap::new();
    let standalone = TypeApplication::ordinary(&empty);
    let standalone_scopes = body_scopes_applied.then(ScopeBindings::new);
    for test in program.tests.values() {
        calls_at(
            &test.actual,
            (&standalone, standalone_scopes.as_ref()),
            &mut program.types,
            &mut pending,
            work,
            &program.records,
            &program.variants,
        )?;
        calls_at(
            &test.expected,
            (&standalone, standalone_scopes.as_ref()),
            &mut program.types,
            &mut pending,
            work,
            &program.records,
            &program.variants,
        )?;
    }
    for port in program.ports.iter() {
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &port.entry
        {
            calls_at(
                code,
                (&standalone, standalone_scopes.as_ref()),
                &mut program.types,
                &mut pending,
                work,
                &program.records,
                &program.variants,
            )?;
        }
    }
    let mut visited = BTreeSet::new();
    while let Some((index, arguments)) = pending.pop_first() {
        step(work)?;
        work.node::<Context>()?;
        work.reserve::<TypeObjectDigest>(arguments.len())?;
        if !visited.insert((index, arguments.clone())) {
            continue;
        }
        if index.1 != program.value_origin {
            return Err(missing());
        }
        let function = program
            .functions
            .get(index.0 as usize)
            .ok_or_else(missing)?;
        if function.type_parameters.len() != arguments.len() {
            return Err(missing());
        }
        if !function.type_arguments.is_empty() && function.type_arguments.as_ref() != arguments {
            return Err(missing());
        }
        for _ in &arguments {
            step(work)?;
            work.node::<(TypeParameterId, TypeObjectDigest)>()?;
        }
        let bindings = function
            .type_parameters
            .iter()
            .copied()
            .zip(arguments)
            .collect();
        let environment = ClosedTypeEnvironment::for_function(function)?;
        let scopes = function_scope_bindings(function, work)?;
        let applied_scopes = if body_scopes_applied {
            Some(applied_scope_identities(&scopes, work)?)
        } else {
            None
        };
        let application_types = TypeApplication {
            bindings: &bindings,
            associated: Some(&environment),
            function: Some(index),
            scopes: Some(&scopes),
        };
        substitute_in(
            &mut program.types,
            function.result.ty,
            &application_types,
            0,
            work,
        )?;
        for parameter in function.parameters.iter() {
            substitute_in(
                &mut program.types,
                parameter.ty,
                &application_types,
                0,
                work,
            )?;
        }
        if function.implementation_parameters.len() != function.implementation_arguments.len() {
            return Err(missing());
        }
        for (parameter, application) in function
            .implementation_parameters
            .iter()
            .zip(function.implementation_arguments.iter())
        {
            if parameter.contract != application.contract
                || substitute_in(
                    &mut program.types,
                    parameter.self_type,
                    &application_types,
                    0,
                    work,
                )? != application.self_type
                || parameter.type_arguments.len() != application.type_arguments.len()
            {
                return Err(missing());
            }
            // Include every contract argument, even when no method mentions it.
            for (expected, actual) in parameter
                .type_arguments
                .iter()
                .zip(application.type_arguments.iter())
            {
                if substitute_in(&mut program.types, *expected, &application_types, 0, work)?
                    != *actual
                {
                    return Err(missing());
                }
            }
        }
        if let NormalizedFunctionBody::Code(code) = &function.body {
            calls_at(
                code,
                (&application_types, applied_scopes.as_ref()),
                &mut program.types,
                &mut pending,
                work,
                &program.records,
                &program.variants,
            )?;
        }
    }
    complete_nominal_layouts(program, work)?;
    loop {
        let before = program.types.len();
        complete_read_view_layouts(program, work)?;
        if program.types.len() == before {
            break;
        }
        // Instantiated private reference fields can introduce an Owned nominal
        // application absent from every callable signature and authored opcode.
        complete_nominal_layouts(program, work)?;
    }
    program.owned_record_declarations.clear();
    for layout in program.records.iter().filter(|layout| layout.owned) {
        step(work)?;
        if !program
            .owned_record_declarations
            .contains(&layout.declaration)
        {
            work.node::<crate::platform::kernel::DeclarationReference>()?;
            program.owned_record_declarations.insert(layout.declaration);
        }
    }
    product_depth::validate_program(program, work)?;
    program.work.type_objects = program.types.len() as u64;
    let unresolved = unresolved_associated_types(&program.types, work)?;
    program.capture_safe_types = property_types(program, work, Property::Capture, &unresolved)?;
    program.ordinary_types = property_types(program, work, Property::Ordinary, &unresolved)?;
    program.buffer_free_types = property_types(program, work, Property::BufferFree, &unresolved)?;
    program.comparable_types = property_types(program, work, Property::Equality, &unresolved)?;
    program.application_free_types =
        property_types(program, work, Property::NoApplication, &unresolved)?;
    let mut bytes = 0usize;
    for function in program.functions.iter() {
        step(work)?;
        bytes = function
            .type_parameter_constraints
            .len()
            .checked_mul(std::mem::size_of::<
                crate::platform::kernel::TypeParameterConstraints,
            >())
            .and_then(|count| {
                count.checked_add(
                    std::mem::size_of::<
                        std::sync::Arc<[crate::platform::kernel::TypeParameterConstraints]>,
                    >() + 2 * std::mem::size_of::<usize>(),
                )
            })
            .and_then(|count| count.checked_add(bytes))
            .ok_or_else(missing)?;
    }
    work.reserve::<u8>(bytes)?;
    super::local_moves::derive_program(program, work)?;
    program.capture_proof_bytes = work.bytes;
    program.work.type_derivation_steps = work.steps as u64;
    program.work.type_metadata_bytes = program.capture_proof_bytes as u64;
    program.prepared_type_lookup = work.finish_type_lookup(program.value_origin);
    Ok(())
}

// Instantiation is keyed by the full canonical type. Template indexes remain stable for
// bytecode field/case selectors; concrete indexes are appended in canonical digest order.
// Generic signatures and intermediate effect substitutions remain useful type metadata,
// but only a fully substituted application has a runtime nominal layout. Including an
// intermediate here would make physical layout indices depend on substitution order.
fn instantiated_type(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    known: &mut BTreeMap<TypeObjectDigest, bool>,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<bool, Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    if let Some(closed) = known.get(&ty) {
        return Ok(*closed);
    }
    let object = types.get(&ty).ok_or_else(missing)?;
    let mut closed = match &object.form {
        TypeForm::TypeParameter { .. }
        | TypeForm::AssociatedContractMember { .. }
        | TypeForm::AssociatedWitnessProjection { .. }
        | TypeForm::AssociatedReadViewMember { .. }
        | TypeForm::AssociatedReadViewProjection { .. } => false,
        TypeForm::TaskFunction { effect, .. } => effect.is_closed(),
        _ => true,
    };
    work.reserve::<TypeObjectDigest>(object.child_type_count())?;
    for child in object.child_types() {
        closed &= instantiated_type(types, child, known, depth + 1, work)?;
    }
    work.node::<(TypeObjectDigest, bool)>()?;
    known.insert(ty, closed);
    Ok(closed)
}

fn complete_nominal_layouts(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    use super::value::{RecordLayoutIndex, VariantLayoutIndex};
    // Recomputing a disposable closure never promotes an instance into its template.
    work.reserve::<NormalizedRecordLayout>(program.records.len())?;
    work.reserve::<NormalizedVariantLayout>(program.variants.len())?;
    program.records = program
        .records
        .iter()
        .filter(|layout| layout.arguments.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .into();
    program.variants = program
        .variants
        .iter()
        .filter(|layout| layout.arguments.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .into();
    program.record_instances.clear();
    program.variant_instances.clear();
    for _ in program.records.iter() {
        step(work)?;
        work.node::<(
            crate::platform::kernel::DeclarationReference,
            NormalizedRecordLayout,
        )>()?;
    }
    for _ in program.variants.iter() {
        step(work)?;
        work.node::<(
            crate::platform::kernel::DeclarationReference,
            NormalizedVariantLayout,
        )>()?;
    }
    let record_templates = program
        .records
        .iter()
        .map(|layout| (layout.declaration, layout.clone()))
        .collect::<BTreeMap<_, _>>();
    let variant_templates = program
        .variants
        .iter()
        .map(|layout| (layout.declaration, layout.clone()))
        .collect::<BTreeMap<_, _>>();
    for (index, layout) in program.records.iter().enumerate() {
        step(work)?;
        if layout.type_parameters.is_empty() {
            let object = TypeObject::new(TypeForm::Named {
                declaration: layout.declaration,
            })?;
            let (ty, _) = encode_type_object(&object)?;
            if !program.types.contains_key(&ty) {
                work.node::<(TypeObjectDigest, TypeObject)>()?;
            }
            program.types.entry(ty).or_insert(object);
            work.node::<(TypeObjectDigest, RecordLayoutIndex)>()?;
            program.record_instances.insert(
                ty,
                RecordLayoutIndex(
                    u32::try_from(index).map_err(|_| missing())?,
                    program.value_origin,
                ),
            );
        }
    }
    for (index, layout) in program.variants.iter().enumerate() {
        step(work)?;
        if layout.type_parameters.is_empty() {
            let object = TypeObject::new(TypeForm::Named {
                declaration: layout.declaration,
            })?;
            let (ty, _) = encode_type_object(&object)?;
            if !program.types.contains_key(&ty) {
                work.node::<(TypeObjectDigest, TypeObject)>()?;
            }
            program.types.entry(ty).or_insert(object);
            work.node::<(TypeObjectDigest, VariantLayoutIndex)>()?;
            program.variant_instances.insert(
                ty,
                VariantLayoutIndex(
                    u32::try_from(index).map_err(|_| missing())?,
                    program.value_origin,
                ),
            );
        }
    }
    let mut pending = BTreeSet::new();
    for (ty, object) in &program.types {
        step(work)?;
        if matches!(object.form, TypeForm::Applied { .. }) {
            work.node::<TypeObjectDigest>()?;
            pending.insert(*ty);
        }
    }
    let mut visited = BTreeSet::new();
    let mut instantiated = BTreeMap::new();
    let mut derived_records = BTreeMap::new();
    let mut derived_variants = BTreeMap::new();
    while let Some(ty) = pending.pop_first() {
        step(work)?;
        if visited.contains(&ty) {
            continue;
        }
        work.node::<TypeObjectDigest>()?;
        visited.insert(ty);
        if !instantiated_type(&program.types, ty, &mut instantiated, 0, work)? {
            continue;
        }
        let TypeForm::Applied { arguments, .. } = &program.types.get(&ty).ok_or_else(missing)?.form
        else {
            return Err(missing());
        };
        work.reserve::<TypeObjectDigest>(arguments.len())?;
        for _ in arguments {
            step(work)?;
        }
        let TypeForm::Applied {
            declaration,
            arguments,
        } = program.types.get(&ty).ok_or_else(missing)?.form.clone()
        else {
            return Err(missing());
        };
        work.reserve::<TypeObjectDigest>(arguments.len())?;
        let mut member_roots = arguments.clone();
        if let Some(template) = record_templates.get(&declaration) {
            if arguments.len() != template.type_parameters.len() {
                return Err(missing());
            }
            work.node::<(TypeObjectDigest, NormalizedRecordLayout)>()?;
            work.reserve::<super::prepare::NormalizedRecordField>(template.fields.len())?;
            work.reserve::<TypeObjectDigest>(template.fields.len())?;
            for _ in &arguments {
                step(work)?;
                work.node::<(TypeParameterId, TypeObjectDigest)>()?;
            }
            let bindings = template
                .type_parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied())
                .collect();
            let mut fields = Vec::new();
            for field in template.fields.iter() {
                step(work)?;
                work.reserve::<u8>(field.name.as_str().len())?;
                let mut field = field.clone();
                field.ty = substitute(&mut program.types, field.ty, &bindings, 0, work)?;
                member_roots.push(field.ty);
                fields.push(field);
            }
            derived_records.insert(
                ty,
                NormalizedRecordLayout {
                    fields: fields.into(),
                    arguments: arguments.into(),
                    ..template.clone()
                },
            );
        } else if let Some(template) = variant_templates.get(&declaration) {
            if arguments.len() != template.type_parameters.len() {
                return Err(missing());
            }
            work.node::<(TypeObjectDigest, NormalizedVariantLayout)>()?;
            work.reserve::<super::prepare::NormalizedVariantCase>(template.cases.len())?;
            work.reserve::<TypeObjectDigest>(template.cases.len())?;
            for _ in &arguments {
                step(work)?;
                work.node::<(TypeParameterId, TypeObjectDigest)>()?;
            }
            let bindings = template
                .type_parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied())
                .collect();
            let mut cases = Vec::new();
            for case in template.cases.iter() {
                step(work)?;
                work.reserve::<u8>(case.name.as_str().len())?;
                let mut case = case.clone();
                case.payload = case
                    .payload
                    .map(|ty| substitute(&mut program.types, ty, &bindings, 0, work))
                    .transpose()?;
                member_roots.extend(case.payload);
                cases.push(case);
            }
            derived_variants.insert(
                ty,
                NormalizedVariantLayout {
                    cases: cases.into(),
                    arguments: arguments.into(),
                    ..template.clone()
                },
            );
        } else {
            return Err(missing());
        }
        let mut reached = BTreeSet::new();
        work.reserve::<(TypeObjectDigest, usize)>(member_roots.len())?;
        let mut roots = member_roots
            .into_iter()
            .map(|ty| (ty, 0usize))
            .collect::<Vec<_>>();
        while let Some((child, depth)) = roots.pop() {
            step(work)?;
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(missing());
            }
            if reached.contains(&child) {
                continue;
            }
            work.node::<TypeObjectDigest>()?;
            reached.insert(child);
            let object = program.types.get(&child).ok_or_else(missing)?;
            if matches!(object.form, TypeForm::Applied { .. })
                && !visited.contains(&child)
                && !pending.contains(&child)
            {
                work.node::<TypeObjectDigest>()?;
                pending.insert(child);
            }
            work.cloned_type(object)?;
            for child in object.child_types() {
                step(work)?;
                work.reserve::<(TypeObjectDigest, usize)>(1)?;
                roots.push((child, depth + 1));
            }
        }
    }
    work.reserve::<NormalizedRecordLayout>(program.records.len())?;
    let mut records = program.records.to_vec();
    for (ty, layout) in derived_records {
        step(work)?;
        work.node::<(TypeObjectDigest, RecordLayoutIndex)>()?;
        work.reserve::<NormalizedRecordLayout>(1)?;
        program.record_instances.insert(
            ty,
            RecordLayoutIndex(
                u32::try_from(records.len()).map_err(|_| missing())?,
                program.value_origin,
            ),
        );
        records.push(layout);
    }
    work.reserve::<NormalizedVariantLayout>(program.variants.len())?;
    let mut variants = program.variants.to_vec();
    for (ty, layout) in derived_variants {
        step(work)?;
        work.node::<(TypeObjectDigest, VariantLayoutIndex)>()?;
        work.reserve::<NormalizedVariantLayout>(1)?;
        program.variant_instances.insert(
            ty,
            VariantLayoutIndex(
                u32::try_from(variants.len()).map_err(|_| missing())?,
                program.value_origin,
            ),
        );
        variants.push(layout);
    }
    work.reserve::<bool>(variants.len())?;
    let mut affine = program.affine_variants.to_vec();
    affine.resize(variants.len(), false);
    program.affine_variants = affine.into();
    program.work.record_layouts = records.len() as u64;
    program.work.variant_layouts = variants.len() as u64;
    program.records = records.into();
    program.variants = variants.into();
    Ok(())
}

fn complete_read_view_layouts(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    use super::prepare::{NormalizedReadViewField, NormalizedReadViewLayout};
    work.reserve::<NormalizedReadViewLayout>(program.read_views.len())?;
    for layout in program.read_views.iter() {
        step(work)?;
        work.node::<(
            crate::platform::kernel::DeclarationReference,
            NormalizedReadViewLayout,
        )>()?;
        work.reserve::<u8>(layout.scope_parameter.name.as_str().len())?;
    }
    let templates = program
        .read_views
        .iter()
        .filter(|layout| {
            layout.read_scope
                == ReadScopeArgument::Parameter(ReadScopeReference {
                    binder: ReadScopeBinder::Declaration(layout.declaration),
                    parameter: layout.scope_parameter.id,
                })
        })
        .cloned()
        .map(|layout| (layout.declaration, layout))
        .collect::<BTreeMap<_, _>>();
    work.reserve::<NormalizedReadViewLayout>(templates.len())?;
    for template in templates.values() {
        work.reserve::<u8>(template.scope_parameter.name.as_str().len())?;
    }
    let mut layouts = templates.values().cloned().collect::<Vec<_>>();
    program.read_view_instances.clear();
    let mut pending = BTreeSet::new();
    for (ty, object) in &program.types {
        step(work)?;
        if matches!(object.form, TypeForm::ReadViewApplied { .. }) {
            work.node::<TypeObjectDigest>()?;
            pending.insert(*ty);
        }
    }
    let mut visited = BTreeSet::new();
    let mut closed = BTreeMap::new();
    while let Some(ty) = pending.pop_first() {
        step(work)?;
        if visited.contains(&ty) {
            continue;
        }
        work.node::<TypeObjectDigest>()?;
        visited.insert(ty);
        if !instantiated_type(&program.types, ty, &mut closed, 0, work)? {
            continue;
        }
        work.cloned_type(program.types.get(&ty).ok_or_else(missing)?)?;
        let TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope,
        } = program.types.get(&ty).ok_or_else(missing)?.form.clone()
        else {
            return Err(missing());
        };
        let template = templates.get(&declaration).ok_or_else(missing)?;
        if arguments.len() != template.type_parameters.len()
            || arguments.len() != template.type_parameter_constraints.len()
        {
            return Err(missing());
        }
        for (argument, constraint) in arguments
            .iter()
            .zip(template.type_parameter_constraints.iter())
        {
            step(work)?;
            if program.is_owned_type(*argument) != constraint.has_owned()
                || matches!(
                    program.types.get(argument).map(|object| &object.form),
                    Some(TypeForm::ReadViewApplied { .. })
                )
            {
                return Err(missing());
            }
        }
        work.reserve::<(TypeParameterId, TypeObjectDigest)>(arguments.len())?;
        work.reserve::<TypeObjectDigest>(arguments.len())?;
        work.reserve::<NormalizedReadViewField>(template.fields.len())?;
        let bindings = template
            .type_parameters
            .iter()
            .copied()
            .zip(arguments.iter().copied())
            .collect();
        work.node::<(ReadScopeReference, ReadScopeArgument)>()?;
        let scopes = BTreeMap::from([(
            ReadScopeReference {
                binder: ReadScopeBinder::Declaration(declaration),
                parameter: template.scope_parameter.id,
            },
            read_scope,
        )]);
        let application = TypeApplication {
            bindings: &bindings,
            associated: None,
            function: None,
            scopes: Some(&scopes),
        };
        let mut fields = Vec::new();
        for field in template.fields.iter() {
            step(work)?;
            work.reserve::<u8>(field.name.as_str().len())?;
            let mut field = field.clone();
            field.ty = substitute_in(&mut program.types, field.ty, &application, 0, work)?;
            if let ReadViewFieldKind::ReadReference { read_scope } = &mut field.kind {
                *read_scope = resolve_scope(*read_scope, Some(&scopes))?;
            }
            fields.push(field);
        }
        // Descend every generic actual and inline field, including unused fields.
        work.reserve::<(TypeObjectDigest, usize)>(
            arguments
                .len()
                .checked_add(fields.len())
                .ok_or_else(missing)?,
        )?;
        let mut roots = arguments
            .iter()
            .copied()
            .chain(fields.iter().map(|field| field.ty))
            .map(|ty| (ty, 0usize))
            .collect::<Vec<_>>();
        let mut reached = BTreeSet::new();
        while let Some((root, depth)) = roots.pop() {
            step(work)?;
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(missing());
            }
            if reached.contains(&root) {
                continue;
            }
            work.node::<TypeObjectDigest>()?;
            reached.insert(root);
            let object = program.types.get(&root).ok_or_else(missing)?;
            if matches!(object.form, TypeForm::ReadViewApplied { .. }) && !visited.contains(&root) {
                work.node::<TypeObjectDigest>()?;
                pending.insert(root);
            }
            work.reserve::<(TypeObjectDigest, usize)>(object.child_type_count())?;
            roots.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
        work.node::<(TypeObjectDigest, u32)>()?;
        work.reserve::<NormalizedReadViewLayout>(1)?;
        work.reserve::<u8>(template.scope_parameter.name.as_str().len())?;
        program
            .read_view_instances
            .insert(ty, u32::try_from(layouts.len()).map_err(|_| missing())?);
        layouts.push(NormalizedReadViewLayout {
            arguments: arguments.into(),
            read_scope,
            fields: fields.into(),
            ..template.clone()
        });
    }
    program.read_views = layouts.into();
    program.work.read_view_layouts = program.read_views.len() as u64;
    Ok(())
}

// Preparation-local greatest fixed point. Disqualifying leaves propagate through reverse
// dependencies, including every argument/member of a cyclic component. No persistent cache.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Property {
    BufferFree,
    Capture,
    Ordinary,
    Equality,
    NoApplication,
}

/// Raw function signatures deliberately retain templates. Even properties whose
/// ordinary callable rules ignore signature children must not certify a type
/// containing an unresolved associated selection as a concrete runtime type.
fn unresolved_associated_types(
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    work: &mut Budget<'_>,
) -> Result<BTreeSet<TypeObjectDigest>, Diagnostic> {
    let mut unresolved = BTreeSet::new();
    let mut pending = Vec::new();
    for (ty, object) in types {
        step(work)?;
        if object.form.is_associated() {
            work.node::<TypeObjectDigest>()?;
            work.reserve::<TypeObjectDigest>(1)?;
            unresolved.insert(*ty);
            pending.push(*ty);
        }
    }
    if pending.is_empty() {
        return Ok(unresolved);
    }
    let mut reverse = BTreeMap::<TypeObjectDigest, Vec<TypeObjectDigest>>::new();
    for (ty, object) in types {
        step(work)?;
        work.reserve::<TypeObjectDigest>(object.child_type_count())?;
        for child in object.child_types() {
            step(work)?;
            if !types.contains_key(&child) {
                return Err(missing());
            }
            if !reverse.contains_key(&child) {
                work.node::<(TypeObjectDigest, Vec<TypeObjectDigest>)>()?;
            }
            work.reserve::<TypeObjectDigest>(1)?;
            reverse.entry(child).or_default().push(*ty);
        }
    }
    while let Some(ty) = pending.pop() {
        step(work)?;
        for parent in reverse.get(&ty).into_iter().flatten() {
            step(work)?;
            if !unresolved.contains(parent) {
                work.node::<TypeObjectDigest>()?;
                work.reserve::<TypeObjectDigest>(1)?;
                unresolved.insert(*parent);
                pending.push(*parent);
            }
        }
    }
    Ok(unresolved)
}

fn property_types(
    program: &NormalizedProgram,
    work: &mut Budget<'_>,
    property: Property,
    unresolved: &BTreeSet<TypeObjectDigest>,
) -> Result<BTreeSet<TypeObjectDigest>, Diagnostic> {
    let mut safe = BTreeSet::new();
    let mut reverse: BTreeMap<TypeObjectDigest, Vec<TypeObjectDigest>> = BTreeMap::new();
    let mut rejected = Vec::new();
    for (root, object) in &program.types {
        step(work)?;
        work.node::<TypeObjectDigest>()?;
        safe.insert(*root);
        let mut admitted = true;
        let mut child = |ty| -> Result<(), Diagnostic> {
            step(work)?;
            if !program.types.contains_key(&ty) {
                return Err(missing());
            }
            if !reverse.contains_key(&ty) {
                work.node::<(TypeObjectDigest, Vec<TypeObjectDigest>)>()?;
            }
            work.reserve::<TypeObjectDigest>(1)?;
            reverse.entry(ty).or_default().push(*root);
            Ok(())
        };
        match &object.form {
            _ if unresolved.contains(root) => admitted = false,
            TypeForm::RegionBrand { .. } | TypeForm::RegionCap { .. } => admitted = false,
            TypeForm::Region { .. } | TypeForm::RelPtr { .. } | TypeForm::RegionSlice { .. } => {
                // Offsets carry no storage. Their live region is proved at each
                // consuming operation; an unscoped capture cannot retain them.
                admitted = property != Property::Capture;
            }
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. }
            | TypeForm::ReadViewApplied { .. } => {
                // Raw signature templates remain in the canonical table. Only
                // their materialized application types can acquire properties.
                admitted = false;
            }
            _ if program.owned_record_layout(*root).is_some() => {
                // Nominal ownership is independent of the field inventory.
                // Even empty and ordinary-only records carry affine custody.
                admitted = false;
            }
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. }
                if property == Property::BufferFree =>
            {
                admitted = false
            }
            _ if property == Property::BufferFree => {
                for ty in object.child_types() {
                    child(ty)?;
                }
            }
            TypeForm::Applied { .. } if property == Property::NoApplication => {
                admitted = false;
            }
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. }
            | TypeForm::Secret
            | TypeForm::Stream { .. }
            | TypeForm::CapabilityResource { .. }
            | TypeForm::TypeParameter { .. }
                if property == Property::NoApplication => {}
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText => {}
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. }
                if property != Property::Equality => {}
            TypeForm::Secret if property == Property::Ordinary => {}
            TypeForm::Function { .. }
            | TypeForm::TaskFunction { .. }
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. }
            | TypeForm::Secret
            | TypeForm::Stream { .. }
            | TypeForm::CapabilityResource { .. }
            | TypeForm::TypeParameter { .. } => {
                admitted = false;
            }
            TypeForm::List { item } | TypeForm::Option { item } => child(*item)?,
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                child(*key)?;
                child(*value)?;
            }
            TypeForm::StructuralRecord { fields } => {
                for field in fields {
                    child(field.ty)?;
                }
            }
            TypeForm::Named { .. } | TypeForm::Applied { .. } => {
                if let Some(index) = program
                    .record_instances
                    .get(root)
                    .map(|index| index.0 as usize)
                {
                    for argument in program.records[index].arguments.iter() {
                        child(*argument)?;
                    }
                    for field in program.records[index].fields.iter() {
                        child(field.ty)?;
                    }
                } else if let Some(index) = program
                    .variant_instances
                    .get(root)
                    .map(|index| index.0 as usize)
                {
                    for argument in program.variants[index].arguments.iter() {
                        child(*argument)?;
                    }
                    for case in program.variants[index].cases.iter() {
                        if let Some(payload) = case.payload {
                            child(payload)?;
                        }
                    }
                } else if matches!(object.form, TypeForm::Applied { .. }) {
                    admitted = false;
                } else {
                    return Err(missing());
                }
            }
        }
        if !admitted {
            work.reserve::<TypeObjectDigest>(1)?;
            safe.remove(root);
            rejected.push(*root);
        }
    }
    while let Some(ty) = rejected.pop() {
        step(work)?;
        if let Some(dependents) = reverse.get(&ty) {
            for dependent in dependents {
                step(work)?;
                if safe.remove(dependent) {
                    work.reserve::<TypeObjectDigest>(1)?;
                    rejected.push(*dependent);
                }
            }
        }
    }
    Ok(safe)
}

#[cfg(test)]
fn calls(
    code: &NormalizedCode,
    application: &TypeApplication<'_>,
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    pending: &mut BTreeSet<Context>,
    work: &mut Budget<'_>,
    records: &[NormalizedRecordLayout],
    variants: &[NormalizedVariantLayout],
) -> Result<(), Diagnostic> {
    calls_at(
        code,
        (application, None),
        types,
        pending,
        work,
        records,
        variants,
    )
}

fn calls_at(
    code: &NormalizedCode,
    (application, instruction_scopes): (&TypeApplication<'_>, Option<&ScopeBindings>),
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    pending: &mut BTreeSet<Context>,
    work: &mut Budget<'_>,
    records: &[NormalizedRecordLayout],
    variants: &[NormalizedVariantLayout],
) -> Result<(), Diagnostic> {
    // Closing rewrites these view operands once. Its exact supplied actuals
    // stay opaque during reprocessing; signatures and other operands still
    // use the original formal substitution and application lookup identity.
    let view_application = TypeApplication {
        bindings: application.bindings,
        associated: application.associated,
        // Rewritten view roots must not publish a function lookup keyed by
        // identity actuals instead of its original formal application.
        function: if instruction_scopes.is_some() {
            None
        } else {
            application.function
        },
        scopes: instruction_scopes.or(application.scopes),
    };
    for instruction in code.instructions.iter() {
        step(work)?;
        if let NormalizedInstruction::Region(operation) = instruction
            && let Some(ty) = operation.ty()
        {
            substitute_in(types, *ty, application, 0, work)?;
        }
        match instruction {
            NormalizedInstruction::BeginReadViewScope { read_scope }
            | NormalizedInstruction::EndReadViewScope { read_scope } => {
                resolve_scope(*read_scope, view_application.scopes)?;
            }
            NormalizedInstruction::BeginReadView { view_type, .. }
            | NormalizedInstruction::FinishReadView { view_type, .. }
            | NormalizedInstruction::BorrowReadView { view_type, .. }
            | NormalizedInstruction::ReborrowReadView { view_type, .. }
            | NormalizedInstruction::MoveReadView { view_type, .. }
            | NormalizedInstruction::DropReadView { view_type, .. }
            | NormalizedInstruction::ReadViewDataField { view_type, .. }
            | NormalizedInstruction::BorrowReadViewField { view_type, .. } => {
                substitute_in(types, *view_type, &view_application, 0, work)?;
            }
            _ => {}
        }
        if let NormalizedInstruction::PackOwned { product_type, .. }
        | NormalizedInstruction::UnpackOwned { product_type, .. }
        | NormalizedInstruction::ChooseOwned {
            choice_type: product_type,
            ..
        }
        | NormalizedInstruction::MatchOwned {
            choice_type: product_type,
            ..
        }
        | NormalizedInstruction::BorrowOwnedField { product_type, .. }
        | NormalizedInstruction::MatchBorrowedOwned {
            choice_type: product_type,
            ..
        }
        | NormalizedInstruction::SequenceEmpty {
            sequence_type: product_type,
        }
        | NormalizedInstruction::SequenceLength {
            sequence_type: product_type,
            ..
        }
        | NormalizedInstruction::SequencePush {
            sequence_type: product_type,
            ..
        }
        | NormalizedInstruction::SequenceGet {
            sequence_type: product_type,
            ..
        }
        | NormalizedInstruction::SequenceReplace {
            sequence_type: product_type,
            ..
        }
        | NormalizedInstruction::SequencePop {
            sequence_type: product_type,
            ..
        }
        | NormalizedInstruction::BorrowOwnedItem {
            sequence_type: product_type,
            ..
        } = instruction
        {
            substitute_in(types, *product_type, application, 0, work)?;
        }
        match instruction {
            NormalizedInstruction::BorrowOwnedField { binding_type, .. }
            | NormalizedInstruction::BorrowOwnedItem { binding_type, .. }
            | NormalizedInstruction::AdoptBorrowResult { binding_type, .. } => {
                substitute_in(types, *binding_type, application, 0, work)?;
            }
            NormalizedInstruction::BorrowReadView { binding_type, .. }
            | NormalizedInstruction::BorrowReadViewField { binding_type, .. }
            | NormalizedInstruction::AdoptViewResult { binding_type, .. } => {
                substitute_in(types, *binding_type, &view_application, 0, work)?;
            }
            NormalizedInstruction::SequencePop { result_type, .. }
            | NormalizedInstruction::SequenceReplace { result_type, .. } => {
                substitute_in(types, *result_type, application, 0, work)?;
            }
            NormalizedInstruction::MatchBorrowedOwned { cases, .. } => {
                for case in cases.iter() {
                    step(work)?;
                    substitute_in(types, case.binding_type, application, 0, work)?;
                }
            }
            _ => {}
        }
        let nominal: Option<(_, _, &[TypeObjectDigest])> = match instruction {
            NormalizedInstruction::Record {
                layout: Some(layout),
                type_arguments,
                ..
            } => {
                let template = records.get(layout.0 as usize).ok_or_else(missing)?;
                Some((
                    template.declaration,
                    template.type_parameters.len(),
                    type_arguments,
                ))
            }
            NormalizedInstruction::Variant {
                layout,
                type_arguments,
                ..
            } => {
                let template = variants.get(layout.0 as usize).ok_or_else(missing)?;
                Some((
                    template.declaration,
                    template.type_parameters.len(),
                    type_arguments,
                ))
            }
            NormalizedInstruction::BeginTransactionOutcome { outcome, .. }
            | NormalizedInstruction::CommitTransactionOutcome { outcome, .. } => Some((
                outcome.contract.outcome,
                1,
                std::slice::from_ref(&outcome.type_argument),
            )),
            _ => None,
        };
        if let Some((declaration, arity, arguments)) = nominal {
            if arity != arguments.len() {
                return Err(missing());
            }
            work.reserve::<TypeObjectDigest>(arguments.len())?;
            let arguments = arguments
                .iter()
                .map(|ty| substitute_in(types, *ty, application, 0, work))
                .collect::<Result<Vec<_>, _>>()?;
            let form = if arguments.is_empty() {
                TypeForm::Named { declaration }
            } else {
                TypeForm::Applied {
                    declaration,
                    arguments,
                }
            };
            let object = TypeObject::new(form)?;
            let (ty, _) = encode_type_object(&object)?;
            step(work)?;
            if !types.contains_key(&ty) {
                work.node::<(TypeObjectDigest, TypeObject)>()?;
            }
            types.entry(ty).or_insert(object);
        }
        if let NormalizedInstruction::Parallel {
            left,
            left_types,
            right,
            right_types,
            result_type,
            ..
        } = instruction
        {
            // A joined pair may be only an intermediate local: no function
            // signature needs to retain its concrete substituted type for us.
            substitute_in(types, *result_type, application, 0, work)?;
            for (function, types_actual) in [(left, left_types), (right, right_types)] {
                work.reserve::<TypeObjectDigest>(types_actual.len())?;
                let arguments = types_actual
                    .iter()
                    .map(|ty| substitute_in(types, *ty, application, 0, work))
                    .collect::<Result<Vec<_>, _>>()?;
                work.node::<Context>()?;
                pending.insert((*function, arguments));
            }
        }
        if let NormalizedInstruction::Call {
            function,
            type_arguments,
            read_scope_arguments,
            ..
        }
        | NormalizedInstruction::TailCall {
            function,
            type_arguments,
            read_scope_arguments,
            ..
        }
        | NormalizedInstruction::FunctionValue {
            effect_arguments: _,
            requirement_arguments: _,
            function,
            type_arguments,
            read_scope_arguments,
        } = instruction
        {
            // Specialization has resolved every supplied scope. Inspect the
            // complete inventory again, including formals unused by types.
            for read_scope in read_scope_arguments.iter() {
                step(work)?;
                if let ReadScopeArgument::Parameter(reference) = read_scope {
                    let scopes = application.scopes.ok_or_else(missing)?;
                    if !scopes.contains_key(reference) {
                        let mut exact_actual = false;
                        for actual in scopes.values() {
                            step(work)?;
                            if actual == read_scope {
                                exact_actual = true;
                                break;
                            }
                        }
                        if !exact_actual {
                            return Err(missing());
                        }
                    }
                }
            }
            work.reserve::<TypeObjectDigest>(type_arguments.len())?;
            let arguments = type_arguments
                .iter()
                .map(|ty| substitute_in(types, *ty, application, 0, work))
                .collect::<Result<_, _>>()?;
            work.node::<Context>()?;
            pending.insert((*function, arguments));
        }
    }
    Ok(())
}

fn missing() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "normalized_instantiation_scope",
        "prepared type closure has a missing exact type, function, or substitution",
    )
}

fn resolve_scope(
    scope: ReadScopeArgument,
    bindings: Option<&ScopeBindings>,
) -> Result<ReadScopeArgument, Diagnostic> {
    match (scope, bindings) {
        (ReadScopeArgument::Parameter(reference), Some(bindings)) => {
            bindings.get(&reference).copied().ok_or_else(missing)
        }
        _ => Ok(scope),
    }
}

// This is derived only from an already admitted application. It grants no
// source naming or scope authority and does not chase another formal mapping.
fn applied_scope_identities(
    supplied: &ScopeBindings,
    work: &mut Budget<'_>,
) -> Result<ScopeBindings, Diagnostic> {
    step(work)?;
    let mut identities = BTreeMap::new();
    for actual in supplied.values() {
        step(work)?;
        if let ReadScopeArgument::Parameter(reference) = actual
            && !identities.contains_key(reference)
        {
            work.node::<(ReadScopeReference, ReadScopeArgument)>()?;
            identities.insert(*reference, *actual);
        }
    }
    Ok(identities)
}

fn function_scope_bindings(
    function: &super::prepare::NormalizedFunction,
    work: &mut Budget<'_>,
) -> Result<ScopeBindings, Diagnostic> {
    if function.read_scope_parameters.len() != function.read_scope_arguments.len() {
        return Err(missing());
    }
    work.reserve::<(ReadScopeReference, ReadScopeArgument)>(function.read_scope_parameters.len())?;
    let mut bindings = BTreeMap::new();
    for (parameter, actual) in function
        .read_scope_parameters
        .iter()
        .zip(function.read_scope_arguments.iter())
    {
        step(work)?;
        work.node::<(ReadScopeReference, ReadScopeArgument)>()?;
        let reference = ReadScopeReference {
            binder: ReadScopeBinder::Declaration(function.declaration),
            parameter: parameter.id,
        };
        if bindings.insert(reference, *actual).is_some() {
            return Err(missing());
        }
    }
    for source in function.read_scope_sources.iter() {
        step(work)?;
        if source.source >= function.parameter_count {
            return Err(missing());
        }
        resolve_scope(source.scope, Some(&bindings))?;
    }
    if let crate::platform::kernel::ResultRelation::ViewFrom { source, scope } =
        function.result.relation
    {
        if source >= function.parameter_count {
            return Err(missing());
        }
        resolve_scope(scope, Some(&bindings))?;
    }
    Ok(bindings)
}

#[cfg(test)]
pub(super) fn check_product_substitution(
    mut types: BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    bindings: BTreeMap<TypeParameterId, TypeObjectDigest>,
) -> Result<(), Diagnostic> {
    let control = crate::platform::execution::ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    substitute(&mut types, ty, &bindings, 0, &mut work)?;
    product_depth::validate(&types, &mut work)
}

/// Test bridge for the independent canonical oracle. Inputs enter at the same
/// effect-closed boundary as complete_budgeted; this runs the real producer
/// substitution and rejects any unresolved output without fabricating an artifact.
#[cfg(test)]
pub(super) fn materialize_associated_roots(
    function: &super::prepare::NormalizedFunction,
    ordinary: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    roots: &[TypeObjectDigest],
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
    if ordinary.len() != function.type_parameters.len()
        || function
            .type_parameters
            .iter()
            .any(|parameter| !ordinary.contains_key(parameter))
    {
        return Err(missing());
    }
    if !function.type_arguments.is_empty()
        && (function.type_arguments.len() != function.type_parameters.len()
            || function
                .type_parameters
                .iter()
                .zip(function.type_arguments.iter())
                .any(|(parameter, actual)| ordinary.get(parameter) != Some(actual)))
    {
        return Err(missing());
    }
    let environment = ClosedTypeEnvironment::for_function(function)?;
    let application = TypeApplication {
        bindings: ordinary,
        associated: Some(&environment),
        function: None,
        scopes: None,
    };
    let mut work = Budget::new(control);
    work.reserve::<TypeObjectDigest>(roots.len())?;
    let mut resolved = Vec::new();
    let mut closed = BTreeMap::new();
    for root in roots {
        let result = substitute_in(types, *root, &application, 0, &mut work)?;
        if !instantiated_type(types, result, &mut closed, 0, &mut work)? {
            return Err(missing());
        }
        resolved.push(result);
    }
    Ok(resolved)
}

/// Effect applications have a finite exact requirement universe. Ordinary type applications
/// remain under the existing recursive-call contract and the subsequent type worklist.
fn close_effect_applications(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    use super::prepare::{NormalizedCode, NormalizedEntryPoint, NormalizedFunction};
    use crate::platform::kernel::{
        EffectParameterReference, EffectRow, FunctionEffect, RequirementOperand,
        RequirementParameterReference, RequirementReference, RequirementSubstitution,
    };
    use std::sync::Arc;

    struct Closing<'a, 'b> {
        templates: Arc<[NormalizedFunction]>,
        functions: Vec<NormalizedFunction>,
        instances: BTreeMap<EffectApplication, FunctionIndex>,
        pending: Vec<(EffectApplication, FunctionIndex)>,
        types: &'a mut BTreeMap<TypeObjectDigest, TypeObject>,
        requirements:
            BTreeMap<crate::platform::kernel::RequirementReference, super::value::RequirementIndex>,
        work: &'a mut Budget<'b>,
    }
    impl Closing<'_, '_> {
        fn rows(&mut self, rows: &[EffectRow]) -> Result<(), Diagnostic> {
            self.work.reserve::<EffectRow>(rows.len())?;
            for row in rows {
                step(self.work)?;
                self.work
                    .reserve::<crate::platform::kernel::RequirementOperand>(
                        row.requirements.len(),
                    )?;
                self.work
                    .reserve::<EffectParameterReference>(row.parameters.len())?;
                row.validate()?;
            }
            Ok(())
        }
        fn application(
            &mut self,
            target: FunctionIndex,
            rows: Vec<EffectRow>,
            requirements: Vec<RequirementReference>,
        ) -> Result<FunctionIndex, Diagnostic> {
            step(self.work)?;
            let function = self.templates.get(target.0 as usize).ok_or_else(missing)?;
            if function.requirement_parameters.len() != requirements.len()
                || function.effect_parameters.len() != rows.len()
                || rows.iter().any(|row| !row.is_closed())
            {
                return Err(missing());
            }
            self.work
                .reserve::<RequirementReference>(requirements.len())?;
            let key = (target, rows, requirements);
            if let Some(index) = self.instances.get(&key) {
                return Ok(*index);
            }
            self.work.node::<(EffectApplication, FunctionIndex)>()?;
            self.work.reserve::<NormalizedFunction>(1)?;
            let index = if function.effect_parameters.is_empty()
                && function.requirement_parameters.is_empty()
            {
                target
            } else {
                self.work.cloned_effect(&function.effect)?;
                let index = FunctionIndex(
                    u32::try_from(self.functions.len()).map_err(|_| missing())?,
                    target.1,
                );
                self.functions.push(function.clone());
                index
            };
            self.rows(&key.1)?;
            self.work.node::<(EffectApplication, FunctionIndex)>()?;
            self.instances.insert(key.clone(), index);
            self.pending.push((key, index));
            Ok(index)
        }
        fn code(
            &mut self,
            code: &mut NormalizedCode,
            bindings: &EffectBindings,
            requirements: &RequirementSubstitution,
        ) -> Result<(), Diagnostic> {
            self.work
                .reserve::<NormalizedInstruction>(code.instructions.len())?;
            // Admit variable operand storage before Arc::make_mut can clone it.
            for instruction in code.instructions.iter() {
                step(self.work)?;
                if let NormalizedInstruction::ImplementationCall {
                    effect_arguments,
                    requirement_arguments,
                    ..
                }
                | NormalizedInstruction::Call {
                    effect_arguments,
                    requirement_arguments,
                    ..
                }
                | NormalizedInstruction::TailCall {
                    effect_arguments,
                    requirement_arguments,
                    ..
                }
                | NormalizedInstruction::FunctionValue {
                    effect_arguments,
                    requirement_arguments,
                    ..
                } = instruction
                {
                    self.rows(effect_arguments)?;
                    self.work
                        .reserve::<RequirementOperand>(requirement_arguments.len())?;
                }
                if let NormalizedInstruction::MethodCall { witness, .. } = instruction {
                    reserve_implementation_operand(witness, 0, self.work)?;
                }
            }
            for instruction in Arc::make_mut(&mut code.instructions) {
                step(self.work)?;
                if let NormalizedInstruction::ImplementationCall {
                    implementations, ..
                } = instruction
                {
                    self.work
                        .reserve::<crate::platform::kernel::ImplementationOperand>(
                            implementations.len(),
                        )?;
                    for operand in implementations.iter() {
                        reserve_implementation_operand(operand, 0, self.work)?;
                    }
                    for operand in Arc::make_mut(implementations) {
                        substitute_implementation_effects(
                            operand,
                            self.types,
                            bindings,
                            requirements,
                            0,
                            self.work,
                        )?;
                    }
                }
                if let NormalizedInstruction::MethodCall { witness, .. } = instruction {
                    substitute_implementation_effects(
                        witness,
                        self.types,
                        bindings,
                        requirements,
                        0,
                        self.work,
                    )?;
                }
                match instruction {
                    NormalizedInstruction::ImplementationCall {
                        function,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        ..
                    }
                    | NormalizedInstruction::Call {
                        function,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        ..
                    }
                    | NormalizedInstruction::TailCall {
                        function,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        ..
                    }
                    | NormalizedInstruction::FunctionValue {
                        function,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        ..
                    } => {
                        self.work.reserve::<EffectRow>(effect_arguments.len())?;
                        let rows = effect_arguments
                            .iter()
                            .map(|row| {
                                row.substitute_requirements(requirements, |n| {
                                    self.work.reserve::<RequirementOperand>(n)?;
                                    for _ in 0..n {
                                        step(self.work)?;
                                    }
                                    Ok(())
                                })?
                                .substitute(bindings, |n| {
                                    self.work
                                        .reserve::<crate::platform::kernel::RequirementOperand>(
                                            n,
                                        )?;
                                    for _ in 0..n {
                                        step(self.work)?;
                                    }
                                    Ok(())
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        self.work
                            .reserve::<RequirementReference>(requirement_arguments.len())?;
                        let arguments = requirement_arguments
                            .iter()
                            .map(|operand| {
                                step(self.work)?;
                                operand
                                    .substitute(requirements)?
                                    .concrete()
                                    .ok_or_else(missing)
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        *function = self.application(*function, rows, arguments)?;
                        *requirement_arguments = Arc::from([]);
                        *effect_arguments = Arc::from([]);
                        self.work
                            .reserve::<TypeObjectDigest>(type_arguments.len())?;
                        for ty in Arc::make_mut(type_arguments) {
                            *ty = substitute_effect_type(
                                self.types,
                                *ty,
                                bindings,
                                requirements,
                                0,
                                self.work,
                            )?;
                        }
                    }
                    NormalizedInstruction::Region(operation) => {
                        *operation = operation.try_map(
                            |ty| {
                                substitute_effect_type(
                                    self.types,
                                    *ty,
                                    bindings,
                                    requirements,
                                    0,
                                    self.work,
                                )
                            },
                            |(requirement, target)| {
                                let requirement = match requirement {
                                    NormalizedTransactionRequirement::Parameter(parameter) => {
                                        let reference = RequirementOperand::Parameter(*parameter)
                                            .substitute(requirements)?
                                            .concrete()
                                            .ok_or_else(missing)?;
                                        NormalizedTransactionRequirement::Concrete(
                                            *self
                                                .requirements
                                                .get(&reference)
                                                .ok_or_else(missing)?,
                                        )
                                    }
                                    requirement => *requirement,
                                };
                                Ok((requirement, *target))
                            },
                        )?;
                    }
                    NormalizedInstruction::PackOwned { product_type, .. }
                    | NormalizedInstruction::UnpackOwned { product_type, .. }
                    | NormalizedInstruction::ChooseOwned {
                        choice_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::MatchOwned {
                        choice_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::SequenceEmpty {
                        sequence_type: product_type,
                    }
                    | NormalizedInstruction::SequenceLength {
                        sequence_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::SequencePush {
                        sequence_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::SequenceGet {
                        sequence_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::BeginReadView {
                        view_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::FinishReadView {
                        view_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::ReborrowReadView {
                        view_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::MoveReadView {
                        view_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::DropReadView {
                        view_type: product_type,
                        ..
                    }
                    | NormalizedInstruction::ReadViewDataField {
                        view_type: product_type,
                        ..
                    } => {
                        *product_type = substitute_effect_type(
                            self.types,
                            *product_type,
                            bindings,
                            requirements,
                            0,
                            self.work,
                        )?;
                    }
                    NormalizedInstruction::AdoptBorrowResult { binding_type, .. }
                    | NormalizedInstruction::AdoptViewResult { binding_type, .. } => {
                        *binding_type = substitute_effect_type(
                            self.types,
                            *binding_type,
                            bindings,
                            requirements,
                            0,
                            self.work,
                        )?;
                    }
                    NormalizedInstruction::BorrowOwnedField {
                        product_type,
                        binding_type,
                        ..
                    }
                    | NormalizedInstruction::BorrowOwnedItem {
                        sequence_type: product_type,
                        binding_type,
                        ..
                    }
                    | NormalizedInstruction::SequencePop {
                        sequence_type: product_type,
                        result_type: binding_type,
                        ..
                    }
                    | NormalizedInstruction::SequenceReplace {
                        sequence_type: product_type,
                        result_type: binding_type,
                        ..
                    }
                    | NormalizedInstruction::BorrowReadView {
                        view_type: product_type,
                        binding_type,
                        ..
                    }
                    | NormalizedInstruction::BorrowReadViewField {
                        view_type: product_type,
                        binding_type,
                        ..
                    } => {
                        *product_type = substitute_effect_type(
                            self.types,
                            *product_type,
                            bindings,
                            requirements,
                            0,
                            self.work,
                        )?;
                        *binding_type = substitute_effect_type(
                            self.types,
                            *binding_type,
                            bindings,
                            requirements,
                            0,
                            self.work,
                        )?;
                    }
                    NormalizedInstruction::MatchBorrowedOwned {
                        choice_type, cases, ..
                    } => {
                        *choice_type = substitute_effect_type(
                            self.types,
                            *choice_type,
                            bindings,
                            requirements,
                            0,
                            self.work,
                        )?;
                        if Arc::get_mut(cases).is_none() {
                            self.work
                                .reserve::<super::prepare::NormalizedBorrowedOwnedChoiceJump>(
                                    cases.len(),
                                )?;
                        }
                        for case in Arc::make_mut(cases) {
                            case.binding_type = substitute_effect_type(
                                self.types,
                                case.binding_type,
                                bindings,
                                requirements,
                                0,
                                self.work,
                            )?;
                        }
                    }
                    NormalizedInstruction::Record { type_arguments, .. }
                    | NormalizedInstruction::Variant { type_arguments, .. } => {
                        self.work
                            .reserve::<TypeObjectDigest>(type_arguments.len())?;
                        for ty in Arc::make_mut(type_arguments) {
                            *ty = substitute_effect_type(
                                self.types,
                                *ty,
                                bindings,
                                requirements,
                                0,
                                self.work,
                            )?;
                        }
                    }
                    NormalizedInstruction::PerformParameter {
                        parameter,
                        operation,
                        arguments,
                    } => {
                        let reference = RequirementOperand::Parameter(*parameter)
                            .substitute(requirements)?
                            .concrete()
                            .ok_or_else(missing)?;
                        let requirement = self
                            .requirements
                            .get(&reference)
                            .copied()
                            .ok_or_else(missing)?;
                        *instruction = NormalizedInstruction::Perform {
                            requirement,
                            operation: *operation,
                            arguments: *arguments,
                        };
                    }
                    NormalizedInstruction::BeginParameterTransaction { parameter, binding } => {
                        let reference = RequirementOperand::Parameter(*parameter)
                            .substitute(requirements)?
                            .concrete()
                            .ok_or_else(missing)?;
                        let requirement = self
                            .requirements
                            .get(&reference)
                            .copied()
                            .ok_or_else(missing)?;
                        *instruction = NormalizedInstruction::BeginTransaction {
                            requirement,
                            binding: *binding,
                        };
                    }
                    NormalizedInstruction::CommitParameterTransaction { parameter, binding } => {
                        let reference = RequirementOperand::Parameter(*parameter)
                            .substitute(requirements)?
                            .concrete()
                            .ok_or_else(missing)?;
                        let requirement = self
                            .requirements
                            .get(&reference)
                            .copied()
                            .ok_or_else(missing)?;
                        *instruction = NormalizedInstruction::CommitTransaction {
                            requirement,
                            binding: *binding,
                        };
                    }
                    NormalizedInstruction::BeginTransactionOutcome {
                        requirement,
                        outcome,
                        ..
                    }
                    | NormalizedInstruction::CommitTransactionOutcome {
                        requirement,
                        outcome,
                        ..
                    } => {
                        outcome.type_argument = substitute_effect_type(
                            self.types,
                            outcome.type_argument,
                            bindings,
                            requirements,
                            0,
                            self.work,
                        )?;
                        if let NormalizedTransactionRequirement::Parameter(parameter) = requirement
                        {
                            let reference = RequirementOperand::Parameter(*parameter)
                                .substitute(requirements)?
                                .concrete()
                                .ok_or_else(missing)?;
                            *requirement = NormalizedTransactionRequirement::Concrete(
                                self.requirements
                                    .get(&reference)
                                    .copied()
                                    .ok_or_else(missing)?,
                            );
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }
    }
    let templates = Arc::clone(&program.functions);
    work.reserve::<NormalizedFunction>(templates.len())?;
    for function in templates.iter() {
        step(work)?;
        work.cloned_effect(&function.effect)?;
    }
    work.reserve::<(
        crate::platform::kernel::RequirementReference,
        super::value::RequirementIndex,
    )>(program.requirements.len())?;
    let mut closing = Closing {
        functions: templates.to_vec(),
        templates,
        instances: BTreeMap::new(),
        pending: Vec::new(),
        types: &mut program.types,
        requirements: program
            .requirements
            .iter()
            .enumerate()
            .map(|(i, r)| {
                Ok((
                    r.reference,
                    super::value::RequirementIndex(u32::try_from(i).map_err(|_| missing())?),
                ))
            })
            .collect::<Result<_, Diagnostic>>()?,
        work,
    };
    for i in 0..closing.templates.len() {
        if closing.templates[i].implementation_parameters.len()
            == closing.templates[i].implementation_arguments.len()
            && closing.templates[i].effect_parameters.is_empty()
            && closing.templates[i].requirement_parameters.is_empty()
        {
            closing.application(
                FunctionIndex(
                    u32::try_from(i).map_err(|_| missing())?,
                    program.value_origin,
                ),
                Vec::new(),
                Vec::new(),
            )?;
        }
    }
    let empty = EffectBindings::new();
    let empty_requirements = RequirementSubstitution::new();
    for test in program.tests.values_mut() {
        closing.code(&mut test.actual, &empty, &empty_requirements)?;
        closing.code(&mut test.expected, &empty, &empty_requirements)?;
    }
    closing
        .work
        .reserve::<super::prepare::NormalizedPort>(program.ports.len())?;
    for port in Arc::make_mut(&mut program.ports) {
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &mut port.entry
        {
            closing.code(code, &empty, &empty_requirements)?;
        }
    }
    while let Some(((target, rows, arguments), index)) = closing.pending.pop() {
        step(closing.work)?;
        closing.work.reserve::<NormalizedFunction>(1)?;
        closing
            .work
            .cloned_effect(&closing.templates[target.0 as usize].effect)?;
        let mut function = closing.templates[target.0 as usize].clone();
        closing
            .work
            .reserve::<(EffectParameterReference, EffectRow)>(rows.len())?;
        closing.rows(&rows)?;
        let bindings = function
            .effect_parameters
            .iter()
            .zip(&rows)
            .map(|(parameter, row)| {
                (
                    EffectParameterReference {
                        package: function.declaration.package,
                        parameter: *parameter,
                    },
                    row.clone(),
                )
            })
            .collect::<EffectBindings>();
        closing
            .work
            .reserve::<(RequirementParameterReference, RequirementOperand)>(arguments.len())?;
        let requirements = function
            .requirement_parameters
            .iter()
            .zip(&arguments)
            .map(|(parameter, reference)| {
                (
                    RequirementParameterReference {
                        package: function.declaration.package,
                        parameter: *parameter,
                    },
                    RequirementOperand::Concrete(*reference),
                )
            })
            .collect::<RequirementSubstitution>();
        closing.work.cloned_effect(&function.effect)?;
        let row = function
            .effect
            .row()
            .substitute_requirements(&requirements, |n| {
                closing.work.reserve::<RequirementOperand>(n)?;
                for _ in 0..n {
                    step(closing.work)?;
                }
                Ok(())
            })?
            .substitute(&bindings, |n| {
                closing
                    .work
                    .reserve::<crate::platform::kernel::RequirementOperand>(n)?;
                for _ in 0..n {
                    step(closing.work)?;
                }
                Ok(())
            })?;
        closing
            .work
            .reserve::<super::value::RequirementIndex>(row.requirements.len())?;
        function.task_requirements = row
            .requirements
            .iter()
            .map(|r| {
                closing
                    .requirements
                    .get(&r.concrete().ok_or_else(missing)?)
                    .copied()
                    .ok_or_else(missing)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into();
        if matches!(function.effect, FunctionEffect::Task { .. }) {
            function.effect = FunctionEffect::Task {
                requirements: row.requirements,
                effect_parameters: Vec::new(),
            };
        }
        function.requirement_parameters = Arc::from([]);
        function.requirement_arguments = arguments
            .into_iter()
            .map(RequirementOperand::Concrete)
            .collect::<Vec<_>>()
            .into();
        function.effect_parameters = Arc::from([]);
        function.effect_arguments = rows.into();
        function.result.ty = substitute_effect_type(
            closing.types,
            function.result.ty,
            &bindings,
            &requirements,
            0,
            closing.work,
        )?;
        closing
            .work
            .reserve::<super::prepare::NormalizedParameter>(function.parameters.len())?;
        for parameter in Arc::make_mut(&mut function.parameters) {
            parameter.ty = substitute_effect_type(
                closing.types,
                parameter.ty,
                &bindings,
                &requirements,
                0,
                closing.work,
            )?;
        }
        if let NormalizedFunctionBody::Code(code) = &mut function.body {
            closing.code(code, &bindings, &requirements)?;
        }
        closing.functions[index.0 as usize] = function;
    }
    program.functions = closing.functions.into();
    Ok(())
}

fn substitute_effect_type(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    bindings: &EffectBindings,
    requirements: &crate::platform::kernel::RequirementSubstitution,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<TypeObjectDigest, Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    let object = types.get(&ty).ok_or_else(missing)?;
    work.cloned_type(object)?;
    let mut object = object.clone();
    if let TypeForm::TaskFunction { effect, .. } = &mut object.form {
        *effect = effect
            .substitute_requirements(requirements, |n| {
                work.reserve::<crate::platform::kernel::RequirementOperand>(n)?;
                for _ in 0..n {
                    step(work)?;
                }
                Ok(())
            })?
            .substitute(bindings, |n| {
                work.reserve::<crate::platform::kernel::RequirementOperand>(n)?;
                for _ in 0..n {
                    step(work)?;
                }
                Ok(())
            })?;
    }
    let mut descend = |ty: &mut TypeObjectDigest| -> Result<(), Diagnostic> {
        *ty = substitute_effect_type(types, *ty, bindings, requirements, depth + 1, work)?;
        Ok(())
    };
    match &mut object.form {
        TypeForm::Applied { arguments, .. } | TypeForm::ReadViewApplied { arguments, .. } => {
            for ty in arguments {
                descend(ty)?;
            }
        }
        TypeForm::StructuralRecord { fields }
        | TypeForm::OwnedProduct { fields }
        | TypeForm::OwnedChoice { cases: fields } => {
            for field in fields {
                descend(&mut field.ty)?;
            }
        }
        TypeForm::List { item }
        | TypeForm::Option { item }
        | TypeForm::Stream { item }
        | TypeForm::OwnedSequence { item } => descend(item)?,
        TypeForm::Map { key, value }
        | TypeForm::Result {
            ok: key,
            error: value,
        } => {
            descend(key)?;
            descend(value)?;
        }
        TypeForm::Function { parameters, result }
        | TypeForm::TaskFunction {
            parameters, result, ..
        } => {
            for ty in parameters {
                descend(ty)?;
            }
            descend(result)?;
        }
        _ => {}
    }
    let object = TypeObject::new(object.form)?;
    let (digest, _) = encode_type_object(&object)?;
    if !types.contains_key(&digest) {
        work.node::<(TypeObjectDigest, TypeObject)>()?;
    }
    types.entry(digest).or_insert(object);
    Ok(digest)
}

fn reserve_implementation_operand(
    operand: &crate::platform::kernel::ImplementationOperand,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    if let crate::platform::kernel::ImplementationOperand::Concrete {
        type_arguments,
        implementations,
        ..
    } = operand
    {
        work.reserve::<TypeObjectDigest>(type_arguments.len())?;
        work.reserve::<crate::platform::kernel::ImplementationOperand>(implementations.len())?;
        for child in implementations {
            reserve_implementation_operand(child, depth + 1, work)?;
        }
    }
    Ok(())
}

fn substitute_implementation_effects(
    operand: &mut crate::platform::kernel::ImplementationOperand,
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    bindings: &EffectBindings,
    requirements: &crate::platform::kernel::RequirementSubstitution,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    if let crate::platform::kernel::ImplementationOperand::Concrete {
        type_arguments,
        implementations,
        ..
    } = operand
    {
        for ty in type_arguments {
            *ty = substitute_effect_type(types, *ty, bindings, requirements, 0, work)?;
        }
        for child in implementations {
            substitute_implementation_effects(
                child,
                types,
                bindings,
                requirements,
                depth + 1,
                work,
            )?;
        }
    }
    Ok(())
}

fn substitute(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<TypeObjectDigest, Diagnostic> {
    substitute_in(types, ty, &TypeApplication::ordinary(bindings), depth, work)
}

fn substitute_in(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    application: &TypeApplication<'_>,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<TypeObjectDigest, Diagnostic> {
    let (result, selected) = substitute_inner(types, ty, application, depth, work)?;
    if depth == 0 && result != ty {
        if selected {
            if let Some(function) = application.function {
                if let Some(scopes) = application.scopes.filter(|scopes| !scopes.is_empty()) {
                    work.remember_scoped_application_substitution(
                        function,
                        ty,
                        result,
                        application.bindings,
                        scopes,
                    )?;
                } else {
                    work.remember_application_substitution(
                        function,
                        ty,
                        result,
                        application.bindings,
                    )?;
                }
            }
        } else if !matches!(
            types.get(&ty).map(|object| &object.form),
            Some(TypeForm::TypeParameter { .. })
        ) {
            work.remember_type_substitution(ty, result, application.bindings)?;
        }
    }
    Ok(result)
}

fn substitute_inner(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    ty: TypeObjectDigest,
    application: &TypeApplication<'_>,
    depth: usize,
    work: &mut Budget<'_>,
) -> Result<(TypeObjectDigest, bool), Diagnostic> {
    step(work)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(missing());
    }
    work.cloned_type(types.get(&ty).ok_or_else(missing)?)?;
    let mut object = types.get(&ty).cloned().ok_or_else(missing)?;
    let mut selected = false;
    let mut descend = |ty: &mut TypeObjectDigest| -> Result<(), Diagnostic> {
        let (resolved, associated) = substitute_inner(types, *ty, application, depth + 1, work)?;
        *ty = resolved;
        selected |= associated;
        Ok(())
    };
    match &mut object.form {
        TypeForm::Region { region } => descend(region)?,
        TypeForm::RelPtr { region, item } | TypeForm::RegionSlice { region, item } => {
            descend(region)?;
            descend(item)?;
        }
        TypeForm::RegionCap { region, mode } => {
            descend(region)?;
            if let crate::platform::kernel::RegionMode::Shared { scope } = mode {
                let resolved = resolve_scope(*scope, application.scopes)?;
                selected |= resolved != *scope;
                *scope = resolved;
            }
        }
        TypeForm::TypeParameter { parameter } => {
            return application
                .bindings
                .get(parameter)
                .copied()
                .map(|ty| (ty, false))
                .ok_or_else(missing);
        }
        TypeForm::AssociatedContractMember { .. }
        | TypeForm::AssociatedWitnessProjection { .. } => {
            let resolved = application
                .associated
                .ok_or_else(missing)?
                .resolve_leaf(&object.form)?
                .ok_or_else(missing)?;
            if !types.contains_key(&resolved) {
                return Err(missing());
            }
            return Ok((resolved, true));
        }
        TypeForm::AssociatedReadViewMember { read_scope, .. }
        | TypeForm::AssociatedReadViewProjection { read_scope, .. } => {
            let scope = resolve_scope(*read_scope, application.scopes)?;
            let selection = application
                .associated
                .ok_or_else(missing)?
                .resolve_selection(&object.form)?
                .ok_or_else(missing)?;
            let parameter = selection
                .binding()
                .scope_parameter
                .as_ref()
                .ok_or_else(missing)?;
            work.node::<(ReadScopeReference, ReadScopeArgument)>()?;
            let bindings = BTreeMap::from([(
                ReadScopeReference {
                    binder: ReadScopeBinder::AssociatedBinding {
                        implementation: selection.selected().implementation,
                        member: selection.binding().member,
                    },
                    parameter: parameter.id,
                },
                scope,
            )]);
            let result = substitute_in(
                types,
                selection.ty(),
                &TypeApplication {
                    bindings: &BTreeMap::new(),
                    associated: None,
                    function: None,
                    scopes: Some(&bindings),
                },
                depth + 1,
                work,
            )?;
            return Ok((result, true));
        }
        TypeForm::StructuralRecord { fields }
        | TypeForm::OwnedProduct { fields }
        | TypeForm::OwnedChoice { cases: fields } => {
            for field in fields {
                descend(&mut field.ty)?;
            }
        }
        TypeForm::Applied { arguments, .. } => {
            for argument in arguments {
                descend(argument)?;
            }
        }
        TypeForm::ReadViewApplied {
            arguments,
            read_scope,
            ..
        } => {
            for argument in arguments {
                descend(argument)?;
            }
            let scope = resolve_scope(*read_scope, application.scopes)?;
            selected |= scope != *read_scope;
            *read_scope = scope;
        }
        TypeForm::List { item }
        | TypeForm::Option { item }
        | TypeForm::Stream { item }
        | TypeForm::OwnedSequence { item } => descend(item)?,
        TypeForm::Map { key, value } => {
            descend(key)?;
            descend(value)?;
        }
        TypeForm::Result { ok, error } => {
            descend(ok)?;
            descend(error)?;
        }
        TypeForm::Function { parameters, result }
        | TypeForm::TaskFunction {
            parameters, result, ..
        } => {
            for parameter in parameters {
                descend(parameter)?;
            }
            descend(result)?;
        }
        _ => {}
    }
    let (identity, _) = encode_type_object(&object)?;
    step(work)?;
    if !types.contains_key(&identity) {
        work.node::<(TypeObjectDigest, TypeObject)>()?;
    }
    types.entry(identity).or_insert(object);
    Ok((identity, selected))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_in(
        types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
        form: TypeForm,
    ) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        types.insert(digest, object);
        digest
    }

    fn field(ty: TypeObjectDigest) -> crate::platform::kernel::StructuralTypeField {
        crate::platform::kernel::StructuralTypeField {
            name: crate::platform::kernel::Name::new("payload").unwrap(),
            ty,
        }
    }

    #[test]
    fn associated_templates_are_uninstantiated_and_need_an_exact_selection() {
        use crate::platform::kernel::{DeclarationReference, PackageId};
        use crate::platform::semantic_id::{
            AssociatedTypeMemberId, DeclarationId, ImplementationParameterId,
        };
        let declaration = DeclarationReference {
            package: PackageId::migrate(b"prepared-associated-template", 0),
            declaration: DeclarationId::migrate(b"prepared-associated-template", 0),
        };
        let member = AssociatedTypeMemberId::migrate(b"prepared-associated-template", 0);
        let parameter = ImplementationParameterId::migrate(b"prepared-associated-template", 0);
        let mut types = BTreeMap::new();
        let cell = type_in(&mut types, TypeForm::OwnedI64Cell);
        let contract_member = type_in(
            &mut types,
            TypeForm::AssociatedContractMember {
                contract: declaration,
                member,
            },
        );
        let projection = type_in(
            &mut types,
            TypeForm::AssociatedWitnessProjection {
                scope: declaration,
                parameter,
                contract: declaration,
                member,
            },
        );
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let mut work = Budget::new(&control);
        let mut known = BTreeMap::new();
        assert!(instantiated_type(&types, cell, &mut known, 0, &mut work).unwrap());
        for root in [contract_member, projection] {
            let composite = type_in(&mut types, TypeForm::OwnedSequence { item: root });
            for ty in [root, composite] {
                assert!(!instantiated_type(&types, ty, &mut known, 0, &mut work).unwrap());
                assert!(substitute(&mut types, ty, &BTreeMap::new(), 0, &mut work).is_err());
            }
        }
    }

    #[test]
    fn sequence_instruction_closure_derives_empty_borrowed_and_pop_types_in_untaken_code() {
        use std::sync::Arc;
        let parameter = TypeParameterId::migrate(b"prepared-sequence-roots", 0);
        let mut types = BTreeMap::new();
        let parameter_type = type_in(&mut types, TypeForm::TypeParameter { parameter });
        let cell = type_in(&mut types, TypeForm::OwnedI64Cell);
        let item = type_in(
            &mut types,
            TypeForm::OwnedProduct {
                fields: vec![field(parameter_type)],
            },
        );
        let sequence = type_in(&mut types, TypeForm::OwnedSequence { item });
        let pop_result = type_in(
            &mut types,
            TypeForm::OwnedChoice {
                cases: vec![field(sequence)],
            },
        );
        let instructions = [
            NormalizedInstruction::SequenceEmpty {
                sequence_type: sequence,
            },
            NormalizedInstruction::SequenceLength {
                sequence_type: sequence,
                source_local: 0,
            },
            NormalizedInstruction::SequencePush {
                sequence_type: sequence,
                source_local: 0,
            },
            NormalizedInstruction::SequencePop {
                sequence_type: sequence,
                result_type: pop_result,
                source_local: 0,
            },
            NormalizedInstruction::BorrowOwnedItem {
                sequence_type: sequence,
                source_local: 0,
                binding_local: 2,
                binding_type: item,
            },
        ];
        let concrete_item = TypeObject::new(TypeForm::OwnedProduct {
            fields: vec![field(cell)],
        })
        .unwrap();
        let concrete_item_digest = encode_type_object(&concrete_item).unwrap().0;
        let concrete_sequence = TypeObject::new(TypeForm::OwnedSequence {
            item: concrete_item_digest,
        })
        .unwrap();
        let concrete_sequence_digest = encode_type_object(&concrete_sequence).unwrap().0;
        let concrete_pop = TypeObject::new(TypeForm::OwnedChoice {
            cases: vec![field(concrete_sequence_digest)],
        })
        .unwrap();
        let concrete_pop_digest = encode_type_object(&concrete_pop).unwrap().0;
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        for instruction in instructions {
            let pop = matches!(instruction, NormalizedInstruction::SequencePop { .. });
            let mut actual = types.clone();
            let code = NormalizedCode {
                parameter_count: 0,
                local_count: 3,
                instructions: Arc::from([
                    NormalizedInstruction::Jump(2),
                    instruction,
                    NormalizedInstruction::Unit,
                ]),
            };
            calls(
                &code,
                &TypeApplication::ordinary(&BTreeMap::from([(parameter, cell)])),
                &mut actual,
                &mut BTreeSet::new(),
                &mut Budget::new(&control),
                &[],
                &[],
            )
            .unwrap();
            assert_eq!(actual.get(&concrete_item_digest), Some(&concrete_item));
            assert_eq!(
                actual.get(&concrete_sequence_digest),
                Some(&concrete_sequence)
            );
            if pop {
                assert_eq!(actual.get(&concrete_pop_digest), Some(&concrete_pop));
            }
        }
    }

    #[test]
    fn ordinary_sequence_reads_and_replacements_close_inactive_generic_type_roots() {
        use crate::platform::kernel::StructuralTypeField;
        use std::sync::Arc;
        let parameter = TypeParameterId::migrate(b"prepared-data-sequence-roots", 0);
        let mut types = BTreeMap::new();
        let item = type_in(&mut types, TypeForm::TypeParameter { parameter });
        let scalar = type_in(&mut types, TypeForm::I64);
        let sequence = type_in(&mut types, TypeForm::OwnedSequence { item });
        let replacement = type_in(
            &mut types,
            TypeForm::OwnedProduct {
                fields: vec![
                    StructuralTypeField {
                        name: crate::platform::kernel::Name::new("rest").unwrap(),
                        ty: sequence,
                    },
                    StructuralTypeField {
                        name: crate::platform::kernel::Name::new("value").unwrap(),
                        ty: item,
                    },
                ],
            },
        );
        let concrete_sequence = TypeObject::new(TypeForm::OwnedSequence { item: scalar }).unwrap();
        let concrete_sequence_digest = encode_type_object(&concrete_sequence).unwrap().0;
        let concrete_replacement = TypeObject::new(TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: crate::platform::kernel::Name::new("rest").unwrap(),
                    ty: concrete_sequence_digest,
                },
                StructuralTypeField {
                    name: crate::platform::kernel::Name::new("value").unwrap(),
                    ty: scalar,
                },
            ],
        })
        .unwrap();
        let concrete_replacement_digest = encode_type_object(&concrete_replacement).unwrap().0;
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        for instruction in [
            NormalizedInstruction::SequenceGet {
                sequence_type: sequence,
                source_local: 0,
            },
            NormalizedInstruction::SequenceReplace {
                sequence_type: sequence,
                result_type: replacement,
                source_local: 0,
            },
        ] {
            let replace = matches!(instruction, NormalizedInstruction::SequenceReplace { .. });
            let mut actual = types.clone();
            let code = NormalizedCode {
                parameter_count: 0,
                local_count: 1,
                instructions: Arc::from([
                    NormalizedInstruction::Jump(2),
                    instruction,
                    NormalizedInstruction::Unit,
                ]),
            };
            calls(
                &code,
                &TypeApplication::ordinary(&BTreeMap::from([(parameter, scalar)])),
                &mut actual,
                &mut BTreeSet::new(),
                &mut Budget::new(&control),
                &[],
                &[],
            )
            .unwrap();
            assert_eq!(
                actual.get(&concrete_sequence_digest),
                Some(&concrete_sequence)
            );
            if replace {
                assert_eq!(
                    actual.get(&concrete_replacement_digest),
                    Some(&concrete_replacement)
                );
            }
        }
    }

    #[test]
    fn empty_sequence_closure_requires_the_complete_item_type() {
        use std::sync::Arc;
        let mut types = BTreeMap::new();
        let missing_cell = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
            .unwrap()
            .0;
        let sequence = type_in(&mut types, TypeForm::OwnedSequence { item: missing_cell });
        let code = NormalizedCode {
            parameter_count: 0,
            local_count: 0,
            instructions: Arc::from([NormalizedInstruction::SequenceEmpty {
                sequence_type: sequence,
            }]),
        };
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let error = calls(
            &code,
            &TypeApplication::ordinary(&BTreeMap::new()),
            &mut types,
            &mut BTreeSet::new(),
            &mut Budget::new(&control),
            &[],
            &[],
        )
        .unwrap_err();
        assert_eq!(error.code, "normalized_instantiation_scope");
    }

    #[test]
    fn sequence_substitution_checks_the_complete_mixed_owned_nesting_depth() {
        let parameter = TypeParameterId::migrate(b"prepared-sequence-depth", 0);
        let mut types = BTreeMap::new();
        let parameter_type = type_in(&mut types, TypeForm::TypeParameter { parameter });
        let symbolic = type_in(
            &mut types,
            TypeForm::OwnedSequence {
                item: parameter_type,
            },
        );
        let mut closed = type_in(&mut types, TypeForm::OwnedI64Cell);
        for depth in 0..crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH - 1 {
            let form = match depth % 3 {
                0 => TypeForm::OwnedSequence { item: closed },
                1 => TypeForm::OwnedProduct {
                    fields: vec![field(closed)],
                },
                _ => TypeForm::OwnedChoice {
                    cases: vec![field(closed)],
                },
            };
            closed = type_in(&mut types, form);
        }
        check_product_substitution(
            types.clone(),
            symbolic,
            BTreeMap::from([(parameter, closed)]),
        )
        .unwrap();
        closed = type_in(&mut types, TypeForm::OwnedSequence { item: closed });
        let error =
            check_product_substitution(types, symbolic, BTreeMap::from([(parameter, closed)]))
                .unwrap_err();
        assert_eq!(error.code, "normalized_product_depth");
        assert_eq!(error.class, DiagnosticClass::Semantic);
    }

    #[test]
    fn derivation_reservations_fit_exactly_and_fail_before_one_over_growth() {
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let mut budget = Budget {
            steps: MAXIMUM_WORK - 1,
            bytes: 0,
            control: &control,
            type_lookup: type_lookup::TypeLookupBuilder::default(),
        };
        step(&mut budget).unwrap();
        assert_eq!(budget.steps, MAXIMUM_WORK);
        assert_eq!(
            step(&mut budget).unwrap_err().code,
            "normalized_instantiation_work"
        );
        let maximum = super::MAXIMUM_METADATA_BYTES;
        let mut bytes = maximum - 32;
        reserve_metadata(&mut bytes, 1, 32).unwrap();
        assert_eq!(bytes, maximum);
        assert_eq!(
            reserve_metadata(&mut bytes, 1, 1).unwrap_err().code,
            "normalized_instantiation_storage"
        );
        assert_eq!(bytes, maximum);
        assert_eq!(
            reserve_metadata(&mut bytes, usize::MAX, 32)
                .unwrap_err()
                .class,
            DiagnosticClass::Resource
        );
    }
}
