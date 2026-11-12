//! Definition-local inference. The graph records structural type constraints
//! before K construction; integer fallback happens only after those constraints
//! have propagated. Method selection and ownership remain checked separately.
use super::{Bindings, Compiler, Result, a, err, is_copy, k};
use std::collections::{BTreeMap, VecDeque};

type Variable = usize;
fn context(mut problem: a::Diagnostic, message: &str) -> a::Diagnostic {
    problem.message = format!("{message}: {}", problem.message);
    problem
}

#[derive(Clone, PartialEq, Eq)]
enum Shape {
    Atom(String),
    Rigid(String),
    Source(a::TypeId),
    Vec(Variable),
    Box(Variable),
    Ref(bool, Variable),
    Slice(Variable),
    Array(Variable, usize),
    Tuple(Vec<Variable>),
    Nominal(a::DeclarationId, Vec<Variable>),
    Never,
}
impl Shape {
    fn children(&self) -> Vec<Variable> {
        match self {
            Self::Vec(v) | Self::Box(v) | Self::Ref(_, v) | Self::Slice(v) | Self::Array(v, _) => {
                vec![*v]
            }
            Self::Tuple(v) | Self::Nominal(_, v) => v.clone(),
            _ => Vec::new(),
        }
    }
    fn integer(&self) -> bool {
        matches!(self, Self::Atom(name) if integer_name(name))
    }
}
fn integer_name(name: &str) -> bool {
    matches!(
        name,
        "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "isize" | "usize"
    )
}
fn integer_suffix(text: &str) -> Option<&'static str> {
    [
        "usize", "isize", "i64", "u64", "i32", "u32", "i16", "u16", "i8", "u8",
    ]
    .into_iter()
    .find(|suffix| text.ends_with(suffix))
}
#[derive(Clone)]
struct Node {
    parent: Variable,
    rank: usize,
    shape: Option<Shape>,
    integer: bool,
    // An unresolved source operation can still constrain this component later.
    // Do not manufacture a numeric default at that unresolved boundary.
    blocked: bool,
}

pub(super) struct DefinitionTypes {
    nodes: Vec<Node>,
    expressions: BTreeMap<a::ExprId, Variable>,
    bindings: BTreeMap<a::PatternId, Variable>,
}
#[derive(Default)]
pub(super) struct Types {
    pub(super) expressions: BTreeMap<a::ExprId, usize>,
    pub(super) bindings: BTreeMap<a::PatternId, usize>,
}
impl DefinitionTypes {
    fn root(&self, mut v: Variable) -> Variable {
        while self.nodes[v].parent != v {
            v = self.nodes[v].parent;
        }
        v
    }
    fn shape(&self, v: Variable) -> Option<Shape> {
        self.nodes[self.root(v)].shape.clone()
    }

    pub(super) fn instantiate(
        &self,
        compiler: &mut Compiler<'_>,
        substitutions: &BTreeMap<String, usize>,
    ) -> Result<Types> {
        let mut known = BTreeMap::new();
        let mut result = Types::default();
        for (expression, v) in &self.expressions {
            let span = compiler.source.exprs[*expression].span;
            if let Some(ty) = self.materialize(*v, compiler, substitutions, &mut known, span)? {
                result.expressions.insert(*expression, ty);
            }
        }
        for (pattern, v) in &self.bindings {
            let span = compiler.source.patterns[*pattern].span;
            if let Some(ty) = self.materialize(*v, compiler, substitutions, &mut known, span)? {
                result.bindings.insert(*pattern, ty);
            }
        }
        Ok(result)
    }

