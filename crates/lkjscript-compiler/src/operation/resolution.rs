use crate::operation::instantiation::{
    both_numeric, callable_arity, instantiate_result, supports_value_equality,
};
use crate::operation::*;

impl Operation {
    pub fn resolve_types(self, arguments: &[Type]) -> Result<(Type, Type), String> {
        let expected = callable_arity(&self.signature())
            .ok_or_else(|| format!("{} has no callable signature", self.name()))?;
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
                                "{}: expected I64 or F64, got {other:?}",
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
                        "equal-value: operands must have the same type, got {left:?} and {right:?}"
                    ));
                }
                if !supports_value_equality(left) {
                    return Err(format!(
                        "equal-value: type {left:?} does not support value equality"
                    ));
                }
                Type::Bool
            }
            Self::SameObject => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "same-object: operands must have the same type, got {left:?} and {right:?}"
                    ));
                }
                if !matches!(left, Type::Buf | Type::Handle) {
                    return Err(format!(
                        "same-object: type {left:?} does not have object identity"
                    ));
                }
                Type::Bool
            }
            Self::ListEqual => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "list-equal: operands must have the same type, got {left:?} and {right:?}"
                    ));
                }
                let Type::List(item) = left else {
                    return Err(format!("list-equal: expected List, got {left:?}"));
                };
                if !supports_value_equality(item) {
                    return Err(format!(
                        "list-equal: element type {item:?} does not support value equality"
                    ));
                }
                Type::Bool
            }
            Self::F64BitsEqual => {
                if arguments == [Type::F64, Type::F64] {
                    Type::Bool
                } else {
                    return Err(format!(
                        "f64-bits-equal: expected F64 and F64, got {:?} and {:?}",
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
                        "{}: expected numeric operands, got {left:?} and {right:?}",
                        self.name()
                    ));
                }
            }
            _ => instantiate_result(self.name(), self.signature(), arguments)?,
        };
        let resolved = Type::Fn {
            params: arguments.to_vec(),
            ret: Box::new(result.clone()),
        };
        Ok((resolved, result))
    }
}
