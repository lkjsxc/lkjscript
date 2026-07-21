//! Typed prelude for builtins.

use super::prelude_sys::{install_result_helpers, install_sys};
use super::ty::Type;
use std::collections::HashMap;

pub fn prelude() -> HashMap<String, Type> {
    let mut m = HashMap::new();
    let bin_num = Type::Fn { params: vec![Type::Any, Type::Any], ret: Box::new(Type::Any) };
    let bin_bool = Type::Fn { params: vec![Type::Any, Type::Any], ret: Box::new(Type::Bool) };
    for n in ["+", "-", "*", "/", "div", "bit-and", "bit-or", "bit-xor"] {
        m.insert(n.into(), bin_num.clone());
    }
    for n in ["=", "!=", "<", "<=", ">", ">=", "lt", "le", "gt", "ge", "eq", "lte", "gte"] {
        m.insert(n.into(), bin_bool.clone());
    }
    m.insert("not".into(), Type::Fn { params: vec![Type::Bool], ret: Box::new(Type::Bool) });
    m.insert("and".into(), Type::Fn { params: vec![Type::Bool, Type::Bool], ret: Box::new(Type::Bool) });
    m.insert("or".into(), Type::Fn { params: vec![Type::Bool, Type::Bool], ret: Box::new(Type::Bool) });
    m.insert("cons".into(), Type::Fn { params: vec![Type::Any, Type::List], ret: Box::new(Type::List) });
    m.insert("car".into(), Type::Fn { params: vec![Type::List], ret: Box::new(Type::Any) });
    m.insert("cdr".into(), Type::Fn { params: vec![Type::List], ret: Box::new(Type::List) });
    m.insert("null?".into(), Type::Fn { params: vec![Type::Any], ret: Box::new(Type::Bool) });
    m.insert("print".into(), Type::Fn { params: vec![Type::Any], ret: Box::new(Type::Nil) });
    m.insert("flush".into(), Type::Fn { params: vec![], ret: Box::new(Type::Nil) });
    m.insert("write-str".into(), Type::Fn { params: vec![Type::Str], ret: Box::new(Type::Nil) });
    m.insert("empty-str".into(), Type::Fn { params: vec![], ret: Box::new(Type::Str) });
    m.insert("argc".into(), Type::Fn { params: vec![], ret: Box::new(Type::Int) });
    m.insert("read-byte".into(), Type::Fn { params: vec![], ret: Box::new(Type::Int) });
    m.insert("stdin-fd".into(), Type::Fn { params: vec![], ret: Box::new(Type::Handle) });
    m.insert("isatty".into(), Type::Fn { params: vec![Type::Handle], ret: Box::new(Type::Bool) });
    m.insert("close".into(), Type::Fn { params: vec![Type::Handle], ret: Box::new(Type::Nil) });
    m.insert("read-byte-fd".into(), Type::Fn { params: vec![Type::Handle], ret: Box::new(Type::Int) });
    m.insert("write-byte-fd".into(), Type::Fn { params: vec![Type::Handle, Type::Int], ret: Box::new(Type::Nil) });
    m.insert("write-byte".into(), Type::Fn { params: vec![Type::Int], ret: Box::new(Type::Nil) });
    m.insert("arg".into(), Type::Fn { params: vec![Type::Int], ret: Box::new(Type::Str) });
    m.insert("exit".into(), Type::Fn { params: vec![Type::Int], ret: Box::new(Type::Nil) });
    m.insert("buf-new".into(), Type::Fn { params: vec![Type::Int], ret: Box::new(Type::Buf) });
    m.insert("buf-len".into(), Type::Fn { params: vec![Type::Buf], ret: Box::new(Type::Int) });
    m.insert("buf-ref".into(), Type::Fn { params: vec![Type::Buf, Type::Int], ret: Box::new(Type::Int) });
    m.insert("buf-set".into(), Type::Fn { params: vec![Type::Buf, Type::Int, Type::Int], ret: Box::new(Type::Nil) });
    m.insert("buf-clone".into(), Type::Fn { params: vec![Type::Buf], ret: Box::new(Type::Buf) });
    m.insert("buf-get-u32".into(), Type::Fn { params: vec![Type::Buf, Type::Int], ret: Box::new(Type::Int) });
    m.insert("buf-set-u32".into(), Type::Fn { params: vec![Type::Buf, Type::Int, Type::Int], ret: Box::new(Type::Nil) });
    m.insert("str-len".into(), Type::Fn { params: vec![Type::Str], ret: Box::new(Type::Int) });
    m.insert("str-ref".into(), Type::Fn { params: vec![Type::Str, Type::Int], ret: Box::new(Type::Int) });
    m.insert("str-append".into(), Type::Fn { params: vec![Type::Str, Type::Str], ret: Box::new(Type::Str) });
    m.insert("str-slice".into(), Type::Fn { params: vec![Type::Str, Type::Int, Type::Int], ret: Box::new(Type::Str) });
    m.insert("str-from-byte".into(), Type::Fn { params: vec![Type::Int], ret: Box::new(Type::Str) });
    m.insert("sys-ioctl".into(), Type::Fn { params: vec![Type::Handle, Type::Int, Type::Buf], ret: Box::new(Type::Int) });
    m.insert("sys-poll".into(), Type::Fn { params: vec![Type::Handle, Type::Int], ret: Box::new(Type::Int) });
    m.insert("tty-guard-save".into(), Type::Fn { params: vec![Type::Buf], ret: Box::new(Type::Nil) });
    m.insert("tty-guard-clear".into(), Type::Fn { params: vec![], ret: Box::new(Type::Nil) });
    install_sys(&mut m);
    install_result_helpers(&mut m);
    m
}
