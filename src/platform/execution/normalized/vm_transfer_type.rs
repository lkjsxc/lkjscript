//! Resolve substitutions by matching already admitted types, without constructing
//! a TypeObject, copying names or allocating canonical encoding scratch at runtime.
use super::*;
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;

pub(in super::super::super) fn resolve(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    control: &ExecutionControl,
) -> Result<TypeObjectDigest, ExecutionError> {
    let mut work = Work { control, nodes: 0 };
    resolve_in(program, template, bindings, &mut work)
}

fn resolve_in(
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

fn matches(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    actual: TypeObjectDigest,
    bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<bool, ExecutionError> {
    work.visit(depth)?;
    let template = program.types.get(&template).ok_or_else(reject)?;
    if let TypeForm::TypeParameter { parameter } = &template.form {
        return Ok(bindings.get(parameter) == Some(&actual) && program.types.contains_key(&actual));
    }
    let actual = program.types.get(&actual).ok_or_else(reject)?;
    if template.contract_version != actual.contract_version {
        return Ok(false);
    }
    let mut child =
        |template, actual| matches(program, template, actual, bindings, depth + 1, work);
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
