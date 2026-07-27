use crate::operation::instantiation::{forall, function};
use crate::operation::*;

pub(in crate::operation) fn value_signature(operation: Operation) -> Type {
    use lkjscript_core::CapabilityKind::{Arguments, Stdio};

    let i64_binary = || function(vec![Type::I64, Type::I64], Type::I64);
    let numeric_binary = || {
        forall(
            &["n"],
            function(
                vec![Type::Param("n".into()), Type::Param("n".into())],
                Type::Param("n".into()),
            ),
        )
    };
    let numeric_comparison = || {
        forall(
            &["n"],
            function(
                vec![Type::Param("n".into()), Type::Param("n".into())],
                Type::Bool,
            ),
        )
    };
    match operation {
        Operation::Add | Operation::Subtract | Operation::Multiply | Operation::Divide => {
            numeric_binary()
        }
        Operation::EqualValue | Operation::SameObject => forall(
            &["t"],
            function(
                vec![Type::Param("t".into()), Type::Param("t".into())],
                Type::Bool,
            ),
        ),
        Operation::ListEqual => {
            let item = Type::Param("t".into());
            let list = Type::List(Box::new(item));
            forall(&["t"], function(vec![list.clone(), list], Type::Bool))
        }
        Operation::F64BitsEqual => function(vec![Type::F64, Type::F64], Type::Bool),
        Operation::F64FromI64Exact => function(
            vec![Type::I64],
            crate::types::result_type(Type::F64, crate::types::numeric_error_type()),
        ),
        Operation::F64FromI64Rounded => function(vec![Type::I64], Type::F64),
        Operation::I64FromF64Exact | Operation::I64FromF64Trunc => function(
            vec![Type::F64],
            crate::types::result_type(Type::I64, crate::types::numeric_error_type()),
        ),
        Operation::Less | Operation::LessEqual | Operation::Greater | Operation::GreaterEqual => {
            numeric_comparison()
        }
        Operation::BitAnd | Operation::BitOr | Operation::BitXor => i64_binary(),
        Operation::Not => function(vec![Type::Bool], Type::Bool),
        Operation::And | Operation::Or => function(vec![Type::Bool, Type::Bool], Type::Bool),
        Operation::Cons => {
            let item = Type::Param("t".into());
            forall(
                &["t"],
                function(
                    vec![item.clone(), Type::List(Box::new(item.clone()))],
                    Type::List(Box::new(item)),
                ),
            )
        }
        Operation::Car => {
            let item = Type::Param("t".into());
            forall(
                &["t"],
                function(vec![Type::List(Box::new(item.clone()))], item),
            )
        }
        Operation::Cdr => {
            let item = Type::Param("t".into());
            forall(
                &["t"],
                function(
                    vec![Type::List(Box::new(item.clone()))],
                    Type::List(Box::new(item)),
                ),
            )
        }
        Operation::IsEmptyList => forall(
            &["t"],
            function(
                vec![Type::List(Box::new(Type::Param("t".into())))],
                Type::Bool,
            ),
        ),
        Operation::Print | Operation::WriteStr => {
            function(vec![Type::Capability(Stdio), Type::Str], Type::Unit)
        }
        Operation::Flush => function(vec![Type::Capability(Stdio)], Type::Unit),
        Operation::ReadByte => function(vec![Type::Capability(Stdio)], Type::I64),
        Operation::WriteByte => function(vec![Type::Capability(Stdio), Type::I64], Type::Unit),
        Operation::Exit => function(vec![Type::I64], Type::Unit),
        Operation::EmptyStr => function(Vec::new(), Type::Str),
        Operation::ArgCount => function(vec![Type::Capability(Arguments)], Type::I64),
        Operation::Arg => function(
            vec![Type::Capability(Arguments), Type::I64],
            crate::types::option_type(Type::Str),
        ),
        _ => unreachable!("operation signature family mismatch"),
    }
}
