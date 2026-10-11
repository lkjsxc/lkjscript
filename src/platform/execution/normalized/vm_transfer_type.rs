//! Resolve substitutions by matching already admitted types, without constructing
//! a TypeObject, copying names or allocating canonical encoding scratch at runtime.
use super::super::super::prepare::{
    ClosedTypeEnvironment, NormalizedImplementationArgument, PreparedApplicationContext,
};
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum OrdinaryBindings<'a> {
    Map(&'a BTreeMap<TypeParameterId, TypeObjectDigest>),
    Selected(&'a NormalizedImplementationArgument),
}

#[derive(Clone, Copy)]
struct TypeContext<'a> {
    ordinary: OrdinaryBindings<'a>,
    associated: ClosedTypeEnvironment<'a>,
}

impl<'a> TypeContext<'a> {
    fn ordinary(bindings: &'a BTreeMap<TypeParameterId, TypeObjectDigest>) -> Self {
        Self {
            ordinary: OrdinaryBindings::Map(bindings),
            associated: ClosedTypeEnvironment::forbidden(),
        }
    }

    fn selected(
        program: &NormalizedProgram,
        selected: &'a NormalizedImplementationArgument,
        work: &mut Work<'_>,
    ) -> Result<Self, ExecutionError> {
        if selected.implementation_type_parameters.len()
            != selected.implementation_type_arguments.len()
            || selected.implementation_type_parameter_constraints.len()
                != selected.implementation_type_arguments.len()
        {
            return Err(reject());
        }
        for (position, parameter) in selected.implementation_type_parameters.iter().enumerate() {
            for previous in &selected.implementation_type_parameters[..position] {
                work.visit(0)?;
                if previous == parameter {
                    return Err(reject());
                }
            }
        }
        for ty in selected.implementation_type_arguments.iter() {
            if resolve_in(program, *ty, &BTreeMap::new(), work)? != *ty {
                return Err(reject());
            }
        }
        Ok(Self {
            ordinary: OrdinaryBindings::Selected(selected),
            // Stored member RHSs are projection-free. Their ordinary formals
            // belong to this exact implementation scheme; a prerequisite does
            // not authorize interpreting a stored RHS as a new projection.
            associated: ClosedTypeEnvironment::forbidden(),
        })
    }

    fn parameter(self, parameter: TypeParameterId) -> Option<TypeObjectDigest> {
        match self.ordinary {
            OrdinaryBindings::Map(bindings) => bindings.get(&parameter).copied(),
            OrdinaryBindings::Selected(selected) => selected
                .implementation_type_parameters
                .iter()
                .position(|id| *id == parameter)
                .map(|position| selected.implementation_type_arguments[position]),
        }
    }
}

pub(in super::super::super) fn context_error(error: Diagnostic) -> ExecutionError {
    match error.class {
        DiagnosticClass::Cancelled => {
            ExecutionError::new(ExecutionFailureClass::Cancelled, error.code, error.message)
        }
        DiagnosticClass::Resource => ExecutionError::resource(error.code, error.message),
        _ => reject(),
    }
}

/// An application hit binds both the complete ordinary map and the exact
/// catalogued witness selection. A miss only falls back for ordinary templates.
pub(in super::super::super) fn resolve_application(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    context: &PreparedApplicationContext<'_>,
    control: &ExecutionControl,
) -> Result<TypeObjectDigest, ExecutionError> {
    let mut work = Work { control, nodes: 0 };
    resolve_application_in(program, template, context, &mut work)
}

pub(super) fn resolve_application_in(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    context: &PreparedApplicationContext<'_>,
    work: &mut Work<'_>,
) -> Result<TypeObjectDigest, ExecutionError> {
    work.control.check()?;
    if !std::ptr::eq(program, context.program()) {
        return Err(reject());
    }
    if let Some(actual) = program
        .prepared_type_lookup
        .get_application(
            program.value_origin,
            context.function(),
            template,
            context.ordinary(),
        )
        .map_err(|()| reject())?
    {
        let exact = matches_context(
            program,
            template,
            actual,
            TypeContext {
                ordinary: OrdinaryBindings::Map(context.ordinary()),
                associated: *context.environment(),
            },
            0,
            work,
        )?;
        work.control.check()?;
        return if exact { Ok(actual) } else { Err(reject()) };
    }
    if contains_associated(program, template, 0, work)? {
        return Err(reject());
    }
    resolve_in(program, template, context.ordinary(), work)
}

pub(in super::super::super) fn contains_associated_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    contains_associated(program, ty, 0, &mut Work { control, nodes: 0 })
}

