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
    EqualValue,
    SameObject,
    ListEqual,
    F64BitsEqual,
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
    BufFromStr,
    BufToStr,
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
    SysReadInto,
    SysWriteFrom,
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
    Some,
    IsSome,
    UnwrapSome,
}

impl Operation {
    /// All current source-visible operations.
    pub const ALL: &'static [Self] = &[
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Divide,
        Self::EqualValue,
        Self::SameObject,
        Self::ListEqual,
        Self::F64BitsEqual,
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
        Self::BufFromStr,
        Self::BufToStr,
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
        Self::SysReadInto,
        Self::SysWriteFrom,
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
        Self::Some,
        Self::IsSome,
        Self::UnwrapSome,
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
            Self::EqualValue => "equal-value",
            Self::SameObject => "same-object",
            Self::ListEqual => "list-equal",
            Self::F64BitsEqual => "f64-bits-equal",
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
            Self::BufFromStr => "buf-from-str",
            Self::BufToStr => "buf-to-str",
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
            Self::SysReadInto => "sys-read-into",
            Self::SysWriteFrom => "sys-write-from",
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
            Self::Some => "some",
            Self::IsSome => "is-some",
            Self::UnwrapSome => "unwrap-some",
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
            Self::EqualValue | Self::SameObject => forall(
                &["T"],
                function(
                    vec![Type::Param("T".into()), Type::Param("T".into())],
                    Type::Bool,
                ),
            ),
            Self::ListEqual => {
                let item = Type::Param("T".into());
                let list = Type::List(Box::new(item));
                forall(&["T"], function(vec![list.clone(), list], Type::Bool))
            }
            Self::F64BitsEqual => function(vec![Type::F64, Type::F64], Type::Bool),
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => {
                numeric_comparison()
            }
            Self::BitAnd | Self::BitOr | Self::BitXor => i64_binary(),
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
            Self::Arg => function(vec![Type::I64], Type::Option(Box::new(Type::Str))),
            Self::BufNew => function(vec![Type::I64], Type::Buf),
            Self::BufLen => function(vec![Type::Buf], Type::I64),
            Self::BufRef | Self::BufGetU32 => function(vec![Type::Buf, Type::I64], Type::I64),
            Self::BufSet | Self::BufSetU32 => {
                function(vec![Type::Buf, Type::I64, Type::I64], Type::Unit)
            }
            Self::BufClone => function(vec![Type::Buf], Type::Buf),
            Self::BufFromStr => function(vec![Type::Str], Type::Buf),
            Self::BufToStr => function(vec![Type::Buf], system_result(Type::Str)),
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
            Self::SysReadInto | Self::SysWriteFrom => function(
                vec![Type::Handle, Type::Buf, Type::I64, Type::I64],
                system_result(Type::I64),
            ),
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
            Self::Some => {
                let value = Type::Param("T".into());
                forall(
                    &["T"],
                    function(vec![value.clone()], Type::Option(Box::new(value))),
                )
            }
            Self::IsSome => {
                let value = Type::Param("T".into());
                forall(
                    &["T"],
                    function(vec![Type::Option(Box::new(value))], Type::Bool),
                )
            }
            Self::UnwrapSome => {
                let value = Type::Param("T".into());
                forall(
                    &["T"],
                    function(vec![Type::Option(Box::new(value.clone()))], value),
                )
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
            Self::EqualValue => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "equal-value: operands must have the same type, got {left:?} and {right:?}"
                    ));
                }
                if !supports_value_equality(left) {
                    return Err(format!(
                        "equal-value: type {left:?} does not support value equality"
                    ));
                }
                Type::Bool
            }
            Self::SameObject => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "same-object: operands must have the same type, got {left:?} and {right:?}"
                    ));
                }
                if !matches!(left, Type::Buf | Type::Handle) {
                    return Err(format!(
                        "same-object: type {left:?} does not have object identity"
                    ));
                }
                Type::Bool
            }
            Self::ListEqual => {
                let left = &arguments[0];
                let right = &arguments[1];
                if left != right {
                    return Err(format!(
                        "list-equal: operands must have the same type, got {left:?} and {right:?}"
                    ));
                }
                let Type::List(item) = left else {
                    return Err(format!("list-equal: expected List, got {left:?}"));
                };
                if !supports_value_equality(item) {
                    return Err(format!(
                        "list-equal: element type {item:?} does not support value equality"
                    ));
                }
                Type::Bool
            }
            Self::F64BitsEqual => {
                if arguments == [Type::F64, Type::F64] {
                    Type::Bool
                } else {
                    return Err(format!(
                        "f64-bits-equal: expected F64 and F64, got {:?} and {:?}",
                        arguments[0], arguments[1]
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
            Self::BufFromStr | Self::BufToStr => EffectSet::ALLOCATES
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::MAY_TRAP),
            Self::Cons
            | Self::StrAppend
            | Self::StrFromByte
            | Self::StrFromI64
            | Self::StrFromF64
            | Self::EmptyStr
            | Self::BufNew
            | Self::BufClone
            | Self::Ok
            | Self::Err
            | Self::Some => EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP),
            Self::Car
            | Self::Cdr
            | Self::BufRef
            | Self::BufGetU32
            | Self::StrRef
            | Self::StrSlice
            | Self::UnwrapOk
            | Self::UnwrapErr
            | Self::UnwrapSome => EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP),
            Self::BufSet | Self::BufSetU32 => EffectSet::WRITES_MEMORY.union(EffectSet::MAY_TRAP),
            Self::BufLen | Self::StrLen | Self::IsOk | Self::IsSome => EffectSet::READS_MEMORY,
            Self::SysReadInto => EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::WRITES_MEMORY)
                .union(EffectSet::MAY_TRAP),
            Self::SysWriteFrom => EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::MAY_TRAP),
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
            Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual
            | Self::Not
            | Self::BitAnd
            | Self::BitOr
            | Self::BitXor
            | Self::And
            | Self::Or
            | Self::IsEmptyList => EffectSet::PURE,
            Self::EqualValue | Self::SameObject | Self::F64BitsEqual => EffectSet::READS_MEMORY,
            Self::ListEqual => EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP),
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

