//! Canonical identities and type schemes for built-in operations.

use crate::types::Type;

/// A built-in operation after name resolution.
///
/// Backends consume this identity rather than comparing source spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Not,
    Cons,
    Car,
    Cdr,
    IsEmptyList,
    Print,
    Flush,
    ReadByte,
    WriteByte,
    Exit,
    BitAnd,
    BitOr,
    BitXor,
    IsNil,
    And,
    Or,
    WriteStr,
    EmptyStr,
    ArgCount,
    Arg,
    BufNew,
    BufLen,
    BufRef,
    BufSet,
    BufClone,
    BufGetU32,
    BufSetU32,
    StrLen,
    StrRef,
    StrAppend,
    StrSlice,
    StrFromByte,
    StrFromI64,
    StrFromF64,
    StdinHandle,
    SysIsatty,
    SysClose,
    SysReadByte,
    SysWriteByte,
    SysTtyGuardSave,
    SysTtyGuardClear,
    SysOpenRead,
    SysOpenWrite,
    SysPathExists,
    SysWaitMs,
    SysNowMs,
    SysSocket,
    SysBind,
    SysListen,
    SysAccept,
    SysRecv,
    SysSend,
    SysPoll,
    SysTtyGet,
    SysTtySet,
    Ok,
    Err,
    IsOk,
    UnwrapOk,
    UnwrapErr,
}

