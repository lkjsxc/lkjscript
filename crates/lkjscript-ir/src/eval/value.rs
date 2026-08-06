#[derive(Debug)]
pub enum EvalValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    /// Legacy string category retained only for rejected mixed-graph diagnostics.
    Str(String),
    /// Evaluator-session static string artifact identity.
    StaticString(u64),
    /// Evaluator-session static symbol artifact identity.
    StaticSymbol(u64),
    Symbol(String),
    /// Compact owned structural root.
    StructuralOwner(EvalStructuralOwner),
    /// Exact root-table projected loan.
    StructuralView(EvalStructuralView),
    /// Internal UTF-8 loan; it can only be consumed by byte-slice operations or end-borrow.
    StructuralUtf8View(EvalStructuralView),
    /// Private write-once construction destination; never a semantic result.
    StructuralDestination(EvalStructuralDestination),
    /// Key-free owned snapshot transferred across the evaluator boundary.
    ReturnedOwned(lkjscript_core::OwnedValue),
    /// Evaluator-local immutable static constant index.
    StaticBytes(u64),
    /// Execution-owned deterministic immutable-bytes key.
    Bytes(lkjscript_core::UniqueKeyWord),
    /// Execution-owned shared immutable-bytes loan token.
    BytesBorrow(u64),
    /// Execution-owned deterministic byte-vector key.
    ByteVector(lkjscript_core::UniqueKeyWord),
    /// Execution-owned shared loan token.
    ByteSlice(u64),
    /// Execution-owned exclusive loan token.
    ByteSliceMut(u64),
    /// Byte-vector backing explicitly transferred across the evaluator boundary.
    ReturnedByteVector(Vec<u8>),
    /// Immutable bytes snapshot transferred across the evaluator boundary.
    ReturnedBytes(Vec<u8>),
    /// Execution-owned deterministic opaque-path key.
    Path(lkjscript_core::UniqueKeyWord),
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(EvalResource),
    Product(ProductId, Vec<Self>),
    RegionProduct(lkjscript_core::RegionProductKey),
    Enum {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        physical_tag: u64,
        payload: Vec<Self>,
    },
    /// Session-region segmented persistent list handle.
    SegmentedList(lkjscript_core::SegmentedListKey),
    /// Key-free returned-list snapshot.
    List(Vec<Self>),
    Function(FunctionId),
}

impl PartialEq for EvalValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::I64(left), Self::I64(right)) => left == right,
            (Self::F64(left), Self::F64(right)) => left.to_bits() == right.to_bits(),
            (Self::Str(left), Self::Str(right)) => left == right,
            (Self::StaticString(left), Self::StaticString(right)) => left == right,
            (Self::StaticSymbol(left), Self::StaticSymbol(right)) => left == right,
            (Self::Symbol(left), Self::Symbol(right)) => left == right,
            (Self::StructuralOwner(left), Self::StructuralOwner(right)) => left == right,
            (Self::StructuralView(left), Self::StructuralView(right)) => left == right,
            (Self::StructuralUtf8View(left), Self::StructuralUtf8View(right)) => left == right,
            (Self::StructuralDestination(left), Self::StructuralDestination(right)) => left == right,
            (Self::ReturnedOwned(left), Self::ReturnedOwned(right)) => left == right,
            (Self::StaticBytes(left), Self::StaticBytes(right)) => left == right,
            (Self::Bytes(left), Self::Bytes(right)) => left == right,
            (Self::BytesBorrow(left), Self::BytesBorrow(right)) => left == right,
            (Self::ByteVector(left), Self::ByteVector(right)) => left == right,
            (Self::ByteSlice(left), Self::ByteSlice(right))
            | (Self::ByteSliceMut(left), Self::ByteSliceMut(right)) => left == right,
            (Self::ReturnedByteVector(left), Self::ReturnedByteVector(right)) => left == right,
            (Self::ReturnedBytes(left), Self::ReturnedBytes(right)) => left == right,
            (Self::Path(left), Self::Path(right)) => left == right,
            (Self::Capability(left), Self::Capability(right)) => left == right,
            (Self::Resource(_), Self::Resource(_)) => false,
            (Self::Product(left_id, left), Self::Product(right_id, right)) => {
                left_id == right_id && left == right
            }
            (Self::RegionProduct(left), Self::RegionProduct(right)) => left == right,
            (
                Self::Enum {
                    enum_id: le,
                    variant: lv,
                    layout: ll,
                    physical_tag: lt,
                    payload: lp,
                },
                Self::Enum {
                    enum_id: re,
                    variant: rv,
                    layout: rl,
                    physical_tag: rt,
                    payload: rp,
                },
            ) => le == re && lv == rv && ll == rl && lt == rt && lp == rp,
            (Self::SegmentedList(left), Self::SegmentedList(right)) => left == right,
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Function(left), Self::Function(right)) => left == right,
            _ => false,
        }
    }
}
