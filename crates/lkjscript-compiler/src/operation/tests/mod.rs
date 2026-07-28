use crate::hir::EffectSet;
use crate::types::Type;

use super::Operation;
use crate::operation::instantiation::function;

mod buffers;
mod equality;
mod files;
mod hash;
mod truthful_effects;

#[test]
fn compiler_operation_order_agrees_with_stable_registry_identities() {
    assert_eq!(Operation::ALL.len(), lkjscript_contracts::OPERATION_COUNT);
    for (index, operation) in Operation::ALL.iter().copied().enumerate() {
        let record = lkjscript_contracts::operation_by_id(
            lkjscript_contracts::OperationIdentity::new(index as u16),
        );
        assert!(record.is_some(), "missing operation record {index}");
        let Some(record) = record else {
            continue;
        };
        assert_eq!(operation.identity(), record.identity);
        assert_eq!(
            camel_to_kebab(&format!("{operation:?}")),
            record.stable_name
        );
        let typed = operation.record();
        assert_eq!(
            canonical_type(&typed.type_scheme),
            record.semantics.type_scheme
        );
        assert_eq!(typed.arity, usize::from(record.semantics.arity));
        let variables = match &typed.type_scheme {
            Type::Forall { vars, .. } => vars.as_slice(),
            _ => &[],
        };
        assert_eq!(variables, record.semantics.generic_variables);
        if matches!(
            operation,
            Operation::DropResource
                | Operation::SysReadByte
                | Operation::SysWriteByte
                | Operation::SysReadInto
                | Operation::SysWriteFrom
                | Operation::SysFsync
                | Operation::SysTruncate
                | Operation::SysPoll
        ) {
            assert!(matches!(
                record.semantics.generic_constraints,
                [constraint]
                    if constraint.starts_with("resource:one-of(")
                        && constraint.ends_with(')')
            ));
        } else {
            assert!(record.semantics.generic_constraints.is_empty());
        }
        assert_eq!(typed.effects.bits(), record.semantics.effects.0);
        assert_eq!(
            typed.capability_requirements,
            record.semantics.capability_requirements
        );
    }
}

fn canonical_type(ty: &Type) -> String {
    match ty {
        Type::Never => "never".into(),
        Type::Unit => "unit".into(),
        Type::Bool => "bool".into(),
        Type::I64 => "i64".into(),
        Type::F64 => "f64".into(),
        Type::Str => "string".into(),
        Type::Buf => "buf".into(),
        Type::Bytes => "bytes".into(),
        Type::ByteVector => "byte-vector".into(),
        Type::ByteSlice => "byte-slice".into(),
        Type::ByteSliceMut => "byte-slice-mut".into(),
        Type::Path => "path".into(),
        Type::Symbol => "symbol".into(),
        Type::Capability(kind) => format!("capability {}", kind.as_str()),
        Type::Resource(kind) => kind.as_str().into(),
        Type::Product(name) | Type::Param(name) => name.clone(),
        Type::Enum {
            name, arguments, ..
        } => format!(
            "{} {}",
            name.rsplit(':').next().unwrap_or(name),
            arguments
                .iter()
                .map(canonical_type)
                .collect::<Vec<_>>()
                .join(" ")
        )
        .trim_end()
        .into(),
        Type::List(inner) => format!("list {}", canonical_type(inner)),
        Type::Fn { params, ret } => format!(
            "fn inputs {} output {}",
            params
                .iter()
                .map(canonical_type)
                .collect::<Vec<_>>()
                .join(" "),
            canonical_type(ret)
        ),
        Type::Forall { vars, body } => {
            format!("forall {} {}", vars.join(" "), canonical_type(body))
        }
    }
}

fn camel_to_kebab(name: &str) -> String {
    let mut output = String::new();
    for (index, character) in name.chars().enumerate() {
        if index > 0 && character.is_ascii_uppercase() {
            output.push('-');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}
