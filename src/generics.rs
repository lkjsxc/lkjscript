//! Global source rules for coherence and finite generic instantiation.
//! The executable K checker still checks concrete function and witness types.
mod bound_solver;
mod signature_bounds;
use crate::ast::*;
use crate::names::qualified;
pub(crate) use bound_solver::{BoundContext, BoundSolver, BoundStep};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Term {
    Unknown,
    Variable(usize),
    Named(String, Vec<usize>),
    DeclaredTrait(usize, Vec<usize>),
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
                if p.declaration.is_none() && p.args.is_empty() {
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
            Type::Path(p) => {
                if p.name.starts_with('\'') {
                    Term::Named("lifetime".to_string(), values)
                } else if let TraitIdentity::Declared(declaration) = bound_identity(source, p) {
                    Term::DeclaredTrait(declaration, values)
                } else {
                    Term::Named(p.name.clone(), values)
                }
            }
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
            Term::Named(_, kids) | Term::DeclaredTrait(_, kids) => {
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
            (Term::DeclaredTrait(a, ak), Term::DeclaredTrait(b, bk))
                if a == b && ak.len() == bk.len() =>
            {
                for (a, b) in ak.iter().zip(bk) {
                    stack.push((*a, *b));
                }
            }
            (Term::Unknown, _) | (_, Term::Unknown) => {}
            _ => return false,
        }
    }
    true
}
#[derive(Default)]
struct Candidates {
    heads: BTreeMap<String, Vec<usize>>,
    generic: Vec<usize>,
    all: Vec<usize>,
}
impl Candidates {
    fn insert(&mut self, head: Option<String>, value: usize) {
        self.all.push(value);
        if let Some(head) = head {
            self.heads.entry(head).or_default().push(value);
        } else {
            self.generic.push(value);
        }
    }
    fn matching(&self, head: Option<&str>) -> Vec<usize> {
        let Some(head) = head else {
            return self.all.clone();
        };
        let mut values = self.generic.clone();
        if let Some(specific) = self.heads.get(head) {
            values.extend_from_slice(specific);
        }
        values
    }
}
fn term_head(terms: &[Term], term: usize) -> Option<String> {
    match &terms[term] {
        Term::Named(name, _) => Some(name.clone()),
        Term::DeclaredTrait(id, _) => Some(format!("trait#{id}")),
        Term::Variable(_) | Term::Unknown => None,
    }
}

