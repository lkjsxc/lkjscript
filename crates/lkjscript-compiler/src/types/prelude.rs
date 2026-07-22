//! Typed prelude: sized numerics + polymorphic containers (no Any).

use super::prelude_sys::{install_result_helpers, install_sys};
use super::ty::Type;
use std::collections::HashMap;

fn forall(vars: &[&str], body: Type) -> Type {
    Type::Forall {
        vars: vars.iter().map(|s| (*s).to_string()).collect(),
        body: Box::new(body),
    }
}

fn fn_ty(params: Vec<Type>, ret: Type) -> Type {
    Type::Fn {
        params,
        ret: Box::new(ret),
    }
}

pub fn prelude() -> HashMap<String, Type> {
    let mut m = HashMap::new();
    let i64_bin = fn_ty(vec![Type::I64, Type::I64], Type::I64);
    let f64_bin = fn_ty(vec![Type::F64, Type::F64], Type::F64);
    let i64_cmp = fn_ty(vec![Type::I64, Type::I64], Type::Bool);
    let f64_cmp = fn_ty(vec![Type::F64, Type::F64], Type::Bool);
    for n in ["+", "-", "*", "/", "div", "bit-and", "bit-or", "bit-xor"] {
        m.insert(n.into(), i64_bin.clone());
    }
    for n in ["f+", "f-", "f*", "f/"] {
        m.insert(n.into(), f64_bin.clone());
    }
    for n in [
        "=", "!=", "<", "<=", ">", ">=", "lt", "le", "gt", "ge", "eq", "lte", "gte",
    ] {
        m.insert(n.into(), i64_cmp.clone());
    }
    for n in ["f=", "f!=", "f<", "f<=", "f>", "f>="] {
        m.insert(n.into(), f64_cmp.clone());
    }
    m.insert(
        "nil?".into(),
        forall(&["T"], fn_ty(vec![Type::Param("T".into())], Type::Bool)),
    );
    m.insert("not".into(), fn_ty(vec![Type::Bool], Type::Bool));
    m.insert(
        "and".into(),
        fn_ty(vec![Type::Bool, Type::Bool], Type::Bool),
    );
    m.insert("or".into(), fn_ty(vec![Type::Bool, Type::Bool], Type::Bool));

    let t = Type::Param("T".into());
    m.insert(
        "cons".into(),
        forall(
            &["T"],
            fn_ty(
                vec![t.clone(), Type::List(Box::new(t.clone()))],
                Type::List(Box::new(t.clone())),
            ),
        ),
    );
    m.insert(
        "car".into(),
        forall(
            &["T"],
            fn_ty(vec![Type::List(Box::new(t.clone()))], t.clone()),
        ),
    );
    m.insert(
        "cdr".into(),
        forall(
            &["T"],
            fn_ty(
                vec![Type::List(Box::new(t.clone()))],
                Type::List(Box::new(t.clone())),
            ),
        ),
    );
    m.insert(
        "null?".into(),
        forall(&["T"], fn_ty(vec![Type::List(Box::new(t))], Type::Bool)),
    );

    m.insert("print".into(), fn_ty(vec![Type::Str], Type::Nil));
    m.insert("flush".into(), fn_ty(vec![], Type::Nil));
    m.insert("write-str".into(), fn_ty(vec![Type::Str], Type::Nil));
    m.insert("empty-str".into(), fn_ty(vec![], Type::Str));
    m.insert("argc".into(), fn_ty(vec![], Type::I64));
    m.insert("read-byte".into(), fn_ty(vec![], Type::I64));
    m.insert("write-byte".into(), fn_ty(vec![Type::I64], Type::Nil));
    m.insert("arg".into(), fn_ty(vec![Type::I64], Type::Str));
    m.insert("exit".into(), fn_ty(vec![Type::I64], Type::Nil));
    m.insert("buf-new".into(), fn_ty(vec![Type::I64], Type::Buf));
    m.insert("buf-len".into(), fn_ty(vec![Type::Buf], Type::I64));
    m.insert(
        "buf-ref".into(),
        fn_ty(vec![Type::Buf, Type::I64], Type::I64),
    );
    m.insert(
        "buf-set".into(),
        fn_ty(vec![Type::Buf, Type::I64, Type::I64], Type::Nil),
    );
    m.insert("buf-clone".into(), fn_ty(vec![Type::Buf], Type::Buf));
    m.insert(
        "buf-get-u32".into(),
        fn_ty(vec![Type::Buf, Type::I64], Type::I64),
    );
    m.insert(
        "buf-set-u32".into(),
        fn_ty(vec![Type::Buf, Type::I64, Type::I64], Type::Nil),
    );
    m.insert("str-len".into(), fn_ty(vec![Type::Str], Type::I64));
    m.insert(
        "str-ref".into(),
        fn_ty(vec![Type::Str, Type::I64], Type::I64),
    );
    m.insert(
        "str-append".into(),
        fn_ty(vec![Type::Str, Type::Str], Type::Str),
    );
    m.insert(
        "str-slice".into(),
        fn_ty(vec![Type::Str, Type::I64, Type::I64], Type::Str),
    );
    m.insert("str-from-byte".into(), fn_ty(vec![Type::I64], Type::Str));
    m.insert("str-from-i64".into(), fn_ty(vec![Type::I64], Type::Str));
    m.insert("str-from-f64".into(), fn_ty(vec![Type::F64], Type::Str));
    m.insert("i64-from-u32".into(), fn_ty(vec![Type::U32], Type::I64));
    m.insert("u32-from-i64".into(), fn_ty(vec![Type::I64], Type::U32));
    m.insert("i64-from-i32".into(), fn_ty(vec![Type::I32], Type::I64));
    m.insert("i32-from-i64".into(), fn_ty(vec![Type::I64], Type::I32));
    install_sys(&mut m);
    install_result_helpers(&mut m);
    m
}
