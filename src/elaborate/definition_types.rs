//! Definition-local inference. The graph records structural type constraints
//! before IR construction; integer fallback happens only after those constraints
//! have propagated. Method selection and ownership remain checked separately.
use super::{Bindings, Compiler, Result, a, err, ir, is_copy};
use std::collections::BTreeMap;

mod capabilities;
mod closures;
mod method_headers;
mod pattern_equations;
mod projection_equations;
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
    Projected(a::DeclarationId, String),
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
    fn primitive(&self) -> super::methods::Receiver {
        use super::methods::Receiver;
        match self {
            Self::Atom(name) if integer_name(name) => Receiver::Integer,
            Self::Atom(name) if name == "char" => Receiver::Character,
            Self::Atom(name) if name == "str" => Receiver::Text,
            Self::Atom(name) if name == "World" => Receiver::World,
            Self::Atom(name) if name == "Stdout" => Receiver::Stdout,
            Self::Atom(name) if name == "Stdin" => Receiver::Stdin,
            Self::Atom(name) if name == "Stderr" => Receiver::Stderr,
            Self::Atom(name) if name == "Directory" => Receiver::Directory,
            Self::Atom(name) if name == "File" => Receiver::File,
            Self::Vec(_) => Receiver::Vector,
            Self::Array(_, _) => Receiver::Array,
            Self::Slice(_) => Receiver::Slice,
            _ => Receiver::Other,
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
    if text.ends_with("usize") {
        Some("usize")
    } else if text.ends_with("isize") {
        Some("isize")
    } else if text.ends_with("i64") {
        Some("i64")
    } else if text.ends_with("u64") {
        Some("u64")
    } else if text.ends_with("i32") {
        Some("i32")
    } else if text.ends_with("u32") {
        Some("u32")
    } else if text.ends_with("i16") {
        Some("i16")
    } else if text.ends_with("u16") {
        Some("u16")
    } else if text.ends_with("i8") {
        Some("i8")
    } else if text.ends_with("u8") {
        Some("u8")
    } else {
        None
    }
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
#[derive(Clone, Default)]
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
                let mut next = children.len();
                while next > 0 {
                    next -= 1;
                    pending.push((self.root(children[next]), false));
                }
                continue;
            }
            let mut values: Vec<usize> = Vec::new();
            let mut complete = true;
            for child in children {
                match *known.get(&self.root(child)).unwrap() {
                    Some(value) => values.push(value),
                    None => complete = false,
                }
            }
            if !complete {
                known.insert(id, None);
                continue;
            }
            if matches!(shape, Shape::Vec(_) | Shape::Box(_) | Shape::Array(..)) {
                compiler.check_owned_element(values[0], span)?;
            }
            let ty = match shape {
                Shape::Atom(name) => {
                    let named = match compiler.type_names.get(&name) {
                        Some(ty) => Some(*ty),
                        None => None,
                    };
                    if let Some(ty) = named {
                        Some(ty)
                    } else if name == "()" {
                        Some(ir::UNIT)
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
                Shape::Rigid(name) | Shape::Projected(_, name) => substitutions.get(&name).copied(),
                Shape::Source(source) => compiler.resolve_if_known(source, substitutions)?,
                Shape::Vec(_) => {
                    Some(compiler.intern(String::new(), ir::TypeKind::Vec { inner: values[0] }))
                }
                Shape::Box(_) => {
                    Some(compiler.intern(String::new(), ir::TypeKind::Box { inner: values[0] }))
                }
                Shape::Ref(mutable, _) => {
                    if mutable && matches!(compiler.types[values[0]].kind, ir::TypeKind::Str) {
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
                    ir::TypeKind::Slice {
                        inner: values[0],
                        mutable: false,
                    },
                )),
                Shape::Array(_, len) => Some(compiler.intern(
                    String::new(),
                    ir::TypeKind::Array {
                        inner: values[0],
                        len,
                    },
                )),
                Shape::Tuple(_) => {
                    if values.is_empty() {
                        Some(ir::UNIT)
                    } else {
                        let mut fields = Vec::new();
                        for index in 0..values.len() {
                            fields.push(ir::Field {
                                name: format!("{index}"),
                                ty: values[index],
                            });
                        }
                        let mut copy = true;
                        for field in &fields {
                            if !is_copy(&compiler.types, field.ty) {
                                copy = false;
                                break;
                            }
                        }
                        Some(compiler.intern(String::new(), ir::TypeKind::Struct { fields, copy }))
                    }
                }
                Shape::Nominal(declaration, _) => {
                    let mut arguments = substitutions.clone();
                    let mut index = 0;
                    for parameter in parameters(compiler.source, declaration) {
                        if index >= values.len() {
                            break;
                        }
                        arguments.insert(parameter, values[index]);
                        index += 1;
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
                    let closure = self.closures.get(&expression).unwrap();
                    let capture_count = closure.captures.len();
                    let mut captures = Vec::new();
                    let mut index = 0;
                    while index < closure.captures.len() && index < values.len() {
                        let capture = &closure.captures[index];
                        captures.push(super::closures::VariableCapture {
                            name: capture.name.clone(),
                            ty: values[index],
                            uses: capture.uses.clone(),
                        });
                        index += 1;
                    }
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
        Ok(*known.get(&root).unwrap())
    }
}
fn parameters(source: &a::Program, declaration: a::DeclarationId) -> Vec<String> {
    let parameters = match source.declarations[declaration].item {
        a::Item::Struct(i) => &source.structs[i].generics,
        a::Item::Enum(i) => &source.enums[i].generics,
        a::Item::Alias(i) => &source.aliases[i].generics,
        _ => return Vec::new(),
    };
    let mut names = Vec::new();
    for parameter in parameters {
        if !parameter.lifetime {
            names.push(parameter.name.clone());
        }
    }
    names
}
#[derive(Clone)]
enum Obligation {
    PatternTest {
        pattern: a::PatternId,
        subject: Variable,
    },
    Pattern {
        pattern: a::PatternId,
        subject: Variable,
        mode: Option<bool>,
        names: Vec<(String, Variable)>,
    },
    Projection {
        receiver: Variable,
        declaration: a::DeclarationId,
        member: String,
        result: Variable,
        span: a::Span,
    },
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
    Unary {
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
        types: Vec<a::TypeId>,
        scope: a::ModuleId,
        arguments: Vec<Variable>,
        result: Variable,
        span: a::Span,
    },
}
struct Infer<'a, 'b> {
    source_function: usize,
    capture_frames: Vec<closures::Frame>,
    callable_parameters: BTreeMap<String, (Vec<Variable>, Variable)>,
    projections: Vec<BTreeMap<(a::DeclarationId, String), Variable>>,
    source: &'a a::Program,
    names: &'b crate::resolve::Resolver<'a>,
    implementations: &'b crate::generics::ImplementationIndex,
    definition: DefinitionTypes,
    bindings: Bindings,
    environment: BTreeMap<String, Variable>,
    result: Variable,
    waiting: Vec<Vec<usize>>,
    obligations: Vec<Option<Obligation>>,
    queue: Vec<usize>,
    fallback: Vec<usize>,
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
        self.projections.push(BTreeMap::new());
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
    fn display_variables(&self, variables: &[Variable]) -> String {
        let mut result = String::new();
        let mut first = true;
        for variable in variables {
            if !first {
                result.push_str(", ");
            }
            first = false;
            result.push_str(&self.display_variable(*variable));
        }
        result
    }
    fn display_variable(&self, variable: Variable) -> String {
        match self.shape(variable) {
            None => "_".to_string(),
            Some(Shape::Atom(name) | Shape::Rigid(name)) => name,
            Some(Shape::Vec(inner)) => format!("Vec<{}>", self.display_variable(inner)),
            Some(Shape::Box(inner)) => format!("Box<{}>", self.display_variable(inner)),
            Some(Shape::Ref(mutable, inner)) => format!(
                "&{}{}",
                if mutable { "mut " } else { "" },
                self.display_variable(inner)
            ),
            Some(Shape::Slice(inner)) => format!("[{}]", self.display_variable(inner)),
            Some(Shape::Array(inner, len)) => format!("[{}; {len}]", self.display_variable(inner)),
            Some(Shape::Tuple(fields)) => format!("({})", self.display_variables(&fields)),
            Some(Shape::Nominal(declaration, args)) => {
                let full_name = self.source.declarations[declaration].name.as_str();
                let mut start = 0;
                while full_name[start..].starts_with("$std::") {
                    start += 6;
                }
                let name = &full_name[start..];
                if args.is_empty() {
                    name.to_string()
                } else {
                    format!("{name}<{}>", self.display_variables(&args))
                }
            }
            Some(Shape::Projected(_, member)) => format!("associated type {member}"),
            Some(Shape::Source(id)) => match &self.source.types[id].kind {
                a::Type::Path(path) => path.name.clone(),
                _ => "declared type".to_string(),
            },
            Some(Shape::Closure(..)) => "closure".to_string(),
            Some(Shape::Never) => "!".to_string(),
        }
    }
    fn never(&self, v: Variable) -> bool {
        self.shape(v) == Some(Shape::Never)
    }
    fn occurs(&self, variable: Variable, within: Variable) -> bool {
        let mut pending = vec![within];
        let mut seen = std::collections::BTreeSet::<Variable>::new();
        while let Some(id) = pending.pop() {
            let id = self.definition.root(id);
            if id == variable {
                return true;
            }
            if seen.insert(id) {
                if let Some(shape) = &self.definition.nodes[id].shape {
                    for child in shape.children() {
                        pending.push(child);
                    }
                }
            }
        }
        false
    }
    fn enqueue(&mut self, id: usize) {
        if !self.queued[id] && self.obligations[id].is_some() {
            self.queue.push(id);
            self.queued[id] = true;
        }
    }
    fn join_roots(
        &mut self,
        mut a: Variable,
        mut b: Variable,
        shape: Option<Shape>,
    ) -> Vec<(Variable, Variable)> {
        if self.definition.nodes[a].rank < self.definition.nodes[b].rank {
            let previous = a;
            a = b;
            b = previous;
        }
        if self.definition.nodes[a].rank == self.definition.nodes[b].rank {
            self.definition.nodes[a].rank += 1;
        }
        self.definition.nodes[b].parent = a;
        self.definition.nodes[a].shape = shape;
        self.definition.nodes[a].integer |= self.definition.nodes[b].integer;
        self.definition.nodes[a].blocked |= self.definition.nodes[b].blocked;
        self.definition.nodes[a].producers += self.definition.nodes[b].producers;
        let waiting_a = std::mem::replace(&mut self.waiting[a], Vec::new());
        let waiting_b = std::mem::replace(&mut self.waiting[b], Vec::new());
        for id in waiting_a {
            self.enqueue(id);
        }
        for id in waiting_b {
            self.enqueue(id);
        }
        let mut equations = Vec::new();
        for (key, value) in std::mem::replace(&mut self.projections[b], BTreeMap::new()) {
            if let Some(previous) = self.projections[a].insert(key, value) {
                equations.push((previous, value));
            }
        }
        equations
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
                if matches!(
                    sa,
                    Some(Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_))
                ) || matches!(
                    sb,
                    Some(Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_))
                ) {
                    continue;
                }
                return Err(err(
                    span,
                    "E_TYPE",
                    "integer inference requires an integer type",
                ));
            }
            let merge = match (&sa, &sb) {
                (None, _) | (_, None) => {
                    if self.occurs(a, b) || self.occurs(b, a) {
                        return Err(err(
                            span,
                            "E_TYPE",
                            "inference would require an infinite type",
                        ));
                    }
                    true
                }
                (Some(Shape::Vec(x)), Some(Shape::Vec(y)))
                | (Some(Shape::Box(x)), Some(Shape::Box(y)))
                | (Some(Shape::Slice(x)), Some(Shape::Slice(y))) => {
                    pending.push((*x, *y));
                    true
                }
                (Some(Shape::Array(x, nx)), Some(Shape::Array(y, ny))) if nx == ny => {
                    pending.push((*x, *y));
                    true
                }
                (Some(Shape::Ref(mx, x)), Some(Shape::Ref(my, y))) if mx == my => {
                    pending.push((*x, *y));
                    true
                }
                (Some(Shape::Tuple(xs)), Some(Shape::Tuple(ys))) if xs.len() == ys.len() => {
                    for index in 0..xs.len() {
                        pending.push((xs[index], ys[index]));
                    }
                    true
                }
                (Some(Shape::Nominal(nx, xs)), Some(Shape::Nominal(ny, ys)))
                    if nx == ny && xs.len() == ys.len() =>
                {
                    for index in 0..xs.len() {
                        pending.push((xs[index], ys[index]));
                    }
                    true
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
                (Some(Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_)), _)
                | (_, Some(Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_))) => {
                    sa == sb
                }
                _ if sa == sb => true,
                _ => {
                    return Err(err(
                        span,
                        "E_TYPE",
                        &format!(
                            "type mismatch: expected {}, found {}; help: make the expression and its declared context use the same type",
                            self.display_variable(right),
                            self.display_variable(left)
                        ),
                    ));
                }
            };
            if merge {
                for equation in self.join_roots(a, b, sa.or(sb)) {
                    pending.push(equation);
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
            (Some(Shape::Ref(_, source)), Some(Shape::Ref(_, target))) => {
                // An unannotated callable input may acquire its actual referent
                // from the enclosing method. Do not infer str from &input before
                // map_err (for example) supplies String and its ordinary coercion.
                self.shape(source).is_none() || self.shape(target).is_none()
            }
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
            } // IR/type checking diagnoses an invalid mutable coercion.
            return self.coerce_referents(from, to, span);
        }
        self.unify(from, to, span)
    }
    fn finish_operation(&mut self, variable: Variable) {
        let root = self.root(variable);
        self.definition.nodes[root].producers -= 1;
        if self.definition.nodes[root].producers == 0 {
            let waiting = std::mem::replace(&mut self.waiting[root], Vec::new());
            for id in waiting {
                self.enqueue(id);
            }
        }
    }
    fn coerce_referents(&mut self, mut from: Variable, to: Variable, span: a::Span) -> Result<()> {
        let text = match self.shape(to) {
            Some(Shape::Atom(name)) => name == "str",
            _ => false,
        };
        if text {
            if let Some(Shape::Nominal(declaration, _)) = self.shape(from) {
                if self
                    .names
                    .library_type("string", "String", span)?
                    .declaration
                    == Some(declaration)
                {
                    return Ok(());
                }
            }
        }
        if let Some(Shape::Slice(element)) = self.shape(to) {
            if let Some(Shape::Vec(source) | Shape::Array(source, _)) = self.shape(from) {
                return self.unify(source, element, span);
            }
        }
        while let Some(Shape::Box(inner)) = self.shape(from) {
            if matches!(
                self.shape(to),
                None | Some(
                    Shape::Box(_) | Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_)
                )
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
        let parameters = parameters(self.source, declaration);
        let mut environment = BTreeMap::new();
        let mut index = 0;
        while index < parameters.len() && index < args.len() {
            environment.insert(parameters[index].clone(), args[index]);
            index += 1;
        }
        environment
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
            let mut fields = BTreeMap::new();
            for field in &self.source.structs[index].fields {
                fields.insert(field.name.clone(), field.ty);
            }
            self.field_types.insert(declaration, fields);
        }
        match self
            .field_types
            .get(&declaration)
            .unwrap()
            .get(&name.to_string())
        {
            Some(ty) => Some(*ty),
            None => None,
        }
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
            let mut variants = BTreeMap::new();
            for variant in &self.source.enums[index].variants {
                variants.insert(variant.name.clone(), variant.fields.clone());
            }
            self.variant_fields.insert(declaration, variants);
        }
        self.variant_fields
            .get(&declaration)
            .unwrap()
            .get(&name.to_string())
            .cloned()
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
        let mut environment = BTreeMap::new();
        for index in 0..params.len() {
            environment.insert(params[index].clone(), arguments[index]);
        }
        let nominal = if let a::Item::Alias(index) = self.source.declarations[declaration].item {
            let ty = self.source.aliases[index].ty;
            self.source_type(ty, &environment)?
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
                let Some(separator) = path.name.rfind("::") else {
                    return Ok(None);
                };
                let variant = &path.name.as_str()[separator + 2..];
                let Some(fields) = self.enum_variant_fields(declaration, variant) else {
                    return Ok(None);
                };
                fields
            }
            _ => return Ok(None),
        };
        let mut declared_fields = BTreeMap::new();
        for field in declared {
            declared_fields.insert(field.name.clone(), field);
        }
        let generic = !parameters(self.source, declaration).is_empty();
        for (name, value) in fields {
            if let Some(field) = declared_fields.get(name) {
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
        self.header_signature(index, type_args, values, BTreeMap::new(), 0, span)
    }
    fn header_signature(
        &mut self,
        index: usize,
        type_args: &[a::TypeId],
        values: &[Variable],
        mut environment: BTreeMap<String, Variable>,
        first: usize,
        span: a::Span,
    ) -> Result<Variable> {
        let function = self.source.functions[index].clone();
        let outer = self.environment.clone();
        let mut type_index = 0;
        for parameter in &function.generics {
            if !parameter.lifetime {
                let value = if let Some(ty) = type_args.get(type_index) {
                    self.source_type(*ty, &outer)?
                } else {
                    self.unknown()
                };
                environment.insert(parameter.name.clone(), value);
                type_index += 1;
            }
        }
        self.parameter_equations(&function.generics, &mut environment, false)?;
        let parameters = &function.params[first..];
        let mut index = 0;
        while index < parameters.len() && index < values.len() {
            let expected = self.source_type(parameters[index].ty, &environment)?;
            self.coerce(values[index], expected, span)?;
            index += 1;
        }
        self.source_type(function.result, &environment)
    }
    fn constrain_pattern(&mut self, pattern: a::PatternId, mut subject: Variable) -> Result<()> {
        if matches!(
            self.source.patterns[pattern].kind,
            a::Pattern::Literal(_) | a::Pattern::Range { .. }
        ) && self.pattern_test_pending(subject)
        {
            self.obligation(Obligation::PatternTest { pattern, subject });
            return Ok(());
        }
        let node = self.source.patterns[pattern].clone();
        if let a::Pattern::Ref { inner, .. } = &node.kind {
            if let Some(Shape::Ref(_, referent)) = self.shape(subject) {
                return self.constrain_pattern(*inner, referent);
            }
            return self.constrain_pattern(*inner, subject);
        }
        if let a::Pattern::Or(patterns) = &node.kind {
            for pattern in patterns {
                self.constrain_pattern(*pattern, subject)?;
            }
            return Ok(());
        }
        if let a::Pattern::Literal(expression) = &node.kind {
            if matches!(self.source.exprs[*expression].kind, a::Expr::String(_)) {
                let value = self.expr(*expression)?;
                return self.unify(subject, value, node.span);
            }
        }
        while let Some(Shape::Ref(_, inner)) = self.shape(subject) {
            subject = inner;
        }
        match node.kind {
            a::Pattern::Literal(expression) => {
                let mut value = self.expr(expression)?;
                // Literal patterns compare referents, including the implicit
                // shared reference carried by a string literal.
                while let Some(Shape::Ref(_, inner)) = self.shape(value) {
                    value = inner;
                }
                self.unify(subject, value, node.span)?;
            }
            a::Pattern::Range { start, end, .. } => {
                if let Some(expression) = start {
                    let value = self.expr(expression)?;
                    self.unify(subject, value, node.span)?;
                }
                if let Some(expression) = end {
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
        if self.pattern_pending(pattern, value) {
            self.defer_pattern(pattern, value, mode);
            return Ok(());
        }
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
                let mut rest = false;
                for field in &fields {
                    if matches!(self.source.patterns[*field].kind, a::Pattern::Rest) {
                        rest = true;
                        break;
                    }
                }
                let values = if rest {
                    match self.shape(value) {
                        Some(Shape::Tuple(values)) => values,
                        Some(Shape::Atom(name)) => {
                            if name != "()" {
                                return Err(err(
                                    node.span,
                                    "E_TYPE",
                                    "tuple rest pattern requires a tuple",
                                ));
                            }
                            Vec::new()
                        }
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
                    let mut values = Vec::new();
                    for _ in &fields {
                        values.push(self.unknown());
                    }
                    values
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
                let no_fields: Vec<(String, Variable)> = Vec::new();
                let count = if let Some(nominal) = self.constructor(&path, &no_fields, node.span)? {
                    if let Some(Shape::Nominal(declaration, _)) = self.shape(nominal) {
                        match self.source.declarations[declaration].item {
                            a::Item::Struct(index) => self.source.structs[index].fields.len(),
                            a::Item::Enum(_) => {
                                if let Some(separator) = path.name.rfind("::") {
                                    let name = &path.name.as_str()[separator + 2..];
                                    match self.enum_variant_fields(declaration, name) {
                                        Some(fields) => fields.len(),
                                        None => 0,
                                    }
                                } else {
                                    0
                                }
                            }
                            _ => fields.len(),
                        }
                    } else {
                        fields.len()
                    }
                } else {
                    fields.len()
                };
                let positions =
                    super::patterns::positional_fields(self.source, &fields, count, node.span)?;
                let mut fields = Vec::new();
                for (index, pattern) in positions {
                    fields.push((format!("{index}"), pattern));
                }
                self.bind_nominal_pattern(&path, &fields, value, mode, node.scope, node.span)?;
            }
            a::Pattern::Struct { path, fields, .. } => {
                self.bind_nominal_pattern(&path, &fields, value, mode, node.scope, node.span)?;
            }
            a::Pattern::Path(path) => {
                let no_fields: Vec<(String, a::PatternId)> = Vec::new();
                self.bind_nominal_pattern(&path, &no_fields, value, mode, node.scope, node.span)?;
            }
            a::Pattern::Or(alternatives) => {
                let scope = self.bindings.len();
                let mut first = None::<BTreeMap<String, Variable>>;
                for alternative in alternatives {
                    self.bindings.truncate(scope);
                    self.bind_pattern_mode(alternative, value, mode)?;
                    let mut bindings = BTreeMap::new();
                    for index in scope..self.bindings.entries.len() {
                        let binding = &self.bindings.entries[index];
                        bindings.insert(binding.0.clone(), binding.1);
                    }
                    if first.is_some() {
                        let first = first.as_ref().unwrap();
                        let mut same_names = first.len() == bindings.len();
                        for name in bindings.keys() {
                            if !first.contains_key(name) {
                                same_names = false;
                                break;
                            }
                        }
                        if !same_names {
                            return Err(err(
                                node.span,
                                "E_TYPE",
                                "or-pattern alternatives must bind the same names",
                            ));
                        }
                        for (name, ty) in &bindings {
                            self.unify(*ty, *first.get(name).unwrap(), node.span)?;
                        }
                    } else {
                        first = Some(bindings);
                    }
                }
                self.bindings.truncate(scope);
                let first = match first {
                    Some(first) => first,
                    None => BTreeMap::new(),
                };
                for binding in first {
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
        let no_fields: Vec<(String, Variable)> = Vec::new();
        let Some(nominal) = self.constructor(&path, &no_fields, span)? else {
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
        let mut variant = None;
        if matches!(self.source.declarations[declaration].item, a::Item::Enum(_)) {
            if let Some(separator) = path.name.rfind("::") {
                let name = &path.name.as_str()[separator + 2..];
                if let Some(fields) = self.enum_variant_fields(declaration, name) {
                    let mut types = BTreeMap::new();
                    for field in fields {
                        types.insert(field.name, field.ty);
                    }
                    variant = Some(types);
                }
            }
        }
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
                                    let ty = self.source.constants[index].ty;
                                    self.source_type(ty, &environment)?
                                } else {
                                    let no_fields: Vec<(String, Variable)> = Vec::new();
                                    self.constructor(&path, &no_fields, span)?
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
                let length = match &self.source.exprs[count].kind {
                    a::Expr::Int(text) => {
                        let text = text.as_str();
                        let mut end = text.len();
                        while text[..end].ends_with("usize") {
                            end -= 5;
                        }
                        let text = text[..end].replace('_', "");
                        text.as_str().parse::<usize>().ok()
                    }
                    _ => None,
                };
                if let Some(len) = length {
                    self.shaped(Shape::Array(value, len))
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
                    let result = self.operation_type();
                    self.obligation(Obligation::Unary {
                        source: value,
                        result,
                        span,
                    });
                    result
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
                } else if matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
                    self.unify(left, right, span)?;
                    self.atom("bool")
                } else {
                    // Primitive operators forward one shared reference layer.
                    // Keep that choice delayed until both operand shapes are
                    // known, just as for unary scalar operations.
                    let left_value = self.operation_type();
                    self.obligation(Obligation::Unary {
                        source: left,
                        result: left_value,
                        span,
                    });
                    let right_value = self.operation_type();
                    self.obligation(Obligation::Unary {
                        source: right,
                        result: right_value,
                        span,
                    });
                    if op != "<<" && op != ">>" {
                        self.unify(left_value, right_value, span)?;
                    }
                    // A shift amount keeps its own integer type.
                    left_value
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
                let callee_scope = self.source.exprs[callee].scope;
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
                        && matches!(path.name.as_str(), "char::from_u32" | "std::char::from_u32")
                    {
                        if values.len() != 1 || !path.args.is_empty() {
                            return Err(err(
                                span,
                                "E_TYPE",
                                "char::from_u32 requires one u32 argument",
                            ));
                        }
                        let integer = self.atom("u32");
                        self.coerce(values[0], integer, span)?;
                        let character = self.atom("char");
                        let path = self.names.library_type("prelude", "Option", span)?;
                        let Some(option) = path.declaration else {
                            return Err(err(span, "E_TYPE", "Option declaration is unavailable"));
                        };
                        self.shaped(Shape::Nominal(option, vec![character]))
                    } else if path.declaration.is_none()
                        && matches!(path.name.as_str(), "drop" | "std::mem::drop")
                    {
                        self.atom("()")
                    } else if let Some(a::Item::Function(index)) =
                        path.declaration.map(|id| self.source.declarations[id].item)
                    {
                        self.call_signature(index, &path.args, &values, span)?
                    } else if let Some(result) =
                        self.associated_header(&path, &values, callee_scope, span)?
                    {
                        result
                    } else {
                        let mut fields = Vec::new();
                        for index in 0..values.len() {
                            fields.push((format!("{index}"), values[index]));
                        }
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
                let result = self.operation_type();
                self.obligation(Obligation::Method {
                    receiver,
                    name,
                    types,
                    scope: node.scope,
                    arguments,
                    result,
                    span,
                });
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
                    if let Some(bound) = start {
                        let bound = self.expr(bound)?;
                        self.unify(bound, usize, span)?;
                    }
                    if let Some(bound) = end {
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
                    let result = self.result;
                    self.coerce(value, result, span)
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
            } => {
                if name == "vec" {
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
                } else {
                    for arg in args {
                        self.expr(arg)?;
                    }
                    if name == "panic" || name == "unreachable" {
                        self.shaped(Shape::Never)
                    } else {
                        self.blocked()
                    }
                }
            }
            a::Expr::Try(value) => {
                let source = self.expr(value)?;
                let result = self.operation_type();
                let target = self.result;
                self.obligation(Obligation::Try {
                    target,
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
            Obligation::PatternTest { pattern, subject } => {
                if self.pattern_test_pending(subject) {
                    self.watch_shape(subject, id);
                    return Ok(false);
                }
                self.constrain_pattern(pattern, subject)?;
            }
            Obligation::Pattern {
                pattern,
                subject,
                mode,
                names,
            } => {
                return self.solve_pattern(id, pattern, subject, mode, names);
            }
            Obligation::Projection {
                receiver,
                declaration,
                member,
                result,
                span,
            } => {
                return self.solve_projection(id, receiver, declaration, &member, result, span);
            }
            Obligation::Callable {
                receiver,
                arguments,
                result,
                span,
                produced,
            } => return self.solve_callable(id, receiver, arguments, result, span, produced),
            Obligation::Coercion {
                source,
                target,
                span,
            } => {
                if self.coercion_pending(source, target) {
                    for index in 0..2usize {
                        let mut variable = if index == 0 { source } else { target };
                        loop {
                            self.watch(variable, id);
                            match self.shape(variable) {
                                Some(Shape::Ref(_, inner) | Shape::Box(inner)) => variable = inner,
                                _ => break,
                            }
                        }
                    }
                    if !self.pending_operation(source) && !self.pending_operation(target) {
                        self.fallback.push(id);
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
                    if matches!(
                        shape,
                        Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_)
                    ) {
                        return Ok(false);
                    }
                    return Err(err(
                        span,
                        "E_TYPE",
                        "question-mark requires Result or Option",
                    ));
                };
                let arity = match self.source.declarations[declaration].name.as_str() {
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
                    // Rust's Result question mark converts its error through
                    // From. The source String conversion is the concrete case
                    // used by the backend; other differing errors still need
                    // explicit map_err until general From evidence is supported.
                    let text_output = matches!(
                        self.shape(output_arguments[1]),
                        Some(Shape::Nominal(declaration, _))
                            if self.source.declarations[declaration].name == "$std::string::String"
                    );
                    let text_input = match self.shape(arguments[1]) {
                        Some(Shape::Ref(false, inner)) => {
                            if text_output && self.shape(inner).is_none() {
                                self.watch(inner, id);
                                return Ok(false);
                            }
                            match self.shape(inner) {
                                Some(Shape::Atom(name)) => name == "str",
                                _ => false,
                            }
                        }
                        None if text_output => {
                            // Argument coercions can still resolve the error to
                            // &str. Do not force it to String before they run.
                            self.watch(arguments[1], id);
                            return Ok(false);
                        }
                        _ => false,
                    };
                    if !(text_input && text_output) {
                        self.unify(arguments[1], output_arguments[1], span)?;
                    }
                }
            }
            Obligation::Unary {
                source,
                result,
                span,
            } => match self.shape(source) {
                Some(Shape::Ref(false, inner)) => match self.shape(inner) {
                    Some(Shape::Atom(name)) => {
                        if name == "bool" || name == "f64" || integer_name(&name) {
                            self.unify(result, inner, span)?;
                        } else {
                            self.unify(result, source, span)?;
                        }
                    }
                    None => {
                        self.watch(inner, id);
                        return Ok(false);
                    }
                    _ => self.unify(result, source, span)?,
                },
                None => {
                    if self.definition.nodes[self.definition.root(source)].integer {
                        self.unify(result, source, span)?;
                    } else {
                        self.watch(source, id);
                        return Ok(false);
                    }
                }
                _ => self.unify(result, source, span)?,
            },
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
                    Shape::Atom(name) => {
                        if !range || name != "str" {
                            return Ok(false);
                        }
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
                        let index = match name.as_str().parse::<usize>().ok() {
                            Some(index) => index,
                            None => return Ok(false),
                        };
                        let field = match fields.get(index) {
                            Some(field) => *field,
                            None => return Ok(false),
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
                types,
                scope,
                arguments,
                result,
                span,
            } => {
                let Some((base, shape)) = self.strip_references(receiver, id) else {
                    return Ok(false);
                };
                if self.inherent_header(base, &name, &types, &arguments, result, (scope, span))? {
                    return Ok(true);
                }
                let primitive = shape.primitive();
                let direct = super::methods::direct(primitive, &name);
                if direct == Some(super::methods::Direct::Character) {
                    let function = self.names.library_function(
                        "char",
                        &format!("__lkjscript_{name}"),
                        span,
                    )?;
                    let mut values = vec![base];
                    values.extend_from_slice(&arguments);
                    let output = self.call_signature(function, &types, &values, span)?;
                    self.unify(result, output, span)?;
                    return Ok(true);
                }
                if direct.is_none()
                    && self.contextual_header(
                        base,
                        &name,
                        &types,
                        &arguments,
                        result,
                        (id, scope, span),
                    )?
                {
                    return Ok(true);
                }
                if !types.is_empty() {
                    return Ok(false);
                }
                // The same primitive inventory drives construction and inference.
                if let Some(method) = super::capabilities::method(primitive, &name) {
                    let (parameters, output) = method.signature();
                    if parameters.len() != arguments.len() {
                        return Err(err(
                            span,
                            "E_TYPE",
                            &format!(
                                "capability method {name} expects {} arguments, found {}",
                                parameters.len(),
                                arguments.len()
                            ),
                        ));
                    }
                    let mut index = 0;
                    while index < parameters.len() {
                        let parameter = self.capability_type(parameters[index], span)?;
                        self.coerce(arguments[index], parameter, span)?;
                        index += 1;
                    }
                    let output = self.capability_type(output, span)?;
                    self.unify(result, output, span)?;
                    return Ok(true);
                }
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
                    ("len", Shape::Atom(atom)) => {
                        if atom != "str" || !arguments.is_empty() {
                            return Ok(false);
                        }
                        let usize = self.atom("usize");
                        self.unify(result, usize, span)?;
                    }
                    ("is_empty", Shape::Atom(atom)) => {
                        if atom != "str" || !arguments.is_empty() {
                            return Ok(false);
                        }
                        let boolean = self.atom("bool");
                        self.unify(result, boolean, span)?;
                    }
                    ("as_bytes", Shape::Atom(atom)) => {
                        if atom != "str" || !arguments.is_empty() {
                            return Ok(false);
                        }
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
        let mut next = 0;
        while next < self.queue.len() {
            let id = self.queue[next];
            next += 1;
            self.queued[id] = false;
            if let Some(obligation) = self.obligations[id].clone() {
                let result = match &obligation {
                    Obligation::PatternTest { .. }
                    | Obligation::Pattern { .. }
                    | Obligation::Coercion { .. }
                    | Obligation::Callable {
                        produced: false, ..
                    } => None,
                    Obligation::Callable {
                        produced: true,
                        result,
                        ..
                    } => Some(*result),
                    Obligation::Projection { result, .. }
                    | Obligation::Try { result, .. }
                    | Obligation::Unary { result, .. }
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
        self.queue.clear();
        Ok(())
    }
    fn finish(mut self) -> Result<DefinitionTypes> {
        self.solve()?;
        // If no surrounding constraint selects a reference coercion, infer the
        // intact referent. Each choice can wake ordinary constraints and another
        // coercion; process those before choosing the next unresolved fallback.
        let mut next_fallback = 0;
        while next_fallback < self.fallback.len() {
            let id = self.fallback[next_fallback];
            next_fallback += 1;
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
        let mut blocked: Vec<Variable> = Vec::new();
        for obligation in &self.obligations {
            if let Some(Obligation::Coercion { source, target, .. }) = obligation {
                blocked.push(*source);
                blocked.push(*target);
            }
        }
        let mut seen = std::collections::BTreeSet::<Variable>::new();
        while let Some(variable) = blocked.pop() {
            let root = self.root(variable);
            if seen.insert(root) {
                self.definition.nodes[root].blocked = true;
                if let Some(shape) = self.shape(root) {
                    for child in shape.children() {
                        blocked.push(child);
                    }
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
    implementations: &crate::generics::ImplementationIndex,
    index: usize,
) -> Result<DefinitionTypes> {
    let function = source.functions[index].clone();
    let mut inference = Infer {
        source_function: index,
        capture_frames: Vec::new(),
        callable_parameters: BTreeMap::new(),
        projections: Vec::new(),
        source,
        names,
        implementations,
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
        queue: Vec::new(),
        fallback: Vec::new(),
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
    let mut environment = inference.environment.clone();
    match source.function_owner(a::FunctionId { index }) {
        a::FunctionOwner::Implementation(owner) => {
            let implementation = source.impls[owner.index].clone();
            inference.parameter_equations(&implementation.generics, &mut environment, true)?;
            if let Some(bound) = implementation.trait_ty {
                let receiver = *environment.get(&"Self".to_string()).unwrap();
                inference.trait_equations(receiver, "Self", bound, &mut environment, true)?;
            }
        }
        a::FunctionOwner::Trait(owner) => {
            inference.parameter_equations(
                &source.traits[owner.index].generics,
                &mut environment,
                true,
            )?;
        }
        a::FunctionOwner::Free => {}
    }
    inference.parameter_equations(&function.generics, &mut environment, true)?;
    inference.environment = environment.clone();
    inference.result = inference.source_type(function.result, &environment)?;
    for parameter in &function.params {
        let ty = inference.source_type(parameter.ty, &environment)?;
        inference.bindings.push((parameter.name.clone(), ty));
    }
    if let Some(body) = function.body {
        let body = inference.block(body)?;
        let result = inference.result;
        inference
            .coerce(body, result, function.span)
            .map_err(|problem| context(problem, "function result type differs"))?;
    }
    inference.finish()
}
