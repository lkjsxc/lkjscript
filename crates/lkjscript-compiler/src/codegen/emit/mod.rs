//! Emit bytecode from resolved typed HIR expressions.

use std::collections::HashMap;

use lkjscript_core::{Chunk, Constant, Error, FunctionProto, Op, Result};

use crate::hir::{BindingId, Expr, ExprKind, LocalDefinition, Operation};

pub(crate) struct Cx<'a> {
    chunk: &'a mut Chunk,
    globals: &'a HashMap<BindingId, u16>,
    locals: HashMap<BindingId, u8>,
    code_base: u16,
    pub(crate) proto: FunctionProto,
}

impl<'a> Cx<'a> {
    pub(crate) fn new(
        chunk: &'a mut Chunk,
        globals: &'a HashMap<BindingId, u16>,
        locals: HashMap<BindingId, u8>,
        code_base: u16,
        proto: FunctionProto,
    ) -> Self {
        Self {
            chunk,
            globals,
            locals,
            code_base,
            proto,
        }
    }
}

pub(crate) fn emit_expr(cx: &mut Cx<'_>, expression: &Expr) -> Result<()> {
    match &expression.kind {
        ExprKind::LitUnit => cx.proto.emit(Op::Unit),
        ExprKind::EmptyList => cx.proto.emit(Op::EmptyList),
        ExprKind::LitNone => cx.proto.emit(Op::OptionNone),
        ExprKind::LitBool(true) => cx.proto.emit(Op::True),
        ExprKind::LitBool(false) => cx.proto.emit(Op::False),
        ExprKind::LitI64(value) => {
            let constant = add_constant(cx.chunk, Constant::I64(*value))?;
            cx.proto.emit_op_u16(Op::LoadConst, constant);
        }
        ExprKind::LitF64(value) => {
            let constant = add_constant(cx.chunk, Constant::F64(*value))?;
            cx.proto.emit_op_u16(Op::LoadConst, constant);
        }
        ExprKind::LitStr(value) => {
            let constant = add_constant(cx.chunk, Constant::Str(value.clone()))?;
            cx.proto.emit_op_u16(Op::LoadConst, constant);
        }
        ExprKind::Load(binding) => emit_load(cx, *binding)?,
        ExprKind::Call { callee, args } => {
            emit_arguments(cx, args)?;
            emit_load(cx, *callee)?;
            let arity = u8::try_from(args.len())
                .map_err(|_| Error::msg("HIR call arity does not fit bytecode u8"))?;
            cx.proto.emit_op_u8(Op::Call, arity);
        }
        ExprKind::Operation {
            operation, args, ..
        } => emit_operation(cx, *operation, args)?,
        ExprKind::Do(expressions) => emit_sequence(cx, expressions, true)?,
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => emit_if(cx, condition, then_branch, else_branch)?,
        ExprKind::While { condition, body } => emit_while(cx, condition, body)?,
        ExprKind::Let { bindings, body } => emit_let(cx, bindings, body)?,
        ExprKind::SetGlobal { target, value } => {
            emit_expr(cx, value)?;
            let slot = global_slot(cx, *target)?;
            cx.proto.emit_op_u16(Op::StoreGlobal, slot);
            cx.proto.emit(Op::Pop);
            cx.proto.emit(Op::Unit);
        }
        ExprKind::QuoteSymbol(symbol) => {
            let constant = add_constant(cx.chunk, Constant::Symbol(symbol.clone()))?;
            cx.proto.emit_op_u16(Op::LoadConst, constant);
        }
    }
    Ok(())
}

pub(crate) fn emit_sequence(
    cx: &mut Cx<'_>,
    expressions: &[Expr],
    empty_yields_unit: bool,
) -> Result<()> {
    if expressions.is_empty() {
        if empty_yields_unit {
            cx.proto.emit(Op::Unit);
        }
        return Ok(());
    }
    for (index, expression) in expressions.iter().enumerate() {
        emit_expr(cx, expression)?;
        if index + 1 != expressions.len() {
            cx.proto.emit(Op::Pop);
        }
    }
    Ok(())
}

pub(crate) fn add_constant(chunk: &mut Chunk, constant: Constant) -> Result<u16> {
    let id = u16::try_from(chunk.constants.len())
        .map_err(|_| Error::msg("too many constants for bytecode u16 IDs"))?;
    chunk.constants.push(constant);
    Ok(id)
}