fn contains_associated(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<bool, ExecutionError> {
    work.visit(depth)?;
    let object = program.types.get(&ty).ok_or_else(reject)?;
    if matches!(
        object.form,
        TypeForm::AssociatedContractMember { .. } | TypeForm::AssociatedWitnessProjection { .. }
    ) {
        return Ok(true);
    }
    let mut child = |ty| contains_associated(program, ty, depth + 1, work);
    match &object.form {
        TypeForm::StructuralRecord { fields }
        | TypeForm::OwnedProduct { fields }
        | TypeForm::OwnedChoice { cases: fields } => {
            for field in fields {
                if child(field.ty)? {
                    return Ok(true);
                }
            }
        }
        TypeForm::Applied { arguments, .. } => {
            for ty in arguments {
                if child(*ty)? {
                    return Ok(true);
                }
            }
        }
        TypeForm::Function { parameters, result }
        | TypeForm::TaskFunction {
            parameters, result, ..
        } => {
            for ty in parameters {
                if child(*ty)? {
                    return Ok(true);
                }
            }
            return child(*result);
        }
        TypeForm::List { item }
        | TypeForm::OwnedSequence { item }
        | TypeForm::Option { item }
        | TypeForm::Stream { item } => return child(*item),
        TypeForm::Map { key, value } => return Ok(child(*key)? || child(*value)?),
        TypeForm::Result { ok, error } => return Ok(child(*ok)? || child(*error)?),
        _ => {}
    }
    Ok(false)
}

fn catalogued(
    program: &NormalizedProgram,
    selected: &NormalizedImplementationArgument,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<(), ExecutionError> {
    work.visit(depth)?;
    let canonical = program
        .implementation_applications
        .get(selected.identity as usize)
        .ok_or_else(reject)?;
    if canonical.identity != selected.identity
        || !std::sync::Arc::ptr_eq(canonical, selected)
        || depth.saturating_add(selected.depth)
            > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
    {
        return Err(reject());
    }
    Ok(())
}

pub(super) fn validate_associated_bindings_in(
    program: &NormalizedProgram,
    selected: &NormalizedImplementationArgument,
    work: &mut Work<'_>,
) -> Result<(), ExecutionError> {
    catalogued(program, selected, 0, work)?;
    if selected.implementation_generation
        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        || selected.contract_generation != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
    {
        return Err(reject());
    }
    let context = TypeContext::selected(program, selected, work)?;
    for (position, binding) in selected.associated_types.iter().enumerate() {
        work.visit(0)?;
        for previous in &selected.associated_types[..position] {
            work.visit(0)?;
            if previous.member == binding.member {
                return Err(reject());
            }
        }
        if !program.is_owned_type(binding.ty)
            || !matches_context(program, binding.template, binding.ty, context, 0, work)?
        {
            return Err(reject());
        }
    }
    Ok(())
}

pub(in super::super::super) fn validate_associated_bindings(
    program: &NormalizedProgram,
    selected: &NormalizedImplementationArgument,
    control: &ExecutionControl,
    nodes: &mut u64,
) -> Result<(), ExecutionError> {
    let mut work = Work {
        control,
        nodes: *nodes,
    };
    let result = validate_associated_bindings_in(program, selected, &mut work);
    *nodes = work.nodes;
    result
}

pub(in super::super::super) fn resolve(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    control: &ExecutionControl,
) -> Result<TypeObjectDigest, ExecutionError> {
    let mut work = Work { control, nodes: 0 };
    resolve_in(program, template, bindings, &mut work)
}

pub(super) fn resolve_in(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    work: &mut Work<'_>,
) -> Result<TypeObjectDigest, ExecutionError> {
    // Closed roots and direct parameters already have a bounded fast path.
    // Positive composite entries are private to this prepared program and the
    // complete binding context; reads neither clone keys nor grow the index.
    if !bindings.is_empty()
        && !matches!(
            program.types.get(&template).map(|object| &object.form),
            Some(TypeForm::TypeParameter { .. })
        )
    {
        work.control.check()?;
        if let Some(actual) = program
            .prepared_type_lookup
            .get(program.value_origin, template, bindings)
            .map_err(|()| reject())?
        {
            // Recheck the current shape even on a hit. A divergent synthetic
            // clone or stale type table cannot turn derived metadata into proof.
            let exact = matches(program, template, actual, bindings, 0, work)?;
            work.control.check()?;
            return if exact { Ok(actual) } else { Err(reject()) };
        }
    }
    scan(program, template, bindings, work)
}

/// Legacy raw/synthetic contexts may be outside the prepared callable closure.
/// Preserve their independent bounded matching behavior without caching misses.
fn scan(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    work: &mut Work<'_>,
) -> Result<TypeObjectDigest, ExecutionError> {
    // Closed types dominate calls and have the same admitted identity. Check this
    // candidate first so their cost does not depend on unrelated program types.
    if matches(program, template, template, bindings, 0, work)? {
        return Ok(template);
    }
    if let TypeForm::TypeParameter { parameter } =
        &program.types.get(&template).ok_or_else(reject)?.form
    {
        return bindings
            .get(parameter)
            .copied()
            .filter(|ty| program.types.contains_key(ty))
            .ok_or_else(reject);
    }
    for candidate in program.types.keys() {
        if *candidate != template && matches(program, template, *candidate, bindings, 0, work)? {
            return Ok(*candidate);
        }
    }
    Err(reject())
}

#[cfg(test)]
#[path = "vm_type_lookup_tests.rs"]
mod lookup_tests;

#[cfg(test)]
#[path = "vm_associated_type_tests.rs"]
mod associated_tests;

fn matches(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    actual: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<bool, ExecutionError> {
    matches_context(
        program,
        template,
        actual,
        TypeContext::ordinary(bindings),
        depth,
        work,
    )
}

fn matches_context(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    actual: TypeObjectDigest,
    context: TypeContext<'_>,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<bool, ExecutionError> {
    work.visit(depth)?;
    let template = program.types.get(&template).ok_or_else(reject)?;
    // Resolve a member before comparing generations or ordinary forms. The
    // stored output is derived metadata; its original RHS is checked again in
    // the selected implementation's own scope and scheme actuals on every hit.
    if let Some(selection) = context
        .associated
        .resolve_selection(&template.form)
        .map_err(context_error)?
    {
        if template.contract_version
            != crate::platform::kernel::contract::OWNED_ASSOCIATED_CONTRACT_VERSION
        {
            return Err(reject());
        }
        catalogued(program, selection.selected(), depth + 1, work)?;
        if selection.ty() != actual || !program.is_owned_type(actual) {
            return Ok(false);
        }
        return matches_context(
            program,
            selection.binding().template,
            actual,
            TypeContext::selected(program, selection.selected(), work)?,
            depth + 1,
            work,
        );
    }
    if let TypeForm::TypeParameter { parameter } = &template.form {
        return Ok(
            context.parameter(*parameter) == Some(actual) && program.types.contains_key(&actual)
        );
    }
    let actual = program.types.get(&actual).ok_or_else(reject)?;
    if template.contract_version != actual.contract_version {
        return Ok(false);
    }
    let mut child =
        |template, actual| matches_context(program, template, actual, context, depth + 1, work);
    match (&template.form, &actual.form) {
        (TypeForm::Unit, TypeForm::Unit)
        | (TypeForm::Bool, TypeForm::Bool)
        | (TypeForm::I64, TypeForm::I64)
        | (TypeForm::F64, TypeForm::F64)
        | (TypeForm::Bytes, TypeForm::Bytes)
        | (TypeForm::Text, TypeForm::Text)
        | (TypeForm::StaticText, TypeForm::StaticText)
        | (TypeForm::Secret, TypeForm::Secret)
        | (TypeForm::ByteBuffer, TypeForm::ByteBuffer)
        | (TypeForm::OwnedI64Cell, TypeForm::OwnedI64Cell) => Ok(true),
        (TypeForm::Named { declaration: t }, TypeForm::Named { declaration: a }) => Ok(t == a),
        (
            TypeForm::CapabilityResource { interface: t },
            TypeForm::CapabilityResource { interface: a },
        ) => Ok(t == a),
        (TypeForm::StructuralRecord { fields: t }, TypeForm::StructuralRecord { fields: a })
        | (TypeForm::OwnedProduct { fields: t }, TypeForm::OwnedProduct { fields: a })
        | (TypeForm::OwnedChoice { cases: t }, TypeForm::OwnedChoice { cases: a }) => {
            if t.len() != a.len() {
                return Ok(false);
            }
            for (t, a) in t.iter().zip(a) {
                if t.name != a.name || !child(t.ty, a.ty)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (
            TypeForm::Applied {
                declaration: td,
                arguments: t,
            },
            TypeForm::Applied {
                declaration: ad,
                arguments: a,
            },
        ) => {
            if td != ad || t.len() != a.len() {
                return Ok(false);
            }
            for (t, a) in t.iter().zip(a) {
                if !child(*t, *a)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (TypeForm::List { item: t }, TypeForm::List { item: a })
        | (TypeForm::OwnedSequence { item: t }, TypeForm::OwnedSequence { item: a })
        | (TypeForm::Option { item: t }, TypeForm::Option { item: a })
        | (TypeForm::Stream { item: t }, TypeForm::Stream { item: a }) => child(*t, *a),
        (TypeForm::Map { key: tk, value: tv }, TypeForm::Map { key: ak, value: av }) => {
            Ok(child(*tk, *ak)? && child(*tv, *av)?)
        }
        (TypeForm::Result { ok: to, error: te }, TypeForm::Result { ok: ao, error: ae }) => {
            Ok(child(*to, *ao)? && child(*te, *ae)?)
        }
        (
            TypeForm::Function {
                parameters: t,
                result: tr,
            },
            TypeForm::Function {
                parameters: a,
                result: ar,
            },
        ) => {
            if t.len() != a.len() {
                return Ok(false);
            }
            for (t, a) in t.iter().zip(a) {
                if !child(*t, *a)? {
                    return Ok(false);
                }
            }
            child(*tr, *ar)
        }
        (
            TypeForm::TaskFunction {
                parameters: t,
                result: tr,
                effect: te,
            },
            TypeForm::TaskFunction {
                parameters: a,
                result: ar,
                effect: ae,
            },
        ) => {
            if !te.is_closed() || te != ae || t.len() != a.len() {
                return Ok(false);
            }
            for (t, a) in t.iter().zip(a) {
                if !child(*t, *a)? {
                    return Ok(false);
                }
            }
            child(*tr, *ar)
        }
        _ => Ok(false),
    }
}