    fn materialize(
        &self,
        v: Variable,
        compiler: &mut Compiler<'_>,
        substitutions: &BTreeMap<String, usize>,
        known: &mut BTreeMap<Variable, Option<usize>>,
        span: a::Span,
    ) -> Result<Option<usize>> {
        let root = self.root(v);
        let mut pending = vec![(root, false)];
        while let Some((id, ready)) = pending.pop() {
            let id = self.root(id);
            if known.contains_key(&id) {
                continue;
            }
            let Some(shape) = self.nodes[id].shape.clone() else {
                known.insert(id, None);
                continue;
            };
            let children = shape.children();
            if !ready && !children.is_empty() {
                pending.push((id, true));
                for child in children.into_iter().rev() {
                    pending.push((self.root(child), false));
                }
                continue;
            }
            let mut values = Vec::new();
            let mut complete = true;
            for child in children {
                match known[&self.root(child)] {
                    Some(value) => values.push(value),
                    None => complete = false,
                }
            }
            if !complete {
                known.insert(id, None);
                continue;
            }
            let ty = match shape {
                Shape::Atom(name) => {
                    if let Some(ty) = compiler.type_names.get(&name) {
                        Some(*ty)
                    } else if name == "()" {
                        Some(k::UNIT)
                    } else {
                        Some(compiler.named_type(
                            &a::Path {
                                declaration: None,
                                name,
                                args: Vec::new(),
                            },
                            substitutions,
                            span,
                        )?)
                    }
                }
                Shape::Rigid(name) => substitutions.get(&name).copied(),
                Shape::Source(source) => compiler.resolve_if_known(source, substitutions)?,
                Shape::Vec(_) => {
                    Some(compiler.intern(String::new(), k::TypeKind::Vec { inner: values[0] }))
                }
                Shape::Box(_) => {
                    Some(compiler.intern(String::new(), k::TypeKind::Box { inner: values[0] }))
                }
                Shape::Ref(mutable, _) => {
                    if mutable && matches!(compiler.types[values[0]].kind, k::TypeKind::Str) {
                        return Err(err(
                            span,
                            "E_UNSUPPORTED",
                            "mutable str views are not implemented",
                        ));
                    }
                    Some(compiler.reference(values[0], mutable))
                }
                Shape::Slice(_) => Some(compiler.intern(
                    String::new(),
                    k::TypeKind::Slice {
                        inner: values[0],
                        mutable: false,
                    },
                )),
                Shape::Array(_, len) => Some(compiler.intern(
                    String::new(),
                    k::TypeKind::Array {
                        inner: values[0],
                        len,
                    },
                )),
                Shape::Tuple(_) => {
                    if values.is_empty() {
                        Some(k::UNIT)
                    } else {
                        let fields: Vec<_> = values
                            .into_iter()
                            .enumerate()
                            .map(|(i, ty)| k::Field {
                                name: i.to_string(),
                                ty,
                            })
                            .collect();
                        let copy = fields
                            .iter()
                            .all(|field| is_copy(&compiler.types, field.ty));
                        Some(compiler.intern(String::new(), k::TypeKind::Struct { fields, copy }))
                    }
                }
                Shape::Nominal(declaration, _) => {
                    let mut arguments = substitutions.clone();
                    for (parameter, ty) in parameters(compiler.source, declaration)
                        .into_iter()
                        .zip(values)
                    {
                        arguments.insert(parameter, ty);
                    }
                    let name = compiler.source.declarations[declaration].name.clone();
                    Some(compiler.named_type(
                        &a::Path {
                            declaration: Some(declaration),
                            name,
                            args: Vec::new(),
                        },
                        &arguments,
                        span,
                    )?)
                }
                Shape::Never => None,
            };
            known.insert(id, ty);
        }
        Ok(known[&root])
    }
}
fn parameters(source: &a::Program, declaration: a::DeclarationId) -> Vec<String> {
    let parameters = match source.declarations[declaration].item {
        a::Item::Struct(i) => &source.structs[i].generics,
        a::Item::Enum(i) => &source.enums[i].generics,
        a::Item::Alias(i) => &source.aliases[i].generics,
        _ => return Vec::new(),
    };
    parameters
        .iter()
        .filter(|p| !p.lifetime)
        .map(|p| p.name.clone())
        .collect()
}
#[derive(Clone)]
enum Obligation {
    Deref {
        source: Variable,
        result: Variable,
        span: a::Span,
    },
    Index {
        source: Variable,
        result: Variable,
        span: a::Span,
    },
    Field {
        source: Variable,
        name: String,
        result: Variable,
        span: a::Span,
    },
    Method {
        receiver: Variable,
        name: String,
        arguments: Vec<Variable>,
        result: Variable,
        span: a::Span,
    },
}
struct Infer<'a, 'b> {
    source: &'a a::Program,
    names: &'b crate::resolve::Resolver<'a>,
    definition: DefinitionTypes,
    bindings: Bindings,
    environment: BTreeMap<String, Variable>,
    result: Variable,
    waiting: Vec<Vec<usize>>,
    obligations: Vec<Option<Obligation>>,
    queue: VecDeque<usize>,
    queued: Vec<bool>,
}
impl Infer<'_, '_> {
    fn fresh(&mut self, shape: Option<Shape>, integer: bool, blocked: bool) -> Variable {
        let id = self.definition.nodes.len();
        self.definition.nodes.push(Node {
            parent: id,
            rank: 0,
            shape,
            integer,
            blocked,
        });
        self.waiting.push(Vec::new());
        id
    }
    fn unknown(&mut self) -> Variable {
        self.fresh(None, false, false)
    }
    fn blocked(&mut self) -> Variable {
        self.fresh(None, false, true)
    }
    fn shaped(&mut self, shape: Shape) -> Variable {
        self.fresh(Some(shape), false, false)
    }
    fn atom(&mut self, name: &str) -> Variable {
        self.shaped(Shape::Atom(name.to_string()))
    }
    fn root(&mut self, v: Variable) -> Variable {
        let root = self.definition.root(v);
        let mut next = v;
        while self.definition.nodes[next].parent != root {
            let parent = self.definition.nodes[next].parent;
            self.definition.nodes[next].parent = root;
            next = parent;
        }
        root
    }
    fn shape(&self, v: Variable) -> Option<Shape> {
        self.definition.shape(v)
    }
    fn never(&self, v: Variable) -> bool {
        self.shape(v) == Some(Shape::Never)
    }
    fn occurs(&self, variable: Variable, within: Variable) -> bool {
        let mut pending = vec![within];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(id) = pending.pop() {
            let id = self.definition.root(id);
            if id == variable {
                return true;
            }
            if seen.insert(id) {
                if let Some(shape) = &self.definition.nodes[id].shape {
                    pending.extend(shape.children());
                }
            }
        }
        false
    }
    fn enqueue(&mut self, id: usize) {
        if !self.queued[id] && self.obligations[id].is_some() {
            self.queue.push_back(id);
            self.queued[id] = true;
        }
    }
    fn join_roots(&mut self, mut a: Variable, mut b: Variable, shape: Option<Shape>) {
        if self.definition.nodes[a].rank < self.definition.nodes[b].rank {
            std::mem::swap(&mut a, &mut b);
        }
        if self.definition.nodes[a].rank == self.definition.nodes[b].rank {
            self.definition.nodes[a].rank += 1;
        }
        self.definition.nodes[b].parent = a;
        self.definition.nodes[a].shape = shape;
        self.definition.nodes[a].integer |= self.definition.nodes[b].integer;
        self.definition.nodes[a].blocked |= self.definition.nodes[b].blocked;
        let mut waiting = std::mem::take(&mut self.waiting[a]);
        waiting.extend(std::mem::take(&mut self.waiting[b]));
        for id in waiting {
            self.enqueue(id);
        }
    }
    fn unify(&mut self, left: Variable, right: Variable, span: a::Span) -> Result<()> {
        let mut pending = vec![(left, right)];
        while let Some((left, right)) = pending.pop() {
            let a = self.root(left);
            let b = self.root(right);
            if a == b {
                continue;
            }
            let sa = self.shape(a);
            let sb = self.shape(b);
            if sa == Some(Shape::Never) || sb == Some(Shape::Never) {
                continue;
            }
            let numeric = self.definition.nodes[a].integer || self.definition.nodes[b].integer;
            if numeric
                && sa
                    .as_ref()
                    .or(sb.as_ref())
                    .is_some_and(|shape| !shape.integer())
            {
                // Opaque generic projections stay with the original type checker.
                if matches!(sa, Some(Shape::Rigid(_) | Shape::Source(_)))
                    || matches!(sb, Some(Shape::Rigid(_) | Shape::Source(_)))
                {
                    continue;
                }
                return Err(err(
                    span,
                    "E_TYPE",
                    "integer inference requires an integer type",
                ));
            }
            match (&sa, &sb) {
                (None, _) | (_, None) => {
                    if self.occurs(a, b) || self.occurs(b, a) {
                        return Err(err(
                            span,
                            "E_TYPE",
                            "inference would require an infinite type",
                        ));
                    }
                    self.join_roots(a, b, sa.or(sb));
                }
                (Some(Shape::Vec(x)), Some(Shape::Vec(y)))
                | (Some(Shape::Box(x)), Some(Shape::Box(y)))
                | (Some(Shape::Slice(x)), Some(Shape::Slice(y))) => {
                    pending.push((*x, *y));
                    self.join_roots(a, b, sa);
                }
                (Some(Shape::Array(x, nx)), Some(Shape::Array(y, ny))) if nx == ny => {
                    pending.push((*x, *y));
                    self.join_roots(a, b, sa);
                }
                (Some(Shape::Ref(mx, x)), Some(Shape::Ref(my, y))) if mx == my => {
                    pending.push((*x, *y));
                    self.join_roots(a, b, sa);
                }
                (Some(Shape::Tuple(xs)), Some(Shape::Tuple(ys))) if xs.len() == ys.len() => {
                    pending.extend(xs.iter().copied().zip(ys.iter().copied()));
                    self.join_roots(a, b, sa);
                }
                (Some(Shape::Nominal(nx, xs)), Some(Shape::Nominal(ny, ys)))
                    if nx == ny && xs.len() == ys.len() =>
                {
                    pending.extend(xs.iter().copied().zip(ys.iter().copied()));
                    self.join_roots(a, b, sa);
                }
                (Some(Shape::Nominal(left, _)), Some(Shape::Nominal(right, _)))
                    if left != right =>
                {
                    return Err(err(
                        span,
                        "E_TYPE",
                        &format!(
                            "inference requires different declaration identities: {} and {}",
                            self.source.declarations[*left].name,
                            self.source.declarations[*right].name
                        ),
                    ));
                }
                (Some(Shape::Rigid(_) | Shape::Source(_)), _)
                | (_, Some(Shape::Rigid(_) | Shape::Source(_))) => {
                    if sa == sb {
                        self.join_roots(a, b, sa);
                    }
                }
                _ if sa == sb => self.join_roots(a, b, sa),
                _ => {
                    return Err(err(
                        span,
                        "E_TYPE",
                        "inference constraints require different types",
                    ));
                }
            }
        }
        Ok(())
    }
    fn coerce(&mut self, from: Variable, to: Variable, span: a::Span) -> Result<()> {
        if let (Some(Shape::Ref(fm, mut from)), Some(Shape::Ref(tm, to))) =
            (self.shape(from), self.shape(to))
        {
            if tm && !fm {
                return Ok(());
            } // K/type checking diagnoses an invalid mutable coercion.
            if let Some(Shape::Slice(element)) = self.shape(to) {
                if let Some(Shape::Vec(source) | Shape::Array(source, _)) = self.shape(from) {
                    return self.unify(source, element, span);
                }
            }
            while let Some(Shape::Box(inner)) = self.shape(from) {
                if matches!(self.shape(to), Some(Shape::Box(_))) {
                    break;
                }
                from = inner;
            }
            return self.unify(from, to, span);
        }
        self.unify(from, to, span)
    }
    fn obligation(&mut self, obligation: Obligation) {
        let id = self.obligations.len();
        self.obligations.push(Some(obligation));
        self.queued.push(false);
        self.enqueue(id);
    }
    fn watch(&mut self, variable: Variable, id: usize) {
        let root = self.root(variable);
        if !self.waiting[root].contains(&id) {
            self.waiting[root].push(id);
        }
    }
    fn strip_references(&mut self, mut variable: Variable, id: usize) -> Option<(Variable, Shape)> {
        loop {
            match self.shape(variable) {
                Some(Shape::Ref(_, inner)) => variable = inner,
                Some(shape) => return Some((variable, shape)),
                None => {
                    self.watch(variable, id);
                    return None;
                }
            }
        }
    }
    fn source_type(
        &mut self,
        id: a::TypeId,
        environment: &BTreeMap<String, Variable>,
    ) -> Result<Variable> {
        let node = self.source.types[id].clone();
        Ok(match node.kind {
            a::Type::Infer => self.unknown(),
            a::Type::Never => self.shaped(Shape::Never),
            a::Type::Ref { mutable, inner, .. } => {
                let inner = self.source_type(inner, environment)?;
                self.shaped(Shape::Ref(mutable, inner))
            }
            a::Type::Slice(inner) => {
                let inner = self.source_type(inner, environment)?;
                self.shaped(Shape::Slice(inner))
            }
            a::Type::Array { inner, len } => {
                let inner = self.source_type(inner, environment)?;
                match len.replace('_', "").parse::<usize>() {
                    Ok(len) => self.shaped(Shape::Array(inner, len)),
                    Err(_) => self.blocked(),
                }
            }
            a::Type::Tuple(fields) => {
                let mut values = Vec::new();
                for field in fields {
                    values.push(self.source_type(field, environment)?);
                }
                if values.is_empty() {
                    self.atom("()")
                } else {
                    self.shaped(Shape::Tuple(values))
                }
            }
            a::Type::Path(path) => {
                if path.declaration.is_none() && path.args.is_empty() {
                    if let Some(value) = environment.get(&path.name) {
                        return Ok(*value);
                    }
                    if integer_name(&path.name)
                        || matches!(
                            path.name.as_str(),
                            "bool" | "f64" | "str" | "World" | "Stdout"
                        )
                    {
                        return Ok(self.atom(&path.name));
                    }
                    if path.name.contains("::") {
                        return Ok(self.blocked());
                    }
                }
                let mut arguments = Vec::new();
                for arg in &path.args {
                    if !matches!(&self.source.types[*arg].kind, a::Type::Path(path) if path.name.starts_with('\''))
                    {
                        arguments.push(self.source_type(*arg, environment)?);
                    }
                }
                if path.declaration.is_none() && arguments.len() == 1 {
                    if path.name == "Vec" {
                        return Ok(self.shaped(Shape::Vec(arguments[0])));
                    }
                    if path.name == "Box" {
                        return Ok(self.shaped(Shape::Box(arguments[0])));
                    }
                }
                if let Some(declaration) = path.declaration {
                    if let a::Item::Alias(index) = self.source.declarations[declaration].item {
                        let mut aliases = environment.clone();
                        for (parameter, value) in parameters(self.source, declaration)
                            .into_iter()
                            .zip(arguments)
                        {
                            aliases.insert(parameter, value);
                        }
                        return self.source_type(self.source.aliases[index].ty, &aliases);
                    }
                    if matches!(
                        self.source.declarations[declaration].item,
                        a::Item::Struct(_) | a::Item::Enum(_)
                    ) {
                        self.shaped(Shape::Nominal(declaration, arguments))
                    } else {
                        self.shaped(Shape::Source(id))
                    }
                } else {
                    self.shaped(Shape::Source(id))
                }
            }
            a::Type::Function { .. } => self.blocked(),
        })
    }
    fn nominal_environment(
        &self,
        declaration: a::DeclarationId,
        args: &[Variable],
    ) -> BTreeMap<String, Variable> {
        parameters(self.source, declaration)
            .into_iter()
            .zip(args.iter().copied())
            .collect()
    }
    fn constructor(
        &mut self,
        path: &a::Path,
        fields: &[(String, Variable)],
        span: a::Span,
    ) -> Result<Option<Variable>> {
        let Some(declaration) = path.declaration else {
            return Ok(None);
        };
        let params = parameters(self.source, declaration);
        let mut arguments = Vec::new();
        let environment = self.environment.clone();
        for ty in &path.args {
            if !matches!(&self.source.types[*ty].kind, a::Type::Path(p) if p.name.starts_with('\''))
            {
                arguments.push(self.source_type(*ty, &environment)?);
            }
        }
        if arguments.is_empty() {
            for _ in &params {
                arguments.push(self.unknown());
            }
        }
        if arguments.len() != params.len() {
            return Ok(None);
        }
        let environment: BTreeMap<_, _> =
            params.into_iter().zip(arguments.iter().copied()).collect();
        let nominal = if let a::Item::Alias(index) = self.source.declarations[declaration].item {
            self.source_type(self.source.aliases[index].ty, &environment)?
        } else {
            self.shaped(Shape::Nominal(declaration, arguments))
        };
        let Some(Shape::Nominal(declaration, arguments)) = self.shape(nominal) else {
            return Ok(None);
        };
        let environment = self.nominal_environment(declaration, &arguments);
        let declared = match self.source.declarations[declaration].item {
            a::Item::Struct(index) => {
                if path.name != self.source.declarations[path.declaration.unwrap()].name {
                    return Ok(None);
                }
                self.source.structs[index].fields.clone()
            }
            a::Item::Enum(index) => {
                let Some((_, variant)) = path.name.rsplit_once("::") else {
                    return Ok(None);
                };
                let Some(variant) = self.source.enums[index]
                    .variants
                    .iter()
                    .find(|item| item.name == variant)
                else {
                    return Ok(None);
                };
                variant.fields.clone()
            }
            _ => return Ok(None),
        };
        for (name, value) in fields {
            if let Some(field) = declared.iter().find(|field| &field.name == name) {
                let expected = self.source_type(field.ty, &environment)?;
                let reason = if parameters(self.source, declaration).is_empty() {
                    "constructor field type differs"
                } else {
                    "generic arguments disagree for constructor field"
                };
                self.coerce(*value, expected, span)
                    .map_err(|problem| context(problem, reason))?;
            }
        }
        Ok(Some(nominal))
    }
    fn call_signature(
        &mut self,
        index: usize,
        type_args: &[a::TypeId],
        values: &[Variable],
        span: a::Span,
    ) -> Result<Variable> {
        let function = self.source.functions[index].clone();
        let mut environment = BTreeMap::new();
        let parameters: Vec<_> = function.generics.iter().filter(|p| !p.lifetime).collect();
        let outer = self.environment.clone();
        for (i, parameter) in parameters.iter().enumerate() {
            let value = if let Some(ty) = type_args.get(i) {
                self.source_type(*ty, &outer)?
            } else {
                self.unknown()
            };
            environment.insert(parameter.name.clone(), value);
        }
        for (parameter, value) in function.params.iter().zip(values) {
            let expected = self.source_type(parameter.ty, &environment)?;
            self.coerce(*value, expected, span)?;
        }
        self.source_type(function.result, &environment)
    }
    fn constrain_pattern(&mut self, pattern: a::PatternId, mut subject: Variable) -> Result<()> {
        while let Some(Shape::Ref(_, inner)) = self.shape(subject) {
            subject = inner;
        }
        let node = self.source.patterns[pattern].clone();
        match node.kind {
            a::Pattern::Literal(expression) => {
                let value = self.expr(expression)?;
                self.unify(subject, value, node.span)?;
            }
            a::Pattern::Range { start, end, .. } => {
                for expression in [start, end].into_iter().flatten() {
                    let value = self.expr(expression)?;
                    self.unify(subject, value, node.span)?;
                }
            }
            a::Pattern::Or(patterns) => {
                for pattern in patterns {
                    self.constrain_pattern(pattern, subject)?;
                }
            }
            a::Pattern::Ref { inner, .. } => self.constrain_pattern(inner, subject)?,
            _ => {}
        }
        Ok(())
    }
    fn bind_pattern(
        &mut self,
        pattern: a::PatternId,
        value: Variable,
        structural: bool,
    ) -> Result<()> {
        let node = self.source.patterns[pattern].clone();
        match node.kind {
            a::Pattern::Bind {
                name,
                by_ref,
                ref_mut,
                ..
            } => {
                let value = if by_ref {
                    self.shaped(Shape::Ref(ref_mut, value))
                } else {
                    value
                };
                self.bindings.push((name, value));
            }
            a::Pattern::Tuple(fields) if structural => {
                let values: Vec<_> = fields.iter().map(|_| self.unknown()).collect();
                let tuple = self.shaped(Shape::Tuple(values.clone()));
                self.unify(value, tuple, node.span)?;
                for (field, value) in fields.into_iter().zip(values) {
                    self.bind_pattern(field, value, true)?;
                }
            }
            a::Pattern::Ref { mutable, inner } if structural => {
                let pointee = self.unknown();
                let reference = self.shaped(Shape::Ref(mutable, pointee));
                self.unify(value, reference, node.span)?;
                self.bind_pattern(inner, pointee, true)?;
            }
            a::Pattern::Tuple(fields)
            | a::Pattern::Variant { fields, .. }
            | a::Pattern::Or(fields) => {
                for field in fields {
                    let value = self.blocked();
                    self.bind_pattern(field, value, false)?;
                }
            }
            a::Pattern::Struct { fields, .. } => {
                for (_, field) in fields {
                    let value = self.blocked();
                    self.bind_pattern(field, value, false)?;
                }
            }
            a::Pattern::Ref { inner, .. } => {
                let value = self.blocked();
                self.bind_pattern(inner, value, false)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn block(&mut self, id: a::BlockId) -> Result<Variable> {
        let block = self.source.blocks[id].clone();
        let bindings = self.bindings.len();
        let mut diverges = false;
        for statement in block.statements {
            match statement {
                a::Stmt::Let {
                    pattern,
                    ty,
                    value,
                    span,
                } => {
                    let environment = self.environment.clone();
                    let subject = match ty {
                        Some(ty) => self.source_type(ty, &environment)?,
                        None => self.unknown(),
                    };
                    if let Some(value) = value {
                        let value = self.expr(value)?;
                        self.coerce(value, subject, span)?;
                        diverges |= self.never(value);
                    }
                    self.definition.bindings.insert(pattern, subject);
                    self.bind_pattern(pattern, subject, true)?;
                }
                a::Stmt::Expr { value, .. } => {
                    let value = self.expr(value)?;
                    diverges |= self.never(value);
                }
                a::Stmt::Item(_) => {}
            }
        }
        let result = if let Some(tail) = block.tail {
            self.expr(tail)?
        } else {
            self.atom("()")
        };
        self.bindings.truncate(bindings);
        if diverges {
            Ok(self.shaped(Shape::Never))
        } else {
            Ok(result)
        }
    }
    fn expr(&mut self, id: a::ExprId) -> Result<Variable> {
        if let Some(value) = self.definition.expressions.get(&id) {
            return Ok(*value);
        }
        let node = self.source.exprs[id].clone();
        let span = node.span;
        let value = match node.kind {
            a::Expr::Int(text) => match integer_suffix(&text) {
                Some(suffix) => self.atom(suffix),
                None => self.fresh(None, true, false),
            },
            a::Expr::Bool(_) => self.atom("bool"),
            a::Expr::Float(_) => self.atom("f64"),
            a::Expr::String(_) => {
                let inner = self.atom("str");
                self.shaped(Shape::Ref(false, inner))
            }
            a::Expr::Bytes(bytes) => {
                let inner = self.atom("u8");
                let array = self.shaped(Shape::Array(inner, bytes.len()));
                self.shaped(Shape::Ref(false, array))
            }
            a::Expr::Path(path) => {
                if let Some(value) = self.bindings.get(&path.name) {
                    value
                } else {
                    match self.names.value_path(node.scope, &path, span) {
                        Ok(path) => {
                            if let Some(declaration) = path.declaration {
                                if let a::Item::Constant(index) =
                                    self.source.declarations[declaration].item
                                {
                                    let environment = self.environment.clone();
                                    self.source_type(self.source.constants[index].ty, &environment)?
                                } else {
                                    self.constructor(&path, &[], span)?
                                        .unwrap_or_else(|| self.blocked())
                                }
                            } else {
                                self.blocked()
                            }
                        }
                        Err(_) => self.blocked(),
                    }
                }
            }
            a::Expr::Tuple(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(self.expr(item)?);
                }
                if values.is_empty() {
                    self.atom("()")
                } else {
                    self.shaped(Shape::Tuple(values))
                }
            }
            a::Expr::Array(items) => {
                let element = self.unknown();
                let len = items.len();
                for item in items {
                    let value = self.expr(item)?;
                    self.coerce(value, element, span)?;
                }
                self.shaped(Shape::Array(element, len))
            }
            a::Expr::Repeat { value, count } => {
                let value = self.expr(value)?;
                let count_value = self.expr(count)?;
                let usize = self.atom("usize");
                self.unify(count_value, usize, span)?;
                if let a::Expr::Int(text) = &self.source.exprs[count].kind {
                    let text = text.trim_end_matches("usize").replace('_', "");
                    if let Ok(len) = text.parse() {
                        self.shaped(Shape::Array(value, len))
                    } else {
                        self.blocked()
                    }
                } else {
                    self.blocked()
                }
            }
            a::Expr::Unary { op, value } => {
                let value = self.expr(value)?;
                if op == "*" {
                    let result = self.unknown();
                    self.obligation(Obligation::Deref {
                        source: value,
                        result,
                        span,
                    });
                    result
                } else {
                    value
                }
            }
            a::Expr::Binary { op, left, right } => {
                let left = self.expr(left)?;
                let right = self.expr(right)?;
                if op == "&&" || op == "||" {
                    let boolean = self.atom("bool");
                    self.unify(left, boolean, span)?;
                    self.unify(right, boolean, span)?;
                    boolean
                } else if op == "==" || op == "!=" {
                    // Equality's trait/autoderef selection is not an inference rule.
                    if !matches!(self.shape(left), Some(Shape::Ref(..)))
                        && !matches!(self.shape(right), Some(Shape::Ref(..)))
                    {
                        self.unify(left, right, span)
                            .map_err(|problem| context(problem, "equality operand types differ"))?;
                    }
                    self.atom("bool")
                } else {
                    self.unify(left, right, span)?;
                    if matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
                        self.atom("bool")
                    } else {
                        left
                    }
                }
            }
            a::Expr::Assign { place, value, .. } => {
                let place = self.expr(place)?;
                let value = self.expr(value)?;
                self.coerce(value, place, span)?;
                self.atom("()")
            }
            a::Expr::Borrow { mutable, value } => {
                let value = self.expr(value)?;
                self.shaped(Shape::Ref(mutable, value))
            }
            a::Expr::Cast { value, ty } => {
                self.expr(value)?;
                let environment = self.environment.clone();
                self.source_type(ty, &environment)?
            }
            a::Expr::Call { callee, args } => {
                let mut values = Vec::new();
                for arg in &args {
                    values.push(self.expr(*arg)?);
                }
                let path = match &self.source.exprs[callee].kind {
                    a::Expr::Path(path) if self.bindings.get(&path.name).is_none() => self
                        .names
                        .value_path(self.source.exprs[callee].scope, path, span)
                        .ok(),
                    _ => None,
                };
                if let Some(path) = path {
                    if path.declaration.is_none()
                        && matches!(path.name.as_str(), "Vec::new" | "Box::new")
                    {
                        let environment = self.environment.clone();
                        let inner = match path.args.first() {
                            Some(ty) => self.source_type(*ty, &environment)?,
                            None => self.unknown(),
                        };
                        if path.name == "Box::new" {
                            if let Some(value) = values.first() {
                                self.coerce(*value, inner, span)?;
                            }
                            self.shaped(Shape::Box(inner))
                        } else {
                            self.shaped(Shape::Vec(inner))
                        }
                    } else if path.declaration.is_none()
                        && matches!(path.name.as_str(), "drop" | "std::mem::drop")
                    {
                        self.atom("()")
                    } else if let Some(a::Item::Function(index)) =
                        path.declaration.map(|id| self.source.declarations[id].item)
                    {
                        self.call_signature(index, &path.args, &values, span)?
                    } else {
                        let fields: Vec<_> = values
                            .iter()
                            .enumerate()
                            .map(|(i, v)| (i.to_string(), *v))
                            .collect();
                        self.constructor(&path, &fields, span)?
                            .unwrap_or_else(|| self.blocked())
                    }
                } else {
                    self.blocked()
                }
            }
            a::Expr::Method {
                receiver,
                name,
                types,
                args,
            } => {
                let receiver = self.expr(receiver)?;
                let mut arguments = Vec::new();
                for arg in args {
                    arguments.push(self.expr(arg)?);
                }
                let result = self.blocked();
                if types.is_empty() {
                    self.obligation(Obligation::Method {
                        receiver,
                        name,
                        arguments,
                        result,
                        span,
                    });
                }
                result
            }
            a::Expr::Field { value, name } => {
                let source = self.expr(value)?;
                let result = self.unknown();
                self.obligation(Obligation::Field {
                    source,
                    name,
                    result,
                    span,
                });
                result
            }
            a::Expr::Index { value, index } => {
                let source = self.expr(value)?;
                let index = self.expr(index)?;
                let usize = self.atom("usize");
                self.unify(index, usize, span)?;
                let result = self.unknown();
                self.obligation(Obligation::Index {
                    source,
                    result,
                    span,
                });
                result
            }
            a::Expr::Struct { path, fields, base } => {
                let mut values = Vec::new();
                for (name, value) in fields {
                    values.push((name, self.expr(value)?));
                }
                let value = self
                    .constructor(&path, &values, span)?
                    .unwrap_or_else(|| self.blocked());
                if let Some(base) = base {
                    let base = self.expr(base)?;
                    self.unify(base, value, span)?;
                }
                value
            }
            a::Expr::Block(block) => self.block(block)?,
            a::Expr::If {
                condition,
                then_block,
                otherwise,
            } => {
                let condition = self.expr(condition)?;
                let boolean = self.atom("bool");
                self.unify(condition, boolean, span)?;
                let first = self.block(then_block)?;
                let second = if let Some(otherwise) = otherwise {
                    self.expr(otherwise)?
                } else {
                    self.atom("()")
                };
                self.coerce(first, second, span)?;
                if self.never(first) { second } else { first }
            }
            a::Expr::While { condition, body } => {
                let condition = self.expr(condition)?;
                let boolean = self.atom("bool");
                self.unify(condition, boolean, span)?;
                self.block(body)?;
                self.atom("()")
            }
            a::Expr::Loop { body } => {
                self.block(body)?;
                self.blocked()
            }
            a::Expr::For {
                pattern,
                iterator,
                body,
            } => {
                let bindings = self.bindings.len();
                let item = if let a::Expr::Range { start, end, .. } =
                    self.source.exprs[iterator].kind.clone()
                {
                    let item = self.fresh(None, true, false);
                    if let Some(start) = start {
                        let start = self.expr(start)?;
                        self.unify(item, start, span)?;
                    }
                    if let Some(end) = end {
                        let end = self.expr(end)?;
                        self.unify(item, end, span)?;
                    }
                    item
                } else {
                    self.expr(iterator)?;
                    self.blocked()
                };
                self.bind_pattern(pattern, item, true)?;
                self.block(body)?;
                self.bindings.truncate(bindings);
                self.atom("()")
            }
            a::Expr::Match { value, arms } => {
                let subject = self.expr(value)?;
                let result = self.unknown();
                let mut all_never = true;
                for arm in arms {
                    self.constrain_pattern(arm.pattern, subject)?;
                    let bindings = self.bindings.len();
                    let value = self.blocked();
                    self.bind_pattern(arm.pattern, value, false)?;
                    if let Some(guard) = arm.guard {
                        let guard = self.expr(guard)?;
                        let boolean = self.atom("bool");
                        self.unify(guard, boolean, arm.span)?;
                    }
                    let value = self.expr(arm.value)?;
                    all_never &= self.never(value);
                    self.coerce(value, result, arm.span)?;
                    self.bindings.truncate(bindings);
                }
                if all_never {
                    self.shaped(Shape::Never)
                } else {
                    result
                }
            }
            a::Expr::Return(value) => {
                if let Some(value) = value {
                    let value = self.expr(value)?;
                    self.coerce(value, self.result, span)
                        .map_err(|problem| context(problem, "function result type differs"))?;
                }
                self.shaped(Shape::Never)
            }
            a::Expr::Break(value) => {
                if let Some(value) = value {
                    self.expr(value)?;
                }
                self.shaped(Shape::Never)
            }
            a::Expr::Continue => self.shaped(Shape::Never),
            a::Expr::Macro { args, .. } => {
                for arg in args {
                    self.expr(arg)?;
                }
                self.blocked()
            }
            a::Expr::Try(value) => {
                self.expr(value)?;
                self.blocked()
            }
            a::Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    self.expr(start)?;
                }
                if let Some(end) = end {
                    self.expr(end)?;
                }
                self.blocked()
            }
            _ => self.blocked(),
        };
        self.definition.expressions.insert(id, value);
        Ok(value)
    }
    fn solve_obligation(&mut self, id: usize, obligation: Obligation) -> Result<bool> {
        match obligation {
            Obligation::Deref {
                source,
                result,
                span,
            } => match self.shape(source) {
                Some(Shape::Ref(_, inner) | Shape::Box(inner)) => {
                    self.unify(result, inner, span)?
                }
                None => {
                    self.watch(source, id);
                    return Ok(false);
                }
                _ => return Ok(false),
            },
            Obligation::Index {
                source,
                result,
                span,
            } => {
                let Some((_, shape)) = self.strip_references(source, id) else {
                    return Ok(false);
                };
                match shape {
                    Shape::Vec(inner) | Shape::Array(inner, _) | Shape::Slice(inner) => {
                        self.unify(result, inner, span)?
                    }
                    _ => return Ok(false),
                }
            }
            Obligation::Field {
                mut source,
                name,
                result,
                span,
            } => {
                let shape = loop {
                    match self.shape(source) {
                        Some(Shape::Ref(_, inner) | Shape::Box(inner)) => source = inner,
                        Some(shape) => break shape,
                        None => {
                            self.watch(source, id);
                            return Ok(false);
                        }
                    }
                };
                match shape {
                    Shape::Tuple(fields) => {
                        let Some(field) = name
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| fields.get(i))
                            .copied()
                        else {
                            return Ok(false);
                        };
                        self.unify(result, field, span)?;
                    }
                    Shape::Nominal(declaration, args) => {
                        let a::Item::Struct(index) = self.source.declarations[declaration].item
                        else {
                            return Ok(false);
                        };
                        let Some(field) = self.source.structs[index]
                            .fields
                            .iter()
                            .find(|field| field.name == name)
                            .cloned()
                        else {
                            return Ok(false);
                        };
                        let environment = self.nominal_environment(declaration, &args);
                        let ty = self.source_type(field.ty, &environment)?;
                        self.unify(result, ty, span)?;
                    }
                    _ => return Ok(false),
                }
            }
            Obligation::Method {
                receiver,
                name,
                arguments,
                result,
                span,
            } => {
                let Some((_, shape)) = self.strip_references(receiver, id) else {
                    return Ok(false);
                };
                match (name.as_str(), shape) {
                    ("len", Shape::Vec(_) | Shape::Slice(_) | Shape::Array(_, _))
                        if arguments.is_empty() =>
                    {
                        let usize = self.atom("usize");
                        self.unify(result, usize, span)?;
                    }
                    ("push", Shape::Vec(inner)) if arguments.len() == 1 => {
                        self.coerce(arguments[0], inner, span)?;
                        let unit = self.atom("()");
                        self.unify(result, unit, span)?;
                    }
                    ("as_slice" | "as_mut_slice", Shape::Vec(inner) | Shape::Array(inner, _))
                        if arguments.is_empty() =>
                    {
                        let slice = self.shaped(Shape::Slice(inner));
                        let reference = self.shaped(Shape::Ref(name == "as_mut_slice", slice));
                        self.unify(result, reference, span)?;
                    }
                    ("len", Shape::Atom(atom)) if atom == "str" && arguments.is_empty() => {
                        let usize = self.atom("usize");
                        self.unify(result, usize, span)?;
                    }
                    ("is_empty", Shape::Atom(atom)) if atom == "str" && arguments.is_empty() => {
                        let boolean = self.atom("bool");
                        self.unify(result, boolean, span)?;
                    }
                    ("as_bytes", Shape::Atom(atom)) if atom == "str" && arguments.is_empty() => {
                        let byte = self.atom("u8");
                        let slice = self.shaped(Shape::Slice(byte));
                        let reference = self.shaped(Shape::Ref(false, slice));
                        self.unify(result, reference, span)?;
                    }
                    _ => return Ok(false),
                }
            }
        }
        Ok(true)
    }
    fn solve(&mut self) -> Result<()> {
        while let Some(id) = self.queue.pop_front() {
            self.queued[id] = false;
            if let Some(obligation) = self.obligations[id].clone() {
                if self.solve_obligation(id, obligation)? {
                    self.obligations[id] = None;
                }
            }
        }
        Ok(())
    }
    fn finish(mut self) -> Result<DefinitionTypes> {
        self.solve()?;
        // All known source constraints get a chance to propagate before numeric
        // defaults. A blocked operation supplies no guessed method signature.
        let span = a::Span::default();
        let mut defaults = Vec::new();
        for id in 0..self.definition.nodes.len() {
            let node = &self.definition.nodes[id];
            if node.parent == id && node.integer && !node.blocked && node.shape.is_none() {
                defaults.push(id);
            }
        }
        for id in defaults {
            let integer = self.atom("i32");
            self.unify(id, integer, span)?;
        }
        self.solve()?;
        Ok(self.definition)
    }
}

