//! Independent execution of raw IR over an independently owned heap and explicit work
//! stack. This model neither invokes the checker nor uses native lowering.
mod io;
mod scalar;
use crate::ir::{self, Op, Place, Program, Projection, TypeKind};
use std::collections::{BTreeMap, BTreeSet};

pub trait Host {
    /// Consume bytes before returning. The borrowed buffer cannot be retained.
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn argument_count(&self) -> usize;
    fn argument(&self, index: usize) -> Option<Vec<u8>>;
    fn read_stdin(&mut self, bytes: &mut [u8]) -> Result<usize, i64>;
    fn write_stderr(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn open_directory(&mut self, path: &[u8]) -> Result<u64, i64>;
    fn open_file(&mut self, directory: u64, path: &[u8], create: bool) -> Result<u64, i64>;
    fn read_file(&mut self, handle: u64, bytes: &mut [u8]) -> Result<usize, i64>;
    fn write_file(&mut self, handle: u64, bytes: &[u8]) -> Result<usize, i64>;
    fn close_handle(&mut self, handle: u64);
    fn set_exit_status(&mut self, status: u8);
    fn world_file(&mut self, kind: u8) -> Result<u64, i64>;
    fn duplicate_file(&mut self, handle: u64) -> Result<u64, i64>;
    fn readonly_file(&mut self, handle: u64) -> Result<u64, i64>;
    fn open_subdirectory(&mut self, directory: u64, path: &[u8]) -> Result<u64, i64>;
    fn read_directory(&mut self, directory: u64, bytes: &mut [u8]) -> Result<usize, i64>;
    fn rewind_handle(&mut self, handle: u64) -> Result<(), i64>;
    fn create_directory(&mut self, directory: u64, name: &[u8]) -> Result<(), i64>;
    fn remove_entry(&mut self, directory: u64, name: &[u8], is_directory: bool) -> Result<(), i64>;
    fn rename_entry(
        &mut self,
        from: u64,
        old_name: &[u8],
        to: u64,
        new_name: &[u8],
    ) -> Result<(), i64>;
    fn file_executable(&mut self, file: u64) -> Result<(), i64>;
    fn monotonic_time(&mut self, bytes: &mut [u8]) -> Result<(), i64>;
    fn spawn_process(
        &mut self,
        executable: u64,
        arguments: &[u8],
        stdin: u64,
        stdout: u64,
        stderr: u64,
        result: u64,
    ) -> Result<u64, i64>;
    fn wait_process(&mut self, child: u64, nonblocking: bool) -> Result<Option<i64>, i64>;
    fn terminate_process(&mut self, child: u64) -> Result<(), i64>;
    fn drop_process(&mut self, child: u64);
}
impl Host for Vec<u8> {
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.extend_from_slice(bytes);
        Ok(())
    }
    fn argument_count(&self) -> usize {
        0
    }
    fn argument(&self, _index: usize) -> Option<Vec<u8>> {
        None
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
    fn duplicate_file(&mut self, _handle: u64) -> Result<u64, i64> {
        Err(38)
    }
    fn readonly_file(&mut self, _handle: u64) -> Result<u64, i64> {
        Err(38)
    }
    fn world_file(&mut self, _kind: u8) -> Result<u64, i64> {
        Err(38)
    }
    fn open_subdirectory(&mut self, _directory: u64, _path: &[u8]) -> Result<u64, i64> {
        Err(38)
    }
    fn read_directory(&mut self, _directory: u64, _bytes: &mut [u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn rewind_handle(&mut self, _handle: u64) -> Result<(), i64> {
        Err(38)
    }
    fn create_directory(&mut self, _directory: u64, _name: &[u8]) -> Result<(), i64> {
        Err(38)
    }
    fn remove_entry(
        &mut self,
        _directory: u64,
        _name: &[u8],
        _is_directory: bool,
    ) -> Result<(), i64> {
        Err(38)
    }
    fn rename_entry(
        &mut self,
        _from: u64,
        _old_name: &[u8],
        _to: u64,
        _new_name: &[u8],
    ) -> Result<(), i64> {
        Err(38)
    }
    fn file_executable(&mut self, _file: u64) -> Result<(), i64> {
        Err(38)
    }
    fn monotonic_time(&mut self, _bytes: &mut [u8]) -> Result<(), i64> {
        Err(38)
    }
    fn spawn_process(
        &mut self,
        _executable: u64,
        _arguments: &[u8],
        _stdin: u64,
        _stdout: u64,
        _stderr: u64,
        _result: u64,
    ) -> Result<u64, i64> {
        Err(38)
    }
    fn wait_process(&mut self, _child: u64, _nonblocking: bool) -> Result<Option<i64>, i64> {
        Err(38)
    }
    fn terminate_process(&mut self, _child: u64) -> Result<(), i64> {
        Err(38)
    }
    fn drop_process(&mut self, _child: u64) {}
}

#[derive(Clone, PartialEq, Eq)]
pub enum InterpreterError {
    InvalidIr,
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
    Interpreter(InterpreterError),
}
#[derive(Clone, PartialEq, Eq)]
pub struct RunError {
    pub kind: ErrorKind,
    pub function: usize,
    pub file: usize,
    pub line: usize,
    pub message: String,
}
impl RunError {
    pub fn code(&self) -> String {
        match &self.kind {
            ErrorKind::Trap(code) => format!("R{code:04}"),
            ErrorKind::Interpreter(kind) => match kind {
                InterpreterError::InvalidIr => "M_INVALID_IR",
                InterpreterError::Unsupported => "M_UNSUPPORTED",
                InterpreterError::UseAfterMove => "M_USE_AFTER_MOVE",
                InterpreterError::DoubleDrop => "M_DOUBLE_DROP",
                InterpreterError::DanglingReference => "M_DANGLING_REFERENCE",
                InterpreterError::ConflictingAccess => "M_CONFLICTING_ACCESS",
                InterpreterError::InactiveVariant => "M_INACTIVE_VARIANT",
                InterpreterError::MissingDrop => "M_MISSING_DROP",
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
#[derive(Clone, PartialEq, Eq)]
pub struct RunResult {
    pub value: Scalar,
    pub peak_frames: usize,
    pub peak_cells: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// Owner and frame handles keep slots allocated. References also carry the
// occupant generation, so a reused cell cannot revive a stale reference.
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
    identity: u64,
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
    Process,
    Clock,
    Child(u64),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Empty {
    Never,
    Moved,
    Dropped,
}
// Recycling a cell advances its occupant generation. Loans retain the old word
// without retaining cells or allocating shared host objects.
struct Cell {
    allocated: bool,
    identity: u64,
    ty: usize,
    value: Option<Value>,
    empty: Empty,
    parent: Option<Location>,
    immutable: bool,
}
struct Loan {
    extent: Option<SequenceExtent>,
    target: Location,
    identity: u64,
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
    memory: &'a mut crate::runtime_memory::Memory,
}
struct Interpreter<'a, H: Host> {
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
    flow: Vec<Option<Flow>>,
    texts: BTreeMap<(usize, Vec<u8>), Location>,
    site: Site,
    result: Option<Scalar>,
}

impl<H: Host> Interpreter<'_, H> {
    fn error(&self, kind: InterpreterError, message: &str) -> RunError {
        RunError {
            kind: ErrorKind::Interpreter(kind),
            function: self.site.function,
            file: self.site.file,
            line: self.site.line,
            message: message.to_string(),
        }
    }
    fn invalid(&self, message: &str) -> RunError {
        self.error(InterpreterError::InvalidIr, message)
    }
    fn trap(&self, code: u64, message: &str) -> RunError {
        RunError {
            kind: ErrorKind::Trap(code),
            function: self.site.function,
            file: self.site.file,
            line: self.site.line,
            message: message.to_string(),
        }
    }
    fn number(&self, result: Result<u64, scalar::NumberError>) -> Result<u64, RunError> {
        result.map_err(|error| match error {
            scalar::NumberError::Trap(1) => self.trap(1, "integer overflow"),
            scalar::NumberError::Trap(code) => self.trap(code as u64, "division by zero"),
            scalar::NumberError::Invalid(message) => self.invalid(&message),
        })
    }
    fn kind(&self, ty: usize) -> Result<&TypeKind, RunError> {
        let kind = &self
            .program
            .types
            .get(ty)
            .ok_or_else(|| self.invalid("unknown type"))?
            .kind;
        if matches!(kind, TypeKind::Int {bits,..} if !matches!(*bits, 8 | 16 | 32 | 64)) {
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
                    InterpreterError::DanglingReference,
                    "location has been released",
                )
            })
    }
    fn same_occupant(&self, location: Location, identity: u64) -> bool {
        self.cells
            .get(location.index)
            .is_some_and(|cell| cell.allocated && cell.identity == identity)
    }
    fn value(&self, location: Location) -> Result<&Value, RunError> {
        self.cell(location)?.value.as_ref().ok_or_else(|| {
            self.error(
                InterpreterError::UseAfterMove,
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
                let identity = self.cells[index]
                    .identity
                    .checked_add(1)
                    .ok_or_else(|| self.trap(6, "cell occupant identity is not representable"))?;
                self.cells[index] = Cell {
                    allocated: true,
                    identity,
                    ty,
                    value,
                    empty: Empty::Never,
                    parent: None,
                    immutable,
                };
                index
            }
            None => {
                let index = self.cells.len();
                self.cells.push(Cell {
                    allocated: true,
                    identity: 0,
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
    fn children(value: &Value) -> Vec<Location> {
        match value {
            Value::Aggregate { fields, .. } | Value::Sequence(fields) => fields.clone(),
            Value::Box(child) => vec![*child],
            _ => Vec::new(),
        }
    }
    fn reparent(&mut self, location: Location) -> Result<(), RunError> {
        let children = match &self.cell(location)?.value {
            Some(value) => Self::children(value),
            None => Vec::new(),
        };
        for child in children {
            self.cell(child)?;
            self.cells[child.index].parent = Some(location);
        }
        Ok(())
    }
    fn release(&mut self, location: Location) -> Result<(), RunError> {
        if self.cell(location)?.value.is_some() {
            return Err(self.error(
                InterpreterError::MissingDrop,
                "releasing initialized storage",
            ));
        }
        self.cells[location.index].allocated = false;
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
                InterpreterError::ConflictingAccess,
                "write to immutable static storage",
            ));
        }
        let permitted = self.permissions(&access.permissions);
        for (tag, loan) in &self.loans {
            if !permitted.contains(tag)
                && self.same_occupant(loan.target, loan.identity)
                && self.overlaps(access.cell, loan.target)
                && (write || loan.mutable)
            {
                return Err(self.error(
                    InterpreterError::ConflictingAccess,
                    &format!(
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
            self.error(
                InterpreterError::DanglingReference,
                "reference tag has expired",
            )
        })?;
        if loan.identity != reference.identity || loan.target != reference.target {
            return Err(self.error(
                InterpreterError::DanglingReference,
                "loan tag belongs to another storage identity",
            ));
        }
        if loan.target != reference.target
            || loan.mutable != reference.mutable
            || loan.extent != reference.extent
        {
            return Err(self.invalid("reference tag differs from its borrow"));
        }
        if !self.same_occupant(reference.target, reference.identity) {
            return Err(self.error(
                InterpreterError::DanglingReference,
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
                InterpreterError::ConflictingAccess,
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
        let mut parents = Vec::new();
        for permission in self.permissions(&access.permissions) {
            parents.push(permission);
        }
        for parent in &parents {
            self.loans.get_mut(parent).unwrap().children += 1;
        }
        let identity = self.cell(access.cell)?.identity;
        self.loans.insert(
            tag,
            Loan {
                extent,
                target: access.cell,
                identity,
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
    fn loan_finished(&self, tag: usize) -> bool {
        match self.loans.get(&tag) {
            Some(loan) => loan.holders == 0 && loan.children == 0,
            None => false,
        }
    }
    fn release_reference(&mut self, reference: &Reference) -> Result<(), RunError> {
        if !self.loans.get(&reference.tag).is_some_and(|loan| {
            loan.identity == reference.identity && loan.target == reference.target
        }) {
            return Err(self.error(
                InterpreterError::DanglingReference,
                "reference loan identity has expired",
            ));
        }
        let holders = self.loans.get(&reference.tag).unwrap().holders;
        if holders == 0 {
            return Err(self.error(
                InterpreterError::DoubleDrop,
                "reference holder already dropped",
            ));
        }
        self.loans.get_mut(&reference.tag).unwrap().holders -= 1;
        let mut pending = vec![reference.tag];
        while let Some(tag) = pending.pop() {
            if self.loan_finished(tag) {
                let loan = self.loans.remove(&tag).unwrap();
                self.free_tags.push(tag);
                for parent in loan.parents {
                    if !self.loans.contains_key(&parent) {
                        return Err(self.error(
                            InterpreterError::DanglingReference,
                            "reborrow parent is missing",
                        ));
                    }
                    self.loans.get_mut(&parent).unwrap().children -= 1;
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
            | Value::File(_)
            | Value::Child(_) => Err(self.error(
                InterpreterError::MissingDrop,
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
                                InterpreterError::InactiveVariant,
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
                    if index.ty != ir::U64 {
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
                    access.cell = *elements
                        .get(*index as usize)
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
    fn element_types(&self, values: &[Location], expected: usize) -> bool {
        for value in values {
            match self.cell(*value) {
                Ok(cell) => {
                    if cell.ty != expected {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }
        true
    }
    fn field_types(&self, fields: &[ir::Field], values: &[Location]) -> bool {
        if fields.len() != values.len() {
            return false;
        }
        for index in 0..fields.len() {
            match self.cell(values[index]) {
                Ok(cell) => {
                    if cell.ty != fields[index].ty {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }
        true
    }
    fn validate_value(&self, ty: usize, value: &Value) -> Result<(), RunError> {
        let valid = match (self.kind(ty)?, value) {
            (TypeKind::Unit, Value::Unit) => true,
            (TypeKind::Bool, Value::Bits(value)) => *value <= 1,
            (TypeKind::Char, Value::Bits(value)) => {
                *value <= 0x10ffff && !(*value >= 0xd800 && *value <= 0xdfff)
            }
            (TypeKind::Int { bits, signed }, Value::Bits(value))
                if matches!(*bits, 8 | 16 | 32 | 64) =>
            {
                scalar::normalize(*value, *bits, *signed) == *value
            }
            (TypeKind::F64, Value::Bits(_)) => true,
            (TypeKind::World, Value::World)
            | (TypeKind::Stdout, Value::Stdout)
            | (TypeKind::Stdin, Value::Stdin)
            | (TypeKind::Stderr, Value::Stderr)
            | (TypeKind::Process, Value::Process)
            | (TypeKind::Clock, Value::Clock) => true,
            (TypeKind::Directory, Value::Directory(handle))
            | (TypeKind::File, Value::File(handle))
            | (TypeKind::Child, Value::Child(handle)) => *handle != 0,
            (TypeKind::Ref { inner, mutable }, Value::Reference(reference)) => {
                *inner == reference.referent && *mutable == reference.mutable
            }
            (
                TypeKind::Struct { fields, .. },
                Value::Aggregate {
                    variant: None,
                    fields: values,
                },
            ) => self.field_types(fields, values),
            (
                TypeKind::Enum { variants, .. },
                Value::Aggregate {
                    variant: Some(variant),
                    fields: values,
                },
            ) => match variants.get(*variant) {
                Some(declaration) => self.field_types(&declaration.fields, values),
                None => false,
            },
            (TypeKind::Vec { inner }, Value::Sequence(values)) => {
                self.element_types(values, *inner)
            }
            (TypeKind::Array { inner, len }, Value::Sequence(values)) => {
                *len == values.len() && self.element_types(values, *inner)
            }
            (TypeKind::Str, Value::Sequence(values)) => self.element_types(values, ir::U8),
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
            return Err(self.error(
                InterpreterError::MissingDrop,
                "destination still owns a value",
            ));
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
                    for field in fields {
                        pending.push(field.ty);
                    }
                }
                TypeKind::Enum {
                    variants,
                    copy: true,
                } => {
                    for variant in variants {
                        for field in &variant.fields {
                            pending.push(field.ty);
                        }
                    }
                }
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
                    let mut index = 0usize;
                    while index < fields.len() {
                        let original = fields[index];
                        let ty = self.cell(original)?.ty;
                        let replacement = self.allocate(ty, None, false)?;
                        pending.push((original, replacement));
                        fields[index] = replacement;
                        index += 1;
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
                        jump[open] = Some(match otherwise {
                            Some(position) => position + 1,
                            None => pc + 1,
                        });
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
                    let mut index = controls.len();
                    let mut remaining = depth;
                    let mut target = None;
                    while index > 0 {
                        index -= 1;
                        if controls[index].1 {
                            if remaining == 0 {
                                target = Some(index);
                                break;
                            }
                            remaining -= 1;
                        }
                    }
                    let index = target.ok_or_else(|| {
                        self.invalid("loop transfer target is outside enclosing loops")
                    })?;
                    let control = &mut controls[index];
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
    ) -> Result<Option<u64>, RunError> {
        if self.runtime.is_none() {
            return Ok(None);
        }
        let Some(primitive) =
            crate::runtime_memory::Primitive::parse(&self.program.functions[function].name)
        else {
            return Ok(None);
        };
        let declaration = &self.program.functions[function];
        if declaration.params != primitive.parameters()
            || declaration.params != arguments.len()
            || declaration.params > declaration.locals.len()
            || !matches!(
                self.kind(declaration.result)?,
                TypeKind::Int {
                    bits: 64,
                    signed: false
                }
            )
            || !matches!(
                declaration.body.first().map(|instruction| &instruction.op),
                Some(Op::Trap { .. })
            )
        {
            return Err(self.invalid("runtime primitive signature or stub differs"));
        }
        let mut words = Vec::new();
        for (index, (ty, value)) in arguments.iter().enumerate() {
            if *ty != declaration.locals[index].ty
                || !matches!(
                    self.kind(*ty)?,
                    TypeKind::Int {
                        bits: 64,
                        signed: false
                    }
                )
            {
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
        action.map(|value: u64| Some(value)).map_err(|fault| {
            if fault == crate::runtime_memory::MemoryFault::Allocation {
                self.trap(6, fault.message())
            } else {
                self.invalid(fault.message())
            }
        })
    }
    fn frame_types(&self, function: usize) -> Result<(usize, Vec<usize>), RunError> {
        let declaration = self
            .program
            .functions
            .get(function)
            .ok_or_else(|| self.invalid("unknown function"))?;
        let mut types = Vec::new();
        for local in &declaration.locals {
            types.push(local.ty);
        }
        Ok((declaration.params, types))
    }
    fn initialized(&self, location: Location) -> Result<bool, RunError> {
        Ok(self.cell(location)?.value.is_some())
    }
    fn sequence_cells(&self, access: &Access) -> Result<Vec<Location>, RunError> {
        Ok(self.sequence(access)?.to_vec())
    }
    fn push_frame(
        &mut self,
        function: usize,
        args: Vec<(usize, Value)>,
        return_to: ReturnTo,
    ) -> Result<(), RunError> {
        let (params, local_types) = self.frame_types(function)?;
        if params != args.len() || params > local_types.len() {
            return Err(self.invalid("call argument count differs"));
        }
        for (index, (ty, value)) in args.iter().enumerate() {
            if *ty != local_types[index] {
                return Err(self.invalid("call argument type differs"));
            }
            self.validate_value(*ty, value)?;
        }
        if self.flow[function].is_none() {
            self.flow[function] = Some(self.prepare_flow(function)?);
        }
        let mut locals = Vec::new();
        for ty in local_types {
            let location = self.allocate(ty, None, false)?;
            locals.push(location);
        }
        for (index, (_, value)) in args.into_iter().enumerate() {
            self.put(locals[index], value)?;
        }
        self.push_work(Work::Frame(Frame {
            function,
            pc: 0,
            locals,
            return_to,
        }))?;
        self.frames += 1;
        self.peak_frames = self.peak_frames.max(self.frames);
        Ok(())
    }
    fn push_work(&mut self, work: Work) -> Result<(), RunError> {
        self.work.push(work);
        Ok(())
    }
    fn scalar_result(&self, ty: usize, value: &Value) -> Result<Scalar, RunError> {
        match (self.kind(ty)?, value) {
            (TypeKind::Unit, Value::Unit) => Ok(Scalar::Unit),
            (TypeKind::Bool, Value::Bits(value)) => Ok(Scalar::Bool(*value != 0)),
            (TypeKind::Char, Value::Bits(value)) => {
                if *value > 0x10ffff {
                    return Err(self.invalid("invalid character result"));
                }
                match char::from_u32(*value as u32) {
                    Some(character) => Ok(Scalar::Char(character)),
                    None => Err(self.invalid("invalid character result")),
                }
            }
            (TypeKind::Int { bits, signed }, Value::Bits(value)) => Ok(Scalar::Int {
                value: *value,
                bits: *bits,
                signed: *signed,
            }),
            (TypeKind::F64, Value::Bits(bits)) => Ok(Scalar::F64 { bits: *bits }),
            _ => Err(self.error(
                InterpreterError::Unsupported,
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
                let mut escaping = false;
                for local in &frame.locals {
                    if self.ancestor(*local, reference.target) {
                        escaping = true;
                    }
                }
                if escaping {
                    return Err(self.error(
                        InterpreterError::DanglingReference,
                        "returned reference points into the retiring frame",
                    ));
                }
            }
            for child in Self::children(&value) {
                if let Some(value) = &self.cell(child)?.value {
                    pending.push(value.clone());
                }
            }
        }
        for local in &frame.locals {
            if self.cell(*local)?.value.is_some() {
                return Err(self.error(
                    InterpreterError::MissingDrop,
                    "return leaves a live local without explicit cleanup",
                ));
            }
        }
        for local in frame.locals {
            self.release(local)?;
        }
        self.frames -= 1;
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
                if ty != ir::UNIT {
                    return Err(self.invalid("destructor returns a value"));
                }
                self.discard_scalar(value)?;
            }
        }
        Ok(())
    }
    fn destructor(&self, ty: usize) -> Option<usize> {
        for entry in &self.program.destructors {
            if entry.0 == ty {
                return Some(entry.1);
            }
        }
        None
    }
    fn drop_work(&mut self, work: DropWork) -> Result<(), RunError> {
        self.site = work.site;
        if !self.initialized(work.access.cell)? {
            // Structural cleanup and explicit DropIf skip an absent value.
            // A strict Drop requires this exact storage to be initialized.
            if !work.optional {
                return Err(self.error(
                    InterpreterError::DoubleDrop,
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
            if let Some(function) = self.destructor(work.access.ty) {
                let declaration = self
                    .program
                    .functions
                    .get(function)
                    .ok_or_else(|| self.invalid("unknown destructor"))?;
                if declaration.params != 1 || declaration.result != ir::UNIT {
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
        let mut children = Self::children(&value);
        match value {
            Value::Reference(reference) => self.release_reference(&reference)?,
            Value::Child(handle) => self.host.drop_process(handle),
            Value::Directory(handle) | Value::File(handle) => self.host.close_handle(handle),
            _ => {}
        }
        if work.release {
            self.release(work.access.cell)?;
        }
        while let Some(child) = children.pop() {
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
        let end = extent
            .start
            .checked_add(extent.count)
            .ok_or_else(|| self.invalid("reference extent exceeds sequence storage"))?;
        if end > elements.len() {
            return Err(self.invalid("reference extent exceeds sequence storage"));
        }
        Ok(&elements[extent.start..end])
    }
    fn bytes(&self, access: &Access) -> Result<Vec<u8>, RunError> {
        self.access(access, false, true)?;
        let elements = self.sequence(access)?;
        let mut bytes = Vec::new();
        for element in elements {
            let element_access = Access {
                extent: None,
                cell: *element,
                ty: ir::U8,
                permissions: access.permissions.clone(),
                writable: false,
            };
            self.access(&element_access, false, true)?;
            if self.cell(*element)?.ty != ir::U8 {
                return Err(self.invalid("byte view contains a non-byte element"));
            }
            let Value::Bits(value) = self.value(*element)? else {
                return Err(self.invalid("byte is not scalar"));
            };
            if *value > 255 {
                return Err(self.invalid("byte exceeds its representation"));
            }
            bytes.push(*value as u8);
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
        for _ in 0..len - 1 {
            let value = self.copy_value(&seed)?;
            fields.push(self.allocate(inner, Some(value), false)?);
        }
        let value = self.take(&seed)?;
        fields.push(self.allocate(inner, Some(value), false)?);
        self.put_local(frame, dst, Value::Sequence(fields))
    }

    fn reference_type(&self, ty: usize) -> Result<(usize, bool), RunError> {
        match self.kind(ty)? {
            TypeKind::Ref { inner, mutable } => Ok((*inner, *mutable)),
            _ => Err(self.invalid("operation requires a reference type")),
        }
    }
    fn result_fields(&self, ty: usize) -> Result<(usize, usize), RunError> {
        let TypeKind::Enum { variants, .. } = self.kind(ty)? else {
            return Err(self.invalid("result must be an enum"));
        };
        if variants.len() != 2 || variants[0].fields.len() != 1 || variants[1].fields.len() != 1 {
            return Err(self.invalid("invalid result shape"));
        }
        Ok((variants[0].fields[0].ty, variants[1].fields[0].ty))
    }
    fn sequence_push(&mut self, cell: Location, child: Location) -> Result<(), RunError> {
        let valid = match &mut self.cells[cell.index].value {
            Some(Value::Sequence(elements)) => {
                elements.push(child);
                true
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(self.invalid("Vec has no buffer"))
        }
    }
    fn sequence_pop(&mut self, cell: Location) -> Result<Option<Location>, RunError> {
        let popped = match &mut self.cells[cell.index].value {
            Some(Value::Sequence(elements)) => Some(elements.pop()),
            _ => None,
        };
        match popped {
            Some(value) => Ok(value),
            None => Err(self.invalid("Vec has no buffer")),
        }
    }
    fn character_result(&self, ty: usize) -> Result<usize, RunError> {
        let TypeKind::Enum { variants, .. } = self.kind(ty)? else {
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
        Ok(variants[1].fields[0].ty)
    }
    fn constructor_types(&self, ty: usize, variant: Option<usize>) -> Result<Vec<usize>, RunError> {
        let mut expected = Vec::new();
        match (variant, self.kind(ty)?) {
            (None, TypeKind::Struct { fields, .. }) => {
                for field in fields {
                    expected.push(field.ty);
                }
            }
            (Some(variant), TypeKind::Enum { variants, .. }) => {
                let declaration = variants
                    .get(variant)
                    .ok_or_else(|| self.invalid("unknown constructor variant"))?;
                for field in &declaration.fields {
                    expected.push(field.ty);
                }
            }
            _ => return Err(self.invalid("aggregate constructor has wrong type")),
        }
        Ok(expected)
    }
    fn intrinsic_reference(
        &self,
        values: &[(usize, Value)],
        index: usize,
    ) -> Result<Access, RunError> {
        match values.get(index) {
            Some((_, Value::Reference(reference))) => self.reference_access(reference),
            _ => Err(self.invalid("intrinsic requires a reference argument")),
        }
    }
    fn intrinsic(
        &mut self,
        frame: &Frame,
        dst: usize,
        name: ir::IntrinsicOp,
        args: &[usize],
    ) -> Result<(), RunError> {
        if name == ir::IntrinsicOp::ArrayRepeat {
            return self.array_repeat(frame, dst, args);
        }
        let destination = self.local(frame, dst)?;
        let mut values = Vec::new();
        for arg in args {
            values.push(self.take_local(frame, *arg)?);
        }
        let result = match name {
            ir::IntrinsicOp::F64FromBits | ir::IntrinsicOp::F64ToBits if values.len() == 1 => {
                let valid = if name == ir::IntrinsicOp::F64FromBits {
                    matches!(
                        self.kind(values[0].0)?,
                        TypeKind::Int {
                            bits: 64,
                            signed: false
                        }
                    ) && matches!(self.kind(destination.ty)?, TypeKind::F64)
                } else {
                    matches!(self.kind(values[0].0)?, TypeKind::F64)
                        && matches!(
                            self.kind(destination.ty)?,
                            TypeKind::Int {
                                bits: 64,
                                signed: false
                            }
                        )
                };
                if !valid {
                    return Err(self.invalid("floating bitcast signature differs"));
                }
                let Value::Bits(bits) = values[0].1 else {
                    return Err(self.invalid("floating bitcast requires scalar bits"));
                };
                Value::Bits(bits)
            }
            ir::IntrinsicOp::Assert if values.len() == 1 => {
                if values[0].0 != ir::BOOL {
                    return Err(self.invalid("assert needs bool"));
                }
                if !matches!(values[0].1, Value::Bits(1)) {
                    return Err(self.trap(5, "assertion failed"));
                }
                Value::Unit
            }
            ir::IntrinsicOp::CharFromU32 if values.len() == 1 => {
                if !matches!(
                    self.kind(values[0].0)?,
                    TypeKind::Int {
                        bits: 32,
                        signed: false
                    }
                ) {
                    return Err(self.invalid("character construction requires u32"));
                }
                let character = self.character_result(destination.ty)?;
                let Value::Bits(code) = values[0].1 else {
                    return Err(self.invalid("character code must be scalar"));
                };
                let character_value = if code <= 0x10ffff {
                    char::from_u32(code as u32)
                } else {
                    None
                };
                match character_value {
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
            ir::IntrinsicOp::Cast if values.len() == 1 => {
                let Value::Bits(value) = values[0].1 else {
                    return Err(self.invalid("cast requires a scalar"));
                };
                Value::Bits(self.number(scalar::cast(
                    self.kind(values[0].0)?,
                    self.kind(destination.ty)?,
                    value,
                ))?)
            }
            ir::IntrinsicOp::VecNew
                if values.is_empty()
                    && matches!(self.kind(destination.ty)?, TypeKind::Vec { .. }) =>
            {
                Value::Sequence(Vec::new())
            }
            ir::IntrinsicOp::BoxNew if values.len() == 1 => {
                let inner = match self.kind(destination.ty)? {
                    TypeKind::Box { inner } => *inner,
                    _ => return Err(self.invalid("box_new result is not Box")),
                };
                if inner != values[0].0 {
                    return Err(self.invalid("Box element type differs"));
                }
                let (ty, value) = values.pop().unwrap();
                Value::Box(self.allocate(ty, Some(value), false)?)
            }
            ir::IntrinsicOp::VecLen | ir::IntrinsicOp::StrLen if values.len() == 1 => {
                let access = self.intrinsic_reference(&values, 0)?;
                self.access(&access, false, true)?;
                Value::Bits(self.sequence(&access)?.len() as u64)
            }
            ir::IntrinsicOp::VecPush if values.len() == 2 => {
                let access = self.intrinsic_reference(&values, 0)?;
                if !access.writable {
                    return Err(self.error(
                        InterpreterError::ConflictingAccess,
                        "Vec push requires an exclusive reference",
                    ));
                }
                self.access(&access, true, true)?;
                let inner = match self.kind(access.ty)? {
                    TypeKind::Vec { inner } => *inner,
                    _ => return Err(self.invalid("push target is not Vec")),
                };
                if inner != values[1].0 {
                    return Err(self.invalid("pushed element type differs"));
                }
                let (ty, value) = values.pop().unwrap();
                let child = self.allocate(ty, Some(value), false)?;
                self.cells[child.index].parent = Some(access.cell);
                self.sequence_push(access.cell, child)?;
                Value::Unit
            }
            ir::IntrinsicOp::VecPop if values.len() == 1 => {
                let access = self.intrinsic_reference(&values, 0)?;
                if !access.writable {
                    return Err(self.error(
                        InterpreterError::ConflictingAccess,
                        "Vec pop requires an exclusive reference",
                    ));
                }
                self.access(&access, true, true)?;
                if !matches!(self.kind(access.ty)?, TypeKind::Vec { .. }) {
                    return Err(self.invalid("pop target is not Vec"));
                }
                match self.sequence_pop(access.cell)? {
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
            ir::IntrinsicOp::Replace if values.len() == 2 => {
                let access = self.intrinsic_reference(&values, 0)?;
                if !access.writable || access.ty != values[1].0 || access.ty != destination.ty {
                    return Err(self.invalid("invalid replacement signature"));
                }
                let old = self.take(&access)?;
                let (_, replacement) = values.pop().unwrap();
                self.put(access.cell, replacement)?;
                old
            }
            ir::IntrinsicOp::Clone if values.len() == 1 => {
                let access = self.intrinsic_reference(&values, 0)?;
                if access.ty != destination.ty {
                    return Err(self.invalid("clone result type differs"));
                }
                self.copy_value(&access)?
            }
            ir::IntrinsicOp::StrAsBytes if values.len() == 1 => {
                let access = self.intrinsic_reference(&values, 0)?;
                if !matches!(self.kind(access.ty)?, TypeKind::Str) {
                    return Err(self.invalid("text projection requires str"));
                }
                let (inner, mutable) = self.reference_type(destination.ty)?;
                if mutable {
                    return Err(self.invalid("text bytes result must be a shared slice"));
                };
                if !matches!(
                    self.kind(inner)?,
                    TypeKind::Slice {
                        inner,
                        mutable: false
                    } if *inner == ir::U8
                ) {
                    return Err(self.invalid("text bytes result is not a byte slice"));
                }
                let (_, Value::Reference(mut reference)) = values.pop().unwrap() else {
                    unreachable!()
                };
                reference.referent = inner;
                Value::Reference(reference)
            }
            ir::IntrinsicOp::StrFromUtf8 if values.len() == 1 => {
                let access = self.intrinsic_reference(&values, 0)?;
                let bytes = self.bytes(&access)?;
                let (success, failure) = self.result_fields(destination.ty)?;
                if std::str::from_utf8(&bytes).is_ok() {
                    let (inner, mutable) = self.reference_type(success)?;
                    if mutable {
                        return Err(self.invalid("UTF-8 success is not a shared reference"));
                    };
                    if !matches!(self.kind(inner)?, TypeKind::Str) {
                        return Err(self.invalid("UTF-8 success is not str"));
                    }
                    let (_, Value::Reference(mut reference)) = values.pop().unwrap() else {
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
            ir::IntrinsicOp::WorldStdout if values.len() == 1 => {
                let access = self.intrinsic_reference(&values, 0)?;
                if !access.writable
                    || !matches!(self.kind(access.ty)?, TypeKind::World)
                    || !matches!(self.value(access.cell)?, Value::World)
                {
                    return Err(self.invalid("stdout authority requires a mutable World"));
                }
                self.access(&access, true, true)?;
                Value::Stdout
            }
            ir::IntrinsicOp::StdoutWrite if values.len() == 2 => {
                let writer = self.intrinsic_reference(&values, 0)?;
                if !writer.writable
                    || !matches!(self.kind(writer.ty)?, TypeKind::Stdout)
                    || !matches!(self.value(writer.cell)?, Value::Stdout)
                {
                    return Err(self.invalid("stdout write requires derived writer authority"));
                }
                self.access(&writer, true, true)?;
                let data = self.intrinsic_reference(&values, 1)?;
                if !matches!(
                    self.kind(data.ty)?,
                    TypeKind::Slice {
                        inner,
                        mutable: false
                    } if *inner == ir::U8
                ) {
                    return Err(self.invalid("stdout write requires bytes"));
                }
                let bytes = self.bytes(&data)?;
                if let Err(message) = self.host.write_stdout(&bytes) {
                    return Err(self.trap(7, &message));
                }
                Value::Unit
            }
            _ => match self.io_intrinsic(destination.ty, name, &values)? {
                Some(value) => value,
                None => {
                    return Err(self.error(
                        InterpreterError::Unsupported,
                        &format!(
                            "intrinsic {} is not implemented by the IR interpreter",
                            name.as_str()
                        ),
                    ));
                }
            },
        };
        for (_, value) in values {
            self.discard_scalar(value)?;
        }
        self.put_local(frame, dst, result)
    }
    fn instruction_at(
        &self,
        function: usize,
        pc: usize,
    ) -> Result<(usize, ir::Instruction), RunError> {
        let declaration = self
            .program
            .functions
            .get(function)
            .ok_or_else(|| self.invalid("unknown function"))?;
        let instruction = declaration
            .body
            .get(pc)
            .ok_or_else(|| self.invalid("function reaches its end without returning"))?
            .clone();
        Ok((declaration.file, instruction))
    }
    fn step(&mut self, mut frame: Frame) -> Result<(), RunError> {
        let (file, instruction) = self.instruction_at(frame.function, frame.pc)?;
        self.site = Site {
            function: frame.function,
            file,
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
                let (inner, mutable) = self.reference_type(self.local(&frame, dst)?.ty)?;
                if mutable {
                    return Err(self.invalid("text literal needs a shared str reference"));
                };
                match self.kind(inner)? {
                    TypeKind::Str => {
                        if std::str::from_utf8(&bytes).is_err() {
                            return Err(self.invalid("text literal is not UTF-8"));
                        }
                    }
                    TypeKind::Array { inner, len } if *inner == ir::U8 && *len == bytes.len() => {}
                    _ => {
                        return Err(
                            self.invalid("byte constant requires str or an exact-length u8 array")
                        );
                    }
                }
                let key = (inner, bytes.clone());
                let existing = match self.texts.get(&key) {
                    Some(cell) => Some(*cell),
                    None => None,
                };
                let cell = match existing {
                    Some(cell) => cell,
                    None => {
                        let mut elements = Vec::new();
                        for byte in &bytes {
                            elements.push(self.allocate(
                                ir::U8,
                                Some(Value::Bits(*byte as u64)),
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
                let (inner, expected) = self.reference_type(self.local(&frame, dst)?.ty)?;
                if expected != mutable {
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
                let (inner, expected) = self.reference_type(self.local(&frame, dst)?.ty)?;
                if expected != mutable {
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
                if self.local(&frame, start)?.ty != ir::U64
                    || self.local(&frame, end)?.ty != ir::U64
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
                    for side in 0usize..2 {
                        let endpoint = if side == 0 { start } else { end };
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
                if self.initialized(destination.cell)? && !destination.writable {
                    return Err(self.error(
                        InterpreterError::ConflictingAccess,
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
                let value = self.number(scalar::unary(self.kind(ty)?, op, value))?;
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
                let comparison = matches!(
                    op,
                    ir::BinaryOp::Eq
                        | ir::BinaryOp::Ne
                        | ir::BinaryOp::Lt
                        | ir::BinaryOp::Le
                        | ir::BinaryOp::Gt
                        | ir::BinaryOp::Ge
                );
                if self.local(&frame, dst)?.ty != if comparison { ir::BOOL } else { left_ty } {
                    return Err(self.invalid("binary result type differs"));
                }
                let (Value::Bits(left), Value::Bits(right)) = (left, right) else {
                    return Err(self.invalid("binary operands are not scalar"));
                };
                let value = self.number(scalar::binary(self.kind(left_ty)?, op, left, right))?;
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
                    Some(value) => {
                        self.put(destination.cell, Value::Bits(value))?;
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
                let expected = self.constructor_types(ty, variant)?;
                let fields = self.fields(&frame, fields, &expected)?;
                self.put_local(&frame, dst, Value::Aggregate { variant, fields })?;
            }
            Op::Array { dst, elements } => {
                let (inner, len) = match self.kind(self.local(&frame, dst)?.ty)? {
                    TypeKind::Array { inner, len } => (*inner, *len),
                    _ => return Err(self.invalid("array constructor has wrong type")),
                };
                let expected = vec![inner; len];
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
                let variant = match self.value(access.cell)? {
                    Value::Aggregate {
                        variant: Some(variant),
                        ..
                    } => *variant,
                    _ => return Err(self.invalid("tag read requires an enum")),
                };
                let value = match instruction.op {
                    Op::TagTest {
                        variant: expected, ..
                    } => (variant == expected) as u64,
                    _ => variant as u64,
                };
                self.put_local(&frame, dst, Value::Bits(value))?;
            }
            Op::Intrinsic { dst, name, args } => self.intrinsic(&frame, dst, name, &args)?,
            Op::If { condition } => {
                let (ty, value) = self.take_local(&frame, condition)?;
                if ty != ir::BOOL {
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
                let mut messages: Vec<String> = Vec::new();
                for function in &self.program.functions {
                    for instruction in &function.body {
                        if let Op::Trap { message } = &instruction.op {
                            if !messages.contains(message) {
                                messages.push(message.clone());
                            }
                        }
                    }
                }
                let mut index = 0usize;
                while index < messages.len() && messages[index] != message {
                    index += 1;
                }
                let code = 16 + index as u64;
                return Err(self.trap(code, &message));
            }
        }
        self.push_work(Work::Frame(frame))?;
        Ok(())
    }
}

/// Run raw IR. The sole-World host entry receives fresh authority implicitly;
/// other entries accept scalar bit patterns only. No caller-supplied integer can
/// populate a reference, owner, World or derived capability.
pub fn run<H: Host>(
    program: &Program,
    entry: usize,
    args: &[u64],
    host: &mut H,
) -> Result<RunResult, RunError> {
    run_inner(program, entry, args, host, None)
}

/// Exercise the sealed source runtime against independent disjoint mappings.
/// Ordinary run never substitutes function bodies, even for identical names.
pub fn run_runtime<H: Host>(
    program: &Program,
    entry: usize,
    args: &[u64],
    memory: &mut crate::runtime_memory::Memory,
    host: &mut H,
) -> Result<RunResult, RunError> {
    run_inner(program, entry, args, host, Some(RuntimeState { memory }))
}
fn run_inner<'a, H: Host>(
    program: &'a Program,
    entry: usize,
    args: &[u64],
    host: &'a mut H,
    runtime: Option<RuntimeState<'a>>,
) -> Result<RunResult, RunError> {
    let site = Site {
        function: entry,
        file: program
            .functions
            .get(entry)
            .map_or(0, |function| function.file),
        line: 1,
    };
    let mut flow: Vec<Option<Flow>> = Vec::new();
    for _ in 0..program.functions.len() {
        flow.push(None);
    }
    let mut interpreter = Interpreter {
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
        flow,
        texts: BTreeMap::new(),
        site,
        result: None,
    };
    let function = program
        .functions
        .get(entry)
        .ok_or_else(|| interpreter.invalid("unknown host entry"))?;
    if function.params > function.locals.len() {
        return Err(interpreter.invalid("entry parameter declarations are missing"));
    }
    let mut values = Vec::new();
    if function.params == 1 && matches!(interpreter.kind(function.locals[0].ty)?, TypeKind::World) {
        if !args.is_empty() {
            return Err(interpreter.invalid("World entry takes no raw arguments"));
        }
        values.push((function.locals[0].ty, Value::World));
    } else {
        if function.params != args.len() {
            return Err(interpreter.invalid("entry argument count differs"));
        }
        for (local, argument) in function.locals.iter().zip(args) {
            if !matches!(
                interpreter.kind(local.ty)?,
                TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64
            ) {
                return Err(interpreter.invalid("raw host arguments may only be scalar values"));
            }
            values.push((local.ty, Value::Bits(*argument)));
        }
    }
    interpreter.push_frame(entry, values, ReturnTo::Entry)?;
    while let Some(work) = interpreter.work.pop() {
        match work {
            Work::Frame(frame) => interpreter.step(frame)?,
            Work::Drop(work) => interpreter.drop_work(work)?,
        }
    }
    let value = interpreter
        .result
        .take()
        .ok_or_else(|| interpreter.invalid("interpreter finished without an entry result"))?;
    Ok(RunResult {
        value,
        peak_frames: interpreter.peak_frames,
        peak_cells: interpreter.peak_cells,
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
            types: ir::primitives(),
            functions: Vec::new(),
            destructors: Vec::new(),
        };
        let mut host = Vec::new();
        let mut interpreter = Interpreter {
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
            flow: Vec::new(),
            texts: BTreeMap::new(),
            site: Site {
                function: 0,
                file: 0,
                line: 1,
            },
            result: None,
        };
        let old = checked(interpreter.allocate(ir::I64, Some(Value::Bits(1)), false));
        let old_access = Access {
            extent: None,
            cell: old,
            ty: ir::I64,
            permissions: Vec::new(),
            writable: true,
        };
        let Value::Reference(stale) = checked(interpreter.borrow(&old_access, ir::I64, false))
        else {
            panic!("borrow did not return a reference")
        };
        // Force the lifetime violation directly: ordinary IR execution reports
        // the conflicting destruction before this allocator boundary is reached.
        interpreter.cells[old.index].value = None;
        checked(interpreter.release(old));
        let replacement = checked(interpreter.allocate(ir::I64, Some(Value::Bits(2)), false));
        assert_eq!(old.index, replacement.index);
        let error = interpreter
            .reference_access(&stale)
            .err()
            .expect("stale slot reference accepted");
        assert!(error.kind == ErrorKind::Interpreter(InterpreterError::DanglingReference));
        checked(interpreter.release_reference(&stale));
        let access = Access {
            extent: None,
            cell: replacement,
            ty: ir::I64,
            permissions: Vec::new(),
            writable: true,
        };
        let Value::Reference(current) = checked(interpreter.borrow(&access, ir::I64, false)) else {
            panic!("borrow did not return a reference")
        };
        assert_eq!(stale.tag, current.tag);
        let error = interpreter
            .reference_access(&stale)
            .err()
            .expect("stale tag reference accepted");
        assert!(error.kind == ErrorKind::Interpreter(InterpreterError::DanglingReference));
        checked(interpreter.reference_access(&current));
        checked(interpreter.release_reference(&current));
        let other = checked(interpreter.allocate(ir::I64, Some(Value::Bits(3)), false));
        let other_access = Access {
            extent: None,
            cell: other,
            ty: ir::I64,
            permissions: Vec::new(),
            writable: true,
        };
        let Value::Reference(other_reference) =
            checked(interpreter.borrow(&other_access, ir::I64, false))
        else {
            panic!("borrow did not return a reference")
        };
        assert_eq!(stale.tag, other_reference.tag);
        assert_eq!(stale.identity, other_reference.identity);
        assert_ne!(stale.target.index, other_reference.target.index);
        let error = interpreter
            .reference_access(&stale)
            .err()
            .expect("stale reference reached another cell");
        assert!(error.kind == ErrorKind::Interpreter(InterpreterError::DanglingReference));
        let error = interpreter
            .release_reference(&stale)
            .err()
            .expect("stale reference released another cell's loan");
        assert!(error.kind == ErrorKind::Interpreter(InterpreterError::DanglingReference));
        checked(interpreter.reference_access(&other_reference));
    }
}
