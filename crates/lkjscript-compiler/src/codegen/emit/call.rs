//! Plain and builtin call emission.

use crate::ast::Expr;
use crate::codegen::emit::{compile_expr, compile_name, Cx};
use lkjscript_core::{Op, Result};

pub fn compile_plain_call(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<()> {
    if try_binop(cx, name, args)? {
        return Ok(());
    }
    if try_unary(cx, name, args)? {
        return Ok(());
    }
    if try_ternary(cx, name, args)? {
        return Ok(());
    }
    if try_host(cx, name, args)? {
        return Ok(());
    }
    if name == "+" && args.len() > 2 {
        compile_expr(cx, &args[0])?;
        for a in &args[1..] {
            compile_expr(cx, a)?;
            cx.proto.emit(Op::Add);
        }
        return Ok(());
    }
    if name == "*" && args.len() > 2 {
        compile_expr(cx, &args[0])?;
        for a in &args[1..] {
            compile_expr(cx, a)?;
            cx.proto.emit(Op::Mul);
        }
        return Ok(());
    }
    for a in args {
        compile_expr(cx, a)?;
    }
    compile_name(cx, name)?;
    cx.proto.emit_op_u8(Op::Call, args.len() as u8);
    Ok(())
}

fn try_binop(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<bool> {
    if args.len() != 2 {
        return Ok(false);
    }
    let op = match name {
        "+" => Op::Add,
        "-" => Op::Sub,
        "*" => Op::Mul,
        "/" | "div" => Op::Div,
        "=" => Op::Eq,
        "!=" => Op::Ne,
        "<" => Op::Lt,
        "<=" => Op::Le,
        ">" => Op::Gt,
        ">=" => Op::Ge,
        "cons" => Op::Cons,
        "str-ref" => Op::StrRef,
        "str-append" => Op::StrAppend,
        "write-byte-fd" => Op::WriteByteFd,
        "sys-send" => Op::SysSend,
        "buf-ref" => Op::BufRef,
        "buf-get-u32" => Op::BufGetU32,
        "sys-poll" => Op::SysPoll,
        "sys-tty-get" => Op::SysTtyGet,
        "sys-tty-set" => Op::SysTtySet,
        "sys-bind" => Op::SysBind,
        "sys-listen" => Op::SysListen,
        "bit-and" => Op::BitAnd,
        "bit-or" => Op::BitOr,
        "bit-xor" => Op::BitXor,
        _ => return Ok(false),
    };
    compile_expr(cx, &args[0])?;
    compile_expr(cx, &args[1])?;
    cx.proto.emit(op);
    Ok(true)
}

fn try_unary(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<bool> {
    if args.len() != 1 {
        return Ok(false);
    }
    let op = match name {
        "not" => Op::Not,
        "nil?" => Op::IsNil,
        "car" => Op::Car,
        "cdr" => Op::Cdr,
        "null?" => Op::IsNull,
        "print" => Op::Print,
        "write-byte" => Op::WriteByte,
        "write-str" => Op::WriteStr,
        "exit" => Op::Exit,
        "str-len" => Op::StrLen,
        "str-from-byte" => Op::StrFromByte,
        "str-from-i64" => Op::StrFromI64,
        "str-from-f64" => Op::StrFromF64,
        "sys-open-read" => Op::SysOpenRead,
        "sys-open-write" => Op::SysOpenWrite,
        "close" => Op::CloseFd,
        "read-byte-fd" => Op::ReadByteFd,
        "arg" => Op::Arg,
        "sys-wait-ms" => Op::SysWaitMs,
        "sys-accept" => Op::SysAccept,
        "sys-recv" => Op::SysRecv,
        "sys-path-exists" => Op::SysPathExists,
        "buf-new" => Op::BufNew,
        "buf-len" => Op::BufLen,
        "buf-clone" => Op::BufClone,
        "isatty" => Op::Isatty,
        "tty-guard-save" => Op::TtyGuardSave,
        "ok" => Op::OkWrap,
        "err" => Op::ErrWrap,
        "is-ok" => Op::IsOk,
        "unwrap-ok" => Op::UnwrapOk,
        "unwrap-err" => Op::UnwrapErr,
        _ => return Ok(false),
    };
    compile_expr(cx, &args[0])?;
    cx.proto.emit(op);
    Ok(true)
}

fn try_ternary(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<bool> {
    if args.len() != 3 {
        return Ok(false);
    }
    let op = match name {
        "str-slice" => Op::StrSlice,
        "buf-set" => Op::BufSet,
        "buf-set-u32" => Op::BufSetU32,
        _ => return Ok(false),
    };
    compile_expr(cx, &args[0])?;
    compile_expr(cx, &args[1])?;
    compile_expr(cx, &args[2])?;
    cx.proto.emit(op);
    Ok(true)
}

fn try_host(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<bool> {
    match (name, args.len()) {
        ("flush", 0) => {
            cx.proto.emit(Op::Flush);
            Ok(true)
        }
        ("read-byte", 0) => {
            cx.proto.emit(Op::ReadByte);
            Ok(true)
        }
        ("argc", 0) => {
            cx.proto.emit(Op::Argc);
            Ok(true)
        }
        ("empty-str", 0) => {
            cx.proto.emit(Op::EmptyStr);
            Ok(true)
        }
        ("stdin-fd", 0) => {
            cx.proto.emit(Op::StdinFd);
            Ok(true)
        }
        ("tty-guard-clear", 0) => {
            cx.proto.emit(Op::TtyGuardClear);
            Ok(true)
        }
        ("sys-now-ms", 0) => {
            cx.proto.emit(Op::SysNowMs);
            Ok(true)
        }
        ("sys-socket", 0) => {
            cx.proto.emit(Op::SysSocket);
            Ok(true)
        }
        _ => Ok(false),
    }
}
