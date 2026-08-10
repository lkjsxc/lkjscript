use crate::operation::instantiation::{
    both_numeric, instantiate_result, supports_list_element_equality, supports_value_equality,
};
use crate::operation::*;

impl Operation {
    pub(crate) fn resolve_types(self, arguments: &[Type]) -> Result<(Type, Type), String> {
        let record = self.record();
        let expected = record.arity;
        if arguments.len() != expected {
            return Err(format!(
                "{}: expected {expected} args, got {}",
                self.name(),
                arguments.len()
            ));
        }
        let result = match self {
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => {
                let mut saw_f64 = false;
                for argument in arguments {
                    match argument {
                        Type::I64 => {}
                        Type::F64 => saw_f64 = true,
                        other => {
                            return Err(format!(
                                "{}: expected i64 or f64, got {other}",
                                self.name()
                            ));
                        }
                    }
                }
                if saw_f64 {
                    Type::F64
                } else {
                    Type::I64
                }
            }
            Self::EqualValue => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "equal-value: operands must have the same type, got {left} and {right}"
                    ));
                }
                if !supports_value_equality(left) && !matches!(left, Type::Param(_)) {
                    return Err(format!(
                        "equal-value: type {left} does not support value equality"
                    ));
                }
                Type::Bool
            }
            Self::SameObject => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "is-same-object: operands must have the same type, got {left} and {right}"
                    ));
                }
                if !matches!(left, Type::Resource(_)) {
                    return Err(format!(
                        "is-same-object: type {left} does not have object identity"
                    ));
                }
                Type::Bool
            }
            Self::ListEqual => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "equal-list: operands must have the same type, got {left} and {right}"
                    ));
                }
                let Type::List(item) = left else {
                    return Err(format!("equal-list: expected list, got {left}"));
                };
                if !supports_list_element_equality(item) {
                    return Err(format!(
                        "equal-list: element type {item} does not support value equality"
                    ));
                }
                Type::Bool
            }
            Self::F64BitsEqual => {
                if arguments == [Type::F64, Type::F64] {
                    Type::Bool
                } else {
                    return Err(format!(
                        "equal-f64-bits: expected F64 and F64, got {:?} and {:?}",
                        arguments[0], arguments[1]
                    ));
                }
            }
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => {
                let left = &arguments[0];
                let right = &arguments[1];
                if both_numeric(left, right) {
                    Type::Bool
                } else {
                    return Err(format!(
                        "{}: expected numeric operands, got {left} and {right}",
                        self.name()
                    ));
                }
            }
            Self::DropResource
            | Self::SysReadByte
            | Self::SysWriteByte
            | Self::SysReadInto
            | Self::SysWriteFrom
            | Self::SysFsync
            | Self::SysTruncate
            | Self::SysPoll => {
                validate_resource_operation(self, arguments)?;
                instantiate_result(self.name(), record.type_scheme, arguments)?
            }
            _ => instantiate_result(self.name(), record.type_scheme, arguments)?,
        };
        let resolved = Type::Fn {
            params: arguments.to_vec(),
            ret: Box::new(result.clone()),
        };
        Ok((resolved, result))
    }
}

fn validate_resource_operation(operation: Operation, arguments: &[Type]) -> Result<(), String> {
    let Some(Type::Resource(kind)) = arguments.first() else {
        return Err(format!(
            "{}: first argument must be a typed resource",
            operation.name()
        ));
    };
    let vocabulary = lkjscript_contracts::operation_by_id(operation.identity())
        .ok_or_else(|| format!("{}: operation vocabulary is missing", operation.name()))?;
    let [constraint] = vocabulary.semantics.generic_constraints else {
        return Err(format!(
            "{}: exact resource constraint is missing",
            operation.name()
        ));
    };
    let values = constraint
        .strip_prefix("resource:one-of(")
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| format!("{}: invalid resource constraint", operation.name()))?;
    let mut allowed = Vec::new();
    for value in values.split(',') {
        let parsed = lkjscript_core::ResourceKind::parse(value)
            .ok_or_else(|| format!("{}: unknown resource kind {value}", operation.name()))?;
        if allowed.contains(&parsed) {
            return Err(format!(
                "{}: duplicate resource kind {value}",
                operation.name()
            ));
        }
        allowed.push(parsed);
    }
    if allowed.contains(kind) {
        Ok(())
    } else {
        Err(format!(
            "{} does not accept resource kind {}",
            operation.name(),
            kind.as_str()
        ))
    }
}
