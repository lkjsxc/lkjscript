use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Source(SourceId),
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingKind {
    Parameter,
    ImmutableLocal,
    MutableLocal,
    Function,
    BuiltinOperation(Operation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub id: BindingId,
    pub name: String,
    pub kind: BindingKind,
    pub ty: Type,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: SourceId,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub sources: Vec<Source>,
    pub bindings: Vec<Binding>,
    pub products: Vec<ProductDefinition>,
    pub enums: Vec<EnumDefinition>,
    pub traits: Vec<TraitDefinition>,
    pub implementations: Vec<ImplDefinition>,
    pub functions: Vec<Function>,
    pub main: Main,
    /// Internal function-closure slots in deterministic bytecode layout order.
    pub global_layout: Vec<BindingId>,
}

impl Program {
    pub fn binding(&self, id: BindingId) -> Option<&Binding> {
        id.index().and_then(|index| self.bindings.get(index))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Main {
    pub origin: SourceId,
    pub return_type: Type,
    pub local_count: u8,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub binding: BindingId,
    pub origin: SourceId,
    pub params: Vec<BindingId>,
    pub param_places: Vec<PlaceId>,
    pub bounds: Vec<TraitBound>,
    pub arity: u8,
    pub local_count: u8,
    pub summary: EffectSet,
    pub body: Expr,
}