pub(super) fn infer(
    source: &a::Program,
    names: &crate::resolve::Resolver<'_>,
    index: usize,
) -> Result<DefinitionTypes> {
    let function = source.functions[index].clone();
    let mut inference = Infer {
        source,
        names,
        definition: DefinitionTypes {
            nodes: Vec::new(),
            expressions: BTreeMap::new(),
            bindings: BTreeMap::new(),
        },
        bindings: Bindings::default(),
        environment: BTreeMap::new(),
        result: 0,
        waiting: Vec::new(),
        obligations: Vec::new(),
        queue: VecDeque::new(),
        queued: Vec::new(),
    };
    for implementation in &source.impls {
        if implementation.methods.contains(&index) {
            for parameter in &implementation.generics {
                if !parameter.lifetime {
                    let variable = inference.shaped(Shape::Rigid(parameter.name.clone()));
                    inference
                        .environment
                        .insert(parameter.name.clone(), variable);
                }
            }
            let environment = inference.environment.clone();
            let receiver = inference.source_type(implementation.target, &environment)?;
            inference.environment.insert("Self".to_string(), receiver);
        }
    }
    for definition in &source.traits {
        if definition.methods.contains(&index) {
            let receiver = inference.shaped(Shape::Rigid("Self".to_string()));
            inference.environment.insert("Self".to_string(), receiver);
            for parameter in &definition.generics {
                if !parameter.lifetime {
                    let variable = inference.shaped(Shape::Rigid(parameter.name.clone()));
                    inference
                        .environment
                        .insert(parameter.name.clone(), variable);
                }
            }
        }
    }
    for parameter in &function.generics {
        if !parameter.lifetime {
            let variable = inference.shaped(Shape::Rigid(parameter.name.clone()));
            inference
                .environment
                .insert(parameter.name.clone(), variable);
        }
    }
    let environment = inference.environment.clone();
    inference.result = inference.source_type(function.result, &environment)?;
    for parameter in &function.params {
        let ty = inference.source_type(parameter.ty, &environment)?;
        inference.bindings.push((parameter.name.clone(), ty));
    }
    if let Some(body) = function.body {
        let body = inference.block(body)?;
        inference
            .coerce(body, inference.result, function.span)
            .map_err(|problem| context(problem, "function result type differs"))?;
    }
    inference.finish()
}
