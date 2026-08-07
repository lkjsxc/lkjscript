use std::path::PathBuf;

use crate::source::{SourceOrigin, SourceSpan};

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) enum Expr {
    LitI64(i64),
    LitF64(f64),
    LitBool(bool),
    LitUnit,
    LitStr(String),
    LitBytes(Vec<u8>),
    Symbol(String),
    Call { name: String, args: Vec<Expr> },
    List(Vec<Expr>),
}

impl Clone for Expr {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a Expr),
            Call(&'a str, usize),
            List(usize),
        }

        let mut work = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(expression) => match expression {
                    Expr::LitI64(value) => completed.push(Expr::LitI64(*value)),
                    Expr::LitF64(value) => completed.push(Expr::LitF64(*value)),
                    Expr::LitBool(value) => completed.push(Expr::LitBool(*value)),
                    Expr::LitUnit => completed.push(Expr::LitUnit),
                    Expr::LitStr(value) => completed.push(Expr::LitStr(value.clone())),
                    Expr::LitBytes(value) => completed.push(Expr::LitBytes(value.clone())),
                    Expr::Symbol(value) => completed.push(Expr::Symbol(value.clone())),
                    Expr::Call { name, args } => {
                        work.push(Work::Call(name, args.len()));
                        work.extend(args.iter().rev().map(Work::Visit));
                    }
                    Expr::List(values) => {
                        work.push(Work::List(values.len()));
                        work.extend(values.iter().rev().map(Work::Visit));
                    }
                },
                Work::Call(name, child_count) => {
                    let split = completed.len() - child_count;
                    let args = completed.split_off(split);
                    completed.push(Expr::Call {
                        name: name.to_string(),
                        args,
                    });
                }
                Work::List(child_count) => {
                    let split = completed.len() - child_count;
                    let values = completed.split_off(split);
                    completed.push(Expr::List(values));
                }
            }
        }
        completed.pop().unwrap_or(Expr::LitUnit)
    }
}

impl Drop for Expr {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_expr_children(self, &mut pending);
        while let Some(mut expression) = pending.pop() {
            take_expr_children(&mut expression, &mut pending);
        }
    }
}

fn take_expr_children(expression: &mut Expr, pending: &mut Vec<Expr>) {
    match expression {
        Expr::Call { args, .. } | Expr::List(args) => pending.append(args),
        Expr::LitI64(_)
        | Expr::LitF64(_)
        | Expr::LitBool(_)
        | Expr::LitUnit
        | Expr::LitStr(_)
        | Expr::LitBytes(_)
        | Expr::Symbol(_) => {}
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SyntaxKind {
    I64 { value: i64 },
    F64 { value: f64 },
    Bool { value: bool },
    Unit,
    Str { value: String },
    Bytes { value: Vec<u8> },
    Symbol { name: String },
    Call { name: String },
}

#[derive(Debug)]
pub(crate) struct SourceNode {
    pub(crate) kind: SyntaxKind,
    pub(crate) span: SourceSpan,
    pub(crate) leading_trivia: Vec<String>,
    pub(crate) before_close_trivia: Vec<String>,
    pub(crate) children: Vec<SourceNode>,
}

impl SourceNode {
    pub(crate) fn project_all(nodes: &[Self]) -> Result<Vec<Expr>, ()> {
        enum Work<'a> {
            Visit(&'a SourceNode),
            Call(&'a str, usize),
        }

        let mut work = Vec::new();
        work.try_reserve(nodes.len()).map_err(|_| ())?;
        work.extend(nodes.iter().rev().map(Work::Visit));
        let mut completed = Vec::new();
        completed.try_reserve(nodes.len()).map_err(|_| ())?;
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(node) => match &node.kind {
                    SyntaxKind::I64 { value } => {
                        completed.try_reserve(1).map_err(|_| ())?;
                        completed.push(Expr::LitI64(*value));
                    }
                    SyntaxKind::F64 { value } => {
                        completed.try_reserve(1).map_err(|_| ())?;
                        completed.push(Expr::LitF64(*value));
                    }
                    SyntaxKind::Bool { value } => {
                        completed.try_reserve(1).map_err(|_| ())?;
                        completed.push(Expr::LitBool(*value));
                    }
                    SyntaxKind::Unit => {
                        completed.try_reserve(1).map_err(|_| ())?;
                        completed.push(Expr::LitUnit);
                    }
                    SyntaxKind::Str { value } => {
                        completed.try_reserve(1).map_err(|_| ())?;
                        completed.push(Expr::LitStr(try_clone_string(value)?));
                    }
                    SyntaxKind::Bytes { value } => {
                        let mut bytes = Vec::new();
                        bytes.try_reserve_exact(value.len()).map_err(|_| ())?;
                        bytes.extend_from_slice(value);
                        completed.try_reserve(1).map_err(|_| ())?;
                        completed.push(Expr::LitBytes(bytes));
                    }
                    SyntaxKind::Symbol { name } => {
                        completed.try_reserve(1).map_err(|_| ())?;
                        completed.push(Expr::Symbol(try_clone_string(name)?));
                    }
                    SyntaxKind::Call { name } => {
                        work.try_reserve(1).map_err(|_| ())?;
                        work.push(Work::Call(name, node.children.len()));
                        work.try_reserve(node.children.len()).map_err(|_| ())?;
                        work.extend(node.children.iter().rev().map(Work::Visit));
                    }
                },
                Work::Call(name, child_count) => {
                    let split = completed.len().checked_sub(child_count).ok_or(())?;
                    let args = completed.split_off(split);
                    completed.try_reserve(1).map_err(|_| ())?;
                    completed.push(Expr::Call {
                        name: try_clone_string(name)?,
                        args,
                    });
                }
            }
        }
        Ok(completed)
    }

    fn clone_all(nodes: &[Self]) -> Vec<Self> {
        enum Work<'a> {
            Visit(&'a SourceNode),
            Finish(&'a SourceNode),
        }

        let mut work = Vec::new();
        work.extend(nodes.iter().rev().map(Work::Visit));
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(node) => {
                    work.push(Work::Finish(node));
                    work.extend(node.children.iter().rev().map(Work::Visit));
                }
                Work::Finish(node) => {
                    let split = completed.len() - node.children.len();
                    let children = completed.split_off(split);
                    completed.push(Self {
                        kind: node.kind.clone(),
                        span: node.span,
                        leading_trivia: node.leading_trivia.clone(),
                        before_close_trivia: node.before_close_trivia.clone(),
                        children,
                    });
                }
            }
        }
        completed
    }
}

fn try_clone_string(value: &str) -> Result<String, ()> {
    let mut output = String::new();
    output.try_reserve_exact(value.len()).map_err(|_| ())?;
    output.push_str(value);
    Ok(output)
}

impl Clone for SourceNode {
    fn clone(&self) -> Self {
        SourceNode::clone_all(std::slice::from_ref(self))
            .pop()
            .unwrap_or_else(|| Self {
                kind: SyntaxKind::Unit,
                span: self.span,
                leading_trivia: Vec::new(),
                before_close_trivia: Vec::new(),
                children: Vec::new(),
            })
    }
}

impl Drop for SourceNode {
    fn drop(&mut self) {
        let mut pending = std::mem::take(&mut self.children);
        while let Some(mut node) = pending.pop() {
            pending.append(&mut node.children);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum TokenKind {
    Open(String),
    Close(String),
    Atom(String),
    Str(String),
    Bytes(Vec<u8>),
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
}
