use crate::operation::instantiation::{forall, function, generic_result};
use crate::operation::*;

pub(in crate::operation) fn variant_signature(operation: Operation) -> Type {
    match operation {
        Operation::Ok => {
            let success = Type::Param("T".into());
            let failure = Type::Param("E".into());
            forall(
                &["T", "E"],
                function(
                    vec![success.clone()],
                    Type::Result(Box::new(success), Box::new(failure)),
                ),
            )
        }
        Operation::Err => {
            let success = Type::Param("T".into());
            let failure = Type::Param("E".into());
            forall(
                &["T", "E"],
                function(
                    vec![failure.clone()],
                    Type::Result(Box::new(success), Box::new(failure)),
                ),
            )
        }
        Operation::IsOk => {
            let result = generic_result();
            forall(&["T", "E"], function(vec![result], Type::Bool))
        }
        Operation::UnwrapOk => {
            let result = generic_result();
            forall(&["T", "E"], function(vec![result], Type::Param("T".into())))
        }
        Operation::UnwrapErr => {
            let result = generic_result();
            forall(&["T", "E"], function(vec![result], Type::Param("E".into())))
        }
        Operation::Some => {
            let value = Type::Param("T".into());
            forall(
                &["T"],
                function(vec![value.clone()], Type::Option(Box::new(value))),
            )
        }
        Operation::IsSome => {
            let value = Type::Param("T".into());
            forall(
                &["T"],
                function(vec![Type::Option(Box::new(value))], Type::Bool),
            )
        }
        Operation::UnwrapSome => {
            let value = Type::Param("T".into());
            forall(
                &["T"],
                function(vec![Type::Option(Box::new(value.clone()))], value),
            )
        }
        _ => unreachable!("operation signature family mismatch"),
    }
}
