//! Sys and Result prelude entries (parametric, no Any).

use super::ty::Type;
use std::collections::HashMap;

fn fn_ty(params: Vec<Type>, ret: Type) -> Type {
    Type::Fn {
        params,
        ret: Box::new(ret),
    }
}

fn forall(vars: &[&str], body: Type) -> Type {
    Type::Forall {
        vars: vars.iter().map(|s| (*s).to_string()).collect(),
        body: Box::new(body),
    }
}

pub fn install_sys(m: &mut HashMap<String, Type>) {
    let res_h = Type::Result(Box::new(Type::Handle), Box::new(Type::Str));
    let res_nil = Type::Result(Box::new(Type::Nil), Box::new(Type::Str));
    let res_i64 = Type::Result(Box::new(Type::I64), Box::new(Type::Str));
    let res_str = Type::Result(Box::new(Type::Str), Box::new(Type::Str));
    m.insert("sys-open-read".into(), fn_ty(vec![Type::Str], res_h.clone()));
    m.insert("sys-open-write".into(), fn_ty(vec![Type::Str], res_h.clone()));
    m.insert("sys-path-exists".into(), fn_ty(vec![Type::Str], Type::Bool));
    m.insert("sys-wait-ms".into(), fn_ty(vec![Type::I64], res_nil.clone()));
    m.insert("sys-now-ms".into(), fn_ty(vec![], res_i64.clone()));
    m.insert("sys-socket".into(), fn_ty(vec![], res_h.clone()));
    m.insert(
        "sys-bind".into(),
        fn_ty(vec![Type::Handle, Type::I64], res_nil.clone()),
    );
    m.insert(
        "sys-listen".into(),
        fn_ty(vec![Type::Handle, Type::I64], res_nil.clone()),
    );
    m.insert("sys-accept".into(), fn_ty(vec![Type::Handle], res_h));
    m.insert("sys-recv".into(), fn_ty(vec![Type::Handle], res_str));
    m.insert(
        "sys-send".into(),
        fn_ty(vec![Type::Handle, Type::Str], res_i64.clone()),
    );
    m.insert(
        "sys-poll".into(),
        fn_ty(vec![Type::Handle, Type::I64], res_i64),
    );
    m.insert(
        "sys-tty-get".into(),
        fn_ty(vec![Type::Handle, Type::Buf], res_nil.clone()),
    );
    m.insert(
        "sys-tty-set".into(),
        fn_ty(vec![Type::Handle, Type::Buf], res_nil),
    );
}

pub fn install_result_helpers(m: &mut HashMap<String, Type>) {
    let t = Type::Param("T".into());
    let e = Type::Param("E".into());
    let r = Type::Result(Box::new(t.clone()), Box::new(e.clone()));
    m.insert(
        "ok".into(),
        forall(&["T", "E"], fn_ty(vec![t.clone()], r.clone())),
    );
    m.insert(
        "err".into(),
        forall(&["T", "E"], fn_ty(vec![e.clone()], r.clone())),
    );
    m.insert("is-ok".into(), forall(&["T", "E"], fn_ty(vec![r.clone()], Type::Bool)));
    m.insert("unwrap-ok".into(), forall(&["T", "E"], fn_ty(vec![r.clone()], t)));
    m.insert("unwrap-err".into(), forall(&["T", "E"], fn_ty(vec![r], e)));
}
