use std::path::PathBuf;

use crate::source::{SourceOrigin, SourceSpan};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    LitI64(i64),
    LitF64(f64),
    LitBool(bool),
    LitUnit,
    LitStr(String),
    Symbol(String),
    Call { name: String, args: Vec<Expr> },
    List(Vec<Expr>),
}

impl Expr {
    #[allow(dead_code)]
    pub(crate) fn depth(&self) -> u32 {
        match self {
            Self::Call { args, .. } | Self::List(args) => {
                1 + args.iter().map(Self::depth).max().unwrap_or(0)
            }
            _ => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SyntaxKind {
    I64 { value: i64 },
    F64 { value: f64 },
    Bool { value: bool },
    Unit,
    Str { value: String },
    Symbol { name: String },
    Call { name: String },
}

#[derive(Clone, Debug)]
pub(crate) struct SourceNode {
    pub(crate) kind: SyntaxKind,
    pub(crate) span: SourceSpan,
    pub(crate) leading_trivia: Vec<String>,
    pub(crate) before_close_trivia: Vec<String>,
    pub(crate) children: Vec<SourceNode>,
}

impl SourceNode {
    pub(crate) fn project(&self) -> Expr {
        match &self.kind {
            SyntaxKind::I64 { value } => Expr::LitI64(*value),
            SyntaxKind::F64 { value } => Expr::LitF64(*value),
            SyntaxKind::Bool { value } => Expr::LitBool(*value),
            SyntaxKind::Unit => Expr::LitUnit,
            SyntaxKind::Str { value } => Expr::LitStr(value.clone()),
            SyntaxKind::Symbol { name } => Expr::Symbol(name.clone()),
            SyntaxKind::Call { name } => Expr::Call {
                name: name.clone(),
                args: self.children.iter().map(Self::project).collect(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum TokenKind {
    Open(String),
    Close(String),
    Atom(String),
    Str(String),
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: SourceSpan,
    pub(crate) leading_trivia: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceFile {
    pub(crate) path: PathBuf,
    pub(crate) origin: SourceOrigin,
    pub(crate) exact_source_len: u64,
    pub(crate) exact_source_sha256: [u8; 32],
    pub(crate) forms: Vec<Expr>,
    pub(crate) syntax: Vec<SourceNode>,
    pub(crate) tokens: Vec<Token>,
    pub(crate) trailing_trivia: Vec<String>,
}