impl Operation {
    /// All current source-visible operations. The leading entries retain the
    /// historical VM global layout used by existing chunks and disassembly.
    pub const ALL: &'static [Self] = &[
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Divide,
        Self::Equal,
        Self::NotEqual,
        Self::Less,
        Self::LessEqual,
        Self::Greater,
        Self::GreaterEqual,
        Self::Not,
        Self::Cons,
        Self::Car,
        Self::Cdr,
        Self::IsEmptyList,
        Self::Print,
        Self::Flush,
        Self::ReadByte,
        Self::WriteByte,
        Self::Exit,
        Self::BitAnd,
        Self::BitOr,
        Self::BitXor,
        Self::IsNil,
        Self::And,
        Self::Or,
        Self::WriteStr,
        Self::EmptyStr,
        Self::ArgCount,
        Self::Arg,
        Self::BufNew,
        Self::BufLen,
        Self::BufRef,
        Self::BufSet,
        Self::BufClone,
        Self::BufGetU32,
        Self::BufSetU32,
        Self::StrLen,
        Self::StrRef,
        Self::StrAppend,
        Self::StrSlice,
        Self::StrFromByte,
        Self::StrFromI64,
        Self::StrFromF64,
        Self::StdinHandle,
        Self::SysIsatty,
        Self::SysClose,
        Self::SysReadByte,
        Self::SysWriteByte,
        Self::SysTtyGuardSave,
        Self::SysTtyGuardClear,
        Self::SysOpenRead,
        Self::SysOpenWrite,
        Self::SysPathExists,
        Self::SysWaitMs,
        Self::SysNowMs,
        Self::SysSocket,
        Self::SysBind,
        Self::SysListen,
        Self::SysAccept,
        Self::SysRecv,
        Self::SysSend,
        Self::SysPoll,
        Self::SysTtyGet,
        Self::SysTtySet,
        Self::Ok,
        Self::Err,
        Self::IsOk,
        Self::UnwrapOk,
        Self::UnwrapErr,
    ];

    pub const LEGACY_GLOBALS: &'static [Self] = &[
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Divide,
        Self::Equal,
        Self::NotEqual,
        Self::Less,
        Self::LessEqual,
        Self::Greater,
        Self::GreaterEqual,
        Self::Not,
        Self::Cons,
        Self::Car,
        Self::Cdr,
        Self::IsEmptyList,
        Self::Print,
        Self::Flush,
        Self::ReadByte,
        Self::WriteByte,
        Self::Exit,
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|operation| operation.name() == name)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "div",
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::Less => "lt",
            Self::LessEqual => "lte",
            Self::Greater => "gt",
            Self::GreaterEqual => "gte",
            Self::Not => "not",
            Self::Cons => "cons",
            Self::Car => "car",
            Self::Cdr => "cdr",
            Self::IsEmptyList => "empty-list?",
            Self::Print => "print",
            Self::Flush => "flush",
            Self::ReadByte => "read-byte",
            Self::WriteByte => "write-byte",
            Self::Exit => "exit",
            Self::BitAnd => "bit-and",
            Self::BitOr => "bit-or",
            Self::BitXor => "bit-xor",
            Self::IsNil => "nil?",
            Self::And => "and",
            Self::Or => "or",
            Self::WriteStr => "write-str",
            Self::EmptyStr => "empty-str",
            Self::ArgCount => "argc",
            Self::Arg => "arg",
            Self::BufNew => "buf-new",
            Self::BufLen => "buf-len",
            Self::BufRef => "buf-ref",
            Self::BufSet => "buf-set",
            Self::BufClone => "buf-clone",
            Self::BufGetU32 => "buf-get-u32",
            Self::BufSetU32 => "buf-set-u32",
            Self::StrLen => "str-len",
            Self::StrRef => "str-ref",
            Self::StrAppend => "str-append",
            Self::StrSlice => "str-slice",
            Self::StrFromByte => "str-from-byte",
            Self::StrFromI64 => "str-from-i64",
            Self::StrFromF64 => "str-from-f64",
            Self::StdinHandle => "stdin-handle",
            Self::SysIsatty => "sys-isatty",
            Self::SysClose => "sys-close",
            Self::SysReadByte => "sys-read-byte",
            Self::SysWriteByte => "sys-write-byte",
            Self::SysTtyGuardSave => "sys-tty-guard-save",
            Self::SysTtyGuardClear => "sys-tty-guard-clear",
            Self::SysOpenRead => "sys-open-read",
            Self::SysOpenWrite => "sys-open-write",
            Self::SysPathExists => "sys-path-exists",
            Self::SysWaitMs => "sys-wait-ms",
            Self::SysNowMs => "sys-now-ms",
            Self::SysSocket => "sys-socket",
            Self::SysBind => "sys-bind",
            Self::SysListen => "sys-listen",
            Self::SysAccept => "sys-accept",
            Self::SysRecv => "sys-recv",
            Self::SysSend => "sys-send",
            Self::SysPoll => "sys-poll",
            Self::SysTtyGet => "sys-tty-get",
            Self::SysTtySet => "sys-tty-set",
            Self::Ok => "ok",
            Self::Err => "err",
            Self::IsOk => "is-ok",
            Self::UnwrapOk => "unwrap-ok",
            Self::UnwrapErr => "unwrap-err",
        }
    }

    pub fn signature(self) -> Type {
        let i64_binary = || function(vec![Type::I64, Type::I64], Type::I64);
        let numeric_binary = || {
            forall(
                &["N"],
                function(
                    vec![Type::Param("N".into()), Type::Param("N".into())],
                    Type::Param("N".into()),
                ),
            )
        };
        let numeric_comparison = || {
            forall(
                &["N"],
                function(
                    vec![Type::Param("N".into()), Type::Param("N".into())],
                    Type::Bool,
                ),
            )
        };
        let system_result = |success| Type::Result(Box::new(success), Box::new(Type::Str));

        match self {
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => numeric_binary(),
            Self::Equal | Self::NotEqual => forall(
                &["T"],
                function(
                    vec![Type::Param("T".into()), Type::Param("T".into())],
                    Type::Bool,
                ),
            ),
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => {
                numeric_comparison()
            }
            Self::BitAnd | Self::BitOr | Self::BitXor => i64_binary(),
            Self::IsNil => forall(&["T"], function(vec![Type::Param("T".into())], Type::Bool)),
            Self::Not => function(vec![Type::Bool], Type::Bool),
            Self::And | Self::Or => function(vec![Type::Bool, Type::Bool], Type::Bool),
            Self::Cons => {
                let item = Type::Param("T".into());
                forall(
                    &["T"],
                    function(
                        vec![item.clone(), Type::List(Box::new(item.clone()))],
                        Type::List(Box::new(item)),
                    ),
                )
            }
            Self::Car => {
                let item = Type::Param("T".into());
                forall(
                    &["T"],
                    function(vec![Type::List(Box::new(item.clone()))], item),
                )
            }
            Self::Cdr => {
                let item = Type::Param("T".into());
                forall(
                    &["T"],
                    function(
                        vec![Type::List(Box::new(item.clone()))],
                        Type::List(Box::new(item)),
                    ),
                )
            }
            Self::IsEmptyList => forall(
                &["T"],
                function(
                    vec![Type::List(Box::new(Type::Param("T".into())))],
                    Type::Bool,
                ),
            ),
            Self::Print | Self::WriteStr => function(vec![Type::Str], Type::Unit),
            Self::Flush => function(Vec::new(), Type::Unit),
            Self::ReadByte => function(Vec::new(), Type::I64),
            Self::WriteByte => function(vec![Type::I64], Type::Unit),
            Self::Exit => function(vec![Type::I64], Type::Unit),
            Self::EmptyStr => function(Vec::new(), Type::Str),
            Self::ArgCount => function(Vec::new(), Type::I64),
            Self::Arg => function(vec![Type::I64], Type::Str),
            Self::BufNew => function(vec![Type::I64], Type::Buf),
            Self::BufLen => function(vec![Type::Buf], Type::I64),
            Self::BufRef | Self::BufGetU32 => function(vec![Type::Buf, Type::I64], Type::I64),
            Self::BufSet | Self::BufSetU32 => {
                function(vec![Type::Buf, Type::I64, Type::I64], Type::Unit)
            }
            Self::BufClone => function(vec![Type::Buf], Type::Buf),
            Self::StrLen => function(vec![Type::Str], Type::I64),
            Self::StrRef => function(vec![Type::Str, Type::I64], Type::I64),
            Self::StrAppend => function(vec![Type::Str, Type::Str], Type::Str),
            Self::StrSlice => function(vec![Type::Str, Type::I64, Type::I64], Type::Str),
            Self::StrFromByte | Self::StrFromI64 => function(vec![Type::I64], Type::Str),
            Self::StrFromF64 => function(vec![Type::F64], Type::Str),
            Self::StdinHandle => function(Vec::new(), Type::Handle),
            Self::SysIsatty => function(vec![Type::Handle], system_result(Type::Bool)),
            Self::SysClose => function(vec![Type::Handle], system_result(Type::Unit)),
            Self::SysReadByte => function(vec![Type::Handle], system_result(Type::I64)),
            Self::SysWriteByte => {
                function(vec![Type::Handle, Type::I64], system_result(Type::Unit))
            }
            Self::SysTtyGuardSave => function(vec![Type::Buf], system_result(Type::Unit)),
            Self::SysTtyGuardClear => function(Vec::new(), system_result(Type::Unit)),
            Self::SysOpenRead | Self::SysOpenWrite => {
                function(vec![Type::Str], system_result(Type::Handle))
            }
            Self::SysPathExists => function(vec![Type::Str], system_result(Type::Bool)),
            Self::SysWaitMs => function(vec![Type::I64], system_result(Type::Unit)),
            Self::SysNowMs => function(Vec::new(), system_result(Type::I64)),
            Self::SysSocket => function(Vec::new(), system_result(Type::Handle)),
            Self::SysBind | Self::SysListen => {
                function(vec![Type::Handle, Type::I64], system_result(Type::Unit))
            }
            Self::SysAccept => function(vec![Type::Handle], system_result(Type::Handle)),
            Self::SysRecv => function(vec![Type::Handle], system_result(Type::Str)),
            Self::SysSend => function(vec![Type::Handle, Type::Str], system_result(Type::I64)),
            Self::SysPoll => function(vec![Type::Handle, Type::I64], system_result(Type::I64)),
            Self::SysTtyGet | Self::SysTtySet => {
                function(vec![Type::Handle, Type::Buf], system_result(Type::Unit))
            }
            Self::Ok => {
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
            Self::Err => {
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
            Self::IsOk => {
                let result = generic_result();
                forall(&["T", "E"], function(vec![result], Type::Bool))
            }
            Self::UnwrapOk => {
                let result = generic_result();
                forall(&["T", "E"], function(vec![result], Type::Param("T".into())))
            }
            Self::UnwrapErr => {
                let result = generic_result();
                forall(&["T", "E"], function(vec![result], Type::Param("E".into())))
            }
        }
    }
}

impl Operation {
    pub fn resolve_types(self, arguments: &[Type]) -> Result<(Type, Type), String> {
        let expected = callable_arity(&self.signature())
            .ok_or_else(|| format!("{} has no callable signature", self.name()))?;
        if arguments.len() != expected {
            return Err(format!(
                "{}: expected {expected} args, got {}",
                self.name(),
                arguments.len()
            ));
        }
        let result = match self {
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => {
                let mut saw_f64 = false;
                for argument in arguments {
                    match argument {
                        Type::I64 => {}
                        Type::F64 => saw_f64 = true,
                        other => {
                            return Err(format!(
                                "{}: expected I64 or F64, got {other:?}",
                                self.name()
                            ));
                        }
                    }
                }
                if saw_f64 {
                    Type::F64
                } else {
                    Type::I64
                }
            }
            Self::Equal | Self::NotEqual => {
                let left = &arguments[0];
                let right = &arguments[1];
                if both_numeric(left, right)
                    || Type::unify_assignable(left, right)
                    || Type::unify_assignable(right, left)
                {
                    Type::Bool
                } else {
                    return Err(format!(
                        "{}: equality type mismatch {left:?} vs {right:?}",
                        self.name()
                    ));
                }
            }
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => {
                let left = &arguments[0];
                let right = &arguments[1];
                if both_numeric(left, right) {
                    Type::Bool
                } else {
                    return Err(format!(
                        "{}: expected numeric operands, got {left:?} and {right:?}",
                        self.name()
                    ));
                }
            }
            _ => instantiate_result(self.name(), self.signature(), arguments)?,
        };
        let resolved = Type::Fn {
            params: arguments.to_vec(),
            ret: Box::new(result.clone()),
        };
        Ok((resolved, result))
    }

    pub const fn effects(self) -> crate::hir::EffectSet {
        use crate::hir::EffectSet;

        match self {
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => EffectSet::MAY_TRAP,
            Self::Cons
            | Self::StrAppend
            | Self::StrFromByte
            | Self::StrFromI64
            | Self::StrFromF64
            | Self::EmptyStr
            | Self::BufNew
            | Self::BufClone
            | Self::Ok
            | Self::Err => EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP),
            Self::Car
            | Self::Cdr
            | Self::BufRef
            | Self::BufGetU32
            | Self::StrRef
            | Self::StrSlice
            | Self::UnwrapOk
            | Self::UnwrapErr => EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP),
            Self::BufSet | Self::BufSetU32 => EffectSet::WRITES_MEMORY.union(EffectSet::MAY_TRAP),
            Self::BufLen | Self::StrLen | Self::IsOk => EffectSet::READS_MEMORY,
            Self::Print
            | Self::Flush
            | Self::ReadByte
            | Self::WriteByte
            | Self::WriteStr
            | Self::ArgCount
            | Self::Arg
            | Self::StdinHandle
            | Self::SysIsatty
            | Self::SysClose
            | Self::SysReadByte
            | Self::SysWriteByte
            | Self::SysTtyGuardSave
            | Self::SysTtyGuardClear
            | Self::SysOpenRead
            | Self::SysOpenWrite
            | Self::SysPathExists
            | Self::SysWaitMs
            | Self::SysNowMs
            | Self::SysSocket
            | Self::SysBind
            | Self::SysListen
            | Self::SysAccept
            | Self::SysRecv
            | Self::SysSend
            | Self::SysPoll
            | Self::SysTtyGet
            | Self::SysTtySet => EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::MAY_TRAP),
            Self::Exit => EffectSet::HOST_IO
                .union(EffectSet::MAY_EXIT)
                .union(EffectSet::MAY_TRAP),
            Self::Equal
            | Self::NotEqual
            | Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual
            | Self::Not
            | Self::BitAnd
            | Self::BitOr
            | Self::BitXor
            | Self::IsNil
            | Self::And
            | Self::Or
            | Self::IsEmptyList => EffectSet::PURE,
        }
    }
}

