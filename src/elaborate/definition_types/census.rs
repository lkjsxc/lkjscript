//! Diagnostic access to genuine source inference facts. Nominal heads retain
//! declaration identity without asserting that their complete layout is known.
use super::*;

#[derive(Clone)]
pub(crate) enum Receiver {
    Atom(String),
    Nominal(a::DeclarationId),
    Vec,
    Box,
    Slice,
    Array,
    Tuple,
    Rigid(String),
}
impl Receiver {
    pub(crate) fn head(&self, source: &a::Program) -> Option<String> {
        match self {
            Self::Atom(name) => Some(name.clone()),
            Self::Nominal(id) => Some(source.declarations[*id].name.clone()),
            Self::Vec => Some("Vec".to_string()),
            Self::Box => Some("Box".to_string()),
            Self::Slice | Self::Array | Self::Tuple | Self::Rigid(_) => None,
        }
    }
    pub(crate) fn display(&self, source: &a::Program) -> String {
        match self {
            Self::Slice => "slice".to_string(),
            Self::Array => "array".to_string(),
            Self::Tuple => "tuple".to_string(),
            Self::Rigid(name) => format!("parameter {name}"),
            _ => self.head(source).unwrap(),
        }
    }
}
impl DefinitionTypes {
    pub(crate) fn unresolved_paths(&self) -> &[(a::ExprId, a::Diagnostic)] {
        &self.unresolved_paths
    }
    pub(crate) fn expressions(&self) -> Vec<a::ExprId> {
        let mut expressions = Vec::new();
        for expression in self.expressions.keys() {
            expressions.push(*expression);
        }
        expressions
    }
    pub(crate) fn receiver(&self, expression: a::ExprId) -> Option<Receiver> {
        let mut variable = *self.expressions.get(&expression)?;
        loop {
            return Some(match self.shape(variable)? {
                Shape::Ref(_, inner) => {
                    variable = inner;
                    continue;
                }
                Shape::Atom(name) => Receiver::Atom(name),
                Shape::Nominal(declaration, _) => Receiver::Nominal(declaration),
                Shape::Vec(_) => Receiver::Vec,
                Shape::Box(_) => Receiver::Box,
                Shape::Slice(_) => Receiver::Slice,
                Shape::Array(_, _) => Receiver::Array,
                Shape::Tuple(_) => Receiver::Tuple,
                Shape::Rigid(name) => Receiver::Rigid(name),
                Shape::Projected(_, _) | Shape::Source(_) | Shape::Closure(_, _) | Shape::Never => {
                    return None;
                }
            });
        }
    }
}
impl Receiver {
    pub(crate) fn rigid(&self) -> bool {
        matches!(self, Self::Rigid(_))
    }
    pub(crate) fn primitive(&self) -> super::super::methods::Receiver {
        use super::super::methods::Receiver as Kind;
        match self {
            Self::Atom(name) if integer_name(name) => Kind::Integer,
            Self::Atom(name) if name == "char" => Kind::Character,
            Self::Atom(name) if name == "str" => Kind::Text,
            Self::Atom(name) if name == "World" => Kind::World,
            Self::Atom(name) if name == "Stdout" => Kind::Stdout,
            Self::Atom(name) if name == "Stdin" => Kind::Stdin,
            Self::Atom(name) if name == "Stderr" => Kind::Stderr,
            Self::Atom(name) if name == "Directory" => Kind::Directory,
            Self::Atom(name) if name == "File" => Kind::File,
            Self::Vec => Kind::Vector,
            Self::Array => Kind::Array,
            Self::Slice => Kind::Slice,
            _ => Kind::Other,
        }
    }
}
impl Infer<'_, '_> {
    pub(super) fn census_path(&self, expression: a::ExprId, path: &a::Path) -> Result<()> {
        if let Some(context) = self.census {
            let node = &self.source.exprs[expression];
            if let Some(problem) = context.path_problem(
                self.names,
                self.source,
                node.scope,
                &path.name,
                crate::resolve::Namespace::Value,
                node.span,
            ) {
                return Err(problem);
            }
        }
        Ok(())
    }
    pub(super) fn census_trait_visibility_complete(
        &self,
        scope: a::ModuleId,
        span: a::Span,
    ) -> bool {
        self.census.is_none_or(|context| {
            self.names
                .uncertain_trait_import(&context.imports, scope, span)
                .is_none()
        })
    }
    pub(super) fn census_contextual_complete(&self, variable: Variable, name: &str) -> bool {
        let Some(context) = self.census else {
            return true;
        };
        let head = self.shape_head(variable);
        let head_view = match &head {
            Some(head) => Some(head.as_str()),
            None => None,
        };
        context
            .domains
            .method_problem(head_view, name, false)
            .is_none()
    }
    pub(super) fn census_inherent_complete(&self, variable: Variable, name: &str) -> bool {
        let Some(context) = self.census else {
            return true;
        };
        let head = match self.shape(variable) {
            Some(Shape::Nominal(declaration, _)) => {
                Some(self.source.declarations[declaration].name.as_str())
            }
            _ => None,
        };
        context.domains.method_problem(head, name, true).is_none()
    }
}
impl DefinitionTypes {
    pub(crate) fn structural(&self) -> bool {
        for id in 0..self.nodes.len() {
            let node = &self.nodes[id];
            if node.parent == id
                && !matches!(
                    node.shape,
                    Some(
                        Shape::Atom(_)
                            | Shape::Vec(_)
                            | Shape::Box(_)
                            | Shape::Ref(_, _)
                            | Shape::Slice(_)
                            | Shape::Array(_, _)
                            | Shape::Tuple(_)
                            | Shape::Never
                    )
                )
            {
                return false;
            }
        }
        true
    }
}
