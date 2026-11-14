//! Availability is separate for source signatures and concrete layouts. A real
//! nominal identity remains usable in source inference when one field is absent.
use crate::ast as a;
use crate::resolve::recovery::{Report, Site};

pub(crate) struct UnavailableTypes {
    pub causes: Vec<Option<usize>>,
    pub problems: Vec<a::Diagnostic>,
}
impl UnavailableTypes {
    pub fn get(&self, id: a::TypeId) -> Option<&a::Diagnostic> {
        self.causes
            .get(id)
            .copied()
            .flatten()
            .map(|cause| &self.problems[cause])
    }
}
struct Graph {
    reverse: Vec<Vec<usize>>,
    causes: Vec<Option<usize>>,
}
impl Graph {
    fn new(size: usize) -> Self {
        Self {
            reverse: vec![Vec::new(); size],
            causes: vec![None; size],
        }
    }
    fn needs(&mut self, node: usize, dependency: usize) {
        self.reverse[dependency].push(node);
    }
    fn finish(mut self) -> Vec<Option<usize>> {
        let mut pending: Vec<usize> = Vec::new();
        for (id, cause) in self.causes.iter().enumerate() {
            if cause.is_some() {
                pending.push(id);
            }
        }
        let mut head = 0;
        while head < pending.len() {
            let dependency = pending[head];
            head += 1;
            for node in &self.reverse[dependency] {
                if self.causes[*node].is_none() {
                    self.causes[*node] = self.causes[dependency];
                    pending.push(*node);
                }
            }
        }
        self.causes
    }
}
fn type_children(kind: &a::Type) -> Vec<a::TypeId> {
    match kind {
        a::Type::Path(path) => path.args.clone(),
        a::Type::Ref { inner, .. } | a::Type::Slice(inner) | a::Type::Array { inner, .. } => {
            vec![*inner]
        }
        a::Type::Tuple(items) => items.clone(),
        a::Type::Function { params, result } => {
            let mut items = params.clone();
            items.push(*result);
            items
        }
        _ => Vec::new(),
    }
}
fn dependency(problem: &a::Diagnostic) -> a::Diagnostic {
    let mut result = problem.clone();
    result.message = format!(
        "unavailable dependency: {} {}",
        problem.code, problem.message
    );
    result.code = "E_BLOCKED".to_string();
    result
}
fn first(types: &UnavailableTypes, roots: &[usize]) -> Option<usize> {
    for id in roots {
        if let Some(cause) = types.causes[*id] {
            return Some(cause);
        }
    }
    None
}
pub(crate) struct Readiness {
    pub types: UnavailableTypes,
    pub layouts: Vec<Option<usize>>,
    pub explicit_lifetimes: Vec<bool>,
    pub signatures: Vec<Option<usize>>,
    pub implementation_headers: Vec<bool>,
    pub extra_problems: Vec<a::Diagnostic>,
}
impl Readiness {
    pub fn new(report: &Report) -> Self {
        let source = &report.program;
        let mut problems = Vec::new();
        let mut graph = Graph::new(source.types.len());
        let mut lifetimes = Graph::new(source.types.len());
        for failure in &report.failures {
            if let Site::Type(id) = failure.site {
                graph.causes[id] = Some(problems.len());
                problems.push(dependency(&failure.problem));
            }
        }
        let mut extra_problems = Vec::new();
        for (alias, problem) in crate::generics::alias_cycle_problems(source) {
            graph.causes[source.aliases[alias].ty] = Some(problems.len());
            problems.push(dependency(&problem));
            extra_problems.push(problem);
        }
        for (id, node) in source.types.iter().enumerate() {
            if matches!(&node.kind, a::Type::Ref { lifetime, .. } if !lifetime.is_empty() && lifetime != "'_" )
                || matches!(&node.kind, a::Type::Path(path) if path.name.starts_with("'"))
            {
                lifetimes.causes[id] = Some(0);
            }
            for child in type_children(&node.kind) {
                lifetimes.needs(id, child);
                graph.needs(id, child);
            }
            if let a::Type::Path(path) = &node.kind {
                if let Some(declaration) = path.declaration {
                    if let a::Item::Alias(alias) = source.declarations[declaration].item {
                        graph.needs(id, source.aliases[alias].ty);
                        lifetimes.needs(id, source.aliases[alias].ty);
                    }
                }
            }
        }
        let mut explicit_lifetimes = Vec::new();
        for cause in lifetimes.finish() {
            explicit_lifetimes.push(cause.is_some());
        }
        let types = UnavailableTypes {
            causes: graph.finish(),
            problems,
        };
        let count = source.types.len();
        let mut layout = Graph::new(count + source.declarations.len());
        let mut cause_index = 0;
        while cause_index < count {
            layout.causes[cause_index] = types.causes[cause_index];
            cause_index += 1;
        }
        for (id, node) in source.types.iter().enumerate() {
            for child in type_children(&node.kind) {
                layout.needs(id, child);
            }
            if let a::Type::Path(path) = &node.kind {
                if let Some(declaration) = path.declaration {
                    layout.needs(id, count + declaration);
                }
            }
        }
        for (declaration, item) in source.declarations.iter().enumerate() {
            match item.item {
                a::Item::Struct(id) => {
                    for field in &source.structs[id].fields {
                        layout.needs(count + declaration, field.ty);
                    }
                }
                a::Item::Enum(id) => {
                    for variant in &source.enums[id].variants {
                        for field in &variant.fields {
                            layout.needs(count + declaration, field.ty);
                        }
                    }
                }
                a::Item::Alias(id) => layout.needs(count + declaration, source.aliases[id].ty),
                _ => {}
            }
        }
        let layouts = layout.finish();
        let mut signatures = Vec::new();
        for (index, function) in source.functions.iter().enumerate() {
            let mut roots = vec![function.result];
            for parameter in &function.params {
                roots.push(parameter.ty);
            }
            for parameter in &function.generics {
                for bound in &parameter.bounds {
                    roots.push(*bound);
                }
            }
            if let a::FunctionOwner::Implementation(owner) =
                source.function_owner(a::FunctionId { index })
            {
                roots.push(source.impls[owner.index].target);
                if let Some(trait_ty) = source.impls[owner.index].trait_ty {
                    roots.push(trait_ty);
                }
                for parameter in &source.impls[owner.index].generics {
                    for bound in &parameter.bounds {
                        roots.push(*bound);
                    }
                }
            }
            signatures.push(first(&types, &roots));
        }
        let mut implementation_headers = Vec::new();
        for implementation in &source.impls {
            implementation_headers.push(types.get(implementation.target).is_none());
        }
        Self {
            types,
            layouts,
            explicit_lifetimes,
            signatures,
            implementation_headers,
            extra_problems,
        }
    }
    pub fn signature_problem(&self, function: usize) -> Option<&a::Diagnostic> {
        self.signatures[function].map(|cause| &self.types.problems[cause])
    }
    pub fn layout_problem(&self, source: &a::Program, function: usize) -> Option<&a::Diagnostic> {
        let definition = &source.functions[function];
        let mut roots = vec![definition.result];
        for parameter in &definition.params {
            roots.push(parameter.ty);
        }
        if let a::FunctionOwner::Implementation(owner) =
            source.function_owner(a::FunctionId { index: function })
        {
            roots.push(source.impls[owner.index].target);
        }
        for id in roots {
            if let Some(cause) = self.layouts[id] {
                return Some(&self.types.problems[cause]);
            }
        }
        None
    }
    pub fn body_problem(
        &self,
        source: &a::Program,
        function: usize,
        blocked: &[super::BlockedRegion],
    ) -> Option<a::Diagnostic> {
        let body = source.functions[function].body?;
        let span = source.blocks[body].span;
        blocked
            .iter()
            .find(|region| {
                matches!(region.kind.as_str(), "statement" | "item")
                    && region.span.file == span.file
                    && region.span.start >= span.start
                    && region.span.end <= span.end
            })
            .map(|region| a::Diagnostic::new(region.span, "E_BLOCKED", &region.reason))
    }
}