fn instantiate_result(name: &str, callable: Type, arguments: &[Type]) -> Result<Type, String> {
    let (parameters, result) = match callable {
        Type::Forall { vars, body } => {
            let Type::Fn { params, ret } = *body else {
                return Err(format!("{name}: forall body is not a function"));
            };
            let mut substitutions = std::collections::HashMap::new();
            for (pattern, argument) in params.iter().zip(arguments) {
                bind_type_params(name, pattern, argument, &vars, &mut substitutions)?;
            }
            for variable in &vars {
                if !substitutions.contains_key(variable) {
                    return Err(format!(
                        "{name}: cannot infer type parameter {variable} from arguments"
                    ));
                }
            }
            (
                params
                    .iter()
                    .map(|parameter| parameter.subst(&substitutions))
                    .collect(),
                ret.subst(&substitutions),
            )
        }
        Type::Fn { params, ret } => (params, *ret),
        other => return Err(format!("{name} is not a function ({other:?})")),
    };
    if parameters.len() != arguments.len() {
        return Err(format!(
            "{name}: expected {} args, got {}",
            parameters.len(),
            arguments.len()
        ));
    }
    for (parameter, argument) in parameters.iter().zip(arguments) {
        if !Type::unify_assignable(argument, parameter) {
            return Err(format!(
                "{name}: arg type {argument:?} not assignable to {parameter:?}"
            ));
        }
    }
    Ok(result)
}

