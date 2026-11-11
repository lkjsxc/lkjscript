//! Rust-compatible syntax to explicit K. Inference lives here; K trusts none of it.
use crate::ast as a;
use crate::check::{is_copy, place_type};
use crate::k;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, a::Diagnostic>;
fn err(span: a::Span, code: &str, message: &str) -> a::Diagnostic {
    a::Diagnostic::new(span, code, message)
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
#[derive(Clone)]
struct Instance {
    source: usize,
    substitutions: BTreeMap<String, usize>,
}
struct Compiler<'a> {
    source: &'a a::Program,
    types: Vec<k::Type>,
    type_names: BTreeMap<String, usize>,
    functions: Vec<k::Function>,
    instances: Vec<Instance>,
    instance_names: BTreeMap<String, usize>,
    applications: BTreeMap<usize, (String, Vec<usize>)>,
    destructors: Vec<(usize, usize)>,
    uses: BTreeMap<String, Vec<usize>>,
}

pub fn elaborate(source: &a::Program) -> Result<k::Program> {
    crate::generics::validate(source)?;
    let mut c = Compiler {
        source,
        types: k::primitives(),
        type_names: BTreeMap::new(),
        functions: Vec::new(),
        instances: Vec::new(),
        instance_names: BTreeMap::new(),
        applications: BTreeMap::new(),
        destructors: Vec::new(),
        uses: BTreeMap::new(),
    };
    for i in 0..c.types.len() {
        c.type_names.insert(c.types[i].name.clone(), i);
    }
    for (name, bits, signed) in [
        ("u64", 64, false),
        ("isize", 64, true),
        ("u16", 16, false),
        ("u32", 32, false),
        ("i8", 8, true),
        ("i16", 16, true),
    ] {
        let id = c.types.len();
        c.types.push(k::Type {
            name: name.to_string(),
            kind: k::TypeKind::Int { bits, signed },
        });
        c.type_names.insert(name.to_string(), id);
    }
    let empty = BTreeMap::new();
    for node in &source.exprs {
        if let a::Expr::Path(path) = &node.kind {
            c.uses
                .entry(path.name.clone())
                .or_default()
                .push(node.span.start);
        }
    }
    for positions in c.uses.values_mut() {
        positions.sort_unstable();
    }
    for s in &source.structs {
        if s.generics.is_empty() {
            let _ = c.named_type(
                &a::Path {
                    name: qualified(&s.module, &s.name),
                    args: Vec::new(),
                },
                &empty,
                s.span,
            )?;
        }
    }
    for e in &source.enums {
        if e.generics.is_empty() {
            let _ = c.named_type(
                &a::Path {
                    name: qualified(&e.module, &e.name),
                    args: Vec::new(),
                },
                &empty,
                e.span,
            )?;
        }
    }

    let mut methods = Vec::new();
    for im in &source.impls {
        methods.extend_from_slice(&im.methods);
    }
    for tr in &source.traits {
        methods.extend_from_slice(&tr.methods);
    }
    for i in 0..source.functions.len() {
        let f = &source.functions[i];
        if f.body.is_some() && f.generics.iter().all(|g| g.lifetime) && !methods.contains(&i) {
            c.instantiate(i, empty.clone())?;
        }
    }
    let mut next = 0;
    let mut next_type = 0;
    while next < c.instances.len() || next_type < c.types.len() {
        while next_type < c.types.len() {
            c.register_drop(next_type)?;
            next_type += 1;
        }
        if next < c.instances.len() {
            let instance = c.instances[next].clone();
            let function = c.compile_function(next, &instance)?;
            c.functions[next] = function;
            next += 1;
        }
    }

    Ok(k::Program {
        types: c.types,
        functions: c.functions,
        destructors: c.destructors,
    })
}

