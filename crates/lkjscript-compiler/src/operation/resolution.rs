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
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "{}: numeric operands must have one exact type, got {left} and {right}",
                        self.name()
                    ));
                }
                match left {
                    Type::I64 => Type::I64,
                    Type::F64 => Type::F64,
                    other => {
                        return Err(format!("{}: expected i64 or f64, got {other}", self.name()));
                    }
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
                if left != right {
                    return Err(format!(
                        "{}: numeric operands must have one exact type, got {left} and {right}",
                        self.name()
                    ));
                }
                if !both_numeric(left, right) {
                    return Err(format!(
                        "{}: expected numeric operands, got {left} and {right}",
                        self.name()
                    ));
                }
                Type::Bool
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

    /// Whether the canonical direct-operation result scheme can produce `expected` for at least one
    /// argument shape. This is a provisional discovery predicate; submitted arguments still pass
    /// `resolve_types` and all canonical ownership checks before publication.
    pub(crate) fn direct_result_matches(self, expected: &Type) -> bool {
        if !self.supports_direct_operation_expression() {
            return false;
        }
        let signature = self.signature();
        let (variables, parameters, result) = match &signature {
            Type::Forall { vars, body } => {
                let Type::Fn { params, ret } = body.as_ref() else {
                    return false;
                };
                (vars.as_slice(), params.as_slice(), ret.as_ref())
            }
            Type::Fn { params, ret } => (&[][..], params.as_slice(), ret.as_ref()),
            _ => return false,
        };
        let mut substitutions = std::collections::HashMap::new();
        if bind_expected_result(result, expected, variables, &mut substitutions).is_err() {
            return false;
        }
        for variable in variables {
            if substitutions.contains_key(variable) {
                continue;
            }
            let Some(inferred) = self.canonical_unbound_witness(variable, parameters) else {
                return false;
            };
            substitutions.insert(variable.clone(), inferred);
        }
        let arguments = parameters
            .iter()
            .map(|parameter| parameter.subst(&substitutions))
            .collect::<Vec<_>>();
        self.resolve_types(&arguments)
            .is_ok_and(|(_, result)| result == *expected)
    }
}

fn bind_expected_result(
    pattern: &Type,
    expected: &Type,
    variables: &[String],
    substitutions: &mut std::collections::HashMap<String, Type>,
) -> Result<(), ()> {
    let mut pending = vec![(pattern, expected)];
    while let Some((pattern, value)) = pending.pop() {
        match (pattern, value) {
            (Type::Param(parameter), value)
                if variables.iter().any(|variable| variable == parameter) =>
            {
                if substitutions
                    .get(parameter)
                    .is_some_and(|previous| previous != value)
                {
                    return Err(());
                }
                substitutions
                    .entry(parameter.clone())
                    .or_insert_with(|| value.clone());
            }
            (Type::List(pattern), Type::List(value)) => pending.push((pattern, value)),
            (
                Type::Enum {
                    id: pattern_id,
                    arguments: patterns,
                },
                Type::Enum {
                    id: value_id,
                    arguments: values,
                },
            ) if pattern_id == value_id && patterns.len() == values.len() => {
                pending.extend(patterns.iter().zip(values));
            }
            (pattern, value) if pattern == value => {}
            _ => return Err(()),
        }
    }
    Ok(())
}

impl Operation {
    fn canonical_unbound_witness(self, variable: &str, parameters: &[Type]) -> Option<Type> {
        if !parameters
            .iter()
            .any(|parameter| type_contains_parameter(parameter, variable))
        {
            return None;
        }
        if self == Operation::SameObject {
            return Some(Type::Resource(lkjscript_core::ResourceKind::InputStream));
        }
        let semantics = lkjscript_contracts::operation_semantics_by_id(self.identity())?;
        if let [constraint] = semantics.generic_constraints {
            let values = constraint
                .strip_prefix("resource:one-of(")?
                .strip_suffix(')')?;
            let kind = values
                .split(',')
                .next()
                .and_then(lkjscript_core::ResourceKind::parse)?;
            return Some(Type::Resource(kind));
        }
        Some(Type::I64)
    }
}

fn type_contains_parameter(root: &Type, variable: &str) -> bool {
    let mut pending = vec![root];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Param(name) if name == variable => return true,
            Type::List(inner) => pending.push(inner),
            Type::Enum { arguments, .. } => pending.extend(arguments),
            Type::Fn { params, ret } => {
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => pending.push(body),
            _ => {}
        }
    }
    false
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
