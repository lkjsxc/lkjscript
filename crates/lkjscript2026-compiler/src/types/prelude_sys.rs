//! Sys and Result prelude entries.

use super::ty::Type;
use std::collections::HashMap;

pub fn install_sys(m: &mut HashMap<String, Type>) {
    let res_handle = Type::Result(Box::new(Type::Handle), Box::new(Type::Str));
    let res_nil = Type::Result(Box::new(Type::Nil), Box::new(Type::Str));
    let res_int = Type::Result(Box::new(Type::Int), Box::new(Type::Str));
    let res_str = Type::Result(Box::new(Type::Str), Box::new(Type::Str));
    m.insert("sys-open-read".into(), Type::Fn { params: vec![Type::Str], ret: Box::new(res_handle.clone()) });
    m.insert("sys-open-write".into(), Type::Fn { params: vec![Type::Str], ret: Box::new(res_handle.clone()) });
    m.insert("sys-path-exists".into(), Type::Fn { params: vec![Type::Str], ret: Box::new(Type::Bool) });
    m.insert("sys-wait-ms".into(), Type::Fn { params: vec![Type::Int], ret: Box::new(res_nil.clone()) });
    m.insert("sys-now-ms".into(), Type::Fn { params: vec![], ret: Box::new(res_int.clone()) });
    m.insert("sys-socket".into(), Type::Fn { params: vec![], ret: Box::new(res_handle.clone()) });
    m.insert("sys-bind".into(), Type::Fn { params: vec![Type::Handle, Type::Int], ret: Box::new(res_nil.clone()) });
    m.insert("sys-listen".into(), Type::Fn { params: vec![Type::Handle, Type::Int], ret: Box::new(res_nil) });
    m.insert("sys-accept".into(), Type::Fn { params: vec![Type::Handle], ret: Box::new(res_handle) });
    m.insert("sys-recv".into(), Type::Fn { params: vec![Type::Handle], ret: Box::new(res_str) });
    m.insert("sys-send".into(), Type::Fn { params: vec![Type::Handle, Type::Str], ret: Box::new(res_int) });
}

pub fn install_result_helpers(m: &mut HashMap<String, Type>) {
    let r = Type::Result(Box::new(Type::Any), Box::new(Type::Any));
    m.insert("ok".into(), Type::Fn { params: vec![Type::Any], ret: Box::new(r.clone()) });
    m.insert("err".into(), Type::Fn { params: vec![Type::Any], ret: Box::new(r.clone()) });
    m.insert("is-ok".into(), Type::Fn { params: vec![r.clone()], ret: Box::new(Type::Bool) });
    m.insert("unwrap-ok".into(), Type::Fn { params: vec![r.clone()], ret: Box::new(Type::Any) });
    m.insert("unwrap-err".into(), Type::Fn { params: vec![r], ret: Box::new(Type::Any) });
}
