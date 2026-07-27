use crate::operation::instantiation::{forall, function, generic_result};
use crate::operation::*;

pub(in crate::operation) fn variant_signature(operation: Operation) -> Type {
    match operation {
        Operation::Ok => {
            let success = Type::Param("t".into());
            let failure = Type::Param("e".into());
            forall(
                &["t", "e"],
                function(
                    vec![success.clone()],
                    crate::types::result_type(success, failure),
                ),
            )
        }
        Operation::Err => {
            let success = Type::Param("t".into());
            let failure = Type::Param("e".into());
            forall(
                &["t", "e"],
                function(
                    vec![failure.clone()],
                    crate::types::result_type(success, failure),
                ),
            )
        }
        Operation::IsOk => {
            let result = generic_result();
            forall(&["t", "e"], function(vec![result], Type::Bool))
        }
        Operation::UnwrapOk => {
            let result = generic_result();
            forall(&["t", "e"], function(vec![result], Type::Param("t".into())))
        }
        Operation::UnwrapErr => {
            let result = generic_result();
            forall(&["t", "e"], function(vec![result], Type::Param("e".into())))
        }
        Operation::Some => {
            let value = Type::Param("t".into());
            forall(
                &["t"],
                function(vec![value.clone()], crate::types::option_type(value)),
            )
        }
        Operation::IsSome => {
            let value = Type::Param("t".into());
            forall(
                &["t"],
                function(vec![crate::types::option_type(value)], Type::Bool),
            )
        }
        Operation::UnwrapSome => {
            let value = Type::Param("t".into());
            forall(
                &["t"],
                function(vec![crate::types::option_type(value.clone())], value),
            )
        }
        _ => unreachable!("operation signature family mismatch"),
    }
}
