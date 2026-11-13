//! Independent execution of raw K over an independently owned heap and explicit work
//! stack. This model neither invokes the checker nor uses native lowering.
mod io;
mod scalar;
use crate::k::{self, Op, Place, Program, Projection, TypeKind};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub trait Host {
    /// Consume bytes before returning. The borrowed buffer cannot be retained.
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn arguments(&self) -> &[Vec<u8>] {
        &[]
    }
    fn read_stdin(&mut self, _bytes: &mut [u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn write_stderr(&mut self, _bytes: &[u8]) -> Result<(), String> {
        Err("stderr is unavailable".to_string())
    }
    fn open_directory(&mut self, _path: &[u8]) -> Result<u64, i64> {
        Err(38)
    }
    fn open_file(&mut self, _directory: u64, _path: &[u8], _create: bool) -> Result<u64, i64> {
        Err(38)
    }
    fn read_file(&mut self, _handle: u64, _bytes: &mut [u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn write_file(&mut self, _handle: u64, _bytes: &[u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn close_handle(&mut self, _handle: u64) {}
    fn set_exit_status(&mut self, _status: u8) {}
}
impl Host for Vec<u8> {
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum MachineError {
    InvalidK,
    Unsupported,
    UseAfterMove,
    DoubleDrop,
    DanglingReference,
    ConflictingAccess,
    InactiveVariant,
    MissingDrop,
}
#[derive(Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Trap(u64),
    Machine(MachineError),
}
#[derive(Clone, PartialEq, Eq)]
pub struct RunError {
    pub kind: ErrorKind,
    pub function: usize,
    pub file: usize,
    pub line: usize,
    // Resource failures must be reportable without allocating an error message.
    pub message: Cow<'static, str>,
}
impl RunError {
    pub fn code(&self) -> String {
        match &self.kind {
            ErrorKind::Trap(code) => format!("R{code:04}"),
            ErrorKind::Machine(kind) => match kind {
                MachineError::InvalidK => "M_INVALID_K",
                MachineError::Unsupported => "M_UNSUPPORTED",
                MachineError::UseAfterMove => "M_USE_AFTER_MOVE",
                MachineError::DoubleDrop => "M_DOUBLE_DROP",
                MachineError::DanglingReference => "M_DANGLING_REFERENCE",
                MachineError::ConflictingAccess => "M_CONFLICTING_ACCESS",
                MachineError::InactiveVariant => "M_INACTIVE_VARIANT",
                MachineError::MissingDrop => "M_MISSING_DROP",
            }
            .to_string(),
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub enum Scalar {
    Unit,
    Bool(bool),
    Char(char),
    Int { value: u64, bits: u8, signed: bool },
    F64 { bits: u64 },
}
/// Execution resources supplied by the host; they never constrain accepted K.
#[derive(Clone, Copy)]
pub struct Resources {
    pub stack_bytes: usize,
}
impl Resources {
    pub const fn unlimited() -> Self {
        Self {
            stack_bytes: usize::MAX,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RunResult {
    pub value: Scalar,
    pub peak_frames: usize,
    pub peak_cells: usize,
    pub peak_stack_bytes: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// Owner and frame handles keep their slots allocated. References additionally
// retain an identity token, so retirement/reuse needs no lifetime counter.
struct Location {
    index: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct SequenceExtent {
    start: usize,
    count: usize,
}
#[derive(Clone)]
struct Reference {
    extent: Option<SequenceExtent>,
    target: Location,
    identity: Rc<()>,
    tag: usize,
    referent: usize,
    mutable: bool,
}
#[derive(Clone)]
enum Value {
    Unit,
    Bits(u64),
    Aggregate {
        variant: Option<usize>,
        fields: Vec<Location>,
    },
    Sequence(Vec<Location>),
    Box(Location),
    Reference(Reference),
    World,
    Stdout,
    Stdin,
    Stderr,
    Directory(u64),
    File(u64),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Empty {
    Never,
    Moved,
    Dropped,
}
// A retired cell drops its token. Stale references retain the old allocation
// until they disappear; a new occupant always gets a distinct live token.
struct Cell {
    allocated: bool,
    identity: Option<Rc<()>>,
    ty: usize,
    value: Option<Value>,
    empty: Empty,
    parent: Option<Location>,
    immutable: bool,
}
struct Loan {
    extent: Option<SequenceExtent>,
    target: Location,
    identity: Rc<()>,
    mutable: bool,
    parents: Vec<usize>,
    holders: usize,
    children: usize,
}
#[derive(Clone)]
struct Access {
    extent: Option<SequenceExtent>,
    cell: Location,
    ty: usize,
    permissions: Vec<usize>,
    writable: bool,
}
#[derive(Clone, Copy)]
struct Site {
    function: usize,
    file: usize,
    line: usize,
}
#[derive(Clone, Copy)]
enum ReturnTo {
    Entry,
    Cell(Location),
    Discard,
}
struct Frame {
    byte_cost: usize,
    function: usize,
    pc: usize,
    locals: Vec<Location>,
    return_to: ReturnTo,
}
struct DropWork {
    access: Access,
    custom: bool,
    release: bool,
    optional: bool,
    site: Site,
}
enum Work {
    Frame(Frame),
    Drop(DropWork),
}
struct Flow {
    jump: Vec<Option<usize>>,
}

struct RuntimeState<'a> {
    runtime: &'a crate::runtime::Runtime,
    memory: &'a mut crate::runtime_memory::Memory,
}
struct Machine<'a, H: Host> {
    program: &'a Program,
    host: &'a mut H,
    runtime: Option<RuntimeState<'a>>,
    cells: Vec<Cell>,
    free: Vec<usize>,
    live_cells: usize,
    peak_cells: usize,
    loans: BTreeMap<usize, Loan>,
    next_tag: usize,
    free_tags: Vec<usize>,
    work: Vec<Work>,
    frames: usize,
    peak_frames: usize,
    resources: Resources,
    stack_bytes: usize,
    peak_stack_bytes: usize,
    flow: Vec<Option<Flow>>,
    texts: BTreeMap<(usize, Vec<u8>), Location>,
    site: Site,
    result: Option<Scalar>,
}

impl<H: Host> Machine<'_, H> {
    fn error(&self, kind: MachineError, message: impl Into<Cow<'static, str>>) -> RunError {
        RunError {
            kind: ErrorKind::Machine(kind),
            function: self.site.function,
            file: self.site.file,
            line: self.site.line,
            message: message.into(),
        }
    }
    fn invalid(&self, message: impl Into<Cow<'static, str>>) -> RunError {
        self.error(MachineError::InvalidK, message)
    }
    fn trap(&self, code: u64, message: impl Into<Cow<'static, str>>) -> RunError {
        RunError {
            kind: ErrorKind::Trap(code),
            function: self.site.function,
            file: self.site.file,
            line: self.site.line,
            message: message.into(),
        }
    }
    fn number(&self, result: Result<u64, scalar::NumberError>) -> Result<u64, RunError> {
        result.map_err(|error| match error {
            scalar::NumberError::Trap(1) => self.trap(1, "integer overflow"),
            scalar::NumberError::Trap(code) => self.trap(u64::from(code), "division by zero"),
            scalar::NumberError::Invalid(message) => self.invalid(message),
        })
    }
    fn kind(&self, ty: usize) -> Result<&TypeKind, RunError> {
        let kind = &self
            .program
            .types
            .get(ty)
            .ok_or_else(|| self.invalid("unknown type"))?
            .kind;
        if matches!(kind, TypeKind::Int {bits,..} if ![8,16,32,64].contains(bits)) {
            return Err(self.invalid("invalid integer width"));
        }
        Ok(kind)
    }
    fn cell(&self, location: Location) -> Result<&Cell, RunError> {
        self.cells
            .get(location.index)
            .filter(|cell| cell.allocated)
            .ok_or_else(|| {
                self.error(
                    MachineError::DanglingReference,
                    "location has been released",
                )
            })
    }
    fn same_occupant(&self, location: Location, identity: &Rc<()>) -> bool {
        self.cells
            .get(location.index)
            .and_then(|cell| cell.identity.as_ref())
            .is_some_and(|current| Rc::ptr_eq(current, identity))
    }
    fn value(&self, location: Location) -> Result<&Value, RunError> {
        self.cell(location)?.value.as_ref().ok_or_else(|| {
            self.error(
                MachineError::UseAfterMove,
                "location is uninitialized, moved, or dropped",
            )
        })
    }
    fn allocate(
        &mut self,
        ty: usize,
        value: Option<Value>,
        immutable: bool,
    ) -> Result<Location, RunError> {
        self.kind(ty)?;
        let index = match self.free.pop() {
            Some(index) => {
                self.cells[index] = Cell {
                    allocated: true,
                    identity: None,
                    ty,
                    value,
                    empty: Empty::Never,
                    parent: None,
                    immutable,
                };
                index
            }
            None => {
                self.cells
                    .try_reserve(1)
                    .map_err(|_| self.trap(6, "allocation failed"))?;
                let index = self.cells.len();
                self.cells.push(Cell {
                    allocated: true,
                    identity: None,
                    ty,
                    value,
                    empty: Empty::Never,
                    parent: None,
                    immutable,
                });
                index
            }
        };
        let location = Location { index };
        self.live_cells += 1;
        self.peak_cells = self.peak_cells.max(self.live_cells);
        self.reparent(location)?;
        Ok(location)
    }
    fn children(value: &Value) -> &[Location] {
        match value {
            Value::Aggregate { fields, .. } | Value::Sequence(fields) => fields,
            Value::Box(child) => std::slice::from_ref(child),
            _ => &[],
        }
    }
    fn reparent(&mut self, location: Location) -> Result<(), RunError> {
        let children = self
            .cell(location)?
            .value
            .as_ref()
            .map(Self::children)
            .unwrap_or(&[])
            .to_vec();
        for child in children {
            self.cell(child)?;
            self.cells[child.index].parent = Some(location);
        }
        Ok(())
    }
    fn release(&mut self, location: Location) -> Result<(), RunError> {
        if self.cell(location)?.value.is_some() {
            return Err(self.error(MachineError::MissingDrop, "releasing initialized storage"));
        }
        self.cells[location.index].allocated = false;
        self.cells[location.index].identity = None;
        self.cells[location.index].parent = None;
        self.free.push(location.index);
        self.live_cells -= 1;
        Ok(())
    }
    fn ancestor(&self, ancestor: Location, mut cell: Location) -> bool {
        loop {
            if cell == ancestor {
                return true;
            }
            let Ok(value) = self.cell(cell) else {
                return false;
            };
            let Some(parent) = value.parent else {
                return false;
            };
            cell = parent;
        }
    }
    fn overlaps(&self, a: Location, b: Location) -> bool {
        self.ancestor(a, b) || self.ancestor(b, a)
    }
    fn permissions(&self, direct: &[usize]) -> BTreeSet<usize> {
        let mut result = BTreeSet::new();
        let mut pending = direct.to_vec();
        while let Some(tag) = pending.pop() {
            if result.insert(tag) {
                if let Some(loan) = self.loans.get(&tag) {
                    pending.extend_from_slice(&loan.parents);
                }
            }
        }
        result
    }
    fn access(&self, access: &Access, write: bool, initialized: bool) -> Result<(), RunError> {
        let cell = self.cell(access.cell)?;
        if initialized {
            self.value(access.cell)?;
        }
        if write && cell.immutable {
            return Err(self.error(
                MachineError::ConflictingAccess,
                "write to immutable static storage",
            ));
        }
        let permitted = self.permissions(&access.permissions);
        for (tag, loan) in &self.loans {
            if !permitted.contains(tag)
                && self.same_occupant(loan.target, &loan.identity)
                && self.overlaps(access.cell, loan.target)
                && (write || loan.mutable)
            {
                return Err(self.error(
                    MachineError::ConflictingAccess,
                    format!(
                        "access to location {} conflicts with loan {} on location {}",
                        access.cell.index, tag, loan.target.index
                    ),
                ));
            }
        }
        Ok(())
    }
    fn reference_access(&self, reference: &Reference) -> Result<Access, RunError> {
        let loan = self.loans.get(&reference.tag).ok_or_else(|| {
            self.error(MachineError::DanglingReference, "reference tag has expired")
        })?;
        if !Rc::ptr_eq(&loan.identity, &reference.identity) {
            return Err(self.error(
                MachineError::DanglingReference,
                "loan tag belongs to another storage identity",
            ));
        }
        if loan.target != reference.target
            || loan.mutable != reference.mutable
            || loan.extent != reference.extent
        {
            return Err(self.invalid("reference tag differs from its borrow"));
        }
        if !self.same_occupant(reference.target, &reference.identity) {
            return Err(self.error(
                MachineError::DanglingReference,
                "reference points to a retired occupant of a reused slot",
            ));
        }
        let unsized_view = matches!(
            self.kind(reference.referent)?,
            TypeKind::Slice { .. } | TypeKind::Str
        );
        if unsized_view != reference.extent.is_some() {
            return Err(self.invalid("reference extent differs from its referent"));
        }
        let access = Access {
            extent: reference.extent,
            cell: reference.target,
            ty: reference.referent,
            permissions: vec![reference.tag],
            writable: reference.mutable,
        };
        if unsized_view {
            self.sequence(&access)?;
        }
        Ok(access)
    }
    fn borrow(
        &mut self,
        access: &Access,
        referent: usize,
        mutable: bool,
    ) -> Result<Value, RunError> {
        if mutable && !access.writable {
            return Err(self.error(
                MachineError::ConflictingAccess,
                "mutable borrow through an immutable place",
            ));
        }
        self.access(access, mutable, true)?;
        let extent = if matches!(self.kind(referent)?, TypeKind::Slice { .. } | TypeKind::Str) {
            let count = self.sequence(access)?.len();
            Some(access.extent.unwrap_or(SequenceExtent { start: 0, count }))
        } else {
            None
        };
        let tag = if let Some(tag) = self.free_tags.pop() {
            tag
        } else {
            let tag = self.next_tag;
            self.next_tag = tag.checked_add(1).ok_or_else(|| {
                self.trap(6, "live borrow identities exceed the host representation")
            })?;
            tag
        };
        let parents = self
            .permissions(&access.permissions)
            .into_iter()
            .collect::<Vec<_>>();
        for parent in &parents {
            self.loans.get_mut(parent).unwrap().children += 1;
        }
        // Unborrowed cells need no allocated identity. Internal owner handles
        // retain their slots; only references can survive a storage retirement.
        if self.cells[access.cell.index].identity.is_none() {
            self.cells[access.cell.index].identity = Some(Rc::new(()));
        }
        let identity = self.cell(access.cell)?.identity.as_ref().unwrap().clone();
        self.loans.insert(
            tag,
            Loan {
                extent,
                target: access.cell,
                identity: identity.clone(),
                mutable,
                parents,
                holders: 1,
                children: 0,
            },
        );
        Ok(Value::Reference(Reference {
            extent,
            target: access.cell,
            identity,
            tag,
            referent,
            mutable,
        }))
    }
    fn release_reference(&mut self, reference: &Reference) -> Result<(), RunError> {
        if !self
            .loans
            .get(&reference.tag)
            .is_some_and(|loan| Rc::ptr_eq(&loan.identity, &reference.identity))
        {
            return Err(self.error(
                MachineError::DanglingReference,
                "reference loan identity has expired",
            ));
        }
        let Some(loan) = self.loans.get_mut(&reference.tag) else {
            return Err(self.error(
                MachineError::DanglingReference,
                "dropping an expired reference tag",
            ));
        };
        if loan.holders == 0 {
            return Err(self.error(MachineError::DoubleDrop, "reference holder already dropped"));
        }
        loan.holders -= 1;
        let mut pending = vec![reference.tag];
        while let Some(tag) = pending.pop() {
            if self
                .loans
                .get(&tag)
                .is_some_and(|loan| loan.holders == 0 && loan.children == 0)
            {
                let loan = self.loans.remove(&tag).unwrap();
                self.free_tags.push(tag);
                for parent in loan.parents {
                    let Some(parent_loan) = self.loans.get_mut(&parent) else {
                        return Err(self.error(
                            MachineError::DanglingReference,
                            "reborrow parent is missing",
                        ));
                    };
                    parent_loan.children -= 1;
                    pending.push(parent);
                }
            }
        }
        Ok(())
    }
    fn discard_scalar(&mut self, value: Value) -> Result<(), RunError> {
        match value {
            Value::Reference(reference) => self.release_reference(&reference),
            Value::Aggregate { .. }
            | Value::Sequence(_)
            | Value::Box(_)
            | Value::Directory(_)
            | Value::File(_) => Err(self.error(
                MachineError::MissingDrop,
                "owned value requires structural destruction",
            )),
            _ => Ok(()),
        }
    }
    fn local(&self, frame: &Frame, local: usize) -> Result<Access, RunError> {
        let cell = *frame
            .locals
            .get(local)
            .ok_or_else(|| self.invalid("local index is not declared"))?;
        let declaration = &self.program.functions[frame.function].locals[local];
        Ok(Access {
            extent: None,
            cell,
            ty: declaration.ty,
            permissions: Vec::new(),
            writable: declaration.mutable,
        })
    }
    fn resolve(&self, frame: &Frame, place: &Place) -> Result<Access, RunError> {
        let mut access = self.local(frame, place.local)?;
        for projection in &place.projections {
            let value = self.value(access.cell)?;
            match projection {
                Projection::Deref => match (self.kind(access.ty)?, value) {
                    (TypeKind::Ref { inner, mutable }, Value::Reference(reference))
                        if *inner == reference.referent && *mutable == reference.mutable =>
                    {
                        self.access(&access, false, true)?;
                        let next = self.reference_access(reference)?;
                        access.writable =
                            *mutable && (access.permissions.is_empty() || access.writable);
                        // Following a shared reference copies its independent permission.
                        // Reading its descriptor was checked above; the descriptor's
                        // carrier need not stay borrowed. The reference's own parents
                        // still preserve any exclusive ancestor. Mutable references
                        // cannot be copied, so their carrier path remains a parent.
                        if !*mutable {
                            access.permissions.clear();
                        }
                        access.permissions.extend_from_slice(&next.permissions);
                        access.cell = next.cell;
                        access.extent = next.extent;
                        access.ty = *inner;
                    }
                    (TypeKind::Box { inner }, Value::Box(child)) => {
                        access.extent = None;
                        access.cell = *child;
                        access.ty = *inner;
                    }
                    _ => return Err(self.invalid("dereference requires a reference or Box")),
                },
                Projection::Field(index) => match (self.kind(access.ty)?, value) {
                    (
                        TypeKind::Struct { fields, .. },
                        Value::Aggregate {
                            variant: None,
                            fields: values,
                        },
                    ) => {
                        access.ty = fields
                            .get(*index)
                            .ok_or_else(|| self.invalid("unknown field"))?
                            .ty;
                        access.extent = None;
                        access.cell = *values
                            .get(*index)
                            .ok_or_else(|| self.invalid("missing field storage"))?;
                    }
                    _ => return Err(self.invalid("field projection requires a struct")),
                },
                Projection::VariantField(variant, index) => match (self.kind(access.ty)?, value) {
                    (
                        TypeKind::Enum { variants, .. },
                        Value::Aggregate {
                            variant: Some(active),
                            fields,
                        },
                    ) => {
                        if variant != active {
                            return Err(self.error(
                                MachineError::InactiveVariant,
                                "projection uses an inactive enum variant",
                            ));
                        }
                        access.ty = variants
                            .get(*variant)
                            .and_then(|variant| variant.fields.get(*index))
                            .ok_or_else(|| self.invalid("unknown variant field"))?
                            .ty;
                        access.extent = None;
                        access.cell = *fields
                            .get(*index)
                            .ok_or_else(|| self.invalid("missing variant storage"))?;
                    }
                    _ => return Err(self.invalid("variant projection requires an enum")),
                },
                Projection::Index(index) => {
                    let index = self.local(frame, *index)?;
                    self.access(&index, false, true)?;
                    if index.ty != k::U64 {
                        return Err(self.invalid("collection index must be usize"));
                    }
                    let Value::Bits(index) = self.value(index.cell)? else {
                        return Err(self.invalid("index is not scalar"));
                    };
                    let inner = match self.kind(access.ty)? {
                        TypeKind::Vec { inner }
                        | TypeKind::Array { inner, .. }
                        | TypeKind::Slice { inner, .. } => *inner,
                        _ => return Err(self.invalid("indexing requires a collection")),
                    };
                    let elements = self.sequence(&access)?;
                    access.cell = *usize::try_from(*index)
                        .ok()
                        .and_then(|index| elements.get(index))
                        .ok_or_else(|| self.trap(3, "index out of bounds"))?;
                    access.extent = None;
                    access.ty = inner;
                }
            }
        }
        self.cell(access.cell)?;
        Ok(access)
    }
    fn take(&mut self, access: &Access) -> Result<Value, RunError> {
        self.access(access, true, true)?;
        let value = self.cells[access.cell.index].value.take().unwrap();
        self.cells[access.cell.index].empty = Empty::Moved;
        for child in Self::children(&value) {
            self.cells[child.index].parent = None;
        }
        Ok(value)
    }
    fn take_local(&mut self, frame: &Frame, local: usize) -> Result<(usize, Value), RunError> {
        let access = self.local(frame, local)?;
        Ok((access.ty, self.take(&access)?))
    }
    fn validate_value(&self, ty: usize, value: &Value) -> Result<(), RunError> {
        let valid =
            match (self.kind(ty)?, value) {
                (TypeKind::Unit, Value::Unit) => true,
                (TypeKind::Bool, Value::Bits(value)) => *value <= 1,
                (TypeKind::Char, Value::Bits(value)) => u32::try_from(*value)
                    .ok()
                    .and_then(char::from_u32)
                    .is_some(),
                (TypeKind::Int { bits, signed }, Value::Bits(value))
                    if [8, 16, 32, 64].contains(bits) =>
                {
                    scalar::normalize(*value, *bits, *signed) == *value
                }
                (TypeKind::F64, Value::Bits(_)) => true,
                (TypeKind::World, Value::World)
                | (TypeKind::Stdout, Value::Stdout)
                | (TypeKind::Stdin, Value::Stdin)
                | (TypeKind::Stderr, Value::Stderr) => true,
                (TypeKind::Directory, Value::Directory(handle))
                | (TypeKind::File, Value::File(handle)) => *handle != 0,
                (TypeKind::Ref { inner, mutable }, Value::Reference(reference)) => {
                    *inner == reference.referent && *mutable == reference.mutable
                }
                (
                    TypeKind::Struct { fields, .. },
                    Value::Aggregate {
                        variant: None,
                        fields: values,
                    },
                ) => {
                    fields.len() == values.len()
                        && fields.iter().zip(values).all(|(field, cell)| {
                            self.cell(*cell).is_ok_and(|cell| cell.ty == field.ty)
                        })
                }
                (
                    TypeKind::Enum { variants, .. },
                    Value::Aggregate {
                        variant: Some(variant),
                        fields: values,
                    },
                ) => variants.get(*variant).is_some_and(|variant| {
                    variant.fields.len() == values.len()
                        && variant.fields.iter().zip(values).all(|(field, cell)| {
                            self.cell(*cell).is_ok_and(|cell| cell.ty == field.ty)
                        })
                }),
                (TypeKind::Vec { inner }, Value::Sequence(values)) => values
                    .iter()
                    .all(|value| self.cell(*value).is_ok_and(|cell| cell.ty == *inner)),
                (TypeKind::Array { inner, len }, Value::Sequence(values)) => {
                    *len == values.len()
                        && values
                            .iter()
                            .all(|value| self.cell(*value).is_ok_and(|cell| cell.ty == *inner))
                }
                (TypeKind::Str, Value::Sequence(values)) => values
                    .iter()
                    .all(|value| self.cell(*value).is_ok_and(|cell| cell.ty == k::U8)),
                (TypeKind::Box { inner }, Value::Box(value)) => self.cell(*value)?.ty == *inner,
                _ => false,
            };
        if valid {
            Ok(())
        } else {
            Err(self.invalid("value does not inhabit its declared type"))
        }
    }
    fn put(&mut self, cell: Location, value: Value) -> Result<(), RunError> {
        let target = self.cell(cell)?;
        if target.value.is_some() {
            return Err(self.error(MachineError::MissingDrop, "destination still owns a value"));
        }
        self.validate_value(target.ty, &value)?;
        self.cells[cell.index].value = Some(value);
        self.cells[cell.index].empty = Empty::Never;
        self.reparent(cell)
    }
    fn put_local(&mut self, frame: &Frame, local: usize, value: Value) -> Result<(), RunError> {
        let destination = self.local(frame, local)?;
        self.put(destination.cell, value)
    }
    fn is_copy(&self, ty: usize) -> Result<bool, RunError> {
        let mut pending = vec![ty];
        let mut seen = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            if !seen.insert(ty) {
                continue;
            }
            match self.kind(ty)? {
                TypeKind::Unit
                | TypeKind::Bool
                | TypeKind::Char
                | TypeKind::Int { .. }
                | TypeKind::F64 => {}
                TypeKind::Ref { mutable: false, .. } => {}
                TypeKind::Array { inner, .. } => pending.push(*inner),
                TypeKind::Struct { fields, copy: true } => {
                    pending.extend(fields.iter().map(|field| field.ty))
                }
                TypeKind::Enum {
                    variants,
                    copy: true,
                } => pending.extend(
                    variants
                        .iter()
                        .flat_map(|variant| variant.fields.iter().map(|field| field.ty)),
                ),
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
    fn copy_value(&mut self, access: &Access) -> Result<Value, RunError> {
        self.access(access, false, true)?;
        if !self.is_copy(access.ty)? {
            return Err(self.invalid("non-Copy value copied"));
        }
        let destination = self.allocate(access.ty, None, false)?;
        let mut pending = vec![(access.cell, destination)];
        while let Some((source, destination)) = pending.pop() {
            let mut value = self.value(source)?.clone();
            match &mut value {
                Value::Aggregate { fields, .. } | Value::Sequence(fields) => {
                    for field in fields {
                        let replacement = self.allocate(self.cell(*field)?.ty, None, false)?;
                        pending.push((*field, replacement));
                        *field = replacement;
                    }
                }
                Value::Reference(reference) => {
                    self.reference_access(reference)?;
                    self.loans.get_mut(&reference.tag).unwrap().holders += 1;
                }
                _ => {}
            }
            self.put(destination, value)?;
        }
        let result = self.cells[destination.index].value.take().unwrap();
        for child in Self::children(&result) {
            self.cells[child.index].parent = None;
        }
        self.release(destination)?;
        Ok(result)
    }
    fn prepare_flow(&self, function: usize) -> Result<Flow, RunError> {
        let body = &self
            .program
            .functions
            .get(function)
            .ok_or_else(|| self.invalid("unknown function"))?
            .body;
        let mut jump = vec![None; body.len()];
        let mut controls: Vec<(usize, bool, Option<usize>, Vec<usize>)> = Vec::new();
        for (pc, instruction) in body.iter().enumerate() {
            match instruction.op {
                Op::If { .. } => controls.push((pc, false, None, Vec::new())),
                Op::Loop => controls.push((pc, true, None, Vec::new())),
                Op::Else => {
                    let control = controls
                        .last_mut()
                        .ok_or_else(|| self.invalid("else outside if"))?;
                    if control.1 || control.2.is_some() {
                        return Err(self.invalid("invalid else"));
                    }
                    control.2 = Some(pc);
                }
                Op::End => {
                    let (open, is_loop, otherwise, breaks) = controls
                        .pop()
                        .ok_or_else(|| self.invalid("end outside control"))?;
                    if is_loop {
                        jump[pc] = Some(open + 1);
                        for transfer in breaks {
                            jump[transfer] = Some(pc + 1);
                        }
                    } else {
                        jump[open] = Some(otherwise.map_or(pc + 1, |otherwise| otherwise + 1));
                        if let Some(otherwise) = otherwise {
                            jump[otherwise] = Some(pc + 1);
                        }
                    }
                }
                Op::Break | Op::Continue | Op::BreakTo { .. } | Op::ContinueTo { .. } => {
                    let depth = match instruction.op {
                        Op::BreakTo { depth } | Op::ContinueTo { depth } => depth,
                        _ => 0,
                    };
                    let control = controls
                        .iter_mut()
                        .rev()
                        .filter(|control| control.1)
                        .nth(depth)
                        .ok_or_else(|| {
                            self.invalid("loop transfer target is outside enclosing loops")
                        })?;
                    if matches!(instruction.op, Op::Break | Op::BreakTo { .. }) {
                        control.3.push(pc);
                    } else {
                        jump[pc] = Some(control.0 + 1);
                    }
                }
                _ => {}
            }
        }
        if !controls.is_empty() {
            return Err(self.invalid("unclosed control block"));
        }
        Ok(Flow { jump })
    }
    fn runtime_call(
        &mut self,
        function: usize,
        arguments: &[(usize, Value)],
    ) -> Result<Option<crate::runtime_memory::Action>, RunError> {
        let Some(primitive) = self
            .runtime
            .as_ref()
            .and_then(|state| state.runtime.primitive(function))
        else {
            return Ok(None);
        };
        let declaration = &self.program.functions[function];
        if declaration.params != arguments.len() {
            return Err(self.invalid("runtime primitive argument count differs"));
        }
        let mut words = Vec::new();
        for (index, (ty, value)) in arguments.iter().enumerate() {
            if *ty != declaration.locals[index].ty {
                return Err(self.invalid("runtime primitive argument type differs"));
            }
            let Value::Bits(value) = value else {
                return Err(self.invalid("runtime primitive argument is not a word"));
            };
            words.push(*value);
        }
        let action = self
            .runtime
            .as_mut()
            .unwrap()
            .memory
            .invoke(primitive, &words);
        action.map(Some).map_err(|fault| {
            if fault == crate::runtime_memory::MemoryFault::Allocation {
                self.trap(6, fault.message())
            } else {
                self.invalid(fault.message())
            }
        })
    }
    fn push_frame(
        &mut self,
        function: usize,
        args: Vec<(usize, Value)>,
        return_to: ReturnTo,
    ) -> Result<(), RunError> {
        let declaration = self
            .program
            .functions
            .get(function)
            .ok_or_else(|| self.invalid("unknown function"))?;
        if declaration.params != args.len() || declaration.params > declaration.locals.len() {
            return Err(self.invalid("call argument count differs"));
        }
        for (index, (ty, value)) in args.iter().enumerate() {
            if *ty != declaration.locals[index].ty {
                return Err(self.invalid("call argument type differs"));
            }
            self.validate_value(*ty, value)?;
        }
        // Frame entry allocates only root cells and their location table.
        // Aggregate, array, Vec and Box payloads allocate separately on the heap.
        let local_bytes = std::mem::size_of::<Location>() + std::mem::size_of::<Cell>();
        let byte_cost = declaration
            .locals
            .len()
            .checked_mul(local_bytes)
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Frame>()))
            .ok_or_else(|| self.trap(8, "stack extent exhausted"))?;
        let stack_bytes = self
            .stack_bytes
            .checked_add(byte_cost)
            .filter(|bytes| *bytes <= self.resources.stack_bytes)
            .ok_or_else(|| self.trap(8, "stack extent exhausted"))?;
        if self.flow[function].is_none() {
            self.flow[function] = Some(self.prepare_flow(function)?);
        }
        let mut locals = Vec::new();
        locals
            .try_reserve_exact(declaration.locals.len())
            .map_err(|_| self.trap(8, "stack extent exhausted"))?;
        for local in &declaration.locals {
            let location = self.allocate(local.ty, None, false).map_err(|error| {
                if error.kind == ErrorKind::Trap(6) {
                    self.trap(8, "stack extent exhausted")
                } else {
                    error
                }
            })?;
            locals.push(location);
        }
        for (index, (_, value)) in args.into_iter().enumerate() {
            self.put(locals[index], value)?;
        }
        self.push_work(Work::Frame(Frame {
            byte_cost,
            function,
            pc: 0,
            locals,
            return_to,
        }))?;
        self.frames += 1;
        self.peak_frames = self.peak_frames.max(self.frames);
        self.stack_bytes = stack_bytes;
        self.peak_stack_bytes = self.peak_stack_bytes.max(stack_bytes);
        Ok(())
    }
    fn push_work(&mut self, work: Work) -> Result<(), RunError> {
        self.work
            .try_reserve(1)
            .map_err(|_| self.trap(8, "stack extent exhausted"))?;
        self.work.push(work);
        Ok(())
    }
    fn scalar_result(&self, ty: usize, value: &Value) -> Result<Scalar, RunError> {
        match (self.kind(ty)?, value) {
            (TypeKind::Unit, Value::Unit) => Ok(Scalar::Unit),
            (TypeKind::Bool, Value::Bits(value)) => Ok(Scalar::Bool(*value != 0)),
            (TypeKind::Char, Value::Bits(value)) => u32::try_from(*value)
                .ok()
                .and_then(char::from_u32)
                .map(Scalar::Char)
                .ok_or_else(|| self.invalid("invalid character result")),
            (TypeKind::Int { bits, signed }, Value::Bits(value)) => Ok(Scalar::Int {
                value: *value,
                bits: *bits,
                signed: *signed,
            }),
            (TypeKind::F64, Value::Bits(bits)) => Ok(Scalar::F64 { bits: *bits }),
            _ => Err(self.error(
                MachineError::Unsupported,
                "host result must be scalar or unit",
            )),
        }
    }
    fn finish_frame(&mut self, frame: Frame, ty: usize, value: Value) -> Result<(), RunError> {
        if ty != self.program.functions[frame.function].result {
            return Err(self.invalid("returned type differs from the signature"));
        }
        // Check escaping addresses before reporting leaked locals: this names the
        // actual dangling reference rather than trusting a static return contract.
        let mut pending = vec![value.clone()];
        while let Some(value) = pending.pop() {
            if let Value::Reference(reference) = &value {
                self.reference_access(reference)?;
                if frame
                    .locals
                    .iter()
                    .any(|local| self.ancestor(*local, reference.target))
                {
                    return Err(self.error(
                        MachineError::DanglingReference,
                        "returned reference points into the retiring frame",
                    ));
                }
            }
            for child in Self::children(&value) {
                if let Some(value) = &self.cell(*child)?.value {
                    pending.push(value.clone());
                }
            }
        }
        for local in &frame.locals {
            if self.cell(*local)?.value.is_some() {
                return Err(self.error(
                    MachineError::MissingDrop,
                    "return leaves a live local without explicit cleanup",
                ));
            }
        }
        for local in frame.locals {
            self.release(local)?;
        }
        self.frames -= 1;
        self.stack_bytes -= frame.byte_cost;
        match frame.return_to {
            ReturnTo::Entry => {
                self.result = Some(self.scalar_result(ty, &value)?);
                self.discard_scalar(value)?;
            }
            ReturnTo::Cell(destination) => {
                if self.cell(destination)?.ty != ty {
                    return Err(self.invalid("call result type differs"));
                }
                self.put(destination, value)?;
            }
            ReturnTo::Discard => {
                if ty != k::UNIT {
                    return Err(self.invalid("destructor returns a value"));
                }
                self.discard_scalar(value)?;
            }
        }
        Ok(())
    }
    fn drop_work(&mut self, work: DropWork) -> Result<(), RunError> {
        self.site = work.site;
        if self.cell(work.access.cell)?.value.is_none() {
            // Structural cleanup and explicit DropIf skip an absent value.
            // A strict Drop requires this exact storage to be initialized.
            if !work.optional {
                return Err(self.error(
                    MachineError::DoubleDrop,
                    "drop of uninitialized, moved, or already dropped storage",
                ));
            }
            self.cells[work.access.cell.index].empty = Empty::Dropped;
            if work.release {
                self.release(work.access.cell)?;
            }
            return Ok(());
        }
        self.access(&work.access, true, true)?;
        if work.custom {
            if let Some((_, function)) = self
                .program
                .destructors
                .iter()
                .find(|entry| entry.0 == work.access.ty)
            {
                let function = *function;
                let declaration = self
                    .program
                    .functions
                    .get(function)
                    .ok_or_else(|| self.invalid("unknown destructor"))?;
                if declaration.params != 1 || declaration.result != k::UNIT {
                    return Err(self.invalid("invalid destructor signature"));
                }
                let parameter = declaration
                    .locals
                    .first()
                    .ok_or_else(|| self.invalid("missing destructor parameter"))?
                    .ty;
                if !matches!(self.kind(parameter)?, TypeKind::Ref { inner, mutable: true } if *inner == work.access.ty)
                {
                    return Err(self.invalid("destructor needs a mutable receiver"));
                }
                let mut access = work.access.clone();
                access.writable = true;
                let value = self.borrow(&access, access.ty, true)?;
                self.push_work(Work::Drop(DropWork {
                    custom: false,
                    ..work
                }))?;
                self.push_frame(function, vec![(parameter, value)], ReturnTo::Discard)?;
                return Ok(());
            }
        }
        let value = self.take(&work.access)?;
        self.cells[work.access.cell.index].empty = Empty::Dropped;
        let children = Self::children(&value).to_vec();
        match value {
            Value::Reference(reference) => self.release_reference(&reference)?,
            Value::Directory(handle) | Value::File(handle) => self.host.close_handle(handle),
            _ => {}
        }
        if work.release {
            self.release(work.access.cell)?;
        }
        for child in children.into_iter().rev() {
            let ty = self.cell(child)?.ty;
            self.push_work(Work::Drop(DropWork {
                access: Access {
                    extent: None,
                    cell: child,
                    ty,
                    permissions: Vec::new(),
                    writable: true,
                },
                custom: true,
                release: true,
                optional: true,
                site: work.site,
            }))?;
        }
        Ok(())
    }
    fn fields(
        &mut self,
        frame: &Frame,
        fields: &[usize],
        expected: &[usize],
    ) -> Result<Vec<Location>, RunError> {
        if fields.len() != expected.len() {
            return Err(self.invalid("constructor field count differs"));
        }
        let mut values = Vec::new();
        for (field, expected) in fields.iter().zip(expected) {
            let (ty, value) = self.take_local(frame, *field)?;
            if ty != *expected {
                return Err(self.invalid("constructor field type differs"));
            }
            values.push(self.allocate(ty, Some(value), false)?);
        }
        Ok(values)
    }
    fn sequence(&self, access: &Access) -> Result<&[Location], RunError> {
        let Value::Sequence(elements) = self.value(access.cell)? else {
            return Err(self.invalid("view requires sequence storage"));
        };
        let extent = access.extent.unwrap_or(SequenceExtent {
            start: 0,
            count: elements.len(),
        });
        extent
            .start
            .checked_add(extent.count)
            .and_then(|end| elements.get(extent.start..end))
            .ok_or_else(|| self.invalid("reference extent exceeds sequence storage"))
    }
    fn bytes(&self, access: &Access) -> Result<Vec<u8>, RunError> {
        self.access(access, false, true)?;
        let elements = self.sequence(access)?;
        let mut bytes = Vec::new();
        for element in elements {
            let element_access = Access {
                extent: None,
                cell: *element,
                ty: k::U8,
                permissions: access.permissions.clone(),
                writable: false,
            };
            self.access(&element_access, false, true)?;
            if self.cell(*element)?.ty != k::U8 {
                return Err(self.invalid("byte view contains a non-byte element"));
            }
            let Value::Bits(value) = self.value(*element)? else {
                return Err(self.invalid("byte is not scalar"));
            };
            bytes.push(
                u8::try_from(*value)
                    .map_err(|_| self.invalid("byte exceeds its representation"))?,
            );
        }
        Ok(bytes)
    }
    fn array_repeat(&mut self, frame: &Frame, dst: usize, args: &[usize]) -> Result<(), RunError> {
        let destination = self.local(frame, dst)?;
        let (inner, len) = match self.kind(destination.ty)? {
            TypeKind::Array { inner, len } => (*inner, *len),
            _ => return Err(self.invalid("array repeat result must be an array")),
        };
        if args.len() != 1 || len < 2 {
            return Err(self.invalid("array repeat needs one seed and an extent of at least two"));
        }
        let seed = self.local(frame, args[0])?;
        if seed.ty != inner || !self.is_copy(inner)? {
            return Err(self.invalid("array repeat seed must have the Copy element type"));
        }
        let mut fields = Vec::new();
        fields
            .try_reserve(len)
            .map_err(|_| self.trap(6, "allocation failed"))?;
        for _ in 0..len - 1 {
            let value = self.copy_value(&seed)?;
            fields.push(self.allocate(inner, Some(value), false)?);
        }
        let value = self.take(&seed)?;
        fields.push(self.allocate(inner, Some(value), false)?);
        self.put_local(frame, dst, Value::Sequence(fields))
    }

    fn intrinsic(
        &mut self,
        frame: &Frame,
        dst: usize,
        name: &str,
        args: &[usize],
    ) -> Result<(), RunError> {
        if name == "array_repeat" {
            return self.array_repeat(frame, dst, args);
        }
        let destination = self.local(frame, dst)?;
        let mut values = Vec::new();
        for arg in args {
            values.push(self.take_local(frame, *arg)?);
        }
        let reference = |index: usize, machine: &Self| -> Result<Access, RunError> {
            match values.get(index).map(|value| &value.1) {
                Some(Value::Reference(reference)) => machine.reference_access(reference),
                _ => Err(machine.invalid("intrinsic requires a reference argument")),
            }
        };
        let result = match name {
            "assert" if values.len() == 1 => {
                if values[0].0 != k::BOOL {
                    return Err(self.invalid("assert needs bool"));
                }
                if !matches!(values[0].1, Value::Bits(1)) {
                    return Err(self.trap(5, "assertion failed"));
                }
                Value::Unit
            }
            "char_from_u32" if values.len() == 1 => {
                if !matches!(
                    self.kind(values[0].0)?,
                    TypeKind::Int {
                        bits: 32,
                        signed: false
                    }
                ) {
                    return Err(self.invalid("character construction requires u32"));
                }
                let TypeKind::Enum { variants, .. } = self.kind(destination.ty)? else {
                    return Err(self.invalid("character construction requires Option<char>"));
                };
                if variants.len() != 2
                    || variants[0].name != "None"
                    || !variants[0].fields.is_empty()
                    || variants[1].name != "Some"
                    || variants[1].fields.len() != 1
                    || !matches!(self.kind(variants[1].fields[0].ty)?, TypeKind::Char)
                {
                    return Err(self.invalid("character construction result shape differs"));
                }
                let character = variants[1].fields[0].ty;
                let Value::Bits(code) = values[0].1 else {
                    return Err(self.invalid("character code must be scalar"));
                };
                match u32::try_from(code).ok().and_then(char::from_u32) {
                    Some(value) => {
                        let field =
                            self.allocate(character, Some(Value::Bits(value as u64)), false)?;
                        Value::Aggregate {
                            variant: Some(1),
                            fields: vec![field],
                        }
                    }
                    None => Value::Aggregate {
                        variant: Some(0),
                        fields: Vec::new(),
                    },
                }
            }
            "cast" if values.len() == 1 => {
                let Value::Bits(value) = values[0].1 else {
                    return Err(self.invalid("cast requires a scalar"));
                };
                Value::Bits(self.number(scalar::cast(
                    self.kind(values[0].0)?,
                    self.kind(destination.ty)?,
                    value,
                ))?)
            }
            "vec_new"
                if values.is_empty()
                    && matches!(self.kind(destination.ty)?, TypeKind::Vec { .. }) =>
            {
                Value::Sequence(Vec::new())
            }
            "box_new" if values.len() == 1 => {
                let TypeKind::Box { inner } = self.kind(destination.ty)? else {
                    return Err(self.invalid("box_new result is not Box"));
                };
                if *inner != values[0].0 {
                    return Err(self.invalid("Box element type differs"));
                }
                let (ty, value) = values.remove(0);
                Value::Box(self.allocate(ty, Some(value), false)?)
            }
            "vec_len" | "str_len" if values.len() == 1 => {
                let access = reference(0, self)?;
                self.access(&access, false, true)?;
                Value::Bits(self.sequence(&access)?.len() as u64)
            }
            "vec_push" if values.len() == 2 => {
                let access = reference(0, self)?;
                if !access.writable {
                    return Err(self.error(
                        MachineError::ConflictingAccess,
                        "Vec push requires an exclusive reference",
                    ));
                }
                self.access(&access, true, true)?;
                let TypeKind::Vec { inner } = self.kind(access.ty)? else {
                    return Err(self.invalid("push target is not Vec"));
                };
                if *inner != values[1].0 {
                    return Err(self.invalid("pushed element type differs"));
                }
                let (ty, value) = values.remove(1);
                let child = self.allocate(ty, Some(value), false)?;
                self.cells[child.index].parent = Some(access.cell);
                let Some(Value::Sequence(elements)) = &mut self.cells[access.cell.index].value
                else {
                    return Err(self.invalid("Vec has no buffer"));
                };
                elements.push(child);
                Value::Unit
            }
            "vec_pop" if values.len() == 1 => {
                let access = reference(0, self)?;
                if !access.writable {
                    return Err(self.error(
                        MachineError::ConflictingAccess,
                        "Vec pop requires an exclusive reference",
                    ));
                }
                self.access(&access, true, true)?;
                if !matches!(self.kind(access.ty)?, TypeKind::Vec { .. }) {
                    return Err(self.invalid("pop target is not Vec"));
                }
                let Some(Value::Sequence(elements)) = &mut self.cells[access.cell.index].value
                else {
                    return Err(self.invalid("Vec has no buffer"));
                };
                match elements.pop() {
                    None => Value::Aggregate {
                        variant: Some(0),
                        fields: Vec::new(),
                    },
                    Some(child) => {
                        self.cells[child.index].parent = None;
                        Value::Aggregate {
                            variant: Some(1),
                            fields: vec![child],
                        }
                    }
                }
            }
            "replace" if values.len() == 2 => {
                let access = reference(0, self)?;
                if !access.writable || access.ty != values[1].0 || access.ty != destination.ty {
                    return Err(self.invalid("invalid replacement signature"));
                }
                let old = self.take(&access)?;
                let (_, replacement) = values.remove(1);
                self.put(access.cell, replacement)?;
                old
            }
            "clone" if values.len() == 1 => {
                let access = reference(0, self)?;
                if access.ty != destination.ty {
                    return Err(self.invalid("clone result type differs"));
                }
                self.copy_value(&access)?
            }
            "str_as_bytes" if values.len() == 1 => {
                let access = reference(0, self)?;
                if !matches!(self.kind(access.ty)?, TypeKind::Str) {
                    return Err(self.invalid("text projection requires str"));
                }
                let TypeKind::Ref {
                    inner,
                    mutable: false,
                } = self.kind(destination.ty)?
                else {
                    return Err(self.invalid("text bytes result must be a shared slice"));
                };
                if !matches!(
                    self.kind(*inner)?,
                    TypeKind::Slice {
                        inner: k::U8,
                        mutable: false
                    }
                ) {
                    return Err(self.invalid("text bytes result is not a byte slice"));
                }
                let inner = *inner;
                let (_, Value::Reference(mut reference)) = values.remove(0) else {
                    unreachable!()
                };
                reference.referent = inner;
                Value::Reference(reference)
            }
            "str_from_utf8" if values.len() == 1 => {
                let access = reference(0, self)?;
                let bytes = self.bytes(&access)?;
                let TypeKind::Enum { variants, .. } = self.kind(destination.ty)? else {
                    return Err(self.invalid("UTF-8 result must be an enum"));
                };
                if variants.len() != 2
                    || variants[0].fields.len() != 1
                    || variants[1].fields.len() != 1
                {
                    return Err(self.invalid("invalid UTF-8 result shape"));
                }
                let success = variants[0].fields[0].ty;
                let failure = variants[1].fields[0].ty;
                if std::str::from_utf8(&bytes).is_ok() {
                    let TypeKind::Ref {
                        inner,
                        mutable: false,
                    } = self.kind(success)?
                    else {
                        return Err(self.invalid("UTF-8 success is not a shared reference"));
                    };
                    if !matches!(self.kind(*inner)?, TypeKind::Str) {
                        return Err(self.invalid("UTF-8 success is not str"));
                    }
                    let inner = *inner;
                    let (_, Value::Reference(mut reference)) = values.remove(0) else {
                        unreachable!()
                    };
                    reference.referent = inner;
                    let child = self.allocate(success, Some(Value::Reference(reference)), false)?;
                    Value::Aggregate {
                        variant: Some(0),
                        fields: vec![child],
                    }
                } else {
                    let error = Value::Aggregate {
                        variant: None,
                        fields: Vec::new(),
                    };
                    self.validate_value(failure, &error)?;
                    let child = self.allocate(failure, Some(error), false)?;
                    Value::Aggregate {
                        variant: Some(1),
                        fields: vec![child],
                    }
                }
            }
            "world_stdout" if values.len() == 1 => {
                let access = reference(0, self)?;
                if !access.writable
                    || !matches!(self.kind(access.ty)?, TypeKind::World)
                    || !matches!(self.value(access.cell)?, Value::World)
                {
                    return Err(self.invalid("stdout authority requires a mutable World"));
                }
                self.access(&access, true, true)?;
                Value::Stdout
            }
            "stdout_write" if values.len() == 2 => {
                let writer = reference(0, self)?;
                if !writer.writable
                    || !matches!(self.kind(writer.ty)?, TypeKind::Stdout)
                    || !matches!(self.value(writer.cell)?, Value::Stdout)
                {
                    return Err(self.invalid("stdout write requires derived writer authority"));
                }
                self.access(&writer, true, true)?;
                let data = reference(1, self)?;
                if !matches!(
                    self.kind(data.ty)?,
                    TypeKind::Slice {
                        inner: k::U8,
                        mutable: false
                    }
                ) {
                    return Err(self.invalid("stdout write requires bytes"));
                }
                let bytes = self.bytes(&data)?;
                self.host
                    .write_stdout(&bytes)
                    .map_err(|message| self.trap(7, message))?;
                Value::Unit
            }
            _ => match self.io_intrinsic(destination.ty, name, &values)? {
                Some(value) => value,
                None => {
                    return Err(self.error(
                        MachineError::Unsupported,
                        format!("intrinsic {name} is not implemented by the K machine"),
                    ));
                }
            },
        };
        for (_, value) in values {
            self.discard_scalar(value)?;
        }
        self.put_local(frame, dst, result)
    }
    fn step(&mut self, mut frame: Frame) -> Result<(), RunError> {
        let declaration = &self.program.functions[frame.function];
        let instruction = declaration
            .body
            .get(frame.pc)
            .ok_or_else(|| self.invalid("function reaches its end without returning"))?
            .clone();
        self.site = Site {
            function: frame.function,
            file: declaration.file,
            line: instruction.line,
        };
        let pc = frame.pc;
        frame.pc += 1;
        match instruction.op {
            Op::Int { dst, value } => self.put_local(&frame, dst, Value::Bits(value as u64))?,
            Op::Float { dst, bits } => {
                if !matches!(self.kind(self.local(&frame, dst)?.ty)?, TypeKind::F64) {
                    return Err(self.invalid("float destination is not f64"));
                }
                self.put_local(&frame, dst, Value::Bits(bits))?;
            }
            Op::Unit { dst } => self.put_local(&frame, dst, Value::Unit)?,
            Op::Text { dst, bytes } => {
                let TypeKind::Ref {
                    inner,
                    mutable: false,
                } = self.kind(self.local(&frame, dst)?.ty)?
                else {
                    return Err(self.invalid("text literal needs a shared str reference"));
                };
                let inner = *inner;
                match self.kind(inner)? {
                    TypeKind::Str => {
                        if std::str::from_utf8(&bytes).is_err() {
                            return Err(self.invalid("text literal is not UTF-8"));
                        }
                    }
                    TypeKind::Array { inner: k::U8, len } if *len == bytes.len() => {}
                    _ => {
                        return Err(
                            self.invalid("byte constant requires str or an exact-length u8 array")
                        );
                    }
                }
                let key = (inner, bytes.clone());
                let cell = match self.texts.get(&key) {
                    Some(cell) => *cell,
                    None => {
                        let mut elements = Vec::new();
                        for byte in &bytes {
                            elements.push(self.allocate(
                                k::U8,
                                Some(Value::Bits(u64::from(*byte))),
                                true,
                            )?);
                        }
                        let cell = self.allocate(inner, Some(Value::Sequence(elements)), true)?;
                        self.texts.insert(key, cell);
                        cell
                    }
                };
                let access = Access {
                    extent: None,
                    cell,
                    ty: inner,
                    permissions: Vec::new(),
                    writable: false,
                };
                let reference = self.borrow(&access, inner, false)?;
                self.put_local(&frame, dst, reference)?;
            }
            Op::Read { dst, place, moving } => {
                let source = self.resolve(&frame, &place)?;
                if source.ty != self.local(&frame, dst)?.ty {
                    return Err(self.invalid("read type differs"));
                }
                let value = if moving {
                    self.take(&source)?
                } else {
                    self.copy_value(&source)?
                };
                self.put_local(&frame, dst, value)?;
            }
            Op::Borrow {
                dst,
                ref place,
                mutable,
            }
            | Op::SliceBorrow {
                dst,
                ref place,
                mutable,
            } => {
                let source = self.resolve(&frame, place)?;
                let TypeKind::Ref {
                    inner,
                    mutable: expected,
                } = self.kind(self.local(&frame, dst)?.ty)?
                else {
                    return Err(self.invalid("borrow destination is not a reference"));
                };
                let inner = *inner;
                if *expected != mutable {
                    return Err(self.invalid("borrow mutability differs"));
                }
                if matches!(instruction.op, Op::SliceBorrow { .. }) {
                    let element = match self.kind(source.ty)? {
                        TypeKind::Vec { inner } | TypeKind::Array { inner, .. } => *inner,
                        _ => return Err(self.invalid("slice borrow requires Vec or array")),
                    };
                    if !matches!(self.kind(inner)?, TypeKind::Slice { inner, .. } if *inner == element)
                    {
                        return Err(self.invalid("slice element differs"));
                    }
                } else if inner != source.ty {
                    return Err(self.invalid("borrow referent type differs"));
                }
                let reference = self.borrow(&source, inner, mutable)?;
                self.put_local(&frame, dst, reference)?;
            }
            Op::ViewRange {
                dst,
                ref place,
                start,
                end,
                inclusive,
                mutable,
            } => {
                let mut source = self.resolve(&frame, place)?;
                let TypeKind::Ref {
                    inner,
                    mutable: expected,
                } = self.kind(self.local(&frame, dst)?.ty)?
                else {
                    return Err(self.invalid("range result must be a reference"));
                };
                let inner = *inner;
                if *expected != mutable {
                    return Err(self.invalid("range mutability differs"));
                }
                let text = matches!(self.kind(source.ty)?, TypeKind::Str);
                match self.kind(source.ty)? {
                    TypeKind::Vec { inner: item }
                    | TypeKind::Array { inner: item, .. }
                    | TypeKind::Slice { inner: item, .. } => {
                        if !matches!(self.kind(inner)?, TypeKind::Slice { inner: result, .. } if item == result)
                        {
                            return Err(self.invalid("range element type differs"));
                        }
                    }
                    TypeKind::Str if !mutable && matches!(self.kind(inner)?, TypeKind::Str) => {}
                    _ => return Err(self.invalid("range source or result type differs")),
                }
                if self.local(&frame, start)?.ty != k::U64 || self.local(&frame, end)?.ty != k::U64
                {
                    return Err(self.invalid("range bounds must be usize"));
                }
                self.access(&source, mutable, true)?;
                let (_, Value::Bits(start)) = self.take_local(&frame, start)? else {
                    return Err(self.invalid("range start is not scalar"));
                };
                let (_, Value::Bits(mut end)) = self.take_local(&frame, end)? else {
                    return Err(self.invalid("range end is not scalar"));
                };
                let elements = self.sequence(&source)?;
                let count = elements.len() as u64;
                if inclusive {
                    if end >= count {
                        return Err(self.trap(3, "range out of bounds"));
                    }
                    end += 1;
                }
                if start > end || end > count {
                    return Err(self.trap(3, "range out of bounds"));
                }
                if text {
                    for endpoint in [start, end] {
                        if endpoint != 0 && endpoint < count {
                            let cell = elements[endpoint as usize];
                            let Value::Bits(byte) = self.value(cell)? else {
                                return Err(self.invalid("text boundary is not a byte"));
                            };
                            if byte & 0xc0 == 0x80 {
                                return Err(self.trap(3, "range endpoint is not a UTF-8 boundary"));
                            }
                        }
                    }
                }
                source.extent = Some(SequenceExtent {
                    start: source.extent.map_or(0, |extent| extent.start) + start as usize,
                    count: (end - start) as usize,
                });
                let reference = self.borrow(&source, inner, mutable)?;
                self.put_local(&frame, dst, reference)?;
            }
            Op::Assign { place, src } => {
                let destination = self.resolve(&frame, &place)?;
                if self.cell(destination.cell)?.value.is_some() && !destination.writable {
                    return Err(self.error(
                        MachineError::ConflictingAccess,
                        "assignment through an immutable place",
                    ));
                }
                self.access(&destination, true, false)?;
                let (ty, value) = self.take_local(&frame, src)?;
                if ty != destination.ty {
                    return Err(self.invalid("assignment type differs"));
                }
                if let Some(old) = self.cells[destination.cell.index].value.take() {
                    self.discard_scalar(old)?;
                }
                self.put(destination.cell, value)?;
            }
            Op::Drop { ref place } | Op::DropIf { ref place } => {
                let optional = matches!(instruction.op, Op::DropIf { .. });
                let access = self.resolve(&frame, place)?;
                let site = self.site;
                self.push_work(Work::Frame(frame))?;
                self.push_work(Work::Drop(DropWork {
                    access,
                    custom: true,
                    release: false,
                    optional,
                    site,
                }))?;
                return Ok(());
            }
            Op::Unary { dst, op, src } => {
                let (ty, value) = self.take_local(&frame, src)?;
                if ty != self.local(&frame, dst)?.ty {
                    return Err(self.invalid("unary result type differs"));
                }
                let Value::Bits(value) = value else {
                    return Err(self.invalid("unary operand is not scalar"));
                };
                let value = self.number(scalar::unary(self.kind(ty)?, &op, value))?;
                self.put_local(&frame, dst, Value::Bits(value))?;
            }
            Op::Binary {
                dst,
                op,
                left,
                right,
            } => {
                let (left_ty, left) = self.take_local(&frame, left)?;
                let (right_ty, right) = self.take_local(&frame, right)?;
                if left_ty != right_ty {
                    return Err(self.invalid("binary operand types differ"));
                }
                let comparison = ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str());
                if self.local(&frame, dst)?.ty != if comparison { k::BOOL } else { left_ty } {
                    return Err(self.invalid("binary result type differs"));
                }
                let (Value::Bits(left), Value::Bits(right)) = (left, right) else {
                    return Err(self.invalid("binary operands are not scalar"));
                };
                let value = self.number(scalar::binary(self.kind(left_ty)?, &op, left, right))?;
                self.put_local(&frame, dst, Value::Bits(value))?;
            }
            Op::Call {
                dst,
                function,
                args,
            } => {
                if self
                    .program
                    .destructors
                    .iter()
                    .any(|entry| entry.1 == function)
                {
                    return Err(self.invalid("destructor called without drop"));
                }
                let destination = self.local(&frame, dst)?;
                let declaration = self
                    .program
                    .functions
                    .get(function)
                    .ok_or_else(|| self.invalid("unknown call target"))?;
                if declaration.result != destination.ty {
                    return Err(self.invalid("call result type differs"));
                }
                let mut values = Vec::new();
                for arg in args {
                    values.push(self.take_local(&frame, arg)?);
                }
                match self.runtime_call(function, &values)? {
                    Some(crate::runtime_memory::Action::Value(value)) => {
                        self.put(destination.cell, Value::Bits(value))?;
                    }
                    Some(crate::runtime_memory::Action::Call { function, argument }) => {
                        let target = self
                            .program
                            .functions
                            .get(function)
                            .ok_or_else(|| self.invalid("unknown runtime cleanup target"))?;
                        if target.params != 1
                            || target.result != destination.ty
                            || !matches!(
                                self.kind(target.locals[0].ty)?,
                                TypeKind::Int {
                                    bits: 64,
                                    signed: false
                                }
                            )
                        {
                            return Err(self.invalid("runtime cleanup signature differs"));
                        }
                        let argument = (target.locals[0].ty, Value::Bits(argument));
                        self.push_work(Work::Frame(frame))?;
                        self.push_frame(
                            function,
                            vec![argument],
                            ReturnTo::Cell(destination.cell),
                        )?;
                        return Ok(());
                    }
                    None => {
                        self.push_work(Work::Frame(frame))?;
                        self.push_frame(function, values, ReturnTo::Cell(destination.cell))?;
                        return Ok(());
                    }
                }
            }
            Op::Struct { dst, ref fields }
            | Op::Variant {
                dst, ref fields, ..
            } => {
                let ty = self.local(&frame, dst)?.ty;
                let variant = match instruction.op {
                    Op::Variant { variant, .. } => Some(variant),
                    _ => None,
                };
                let expected = match (variant, self.kind(ty)?) {
                    (None, TypeKind::Struct { fields, .. }) => {
                        fields.iter().map(|field| field.ty).collect::<Vec<_>>()
                    }
                    (Some(variant), TypeKind::Enum { variants, .. }) => variants
                        .get(variant)
                        .ok_or_else(|| self.invalid("unknown constructor variant"))?
                        .fields
                        .iter()
                        .map(|field| field.ty)
                        .collect(),
                    _ => return Err(self.invalid("aggregate constructor has wrong type")),
                };
                let fields = self.fields(&frame, fields, &expected)?;
                self.put_local(&frame, dst, Value::Aggregate { variant, fields })?;
            }
            Op::Array { dst, elements } => {
                let TypeKind::Array { inner, len } = self.kind(self.local(&frame, dst)?.ty)? else {
                    return Err(self.invalid("array constructor has wrong type"));
                };
                let expected = vec![*inner; *len];
                let fields = self.fields(&frame, &elements, &expected)?;
                self.put_local(&frame, dst, Value::Sequence(fields))?;
            }
            Op::Discriminant { dst, .. } | Op::TagTest { dst, .. } => {
                let mut place = match &instruction.op {
                    Op::Discriminant { src, .. } => Place::local(*src),
                    Op::TagTest { place, .. } => place.clone(),
                    _ => unreachable!(),
                };
                let initial = self.resolve(&frame, &place)?;
                if matches!(
                    self.kind(self.cell(initial.cell)?.ty)?,
                    TypeKind::Ref { .. }
                ) {
                    place.projections.push(Projection::Deref);
                }
                let access = self.resolve(&frame, &place)?;
                self.access(&access, false, true)?;
                let Value::Aggregate {
                    variant: Some(variant),
                    ..
                } = self.value(access.cell)?
                else {
                    return Err(self.invalid("tag read requires an enum"));
                };
                let value = match instruction.op {
                    Op::TagTest {
                        variant: expected, ..
                    } => u64::from(*variant == expected),
                    _ => *variant as u64,
                };
                self.put_local(&frame, dst, Value::Bits(value))?;
            }
            Op::Intrinsic { dst, name, args } => self.intrinsic(&frame, dst, &name, &args)?,
            Op::If { condition } => {
                let (ty, value) = self.take_local(&frame, condition)?;
                if ty != k::BOOL {
                    return Err(self.invalid("if requires bool"));
                }
                let Value::Bits(condition) = value else {
                    return Err(self.invalid("if condition is not scalar"));
                };
                if condition == 0 {
                    frame.pc = self.flow[frame.function].as_ref().unwrap().jump[pc].unwrap();
                }
            }
            Op::Else | Op::Break | Op::Continue | Op::BreakTo { .. } | Op::ContinueTo { .. } => {
                frame.pc = self.flow[frame.function].as_ref().unwrap().jump[pc]
                    .ok_or_else(|| self.invalid("control transfer has no target"))?;
            }
            Op::End => {
                if let Some(target) = self.flow[frame.function].as_ref().unwrap().jump[pc] {
                    frame.pc = target;
                }
            }
            Op::Loop => {}
            Op::Return { src } => {
                let (ty, value) = self.take_local(&frame, src)?;
                self.finish_frame(frame, ty, value)?;
                return Ok(());
            }
            Op::Trap { message } => {
                let mut messages = Vec::new();
                for function in &self.program.functions {
                    for instruction in &function.body {
                        if let Op::Trap { message } = &instruction.op {
                            if !messages.contains(message) {
                                messages.push(message.clone());
                            }
                        }
                    }
                }
                let code = 16
                    + messages
                        .iter()
                        .position(|candidate| *candidate == message)
                        .unwrap() as u64;
                return Err(self.trap(code, message));
            }
        }
        self.push_work(Work::Frame(frame))?;
        Ok(())
    }
}

/// Run raw K. The sole-World host entry receives fresh authority implicitly;
/// other entries accept scalar bit patterns only. No caller-supplied integer can
/// populate a reference, owner, World or derived capability.
pub fn run<H: Host>(
    program: &Program,
    entry: usize,
    args: &[u64],
    host: &mut H,
    resources: Resources,
) -> Result<RunResult, RunError> {
    run_inner(program, entry, args, host, None, resources)
}

/// Exercise an explicitly sealed source runtime in a separate raw byte arena.
/// Ordinary run never substitutes function bodies, even for identical names.
pub fn run_runtime<H: Host>(
    runtime: &crate::runtime::Runtime,
    entry: usize,
    args: &[u64],
    memory: &mut crate::runtime_memory::Memory,
    host: &mut H,
    resources: Resources,
) -> Result<RunResult, RunError> {
    run_inner(
        runtime.checked().program(),
        entry,
        args,
        host,
        Some(RuntimeState { runtime, memory }),
        resources,
    )
}
fn run_inner<'a, H: Host>(
    program: &'a Program,
    entry: usize,
    args: &[u64],
    host: &'a mut H,
    runtime: Option<RuntimeState<'a>>,
    resources: Resources,
) -> Result<RunResult, RunError> {
    let site = Site {
        function: entry,
        file: program
            .functions
            .get(entry)
            .map_or(0, |function| function.file),
        line: 1,
    };
    let mut machine = Machine {
        program,
        host,
        runtime,
        cells: Vec::new(),
        free: Vec::new(),
        live_cells: 0,
        peak_cells: 0,
        loans: BTreeMap::new(),
        next_tag: 0,
        free_tags: Vec::new(),
        work: Vec::new(),
        frames: 0,
        peak_frames: 0,
        resources,
        stack_bytes: 0,
        peak_stack_bytes: 0,
        flow: (0..program.functions.len()).map(|_| None).collect(),
        texts: BTreeMap::new(),
        site,
        result: None,
    };
    let function = program
        .functions
        .get(entry)
        .ok_or_else(|| machine.invalid("unknown host entry"))?;
    if function.params > function.locals.len() {
        return Err(machine.invalid("entry parameter declarations are missing"));
    }
    let mut values = Vec::new();
    if function.params == 1 && matches!(machine.kind(function.locals[0].ty)?, TypeKind::World) {
        if !args.is_empty() {
            return Err(machine.invalid("World entry takes no raw arguments"));
        }
        values.push((function.locals[0].ty, Value::World));
    } else {
        if function.params != args.len() {
            return Err(machine.invalid("entry argument count differs"));
        }
        for (local, argument) in function.locals.iter().zip(args) {
            if !matches!(
                machine.kind(local.ty)?,
                TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64
            ) {
                return Err(machine.invalid("raw host arguments may only be scalar values"));
            }
            values.push((local.ty, Value::Bits(*argument)));
        }
    }
    machine.push_frame(entry, values, ReturnTo::Entry)?;
    while let Some(work) = machine.work.pop() {
        match work {
            Work::Frame(frame) => machine.step(frame)?,
            Work::Drop(work) => machine.drop_work(work)?,
        }
    }
    let value = machine
        .result
        .take()
        .ok_or_else(|| machine.invalid("machine finished without an entry result"))?;
    Ok(RunResult {
        value,
        peak_frames: machine.peak_frames,
        peak_cells: machine.peak_cells,
        peak_stack_bytes: machine.peak_stack_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_reference_never_aliases_a_reused_slot_or_loan_number() {
        fn checked<T>(value: Result<T, RunError>) -> T {
            value.unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message))
        }
        let program = Program {
            types: k::primitives(),
            functions: Vec::new(),
            destructors: Vec::new(),
        };
        let mut host = Vec::new();
        let mut machine = Machine {
            program: &program,
            host: &mut host,
            runtime: None,
            cells: Vec::new(),
            free: Vec::new(),
            live_cells: 0,
            peak_cells: 0,
            loans: BTreeMap::new(),
            next_tag: 0,
            free_tags: Vec::new(),
            work: Vec::new(),
            frames: 0,
            peak_frames: 0,
            resources: Resources::unlimited(),
            stack_bytes: 0,
            peak_stack_bytes: 0,
            flow: Vec::new(),
            texts: BTreeMap::new(),
            site: Site {
                function: 0,
                file: 0,
                line: 1,
            },
            result: None,
        };
        let old = checked(machine.allocate(k::I64, Some(Value::Bits(1)), false));
        let old_access = Access {
            extent: None,
            cell: old,
            ty: k::I64,
            permissions: Vec::new(),
            writable: true,
        };
        let Value::Reference(stale) = checked(machine.borrow(&old_access, k::I64, false)) else {
            panic!("borrow did not return a reference")
        };
        // Force the lifetime violation directly: ordinary K execution reports
        // the conflicting destruction before this allocator boundary is reached.
        machine.cells[old.index].value = None;
        checked(machine.release(old));
        let replacement = checked(machine.allocate(k::I64, Some(Value::Bits(2)), false));
        assert_eq!(old.index, replacement.index);
        let error = machine
            .reference_access(&stale)
            .err()
            .expect("stale slot reference accepted");
        assert!(error.kind == ErrorKind::Machine(MachineError::DanglingReference));
        checked(machine.release_reference(&stale));
        let access = Access {
            extent: None,
            cell: replacement,
            ty: k::I64,
            permissions: Vec::new(),
            writable: true,
        };
        let Value::Reference(current) = checked(machine.borrow(&access, k::I64, false)) else {
            panic!("borrow did not return a reference")
        };
        assert_eq!(stale.tag, current.tag);
        let error = machine
            .reference_access(&stale)
            .err()
            .expect("stale tag reference accepted");
        assert!(error.kind == ErrorKind::Machine(MachineError::DanglingReference));
        checked(machine.reference_access(&current));
    }
}