fn coherence(source: &Program) -> Result<(), Diagnostic> {
    let mut by_trait: BTreeMap<TraitIdentity, Candidates> = BTreeMap::new();
    let mut terms = Vec::new();
    let mut next = 0;
    let mut normalized = Vec::<(usize, usize)>::new();
    for i in 0..source.impls.len() {
        let implementation = &source.impls[i];
        let trait_ty = match implementation.trait_ty {
            Some(ty) => ty,
            None => continue,
        };
        let identity = match &source.types[trait_ty].kind {
            Type::Path(path) => bound_identity(source, path),
            _ => continue,
        };
        let environment = variables(source, &implementation.generics, &mut terms, &mut next);
        let a = signature_type(source, implementation.target, &environment, &mut terms)?;
        let at = signature_type(source, trait_ty, &environment, &mut terms)?;
        let head = term_head(&terms, a);
        if let Some(previous) = by_trait.get(&identity) {
            for j in previous.matching(head.as_deref()) {
                let (b, bt) = normalized[j];
                if unify(&terms, vec![(a, b), (at, bt)]) {
                    return Err(Diagnostic::new(
                        implementation.span,
                        "E_TRAIT",
                        "overlapping trait implementations",
                    ));
                }
            }
        }
        let index = normalized.len();
        normalized.push((a, at));
        by_trait
            .entry(identity.clone())
            .or_default()
            .insert(head, index);
        if let TraitIdentity::Declared(t) = identity {
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
            Term::Named(_, kids) | Term::DeclaredTrait(_, kids) => {
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
            Term::Named(_, kids) | Term::DeclaredTrait(_, kids) => {
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
            if path.declaration.is_none() && path.args.is_empty() {
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
            Type::Path(path) => path.name.as_str().to_string(),
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
    resolution: crate::resolve::Resolver<'a>,
    terms: Vec<Term>,
    generics: Vec<BTreeMap<String, usize>>,
    nodes: Vec<Vec<usize>>,
    edges: Vec<Edge>,
    count: usize,
}
impl Analysis<'_> {
    fn term(&mut self, kind: Term) -> usize {
        push(&mut self.terms, kind)
    }
    fn lookup(&self, module: &str, path: &Path, span: Span) -> Option<FunctionId> {
        let path = self
            .resolution
            .value_path(self.resolution.scope(module), path, span)
            .ok()?;
        match self.source.declarations.get(path.declaration?)?.item {
            Item::Function(index) => Some(FunctionId { index }),
            _ => None,
        }
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
            let concrete = match substitutions.get(&g.name).copied() {
                Some(t) => normalize(self.source, t, &mut self.terms).ok(),
                None => None,
            };
            match concrete {
                Some(t) if !unknown(&self.terms, t) => {
                    substitutions.insert(g.name.clone(), t);
                    flow(&self.terms, t, target, span, &mut self.edges)
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
                            Stmt::Expr { value, .. } => tasks.push(Task::Expr(*value)),
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
                        | Expr::Try(value)
                        | Expr::Diverge(value) => kids.push(*value),
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
                        Expr::Try(value) => match &self.terms[exprs[value]] {
                            Term::Named(name, children)
                                if (name == "$std::prelude::Result"
                                    || name == "$std::prelude::Option")
                                    && !children.is_empty() =>
                            {
                                children[0]
                            }
                            _ => 0,
                        },
                        Expr::Diverge(_) => self.term(Term::Named("!".to_string(), Vec::new())),
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
                                if let Some(target) =
                                    self.lookup(&function.module, path, expression.span)
                                {
                                    self.call(
                                        caller,
                                        target.index,
                                        &path.args,
                                        &ts,
                                        expression.span,
                                    )
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
                            ..
                        } => {
                            if name == "clone" && args.is_empty() {
                                exprs[receiver]
                            } else {
                                // Method syntax never denotes a same-spelled free
                                // function. Its selected witness belongs to typing.
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
                            self.term(Term::Named(path.name.as_str().to_string(), ts))
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
        resolution: crate::resolve::Resolver::new(source)?,
        terms: vec![Term::Unknown],
        generics: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        count: 0,
    };
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
    if !source.resolved {
        // Direct callers use the same recorded identities as compilation.
        return validate(&crate::resolve::resolve(source)?);
    }
    declarations(source)?;
    alias_cycles(source)?;
    constant_cycles(source)?;
    finite_data(source)?;
    coherence(source)?;
    signatures(source)?;
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
        validate_text("type Identity<T> = T; fn f<T>(x:T){f::<Identity<T>>(x);}").unwrap();
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
                let name = path.name.as_str();
                let mut generic = false;
                for param in &implementation.generics {
                    if path.declaration.is_none()
                        && !param.lifetime
                        && param.name == path.name
                        && path.args.is_empty()
                    {
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
                    if qualified(&alias.module, &alias.name) == name
                        && alias.generics.is_empty()
                        && args.is_empty()
                    {
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
                    if concrete_name.as_str() != name || args.len() != concrete_args.len() {
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
                        _ => {
                            if !args.is_empty() || actual_type.name.as_str() != name {
                                return None;
                            }
                        }
                    }
                }
            }
            Type::Ref { mutable, inner, .. } => {
                if let TypeKind::Ref {
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
            Type::Slice(inner) => {
                if let TypeKind::Slice {
                    inner: actual_inner,
                    ..
                } = actual_type.kind
                {
                    work.push((*inner, actual_inner));
                } else {
                    return None;
                }
            }
            Type::Array { inner, len } => {
                let extent = len
                    .replace('_', "")
                    .trim_end_matches("usize")
                    .parse::<usize>()
                    .ok()?;
                if let TypeKind::Array {
                    inner: actual_inner,
                    len: actual_len,
                } = actual_type.kind
                {
                    if extent != actual_len {
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

/// Builtin predicates and source trait declarations have distinct identities,
/// even when a declaration uses a builtin's short spelling.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TraitIdentity {
    Builtin(String),
    Declared(usize),
    Unknown(String),
}
/// Builtin supertrait contracts shared by assumptions and derive validation.
pub(crate) fn builtin_parents(name: &str) -> &'static [&'static str] {
    match name {
        "Copy" => &["Clone"],
        "Ord" => &["Eq", "PartialOrd", "PartialEq"],
        "Eq" | "PartialOrd" => &["PartialEq"],
        _ => &[],
    }
}

pub(crate) fn bound_identity(source: &Program, path: &Path) -> TraitIdentity {
    if let Some(declaration) = path.declaration {
        return match source.declarations.get(declaration).map(|item| item.item) {
            Some(Item::Trait(index)) => TraitIdentity::Declared(index),
            _ => TraitIdentity::Unknown(path.name.clone()),
        };
    }
    if !source.resolved {
        if let Some(index) = source
            .traits
            .iter()
            .position(|item| qualified(&item.module, &item.name) == path.name)
        {
            return TraitIdentity::Declared(index);
        }
    }
    if matches!(
        path.name.as_str(),
        "Copy"
            | "Clone"
            | "Data"
            | "Send"
            | "Sync"
            | "Ord"
            | "Eq"
            | "PartialEq"
            | "PartialOrd"
            | "Drop"
    ) {
        TraitIdentity::Builtin(path.name.clone())
    } else {
        TraitIdentity::Unknown(path.name.clone())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Goal {
    pub(crate) ty: usize,
    pub(crate) bound: TraitIdentity,
}
struct Requirements {
    children: Vec<Goal>,
    structural: bool,
    witness: Option<ImplementationWitness>,
    infer: bool,
    parameterized: Vec<(TypeId, usize)>,
}
impl Requirements {
    fn structural(children: Vec<Goal>) -> Self {
        Self {
            children,
            structural: true,
            witness: None,
            infer: false,
            parameterized: Vec::new(),
        }
    }
}

/// A derived implementation covers its nominal constructor. Reuse the same
/// normalized terms as ordinary coherence, without looking up a short spelling.
pub(crate) fn derived_coherence(
    source: &Program,
    declaration: DeclarationId,
    bound: &str,
) -> Result<(), Diagnostic> {
    let owner = &source.declarations[declaration];
    for implementation in &source.impls {
        let Some(trait_ty) = implementation.trait_ty else {
            continue;
        };
        let Type::Path(path) = &source.types[trait_ty].kind else {
            continue;
        };
        if bound_identity(source, path) != TraitIdentity::Builtin(bound.to_string()) {
            continue;
        }
        let mut terms = Vec::new();
        let mut next = 0;
        let environment = variables(source, &implementation.generics, &mut terms, &mut next);
        let target = signature_type(source, implementation.target, &environment, &mut terms)?;
        let overlaps = match &terms[target] {
            Term::Named(name, _) => *name == owner.name,
            Term::Variable(_) | Term::Unknown => true,
            Term::DeclaredTrait(_, _) => false,
        };
        if overlaps {
            return Err(Diagnostic::new(
                implementation.span,
                "E_TRAIT",
                &format!(
                    "explicit {bound} implementation overlaps derive for {}",
                    owner.name
                ),
            ));
        }
    }
    Ok(())
}

fn derived(
    source: &Program,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    ty: usize,
    bound: &str,
) -> bool {
    let name = match applications.get(&ty) {
        Some((name, _)) => name.as_str(),
        None => types[ty].name.as_str(),
    };
    for s in &source.structs {
        if qualified(&s.module, &s.name) == name {
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
        if qualified(&e.module, &e.name) == name {
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
            bound: bound_identity(source, path),
        });
        true
    } else {
        false
    }
}
#[derive(Clone)]
pub(crate) struct ImplementationWitness {
    pub(crate) implementation: usize,
    pub(crate) substitutions: BTreeMap<String, usize>,
}
/// Candidate filtering never proves a bound: the original structural matcher
/// and prerequisite solver still validate each selected implementation.
/// A generic or non-nominal target stays in the fallback set.
pub(crate) struct ImplementationIndex {
    by_trait: BTreeMap<TraitIdentity, Candidates>,
    by_method: BTreeMap<String, Candidates>,
}
impl ImplementationIndex {
    pub(crate) fn new(source: &Program) -> Result<Self, Diagnostic> {
        let mut index = Self {
            by_trait: BTreeMap::new(),
            by_method: BTreeMap::new(),
        };
        for (id, implementation) in source.impls.iter().enumerate() {
            let mut terms = Vec::new();
            let mut next = 0;
            let environment = variables(source, &implementation.generics, &mut terms, &mut next);
            let target = signature_type(source, implementation.target, &environment, &mut terms)?;
            let head = match &terms[target] {
                Term::Named(name, _)
                    if !matches!(name.as_str(), "&" | "&mut" | "[]" | "tuple" | "fn" | "!")
                        && !name.starts_with("[;") =>
                {
                    Some(name.clone())
                }
                _ => None,
            };
            if let Some(trait_type) = implementation.trait_ty {
                if let Type::Path(path) = &source.types[trait_type].kind {
                    index
                        .by_trait
                        .entry(bound_identity(source, path))
                        .or_default()
                        .insert(head.clone(), id);
                }
            }
            let mut methods = BTreeSet::new();
            for method in &implementation.methods {
                let name = &source.functions[*method].name;
                if methods.insert(name) {
                    index
                        .by_method
                        .entry(name.clone())
                        .or_default()
                        .insert(head.clone(), id);
                }
            }
        }
        Ok(index)
    }
    fn receiver_head<'a>(
        types: &'a [crate::k::Type],
        applications: &'a BTreeMap<usize, (String, Vec<usize>)>,
        ty: usize,
    ) -> Option<&'a str> {
        if let Some((name, _)) = applications.get(&ty) {
            Some(name.as_str())
        } else {
            match &types[ty].kind {
                crate::k::TypeKind::Vec { .. } => Some("Vec"),
                crate::k::TypeKind::Box { .. } => Some("Box"),
                _ if !types[ty].name.is_empty() => Some(types[ty].name.as_str()),
                _ => None,
            }
        }
    }
    pub(crate) fn candidates(
        &self,
        types: &[crate::k::Type],
        applications: &BTreeMap<usize, (String, Vec<usize>)>,
        ty: usize,
        bound: &TraitIdentity,
    ) -> Vec<usize> {
        self.by_trait
            .get(bound)
            .map_or_else(Vec::new, |candidates| {
                candidates.matching(Self::receiver_head(types, applications, ty))
            })
    }
    /// A method name and nominal receiver exclude unrelated implementations;
    /// generic and structural targets still require the complete matcher.
    pub(crate) fn method_candidates(
        &self,
        types: &[crate::k::Type],
        applications: &BTreeMap<usize, (String, Vec<usize>)>,
        ty: usize,
        name: &str,
    ) -> Vec<usize> {
        let mut values = self
            .by_method
            .get(name)
            .map_or_else(Vec::new, |candidates| {
                candidates.matching(Self::receiver_head(types, applications, ty))
            });
        // Preserve source-order diagnostics when fallback and nominal entries mix.
        values.sort_unstable();
        values
    }
}

pub(crate) fn select_implementation(
    source: &Program,
    index: &ImplementationIndex,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    ty: usize,
    bound: &TraitIdentity,
) -> Option<ImplementationWitness> {
    let mut selected = None;
    for candidate in index.candidates(types, applications, ty, bound) {
        let implementation = &source.impls[candidate];
        let trait_type = match implementation.trait_ty {
            Some(ty) => ty,
            None => continue,
        };
        let path = match &source.types[trait_type].kind {
            Type::Path(path) if path.args.is_empty() => path,
            _ => continue,
        };
        if bound_identity(source, path) != *bound {
            continue;
        }
        if let Some(substitutions) = impl_arguments(source, types, applications, implementation, ty)
        {
            if selected.is_some() {
                return None;
            }
            selected = Some(ImplementationWitness {
                implementation: candidate,
                substitutions,
            });
        }
    }
    selected
}

fn trait_requirements(
    source: &Program,
    index: &ImplementationIndex,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    goal: &Goal,
    rigid: &BTreeMap<usize, BTreeSet<TraitIdentity>>,
    inferred: &BTreeMap<Goal, ImplementationWitness>,
) -> Option<Requirements> {
    use crate::k::TypeKind;
    let ty = types.get(goal.ty)?;
    if let Some(assumptions) = rigid.get(&goal.ty) {
        if assumptions.contains(&goal.bound) {
            return Some(Requirements::structural(Vec::new()));
        }
        // An opaque parameter has no inferred builtin properties. User blanket
        // implementations can still prove declared traits through prerequisites.
        if matches!(goal.bound, TraitIdentity::Builtin(_)) {
            return None;
        }
    }
    let bound = match &goal.bound {
        TraitIdentity::Builtin(name) => name.as_str(),
        TraitIdentity::Declared(_) => "",
        TraitIdentity::Unknown(_) => return None,
    };
    if bound == "Copy" {
        return if crate::check::is_copy(types, goal.ty) {
            Some(Requirements::structural(Vec::new()))
        } else {
            None
        };
    }
    if bound == "Data" {
        let mut children = Vec::new();
        match &ty.kind {
            TypeKind::World => return None,
            TypeKind::Ref { inner, .. }
            | TypeKind::Slice { inner, .. }
            | TypeKind::Vec { inner }
            | TypeKind::Box { inner }
            | TypeKind::Array { inner, .. } => {
                children.push(Goal {
                    ty: *inner,
                    bound: goal.bound.clone(),
                });
            }
            TypeKind::Struct { fields, .. } => {
                for field in fields {
                    children.push(Goal {
                        ty: field.ty,
                        bound: goal.bound.clone(),
                    });
                }
            }
            TypeKind::Enum { variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        children.push(Goal {
                            ty: field.ty,
                            bound: goal.bound.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
        return Some(Requirements::structural(children));
    }
    let automatic = bound == "Send" || bound == "Sync";
    let comparison =
        bound == "Ord" || bound == "Eq" || bound == "PartialOrd" || bound == "PartialEq";
    let clone = bound == "Clone";
    if automatic || comparison || clone {
        let mut children = Vec::new();
        let mut structural = true;
        match &ty.kind {
            TypeKind::Unit | TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } => {}
            TypeKind::Str => {
                if !automatic {
                    return None;
                }
            }
            TypeKind::F64 => {
                if bound == "Ord" || bound == "Eq" {
                    return None;
                }
            }
            TypeKind::World | TypeKind::Stdout => {
                return None;
            }
            TypeKind::Vec { inner } | TypeKind::Box { inner } | TypeKind::Array { inner, .. } => {
                children.push(Goal {
                    ty: *inner,
                    bound: goal.bound.clone(),
                })
            }
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
                        bound: TraitIdentity::Builtin(child_bound.to_string()),
                    });
                }
            }
            TypeKind::Struct { fields, .. } => {
                let mut tuple = ty.name.starts_with('(') && ty.name.ends_with(')');
                for i in 0..fields.len() {
                    if fields[i].name != i.to_string() {
                        tuple = false;
                    }
                }
                if automatic || tuple || derived(source, types, applications, goal.ty, bound) {
                    for field in fields {
                        children.push(Goal {
                            ty: field.ty,
                            bound: goal.bound.clone(),
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
                                bound: goal.bound.clone(),
                            });
                        }
                    }
                } else {
                    structural = false;
                }
            }
        }
        if structural {
            return Some(Requirements::structural(children));
        }
    }
    let selected = match inferred.get(goal) {
        Some(witness) => witness.clone(),
        None => select_implementation(source, index, types, applications, goal.ty, &goal.bound)?,
    };
    let implementation = &source.impls[selected.implementation];
    let typed = implementation.generics.iter().any(|parameter| {
        !parameter.lifetime
            && (!selected.substitutions.contains_key(&parameter.name)
                || parameter.bounds.iter().any(|bound| {
                    matches!(&source.types[*bound].kind,
                Type::Path(path) if !path.args.is_empty())
                }))
    });
    if typed && !inferred.contains_key(goal) {
        return Some(Requirements {
            children: Vec::new(),
            structural: false,
            witness: Some(selected),
            infer: true,
            parameterized: Vec::new(),
        });
    }
    let env = &selected.substitutions;
    let mut children = Vec::new();
    let mut parameterized = Vec::new();
    for param in &implementation.generics {
        if param.lifetime {
            continue;
        }
        let concrete = *env.get(&param.name)?;
        for constraint in &param.bounds {
            if !add_bound(source, *constraint, concrete, &mut children) {
                parameterized.push((*constraint, concrete));
            }
        }
    }
    if let TraitIdentity::Declared(index) = goal.bound {
        for constraint in &source.traits[index].bounds {
            if !add_bound(source, *constraint, goal.ty, &mut children) {
                parameterized.push((*constraint, goal.ty));
            }
        }
    }
    Some(Requirements {
        children,
        structural: false,
        witness: Some(selected),
        infer: false,
        parameterized,
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
    let identity = if let Some(index) = source
        .traits
        .iter()
        .position(|item| qualified(&item.module, &item.name) == bound)
    {
        TraitIdentity::Declared(index)
    } else {
        TraitIdentity::Builtin(bound.to_string())
    };
    let Ok(index) = ImplementationIndex::new(source) else {
        return false;
    };
    satisfies_resolved(source, &index, types, applications, ty, &identity)
}

pub(crate) fn satisfies_resolved(
    source: &Program,
    index: &ImplementationIndex,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    ty: usize,
    bound: &TraitIdentity,
) -> bool {
    satisfies_under(
        source,
        index,
        types,
        applications,
        ty,
        bound,
        &BTreeMap::new(),
    )
}

pub(crate) fn satisfies_under(
    source: &Program,
    index: &ImplementationIndex,
    types: &[crate::k::Type],
    applications: &BTreeMap<usize, (String, Vec<usize>)>,
    ty: usize,
    bound: &TraitIdentity,
    rigid: &BTreeMap<usize, BTreeSet<TraitIdentity>>,
) -> bool {
    let mut solver = BoundSolver::new(ty, bound.clone());
    loop {
        match solver.advance(BoundContext {
            source,
            index,
            types,
            applications,
            rigid,
        }) {
            BoundStep::Done(valid) => return valid,
            BoundStep::Infer { goal, .. } => solver.inferred(goal, None),
            BoundStep::Validate { goal, .. } => solver.validated(goal, None),
        }
    }
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

fn alias_cycles(source: &Program) -> Result<(), Diagnostic> {
    let mut names = BTreeMap::new();
    for i in 0..source.aliases.len() {
        names.insert(
            qualified(&source.aliases[i].module, &source.aliases[i].name),
            i,
        );
    }
    let mut edges = Vec::new();
    for i in 0..source.aliases.len() {
        let alias = &source.aliases[i];
        let mut stack = vec![alias.ty];
        while let Some(ty) = stack.pop() {
            if let Type::Path(path) = &source.types[ty].kind {
                let mut parameter = false;
                for g in &alias.generics {
                    if path.declaration.is_none() && g.name == path.name {
                        parameter = true;
                    }
                }
                if !parameter {
                    if let Some(to) = names.get(&path.name) {
                        edges.push(Edge {
                            from: i,
                            to: *to,
                            expanding: false,
                            unknown: false,
                            span: alias.span,
                        });
                    }
                }
            }
            stack.extend(type_children(source, ty));
        }
    }
    let groups = components(source.aliases.len(), &edges);
    for edge in edges {
        if groups[edge.from] == groups[edge.to] {
            return Err(Diagnostic::new(edge.span, "E_TYPE", "recursive type alias"));
        }
    }
    Ok(())
}
#[derive(Clone)]
enum Normalize {
    Enter(usize),
    Alias(usize, usize),
    Node(usize, String, Vec<usize>),
    DeclaredTrait(usize, usize, Vec<usize>),
}
fn normalize(source: &Program, term: usize, terms: &mut Vec<Term>) -> Result<usize, Diagnostic> {
    let mut done = BTreeMap::new();
    let mut tasks = vec![Normalize::Enter(term)];
    while let Some(task) = tasks.pop() {
        match task {
            Normalize::Enter(id) => {
                if done.contains_key(&id) {
                    continue;
                }
                match terms[id].clone() {
                    Term::DeclaredTrait(declaration, kids) => {
                        tasks.push(Normalize::DeclaredTrait(id, declaration, kids.clone()));
                        for child in kids {
                            tasks.push(Normalize::Enter(child));
                        }
                    }
                    Term::Named(name, kids) => {
                        let mut alias = None;
                        for a in &source.aliases {
                            if qualified(&a.module, &a.name) == name {
                                alias = Some(a);
                                break;
                            }
                        }
                        if let Some(a) = alias {
                            if a.generics.len() != kids.len() {
                                return Err(Diagnostic::new(
                                    a.span,
                                    "E_TYPE",
                                    "type alias argument count differs",
                                ));
                            }
                            let mut env = BTreeMap::new();
                            for i in 0..kids.len() {
                                env.insert(a.generics[i].name.clone(), kids[i]);
                            }
                            let expanded = type_term(source, a.ty, &env, terms);
                            tasks.push(Normalize::Alias(id, expanded));
                            tasks.push(Normalize::Enter(expanded));
                        } else {
                            tasks.push(Normalize::Node(id, name, kids.clone()));
                            for k in kids {
                                tasks.push(Normalize::Enter(k));
                            }
                        }
                    }
                    _ => {
                        done.insert(id, id);
                    }
                }
            }
            Normalize::Alias(id, expanded) => {
                done.insert(id, done[&expanded]);
            }
            Normalize::DeclaredTrait(id, declaration, kids) => {
                let normalized = kids.into_iter().map(|child| done[&child]).collect();
                let new = push(terms, Term::DeclaredTrait(declaration, normalized));
                done.insert(id, new);
            }
            Normalize::Node(id, name, kids) => {
                let mut normalized = Vec::new();
                for k in kids {
                    normalized.push(done[&k]);
                }
                let new = push(terms, Term::Named(name, normalized));
                done.insert(id, new);
            }
        }
    }
    Ok(done[&term])
}
fn equivalent(terms: &[Term], a: usize, b: usize) -> bool {
    let mut stack = vec![(a, b)];
    while let Some((a, b)) = stack.pop() {
        match (&terms[a], &terms[b]) {
            (Term::Variable(x), Term::Variable(y)) if x == y => {}
            (Term::DeclaredTrait(a, ak), Term::DeclaredTrait(b, bk))
                if a == b && ak.len() == bk.len() =>
            {
                for (a, b) in ak.iter().zip(bk) {
                    stack.push((*a, *b));
                }
            }
            (Term::Named(an, ak), Term::Named(bn, bk)) if an == bn && ak.len() == bk.len() => {
                for i in 0..ak.len() {
                    stack.push((ak[i], bk[i]));
                }
            }
            _ => return false,
        }
    }
    true
}
fn signature_type(
    source: &Program,
    id: TypeId,
    env: &BTreeMap<String, usize>,
    terms: &mut Vec<Term>,
) -> Result<usize, Diagnostic> {
    let raw = type_term(source, id, env, terms);
    normalize(source, raw, terms)
}
fn bound_closure(
    source: &Program,
    first: usize,
    self_ty: usize,
    terms: &mut Vec<Term>,
) -> Result<Vec<usize>, Diagnostic> {
    let mut result = Vec::new();
    let mut pending = vec![first];
    while let Some(bound) = pending.pop() {
        if result
            .iter()
            .any(|previous| equivalent(terms, bound, *previous))
        {
            continue;
        }
        result.push(bound);
        match terms[bound].clone() {
            Term::Named(name, args) => {
                let parents: &[&str] = match name.as_str() {
                    "Fn" => &["FnMut"],
                    "FnMut" => &["FnOnce"],
                    _ => builtin_parents(&name),
                };
                for parent in parents {
                    pending.push(push(
                        terms,
                        Term::Named((*parent).to_string(), args.clone()),
                    ));
                }
            }
            Term::DeclaredTrait(index, args) => {
                let declaration = &source.traits[index];
                if declaration.generics.len() != args.len() {
                    continue;
                }
                let mut env = BTreeMap::new();
                env.insert("Self".to_string(), self_ty);
                for (parameter, argument) in declaration.generics.iter().zip(args) {
                    env.insert(parameter.name.clone(), argument);
                }
                for parent in &declaration.bounds {
                    pending.push(signature_type(source, *parent, &env, terms)?);
                }
            }
            _ => {}
        }
    }
    Ok(result)
}

fn known_bounds(
    source: &Program,
    parameters: &[GenericParam],
    env: &BTreeMap<String, usize>,
    terms: &mut Vec<Term>,
) -> Result<Vec<(usize, usize)>, Diagnostic> {
    let mut known = Vec::new();
    for p in parameters {
        if p.lifetime {
            continue;
        }
        let ty = match env.get(&p.name) {
            Some(t) => *t,
            None => continue,
        };
        for b in &p.bounds {
            let bound = signature_type(source, *b, env, terms)?;
            for parent in bound_closure(source, bound, ty, terms)? {
                known.push((ty, parent));
            }
        }
    }
    Ok(known)
}
fn symbolic_bound(
    terms: &mut Vec<Term>,
    ty: usize,
    bound: usize,
    known: &[(usize, usize)],
) -> bool {
    let mut pending = vec![(ty, bound)];
    while let Some((ty, bound)) = pending.pop() {
        let mut found = false;
        for (known_ty, known_bound) in known {
            if equivalent(terms, ty, *known_ty) && equivalent(terms, bound, *known_bound) {
                found = true;
                break;
            }
        }
        if found {
            continue;
        }
        let (name, args) = match terms[bound].clone() {
            Term::Named(n, a) => (n, a),
            _ => return false,
        };
        if !args.is_empty() {
            return false;
        }
        let (type_name, children) = match terms[ty].clone() {
            Term::Named(n, a) => (n, a),
            _ => return false,
        };
        let marker = name == "Copy"
            || name == "Clone"
            || name == "Data"
            || name == "Send"
            || name == "Sync"
            || name == "Eq"
            || name == "Ord"
            || name == "PartialEq"
            || name == "PartialOrd";
        if !marker {
            return false;
        }
        match type_name.as_str() {
            "bool" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32"
            | "i64" | "i128" | "isize" | "char" => {}
            "f64" | "f32" => {
                if name == "Eq" || name == "Ord" {
                    return false;
                }
            }
            "String" => {
                if name == "Copy" {
                    return false;
                }
            }
            "Vec" | "Box" => {
                if name == "Copy" {
                    return false;
                }
                for child in children {
                    pending.push((child, bound));
                }
            }
            "tuple" => {
                for child in children {
                    pending.push((child, bound));
                }
            }
            "&" => {
                if name != "Copy" && name != "Clone" {
                    let required = if name == "Send" {
                        push(terms, Term::Named("Sync".to_string(), Vec::new()))
                    } else {
                        bound
                    };
                    for child in children {
                        pending.push((child, required));
                    }
                }
            }
            "&mut" => {
                if name == "Copy" || name == "Clone" {
                    return false;
                }
                for child in children {
                    pending.push((child, bound));
                }
            }
            _ if type_name.starts_with("[;") => {
                for child in children {
                    pending.push((child, bound));
                }
            }
            _ => return false,
        }
    }
    true
}
fn associated_dependencies(source: &Program, ty: TypeId, env: &BTreeMap<String, usize>) -> bool {
    let mut work = vec![ty];
    while let Some(t) = work.pop() {
        if let Type::Path(path) = &source.types[t].kind {
            if path.name.starts_with("Self::") && !env.contains_key(&path.name) {
                return false;
            }
        }
        work.extend(type_children(source, t));
    }
    true
}
fn signatures(source: &Program) -> Result<(), Diagnostic> {
    let implementations = ImplementationIndex::new(source)?;
    for implementation in &source.impls {
        let trait_id = match implementation.trait_ty {
            Some(id) => id,
            None => continue,
        };
        let path = match &source.types[trait_id].kind {
            Type::Path(p) => p,
            _ => continue,
        };
        let declaration = match bound_identity(source, path) {
            TraitIdentity::Declared(index) => &source.traits[index],
            _ => continue,
        };
        let mut terms = Vec::new();
        let mut next = 0;
        let mut actual_env = variables(source, &implementation.generics, &mut terms, &mut next);
        let target = signature_type(source, implementation.target, &actual_env, &mut terms)?;
        actual_env.insert("Self".to_string(), target);
        let mut expected_env = BTreeMap::new();
        expected_env.insert("Self".to_string(), target);
        if path.args.len() != declaration.generics.len() {
            return Err(Diagnostic::new(
                implementation.span,
                "E_TRAIT",
                "trait type argument count differs",
            ));
        }
        for i in 0..path.args.len() {
            let t = signature_type(source, path.args[i], &actual_env, &mut terms)?;
            expected_env.insert(declaration.generics[i].name.clone(), t);
        }
        let known = known_bounds(source, &implementation.generics, &actual_env, &mut terms)?;
        for generic in &declaration.generics {
            if generic.lifetime {
                continue;
            }
            let concrete = expected_env[&generic.name];
            for bound in &generic.bounds {
                let b = signature_type(source, *bound, &expected_env, &mut terms)?;
                if !signature_bounds::prove(
                    source,
                    &implementations,
                    &mut terms,
                    concrete,
                    b,
                    &known,
                )? {
                    return Err(Diagnostic::new(
                        implementation.span,
                        "E_TRAIT",
                        "implementation does not establish a trait type-argument bound",
                    ));
                }
            }
        }
        let mut associated = Vec::new();
        for expected in &declaration.associated {
            let value = implementation
                .associated
                .iter()
                .find(|a| a.name == expected.name)
                .and_then(|a| a.value);
            if let Some(value) = value {
                associated.push((expected.name.clone(), value, false));
            } else if let Some(value) = expected.value {
                associated.push((expected.name.clone(), value, true));
            }
        }
        let mut remaining = associated.len();
        let mut done = vec![false; remaining];
        while remaining > 0 {
            let before = remaining;
            for i in 0..associated.len() {
                if done[i] {
                    continue;
                }
                let (name, ty, default) = &associated[i];
                let env = if *default { &expected_env } else { &actual_env };
                if !associated_dependencies(source, *ty, env) {
                    continue;
                }
                let value = signature_type(source, *ty, env, &mut terms)?;
                let key = format!("Self::{name}");
                actual_env.insert(key.clone(), value);
                expected_env.insert(key, value);
                done[i] = true;
                remaining -= 1;
            }
            if remaining == before {
                return Err(Diagnostic::new(
                    implementation.span,
                    "E_TRAIT",
                    "associated type definitions are cyclic or unresolved",
                ));
            }
        }
        for associated in &declaration.associated {
            if let Some(concrete) = expected_env.get(&format!("Self::{}", associated.name)) {
                for constraint in &associated.bounds {
                    let b = signature_type(source, *constraint, &expected_env, &mut terms)?;
                    if !signature_bounds::prove(
                        source,
                        &implementations,
                        &mut terms,
                        *concrete,
                        b,
                        &known,
                    )? {
                        return Err(Diagnostic::new(
                            implementation.span,
                            "E_TRAIT",
                            "associated type does not establish its declared bound",
                        ));
                    }
                }
            }
        }
        for method in &implementation.methods {
            let actual = &source.functions[*method];
            let expected = match declaration
                .methods
                .iter()
                .map(|m| &source.functions[*m])
                .find(|m| m.name == actual.name)
            {
                Some(f) => f,
                None => continue,
            };
            let mut aenv = actual_env.clone();
            let mut eenv = expected_env.clone();
            for i in 0..actual.generics.len() {
                let a = &actual.generics[i];
                let e = &expected.generics[i];
                if a.lifetime != e.lifetime {
                    return Err(Diagnostic::new(
                        actual.span,
                        "E_TRAIT",
                        "method generic parameter kinds differ",
                    ));
                }
                if a.lifetime {
                    continue;
                }
                if aenv.contains_key(&a.name) || eenv.contains_key(&e.name) {
                    return Err(Diagnostic::new(
                        actual.span,
                        "E_TRAIT",
                        "method type parameter shadows an enclosing type parameter",
                    ));
                }
                let value = push(&mut terms, Term::Variable(next));
                next += 1;
                aenv.insert(a.name.clone(), value);
                eenv.insert(e.name.clone(), value);
            }
            let mut pairs = vec![(actual.result, expected.result)];
            for i in 0..actual.params.len() {
                pairs.push((actual.params[i].ty, expected.params[i].ty));
            }
            for (a, e) in pairs {
                let a = signature_type(source, a, &aenv, &mut terms)?;
                let e = signature_type(source, e, &eenv, &mut terms)?;
                if !equivalent(&terms, a, e) {
                    return Err(Diagnostic::new(
                        actual.span,
                        "E_TRAIT",
                        "implementation method type differs from trait signature",
                    ));
                }
            }
            let mut available = known.clone();
            available.extend(known_bounds(source, &expected.generics, &eenv, &mut terms)?);
            for g in &actual.generics {
                if g.lifetime {
                    continue;
                }
                let ty = aenv[&g.name];
                for b in &g.bounds {
                    let bound = signature_type(source, *b, &aenv, &mut terms)?;
                    if !signature_bounds::prove(
                        source,
                        &implementations,
                        &mut terms,
                        ty,
                        bound,
                        &available,
                    )? {
                        return Err(Diagnostic::new(
                            actual.span,
                            "E_TRAIT",
                            "implementation method requires a stronger generic bound",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod signature_tests {
    use super::*;
    fn check(text: &str) -> Result<(), Diagnostic> {
        validate(&crate::syntax::parse(text).unwrap())
    }
    #[test]
    fn mismatched_unused_result() {
        assert_eq!(check("trait A {fn f(&self)->i64;}impl A for bool{fn f(&self)->bool{true}}fn probe()->i64{42}").unwrap_err().code,"E_TRAIT");
    }
    #[test]
    fn receiver_convention_matters() {
        assert_eq!(
            check("trait A {fn f(&self);}impl A for i64{fn f(&mut self){}}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    #[test]
    fn associated_substitution_and_alias() {
        check("type Alias<T> = Vec<T>;trait A {type Out;fn f(&self)->Self::Out;}impl A for i64{type Out=Alias<i64>;fn f(&self)->Vec<i64>{vec![1]}}").unwrap();
    }
    #[test]
    fn renamed_method_generics() {
        check("trait A {fn f<T>(&self,x:T)->T;}impl A for i64{fn f<U>(&self,x:U)->U{x}}").unwrap();
    }
    #[test]
    fn stronger_method_bound_rejected() {
        assert_eq!(
            check("trait A {fn f<T>(&self,x:T);}impl A for i64{fn f<U:Copy>(&self,x:U){}}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    #[test]
    fn omitted_inherited_bound_accepted() {
        check("trait A {fn f<T:Copy>(&self,x:T);}impl A for i64{fn f<U>(&self,x:U){}}").unwrap();
    }
    #[test]
    fn implied_method_bound() {
        check("trait A {fn f<T:Ord>(&self,x:T);}impl A for i64{fn f<U:Eq>(&self,x:U){}}").unwrap();
    }
    #[test]
    fn trait_argument_substitution() {
        check("trait A<T>{fn f(&self,x:T)->T;}impl<U> A<U> for Vec<U>{fn f(&self,x:U)->U{x}}")
            .unwrap();
    }
    #[test]
    fn missing_trait_argument_bound() {
        assert_eq!(
            check("trait A<T:Copy>{}impl<U> A<U> for Vec<U>{}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
    }
    #[test]
    fn associated_bound_checked() {
        assert_eq!(
            check("trait A {type Out:Copy;}impl A for i64{type Out=Vec<i64>;}")
                .unwrap_err()
                .code,
            "E_TRAIT"
        );
        check("trait A{type Out:Copy;}impl<T:Copy> A for Vec<T>{type Out=T;}").unwrap();
    }
    #[test]
    fn alias_cycle_rejected() {
        check("mod std {pub mod result {pub enum Result<T,E> {Ok(T),Err(E)}}} type Result<T> = std::result::Result<T, i64>; trait A {fn f(&self)->Result<bool>;} impl A for i64 {fn f(&self)->std::result::Result<bool,i64>{std::result::Result::Ok(true)}}").unwrap();
        assert_eq!(check("type A=B;type B=Vec<A>;").unwrap_err().code, "E_TYPE");
    }
}

fn add_name(
    names: &mut BTreeMap<String, Span>,
    module: &str,
    name: &str,
    span: Span,
) -> Result<(), Diagnostic> {
    let path = qualified(module, name);
    if names.insert(path.clone(), span).is_some() {
        return Err(Diagnostic::new(
            span,
            "E_NAME",
            &format!("duplicate declaration: {path}"),
        ));
    }
    Ok(())
}
fn generic_names(params: &[GenericParam]) -> Result<(), Diagnostic> {
    let mut names = BTreeMap::new();
    for p in params {
        add_name(&mut names, "", &p.name, p.span)?;
    }
    Ok(())
}
fn declarations(source: &Program) -> Result<(), Diagnostic> {
    let mut type_names = BTreeMap::new();
    let mut value_names = BTreeMap::new();
    for im in &source.impls {
        generic_names(&im.generics)?;
    }
    for module in &source.modules {
        if module.external {
            return Err(Diagnostic::new(
                module.span,
                "E_UNSUPPORTED",
                "external module loading is not implemented",
            ));
        }
        add_name(&mut type_names, &module.parent, &module.name, module.span)?;
    }
    for i in 0..source.functions.len() {
        let f = &source.functions[i];
        generic_names(&f.generics)?;
        let mut parameters = BTreeMap::new();
        for p in &f.params {
            if p.name != "_" {
                add_name(&mut parameters, "", &p.name, p.span)?;
            }
        }
        if source.function_owner(FunctionId { index: i }) == FunctionOwner::Free {
            add_name(&mut value_names, &f.module, &f.name, f.span)?;
        }
    }
    for s in &source.structs {
        add_name(&mut type_names, &s.module, &s.name, s.span)?;
        generic_names(&s.generics)?;
        if s.tuple || s.fields.is_empty() {
            add_name(&mut value_names, &s.module, &s.name, s.span)?;
        }
        let mut fields = BTreeMap::new();
        for f in &s.fields {
            add_name(&mut fields, "", &f.name, f.span)?;
        }
    }
    for e in &source.enums {
        add_name(&mut type_names, &e.module, &e.name, e.span)?;
        generic_names(&e.generics)?;
        let mut variants = BTreeMap::new();
        for v in &e.variants {
            add_name(&mut variants, "", &v.name, v.span)?;
            let mut fields = BTreeMap::new();
            for f in &v.fields {
                add_name(&mut fields, "", &f.name, f.span)?;
            }
        }
    }
    for t in &source.traits {
        add_name(&mut type_names, &t.module, &t.name, t.span)?;
        generic_names(&t.generics)?;
        let mut functions = BTreeMap::new();
        for f in &t.methods {
            let f = &source.functions[*f];
            add_name(&mut functions, "", &f.name, f.span)?;
        }
        let mut associated = BTreeMap::new();
        for a in &t.associated {
            add_name(&mut associated, "", &a.name, a.span)?;
        }
    }
    for alias in &source.aliases {
        add_name(&mut type_names, &alias.module, &alias.name, alias.span)?;
        generic_names(&alias.generics)?;
    }
    for constant in &source.constants {
        add_name(
            &mut value_names,
            &constant.module,
            &constant.name,
            constant.span,
        )?;
    }
    Ok(())
}
fn constant_cycles(source: &Program) -> Result<(), Diagnostic> {
    let mut names = BTreeMap::new();
    let mut starts = BTreeMap::new();
    for i in 0..source.constants.len() {
        let c = &source.constants[i];
        names.insert(qualified(&c.module, &c.name), i);
        starts.insert(source.exprs[c.value].span.start, i);
    }
    let mut edges = Vec::new();
    for expression in &source.exprs {
        let path = match &expression.kind {
            Expr::Path(p) => p,
            _ => continue,
        };
        let from = match starts.range(..=expression.span.start).next_back() {
            Some((_, i)) => *i,
            None => continue,
        };
        let constant = &source.constants[from];
        if expression.span.end > source.exprs[constant.value].span.end {
            continue;
        }
        let mut name = qualified(&constant.module, &path.name);
        if let Some(rest) = path.name.strip_prefix("crate::") {
            name = rest.to_string();
        } else if let Some(rest) = path.name.strip_prefix("self::") {
            name = qualified(&constant.module, rest);
        }
        let to = names.get(&name).or_else(|| names.get(&path.name));
        if let Some(to) = to {
            edges.push(Edge {
                from,
                to: *to,
                expanding: false,
                unknown: false,
                span: expression.span,
            });
        }
    }
    let groups = components(source.constants.len(), &edges);
    for edge in edges {
        if groups[edge.from] == groups[edge.to] {
            return Err(Diagnostic::new(
                edge.span,
                "E_TYPE",
                "recursive constant definition",
            ));
        }
    }
    Ok(())
}
struct DataDeclaration<'a> {
    name: String,
    params: &'a [GenericParam],
    fields: Vec<TypeId>,
    span: Span,
}
fn finite_data(source: &Program) -> Result<(), Diagnostic> {
    let mut declarations = Vec::new();
    for s in &source.structs {
        declarations.push(DataDeclaration {
            name: qualified(&s.module, &s.name),
            params: &s.generics,
            fields: s.fields.iter().map(|f| f.ty).collect(),
            span: s.span,
        });
    }
    for e in &source.enums {
        let mut fields = Vec::new();
        for v in &e.variants {
            for f in &v.fields {
                fields.push(f.ty);
            }
        }
        declarations.push(DataDeclaration {
            name: qualified(&e.module, &e.name),
            params: &e.generics,
            fields,
            span: e.span,
        });
    }
    let mut terms = Vec::new();
    let mut count = 0;
    let mut environments = Vec::new();
    let mut target_nodes = Vec::new();
    let mut names = BTreeMap::new();
    for i in 0..declarations.len() {
        let declaration = &declarations[i];
        let env = variables(source, declaration.params, &mut terms, &mut count);
        let mut nodes = Vec::new();
        for p in declaration.params {
            if p.lifetime {
                continue;
            }
            if let Term::Variable(v) = terms[env[&p.name]] {
                nodes.push(v);
            }
        }
        environments.push(env);
        target_nodes.push(nodes);
        names.insert(declaration.name.clone(), i);
    }
    let mut edges = Vec::new();
    for i in 0..declarations.len() {
        let declaration = &declarations[i];
        if target_nodes[i].is_empty() {
            continue;
        }
        for field in &declaration.fields {
            let first = signature_type(source, *field, &environments[i], &mut terms)?;
            let mut pending = vec![first];
            while let Some(term) = pending.pop() {
                if let Term::Named(name, args) = terms[term].clone() {
                    if let Some(target) = names.get(&name) {
                        let mut concrete = Vec::new();
                        for arg in &args {
                            if !matches!(&terms[*arg],Term::Named(name,_) if name=="lifetime") {
                                concrete.push(*arg);
                            }
                        }
                        if concrete.len() == target_nodes[*target].len() {
                            for j in 0..concrete.len() {
                                flow(
                                    &terms,
                                    concrete[j],
                                    target_nodes[*target][j],
                                    declaration.span,
                                    &mut edges,
                                );
                            }
                        }
                    }
                    pending.extend(args);
                }
            }
        }
    }
    let groups = components(count, &edges);
    for edge in edges {
        if edge.expanding && groups[edge.from] == groups[edge.to] {
            return Err(Diagnostic::new(
                edge.span,
                "E_RECURSION",
                "data type parameter flow expands inside a recursive component",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod declaration_tests {
    use super::*;
    fn check(text: &str) -> Result<(), Diagnostic> {
        validate(&crate::syntax::parse(text).unwrap())
    }
    #[test]
    fn duplicate_functions() {
        assert_eq!(check("fn f(){} fn f(){}").unwrap_err().code, "E_NAME");
    }
    #[test]
    fn type_and_value_namespaces() {
        check("struct S{x:i64} fn S()->i64{42}").unwrap();
        assert_eq!(
            check("struct S(i64);fn S()->i64{42}").unwrap_err().code,
            "E_NAME"
        );
    }
    #[test]
    fn duplicate_fields_and_parameters() {
        assert_eq!(check("struct S{x:i64,x:bool}").unwrap_err().code, "E_NAME");
        assert_eq!(check("fn f(x:i64,x:i64){}").unwrap_err().code, "E_NAME");
    }
    #[test]
    fn direct_and_mutual_constant_cycles() {
        assert_eq!(check("const A:i64=A;").unwrap_err().code, "E_TYPE");
        assert_eq!(
            check("const A:i64=B;const B:i64=A;").unwrap_err().code,
            "E_TYPE"
        );
        check("const A:i64=1;const B:i64=A+1;").unwrap();
    }
    #[test]
    fn external_module_loading_is_explicit() {
        assert_eq!(check("mod missing;").unwrap_err().code, "E_UNSUPPORTED");
        check("mod a { fn f(){} } mod b {fn f(){}}").unwrap();
    }
    #[test]
    fn expanding_data_recursion() {
        assert_eq!(
            check("struct Node<T>{value:T,next:Box<Node<Vec<T>>>}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    #[test]
    fn mutual_data_expansion() {
        assert_eq!(
            check("struct A<T>{x:Box<B<Vec<T>>>}struct B<U>{x:Box<A<U>>}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    #[test]
    fn alias_data_expansion() {
        assert_eq!(
            check("type Nest<T>=Node<Vec<T>>;struct Node<T>{next:Box<Nest<T>>}")
                .unwrap_err()
                .code,
            "E_RECURSION"
        );
    }
    #[test]
    fn regular_data_recursion() {
        check("struct Node<T>{value:T,next:Box<Node<T>>}").unwrap();
        check("enum List<T>{Nil,Cons(T,Box<List<T>>)}").unwrap();
    }
}