fn emit_load(cx: &mut Cx<'_>, binding: BindingId) -> Result<()> {
    if let Some(slot) = cx.locals.get(&binding).copied() {
        cx.proto.emit_op_u8(Op::LoadLocal, slot);
        return Ok(());
    }
    let slot = global_slot(cx, binding)?;
    cx.proto.emit_op_u16(Op::LoadGlobal, slot);
    Ok(())
}

fn global_slot(cx: &Cx<'_>, binding: BindingId) -> Result<u16> {
    cx.globals.get(&binding).copied().ok_or_else(|| {
        Error::msg(format!(
            "resolved HIR binding {} has no bytecode global slot",
            binding.raw()
        ))
    })
}

fn emit_arguments(cx: &mut Cx<'_>, args: &[Expr]) -> Result<()> {
    for argument in args {
        emit_expr(cx, argument)?;
    }
    Ok(())
}

fn emit_operation(cx: &mut Cx<'_>, operation: Operation, args: &[Expr]) -> Result<()> {
    match operation {
        Operation::And => emit_logic(cx, args, false),
        Operation::Or => emit_logic(cx, args, true),
        _ => {
            emit_arguments(cx, args)?;
            let opcode = operation_opcode(operation).ok_or_else(|| {
                Error::msg(format!(
                    "canonical operation {operation:?} has no bytecode lowering"
                ))
            })?;
            cx.proto.emit(opcode);
            Ok(())
        }
    }
}

fn emit_logic(cx: &mut Cx<'_>, args: &[Expr], is_or: bool) -> Result<()> {
    let [left, right] = args else {
        return Err(Error::msg(
            "resolved short-circuit operation must have two arguments",
        ));
    };
    emit_expr(cx, left)?;
    cx.proto.emit(Op::JumpIfFalse);
    let alternate_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    if is_or {
        cx.proto.emit(Op::True);
    } else {
        emit_expr(cx, right)?;
    }
    cx.proto.emit(Op::Jump);
    let end_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    let alternate = code_offset(cx)?;
    patch_jump(&mut cx.proto, alternate_jump, alternate)?;
    if is_or {
        emit_expr(cx, right)?;
    } else {
        cx.proto.emit(Op::False);
    }
    let end = code_offset(cx)?;
    patch_jump(&mut cx.proto, end_jump, end)
}

fn emit_if(
    cx: &mut Cx<'_>,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
) -> Result<()> {
    emit_expr(cx, condition)?;
    cx.proto.emit(Op::JumpIfFalse);
    let else_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    emit_expr(cx, then_branch)?;
    cx.proto.emit(Op::Jump);
    let end_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    let else_offset = code_offset(cx)?;
    patch_jump(&mut cx.proto, else_jump, else_offset)?;
    emit_expr(cx, else_branch)?;
    let end_offset = code_offset(cx)?;
    patch_jump(&mut cx.proto, end_jump, end_offset)
}

fn emit_while(cx: &mut Cx<'_>, condition: &Expr, body: &[Expr]) -> Result<()> {
    let loop_start = code_offset(cx)?;
    emit_expr(cx, condition)?;
    cx.proto.emit(Op::JumpIfFalse);
    let exit_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    emit_sequence(cx, body, true)?;
    cx.proto.emit(Op::Pop);
    cx.proto.emit(Op::Jump);
    let back_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    patch_jump(&mut cx.proto, back_jump, loop_start)?;
    let end = code_offset(cx)?;
    patch_jump(&mut cx.proto, exit_jump, end)?;
    cx.proto.emit(Op::Unit);
    Ok(())
}

fn emit_let(cx: &mut Cx<'_>, bindings: &[LocalDefinition], body: &Expr) -> Result<()> {
    for binding in bindings {
        emit_expr(cx, &binding.value)?;
        if cx.locals.insert(binding.binding, binding.slot).is_some() {
            return Err(Error::msg(format!(
                "duplicate HIR local binding {} during bytecode lowering",
                binding.binding.raw()
            )));
        }
        cx.proto.emit_op_u8(Op::StoreLocal, binding.slot);
        cx.proto.emit(Op::Pop);
    }
    emit_expr(cx, body)?;
    for binding in bindings {
        cx.locals.remove(&binding.binding);
    }
    Ok(())
}

fn code_offset(cx: &Cx<'_>) -> Result<u16> {
    let local = u16::try_from(cx.proto.len())
        .map_err(|_| Error::msg("bytecode jump offset exceeds u16"))?;
    cx.code_base
        .checked_add(local)
        .ok_or_else(|| Error::msg("bytecode jump offset exceeds u16"))
}

