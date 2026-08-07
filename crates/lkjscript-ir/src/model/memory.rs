use super::StructuralDropGlueIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropGlueIdentity {
    ByteVector,
    Bytes,
    Resource(lkjscript_contracts::ResourceKind),
    Structural(StructuralDropGlueIdentity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropEventKind {
    ImplicitCleanup,
    ExplicitClose,
}
