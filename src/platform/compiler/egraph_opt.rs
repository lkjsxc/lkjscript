//! Deterministic bounded algebraic saturation over pure normalized expression trees.
//!
//! Rewrites are a derived native lowering plan. The admitted NormalizedProgram and
//! its metered VM instruction stream stay unchanged: work, allocation and failure
//! checkpoints are semantic observations, not optimizer costs.
use crate::platform::execution::normalized::prepare::{
    NormalizedCode, NormalizedFunctionBody, NormalizedInstruction as I, NormalizedProgram,
};
use std::collections::{BTreeMap, BTreeSet};

mod fusion;

pub(crate) const MAX_ITERATIONS: usize = 8;
pub(crate) const MAX_NODES: usize = 1024;
pub(crate) const MAX_INSTRUCTIONS: usize = 65_536;

#[derive(Clone, Debug)]
pub(crate) struct Rewrite {
    pub end: usize,
    pub instructions: Vec<I>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Plan {
    pub rewrites: BTreeMap<usize, Rewrite>,
    pub iterations: usize,
    pub nodes: usize,
    pub fusions: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Less,
    LessEqual,
}

#[derive(Clone, Debug)]
enum Node {
    I64(i64),
    Bool(bool),
    Leaf(usize),
    Binary {
        operation: Op,
        left: usize,
        right: usize,
        call: usize,
    },
    Alias(usize),
}

#[derive(Clone, Copy)]
struct Term {
    node: usize,
    start: usize,
}

fn operation(program: &NormalizedProgram, instruction: &I) -> Option<Op> {
    let (I::Call {
        function,
        arguments: 2,
        ..
    }
    | I::TailCall {
        function,
        arguments: 2,
        ..
    }) = instruction
    else {
        return None;
    };
    let function = program.functions.get(function.0 as usize)?;
    if function.effect != crate::platform::kernel::FunctionEffect::Pure
        || !function.task_requirements.is_empty()
    {
        return None;
    }
    let NormalizedFunctionBody::External(name) = &function.body else {
        return None;
    };
    match name.as_str() {
        "core.i64.add" => Some(Op::Add),
        "core.i64.subtract" => Some(Op::Sub),
        "core.i64.multiply" => Some(Op::Mul),
        "core.i64.divide" => Some(Op::Div),
        "core.i64.equal" => Some(Op::Eq),
        "core.i64.less" => Some(Op::Less),
        "core.i64.less-equal" => Some(Op::LessEqual),
        _ => None,
    }
}

fn root(nodes: &[Node], mut n: usize) -> usize {
    // Alias edges always point to a strict ancestor in the expression DAG.
    while let Node::Alias(next) = nodes[n] {
        n = next;
    }
    n
}

fn integer(nodes: &[Node], n: usize) -> Option<i64> {
    if let Node::I64(value) = nodes[root(nodes, n)] {
        Some(value)
    } else {
        None
    }
}

fn saturate(nodes: &mut [Node], first: usize) -> usize {
    for iteration in 0..MAX_ITERATIONS {
        let mut changed = false;
        // Earlier nodes reached a fixed point before this node was appended.
        for index in first..nodes.len() {
            let Node::Binary {
                operation,
                left,
                right,
                ..
            } = nodes[index]
            else {
                continue;
            };
            let l = integer(nodes, left);
            let r = integer(nodes, right);
            let replacement = if let (Some(l), Some(r)) = (l, r) {
                match operation {
                    Op::Add => l.checked_add(r).map(Node::I64),
                    Op::Sub => l.checked_sub(r).map(Node::I64),
                    Op::Mul => l.checked_mul(r).map(Node::I64),
                    Op::Div => l.checked_div(r).map(Node::I64),
                    Op::Eq => Some(Node::Bool(l == r)),
                    Op::Less => Some(Node::Bool(l < r)),
                    Op::LessEqual => Some(Node::Bool(l <= r)),
                }
            } else {
                match (operation, l, r) {
                    (Op::Add, _, Some(0))
                    | (Op::Sub, _, Some(0))
                    | (Op::Mul, _, Some(1))
                    | (Op::Div, _, Some(1)) => Some(Node::Alias(left)),
                    (Op::Add, Some(0), _) | (Op::Mul, Some(1), _) => Some(Node::Alias(right)),
                    _ => None,
                }
            };
            if let Some(replacement) = replacement {
                nodes[index] = replacement;
                changed = true;
            }
        }
        if !changed {
            return iteration + 1;
        }
    }
    MAX_ITERATIONS
}

fn extract(nodes: &[Node], node: usize, code: &NormalizedCode) -> Vec<I> {
    let mut output = Vec::new();
    let mut pending = vec![(node, false)];
    while let Some((node, ready)) = pending.pop() {
        match nodes[root(nodes, node)] {
            Node::I64(v) => output.push(I::I64(v)),
            Node::Bool(v) => output.push(I::Bool(v)),
            Node::Leaf(ip) => output.push(code.instructions[ip].clone()),
            Node::Binary {
                left, right, call, ..
            } => {
                if ready {
                    output.push(code.instructions[call].clone());
                } else {
                    pending.extend([(node, true), (right, false), (left, false)]);
                }
            }
            Node::Alias(_) => {}
        }
    }
    output
}

pub(crate) fn optimize(program: &NormalizedProgram, code: &NormalizedCode) -> Plan {
    let mut plan = Plan::default();
    if code.instructions.len() > MAX_INSTRUCTIONS {
        return plan;
    }
    let mut boundaries = BTreeSet::new();
    for instruction in code.instructions.iter() {
        match instruction {
            I::Jump(ip) | I::JumpIfFalse(ip) => {
                boundaries.insert(*ip as usize);
            }
            I::SwitchVariant(arms) => {
                boundaries.extend(arms.iter().map(|a| a.target as usize));
            }
            I::MatchOwned { cases, .. } => {
                boundaries.extend(cases.iter().map(|a| a.target as usize));
            }
            I::MatchBorrowedOwned { cases, .. } => {
                boundaries.extend(cases.iter().map(|a| a.target as usize));
            }
            _ => {}
        }
    }
    let mut nodes = Vec::new();
    let mut stack = Vec::<Term>::new();
    for (ip, instruction) in code.instructions.iter().enumerate() {
        if boundaries.contains(&ip) {
            stack.clear();
            nodes.clear();
        }
        // This is a total proof-work cap, not a per-window reset. Large code
        // keeps its remaining instructions verbatim after the cap is reached.
        if plan.nodes >= MAX_NODES {
            break;
        }
        let node = match instruction {
            I::I64(v) => Some(Node::I64(*v)),
            I::Bool(v) => Some(Node::Bool(*v)),
            I::LoadLocal { .. } | I::MoveLocal(_) => Some(Node::Leaf(ip)),
            _ => None,
        };
        if let Some(node) = node {
            stack.push(Term {
                node: nodes.len(),
                start: ip,
            });
            nodes.push(node);
            plan.nodes += 1;
        } else if let Some(operation) = operation(program, instruction) {
            let (Some(right), Some(left)) = (stack.pop(), stack.pop()) else {
                stack.clear();
                nodes.clear();
                continue;
            };
            let node = nodes.len();
            nodes.push(Node::Binary {
                operation,
                left: left.node,
                right: right.node,
                call: ip,
            });
            plan.nodes += 1;
            plan.iterations = plan.iterations.max(saturate(&mut nodes, node));
            if !matches!(nodes[node], Node::Binary { .. }) {
                let mut instructions = extract(&nodes, node, code);
                if matches!(instruction, I::TailCall { .. }) {
                    instructions.push(I::Return);
                }
                let nested: Vec<_> = plan
                    .rewrites
                    .range(left.start..=ip)
                    .map(|(&start, _)| start)
                    .collect();
                for start in nested {
                    plan.rewrites.remove(&start);
                }
                plan.rewrites.insert(
                    left.start,
                    Rewrite {
                        end: ip + 1,
                        instructions,
                    },
                );
            }
            stack.push(Term {
                node,
                start: left.start,
            });
            if matches!(instruction, I::TailCall { .. }) {
                stack.clear();
                nodes.clear();
            }
        } else {
            stack.clear();
            nodes.clear();
        }
    }
    fusion::augment(program, code, &mut plan);
    plan
}