fn supports_value_equality(ty: &Type) -> bool {
    match ty {
        Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Symbol => true,
        Type::Option(value) => supports_value_equality(value),
        Type::Result(ok, err) => supports_value_equality(ok) && supports_value_equality(err),
        Type::Buf
        | Type::Handle
        | Type::Product(_)
        | Type::Param(_)
        | Type::List(_)
        | Type::Fn { .. }
        | Type::Forall { .. } => false,
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

#[cfg(test)]
mod tests {
    use crate::hir::EffectSet;
    use crate::types::Type;

    use super::{function, Operation};

    #[test]
    fn explicit_equality_families_enforce_static_categories() {
        for ty in [
            Type::Unit,
            Type::Bool,
            Type::I64,
            Type::F64,
            Type::Str,
            Type::Symbol,
            Type::Option(Box::new(Type::I64)),
            Type::Result(Box::new(Type::Str), Box::new(Type::I64)),
        ] {
            assert!(Operation::EqualValue
                .resolve_types(&[ty.clone(), ty])
                .is_ok());
        }
        for ty in [
            Type::Buf,
            Type::Handle,
            Type::List(Box::new(Type::I64)),
            Type::Param("T".into()),
            Type::Fn {
                params: Vec::new(),
                ret: Box::new(Type::Unit),
            },
        ] {
            assert!(Operation::EqualValue
                .resolve_types(&[ty.clone(), ty])
                .is_err());
        }
        assert!(Operation::EqualValue
            .resolve_types(&[Type::I64, Type::F64])
            .is_err());

        for ty in [Type::Buf, Type::Handle] {
            assert!(Operation::SameObject
                .resolve_types(&[ty.clone(), ty])
                .is_ok());
        }
        assert!(Operation::SameObject
            .resolve_types(&[Type::I64, Type::I64])
            .is_err());

        let list = Type::List(Box::new(Type::Option(Box::new(Type::Str))));
        assert!(Operation::ListEqual
            .resolve_types(&[list.clone(), list])
            .is_ok());
        let nested = Type::List(Box::new(Type::List(Box::new(Type::I64))));
        assert!(Operation::ListEqual
            .resolve_types(&[nested.clone(), nested])
            .is_err());
        let buffers = Type::List(Box::new(Type::Buf));
        assert!(Operation::ListEqual
            .resolve_types(&[buffers.clone(), buffers])
            .is_err());

        assert!(Operation::F64BitsEqual
            .resolve_types(&[Type::F64, Type::F64])
            .is_ok());
        assert!(Operation::F64BitsEqual
            .resolve_types(&[Type::I64, Type::I64])
            .is_err());
    }

    #[test]
    fn lossless_bulk_byte_operations_have_exact_signatures_and_effects() {
        let result_i64 = Type::Result(Box::new(Type::I64), Box::new(Type::Str));
        let result_str = Type::Result(Box::new(Type::Str), Box::new(Type::Str));
        assert_eq!(
            Operation::from_name("buf-from-str"),
            Some(Operation::BufFromStr)
        );
        assert_eq!(
            Operation::from_name("buf-to-str"),
            Some(Operation::BufToStr)
        );
        assert_eq!(
            Operation::SysReadInto.resolve_types(&[Type::Handle, Type::Buf, Type::I64, Type::I64]),
            Ok((
                function(
                    vec![Type::Handle, Type::Buf, Type::I64, Type::I64],
                    result_i64.clone()
                ),
                result_i64.clone(),
            ))
        );
        assert_eq!(
            Operation::BufToStr.resolve_types(&[Type::Buf]),
            Ok((function(vec![Type::Buf], result_str.clone()), result_str))
        );
        assert_eq!(
            Operation::BufFromStr.effects(),
            EffectSet::ALLOCATES
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::MAY_TRAP)
        );
        assert_eq!(
            Operation::SysReadInto.effects(),
            EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::WRITES_MEMORY)
                .union(EffectSet::MAY_TRAP)
        );
    }

    #[test]
    fn removed_equality_names_and_effects_are_truthful() {
        assert!(Operation::from_name("eq").is_none());
        assert!(Operation::from_name("ne").is_none());
        assert_eq!(Operation::EqualValue.effects(), EffectSet::READS_MEMORY);
        assert_eq!(Operation::SameObject.effects(), EffectSet::READS_MEMORY);
        assert_eq!(Operation::F64BitsEqual.effects(), EffectSet::READS_MEMORY);
        assert_eq!(
            Operation::ListEqual.effects(),
            EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP)
        );
    }
}