fn patch_jump(proto: &mut FunctionProto, at: usize, target: u16) -> Result<()> {
    let Some(end) = at.checked_add(2) else {
        return Err(Error::msg("bytecode jump patch offset overflow"));
    };
    let Some(bytes) = proto.code.get_mut(at..end) else {
        return Err(Error::msg("bytecode jump patch is outside function code"));
    };
    bytes.copy_from_slice(&target.to_le_bytes());
    Ok(())
}

fn operation_opcode(operation: Operation) -> Option<Op> {
    Some(match operation {
        Operation::Add => Op::Add,
        Operation::Subtract => Op::Sub,
        Operation::Multiply => Op::Mul,
        Operation::Divide => Op::Div,
        Operation::EqualValue => Op::EqualValue,
        Operation::SameObject => Op::SameObject,
        Operation::ListEqual => Op::ListEqual,
        Operation::F64BitsEqual => Op::F64BitsEqual,
        Operation::Less => Op::Lt,
        Operation::LessEqual => Op::Le,
        Operation::Greater => Op::Gt,
        Operation::GreaterEqual => Op::Ge,
        Operation::Not => Op::Not,
        Operation::Cons => Op::Cons,
        Operation::Car => Op::Car,
        Operation::Cdr => Op::Cdr,
        Operation::IsEmptyList => Op::IsEmptyList,
        Operation::Print => Op::Print,
        Operation::Flush => Op::Flush,
        Operation::ReadByte => Op::ReadByte,
        Operation::WriteByte => Op::WriteByte,
        Operation::Exit => Op::Exit,
        Operation::BitAnd => Op::BitAnd,
        Operation::BitOr => Op::BitOr,
        Operation::BitXor => Op::BitXor,
        Operation::WriteStr => Op::WriteStr,
        Operation::EmptyStr => Op::EmptyStr,
        Operation::ArgCount => Op::Argc,
        Operation::Arg => Op::Arg,
        Operation::BufNew => Op::BufNew,
        Operation::BufLen => Op::BufLen,
        Operation::BufRef => Op::BufRef,
        Operation::BufSet => Op::BufSet,
        Operation::BufClone => Op::BufClone,
        Operation::BufGetU32 => Op::BufGetU32,
        Operation::BufSetU32 => Op::BufSetU32,
        Operation::StrLen => Op::StrLen,
        Operation::StrRef => Op::StrRef,
        Operation::StrAppend => Op::StrAppend,
        Operation::StrSlice => Op::StrSlice,
        Operation::StrFromByte => Op::StrFromByte,
        Operation::StrFromI64 => Op::StrFromI64,
        Operation::StrFromF64 => Op::StrFromF64,
        Operation::StdinHandle => Op::StdinHandle,
        Operation::SysIsatty => Op::SysIsatty,
        Operation::SysClose => Op::SysClose,
        Operation::SysReadByte => Op::SysReadByte,
        Operation::SysWriteByte => Op::SysWriteByte,
        Operation::SysTtyGuardSave => Op::SysTtyGuardSave,
        Operation::SysTtyGuardClear => Op::SysTtyGuardClear,
        Operation::SysOpenRead => Op::SysOpenRead,
        Operation::SysOpenWrite => Op::SysOpenWrite,
        Operation::SysPathExists => Op::SysPathExists,
        Operation::SysWaitMs => Op::SysWaitMs,
        Operation::SysNowMs => Op::SysNowMs,
        Operation::SysSocket => Op::SysSocket,
        Operation::SysBind => Op::SysBind,
        Operation::SysListen => Op::SysListen,
        Operation::SysAccept => Op::SysAccept,
        Operation::SysRecv => Op::SysRecv,
        Operation::SysSend => Op::SysSend,
        Operation::SysPoll => Op::SysPoll,
        Operation::SysTtyGet => Op::SysTtyGet,
        Operation::SysTtySet => Op::SysTtySet,
        Operation::Ok => Op::OkWrap,
        Operation::Err => Op::ErrWrap,
        Operation::IsOk => Op::IsOk,
        Operation::UnwrapOk => Op::UnwrapOk,
        Operation::UnwrapErr => Op::UnwrapErr,
        Operation::Some => Op::SomeWrap,
        Operation::IsSome => Op::IsSome,
        Operation::UnwrapSome => Op::UnwrapSome,
        Operation::And | Operation::Or => return None,
    })
}
