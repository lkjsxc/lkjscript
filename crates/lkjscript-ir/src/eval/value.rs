#[derive(Clone)]
pub struct EvalBuffer {
    id: u64,
    bytes: Rc<RefCell<Vec<u8>>>,
}

impl fmt::Debug for EvalBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let length = self.bytes.try_borrow().map_or(0, |bytes| bytes.len());
        formatter
            .debug_struct("EvalBuffer")
            .field("id", &self.id)
            .field("length", &length)
            .finish()
    }
}

impl PartialEq for EvalBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone)]
pub enum EvalValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Symbol(String),
    /// Transitional traced evaluator buffer.
    Buf(EvalBuffer),
    /// Execution-owned deterministic byte-vector key.
    ByteVector(lkjscript_core::UniqueKeyWord),
    /// Execution-owned shared loan token.
    ByteSlice(u64),
    /// Execution-owned exclusive loan token.
    ByteSliceMut(u64),
    /// Byte-vector backing explicitly transferred across the evaluator boundary.
    ReturnedByteVector(Vec<u8>),
    Path(Vec<u8>),
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(EvalResource),
    Product(ProductId, Vec<Self>),
    Enum {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        physical_tag: u16,
        payload: Vec<Self>,
    },
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
            (Self::Str(left), Self::Str(right)) | (Self::Symbol(left), Self::Symbol(right)) => {
                left == right
            }
            (Self::Buf(left), Self::Buf(right)) => left == right,
            (Self::ByteVector(left), Self::ByteVector(right)) => left == right,
            (Self::ByteSlice(left), Self::ByteSlice(right))
            | (Self::ByteSliceMut(left), Self::ByteSliceMut(right)) => left == right,
            (Self::ReturnedByteVector(left), Self::ReturnedByteVector(right)) => left == right,
            (Self::Path(left), Self::Path(right)) => left == right,
            (Self::Capability(left), Self::Capability(right)) => left == right,
            (Self::Resource(_), Self::Resource(_)) => false,
            (Self::Product(left_id, left), Self::Product(right_id, right)) => {
                left_id == right_id && left == right
            }
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
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Function(left), Self::Function(right)) => left == right,
            _ => false,
        }
    }
}