impl Compiler<'_> {
    fn type_text(&self, id: usize) -> String {
        match &self.source.types[id].kind {
            a::Type::Path(p) => p.name.clone(),
            _ => format!("{id}"),
        }
    }
    fn intern(&mut self, name: String, kind: k::TypeKind) -> usize {
        if let Some(id) = self.type_names.get(&name) {
            return *id;
        }
        let id = self.types.len();
        self.types.push(k::Type {
            name: name.clone(),
            kind,
        });
        self.type_names.insert(name, id);
        id
    }
    fn reference(&mut self, inner: usize, mutable: bool) -> usize {
        self.intern(
            format!(
                "&{}{}",
                if mutable { "mut " } else { "" },
                self.types[inner].name
            ),
            k::TypeKind::Ref { inner, mutable },
        )
    }
    fn resolve(&mut self, id: usize, sub: &BTreeMap<String, usize>) -> Result<usize> {
        let node = self.source.types[id].clone();
        match node.kind {
            a::Type::Infer => Err(err(node.span, "E_TYPE", "type needs an annotation")),
            a::Type::Path(p) => self.named_type(&p, sub, node.span),
            a::Type::Ref { mutable, inner, .. } => {
                let inner = self.resolve(inner, sub)?;
                Ok(self.reference(inner, mutable))
            }
            a::Type::Tuple(fields) => {
                if fields.is_empty() {
                    return Ok(k::UNIT);
                }
                let mut fs = Vec::new();
                let mut names = Vec::new();
                for i in 0..fields.len() {
                    let t = self.resolve(fields[i], sub)?;
                    fs.push(k::Field {
                        name: i.to_string(),
                        ty: t,
                    });
                    names.push(self.types[t].name.clone());
                }
                let copy = fs.iter().all(|f| is_copy(&self.types, f.ty));
                Ok(self.intern(
                    format!("({})", names.join(",")),
                    k::TypeKind::Struct { fields: fs, copy },
                ))
            }
            a::Type::Slice(inner) => {
                let t = self.resolve(inner, sub)?;
                Ok(self.intern(
                    format!("[{}]", self.types[t].name),
                    k::TypeKind::Slice {
                        inner: t,
                        mutable: false,
                    },
                ))
            }
            a::Type::Array { inner, len } => {
                let inner = self.resolve(inner, sub)?;
                let len = len.replace('_', "").parse::<usize>().map_err(|_| {
                    err(
                        node.span,
                        "E_TYPE",
                        "array length must be an integer literal",
                    )
                })?;
                Ok(self.intern(
                    format!("[{};{}]", self.types[inner].name, len),
                    k::TypeKind::Array { inner, len },
                ))
            }
            a::Type::Never => Ok(k::UNIT),
            _ => Err(err(
                node.span,
                "E_UNSUPPORTED",
                "this type form is not implemented yet",
            )),
        }
    }
    fn named_type(
        &mut self,
        path: &a::Path,
        sub: &BTreeMap<String, usize>,
        span: a::Span,
    ) -> Result<usize> {
        if let Some(id) = sub.get(&path.name) {
            return Ok(*id);
        }
        let name = short(&path.name);
        if path.args.is_empty() {
            if let Some(id) = self
                .type_names
                .get(&path.name)
                .or_else(|| self.type_names.get(name))
            {
                return Ok(*id);
            }
        }
        let mut args = Vec::new();
        for arg in &path.args {
            if !matches!(&self.source.types[*arg].kind,a::Type::Path(p) if p.name.starts_with("'"))
            {
                args.push(self.resolve(*arg, sub)?);
            }
        }
        if args.is_empty() {
            let mut parameters = Vec::new();
            if let Some(s) = self.source.structs.iter().find(|s| s.name == path.name) {
                parameters = s.generics.clone();
            }
            if let Some(e) = self.source.enums.iter().find(|e| e.name == path.name) {
                parameters = e.generics.clone();
            }
            for p in &parameters {
                if !p.lifetime {
                    if let Some(t) = sub.get(&p.name) {
                        args.push(*t);
                    }
                }
            }
        }
        let key = if args.is_empty() {
            path.name.clone()
        } else {
            format!(
                "{}<{}>",
                path.name,
                args.iter()
                    .map(|t| self.types[*t].name.clone())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        if let Some(id) = self.type_names.get(&key) {
            return Ok(*id);
        }
        if name == "Vec" || name == "Box" {
            if args.len() != 1 {
                return Err(err(span, "E_TYPE", "collection requires one type argument"));
            }
            return Ok(self.intern(
                key,
                if name == "Vec" {
                    k::TypeKind::Vec { inner: args[0] }
                } else {
                    k::TypeKind::Box { inner: args[0] }
                },
            ));
        }
        if name == "World" {
            return Ok(self.intern(key, k::TypeKind::World));
        }
        if let Some(s) = self
            .source
            .structs
            .iter()
            .find(|s| qualified(&s.module, &s.name) == path.name || s.name == path.name)
            .cloned()
        {
            let params: Vec<_> = s.generics.iter().filter(|g| !g.lifetime).collect();
            if args.len() != params.len() {
                return Err(err(span, "E_TYPE", "wrong number of type arguments"));
            }
            let mut bound = sub.clone();
            for i in 0..args.len() {
                bound.insert(params[i].name.clone(), args[i]);
            }
            let id = self.types.len();
            self.type_names.insert(key.clone(), id);
            self.types.push(k::Type {
                name: key,
                kind: k::TypeKind::Unit,
            });
            let mut fields = Vec::new();
            for f in s.fields {
                fields.push(k::Field {
                    name: f.name,
                    ty: self.resolve(f.ty, &bound)?,
                });
            }
            let copy = s
                .attrs
                .iter()
                .any(|a| a.name == "derive" && a.args.iter().any(|s| s == "Copy"));
            if copy && fields.iter().any(|f| !is_copy(&self.types, f.ty)) {
                return Err(err(span, "E_TYPE", "Copy requires Copy fields"));
            }
            self.types[id].kind = k::TypeKind::Struct { fields, copy };
            self.applications.insert(id, (path.name.clone(), args));
            return Ok(id);
        }
        if let Some(e) = self
            .source
            .enums
            .iter()
            .find(|e| qualified(&e.module, &e.name) == path.name || e.name == path.name)
            .cloned()
        {
            let params: Vec<_> = e.generics.iter().filter(|g| !g.lifetime).collect();
            if args.len() != params.len() {
                return Err(err(span, "E_TYPE", "wrong number of type arguments"));
            }
            let mut bound = sub.clone();
            for i in 0..args.len() {
                bound.insert(params[i].name.clone(), args[i]);
            }
            let id = self.types.len();
            self.type_names.insert(key.clone(), id);
            self.types.push(k::Type {
                name: key,
                kind: k::TypeKind::Unit,
            });
            let mut variants = Vec::new();
            for v in e.variants {
                let mut fields = Vec::new();
                for f in v.fields {
                    fields.push(k::Field {
                        name: f.name,
                        ty: self.resolve(f.ty, &bound)?,
                    });
                }
                variants.push(k::Variant {
                    name: v.name,
                    fields,
                });
            }
            let copy = e
                .attrs
                .iter()
                .any(|a| a.name == "derive" && a.args.iter().any(|s| s == "Copy"));
            self.types[id].kind = k::TypeKind::Enum { variants, copy };
            self.applications.insert(id, (path.name.clone(), args));
            return Ok(id);
        }
        if let Some(alias) = self
            .source
            .aliases
            .iter()
            .find(|a| a.name == path.name)
            .cloned()
        {
            return self.resolve(alias.ty, sub);
        }
        Err(err(span, "E_TYPE", &format!("unknown type {}", path.name)))
    }
    fn has_view(&self, ty: usize) -> bool {
        let mut todo = vec![ty];
        let mut seen = vec![false; self.types.len()];
        while let Some(t) = todo.pop() {
            if seen[t] {
                continue;
            }
            seen[t] = true;
            match &self.types[t].kind {
                k::TypeKind::Ref { .. } | k::TypeKind::Slice { .. } => return true,
                k::TypeKind::Struct { fields, .. } => {
                    for f in fields {
                        todo.push(f.ty);
                    }
                }
                k::TypeKind::Enum { variants, .. } => {
                    for v in variants {
                        for f in &v.fields {
                            todo.push(f.ty);
                        }
                    }
                }
                k::TypeKind::Array { inner, .. } => todo.push(*inner),
                _ => {}
            }
        }
        false
    }
    fn lifetimes(&self, id: usize) -> Vec<String> {
        let mut found = Vec::new();
        let mut todo = vec![id];
        while let Some(t) = todo.pop() {
            match &self.source.types[t].kind {
                a::Type::Ref {
                    lifetime, inner, ..
                } => {
                    if !lifetime.is_empty() && lifetime != "'_" && !found.contains(lifetime) {
                        found.push(lifetime.clone());
                    }
                    todo.push(*inner);
                }
                a::Type::Path(p) => {
                    if p.name.starts_with("'") && p.name != "'_" && !found.contains(&p.name) {
                        found.push(p.name.clone());
                    }
                    todo.extend_from_slice(&p.args);
                }
                a::Type::Tuple(ts) => todo.extend_from_slice(ts),
                a::Type::Slice(t) | a::Type::Array { inner: t, .. } => todo.push(*t),
                _ => {}
            }
        }
        found
    }
    fn return_source(
        &self,
        source: &a::Function,
        locals: &[k::Local],
        result: usize,
    ) -> Result<Option<usize>> {
        if !self.has_view(result) {
            return Ok(None);
        }
        let lifetimes = self.lifetimes(source.result);
        if lifetimes.len() > 1 {
            return Err(err(
                source.span,
                "E_BORROW",
                "a returned view has one source lifetime",
            ));
        }
        let mut candidates = Vec::new();
        if let Some(lifetime) = lifetimes.first() {
            for i in 0..source.params.len() {
                if self.lifetimes(source.params[i].ty).contains(lifetime) {
                    candidates.push(i);
                }
            }
        } else if source.params.first().is_some_and(|p| p.name == "self")
            && self.has_view(locals[0].ty)
        {
            candidates.push(0);
        } else {
            for i in 0..source.params.len() {
                if self.has_view(locals[i].ty) {
                    candidates.push(i);
                }
            }
        }
        if candidates.len() > 1 {
            return Err(err(
                source.span,
                "E_BORROW",
                "returned view selects more than one source input",
            ));
        }
        Ok(candidates.first().copied())
    }
    fn instantiate(&mut self, index: usize, sub: BTreeMap<String, usize>) -> Result<usize> {
        let source = self.source.functions[index].clone();
        let mut name = qualified(&source.module, &source.name);
        if !sub.is_empty() {
            name.push('<');
            for (key, value) in &sub {
                name.push_str(key);
                name.push('=');
                name.push_str(&self.types[*value].name);
                name.push(';');
            }
            name.push('>');
        }
        let key = format!("{index}:{name}");
        if let Some(id) = self.instance_names.get(&key) {
            return Ok(*id);
        }
        self.bounds(&source.generics, &sub)?;
        let mut locals = Vec::new();
        for p in &source.params {
            locals.push(k::Local {
                name: p.name.clone(),
                ty: self.resolve(p.ty, &sub)?,
                mutable: p.mutable,
            });
        }
        let result = self.resolve(source.result, &sub)?;
        let return_source = self.return_source(&source, &locals, result)?;
        let id = self.functions.len();
        self.instance_names.insert(key, id);
        self.instances.push(Instance {
            source: index,
            substitutions: sub,
        });
        self.functions.push(k::Function {
            name,
            params: locals.len(),
            result,
            return_source,
            locals,
            body: Vec::new(),
            test: source.attrs.iter().any(|a| a.name == "test"),
        });
        Ok(id)
    }
    fn bounds(&self, parameters: &[a::GenericParam], sub: &BTreeMap<String, usize>) -> Result<()> {
        for parameter in parameters {
            if parameter.lifetime {
                continue;
            }
            let ty = *sub
                .get(&parameter.name)
                .ok_or_else(|| err(parameter.span, "E_TYPE", "cannot infer generic argument"))?;
            for id in &parameter.bounds {
                let bound = match &self.source.types[*id].kind {
                    a::Type::Path(p) if p.args.is_empty() => p.name.as_str(),
                    _ => {
                        return Err(err(
                            parameter.span,
                            "E_UNSUPPORTED",
                            "parameterized trait bounds are not implemented yet",
                        ));
                    }
                };
                if !crate::generics::satisfies(
                    self.source,
                    &self.types,
                    &self.applications,
                    ty,
                    bound,
                ) {
                    return Err(err(
                        parameter.span,
                        "E_TRAIT",
                        &format!("{} does not implement {}", self.types[ty].name, bound),
                    ));
                }
            }
        }
        Ok(())
    }
    fn register_drop(&mut self, ty: usize) -> Result<()> {
        if !matches!(
            self.types[ty].kind,
            k::TypeKind::Struct { .. } | k::TypeKind::Enum { .. }
        ) {
            return Ok(());
        }
        let impls = self.source.impls.clone();
        for im in impls {
            if let Some(trait_ty) = im.trait_ty {
                if self.type_text(trait_ty) == "Drop" {
                    let mut sub = BTreeMap::new();
                    let pattern = match self.source.types[im.target].kind.clone() {
                        a::Type::Path(p) => p,
                        _ => return Err(err(im.span, "E_TRAIT", "Drop requires a nominal type")),
                    };
                    let (name, actual) = self
                        .applications
                        .get(&ty)
                        .cloned()
                        .unwrap_or((self.types[ty].name.clone(), Vec::new()));
                    if short(&pattern.name) != short(&name) {
                        continue;
                    }
                    let parameters=pattern.args.iter().filter(|p|!matches!(&self.source.types[**p].kind,a::Type::Path(path) if path.name.starts_with("'"))).copied().collect::<Vec<_>>();
                    if parameters.len() != actual.len() {
                        continue;
                    }
                    for i in 0..parameters.len() {
                        match &self.source.types[parameters[i]].kind {
                            a::Type::Path(p) if p.args.is_empty() => {
                                sub.insert(p.name.clone(), actual[i]);
                            }
                            _ => {
                                return Err(err(
                                    im.span,
                                    "E_TRAIT",
                                    "Drop implementations cannot specialize type arguments",
                                ));
                            }
                        }
                    }
                    sub.insert("Self".to_string(), ty);
                    self.bounds(&im.generics, &sub)?;
                    let methods = im
                        .methods
                        .iter()
                        .filter(|i| self.source.functions[**i].name == "drop")
                        .copied()
                        .collect::<Vec<_>>();
                    if methods.len() != 1 {
                        return Err(err(im.span, "E_TRAIT", "Drop requires one drop method"));
                    }
                    let function = self.instantiate(methods[0], sub)?;
                    self.destructors.push((ty, function));
                }
            }
        }
        Ok(())
    }
    fn compile_function(&mut self, index: usize, instance: &Instance) -> Result<k::Function> {
        let source = self.source.functions[instance.source].clone();
        let mut function = self.functions[index].clone();
        let bindings = function
            .locals
            .iter()
            .enumerate()
            .map(|(i, l)| (l.name.clone(), i))
            .collect();
        let live = vec![true; function.locals.len()];
        let mut builder = Builder {
            compiler: self,
            function: &mut function,
            sub: instance.substitutions.clone(),
            bindings,
            ref_bindings: Vec::new(),
            written: live.clone(),
            live,
            loops: Vec::new(),
            reachable: true,
        };
        let result_ty = builder.function.result;
        let result = builder.block(
            source
                .body
                .ok_or_else(|| err(source.span, "E_TYPE", "function has no body"))?,
            Some(result_ty),
        )?;
        if builder.reachable {
            if result.1 != result_ty {
                return Err(err(source.span, "E_TYPE", "function result type differs"));
            }
            builder.cleanup(0, Some(result.0), source.span.line);
            builder.emit(k::Op::Return { src: result.0 }, source.span.line);
        }
        Ok(function)
    }
}

#[derive(Clone)]
struct Snapshot {
    live: Vec<bool>,
    written: Vec<bool>,
}
struct LoopScope {
    start: usize,
    range: Option<(usize, usize, bool)>,
    exits: Vec<Snapshot>,
}
struct Builder<'a, 'b> {
    compiler: &'a mut Compiler<'b>,
    function: &'a mut k::Function,
    sub: BTreeMap<String, usize>,
    bindings: Vec<(String, usize)>,
    ref_bindings: Vec<(String, usize)>,
    live: Vec<bool>,
    written: Vec<bool>,
    loops: Vec<LoopScope>,
    reachable: bool,
}
impl Builder<'_, '_> {
    fn local(&mut self, ty: usize, name: &str, mutable: bool) -> usize {
        let id = self.function.locals.len();
        self.function.locals.push(k::Local {
            name: name.to_string(),
            ty,
            mutable,
        });
        self.live.push(false);
        self.written.push(false);
        id
    }
    fn temp(&mut self, ty: usize) -> usize {
        self.local(ty, "$", false)
    }
    fn ty(&self, id: usize) -> usize {
        self.function.locals[id].ty
    }
    fn binding(&self, name: &str) -> Option<usize> {
        self.bindings
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, i)| *i)
    }
    fn emit(&mut self, op: k::Op, line: usize) {
        if let k::Op::Assign { place, src } = &op {
            if self.live[place.local] && place.local != *src {
                if let Some(t) = place_type(&self.compiler.types, self.function, place) {
                    if !matches!(
                        self.compiler.types[t].kind,
                        k::TypeKind::Unit
                            | k::TypeKind::Bool
                            | k::TypeKind::Int { .. }
                            | k::TypeKind::F64
                    ) {
                        self.emit(
                            k::Op::Drop {
                                place: place.clone(),
                            },
                            line,
                        );
                    }
                }
            }
        }
        match &op {
            k::Op::Int { dst, .. }
            | k::Op::Float { dst, .. }
            | k::Op::Unit { dst }
            | k::Op::Borrow { dst, .. }
            | k::Op::SliceBorrow { dst, .. }
            | k::Op::Discriminant { dst, .. }
            | k::Op::TagTest { dst, .. } => self.live[*dst] = true,
            k::Op::Read { dst, place, moving } => {
                if *moving && place.projections.is_empty() {
                    self.live[place.local] = false;
                }
                self.live[*dst] = true;
            }
            k::Op::Assign { place, src } => {
                self.live[*src] = false;
                self.live[place.local] = true;
                self.written[place.local] = true;
            }
            k::Op::Drop { place } => {
                if place.projections.is_empty() {
                    self.live[place.local] = false;
                }
            }
            k::Op::Unary { dst, src, .. } => {
                self.live[*src] = false;
                self.live[*dst] = true;
            }
            k::Op::Binary {
                dst, left, right, ..
            } => {
                self.live[*left] = false;
                self.live[*right] = false;
                self.live[*dst] = true;
            }
            k::Op::Call { dst, args, .. } | k::Op::Intrinsic { dst, args, .. } => {
                for arg in args {
                    self.live[*arg] = false;
                }
                self.live[*dst] = true;
            }
            k::Op::Struct { dst, fields }
            | k::Op::Variant { dst, fields, .. }
            | k::Op::Array {
                dst,
                elements: fields,
            } => {
                for field in fields {
                    self.live[*field] = false;
                }
                self.live[*dst] = true;
            }
            k::Op::If { condition } => self.live[*condition] = false,
            k::Op::Return { src } => {
                self.live[*src] = false;
                self.reachable = false;
            }
            k::Op::Break => {
                let state = self.snapshot();
                if let Some(scope) = self.loops.last_mut() {
                    scope.exits.push(state);
                }
                self.reachable = false;
            }
            k::Op::Continue | k::Op::Trap { .. } => self.reachable = false,
            _ => {}
        }
        self.function.body.push(k::Instruction { op, line });
    }
    fn cleanup(&mut self, start: usize, except: Option<usize>, line: usize) {
        for i in (start..self.live.len()).rev() {
            if self.live[i] && Some(i) != except {
                self.emit(
                    k::Op::Drop {
                        place: k::Place::local(i),
                    },
                    line,
                );
            }
        }
    }
    fn unit(&mut self, line: usize) -> (usize, usize) {
        let d = self.temp(k::UNIT);
        self.emit(k::Op::Unit { dst: d }, line);
        (d, k::UNIT)
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            live: self.live.clone(),
            written: self.written.clone(),
        }
    }
    fn restore(&mut self, state: &Snapshot) {
        self.live = state.live.clone();
        self.live.resize(self.function.locals.len(), false);
        self.written = state.written.clone();
        self.written.resize(self.function.locals.len(), false);
        self.reachable = true;
    }
    fn read(&mut self, place: k::Place, line: usize) -> Result<(usize, usize)> {
        let ty = place_type(&self.compiler.types, self.function, &place).ok_or_else(|| {
            err(
                a::Span {
                    line,
                    ..a::Span::default()
                },
                "E_TYPE",
                "invalid place",
            )
        })?;
        let dst = self.temp(ty);
        self.emit(
            k::Op::Read {
                dst,
                place,
                moving: !is_copy(&self.compiler.types, ty),
            },
            line,
        );
        Ok((dst, ty))
    }
    fn borrow(&mut self, place: k::Place, mutable: bool, line: usize) -> Result<(usize, usize)> {
        let ty = place_type(&self.compiler.types, self.function, &place).ok_or_else(|| {
            err(
                a::Span {
                    line,
                    ..a::Span::default()
                },
                "E_TYPE",
                "invalid borrowed place",
            )
        })?;
        let rt = self.compiler.reference(ty, mutable);
        let dst = self.temp(rt);
        self.emit(
            k::Op::Borrow {
                dst,
                place,
                mutable,
            },
            line,
        );
        Ok((dst, rt))
    }
    fn hint(&mut self, id: usize) -> Option<usize> {
        match self.compiler.source.exprs[id].kind.clone() {
            a::Expr::Path(p) => self.binding(&p.name).map(|i| self.ty(i)),
            a::Expr::Cast { ty, .. } => self.compiler.resolve(ty, &self.sub).ok(),
            a::Expr::Bool(_) => Some(k::BOOL),
            a::Expr::Float(_) => Some(k::F64),
            a::Expr::Int(s) => {
                for (name, t) in &self.compiler.type_names {
                    if s.ends_with(name.as_str()) && s.len() > name.len() {
                        return Some(*t);
                    }
                }
                None
            }
            a::Expr::Field { value, name } => {
                let mut t = self.hint(value)?;
                while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
                    self.compiler.types[t].kind
                {
                    t = inner;
                }
                match &self.compiler.types[t].kind {
                    k::TypeKind::Struct { fields, .. } => {
                        fields.iter().find(|f| f.name == name).map(|f| f.ty)
                    }
                    _ => None,
                }
            }
            a::Expr::Index { value, .. } => {
                let mut t = self.hint(value)?;
                while let k::TypeKind::Ref { inner, .. } = self.compiler.types[t].kind {
                    t = inner;
                }
                match self.compiler.types[t].kind {
                    k::TypeKind::Vec { inner }
                    | k::TypeKind::Slice { inner, .. }
                    | k::TypeKind::Array { inner, .. } => Some(inner),
                    _ => None,
                }
            }
            a::Expr::Method { name, .. } if name == "len" => Some(k::U64),
            _ => None,
        }
    }
    fn place(&mut self, id: usize) -> Result<k::Place> {
        let node = self.compiler.source.exprs[id].clone();
        match node.kind {
            a::Expr::Path(path) => self
                .binding(&path.name)
                .map(k::Place::local)
                .ok_or_else(|| err(node.span, "E_TYPE", &format!("unknown value {}", path.name))),
            a::Expr::Unary { op, value } if op == "*" => {
                let mut place = self.addressable(value)?;
                place.projections.push(k::Projection::Deref);
                Ok(place)
            }
            a::Expr::Field { value, name } => {
                let mut place = self.addressable(value)?;
                let mut ty = place_type(&self.compiler.types, self.function, &place)
                    .ok_or_else(|| err(node.span, "E_TYPE", "invalid field base"))?;
                while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
                    self.compiler.types[ty].kind
                {
                    place.projections.push(k::Projection::Deref);
                    ty = inner;
                }
                if let k::TypeKind::Struct { fields, .. } = &self.compiler.types[ty].kind {
                    let index = fields
                        .iter()
                        .position(|f| f.name == name)
                        .ok_or_else(|| err(node.span, "E_TYPE", "unknown field"))?;
                    place.projections.push(k::Projection::Field(index));
                    Ok(place)
                } else {
                    Err(err(node.span, "E_TYPE", "field access requires a struct"))
                }
            }
            a::Expr::Index { value, index } => {
                let mut place = self.addressable(value)?;
                let mut ty = place_type(&self.compiler.types, self.function, &place)
                    .ok_or_else(|| err(node.span, "E_TYPE", "invalid index base"))?;
                while let k::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
                    place.projections.push(k::Projection::Deref);
                    ty = inner;
                }
                let (index, _) = self.expr(index, Some(k::U64))?;
                place.projections.push(k::Projection::Index(index));
                Ok(place)
            }
            _ => Err(err(node.span, "E_TYPE", "expression is not a place")),
        }
    }
    fn addressable(&mut self, id: usize) -> Result<k::Place> {
        match self.compiler.source.exprs[id].kind {
            a::Expr::Path(_)
            | a::Expr::Field { .. }
            | a::Expr::Index { .. }
            | a::Expr::Unary { .. } => self.place(id),
            _ => {
                let v = self.expr(id, None)?;
                Ok(k::Place::local(v.0))
            }
        }
    }
    fn end_unused_borrows(&mut self, start: usize, position: usize, end: usize, line: usize) {
        let mut drops = Vec::new();
        for (name, local) in &self.ref_bindings {
            if *local >= start && self.live[*local] {
                let mut used = false;
                if let Some(positions) = self.compiler.uses.get(name) {
                    let mut low = 0;
                    let mut high = positions.len();
                    while low < high {
                        let mid = low + (high - low) / 2;
                        if positions[mid] < position {
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                    used = low < positions.len() && positions[low] < end;
                }
                if !used {
                    drops.push(*local);
                }
            }
        }
        for local in drops {
            self.emit(
                k::Op::Drop {
                    place: k::Place::local(local),
                },
                line,
            );
        }
    }
    fn bind(&mut self, pattern: usize, value: (usize, usize), line: usize) -> Result<()> {
        let p = self.compiler.source.patterns[pattern].clone();
        match p.kind {
            a::Pattern::Bind {
                name,
                mutable,
                by_ref,
                ref_mut,
            } => {
                let val = if by_ref {
                    self.borrow(k::Place::local(value.0), ref_mut, line)?
                } else {
                    value
                };
                let dst = self.local(val.1, &name, mutable);
                self.emit(
                    k::Op::Assign {
                        place: k::Place::local(dst),
                        src: val.0,
                    },
                    line,
                );
                if matches!(self.compiler.types[val.1].kind, k::TypeKind::Ref { .. }) {
                    self.ref_bindings.push((name.clone(), dst));
                }
                self.bindings.push((name, dst));
                Ok(())
            }
            a::Pattern::Wild => {
                self.emit(
                    k::Op::Drop {
                        place: k::Place::local(value.0),
                    },
                    line,
                );
                Ok(())
            }
            a::Pattern::Tuple(items) => {
                for (i, item) in items.iter().enumerate() {
                    let v = self.read(
                        k::Place {
                            local: value.0,
                            projections: vec![k::Projection::Field(i)],
                        },
                        line,
                    )?;
                    self.bind(*item, v, line)?;
                }
                Ok(())
            }
            _ => Err(err(
                p.span,
                "E_UNSUPPORTED",
                "this binding pattern is not implemented yet",
            )),
        }
    }
    fn extends_temporary(&self, value: usize, pattern: usize) -> bool {
        if matches!(
            self.compiler.source.patterns[pattern].kind,
            a::Pattern::Bind { by_ref: true, .. }
        ) {
            return true;
        }
        let mut todo = vec![value];
        while let Some(id) = todo.pop() {
            match &self.compiler.source.exprs[id].kind {
                a::Expr::Borrow { .. } => return true,
                a::Expr::Tuple(items) | a::Expr::Array(items) => todo.extend_from_slice(items),
                a::Expr::Struct { fields, .. } => {
                    for (_, value) in fields {
                        todo.push(*value);
                    }
                }
                _ => {}
            }
        }
        false
    }
    fn cleanup_temporaries(&mut self, start: usize, line: usize) {
        for local in (start..self.live.len()).rev() {
            if self.live[local] && self.function.locals[local].name.starts_with('$') {
                self.emit(
                    k::Op::Drop {
                        place: k::Place::local(local),
                    },
                    line,
                );
            }
        }
    }
    fn block(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
        let block = self.compiler.source.blocks[id].clone();
        let start = self.live.len();
        let bindings = self.bindings.len();
        for stmt in block.statements {
            if !self.reachable {
                break;
            }
            let end = match &stmt {
                a::Stmt::Let { span, .. } => span.end,
                a::Stmt::Expr(id) => self.compiler.source.exprs[*id].span.end,
                _ => 0,
            };
            match stmt {
                a::Stmt::Let {
                    pattern,
                    ty,
                    value,
                    span,
                } => {
                    let expected = match ty {
                        Some(t) => Some(self.compiler.resolve(t, &self.sub)?),
                        None => None,
                    };
                    if let Some(value) = value {
                        let before = self.live.len();
                        let extends = self.extends_temporary(value, pattern);
                        let v = self.expr(value, expected)?;
                        if let Some(t) = expected {
                            if t != v.1 {
                                return Err(err(span, "E_TYPE", "binding type differs"));
                            }
                        }
                        self.bind(pattern, v, span.line)?;
                        if !extends {
                            self.cleanup_temporaries(before, span.line);
                        }
                    } else {
                        let p = self.compiler.source.patterns[pattern].clone();
                        if let a::Pattern::Bind { name, mutable, .. } = p.kind {
                            let t = expected.ok_or_else(|| {
                                err(span, "E_TYPE", "uninitialized binding requires a type")
                            })?;
                            let local = self.local(t, &name, mutable);
                            self.bindings.push((name, local));
                        } else {
                            return Err(err(span, "E_UNSUPPORTED", "uninitialized destructuring"));
                        }
                    }
                }
                a::Stmt::Expr(expr) => {
                    let before = self.live.len();
                    let _ = self.expr(expr, None)?;
                    self.cleanup(before, None, block.span.line);
                }
                a::Stmt::Item(_) => {
                    return Err(err(
                        block.span,
                        "E_UNSUPPORTED",
                        "local declarations are not implemented yet",
                    ));
                }
            }
            if self.reachable {
                self.end_unused_borrows(start, end, block.span.end, block.span.line);
            }
        }
        let result = if self.reachable {
            if let Some(tail) = block.tail {
                self.expr(tail, expected)?
            } else {
                self.unit(block.span.line)
            }
        } else {
            self.unit(block.span.line)
        };
        self.cleanup(start, Some(result.0), block.span.line);
        self.bindings.truncate(bindings);
        Ok(result)
    }
    fn expr(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
        if let Some(target) = expected {
            if let k::TypeKind::Ref {
                inner,
                mutable: true,
            } = self.compiler.types[target].kind
            {
                if let a::Expr::Path(path) = &self.compiler.source.exprs[id].kind {
                    if let Some(local) = self.binding(&path.name) {
                        if self.ty(local) == target {
                            return self.borrow(
                                k::Place {
                                    local,
                                    projections: vec![k::Projection::Deref],
                                },
                                true,
                                self.compiler.source.exprs[id].span.line,
                            );
                        }
                        let _ = inner;
                    }
                }
            }
        }
        let value = self.expr_inner(id, expected)?;
        if let Some(target) = expected {
            if target != value.1 {
                if let (
                    k::TypeKind::Ref {
                        inner: want,
                        mutable: want_mut,
                    },
                    k::TypeKind::Ref {
                        inner: have,
                        mutable: have_mut,
                    },
                ) = (
                    self.compiler.types[target].kind.clone(),
                    self.compiler.types[value.1].kind.clone(),
                ) {
                    if !want_mut || have_mut {
                        let mut inner = have;
                        let mut place = k::Place {
                            local: value.0,
                            projections: vec![k::Projection::Deref],
                        };
                        if let k::TypeKind::Slice { inner: element, .. } =
                            self.compiler.types[want].kind
                        {
                            if matches!(self.compiler.types[inner].kind,k::TypeKind::Vec{inner:t}|k::TypeKind::Array{inner:t,..} if t==element)
                            {
                                let dst = self.temp(target);
                                self.emit(
                                    k::Op::SliceBorrow {
                                        dst,
                                        place,
                                        mutable: want_mut,
                                    },
                                    self.compiler.source.exprs[id].span.line,
                                );
                                self.emit(
                                    k::Op::Drop {
                                        place: k::Place::local(value.0),
                                    },
                                    self.compiler.source.exprs[id].span.line,
                                );
                                return Ok((dst, target));
                            }
                        }
                        while inner != want {
                            if let k::TypeKind::Box { inner: next } =
                                self.compiler.types[inner].kind
                            {
                                inner = next;
                                place.projections.push(k::Projection::Deref);
                            } else {
                                break;
                            }
                        }
                        if inner == want {
                            let result = self.borrow(
                                place,
                                want_mut,
                                self.compiler.source.exprs[id].span.line,
                            )?;
                            self.emit(
                                k::Op::Drop {
                                    place: k::Place::local(value.0),
                                },
                                self.compiler.source.exprs[id].span.line,
                            );
                            return Ok(result);
                        }
                    }
                }
            }
        }
        Ok(value)
    }
    fn integer(
        &mut self,
        text: &str,
        expected: Option<usize>,
        negative: bool,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let mut digits = text.replace('_', "");
        let mut ty = expected.unwrap_or(k::I32);
        let suffixes = [
            "usize", "isize", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
        ];
        for suffix in suffixes {
            if digits.ends_with(suffix) {
                ty = *self
                    .compiler
                    .type_names
                    .get(suffix)
                    .ok_or_else(|| err(span, "E_TYPE", "unknown integer suffix"))?;
                digits.truncate(digits.len() - suffix.len());
                break;
            }
        }
        let radix = if digits.starts_with("0x") {
            16
        } else if digits.starts_with("0b") {
            2
        } else if digits.starts_with("0o") {
            8
        } else {
            10
        };
        let magnitude =
            u64::from_str_radix(if radix == 10 { &digits } else { &digits[2..] }, radix)
                .map_err(|_| err(span, "E_TYPE", "integer literal exceeds its representation"))?;
        let (bits, signed) = match self.compiler.types[ty].kind {
            k::TypeKind::Int { bits, signed } => (bits, signed),
            _ => {
                return Err(err(
                    span,
                    "E_TYPE",
                    "integer literal requires an integer type",
                ));
            }
        };
        if negative && !signed {
            return Err(err(span, "E_TYPE", "unsigned integers cannot be negated"));
        }
        let maximum = if signed {
            if negative {
                1_u64 << (bits - 1)
            } else {
                (1_u64 << (bits - 1)) - 1
            }
        } else if bits == 64 {
            u64::MAX
        } else {
            (1_u64 << bits) - 1
        };
        if magnitude > maximum {
            return Err(err(span, "E_TYPE", "integer literal does not fit its type"));
        }
        let value = if negative {
            0_u64.wrapping_sub(magnitude) as i64
        } else {
            magnitude as i64
        };
        let dst = self.temp(ty);
        self.emit(k::Op::Int { dst, value }, span.line);
        Ok((dst, ty))
    }
    fn expr_inner(&mut self, id: usize, expected: Option<usize>) -> Result<(usize, usize)> {
        let node = self.compiler.source.exprs[id].clone();
        let line = node.span.line;
        match node.kind {
            a::Expr::Int(text) => self.integer(&text, expected, false, node.span),
            a::Expr::Bool(value) => {
                let d = self.temp(k::BOOL);
                self.emit(
                    k::Op::Int {
                        dst: d,
                        value: i64::from(value),
                    },
                    line,
                );
                Ok((d, k::BOOL))
            }
            a::Expr::Float(text) => {
                let value = text
                    .trim_end_matches("f64")
                    .replace('_', "")
                    .parse::<f64>()
                    .map_err(|_| err(node.span, "E_TYPE", "invalid float literal"))?;
                if !value.is_finite() {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "floating-point literal exceeds f64",
                    ));
                }
                let dst = self.temp(k::F64);
                self.emit(
                    k::Op::Float {
                        dst,
                        bits: value.to_bits(),
                    },
                    line,
                );
                Ok((dst, k::F64))
            }
            a::Expr::Path(path) => {
                if self.binding(&path.name).is_some() {
                    let place = self.place(id)?;
                    return self.read(place, line);
                }
                if let Some(constant) = self
                    .compiler
                    .source
                    .constants
                    .iter()
                    .find(|c| c.name == path.name)
                    .cloned()
                {
                    let t = self.compiler.resolve(constant.ty, &self.sub)?;
                    return self.expr(constant.value, Some(t));
                }
                if path.name.contains("::")
                    || ["None", "Some", "Ok", "Err"].contains(&path.name.as_str())
                {
                    if let Ok((ty, tag)) = self.variant(&path, expected, node.span) {
                        let d = self.temp(ty);
                        self.emit(
                            k::Op::Variant {
                                dst: d,
                                variant: tag,
                                fields: Vec::new(),
                            },
                            line,
                        );
                        return Ok((d, ty));
                    }
                }
                Err(err(
                    node.span,
                    "E_TYPE",
                    &format!("unknown value {}", path.name),
                ))
            }
            a::Expr::Field { .. } | a::Expr::Index { .. } => {
                let place = self.place(id)?;
                self.read(place, line)
            }
            a::Expr::Borrow { mutable, value } => {
                let place = if matches!(
                    self.compiler.source.exprs[value].kind,
                    a::Expr::Path(_)
                        | a::Expr::Field { .. }
                        | a::Expr::Index { .. }
                        | a::Expr::Unary { .. }
                ) {
                    self.place(value)?
                } else {
                    let inner = expected.and_then(|t| match self.compiler.types[t].kind {
                        k::TypeKind::Ref { inner, .. } => Some(inner),
                        _ => None,
                    });
                    let v = self.expr(value, inner)?;
                    if mutable {
                        self.function.locals[v.0].mutable = true;
                    }
                    k::Place::local(v.0)
                };
                self.borrow(place, mutable, line)
            }
            a::Expr::Unary { op, value } => {
                if op == "-" {
                    if let a::Expr::Int(text) = self.compiler.source.exprs[value].kind.clone() {
                        return self.integer(&text, expected, true, node.span);
                    }
                }
                if op == "*" {
                    let place = self.place(id)?;
                    return self.read(place, line);
                }
                let (src, ty) = self.expr(value, expected)?;
                let dst = self.temp(ty);
                self.emit(k::Op::Unary { dst, op, src }, line);
                Ok((dst, ty))
            }
            a::Expr::Binary { op, left, right } => {
                if op == "&&" || op == "||" {
                    return self.short_circuit(&op, left, right, line);
                }
                if ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str()) {
                    if let Some(t) = self.hint(left).or_else(|| self.hint(right)) {
                        if let k::TypeKind::Vec { inner } = self.compiler.types[t].kind {
                            return self.vector_compare(&op, left, right, inner, node.span);
                        }
                    }
                }
                let comparison = ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str());
                let operand = self
                    .hint(left)
                    .or_else(|| self.hint(right))
                    .or(if comparison { None } else { expected });
                let (left, ty) = self.expr(left, operand)?;
                let (right, rt) = self.expr(right, Some(ty))?;
                if ty != rt {
                    return Err(err(node.span, "E_TYPE", "binary operand types differ"));
                }
                let result = if comparison { k::BOOL } else { ty };
                let dst = self.temp(result);
                self.emit(
                    k::Op::Binary {
                        dst,
                        op,
                        left,
                        right,
                    },
                    line,
                );
                Ok((dst, result))
            }
            a::Expr::Assign { op, place, value } => {
                let place = self.place(place)?;
                if place.projections.is_empty()
                    && !self.function.locals[place.local].mutable
                    && self.written[place.local]
                {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "assignment to an immutable binding",
                    ));
                }
                let t = place_type(&self.compiler.types, self.function, &place)
                    .ok_or_else(|| err(node.span, "E_TYPE", "invalid assignment place"))?;
                let v = if op == "=" {
                    self.expr(value, Some(t))?
                } else {
                    let (left, _) = self.read(place.clone(), line)?;
                    let (right, _) = self.expr(value, Some(t))?;
                    let dst = self.temp(t);
                    self.emit(
                        k::Op::Binary {
                            dst,
                            op: op.trim_end_matches('=').to_string(),
                            left,
                            right,
                        },
                        line,
                    );
                    (dst, t)
                };
                self.emit(k::Op::Assign { place, src: v.0 }, line);
                Ok(self.unit(line))
            }
            a::Expr::Cast { value, ty } => {
                let t = self.compiler.resolve(ty, &self.sub)?;
                let (src, _) = self.expr(value, None)?;
                let dst = self.temp(t);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "cast".to_string(),
                        args: vec![src],
                    },
                    line,
                );
                Ok((dst, t))
            }
            a::Expr::Call { callee, args } => self.call(callee, &args, expected, node.span),
            a::Expr::Method {
                receiver,
                name,
                types,
                args,
            } => self.method(receiver, &name, &types, &args, expected, node.span),
            a::Expr::Struct { path, fields, base } => {
                if base.is_some() {
                    return Err(err(
                        node.span,
                        "E_UNSUPPORTED",
                        "struct update is not implemented yet",
                    ));
                }
                self.construct_struct(&path, &fields, expected, node.span)
            }
            a::Expr::Array(elements) => self.array_expr(&elements, expected, node.span),
            a::Expr::Repeat { value, count } => {
                let length = match &self.compiler.source.exprs[count].kind {
                    a::Expr::Int(text) => text.replace('_', "").parse::<usize>().map_err(|_| {
                        err(
                            node.span,
                            "E_TYPE",
                            "array repeat length must be an integer literal",
                        )
                    })?,
                    _ => {
                        return Err(err(
                            node.span,
                            "E_UNSUPPORTED",
                            "array repeat length must be constant",
                        ));
                    }
                };
                let inner = expected.and_then(|t| match self.compiler.types[t].kind {
                    k::TypeKind::Array { inner, .. } => Some(inner),
                    _ => None,
                });
                let v = self.expr(value, inner)?;
                if length > 1 && !is_copy(&self.compiler.types, v.1) {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "repeated array element must be Copy",
                    ));
                }
                let mut elements = Vec::new();
                for _ in 0..length {
                    elements.push(self.read(k::Place::local(v.0), line)?.0);
                }
                let ty = self.compiler.intern(
                    format!("[{};{}]", self.compiler.types[v.1].name, length),
                    k::TypeKind::Array {
                        inner: v.1,
                        len: length,
                    },
                );
                let dst = self.temp(ty);
                self.emit(k::Op::Array { dst, elements }, line);
                if self.live[v.0] {
                    self.emit(
                        k::Op::Drop {
                            place: k::Place::local(v.0),
                        },
                        line,
                    );
                }
                Ok((dst, ty))
            }
            a::Expr::Tuple(items) => {
                if items.is_empty() {
                    return Ok(self.unit(line));
                }
                let mut fields = Vec::new();
                let mut values = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let v = self.expr(*item, None)?;
                    fields.push(k::Field {
                        name: i.to_string(),
                        ty: v.1,
                    });
                    values.push(v.0);
                }
                let names = fields
                    .iter()
                    .map(|f| self.compiler.types[f.ty].name.clone())
                    .collect::<Vec<_>>()
                    .join(",");
                let copy = fields.iter().all(|f| is_copy(&self.compiler.types, f.ty));
                let ty = self
                    .compiler
                    .intern(format!("({names})"), k::TypeKind::Struct { fields, copy });
                let dst = self.temp(ty);
                self.emit(
                    k::Op::Struct {
                        dst,
                        fields: values,
                    },
                    line,
                );
                Ok((dst, ty))
            }
            a::Expr::Block(block) => self.block(block, expected),
            a::Expr::If {
                condition,
                then_block,
                otherwise,
            } => self.if_expr(condition, then_block, otherwise, expected, line),
            a::Expr::While { condition, body } => self.while_expr(Some(condition), body, line),
            a::Expr::Loop { body } => self.while_expr(None, body, line),
            a::Expr::For {
                pattern,
                iterator,
                body,
            } => self.for_expr(pattern, iterator, body, line),
            a::Expr::Match { value, arms } => self.match_expr(value, &arms, expected, node.span),
            a::Expr::Return(value) => {
                let result = if let Some(v) = value {
                    self.expr(v, Some(self.function.result))?
                } else {
                    self.unit(line)
                };
                self.cleanup(0, Some(result.0), line);
                self.emit(k::Op::Return { src: result.0 }, line);
                Ok(result)
            }
            a::Expr::Break(value) => {
                if value.is_some() {
                    return Err(err(
                        node.span,
                        "E_UNSUPPORTED",
                        "break values are not implemented yet",
                    ));
                }
                let start = self
                    .loops
                    .last()
                    .ok_or_else(|| err(node.span, "E_TYPE", "break outside loop"))?
                    .start;
                self.cleanup(start, None, line);
                self.emit(k::Op::Break, line);
                Ok(self.unit(line))
            }
            a::Expr::Continue => {
                let start = self
                    .loops
                    .last()
                    .ok_or_else(|| err(node.span, "E_TYPE", "continue outside loop"))?
                    .start;
                self.advance_range(line)?;
                self.cleanup(start, None, line);
                self.emit(k::Op::Continue, line);
                Ok(self.unit(line))
            }
            a::Expr::Macro {
                name,
                args,
                separator,
            } => self.macro_expr(&name, &args, &separator, expected, node.span),
            a::Expr::Closure { .. } => Err(err(
                node.span,
                "E_UNSUPPORTED",
                "closures are not implemented yet",
            )),
            _ => Err(err(
                node.span,
                "E_UNSUPPORTED",
                "this expression form is not implemented yet",
            )),
        }
    }
    fn array_expr(
        &mut self,
        elements: &[usize],
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let mut inner = expected.and_then(|t| match self.compiler.types[t].kind {
            k::TypeKind::Array { inner, .. } => Some(inner),
            _ => None,
        });
        let mut values = Vec::new();
        for element in elements {
            let value = self.expr(*element, inner)?;
            if inner.is_none() {
                inner = Some(value.1);
            }
            values.push(value.0);
        }
        let inner =
            inner.ok_or_else(|| err(span, "E_TYPE", "empty array needs an element type"))?;
        let ty = self.compiler.intern(
            format!("[{};{}]", self.compiler.types[inner].name, elements.len()),
            k::TypeKind::Array {
                inner,
                len: elements.len(),
            },
        );
        let dst = self.temp(ty);
        self.emit(
            k::Op::Array {
                dst,
                elements: values,
            },
            span.line,
        );
        Ok((dst, ty))
    }
    fn vector_compare(
        &mut self,
        op: &str,
        left: usize,
        right: usize,
        inner: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if !matches!(
            self.compiler.types[inner].kind,
            k::TypeKind::Int { .. } | k::TypeKind::Bool
        ) {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "vector ordering currently requires integer or bool elements",
            ));
        }
        let lp = self.addressable(left)?;
        let l = self.borrow(lp, false, span.line)?.0;
        let rp = self.addressable(right)?;
        let r = self.borrow(rp, false, span.line)?.0;
        let index = self
            .compiler
            .source
            .functions
            .iter()
            .position(|f| f.name == "__lkj_vec_compare")
            .ok_or_else(|| {
                err(
                    span,
                    "E_UNSUPPORTED",
                    "source sequence library is not loaded",
                )
            })?;
        let mut sub = BTreeMap::new();
        sub.insert("T".to_string(), inner);
        let function = self.compiler.instantiate(index, sub)?;
        let comparison = self.temp(k::I32);
        self.emit(
            k::Op::Call {
                dst: comparison,
                function,
                args: vec![l, r],
            },
            span.line,
        );
        let zero = self.temp(k::I32);
        self.emit(
            k::Op::Int {
                dst: zero,
                value: 0,
            },
            span.line,
        );
        let dst = self.temp(k::BOOL);
        self.emit(
            k::Op::Binary {
                dst,
                op: op.to_string(),
                left: comparison,
                right: zero,
            },
            span.line,
        );
        Ok((dst, k::BOOL))
    }
    fn short_circuit(
        &mut self,
        op: &str,
        left: usize,
        right: usize,
        line: usize,
    ) -> Result<(usize, usize)> {
        let (condition, _) = self.expr(left, Some(k::BOOL))?;
        let result = self.local(k::BOOL, "$if", true);
        self.emit(k::Op::If { condition }, line);
        let state = self.snapshot();
        let first = if op == "&&" {
            self.expr(right, Some(k::BOOL))?
        } else {
            let d = self.temp(k::BOOL);
            self.emit(k::Op::Int { dst: d, value: 1 }, line);
            (d, k::BOOL)
        };
        self.emit(
            k::Op::Assign {
                place: k::Place::local(result),
                src: first.0,
            },
            line,
        );
        self.cleanup(state.live.len(), Some(result), line);
        self.emit(k::Op::Else, line);
        self.restore(&state);
        let second = if op == "||" {
            self.expr(right, Some(k::BOOL))?
        } else {
            let d = self.temp(k::BOOL);
            self.emit(k::Op::Int { dst: d, value: 0 }, line);
            (d, k::BOOL)
        };
        self.emit(
            k::Op::Assign {
                place: k::Place::local(result),
                src: second.0,
            },
            line,
        );
        self.cleanup(state.live.len(), Some(result), line);
        self.emit(k::Op::End, line);
        Ok((result, k::BOOL))
    }
    fn if_expr(
        &mut self,
        condition: usize,
        then_block: usize,
        otherwise: Option<usize>,
        expected: Option<usize>,
        line: usize,
    ) -> Result<(usize, usize)> {
        let (condition, _) = self.expr(condition, Some(k::BOOL))?;
        self.emit(k::Op::If { condition }, line);
        let state = self.snapshot();
        let before = self.live.len();
        let bindings = self.bindings.len();
        let first = self.block(then_block, expected)?;
        let first_reachable = self.reachable;
        let result = self.local(
            expected.unwrap_or(if first_reachable { first.1 } else { k::UNIT }),
            "$if",
            true,
        );
        if first_reachable {
            self.emit(
                k::Op::Assign {
                    place: k::Place::local(result),
                    src: first.0,
                },
                line,
            );
            self.cleanup(before, Some(result), line);
        }
        let first_state = self.snapshot();
        self.emit(k::Op::Else, line);
        self.restore(&state);
        self.bindings.truncate(bindings);
        let second = if let Some(other) = otherwise {
            self.expr(
                other,
                if first_reachable {
                    Some(self.ty(result))
                } else {
                    expected
                },
            )?
        } else {
            self.unit(line)
        };
        let second_reachable = self.reachable;
        if second_reachable && !first_reachable && expected.is_none() {
            self.function.locals[result].ty = second.1;
        }
        if second_reachable {
            self.emit(
                k::Op::Assign {
                    place: k::Place::local(result),
                    src: second.0,
                },
                line,
            );
            self.cleanup(before, Some(result), line);
        }
        self.emit(k::Op::End, line);
        if !second_reachable && first_reachable {
            self.restore(&first_state);
        }
        self.reachable = first_reachable || second_reachable;
        Ok((result, self.ty(result)))
    }
    fn advance_range(&mut self, line: usize) -> Result<()> {
        let (start, range) = match self.loops.last() {
            Some(scope) => (scope.start, scope.range),
            None => return Ok(()),
        };
        if let Some((counter, finish, inclusive)) = range {
            if inclusive {
                let before = self.snapshot();
                let left = self.read(k::Place::local(counter), line)?.0;
                let right = self.read(k::Place::local(finish), line)?.0;
                let done = self.temp(k::BOOL);
                self.emit(
                    k::Op::Binary {
                        dst: done,
                        op: "==".to_string(),
                        left,
                        right,
                    },
                    line,
                );
                self.emit(k::Op::If { condition: done }, line);
                self.cleanup(start, None, line);
                self.emit(k::Op::Break, line);
                self.emit(k::Op::End, line);
                self.restore(&before);
            }
            let t = self.ty(counter);
            let left = self.read(k::Place::local(counter), line)?.0;
            let right = self.temp(t);
            self.emit(
                k::Op::Int {
                    dst: right,
                    value: 1,
                },
                line,
            );
            let dst = self.temp(t);
            self.emit(
                k::Op::Binary {
                    dst,
                    op: "+".to_string(),
                    left,
                    right,
                },
                line,
            );
            self.emit(
                k::Op::Assign {
                    place: k::Place::local(counter),
                    src: dst,
                },
                line,
            );
        }
        Ok(())
    }
    fn while_expr(
        &mut self,
        condition: Option<usize>,
        body: usize,
        line: usize,
    ) -> Result<(usize, usize)> {
        let start = self.live.len();
        let entry = self.snapshot();
        self.loops.push(LoopScope {
            start,
            range: None,
            exits: Vec::new(),
        });
        self.emit(k::Op::Loop, line);
        if let Some(condition) = condition {
            let (c, _) = self.expr(condition, Some(k::BOOL))?;
            let not = self.temp(k::BOOL);
            self.emit(
                k::Op::Unary {
                    dst: not,
                    op: "!".to_string(),
                    src: c,
                },
                line,
            );
            self.emit(k::Op::If { condition: not }, line);
            self.cleanup(start, None, line);
            self.emit(k::Op::Break, line);
            self.emit(k::Op::End, line);
            self.restore(&entry);
        }
        let _ = self.block(body, None)?;
        if self.reachable {
            self.cleanup(start, None, line);
            self.emit(k::Op::Continue, line);
        }
        self.emit(k::Op::End, line);
        let scope = self.loops.pop().unwrap();
        if let Some(exit) = scope.exits.first() {
            self.restore(exit);
        } else {
            self.restore(&entry);
            self.reachable = false;
        }
        Ok(self.unit(line))
    }
    fn for_expr(
        &mut self,
        pattern: usize,
        iterator: usize,
        body: usize,
        line: usize,
    ) -> Result<(usize, usize)> {
        let range = self.compiler.source.exprs[iterator].clone();
        let (start, end, inclusive) = match range.kind {
            a::Expr::Range {
                start: Some(s),
                end: Some(e),
                inclusive,
            } => (s, e, inclusive),
            _ => {
                return Err(err(
                    range.span,
                    "E_UNSUPPORTED",
                    "for currently requires an integer range",
                ));
            }
        };
        let p = self.compiler.source.patterns[pattern].clone();
        let (name, mutable) = match p.kind {
            a::Pattern::Bind { name, mutable, .. } => (name, mutable),
            _ => return Err(err(p.span, "E_UNSUPPORTED", "for requires a name binding")),
        };
        let expected = self.hint(end).or_else(|| self.hint(start));
        let (s, t) = self.expr(start, expected)?;
        let (e, _) = self.expr(end, Some(t))?;
        let counter = self.local(t, "$counter", true);
        self.emit(
            k::Op::Assign {
                place: k::Place::local(counter),
                src: s,
            },
            line,
        );
        let finish = self.local(t, "$end", false);
        self.emit(
            k::Op::Assign {
                place: k::Place::local(finish),
                src: e,
            },
            line,
        );
        let bindings = self.bindings.len();
        let first = self.live.len();
        let binding = self.local(t, &name, mutable);
        self.bindings.push((name, binding));
        let state = self.snapshot();
        self.loops.push(LoopScope {
            start: first,
            range: Some((counter, finish, inclusive)),
            exits: Vec::new(),
        });
        self.emit(k::Op::Loop, line);
        let left = self.read(k::Place::local(counter), line)?.0;
        let right = self.read(k::Place::local(finish), line)?.0;
        let done = self.temp(k::BOOL);
        self.emit(
            k::Op::Binary {
                dst: done,
                op: if inclusive { ">" } else { ">=" }.to_string(),
                left,
                right,
            },
            line,
        );
        self.emit(k::Op::If { condition: done }, line);
        self.cleanup(first, None, line);
        self.emit(k::Op::Break, line);
        self.emit(k::Op::End, line);
        self.restore(&state);
        let current = self.read(k::Place::local(counter), line)?.0;
        self.emit(
            k::Op::Assign {
                place: k::Place::local(binding),
                src: current,
            },
            line,
        );
        let _ = self.block(body, None)?;
        if self.reachable {
            self.advance_range(line)?;
            self.cleanup(first, None, line);
            self.emit(k::Op::Continue, line);
        }
        self.emit(k::Op::End, line);
        let scope = self.loops.pop().unwrap();
        if let Some(exit) = scope.exits.first() {
            self.restore(exit);
        } else {
            self.restore(&state);
            self.reachable = false;
        }
        self.bindings.truncate(bindings);
        self.emit(
            k::Op::Drop {
                place: k::Place::local(counter),
            },
            line,
        );
        self.emit(
            k::Op::Drop {
                place: k::Place::local(finish),
            },
            line,
        );
        Ok(self.unit(line))
    }
    fn variant(
        &mut self,
        path: &a::Path,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let (enum_name, variant) = path
            .name
            .rsplit_once("::")
            .unwrap_or(("", path.name.as_str()));
        let ty = if let Some(t) = expected {
            if matches!(self.compiler.types[t].kind, k::TypeKind::Enum { .. }) {
                t
            } else {
                return Err(err(span, "E_TYPE", "variant requires an enum result type"));
            }
        } else if !enum_name.is_empty() {
            self.compiler.named_type(
                &a::Path {
                    name: enum_name.to_string(),
                    args: path.args.clone(),
                },
                &self.sub,
                span,
            )?
        } else {
            return Err(err(span, "E_TYPE", "variant requires a result type"));
        };
        if let k::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind {
            let tag = variants
                .iter()
                .position(|v| v.name == variant)
                .ok_or_else(|| err(span, "E_TYPE", "unknown variant"))?;
            Ok((ty, tag))
        } else {
            Err(err(span, "E_TYPE", "variant requires enum"))
        }
    }
    fn infer_type(
        &mut self,
        pattern: usize,
        actual: usize,
        sub: &mut BTreeMap<String, usize>,
    ) -> Result<()> {
        match self.compiler.source.types[pattern].kind.clone() {
            a::Type::Path(p) => {
                if p.args.is_empty()
                    && !self.compiler.type_names.contains_key(&p.name)
                    && !self
                        .compiler
                        .source
                        .structs
                        .iter()
                        .any(|s| s.name == p.name)
                    && !self.compiler.source.enums.iter().any(|e| e.name == p.name)
                {
                    if let Some(existing) = sub.get(&p.name) {
                        if *existing != actual {
                            return Err(err(
                                self.compiler.source.types[pattern].span,
                                "E_TYPE",
                                "generic arguments disagree",
                            ));
                        }
                    } else {
                        sub.insert(p.name, actual);
                    }
                } else if let Some((name, arguments)) =
                    self.compiler.applications.get(&actual).cloned()
                {
                    let ps=p.args.iter().filter(|id|!matches!(&self.compiler.source.types[**id].kind,a::Type::Path(p) if p.name.starts_with('\''))).copied().collect::<Vec<_>>();
                    if short(&p.name) == short(&name) && ps.len() == arguments.len() {
                        for i in 0..ps.len() {
                            self.infer_type(ps[i], arguments[i], sub)?;
                        }
                    }
                } else if p.args.len() == 1 {
                    let inner = match self.compiler.types[actual].kind {
                        k::TypeKind::Vec { inner } | k::TypeKind::Box { inner } => Some(inner),
                        _ => None,
                    };
                    if let Some(inner) = inner {
                        self.infer_type(p.args[0], inner, sub)?;
                    }
                }
            }
            a::Type::Ref { inner, .. } => {
                if let k::TypeKind::Ref { inner: t, .. } = self.compiler.types[actual].kind {
                    self.infer_type(inner, t, sub)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn call(
        &mut self,
        callee: usize,
        args: &[usize],
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let path = match self.compiler.source.exprs[callee].kind.clone() {
            a::Expr::Path(p) => p,
            _ => {
                return Err(err(
                    span,
                    "E_UNSUPPORTED",
                    "indirect calls are not implemented yet",
                ));
            }
        };
        let line = span.line;
        if short(&path.name) == "scope" {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "structured task scopes are not implemented yet",
            ));
        }
        if path.name == "std::mem::replace" || path.name == "mem::replace" {
            if args.len() != 2 {
                return Err(err(span, "E_TYPE", "replace requires a place and a value"));
            }
            let receiver = self.expr(args[0], None)?;
            let inner = match self.compiler.types[receiver.1].kind {
                k::TypeKind::Ref {
                    inner,
                    mutable: true,
                } => inner,
                _ => return Err(err(span, "E_TYPE", "replace requires &mut T")),
            };
            let value = self.expr(args[1], Some(inner))?;
            let dst = self.temp(inner);
            self.emit(
                k::Op::Intrinsic {
                    dst,
                    name: "replace".to_string(),
                    args: vec![receiver.0, value.0],
                },
                line,
            );
            return Ok((dst, inner));
        }
        if path.name == "drop" || path.name == "std::mem::drop" {
            if args.len() != 1 {
                return Err(err(span, "E_TYPE", "drop requires one argument"));
            }
            let (src, _) = self.expr(args[0], None)?;
            self.emit(
                k::Op::Drop {
                    place: k::Place::local(src),
                },
                line,
            );
            return Ok(self.unit(line));
        }
        if path.name.ends_with("::new")
            && (path.name.starts_with("Vec")
                || path.name.starts_with("Box")
                || path.name.contains("::Vec")
                || path.name.contains("::Box"))
        {
            let boxed = path.name.contains("Box");
            let ty = if let Some(t) = expected {
                t
            } else if !path.args.is_empty() {
                self.compiler.named_type(
                    &a::Path {
                        name: if boxed { "Box" } else { "Vec" }.to_string(),
                        args: path.args.clone(),
                    },
                    &self.sub,
                    span,
                )?
            } else if boxed && args.len() == 1 {
                let value = self.expr(args[0], None)?;
                let t = self.compiler.intern(
                    format!("Box<{}>", self.compiler.types[value.1].name),
                    k::TypeKind::Box { inner: value.1 },
                );
                let d = self.temp(t);
                self.emit(
                    k::Op::Intrinsic {
                        dst: d,
                        name: "box_new".to_string(),
                        args: vec![value.0],
                    },
                    line,
                );
                return Ok((d, t));
            } else {
                return Err(err(
                    span,
                    "E_TYPE",
                    "collection element type needs an annotation",
                ));
            };
            let mut values = Vec::new();
            if boxed {
                let inner = match self.compiler.types[ty].kind {
                    k::TypeKind::Box { inner } => inner,
                    _ => return Err(err(span, "E_TYPE", "Box constructor result type differs")),
                };
                if args.len() != 1 {
                    return Err(err(span, "E_TYPE", "Box::new requires one argument"));
                }
                values.push(self.expr(args[0], Some(inner))?.0);
            } else if !args.is_empty() {
                return Err(err(span, "E_TYPE", "Vec::new takes no arguments"));
            }
            let dst = self.temp(ty);
            self.emit(
                k::Op::Intrinsic {
                    dst,
                    name: if boxed { "box_new" } else { "vec_new" }.to_string(),
                    args: values,
                },
                line,
            );
            return Ok((dst, ty));
        }
        if let Ok((ty, tag)) = self.variant(&path, expected, span) {
            let fields = match &self.compiler.types[ty].kind {
                k::TypeKind::Enum { variants, .. } => variants[tag].fields.clone(),
                _ => Vec::new(),
            };
            if fields.len() != args.len() {
                return Err(err(span, "E_TYPE", "variant field count differs"));
            }
            let mut values = Vec::new();
            for i in 0..args.len() {
                values.push(self.expr(args[i], Some(fields[i].ty))?.0);
            }
            let dst = self.temp(ty);
            self.emit(
                k::Op::Variant {
                    dst,
                    variant: tag,
                    fields: values,
                },
                line,
            );
            return Ok((dst, ty));
        }
        if let Some(s) = self
            .compiler
            .source
            .structs
            .iter()
            .find(|s| s.name == path.name && s.tuple)
            .cloned()
        {
            let ty = self.compiler.named_type(&path, &self.sub, span)?;
            let fields = match &self.compiler.types[ty].kind {
                k::TypeKind::Struct { fields, .. } => fields.clone(),
                _ => Vec::new(),
            };
            if fields.len() != args.len() {
                return Err(err(span, "E_TYPE", "tuple struct field count differs"));
            }
            let mut values = Vec::new();
            for i in 0..args.len() {
                values.push(self.expr(args[i], Some(fields[i].ty))?.0);
            }
            let dst = self.temp(ty);
            self.emit(
                k::Op::Struct {
                    dst,
                    fields: values,
                },
                s.span.line,
            );
            return Ok((dst, ty));
        }
        let mut initial_sub = BTreeMap::new();
        let mut associated = None;
        if let Some((owner, method)) = path.name.rsplit_once("::") {
            let target_type = expected.or_else(|| {
                self.compiler
                    .named_type(
                        &a::Path {
                            name: owner.to_string(),
                            args: path.args.clone(),
                        },
                        &self.sub,
                        span,
                    )
                    .ok()
            });
            if let Some(t) = target_type {
                let impls = self.compiler.source.impls.clone();
                for im in impls {
                    let mut sub = BTreeMap::new();
                    self.infer_type(im.target, t, &mut sub)?;
                    sub.insert("Self".to_string(), t);
                    if self.compiler.resolve(im.target, &sub).ok() == Some(t) {
                        self.compiler.bounds(&im.generics, &sub)?;
                        for candidate in im.methods {
                            let f = &self.compiler.source.functions[candidate];
                            if f.name == method && f.params.first().is_none_or(|p| p.name != "self")
                            {
                                if associated.is_some() {
                                    return Err(err(
                                        span,
                                        "E_TRAIT",
                                        "ambiguous associated function",
                                    ));
                                }
                                associated = Some(candidate);
                                initial_sub = sub.clone();
                            }
                        }
                    }
                }
            }
        }
        let index = if let Some(index) = associated {
            index
        } else {
            self.compiler
                .source
                .functions
                .iter()
                .position(|f| qualified(&f.module, &f.name) == path.name || f.name == path.name)
                .ok_or_else(|| err(span, "E_TYPE", &format!("unknown function {}", path.name)))?
        };
        let target = self.compiler.source.functions[index].clone();
        if target.params.len() != args.len() {
            return Err(err(span, "E_TYPE", "function argument count differs"));
        }
        let mut sub = initial_sub;
        let generic: Vec<_> = target.generics.iter().filter(|g| !g.lifetime).collect();
        if !path.args.is_empty() && associated.is_none() {
            if path.args.len() != generic.len() {
                return Err(err(span, "E_TYPE", "wrong generic argument count"));
            }
            for i in 0..generic.len() {
                sub.insert(
                    generic[i].name.clone(),
                    self.compiler.resolve(path.args[i], &self.sub)?,
                );
            }
        }
        let mut values = Vec::new();
        for i in 0..args.len() {
            let parameter = self.compiler.resolve(target.params[i].ty, &sub).ok();
            let value = self.expr(args[i], parameter)?;
            self.infer_type(target.params[i].ty, value.1, &mut sub)?;
            values.push(value.0);
        }
        let function = self.compiler.instantiate(index, sub)?;
        let ty = self.compiler.functions[function].result;
        let dst = self.temp(ty);
        self.emit(
            k::Op::Call {
                dst,
                function,
                args: values,
            },
            line,
        );
        Ok((dst, ty))
    }
    fn method(
        &mut self,
        receiver: usize,
        name: &str,
        _types: &[usize],
        args: &[usize],
        _expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let place = self.addressable(receiver)?;
        let mut base = place_type(&self.compiler.types, self.function, &place)
            .ok_or_else(|| err(span, "E_TYPE", "invalid method receiver"))?;
        let mut target = place.clone();
        while let k::TypeKind::Ref { inner, .. } = self.compiler.types[base].kind {
            target.projections.push(k::Projection::Deref);
            base = inner;
        }
        match name {
            "as_slice" | "as_mut_slice"
                if matches!(
                    self.compiler.types[base].kind,
                    k::TypeKind::Vec { .. } | k::TypeKind::Array { .. }
                ) =>
            {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "slice projection takes no arguments"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } | k::TypeKind::Array { inner, .. } => inner,
                    _ => return Err(err(span, "E_TYPE", "slice projection needs a collection")),
                };
                let slice = self.compiler.intern(
                    format!("[{}]", self.compiler.types[inner].name),
                    k::TypeKind::Slice {
                        inner,
                        mutable: false,
                    },
                );
                let mutable = name == "as_mut_slice";
                let ty = self.compiler.reference(slice, mutable);
                let dst = self.temp(ty);
                self.emit(
                    k::Op::SliceBorrow {
                        dst,
                        place: target,
                        mutable,
                    },
                    span.line,
                );
                Ok((dst, ty))
            }
            "len"
                if matches!(
                    self.compiler.types[base].kind,
                    k::TypeKind::Vec { .. }
                        | k::TypeKind::Slice { .. }
                        | k::TypeKind::Array { .. }
                        | k::TypeKind::String
                ) =>
            {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "len takes no arguments"));
                }
                let r = self.borrow(target, false, span.line)?.0;
                let dst = self.temp(k::U64);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_len".to_string(),
                        args: vec![r],
                    },
                    span.line,
                );
                Ok((dst, k::U64))
            }
            "push" if matches!(self.compiler.types[base].kind, k::TypeKind::Vec { .. }) => {
                if args.len() != 1 {
                    return Err(err(span, "E_TYPE", "push takes one argument"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } => inner,
                    _ => return Err(err(span, "E_TYPE", "push requires Vec")),
                };
                let value = self.expr(args[0], Some(inner))?;
                let r = self.borrow(target, true, span.line)?.0;
                let dst = self.temp(k::UNIT);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_push".to_string(),
                        args: vec![r, value.0],
                    },
                    span.line,
                );
                Ok((dst, k::UNIT))
            }
            "pop" if matches!(self.compiler.types[base].kind, k::TypeKind::Vec { .. }) => {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "pop takes no arguments"));
                }
                let inner = match self.compiler.types[base].kind {
                    k::TypeKind::Vec { inner } => inner,
                    _ => return Err(err(span, "E_TYPE", "pop requires Vec")),
                };
                let mut sub = BTreeMap::new();
                sub.insert("T".to_string(), inner);
                let ty = self.compiler.named_type(
                    &a::Path {
                        name: "Option".to_string(),
                        args: Vec::new(),
                    },
                    &sub,
                    span,
                )?;
                let receiver = self.borrow(target, true, span.line)?.0;
                let dst = self.temp(ty);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_pop".to_string(),
                        args: vec![receiver],
                    },
                    span.line,
                );
                Ok((dst, ty))
            }
            "clone" => {
                if !args.is_empty() {
                    return Err(err(span, "E_TYPE", "clone takes no arguments"));
                }
                if !is_copy(&self.compiler.types, base) {
                    return Err(err(
                        span,
                        "E_UNSUPPORTED",
                        "Clone for affine values is not implemented yet",
                    ));
                }
                let r = self.borrow(target, false, span.line)?.0;
                let dst = self.temp(base);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "clone".to_string(),
                        args: vec![r],
                    },
                    span.line,
                );
                Ok((dst, base))
            }
            _ => {
                let mut candidates = Vec::new();
                let impls = self.compiler.source.impls.clone();
                for im in impls {
                    let mut sub = BTreeMap::new();
                    self.infer_type(im.target, base, &mut sub)?;
                    sub.insert("Self".to_string(), base);
                    if let Ok(t) = self.compiler.resolve(im.target, &sub) {
                        if t == base {
                            self.compiler.bounds(&im.generics, &sub)?;
                            for method in &im.methods {
                                if self.compiler.source.functions[*method].name == name {
                                    candidates.push((*method, sub.clone()));
                                }
                            }
                        }
                    }
                }
                if candidates.len() != 1 {
                    return Err(err(
                        span,
                        if candidates.is_empty() {
                            "E_UNSUPPORTED"
                        } else {
                            "E_TRAIT"
                        },
                        "method resolution requires one implementation",
                    ));
                }
                let (index, mut sub) = candidates[0].clone();
                let f = self.compiler.source.functions[index].clone();
                sub.insert("Self".to_string(), base);
                let mut values = Vec::new();
                if f.params.is_empty() {
                    return Err(err(span, "E_TYPE", "method has no receiver"));
                }
                let first = self.compiler.resolve(f.params[0].ty, &sub)?;
                let value = match self.compiler.types[first].kind {
                    k::TypeKind::Ref { mutable, .. } => self.borrow(target, mutable, span.line)?,
                    _ => self.read(target, span.line)?,
                };
                values.push(value.0);
                if f.params.len() != args.len() + 1 {
                    return Err(err(span, "E_TYPE", "method argument count differs"));
                }
                for i in 0..args.len() {
                    let t = self.compiler.resolve(f.params[i + 1].ty, &sub)?;
                    values.push(self.expr(args[i], Some(t))?.0);
                }
                let function = self.compiler.instantiate(index, sub)?;
                let t = self.compiler.functions[function].result;
                let dst = self.temp(t);
                self.emit(
                    k::Op::Call {
                        dst,
                        function,
                        args: values,
                    },
                    span.line,
                );
                Ok((dst, t))
            }
        }
    }
    fn construct_struct(
        &mut self,
        path: &a::Path,
        fields: &[(String, usize)],
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let definition = self
            .compiler
            .source
            .structs
            .iter()
            .find(|s| s.name == path.name || qualified(&s.module, &s.name) == path.name)
            .cloned()
            .ok_or_else(|| err(span, "E_TYPE", "unknown struct"))?;
        let mut sub = self.sub.clone();
        let mut values = Vec::new();
        for (name, expr) in fields {
            let field = definition
                .fields
                .iter()
                .find(|f| f.name == *name)
                .ok_or_else(|| err(span, "E_TYPE", "unknown field"))?;
            let hint = self.compiler.resolve(field.ty, &sub).ok();
            let value = self.expr(*expr, hint)?;
            self.infer_type(field.ty, value.1, &mut sub)?;
            values.push((name.clone(), value.0));
        }
        let ty = match expected {
            Some(t) if matches!(self.compiler.types[t].kind, k::TypeKind::Struct { .. }) => t,
            _ => self.compiler.named_type(path, &sub, span)?,
        };
        let expected_fields = match &self.compiler.types[ty].kind {
            k::TypeKind::Struct { fields, .. } => fields.clone(),
            _ => return Err(err(span, "E_TYPE", "constructor requires a struct")),
        };
        if fields.len() != expected_fields.len() {
            return Err(err(span, "E_TYPE", "constructor field count differs"));
        }
        let mut ordered = Vec::new();
        for field in expected_fields {
            let hits = values
                .iter()
                .filter(|(name, _)| *name == field.name)
                .collect::<Vec<_>>();
            if hits.len() != 1 {
                return Err(err(span, "E_TYPE", "missing or duplicate field"));
            }
            ordered.push(hits[0].1);
        }
        let dst = self.temp(ty);
        self.emit(
            k::Op::Struct {
                dst,
                fields: ordered,
            },
            span.line,
        );
        Ok((dst, ty))
    }
    fn macro_expr(
        &mut self,
        name: &str,
        args: &[usize],
        separator: &str,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        match name {
            "assert" => {
                if args.is_empty() {
                    return Err(err(span, "E_TYPE", "assert needs a condition"));
                }
                let condition = self.expr(args[0], Some(k::BOOL))?.0;
                let dst = self.temp(k::UNIT);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "assert".to_string(),
                        args: vec![condition],
                    },
                    span.line,
                );
                Ok((dst, k::UNIT))
            }
            "assert_eq" | "assert_ne" => {
                if args.len() < 2 {
                    return Err(err(span, "E_TYPE", "assertion needs two operands"));
                }
                let t = self.hint(args[0]).or_else(|| self.hint(args[1]));
                let (left, ty) = self.expr(args[0], t)?;
                let (right, _) = self.expr(args[1], Some(ty))?;
                let condition = self.temp(k::BOOL);
                self.emit(
                    k::Op::Binary {
                        dst: condition,
                        op: if name == "assert_eq" { "==" } else { "!=" }.to_string(),
                        left,
                        right,
                    },
                    span.line,
                );
                let dst = self.temp(k::UNIT);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "assert".to_string(),
                        args: vec![condition],
                    },
                    span.line,
                );
                Ok((dst, k::UNIT))
            }
            "panic" | "unreachable" => {
                self.emit(
                    k::Op::Trap {
                        message: "explicit panic".to_string(),
                    },
                    span.line,
                );
                Ok(self.unit(span.line))
            }
            "vec" => {
                if separator == ";" {
                    return Err(err(
                        span,
                        "E_UNSUPPORTED",
                        "repeated Vec literals are not implemented yet",
                    ));
                }
                let inner = expected.and_then(|t| match self.compiler.types[t].kind {
                    k::TypeKind::Vec { inner } => Some(inner),
                    _ => None,
                });
                let mut values = Vec::new();
                let mut inferred = inner;
                for arg in args {
                    let value = self.expr(*arg, inferred)?;
                    inferred = Some(value.1);
                    values.push(value.0);
                }
                let inner =
                    inferred.ok_or_else(|| err(span, "E_TYPE", "empty Vec needs a type"))?;
                let ty = self.compiler.intern(
                    format!("Vec<{}>", self.compiler.types[inner].name),
                    k::TypeKind::Vec { inner },
                );
                let dst = self.local(ty, "$vec", true);
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: "vec_new".to_string(),
                        args: Vec::new(),
                    },
                    span.line,
                );
                for value in values {
                    let r = self.borrow(k::Place::local(dst), true, span.line)?.0;
                    let unit = self.temp(k::UNIT);
                    self.emit(
                        k::Op::Intrinsic {
                            dst: unit,
                            name: "vec_push".to_string(),
                            args: vec![r, value],
                        },
                        span.line,
                    );
                    self.emit(
                        k::Op::Drop {
                            place: k::Place::local(unit),
                        },
                        span.line,
                    );
                }
                Ok((dst, ty))
            }
            _ => Err(err(
                span,
                "E_UNSUPPORTED",
                &format!("macro {name} is not implemented"),
            )),
        }
    }
    fn match_expr(
        &mut self,
        value: usize,
        arms: &[a::Arm],
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let (scrutinee, original_ty) = self.expr(value, None)?;
        let (ty, borrowed, mutable) = match self.compiler.types[original_ty].kind {
            k::TypeKind::Ref { inner, mutable } => (inner, true, mutable),
            _ => (original_ty, false, false),
        };
        let start = self.live.len();
        let base = self.snapshot();
        let binding_count = self.bindings.len();
        let mut result: Option<usize> = None;
        let mut exit_state: Option<Snapshot> = None;
        let mut open = 0;
        let mut exhaustive = false;
        let mut default_seen = false;
        let mut tags = Vec::new();
        for arm in arms {
            if arm.guard.is_some() {
                return Err(err(
                    arm.span,
                    "E_UNSUPPORTED",
                    "match guards are not implemented yet",
                ));
            }
            let pattern = self.compiler.source.patterns[arm.pattern].clone();
            let mut fields = Vec::new();
            let condition = match pattern.kind {
                a::Pattern::Wild => {
                    exhaustive = true;
                    default_seen = true;
                    None
                }
                a::Pattern::Bind { .. } => {
                    exhaustive = true;
                    default_seen = true;
                    let v = self.read(k::Place::local(scrutinee), span.line)?;
                    self.bind(arm.pattern, v, span.line)?;
                    None
                }
                a::Pattern::Variant {
                    path,
                    fields: patterns,
                } => {
                    let (_, tag) = self.variant(&path, Some(ty), arm.span)?;
                    tags.push(tag);
                    for (i, p) in patterns.iter().enumerate() {
                        fields.push((*p, k::Projection::VariantField(tag, i)));
                    }
                    let c = self.temp(k::BOOL);
                    self.emit(
                        k::Op::TagTest {
                            dst: c,
                            src: scrutinee,
                            variant: tag,
                        },
                        span.line,
                    );
                    Some(c)
                }
                a::Pattern::Path(path) => {
                    let (_, tag) = self.variant(&path, Some(ty), arm.span)?;
                    tags.push(tag);
                    let c = self.temp(k::BOOL);
                    self.emit(
                        k::Op::TagTest {
                            dst: c,
                            src: scrutinee,
                            variant: tag,
                        },
                        span.line,
                    );
                    Some(c)
                }
                a::Pattern::Literal(expr) => {
                    let l = self.read(k::Place::local(scrutinee), span.line)?.0;
                    let r = self.expr(expr, Some(ty))?.0;
                    let c = self.temp(k::BOOL);
                    self.emit(
                        k::Op::Binary {
                            dst: c,
                            op: "==".to_string(),
                            left: l,
                            right: r,
                        },
                        span.line,
                    );
                    Some(c)
                }
                _ => {
                    return Err(err(
                        pattern.span,
                        "E_UNSUPPORTED",
                        "this match pattern is not implemented yet",
                    ));
                }
            };
            if let Some(c) = condition {
                self.emit(k::Op::If { condition: c }, span.line);
                open += 1;
            }
            for (pattern, projection) in fields {
                let mut projections = Vec::new();
                if borrowed {
                    projections.push(k::Projection::Deref);
                }
                projections.push(projection);
                let place = k::Place {
                    local: scrutinee,
                    projections,
                };
                if matches!(
                    self.compiler.source.patterns[pattern].kind,
                    a::Pattern::Wild
                ) {
                    continue;
                }
                let v = if borrowed {
                    self.borrow(place, mutable, span.line)?
                } else {
                    self.read(place, span.line)?
                };
                self.bind(pattern, v, span.line)?;
            }
            let v = self.expr(arm.value, expected.or(result.map(|i| self.ty(i))))?;
            if self.reachable {
                let target = if let Some(r) = result {
                    r
                } else {
                    let d = self.local(expected.unwrap_or(v.1), "$match", true);
                    result = Some(d);
                    d
                };
                self.emit(
                    k::Op::Assign {
                        place: k::Place::local(target),
                        src: v.0,
                    },
                    span.line,
                );
                self.cleanup(start, Some(target), span.line);
                if self.live[scrutinee] {
                    self.emit(
                        k::Op::Drop {
                            place: k::Place::local(scrutinee),
                        },
                        span.line,
                    );
                }
                if exit_state.is_none() {
                    exit_state = Some(self.snapshot());
                }
            }
            self.bindings.truncate(binding_count);
            if condition.is_some() {
                self.emit(k::Op::Else, span.line);
                self.restore(&base);
            } else {
                break;
            }
        }
        if let k::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind {
            tags.sort_unstable();
            tags.dedup();
            if tags.len() == variants.len() {
                exhaustive = true;
            }
        }
        if !exhaustive {
            return Err(err(span, "E_TYPE", "match is not exhaustive"));
        }
        if open > 0 {
            if !default_seen {
                self.emit(
                    k::Op::Trap {
                        message: "unreachable match".to_string(),
                    },
                    span.line,
                );
            }
            for _ in 0..open {
                self.emit(k::Op::End, span.line);
            }
        }
        if let Some(exit) = exit_state {
            self.restore(&exit);
            let result = result.ok_or_else(|| err(span, "E_TYPE", "match result is missing"))?;
            Ok((result, self.ty(result)))
        } else {
            self.reachable = false;
            Ok(self.unit(span.line))
        }
    }
}
