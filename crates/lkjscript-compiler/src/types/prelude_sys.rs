//! Typed system and Result prelude entries.

use std::collections::HashMap;

use super::ty::Type;

fn fn_ty(params: Vec<Type>, ret: Type) -> Type {
    Type::Fn {
        params,
        ret: Box::new(ret),
    }
}

fn forall(vars: &[&str], body: Type) -> Type {
    Type::Forall {
        vars: vars.iter().map(|name| (*name).to_string()).collect(),
        body: Box::new(body),
    }
}

fn system_result(success: Type) -> Type {
    Type::Result(Box::new(success), Box::new(Type::Str))
}

pub fn install_sys(prelude: &mut HashMap<String, Type>) {
    prelude.insert("stdin-handle".into(), fn_ty(vec![], Type::Handle));
    prelude.insert(
        "sys-isatty".into(),
        fn_ty(vec![Type::Handle], system_result(Type::Bool)),
    );
    prelude.insert(
        "sys-close".into(),
        fn_ty(vec![Type::Handle], system_result(Type::Nil)),
    );
    prelude.insert(
        "sys-read-byte".into(),
        fn_ty(vec![Type::Handle], system_result(Type::I64)),
    );
    prelude.insert(
        "sys-write-byte".into(),
        fn_ty(vec![Type::Handle, Type::I64], system_result(Type::Nil)),
    );
    prelude.insert(
        "sys-tty-guard-save".into(),
        fn_ty(vec![Type::Buf], system_result(Type::Nil)),
    );
    prelude.insert(
        "sys-tty-guard-clear".into(),
        fn_ty(vec![], system_result(Type::Nil)),
    );
    prelude.insert(
        "sys-open-read".into(),
        fn_ty(vec![Type::Str], system_result(Type::Handle)),
    );
    prelude.insert(
        "sys-open-write".into(),
        fn_ty(vec![Type::Str], system_result(Type::Handle)),
    );
    prelude.insert(
        "sys-path-exists".into(),
        fn_ty(vec![Type::Str], system_result(Type::Bool)),
    );
    prelude.insert(
        "sys-wait-ms".into(),
        fn_ty(vec![Type::I64], system_result(Type::Nil)),
    );
    prelude.insert("sys-now-ms".into(), fn_ty(vec![], system_result(Type::I64)));
    prelude.insert(
        "sys-socket".into(),
        fn_ty(vec![], system_result(Type::Handle)),
    );
    prelude.insert(
        "sys-bind".into(),
        fn_ty(vec![Type::Handle, Type::I64], system_result(Type::Nil)),
    );
    prelude.insert(
        "sys-listen".into(),
        fn_ty(vec![Type::Handle, Type::I64], system_result(Type::Nil)),
    );
    prelude.insert(
        "sys-accept".into(),
        fn_ty(vec![Type::Handle], system_result(Type::Handle)),
    );
    prelude.insert(
        "sys-recv".into(),
        fn_ty(vec![Type::Handle], system_result(Type::Str)),
    );
    prelude.insert(
        "sys-send".into(),
        fn_ty(vec![Type::Handle, Type::Str], system_result(Type::I64)),
    );
    prelude.insert(
        "sys-poll".into(),
        fn_ty(vec![Type::Handle, Type::I64], system_result(Type::I64)),
    );
    prelude.insert(
        "sys-tty-get".into(),
        fn_ty(vec![Type::Handle, Type::Buf], system_result(Type::Nil)),
    );
    prelude.insert(
        "sys-tty-set".into(),
        fn_ty(vec![Type::Handle, Type::Buf], system_result(Type::Nil)),
    );
}

pub fn install_result_helpers(prelude: &mut HashMap<String, Type>) {
    let success = Type::Param("T".into());
    let failure = Type::Param("E".into());
    let result = Type::Result(Box::new(success.clone()), Box::new(failure.clone()));
    prelude.insert(
        "ok".into(),
        forall(&["T", "E"], fn_ty(vec![success.clone()], result.clone())),
    );
    prelude.insert(
        "err".into(),
        forall(&["T", "E"], fn_ty(vec![failure.clone()], result.clone())),
    );
    prelude.insert(
        "is-ok".into(),
        forall(&["T", "E"], fn_ty(vec![result.clone()], Type::Bool)),
    );
    prelude.insert(
        "unwrap-ok".into(),
        forall(&["T", "E"], fn_ty(vec![result.clone()], success)),
    );
    prelude.insert(
        "unwrap-err".into(),
        forall(&["T", "E"], fn_ty(vec![result], failure)),
    );
}
