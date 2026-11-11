//! Global source rules for coherence and finite generic instantiation.
//! The executable K checker still checks concrete function and witness types.
use crate::ast::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
enum Term {
    Unknown,
    Variable(usize),
    Named(String, Vec<usize>),
}

fn short(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}
fn qualified(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_string()
    } else {
        format!("{module}::{name}")
    }
}
fn push(terms: &mut Vec<Term>, term: Term) -> usize {
    let n = terms.len();
    terms.push(term);
    n
}
fn type_children(source: &Program, id: TypeId) -> Vec<TypeId> {
    match &source.types[id].kind {
        Type::Path(p) => p.args.clone(),
        Type::Ref { inner, .. } | Type::Slice(inner) => vec![*inner],
        Type::Array { inner, .. } => vec![*inner],
        Type::Tuple(v) => v.clone(),
        Type::Function { params, result } => {
            let mut v = params.clone();
            v.push(*result);
            v
        }
        _ => Vec::new(),
    }
}
fn type_term(
    source: &Program,
    id: TypeId,
    env: &BTreeMap<String, usize>,
    terms: &mut Vec<Term>,
) -> usize {
    let mut done: BTreeMap<usize, usize> = BTreeMap::new();
    let mut work = vec![(id, false)];
    while let Some((ty, ready)) = work.pop() {
        if done.contains_key(&ty) {
            continue;
        }
        if !ready {
            if let Type::Path(p) = &source.types[ty].kind {
                if p.args.is_empty() {
                    if let Some(value) = env.get(&p.name) {
                        done.insert(ty, *value);
                        continue;
                    }
                }
            }
            work.push((ty, true));
            let kids = type_children(source, ty);
            let mut i = kids.len();
            while i > 0 {
                i -= 1;
                work.push((kids[i], false));
            }
            continue;
        }
        let kids = type_children(source, ty);
        let mut values = Vec::new();
        for child in kids {
            values.push(done[&child]);
        }
        let term = match &source.types[ty].kind {
            Type::Infer => Term::Unknown,
            Type::Never => Term::Named("!".to_string(), values),
            Type::Path(p) => Term::Named(
                if p.name.starts_with('\'') {
                    "lifetime".to_string()
                } else {
                    short(&p.name).to_string()
                },
                values,
            ),
            Type::Ref { mutable, .. } => {
                Term::Named(if *mutable { "&mut" } else { "&" }.to_string(), values)
            }
            Type::Slice(_) => Term::Named("[]".to_string(), values),
            Type::Array { len, .. } => Term::Named(format!("[;{}]", len.replace('_', "")), values),
            Type::Tuple(_) => Term::Named("tuple".to_string(), values),
            Type::Function { .. } => Term::Named("fn".to_string(), values),
        };
        let value = push(terms, term);
        done.insert(ty, value);
    }
    done[&id]
}
fn variables(
    source: &Program,
    params: &[GenericParam],
    terms: &mut Vec<Term>,
    next: &mut usize,
) -> BTreeMap<String, usize> {
    let mut env = BTreeMap::new();
    for p in params {
        if !p.lifetime {
            let term = push(terms, Term::Variable(*next));
            *next += 1;
            env.insert(p.name.clone(), term);
        }
    }
    let _ = source;
    env
}
fn deref(terms: &[Term], bindings: &BTreeMap<usize, usize>, mut term: usize) -> usize {
    loop {
        match terms[term] {
            Term::Variable(v) => match bindings.get(&v) {
                Some(next) => term = *next,
                None => return term,
            },
            _ => return term,
        }
    }
}
fn occurs(terms: &[Term], bindings: &BTreeMap<usize, usize>, variable: usize, term: usize) -> bool {
    let mut stack = vec![term];
    while let Some(t) = stack.pop() {
        let t = deref(terms, bindings, t);
        match &terms[t] {
            Term::Variable(v) => {
                if *v == variable {
                    return true;
                }
            }
            Term::Named(_, kids) => {
                for child in kids {
                    stack.push(*child);
                }
            }
            Term::Unknown => {}
        }
    }
    false
}
fn unify(terms: &[Term], pairs: Vec<(usize, usize)>) -> bool {
    let mut bindings = BTreeMap::new();
    let mut stack = pairs;
    while let Some((a, b)) = stack.pop() {
        let a = deref(terms, &bindings, a);
        let b = deref(terms, &bindings, b);
        if a == b {
            continue;
        }
        match (&terms[a], &terms[b]) {
            (Term::Variable(x), Term::Variable(y)) if x == y => {}
            (Term::Variable(v), _) => {
                if occurs(terms, &bindings, *v, b) {
                    return false;
                }
                bindings.insert(*v, b);
            }
            (_, Term::Variable(v)) => {
                if occurs(terms, &bindings, *v, a) {
                    return false;
                }
                bindings.insert(*v, a);
            }
            (Term::Named(an, ak), Term::Named(bn, bk)) => {
                if an != bn || ak.len() != bk.len() {
                    return false;
                }
                for i in 0..ak.len() {
                    stack.push((ak[i], bk[i]));
                }
            }
            (Term::Unknown, _) | (_, Term::Unknown) => {}
        }
    }
    true
}
fn coherence(source: &Program) -> Result<(), Diagnostic> {
    let mut by_trait: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for i in 0..source.impls.len() {
        let implementation = &source.impls[i];
        let trait_ty = match implementation.trait_ty {
            Some(ty) => ty,
            None => continue,
        };
        let trait_name = match &source.types[trait_ty].kind {
            Type::Path(p) => short(&p.name).to_string(),
            _ => continue,
        };
        if let Some(previous) = by_trait.get(&trait_name) {
            for j in previous {
                let other = &source.impls[*j];
                let mut terms = Vec::new();
                let mut next = 0;
                let left = variables(source, &implementation.generics, &mut terms, &mut next);
                let right = variables(source, &other.generics, &mut terms, &mut next);
                let a = type_term(source, implementation.target, &left, &mut terms);
                let b = type_term(source, other.target, &right, &mut terms);
                let at = type_term(source, trait_ty, &left, &mut terms);
                let bt = type_term(source, other.trait_ty.unwrap(), &right, &mut terms);
                if unify(&terms, vec![(a, b), (at, bt)]) {
                    return Err(Diagnostic::new(
                        implementation.span,
                        "E_TRAIT",
                        "overlapping trait implementations",
                    ));
                }
            }
        }
        by_trait.entry(trait_name.clone()).or_default().push(i);
        let mut declared = None;
        for t in 0..source.traits.len() {
            if source.traits[t].name == trait_name {
                if declared.is_some() {
                    declared = None;
                    break;
                }
                declared = Some(t);
            }
        }
        if let Some(t) = declared {
            let trait_def = &source.traits[t];
            let mut names = BTreeMap::new();
            for method in &implementation.methods {
                let f = &source.functions[*method];
                if names.insert(f.name.clone(), *method).is_some() {
                    return Err(Diagnostic::new(
                        f.span,
                        "E_TRAIT",
                        "duplicate implementation method",
                    ));
                }
                let mut declaration = None;
                for m in &trait_def.methods {
                    if source.functions[*m].name == f.name {
                        declaration = Some(*m);
                        break;
                    }
                }
                match declaration {
                    Some(m) => {
                        let expected = &source.functions[m];
                        if expected.params.len() != f.params.len()
                            || expected.generics.len() != f.generics.len()
                        {
                            return Err(Diagnostic::new(
                                f.span,
                                "E_TRAIT",
                                "implementation method parameter count differs from trait",
                            ));
                        }
                    }
                    None => {
                        return Err(Diagnostic::new(
                            f.span,
                            "E_TRAIT",
                            "implementation method is not declared by trait",
                        ));
                    }
                }
            }
            for m in &trait_def.methods {
                let f = &source.functions[*m];
                if f.body.is_none() && !names.contains_key(&f.name) {
                    return Err(Diagnostic::new(
                        implementation.span,
                        "E_TRAIT",
                        "implementation is missing a required method",
                    ));
                }
            }
            let mut associated = BTreeMap::new();
            for a in &implementation.associated {
                if associated.insert(a.name.clone(), ()).is_some() {
                    return Err(Diagnostic::new(
                        a.span,
                        "E_TRAIT",
                        "duplicate associated type",
                    ));
                }
                let mut found = false;
                for expected in &trait_def.associated {
                    if expected.name == a.name {
                        found = true;
                    }
                }
                if !found {
                    return Err(Diagnostic::new(
                        a.span,
                        "E_TRAIT",
                        "associated type is not declared by trait",
                    ));
                }
            }
            for a in &trait_def.associated {
                if a.value.is_none() && !associated.contains_key(&a.name) {
                    return Err(Diagnostic::new(
                        implementation.span,
                        "E_TRAIT",
                        "implementation is missing an associated type",
                    ));
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
struct Edge {
    from: usize,
    to: usize,
    expanding: bool,
    unknown: bool,
    span: Span,
}
fn components(count: usize, edges: &[Edge]) -> Vec<usize> {
    let mut forward = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for edge in edges {
        forward[edge.from].push(edge.to);
        reverse[edge.to].push(edge.from);
    }
    let mut seen = vec![false; count];
    let mut order = Vec::new();
    for start in 0..count {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0)];
        while let Some((vertex, next)) = stack.pop() {
            if next < forward[vertex].len() {
                stack.push((vertex, next + 1));
                let child = forward[vertex][next];
                if !seen[child] {
                    seen[child] = true;
                    stack.push((child, 0));
                }
            } else {
                order.push(vertex);
            }
        }
    }
    let mut result = vec![usize::MAX; count];
    let mut group = 0;
    while let Some(start) = order.pop() {
        if result[start] != usize::MAX {
            continue;
        }
        result[start] = group;
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            for next in &reverse[v] {
                if result[*next] == usize::MAX {
                    result[*next] = group;
                    stack.push(*next);
                }
            }
        }
        group += 1;
    }
    result
}
fn flow(terms: &[Term], term: usize, to: usize, span: Span, edges: &mut Vec<Edge>) {
    let mut stack = vec![(term, false)];
    while let Some((t, expanding)) = stack.pop() {
        match &terms[t] {
            Term::Variable(from) => edges.push(Edge {
                from: *from,
                to,
                expanding,
                unknown: false,
                span,
            }),
            Term::Named(_, kids) => {
                for child in kids {
                    stack.push((*child, true));
                }
            }
            Term::Unknown => {}
        }
    }
}
fn unknown(terms: &[Term], term: usize) -> bool {
    let mut stack = vec![term];
    while let Some(t) = stack.pop() {
        match &terms[t] {
            Term::Unknown => return true,
            Term::Named(_, kids) => {
                for k in kids {
                    stack.push(*k);
                }
            }
            Term::Variable(_) => {}
        }
    }
    false
}
fn infer(
    source: &Program,
    pattern: TypeId,
    actual: usize,
    parameters: &[GenericParam],
    terms: &[Term],
    env: &mut BTreeMap<String, usize>,
) {
    let mut stack = vec![(pattern, actual)];
    while let Some((p, a)) = stack.pop() {
        if let Type::Path(path) = &source.types[p].kind {
            if path.args.is_empty() {
                let mut parameter = false;
                for g in parameters {
                    if !g.lifetime && g.name == path.name {
                        parameter = true;
                        break;
                    }
                }
                if parameter {
                    env.entry(path.name.clone()).or_insert(a);
                    continue;
                }
            }
        }
        let expected = match &source.types[p].kind {
            Type::Path(path) => short(&path.name).to_string(),
            Type::Ref { mutable, .. } => if *mutable { "&mut" } else { "&" }.to_string(),
            Type::Tuple(_) => "tuple".to_string(),
            Type::Slice(_) => "[]".to_string(),
            Type::Array { len, .. } => format!("[;{}]", len.replace('_', "")),
            Type::Function { .. } => "fn".to_string(),
            _ => continue,
        };
        if let Term::Named(name, kids) = &terms[a] {
            let pattern_kids = type_children(source, p);
            if *name == expected && kids.len() == pattern_kids.len() {
                for i in 0..kids.len() {
                    stack.push((pattern_kids[i], kids[i]));
                }
            }
        }
    }
}
#[derive(Clone)]
enum Task {
    Expr(ExprId),
    Done(ExprId),
    Block(BlockId),
    BlockDone(BlockId, usize),
    Bind(PatternId, Option<TypeId>, Option<ExprId>),
}
struct Analysis<'a> {
    source: &'a Program,
    terms: Vec<Term>,
    generics: Vec<BTreeMap<String, usize>>,
    nodes: Vec<Vec<usize>>,
    names: BTreeMap<String, usize>,
    edges: Vec<Edge>,
    count: usize,
}
impl Analysis<'_> {
    fn term(&mut self, kind: Term) -> usize {
        push(&mut self.terms, kind)
    }
    fn lookup(&self, module: &str, name: &str) -> Option<usize> {
        if let Some(n) = self.names.get(name) {
            return Some(*n);
        }
        self.names.get(&qualified(module, name)).copied()
    }
    fn call(
        &mut self,
        caller: usize,
        callee: usize,
        types: &[TypeId],
        args: &[usize],
        span: Span,
    ) -> usize {
        let f = &self.source.functions[callee];
        let mut substitutions = BTreeMap::new();
        let mut index = 0;
        for g in &f.generics {
            if g.lifetime {
                continue;
            }
            if index < types.len() {
                let t = type_term(
                    self.source,
                    types[index],
                    &self.generics[caller],
                    &mut self.terms,
                );
                substitutions.insert(g.name.clone(), t);
            }
            index += 1;
        }
        for i in 0..args.len().min(f.params.len()) {
            infer(
                self.source,
                f.params[i].ty,
                args[i],
                &f.generics,
                &self.terms,
                &mut substitutions,
            );
        }
        for g in &f.generics {
            if g.lifetime {
                continue;
            }
            let target = match self.generics[callee].get(&g.name) {
                Some(t) => match self.terms[*t] {
                    Term::Variable(v) => v,
                    _ => continue,
                },
                None => continue,
            };
            match substitutions.get(&g.name) {
                Some(t) if !unknown(&self.terms, *t) => {
                    flow(&self.terms, *t, target, span, &mut self.edges)
                }
                _ => {
                    for from in &self.nodes[caller] {
                        self.edges.push(Edge {
                            from: *from,
                            to: target,
                            expanding: false,
                            unknown: true,
                            span,
                        });
                    }
                }
            }
        }
        type_term(self.source, f.result, &substitutions, &mut self.terms)
    }
    fn bind(&self, pattern: PatternId, term: usize, bindings: &mut Vec<(String, usize)>) {
        let mut stack = vec![(pattern, term)];
        while let Some((p, t)) = stack.pop() {
            match &self.source.patterns[p].kind {
                Pattern::Bind { name, .. } => bindings.push((name.clone(), t)),
                Pattern::Tuple(items) => {
                    if let Term::Named(name, kids) = &self.terms[t] {
                        if name == "tuple" && kids.len() == items.len() {
                            for i in 0..items.len() {
                                stack.push((items[i], kids[i]));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    fn function(&mut self, caller: usize) {
        let function = &self.source.functions[caller];
        let body = match function.body {
            Some(body) => body,
            None => return,
        };
        let env = self.generics[caller].clone();
        let mut bindings = Vec::new();
        for param in &function.params {
            let t = type_term(self.source, param.ty, &env, &mut self.terms);
            bindings.push((param.name.clone(), t));
        }
        let mut exprs: BTreeMap<usize, usize> = BTreeMap::new();
        let mut blocks: BTreeMap<usize, usize> = BTreeMap::new();
        let mut tasks = vec![Task::Block(body)];
        while let Some(task) = tasks.pop() {
            match task {
                Task::Block(id) => {
                    let block = &self.source.blocks[id];
                    tasks.push(Task::BlockDone(id, bindings.len()));
                    if let Some(tail) = block.tail {
                        tasks.push(Task::Expr(tail));
                    }
                    let mut i = block.statements.len();
                    while i > 0 {
                        i -= 1;
                        match &block.statements[i] {
                            Stmt::Let {
                                pattern, ty, value, ..
                            } => {
                                tasks.push(Task::Bind(*pattern, *ty, *value));
                                if let Some(value) = value {
                                    tasks.push(Task::Expr(*value));
                                }
                            }
                            Stmt::Expr(e) => tasks.push(Task::Expr(*e)),
                            Stmt::Item(_) => {}
                        }
                    }
                }
                Task::BlockDone(id, base) => {
                    let t = match self.source.blocks[id].tail {
                        Some(e) => *exprs.get(&e).unwrap_or(&0),
                        None => self.term(Term::Named("tuple".to_string(), Vec::new())),
                    };
                    blocks.insert(id, t);
                    bindings.truncate(base);
                }
                Task::Bind(pattern, ty, value) => {
                    let t = match ty {
                        Some(t) => type_term(self.source, t, &env, &mut self.terms),
                        None => match value {
                            Some(e) => *exprs.get(&e).unwrap_or(&0),
                            None => 0,
                        },
                    };
                    self.bind(pattern, t, &mut bindings);
                }
                Task::Expr(id) => {
                    tasks.push(Task::Done(id));
                    let kind = &self.source.exprs[id].kind;
                    let mut kids = Vec::new();
                    let mut child_blocks = Vec::new();
                    match kind {
                        Expr::Unary { value, .. }
                        | Expr::Borrow { value, .. }
                        | Expr::Cast { value, .. }
                        | Expr::Field { value, .. }
                        | Expr::Try(value) => kids.push(*value),
                        Expr::Binary { left, right, .. } => {
                            kids.push(*left);
                            kids.push(*right);
                        }
                        Expr::Assign { place, value, .. } => {
                            kids.push(*place);
                            kids.push(*value);
                        }
                        Expr::Call { callee, args } => {
                            kids.push(*callee);
                            kids.extend(args.clone());
                        }
                        Expr::Method { receiver, args, .. } => {
                            kids.push(*receiver);
                            kids.extend(args.clone());
                        }
                        Expr::Index { value, index } => {
                            kids.push(*value);
                            kids.push(*index);
                        }
                        Expr::Tuple(v) | Expr::Array(v) | Expr::Macro { args: v, .. } => {
                            kids.extend(v.clone())
                        }
                        Expr::Repeat { value, count } => {
                            kids.push(*value);
                            kids.push(*count);
                        }
                        Expr::Struct { fields, base, .. } => {
                            for (_, e) in fields {
                                kids.push(*e);
                            }
                            if let Some(e) = base {
                                kids.push(*e);
                            }
                        }
                        Expr::Block(b) => child_blocks.push(*b),
                        Expr::If {
                            condition,
                            then_block,
                            otherwise,
                        } => {
                            kids.push(*condition);
                            child_blocks.push(*then_block);
                            if let Some(e) = otherwise {
                                kids.push(*e);
                            }
                        }
                        Expr::IfLet {
                            value,
                            then_block,
                            otherwise,
                            ..
                        } => {
                            kids.push(*value);
                            child_blocks.push(*then_block);
                            if let Some(e) = otherwise {
                                kids.push(*e);
                            }
                        }
                        Expr::While { condition, body }
                        | Expr::WhileLet {
                            value: condition,
                            body,
                            ..
                        } => {
                            kids.push(*condition);
                            child_blocks.push(*body);
                        }
                        Expr::Loop { body } => child_blocks.push(*body),
                        Expr::For { iterator, body, .. } => {
                            kids.push(*iterator);
                            child_blocks.push(*body);
                        }
                        Expr::Match { value, arms } => {
                            kids.push(*value);
                            for arm in arms {
                                if let Some(g) = arm.guard {
                                    kids.push(g);
                                }
                                kids.push(arm.value);
                            }
                        }
                        Expr::Closure { body, .. } => kids.push(*body),
                        Expr::Return(value) | Expr::Break(value) => {
                            if let Some(value) = value {
                                kids.push(*value);
                            }
                        }
                        Expr::Range { start, end, .. } => {
                            if let Some(e) = start {
                                kids.push(*e);
                            }
                            if let Some(e) = end {
                                kids.push(*e);
                            }
                        }
                        _ => {}
                    }
                    let mut i = child_blocks.len();
                    while i > 0 {
                        i -= 1;
                        tasks.push(Task::Block(child_blocks[i]));
                    }
                    i = kids.len();
                    while i > 0 {
                        i -= 1;
                        tasks.push(Task::Expr(kids[i]));
                    }
                }
                Task::Done(id) => {
                    let expression = &self.source.exprs[id];
                    let value = match &expression.kind {
                        Expr::Path(p) => {
                            let mut t = 0;
                            let mut i = bindings.len();
                            while i > 0 {
                                i -= 1;
                                if bindings[i].0 == p.name {
                                    t = bindings[i].1;
                                    break;
                                }
                            }
                            t
                        }
                        Expr::Int(text) => {
                            let name = if text.ends_with("i64") {
                                "i64"
                            } else if text.ends_with("u64") {
                                "u64"
                            } else if text.ends_with("usize") {
                                "usize"
                            } else {
                                "i32"
                            };
                            self.term(Term::Named(name.to_string(), Vec::new()))
                        }
                        Expr::Bool(_) => self.term(Term::Named("bool".to_string(), Vec::new())),
                        Expr::Float(_) => self.term(Term::Named("f64".to_string(), Vec::new())),
                        Expr::Borrow { mutable, value } => self.term(Term::Named(
                            if *mutable { "&mut" } else { "&" }.to_string(),
                            vec![exprs[value]],
                        )),
                        Expr::Cast { ty, .. } => type_term(self.source, *ty, &env, &mut self.terms),
                        Expr::Unary { op, value } => {
                            let t = exprs[value];
                            if op == "*" {
                                match &self.terms[t] {
                                    Term::Named(name, kids)
                                        if (name == "&" || name == "&mut" || name == "Box")
                                            && !kids.is_empty() =>
                                    {
                                        kids[0]
                                    }
                                    _ => 0,
                                }
                            } else {
                                t
                            }
                        }
                        Expr::Tuple(items) => {
                            let mut ts = Vec::new();
                            for e in items {
                                ts.push(exprs[e]);
                            }
                            self.term(Term::Named("tuple".to_string(), ts))
                        }
                        Expr::Array(items) => {
                            let first = if items.is_empty() {
                                0
                            } else {
                                exprs[&items[0]]
                            };
                            self.term(Term::Named(format!("[;{}]", items.len()), vec![first]))
                        }
                        Expr::Repeat { value, .. } => {
                            self.term(Term::Named("[]".to_string(), vec![exprs[value]]))
                        }
                        Expr::Macro { name, args, .. } if name == "vec" => {
                            let first = if args.is_empty() { 0 } else { exprs[&args[0]] };
                            self.term(Term::Named("Vec".to_string(), vec![first]))
                        }
                        Expr::Call { callee, args } => {
                            if let Expr::Path(path) = &self.source.exprs[*callee].kind {
                                let mut ts = Vec::new();
                                for e in args {
                                    ts.push(exprs[e]);
                                }
                                if let Some(target) = self.lookup(&function.module, &path.name) {
                                    self.call(caller, target, &path.args, &ts, expression.span)
                                } else if path.name == "Box::new" && ts.len() == 1 {
                                    self.term(Term::Named("Box".to_string(), ts))
                                } else if path.name == "Vec::new"
                                    || path.name == "Vec::with_capacity"
                                {
                                    let inner = if path.args.is_empty() {
                                        0
                                    } else {
                                        type_term(self.source, path.args[0], &env, &mut self.terms)
                                    };
                                    self.term(Term::Named("Vec".to_string(), vec![inner]))
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        }
                        Expr::Method {
                            receiver,
                            name,
                            args,
                            types,
                        } => {
                            let receiver_type = exprs[receiver];
                            if name == "clone" && args.is_empty() {
                                receiver_type
                            } else if let Some(target) = self.lookup(&function.module, name) {
                                let mut ts = vec![receiver_type];
                                for e in args {
                                    ts.push(exprs[e]);
                                }
                                self.call(caller, target, types, &ts, expression.span)
                            } else {
                                0
                            }
                        }
                        Expr::Index { value, .. } => match &self.terms[exprs[value]] {
                            Term::Named(name, kids)
                                if (name == "Vec" || name == "[]" || name.starts_with("[;"))
                                    && !kids.is_empty() =>
                            {
                                kids[0]
                            }
                            _ => 0,
                        },
                        Expr::Block(b) => *blocks.get(b).unwrap_or(&0),
                        Expr::If { then_block, .. } | Expr::IfLet { then_block, .. } => {
                            *blocks.get(then_block).unwrap_or(&0)
                        }
                        Expr::Binary { left, .. } => exprs[left],
                        Expr::Return(Some(e)) | Expr::Break(Some(e)) => exprs[e],
                        Expr::Struct { path, .. } => {
                            let mut ts = Vec::new();
                            for t in &path.args {
                                ts.push(type_term(self.source, *t, &env, &mut self.terms));
                            }
                            self.term(Term::Named(short(&path.name).to_string(), ts))
                        }
                        _ => 0,
                    };
                    exprs.insert(id, value);
                }
            }
        }
    }
}
fn finite(source: &Program) -> Result<(), Diagnostic> {
    let mut analysis = Analysis {
        source,
        terms: vec![Term::Unknown],
        generics: Vec::new(),
        nodes: Vec::new(),
        names: BTreeMap::new(),
        edges: Vec::new(),
        count: 0,
    };
    let mut duplicate = BTreeMap::new();
    for i in 0..source.functions.len() {
        let f = &source.functions[i];
        let env = variables(
            source,
            &f.generics,
            &mut analysis.terms,
            &mut analysis.count,
        );
        let mut nodes = Vec::new();
        for t in env.values() {
            if let Term::Variable(v) = analysis.terms[*t] {
                nodes.push(v);
            }
        }
        analysis.generics.push(env);
        analysis.nodes.push(nodes);
        let name = qualified(&f.module, &f.name);
        if analysis.names.insert(name.clone(), i).is_some() {
            duplicate.insert(name, ());
        }
    }
    for name in duplicate.keys() {
        analysis.names.remove(name);
    }
    for i in 0..source.functions.len() {
        if !analysis.nodes[i].is_empty() {
            analysis.function(i);
        }
    }
    let groups = components(analysis.count, &analysis.edges);
    for edge in &analysis.edges {
        if groups[edge.from] == groups[edge.to] && edge.expanding {
            return Err(Diagnostic::new(
                edge.span,
                "E_RECURSION",
                "generic parameter flow expands a type inside a recursive component",
            ));
        }
    }
    for edge in &analysis.edges {
        if groups[edge.from] == groups[edge.to] && edge.unknown {
            return Err(Diagnostic::new(
                edge.span,
                "E_GENERIC_ARGS",
                "recursive generic call needs explicit type arguments to determine finite instantiation",
            ));
        }
    }
    Ok(())
}
pub fn validate(source: &Program) -> Result<(), Diagnostic> {
    coherence(source)?;
    finite(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn validate_text(text: &str) -> Result<(), Diagnostic> {
        validate(&crate::syntax::parse(text).unwrap())
    }
    #[test]
    fn unused_expanding_recursion() {
        let e =
            validate_text("fn f<T>(x:T){ f::<Vec<T>>(vec![x]); } fn probe()->i64{42}").unwrap_err();
        assert_eq!(e.code, "E_RECURSION");
    }
    #[test]
    fn mutual_expansion() {
        let e = validate_text("fn f<T>(x:T){g::<Vec<T>>(vec![x]);}fn g<U>(x:U){f::<U>(x);}")
            .unwrap_err();
        assert_eq!(e.code, "E_RECURSION");
    }
    #[test]
    fn inferred_expansion() {
        let e = validate_text("fn f<T>(x:T){f(vec![x]);}").unwrap_err();
        assert_eq!(e.code, "E_RECURSION");
    }
    #[test]
    fn nonexpanding_recursion() {
        validate_text("fn f<T>(x:T){g::<T>(x);}fn g<U>(x:U){f::<U>(x);}").unwrap();
    }
    #[test]
    fn expansion_outside_component() {
        validate_text("fn f<T>(x:T){g::<Vec<T>>(vec![x]);}fn g<U>(x:U){}").unwrap();
    }
    #[test]
    fn reset_breaks_type_cycle() {
        validate_text("fn f<T>(x:T){g::<Vec<T>>(vec![x]);}fn g<U>(x:U){f::<i64>(1i64);}").unwrap();
    }
    #[test]
    fn generic_local_flow() {
        assert_eq!(
            validate_text("fn f<T>(x:T){let y: Vec<T> = vec![x];f(y);}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    #[test]
    fn blanket_overlap() {
        assert_eq!(
            validate_text("trait A {} struct S<T>{x:T} impl<T> A for S<T>{} impl A for S<i64>{}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    #[test]
    fn disjoint_impls() {
        validate_text("trait A {} impl A for i64{} impl A for bool{}").unwrap();
    }
    #[test]
    fn occurs_check_excludes_infinite_overlap() {
        validate_text("trait A<T> {} impl<T> A<T> for T {} impl<U> A<Vec<U>> for U {}").unwrap();
    }
    #[test]
    fn required_method_and_type() {
        assert_eq!(
            validate_text("trait A {fn f(&self);type Item;}impl A for i64{}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
}

// Trait selection uses concrete K types. Prerequisites form a work graph rather
// than a recursive search with a depth limit. Structural traits are coinductive;
// a cycle made solely of user implementation prerequisites proves nothing.
fn impl_arguments(
    source: &Program,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    implementation: &Impl,
    actual: usize,
) -> Option<BTreeMap<String, usize>> {
    use crate::k::TypeKind;
    let mut env = BTreeMap::new();
    let mut work = vec![(implementation.target, actual)];
    let mut aliases = BTreeMap::new();
    while let Some((pattern, actual)) = work.pop() {
        let actual_type = types.get(actual)?;
        match &source.types[pattern].kind {
            Type::Path(path) => {
                let name = short(&path.name);
                let mut generic = false;
                for param in &implementation.generics {
                    if !param.lifetime && param.name == path.name && path.args.is_empty() {
                        generic = true;
                    }
                }
                if generic {
                    if let Some(previous) = env.insert(path.name.clone(), actual) {
                        if previous != actual {
                            return None;
                        }
                    }
                    continue;
                }
                let mut args = Vec::new();
                for arg in &path.args {
                    let lifetime = match &source.types[*arg].kind {
                        Type::Path(p) => p.name.starts_with('\''),
                        _ => false,
                    };
                    if !lifetime {
                        args.push(*arg);
                    }
                }
                let mut expanded = false;
                for alias in &source.aliases {
                    if alias.name == name && alias.generics.is_empty() && args.is_empty() {
                        if aliases.insert((pattern, actual), ()).is_some() {
                            return None;
                        }
                        work.push((alias.ty, actual));
                        expanded = true;
                        break;
                    }
                }
                if expanded {
                    continue;
                }
                if let Some((concrete_name, concrete_args)) = applications.get(&actual) {
                    if short(concrete_name) != name || args.len() != concrete_args.len() {
                        return None;
                    }
                    for i in 0..args.len() {
                        work.push((args[i], concrete_args[i]));
                    }
                } else {
                    match (&actual_type.kind, name) {
                        (TypeKind::Vec { inner }, "Vec") | (TypeKind::Box { inner }, "Box") => {
                            if args.len() != 1 {
                                return None;
                            }
                            work.push((args[0], *inner));
                        }
                        (TypeKind::Unit, "()") => {
                            if !args.is_empty() {
                                return None;
                            }
                        }
                        (
                            TypeKind::Int {
                                bits: 64,
                                signed: false,
                            },
                            "u64" | "usize",
                        ) => {
                            if !args.is_empty() {
                                return None;
                            }
                        }
                        _ => {
                            if !args.is_empty() || short(&actual_type.name) != name {
                                return None;
                            }
                        }
                    }
                }
            }
            Type::Ref { mutable, inner, .. } => {
                if let Type::Slice(element) = source.types[*inner].kind {
                    if let TypeKind::Slice {
                        mutable: actual_mutable,
                        inner: actual_inner,
                    } = actual_type.kind
                    {
                        if *mutable != actual_mutable {
                            return None;
                        }
                        work.push((element, actual_inner));
                    } else {
                        return None;
                    }
                } else if let TypeKind::Ref {
                    mutable: actual_mutable,
                    inner: actual_inner,
                } = actual_type.kind
                {
                    if *mutable != actual_mutable {
                        return None;
                    }
                    work.push((*inner, actual_inner));
                } else {
                    return None;
                }
            }
            Type::Tuple(parts) => match &actual_type.kind {
                TypeKind::Unit if parts.is_empty() => {}
                TypeKind::Struct { fields, .. } if parts.len() == fields.len() => {
                    for i in 0..parts.len() {
                        if fields[i].name != i.to_string() {
                            return None;
                        }
                        work.push((parts[i], fields[i].ty));
                    }
                }
                _ => return None,
            },
            _ => return None,
        }
    }
    Some(env)
}

#[derive(Clone)]
struct Goal {
    ty: usize,
    bound: String,
}
struct Requirements {
    children: Vec<Goal>,
    structural: bool,
}

fn derived(
    source: &Program,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    ty: usize,
    bound: &str,
) -> bool {
    let name = match applications.get(&ty) {
        Some((name, _)) => short(name),
        None => short(&types[ty].name),
    };
    for s in &source.structs {
        if s.name == name {
            for attr in &s.attrs {
                if attr.name == "derive" {
                    for arg in &attr.args {
                        if arg == bound {
                            return true;
                        }
                    }
                }
            }
        }
    }
    for e in &source.enums {
        if e.name == name {
            for attr in &e.attrs {
                if attr.name == "derive" {
                    for arg in &attr.args {
                        if arg == bound {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
fn add_bound(source: &Program, id: TypeId, ty: usize, children: &mut Vec<Goal>) -> bool {
    if let Type::Path(path) = &source.types[id].kind {
        if path.name.starts_with('\'') {
            return true;
        }
        if !path.args.is_empty() {
            return false;
        }
        children.push(Goal {
            ty,
            bound: short(&path.name).to_string(),
        });
        true
    } else {
        false
    }
}
fn trait_requirements(
    source: &Program,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    goal: &Goal,
) -> Option<Requirements> {
    use crate::k::TypeKind;
    let ty = types.get(goal.ty)?;
    let bound = goal.bound.as_str();
    if bound == "Copy" {
        return if crate::check::is_copy(types, goal.ty) {
            Some(Requirements {
                children: Vec::new(),
                structural: true,
            })
        } else {
            None
        };
    }
    if bound == "Data" {
        return if crate::check::is_data(types, goal.ty) {
            Some(Requirements {
                children: Vec::new(),
                structural: true,
            })
        } else {
            None
        };
    }
    if bound == "Clone" && crate::check::is_copy(types, goal.ty) {
        return Some(Requirements {
            children: Vec::new(),
            structural: true,
        });
    }
    let automatic = bound == "Send" || bound == "Sync";
    let comparison =
        bound == "Ord" || bound == "Eq" || bound == "PartialOrd" || bound == "PartialEq";
    let clone = bound == "Clone";
    if automatic || comparison || clone {
        let mut children = Vec::new();
        let mut structural = true;
        match &ty.kind {
            TypeKind::Unit | TypeKind::Bool | TypeKind::Int { .. } | TypeKind::String => {}
            TypeKind::F64 => {
                if bound == "Ord" || bound == "Eq" {
                    return None;
                }
            }
            TypeKind::World => {
                return None;
            }
            TypeKind::Vec { inner } | TypeKind::Box { inner } => children.push(Goal {
                ty: *inner,
                bound: bound.to_string(),
            }),
            TypeKind::Ref { inner, mutable } | TypeKind::Slice { inner, mutable } => {
                if clone {
                    if *mutable {
                        return None;
                    }
                } else {
                    let child_bound = if bound == "Send" && !*mutable {
                        "Sync"
                    } else {
                        bound
                    };
                    children.push(Goal {
                        ty: *inner,
                        bound: child_bound.to_string(),
                    });
                }
            }
            TypeKind::Struct { fields, .. } => {
                let tuple = ty.name.starts_with('(')
                    && ty.name.ends_with(')')
                    && fields
                        .iter()
                        .enumerate()
                        .all(|(i, f)| f.name == i.to_string());
                if automatic || tuple || derived(source, types, applications, goal.ty, bound) {
                    for field in fields {
                        children.push(Goal {
                            ty: field.ty,
                            bound: bound.to_string(),
                        });
                    }
                } else {
                    structural = false;
                }
            }
            TypeKind::Enum { variants, .. } => {
                if automatic || derived(source, types, applications, goal.ty, bound) {
                    for variant in variants {
                        for field in &variant.fields {
                            children.push(Goal {
                                ty: field.ty,
                                bound: bound.to_string(),
                            });
                        }
                    }
                } else {
                    structural = false;
                }
            }
        }
        if structural {
            return Some(Requirements {
                children,
                structural: true,
            });
        }
    }
    let mut selected = None;
    for implementation in &source.impls {
        let trait_ty = match implementation.trait_ty {
            Some(t) => t,
            None => continue,
        };
        let trait_name = match &source.types[trait_ty].kind {
            Type::Path(path) if path.args.is_empty() => short(&path.name),
            _ => continue,
        };
        if trait_name != bound {
            continue;
        }
        if let Some(env) = impl_arguments(source, types, applications, implementation, goal.ty) {
            if selected.is_some() {
                return None;
            }
            selected = Some((implementation, env));
        }
    }
    let (implementation, env) = selected?;
    let mut children = Vec::new();
    for param in &implementation.generics {
        if param.lifetime {
            continue;
        }
        let concrete = *env.get(&param.name)?;
        for constraint in &param.bounds {
            if !add_bound(source, *constraint, concrete, &mut children) {
                return None;
            }
        }
    }
    for declaration in &source.traits {
        if declaration.name == bound {
            for constraint in &declaration.bounds {
                if !add_bound(source, *constraint, goal.ty, &mut children) {
                    return None;
                }
            }
        }
    }
    Some(Requirements {
        children,
        structural: false,
    })
}

/// Select a bare trait bound for a concrete type, including every prerequisite.
/// Parameterized bounds are deliberately rejected until the caller supplies
/// their explicit witness/type arguments. Unknown or cyclic user bounds fail.
pub fn satisfies(
    source: &Program,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    ty: usize,
    bound: &str,
) -> bool {
    if bound.contains('<') || bound.contains('(') {
        return false;
    }
    let first = Goal {
        ty,
        bound: short(bound).to_string(),
    };
    let mut state: BTreeMap<(usize, String), u8> = BTreeMap::new();
    let mut structural: BTreeMap<(usize, String), bool> = BTreeMap::new();
    let mut tasks = vec![(first.clone(), false, Vec::<Goal>::new())];
    while let Some((goal, finish, children)) = tasks.pop() {
        let key = (goal.ty, goal.bound.clone());
        if finish {
            if state.get(&key) == Some(&1) {
                continue;
            }
            let mut valid = true;
            for child in children {
                let ck = (child.ty, child.bound);
                match state.get(&ck) {
                    Some(2) => {}
                    Some(0) if structural.get(&ck) == Some(&true) => {}
                    _ => {
                        valid = false;
                        break;
                    }
                }
            }
            state.insert(key, if valid { 2 } else { 1 });
            continue;
        }
        if let Some(previous) = state.get(&key) {
            if *previous == 0 && structural.get(&key) != Some(&true) {
                state.insert(key, 1);
            }
            continue;
        }
        match trait_requirements(source, types, applications, &goal) {
            None => {
                state.insert(key, 1);
            }
            Some(requirements) => {
                state.insert(key.clone(), 0);
                structural.insert(key, requirements.structural);
                tasks.push((goal, true, requirements.children.clone()));
                let mut i = requirements.children.len();
                while i > 0 {
                    i -= 1;
                    tasks.push((requirements.children[i].clone(), false, Vec::new()));
                }
            }
        }
    }
    state.get(&(first.ty, first.bound)) == Some(&2)
}

#[cfg(test)]
mod bound_tests {
    use super::*;
    use crate::k::{self, TypeKind};
    #[test]
    fn missing_unused_bound_is_false() {
        let source = crate::syntax::parse(
            "trait Score {fn score(&self)->i64;} fn ignored<T:Score>(x:T)->i64{0}",
        )
        .unwrap();
        assert!(!satisfies(
            &source,
            &k::primitives(),
            &BTreeMap::new(),
            k::I64,
            "Score"
        ));
    }
    #[test]
    fn prerequisite_is_checked() {
        let source = crate::syntax::parse(
            "trait A {} trait B {} struct S<T>{x:T} impl A for i64 {} impl<T:A> B for S<T> {}",
        )
        .unwrap();
        let mut types = k::primitives();
        let si = types.len();
        types.push(k::Type {
            name: "S<i64>".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![k::Field {
                    name: "x".to_string(),
                    ty: k::I64,
                }],
            },
        });
        let sb = types.len();
        types.push(k::Type {
            name: "S<bool>".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![k::Field {
                    name: "x".to_string(),
                    ty: k::BOOL,
                }],
            },
        });
        let mut apps = BTreeMap::new();
        apps.insert(si, ("S".to_string(), vec![k::I64]));
        apps.insert(sb, ("S".to_string(), vec![k::BOOL]));
        assert!(satisfies(&source, &types, &apps, si, "B"));
        assert!(!satisfies(&source, &types, &apps, sb, "B"));
    }
    #[test]
    fn user_prerequisite_cycle_is_false() {
        let source =
            crate::syntax::parse("trait A {} trait B {} impl<T:B> A for T {} impl<T:A> B for T {}")
                .unwrap();
        assert!(!satisfies(
            &source,
            &k::primitives(),
            &BTreeMap::new(),
            k::I64,
            "A"
        ));
    }
    #[test]
    fn recursive_structural_traits_have_no_depth_limit() {
        let source = crate::syntax::parse("struct Link {next:Box<Link>}").unwrap();
        let mut types = k::primitives();
        let link = types.len();
        let bx = link + 1;
        types.push(k::Type {
            name: "Link".to_string(),
            kind: TypeKind::Struct {
                copy: false,
                fields: vec![k::Field {
                    name: "next".to_string(),
                    ty: bx,
                }],
            },
        });
        types.push(k::Type {
            name: "Box<Link>".to_string(),
            kind: TypeKind::Box { inner: link },
        });
        assert!(satisfies(&source, &types, &BTreeMap::new(), link, "Send"));
        assert!(satisfies(&source, &types, &BTreeMap::new(), link, "Sync"));
        assert!(satisfies(&source, &types, &BTreeMap::new(), link, "Data"));
        assert!(!satisfies(&source, &types, &BTreeMap::new(), link, "Copy"));
    }
    #[test]
    fn ord_and_clone_on_collections() {
        let source = Program::default();
        let mut types = k::primitives();
        let vi = types.len();
        types.push(k::Type {
            name: "Vec<i64>".to_string(),
            kind: TypeKind::Vec { inner: k::I64 },
        });
        let vf = types.len();
        types.push(k::Type {
            name: "Vec<f64>".to_string(),
            kind: TypeKind::Vec { inner: k::F64 },
        });
        assert!(satisfies(&source, &types, &BTreeMap::new(), vi, "Ord"));
        assert!(!satisfies(&source, &types, &BTreeMap::new(), vf, "Ord"));
        assert!(satisfies(&source, &types, &BTreeMap::new(), vf, "Clone"));
    }
}
