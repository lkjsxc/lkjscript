//! Definition-local inference. The graph records structural type constraints
//! before K construction; integer fallback happens only after those constraints
//! have propagated. Method selection and ownership remain checked separately.
use super::{Bindings, Compiler, Result, a, err, is_copy, k};
use std::collections::{BTreeMap, VecDeque};

mod closures;
mod source_types;

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
    Closure(a::ExprId, Vec<Variable>),
    Never,
}
impl Shape {
    fn children(&self) -> Vec<Variable> {
        match self {
            Self::Vec(v) | Self::Box(v) | Self::Ref(_, v) | Self::Slice(v) | Self::Array(v, _) => {
                vec![*v]
            }
            Self::Tuple(v) | Self::Nominal(_, v) | Self::Closure(_, v) => v.clone(),
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
    // Expression operations establish their own result types before coercions.
    producers: usize,
}

pub(super) struct DefinitionTypes {
    nodes: Vec<Node>,
    expressions: BTreeMap<a::ExprId, Variable>,
    bindings: BTreeMap<a::PatternId, Variable>,
    closures: BTreeMap<a::ExprId, closures::Closure>,
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
                Shape::Closure(expression, _) => {
                    let closure = &self.closures[&expression];
                    let capture_count = closure.captures.len();
                    let captures = closure
                        .captures
                        .iter()
                        .zip(&values)
                        .map(|(capture, ty)| super::closures::VariableCapture {
                            name: capture.name.clone(),
                            ty: *ty,
                            uses: capture.uses.clone(),
                        })
                        .collect();
                    Some(compiler.closure_type(
                        expression,
                        closure.source_function,
                        substitutions,
                        captures,
                        values[capture_count..values.len() - 1].to_vec(),
                        *values.last().unwrap(),
                    ))
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
    Callable {
        receiver: Variable,
        arguments: Vec<Variable>,
        result: Variable,
        span: a::Span,
        produced: bool,
    },
    Coercion {
        source: Variable,
        target: Variable,
        span: a::Span,
    },
    Try {
        target: Variable,
        source: Variable,
        result: Variable,
        span: a::Span,
    },
    Deref {
        source: Variable,
        result: Variable,
        span: a::Span,
    },
    Index {
        source: Variable,
        result: Variable,
        range: bool,
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
    source_function: usize,
    capture_frames: Vec<closures::Frame>,
    callable_parameters: BTreeMap<String, (Vec<Variable>, Variable)>,
    source: &'a a::Program,
    names: &'b crate::resolve::Resolver<'a>,
    definition: DefinitionTypes,
    bindings: Bindings,
    environment: BTreeMap<String, Variable>,
    result: Variable,
    waiting: Vec<Vec<usize>>,
    obligations: Vec<Option<Obligation>>,
    queue: VecDeque<usize>,
    fallback: VecDeque<usize>,
    queued: Vec<bool>,
    field_types: BTreeMap<a::DeclarationId, BTreeMap<String, a::TypeId>>,
    variant_fields: BTreeMap<a::DeclarationId, BTreeMap<String, Vec<a::Field>>>,
    loops: Vec<(Variable, bool, bool)>,
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
            producers: if blocked { 1 } else { 0 },
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
    fn operation_type(&mut self) -> Variable {
        let id = self.unknown();
        self.definition.nodes[id].producers = 1;
        id
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
        self.definition.nodes[a].producers += self.definition.nodes[b].producers;
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
    fn pending_operation(&self, mut variable: Variable) -> bool {
        loop {
            let root = self.definition.root(variable);
            match self.shape(root) {
                None => return self.definition.nodes[root].producers != 0,
                Some(Shape::Ref(_, inner) | Shape::Box(inner)) => variable = inner,
                _ => return false,
            }
        }
    }
    fn coercion_pending(&self, from: Variable, to: Variable) -> bool {
        // A known scalar target fixes the type even when its producer is still
        // awaiting a method/projection contract. No reference coercion can
        // change that choice; propagate it before numeric fallback.
        if matches!(self.shape(to), Some(Shape::Atom(_))) {
            return false;
        }
        if self.pending_operation(from) || self.pending_operation(to) {
            return true;
        }
        match (self.shape(from), self.shape(to)) {
            (Some(Shape::Ref(_, _)), None) => true,
            (Some(Shape::Ref(_, _)), Some(Shape::Ref(_, inner))) => self.shape(inner).is_none(),
            _ => false,
        }
    }
    fn coerce(&mut self, from: Variable, to: Variable, span: a::Span) -> Result<()> {
        if self.coercion_pending(from, to) {
            self.obligation(Obligation::Coercion {
                source: from,
                target: to,
                span,
            });
            return Ok(());
        }
        self.coerce_known(from, to, span)
    }
    fn coerce_known(&mut self, from: Variable, to: Variable, span: a::Span) -> Result<()> {
        if let (Some(Shape::Ref(fm, from)), Some(Shape::Ref(tm, to))) =
            (self.shape(from), self.shape(to))
        {
            if tm && !fm {
                return Ok(());
            } // K/type checking diagnoses an invalid mutable coercion.
            return self.coerce_referents(from, to, span);
        }
        self.unify(from, to, span)
    }
    fn finish_operation(&mut self, variable: Variable) {
        let root = self.root(variable);
        self.definition.nodes[root].producers -= 1;
        if self.definition.nodes[root].producers == 0 {
            let waiting = std::mem::take(&mut self.waiting[root]);
            for id in waiting {
                self.enqueue(id);
            }
        }
    }
    fn coerce_referents(&mut self, mut from: Variable, to: Variable, span: a::Span) -> Result<()> {
        if let Some(Shape::Slice(element)) = self.shape(to) {
            if let Some(Shape::Vec(source) | Shape::Array(source, _)) = self.shape(from) {
                return self.unify(source, element, span);
            }
        }
        while let Some(Shape::Box(inner)) = self.shape(from) {
            if matches!(
                self.shape(to),
                None | Some(Shape::Box(_) | Shape::Rigid(_) | Shape::Source(_))
            ) {
                break;
            }
            from = inner;
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
    fn struct_field_type(
        &mut self,
        declaration: a::DeclarationId,
        name: &str,
    ) -> Option<a::TypeId> {
        if !self.field_types.contains_key(&declaration) {
            let a::Item::Struct(index) = self.source.declarations[declaration].item else {
                return None;
            };
            self.field_types.insert(
                declaration,
                self.source.structs[index]
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.ty))
                    .collect(),
            );
        }
        self.field_types[&declaration].get(name).copied()
    }
    fn enum_variant_fields(
        &mut self,
        declaration: a::DeclarationId,
        name: &str,
    ) -> Option<Vec<a::Field>> {
        if !self.variant_fields.contains_key(&declaration) {
            let a::Item::Enum(index) = self.source.declarations[declaration].item else {
                return None;
            };
            self.variant_fields.insert(
                declaration,
                self.source.enums[index]
                    .variants
                    .iter()
                    .map(|variant| (variant.name.clone(), variant.fields.clone()))
                    .collect(),
            );
        }
        self.variant_fields[&declaration].get(name).cloned()
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
            a::Item::Enum(_) => {
                let Some((_, variant)) = path.name.rsplit_once("::") else {
                    return Ok(None);
                };
                let Some(fields) = self.enum_variant_fields(declaration, variant) else {
                    return Ok(None);
                };
                fields
            }
            _ => return Ok(None),
        };
        let declared: BTreeMap<_, _> = declared
            .into_iter()
            .map(|field| (field.name.clone(), field))
            .collect();
        let generic = !parameters(self.source, declaration).is_empty();
        for (name, value) in fields {
            if let Some(field) = declared.get(name) {
                let expected = self.source_type(field.ty, &environment)?;
                let reason = if !generic {
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
        for parameter in &function.generics {
            if let Some(receiver) = environment.get(&parameter.name).copied() {
                for bound in &parameter.bounds {
                    self.callable_bound(receiver, *bound, &environment)?;
                }
            }
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
        _structural: bool,
    ) -> Result<()> {
        self.bind_pattern_mode(pattern, value, None)
    }
    fn bind_pattern_mode(
        &mut self,
        pattern: a::PatternId,
        mut value: Variable,
        mut mode: Option<bool>,
    ) -> Result<()> {
        let node = self.source.patterns[pattern].clone();
        // Structural patterns follow references and carry the resulting binding
        // mode down to their fields; a bare binding preserves its subject type.
        if matches!(
            node.kind,
            a::Pattern::Tuple(_)
                | a::Pattern::Variant { .. }
                | a::Pattern::Struct { .. }
                | a::Pattern::Path(_)
        ) {
            while let Some(Shape::Ref(mutable, inner)) = self.shape(value) {
                value = inner;
                mode = Some(mode.unwrap_or(true) && mutable);
            }
        }
        match node.kind {
            a::Pattern::Bind {
                name,
                by_ref,
                ref_mut,
                ..
            } => {
                let reference = if by_ref { Some(ref_mut) } else { mode };
                let value = if let Some(mutable) = reference {
                    self.shaped(Shape::Ref(mutable, value))
                } else {
                    value
                };
                self.bindings.push((name, value));
            }
            a::Pattern::Tuple(fields) => {
                let rest = fields
                    .iter()
                    .any(|field| matches!(self.source.patterns[*field].kind, a::Pattern::Rest));
                let values = if rest {
                    match self.shape(value) {
                        Some(Shape::Tuple(values)) => values,
                        Some(Shape::Atom(name)) if name == "()" => Vec::new(),
                        None => {
                            // A method without a known result contract stays
                            // blocked; concrete pattern construction checks arity.
                            for field in fields {
                                if !matches!(self.source.patterns[field].kind, a::Pattern::Rest) {
                                    let unknown = self.blocked();
                                    self.bind_pattern_mode(field, unknown, mode)?;
                                }
                            }
                            return Ok(());
                        }
                        _ => {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "tuple rest pattern requires a tuple",
                            ));
                        }
                    }
                } else {
                    fields.iter().map(|_| self.unknown()).collect()
                };
                let tuple = if values.is_empty() {
                    self.atom("()")
                } else {
                    self.shaped(Shape::Tuple(values.clone()))
                };
                self.unify(value, tuple, node.span)?;
                for (index, field) in super::patterns::positional_fields(
                    self.source,
                    &fields,
                    values.len(),
                    node.span,
                )? {
                    self.bind_pattern_mode(field, values[index], mode)?;
                }
            }
            a::Pattern::Ref { mutable, inner } => {
                let pointee = self.unknown();
                let reference = self.shaped(Shape::Ref(mutable, pointee));
                self.unify(value, reference, node.span)?;
                self.bind_pattern_mode(inner, pointee, None)?;
            }
            a::Pattern::Variant { path, fields } => {
                let path = if path.declaration.is_some() {
                    path
                } else {
                    self.names.value_path(node.scope, &path, node.span)?
                };
                let count = if let Some(nominal) = self.constructor(&path, &[], node.span)? {
                    if let Some(Shape::Nominal(declaration, _)) = self.shape(nominal) {
                        match self.source.declarations[declaration].item {
                            a::Item::Struct(index) => self.source.structs[index].fields.len(),
                            a::Item::Enum(_) => path
                                .name
                                .rsplit_once("::")
                                .and_then(|(_, name)| self.enum_variant_fields(declaration, name))
                                .map_or(0, |fields| fields.len()),
                            _ => fields.len(),
                        }
                    } else {
                        fields.len()
                    }
                } else {
                    fields.len()
                };
                let fields =
                    super::patterns::positional_fields(self.source, &fields, count, node.span)?
                        .into_iter()
                        .map(|(index, pattern)| (index.to_string(), pattern))
                        .collect::<Vec<_>>();
                self.bind_nominal_pattern(&path, &fields, value, mode, node.scope, node.span)?;
            }
            a::Pattern::Struct { path, fields, .. } => {
                self.bind_nominal_pattern(&path, &fields, value, mode, node.scope, node.span)?;
            }
            a::Pattern::Path(path) => {
                self.bind_nominal_pattern(&path, &[], value, mode, node.scope, node.span)?;
            }
            a::Pattern::Or(alternatives) => {
                let scope = self.bindings.len();
                let mut first = None::<BTreeMap<String, Variable>>;
                for alternative in alternatives {
                    self.bindings.truncate(scope);
                    self.bind_pattern_mode(alternative, value, mode)?;
                    let bindings: BTreeMap<_, _> =
                        self.bindings.entries[scope..].iter().cloned().collect();
                    if let Some(first) = &first {
                        if first.len() != bindings.len() || first.keys().ne(bindings.keys()) {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "or-pattern alternatives must bind the same names",
                            ));
                        }
                        for (name, ty) in &bindings {
                            self.unify(*ty, first[name], node.span)?;
                        }
                    } else {
                        first = Some(bindings);
                    }
                }
                self.bindings.truncate(scope);
                for binding in first.unwrap_or_default() {
                    self.bindings.push(binding);
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn bind_nominal_pattern(
        &mut self,
        path: &a::Path,
        fields: &[(String, a::PatternId)],
        value: Variable,
        mode: Option<bool>,
        scope: a::ModuleId,
        span: a::Span,
    ) -> Result<()> {
        let path = if path.declaration.is_some() {
            path.clone()
        } else {
            self.names.value_path(scope, path, span)?
        };
        let Some(nominal) = self.constructor(&path, &[], span)? else {
            for (_, pattern) in fields {
                let value = self.blocked();
                self.bind_pattern_mode(*pattern, value, mode)?;
            }
            return Ok(());
        };
        self.unify(value, nominal, span)?;
        let Some(Shape::Nominal(declaration, arguments)) = self.shape(nominal) else {
            return Ok(());
        };
        let environment = self.nominal_environment(declaration, &arguments);
        let variant = if matches!(self.source.declarations[declaration].item, a::Item::Enum(_)) {
            path.name
                .rsplit_once("::")
                .and_then(|(_, name)| self.enum_variant_fields(declaration, name))
                .map(|fields| {
                    fields
                        .into_iter()
                        .map(|field| (field.name, field.ty))
                        .collect::<BTreeMap<_, _>>()
                })
        } else {
            None
        };
        for (name, pattern) in fields {
            let declared = if let Some(variant) = &variant {
                variant.get(name).copied()
            } else {
                self.struct_field_type(declaration, name)
            };
            let field = match declared {
                Some(ty) => self.source_type(ty, &environment)?,
                None => self.blocked(),
            };
            self.bind_pattern_mode(*pattern, field, mode)?;
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
                    let initializer = value.map(|value| self.expr(value)).transpose()?;
                    let subject = match ty {
                        Some(ty) => {
                            let subject = self.source_type(ty, &environment)?;
                            if let Some(value) = initializer {
                                self.coerce(value, subject, span)?;
                            }
                            subject
                        }
                        None => initializer.unwrap_or_else(|| self.unknown()),
                    };
                    if let Some(value) = initializer {
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
            a::Expr::Char(_) => self.atom("char"),
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
                self.capture_path(id, &path.name);
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
                    let result = self.operation_type();
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
                    let receiver = self.expr(callee)?;
                    let result = self.operation_type();
                    self.obligation(Obligation::Callable {
                        receiver,
                        arguments: values,
                        result,
                        span,
                        produced: true,
                    });
                    result
                }
            }
            a::Expr::Closure {
                params,
                patterns,
                result,
                body,
                ..
            } => self.closure_expression(id, &params, &patterns, result, body)?,
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
                let result = self.operation_type();
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
                let usize = self.atom("usize");
                let range = if let a::Expr::Range { start, end, .. } =
                    self.source.exprs[index].kind.clone()
                {
                    for bound in [start, end].into_iter().flatten() {
                        let bound = self.expr(bound)?;
                        self.unify(bound, usize, span)?;
                    }
                    true
                } else {
                    let index = self.expr(index)?;
                    self.unify(index, usize, span)?;
                    false
                };
                let result = self.operation_type();
                self.obligation(Obligation::Index {
                    source,
                    result,
                    range,
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
                let unit = self.atom("()");
                self.loops.push((unit, false, false));
                self.block(body)?;
                self.loops.pop();
                unit
            }
            a::Expr::Loop { body } => {
                let result = self.unknown();
                self.loops.push((result, true, false));
                self.block(body)?;
                let (_, _, has_break) = self.loops.pop().unwrap();
                if has_break {
                    result
                } else {
                    self.shaped(Shape::Never)
                }
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
                let unit = self.atom("()");
                self.loops.push((unit, false, false));
                self.block(body)?;
                self.loops.pop();
                self.bindings.truncate(bindings);
                unit
            }
            a::Expr::Match { value, arms } => {
                let subject = self.expr(value)?;
                let result = self.unknown();
                let mut all_never = true;
                for arm in arms {
                    self.constrain_pattern(arm.pattern, subject)?;
                    let bindings = self.bindings.len();
                    self.bind_pattern(arm.pattern, subject, true)?;
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
                let Some(&(result, allow_value, _)) = self.loops.last() else {
                    return Err(err(span, "E_CONTROL", "break outside a loop"));
                };
                if value.is_some() && !allow_value {
                    return Err(err(
                        span,
                        "E_TYPE",
                        "break value requires a loop expression",
                    ));
                }
                let value = if let Some(value) = value {
                    self.expr(value)?
                } else {
                    self.atom("()")
                };
                self.coerce(value, result, span)?;
                self.loops.last_mut().unwrap().2 = true;
                self.shaped(Shape::Never)
            }
            a::Expr::Continue => self.shaped(Shape::Never),
            a::Expr::Macro {
                name,
                args,
                separator,
            } if name == "vec" => {
                let element = self.unknown();
                if separator == ";" {
                    if args.len() != 2 {
                        return Err(err(
                            span,
                            "E_TYPE",
                            "Vec repetition requires an element and count",
                        ));
                    }
                    let value = self.expr(args[0])?;
                    self.coerce(value, element, span)?;
                    let count = self.expr(args[1])?;
                    let size = self.atom("usize");
                    self.coerce(count, size, span)?;
                } else {
                    for arg in args {
                        let value = self.expr(arg)?;
                        self.coerce(value, element, span)?;
                    }
                }
                self.shaped(Shape::Vec(element))
            }
            a::Expr::Macro { name, args, .. } => {
                for arg in args {
                    self.expr(arg)?;
                }
                if name == "panic" || name == "unreachable" {
                    self.shaped(Shape::Never)
                } else {
                    self.blocked()
                }
            }
            a::Expr::Try(value) => {
                let source = self.expr(value)?;
                let result = self.operation_type();
                self.obligation(Obligation::Try {
                    target: self.result,
                    source,
                    result,
                    span,
                });
                result
            }
            a::Expr::Diverge(value) => {
                let value = self.expr(value)?;
                if self.shape(value).is_some() && !self.never(value) {
                    return Err(err(span, "E_TYPE", "let-else branch must diverge"));
                }
                self.shaped(Shape::Never)
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
            Obligation::Callable {
                receiver,
                arguments,
                result,
                span,
                ..
            } => return self.solve_callable(id, receiver, arguments, result, span),
            Obligation::Coercion {
                source,
                target,
                span,
            } => {
                if self.coercion_pending(source, target) {
                    for mut variable in [source, target] {
                        loop {
                            self.watch(variable, id);
                            match self.shape(variable) {
                                Some(Shape::Ref(_, inner) | Shape::Box(inner)) => variable = inner,
                                _ => break,
                            }
                        }
                    }
                    if !self.pending_operation(source) && !self.pending_operation(target) {
                        self.fallback.push_back(id);
                    }
                    return Ok(false);
                }
                self.coerce_known(source, target, span)?;
            }
            Obligation::Try {
                target,
                source,
                result,
                span,
            } => {
                let Some(shape) = self.shape(source) else {
                    self.watch(source, id);
                    return Ok(false);
                };
                let Shape::Nominal(declaration, arguments) = shape else {
                    if matches!(shape, Shape::Rigid(_) | Shape::Source(_)) {
                        return Ok(false);
                    }
                    return Err(err(
                        span,
                        "E_TYPE",
                        "question-mark requires Result or Option",
                    ));
                };
                let name = self.source.declarations[declaration].name.as_str();
                let arity = match name {
                    "$std::prelude::Result" => 2,
                    "$std::prelude::Option" => 1,
                    _ => {
                        return Err(err(
                            span,
                            "E_TYPE",
                            "question-mark requires the source-library Result or Option",
                        ));
                    }
                };
                if arguments.len() != arity {
                    return Err(err(span, "E_TYPE", "question-mark type arguments differ"));
                }
                let Some(output_shape) = self.shape(target) else {
                    self.watch(target, id);
                    return Ok(false);
                };
                let Shape::Nominal(output, output_arguments) = output_shape else {
                    return Err(err(
                        span,
                        "E_TYPE",
                        "question-mark requires the same return enum",
                    ));
                };
                if output != declaration || output_arguments.len() != arity {
                    return Err(err(
                        span,
                        "E_TYPE",
                        "question-mark requires the same return enum",
                    ));
                }
                self.unify(result, arguments[0], span)?;
                if arity == 2 {
                    self.unify(arguments[1], output_arguments[1], span)?;
                }
            }
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
                range,
                span,
            } => {
                let Some((_, shape)) = self.strip_references(source, id) else {
                    return Ok(false);
                };
                match shape {
                    Shape::Vec(inner) | Shape::Array(inner, _) | Shape::Slice(inner) => {
                        let selected = if range {
                            self.shaped(Shape::Slice(inner))
                        } else {
                            inner
                        };
                        self.unify(result, selected, span)?
                    }
                    Shape::Atom(name) if range && name == "str" => {
                        let text = self.atom("str");
                        self.unify(result, text, span)?;
                    }
                    Shape::Nominal(declaration, _)
                        if range
                            && self.source.declarations[declaration].name
                                == "$std::string::String" =>
                    {
                        let text = self.atom("str");
                        self.unify(result, text, span)?;
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
                        let Some(field_type) = self.struct_field_type(declaration, &name) else {
                            return Ok(false);
                        };
                        let environment = self.nominal_environment(declaration, &args);
                        let ty = self.source_type(field_type, &environment)?;
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
                    ("wrapping_add" | "wrapping_sub" | "wrapping_mul", Shape::Atom(atom))
                        if integer_name(&atom) && arguments.len() == 1 =>
                    {
                        let integer = self.atom(&atom);
                        self.coerce(arguments[0], integer, span)?;
                        self.unify(result, integer, span)?;
                    }
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
                let result = match &obligation {
                    Obligation::Coercion { .. }
                    | Obligation::Callable {
                        produced: false, ..
                    } => None,
                    Obligation::Callable {
                        produced: true,
                        result,
                        ..
                    } => Some(*result),
                    Obligation::Try { result, .. }
                    | Obligation::Deref { result, .. }
                    | Obligation::Index { result, .. }
                    | Obligation::Field { result, .. }
                    | Obligation::Method { result, .. } => Some(*result),
                };
                if self.solve_obligation(id, obligation)? {
                    self.obligations[id] = None;
                    if let Some(result) = result {
                        self.finish_operation(result);
                    }
                }
            }
        }
        Ok(())
    }
    fn finish(mut self) -> Result<DefinitionTypes> {
        self.solve()?;
        // If no surrounding constraint selects a reference coercion, infer the
        // intact referent. Each choice can wake ordinary constraints and another
        // coercion; process those before choosing the next unresolved fallback.
        while let Some(id) = self.fallback.pop_front() {
            if let Some(Obligation::Coercion {
                source,
                target,
                span,
            }) = self.obligations[id].clone()
            {
                if !self.pending_operation(source) && !self.pending_operation(target) {
                    self.obligations[id] = None;
                    self.coerce_known(source, target, span)?;
                    self.solve()?;
                }
            }
        }
        // An operation still missing its type contract can constrain every leaf
        // of an adjacent coercion later. Leave those defaults to the old gate.
        let mut blocked = Vec::new();
        for obligation in self.obligations.iter().flatten() {
            if let Obligation::Coercion { source, target, .. } = obligation {
                blocked.push(*source);
                blocked.push(*target);
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        while let Some(variable) = blocked.pop() {
            let root = self.root(variable);
            if seen.insert(root) {
                self.definition.nodes[root].blocked = true;
                if let Some(shape) = self.shape(root) {
                    blocked.extend(shape.children());
                }
            }
        }
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
        source_function: index,
        capture_frames: Vec::new(),
        callable_parameters: BTreeMap::new(),
        source,
        names,
        definition: DefinitionTypes {
            nodes: Vec::new(),
            expressions: BTreeMap::new(),
            bindings: BTreeMap::new(),
            closures: BTreeMap::new(),
        },
        bindings: Bindings::default(),
        environment: BTreeMap::new(),
        result: 0,
        waiting: Vec::new(),
        obligations: Vec::new(),
        queue: VecDeque::new(),
        fallback: VecDeque::new(),
        queued: Vec::new(),
        field_types: BTreeMap::new(),
        variant_fields: BTreeMap::new(),
        loops: Vec::new(),
    };
    match source.function_owner(a::FunctionId { index }) {
        a::FunctionOwner::Implementation(owner) => {
            let implementation = &source.impls[owner.index];
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
        a::FunctionOwner::Trait(owner) => {
            let definition = &source.traits[owner.index];
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
        a::FunctionOwner::Free => {}
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
    for parameter in &function.generics {
        if let Some(receiver) = environment.get(&parameter.name).copied() {
            for bound in &parameter.bounds {
                inference.callable_bound(receiver, *bound, &environment)?;
            }
        }
    }
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