fn bind_type_params(
    name: &str,
    pattern: &Type,
    argument: &Type,
    variables: &[String],
    substitutions: &mut std::collections::HashMap<String, Type>,
) -> Result<(), String> {
    match (pattern, argument) {
        (Type::Param(parameter), argument) if variables.iter().any(|item| item == parameter) => {
            if let Some(previous) = substitutions.get(parameter) {
                if previous != argument {
                    return Err(format!(
                        "{name}: type parameter {parameter} conflict: {previous:?} vs {argument:?}"
                    ));
                }
            } else {
                substitutions.insert(parameter.clone(), argument.clone());
            }
            Ok(())
        }
        (Type::List(pattern), Type::List(argument))
        | (Type::Option(pattern), Type::Option(argument)) => {
            bind_type_params(name, pattern, argument, variables, substitutions)
        }
        (Type::Result(ok_pattern, err_pattern), Type::Result(ok_argument, err_argument)) => {
            bind_type_params(name, ok_pattern, ok_argument, variables, substitutions)?;
            bind_type_params(name, err_pattern, err_argument, variables, substitutions)
        }
        (pattern, argument) if Type::unify_assignable(argument, pattern) => Ok(()),
        (pattern, argument) => Err(format!(
            "{name}: cannot instantiate {pattern:?} from {argument:?}"
        )),
    }
}

fn callable_arity(ty: &Type) -> Option<usize> {
    match ty {
        Type::Fn { params, .. } => Some(params.len()),
        Type::Forall { body, .. } => callable_arity(body),
        _ => None,
    }
}

fn both_numeric(left: &Type, right: &Type) -> bool {
    matches!(left, Type::I64 | Type::F64) && matches!(right, Type::I64 | Type::F64)
}

fn function(params: Vec<Type>, ret: Type) -> Type {
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

fn generic_result() -> Type {
    Type::Result(
        Box::new(Type::Param("T".into())),
        Box::new(Type::Param("E".into())),
    )
}
