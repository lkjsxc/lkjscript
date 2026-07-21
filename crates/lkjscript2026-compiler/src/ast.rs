//! XML-like expression AST.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Number, bool, or nil literal.
    LitNum(f64),
    LitBool(bool),
    LitNil,
    LitStr(String),
    Symbol(String),
    /// Generic call: tag name with child args.
    Call { name: String, args: Vec<Expr> },
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
