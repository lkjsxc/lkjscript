//! Expression AST lowered from canonical lkjscript source.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Numeric literal. `float_syntax` is true when the source used a `.` (e.g. `2.0`).
    LitNum {
        value: f64,
        float_syntax: bool,
    },
    LitBool(bool),
    LitNil,
    LitStr(String),
    Symbol(String),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    List(Vec<Expr>),
}

impl Expr {
    pub fn depth(&self) -> u32 {
        match self {
            Expr::Call { args, .. } | Expr::List(args) => {
                1 + args.iter().map(Expr::depth).max().unwrap_or(0)
            }
            _ => 0,
        }
    }
}
