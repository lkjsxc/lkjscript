//! Plain and builtin call emission.

use crate::ast::Expr;
use crate::codegen::emit::{compile_expr, compile_name, Cx};
use lkjscript2026_core::{Op, Result};

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
        "/" => Op::Div,
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
        "tcp-send" => Op::TcpSend,
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
        "car" => Op::Car,
        "cdr" => Op::Cdr,
        "null?" => Op::IsNull,
        "print" => Op::Print,
        "write-byte" => Op::WriteByte,
        "write-str" => Op::WriteStr,
        "exit" => Op::Exit,
        "str-len" => Op::StrLen,
        "str-from-byte" => Op::StrFromByte,
        "open-read" => Op::OpenRead,
        "open-write" => Op::OpenWrite,
        "close" => Op::CloseFd,
        "read-byte-fd" => Op::ReadByteFd,
        "arg" => Op::Arg,
        "wait-ms" => Op::WaitMs,
        "tcp-listen" => Op::TcpListen,
        "tcp-accept" => Op::TcpAccept,
        "tcp-recv" => Op::TcpRecv,
        "path-exists" => Op::PathExists,
        _ => return Ok(false),
    };
    compile_expr(cx, &args[0])?;
    cx.proto.emit(op);
    Ok(true)
}

fn try_ternary(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<bool> {
    if args.len() != 3 || name != "str-slice" {
        return Ok(false);
    }
    compile_expr(cx, &args[0])?;
    compile_expr(cx, &args[1])?;
    compile_expr(cx, &args[2])?;
    cx.proto.emit(Op::StrSlice);
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
        ("term-raw", 0) => {
            cx.proto.emit(Op::TermRaw);
            Ok(true)
        }
        ("term-cooked", 0) => {
            cx.proto.emit(Op::TermCooked);
            Ok(true)
        }
        ("poll-byte", 0) => {
            cx.proto.emit(Op::PollByte);
            Ok(true)
        }
        ("now-ms", 0) => {
            cx.proto.emit(Op::NowMs);
            Ok(true)
        }
        _ => Ok(false),
    }
}
