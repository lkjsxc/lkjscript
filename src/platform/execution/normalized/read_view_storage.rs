//! Runtime custody for affine read-view descriptors.
//!
//! Canonical keys below are observations, not certificates. Both evaluators must implement
//! `ReadViewAdmission` from their own complete source/type/provenance admission. This module
//! then checks actual sealed read tokens and owns their cleanup. It never interprets a dense
//! function index as authority, accepts an owning token as custody, or exposes passive custody
//! through field lookup. Existing checked-value ancestor packets must be flattened into
//! `SourceLoans`/retained descriptor leases before entering this boundary.

use super::value::{MAXIMUM_ADMISSION_ITEMS, NormalizedValue, ValueOrigin};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{ReadScopeArgument, TypeObjectDigest};
use crate::platform::semantic_id::{BindingId, FieldId};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(test)]
#[path = "read_view_storage_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ViewIdentity {
    pub ty: TypeObjectDigest,
    pub program: ValueOrigin,
}

/// Exact canonical key, including contract methods, members and implementation bindings.
/// It is never a runtime lease identity or permission.
pub(super) type CanonicalScopeKey = ReadScopeArgument;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SourceCoordinate {
    Parameter(u32),
    Binding(BindingId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReadPathStep {
    OwnedField(FieldId),
    OwnedSlot {
        ty: TypeObjectDigest,
        field: u32,
    },
    OwnedChoiceSlot {
        ty: TypeObjectDigest,
        case: u32,
    },
    OwnedIndex {
        ty: TypeObjectDigest,
        index: u64,
    },
    ViewReference {
        ty: TypeObjectDigest,
        field: FieldId,
    },
}

/// The checked caller supplies a complete exact application identity, fresh invocation ID,
/// ordered argument coordinate and tagged path. Constructing/cloning this key grants nothing.
/// Path backing is prepared and charged by the caller; storage never reconstructs paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SourceAnchor {
    pub program: ValueOrigin,
    pub application: ValueOrigin,
    pub invocation: ValueOrigin,
    pub coordinate: SourceCoordinate,
    pub origin: ValueOrigin,
    pub path: Arc<[ReadPathStep]>,
}

pub(super) struct ViewConstruction {
    pub identity: ViewIdentity,
    pub source: SourceAnchor,
    pub scope: RuntimeScopeLease,
    pub fields: usize,
}

/// The checked caller derives this effective identity and scope by exact canonical
/// substitution. The immutable backing keeps its original layout and token origins.
pub(super) struct ViewTarget {
    pub identity: ViewIdentity,
    pub source: SourceAnchor,
    pub scope: RuntimeScopeLease,
    pub sharing: bool,
}

/// A borrowed token plus its immutable actual origin. The adapter supplies the complete
/// ancestor closure, including tokens retained by existing provenance/return packets.
pub(super) struct HeldRead {
    pub origin: ValueOrigin,
    pub value: NormalizedValue,
}

/// Passive source custody. Only the trusted admission callback can inspect this packet;
/// no descriptor field/projection method returns it or follows it as a readable edge.
pub(super) struct SourceLoans {
    reads: Vec<HeldRead>,
    expected: usize,
}

impl SourceLoans {
    pub(super) fn new(
        selected: HeldRead,
        ancestors: usize,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let expected = ancestors.checked_add(1).ok_or_else(capacity)?;
        bounded_items(expected)?;
        validate_read(&selected.value, selected.origin, None, control)?;
        let mut reads = reserved_vec(expected, control, reserve)?;
        reads.push(selected);
        Ok(Self { reads, expected })
    }

    pub(super) fn retain_ancestor(
        &mut self,
        read: HeldRead,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        if self.reads.len() == self.expected {
            return Err(capacity());
        }
        validate_read(&read.value, read.origin, None, control)?;
        self.reads.push(read);
        Ok(())
    }

    pub(super) fn reads(&self) -> &[HeldRead] {
        &self.reads
    }

    fn validate(
        &self,
        program: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        if self.reads.len() != self.expected {
            return Err(reject());
        }
        for read in &self.reads {
            control.check()?;
            validate_read(&read.value, read.origin, Some(program), control)?;
        }
        Ok(())
    }
}

/// Source-scope lifetime, independently of any descriptor local/frame lifetime. The trusted
/// caller holds this owner for the actual source scope, rather than for the view variable.
pub(super) struct RuntimeScopeOwner {
    live: Arc<AtomicBool>,
}

pub(super) struct RuntimeScopeLease {
    key: CanonicalScopeKey,
    seal: ValueOrigin,
    source: SourceAnchor,
    // Flat live controls avoid a recursive parent-scope Arc drop cascade.
    live: Arc<Vec<Arc<AtomicBool>>>,
}

impl RuntimeScopeOwner {
    pub(super) fn open(
        key: CanonicalScopeKey,
        source: &SourceAnchor,
        parent: Option<&RuntimeScopeLease>,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<(Self, RuntimeScopeLease), ExecutionError> {
        if let Some(parent) = parent {
            parent.validate(control)?;
        }
        admit(
            admission,
            AdmissionRequest::Scope {
                key,
                source,
                parent,
            },
            control,
        )?;
        let count = match parent {
            Some(parent) => parent.live.len().checked_add(1).ok_or_else(capacity)?,
            None => 1,
        };
        bounded_items(count)?;
        let mut live = reserved_vec(count, control, reserve)?;
        if let Some(parent) = parent {
            for flag in parent.live.iter() {
                control.check()?;
                live.push(Arc::clone(flag));
            }
        }
        charge::<AtomicBool>(1, control, reserve)?;
        charge::<Vec<Arc<AtomicBool>>>(1, control, reserve)?;
        let seal = ValueOrigin::fresh().ok_or_else(capacity)?;
        let flag = Arc::new(AtomicBool::new(true));
        live.push(Arc::clone(&flag));
        let lease = RuntimeScopeLease {
            key,
            seal,
            source: source.clone(),
            live: Arc::new(live),
        };
        Ok((Self { live: flag }, lease))
    }
}

impl Drop for RuntimeScopeOwner {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
    }
}

impl RuntimeScopeLease {
    pub(super) fn key(&self) -> CanonicalScopeKey {
        self.key
    }

    pub(super) fn source(&self) -> &SourceAnchor {
        &self.source
    }

    pub(super) fn same_live_scope(&self, other: &Self) -> bool {
        self.seal == other.seal && Arc::ptr_eq(&self.live, &other.live)
    }

    fn validate(&self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        for flag in self.live.iter() {
            control.check()?;
            if !flag.load(Ordering::Acquire) {
                return Err(reject());
            }
        }
        Ok(())
    }

    fn within(&self, parent: &Self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        self.validate(control)?;
        parent.validate(control)?;
        if self.live.len() < parent.live.len() {
            return Err(reject());
        }
        for (child, parent) in self.live.iter().zip(parent.live.iter()) {
            control.check()?;
            if !Arc::ptr_eq(child, parent) {
                return Err(reject());
            }
        }
        Ok(())
    }

    pub(super) fn reborrow(&self, control: &ExecutionControl) -> Result<Self, ExecutionError> {
        self.validate(control)?;
        Ok(Self {
            key: self.key,
            seal: self.seal,
            source: self.source.clone(),
            live: Arc::clone(&self.live),
        })
    }
}

/// All callback invocations are outside backing locks. Implementations must independently
/// check canonical complete layouts, type/phantom actuals, source provenance, result modes,
/// scope containment and (for sharing) every declared accessible field, even inactive ones.
/// `Scope`/`Source` are also mandatory for metadata-only, zero-reference descriptors.
pub(super) trait ReadViewAdmission {
    fn validate(
        &self,
        request: AdmissionRequest<'_>,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError>;
}

pub(super) enum AdmissionRequest<'a> {
    Scope {
        key: CanonicalScopeKey,
        source: &'a SourceAnchor,
        parent: Option<&'a RuntimeScopeLease>,
    },
    Source {
        construction: &'a ViewConstruction,
        loans: &'a SourceLoans,
    },
    Subview {
        construction: &'a ViewConstruction,
        source: &'a ReadView,
    },
    Field {
        construction: &'a ViewConstruction,
        input: FieldAdmission<'a>,
    },
    Finish {
        construction: &'a ViewConstruction,
        fields: &'a [StoredField],
    },
    Retain {
        construction: &'a ViewConstruction,
        guard: &'a ReadView,
    },
    RetainProjected {
        construction: &'a ViewConstruction,
        guard: &'a ProjectedOwnedRead,
    },
    RetainRead {
        construction: &'a ViewConstruction,
        read: &'a HeldRead,
    },
    Reborrow {
        source: &'a ReadView,
        target: &'a ViewTarget,
    },
    Projection {
        source: &'a ReadView,
        field: &'a FieldDescriptor,
        destination: &'a SourceAnchor,
        target: Option<&'a ViewTarget>,
    },
    MetadataResult {
        source: &'a ReadView,
        field: FieldId,
        ty: TypeObjectDigest,
        value: &'a NormalizedValue,
    },
}

pub(super) enum FieldAdmission<'a> {
    Metadata {
        field: FieldId,
        ty: TypeObjectDigest,
        value: &'a NormalizedValue,
    },
    Owned {
        field: FieldId,
        ty: TypeObjectDigest,
        source: &'a SourceAnchor,
        value: &'a NormalizedValue,
    },
    View {
        field: FieldId,
        ty: TypeObjectDigest,
        source: &'a SourceAnchor,
        value: &'a ReadView,
    },
}

#[derive(Clone, Debug)]
pub(super) struct FieldDescriptor {
    pub field: FieldId,
    pub ty: TypeObjectDigest,
    pub source: Option<SourceAnchor>,
    pub kind: FieldKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FieldKind {
    Metadata,
    OwnedReference,
    ViewReference,
}

pub(super) struct StoredField {
    descriptor: FieldDescriptor,
    value: StoredValue,
}

impl StoredField {
    pub(super) fn descriptor(&self) -> &FieldDescriptor {
        &self.descriptor
    }
}

enum StoredValue {
    Metadata(Arc<NormalizedValue>),
    Owned(NormalizedValue),
    View(Guard),
}

struct CustodyOnly {
    root: Arc<SourceLoans>,
    // Parent, provenance and return guards use the same iterative-release protocol. None is
    // an accessible field, including the chosen parent source of a metadata-only subview.
    links: Vec<Guard>,
    // Field evaluation can acquire additional flat ancestor loans after source admission.
    // These remain inaccessible and do not replace/relabel the selected source packet.
    reads: Vec<HeldRead>,
}

impl CustodyOnly {
    fn validate(
        &self,
        program: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        self.root.validate(program, control)?;
        for read in &self.reads {
            validate_read(&read.value, read.origin, Some(program), control)?;
        }
        Ok(())
    }
}

struct ReadyCleanup {
    fields: Vec<StoredField>,
    custody: Option<CustodyOnly>,
    next: Option<Box<ReadyCleanup>>,
    #[cfg(test)]
    finalized: Option<Arc<std::sync::atomic::AtomicUsize>>,
}

struct State {
    leases: usize,
    ready: Option<Box<ReadyCleanup>>,
}

struct Backing {
    identity: ViewIdentity,
    source: SourceAnchor,
    scope: RuntimeScopeLease,
    state: Mutex<State>,
}

impl Backing {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// `live` is a semantic affine/read lease, not an Arc strong count. Raw clones are inert.
struct Lease {
    backing: Option<Arc<Backing>>,
    live: bool,
}

impl Lease {
    fn backing(&self) -> Result<&Arc<Backing>, ExecutionError> {
        if !self.live {
            return Err(reject());
        }
        self.backing.as_ref().ok_or_else(reject)
    }

    fn validate(&self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        let backing = self.backing()?;
        backing.scope.validate(control)?;
        let state = backing.lock();
        let ready = state.ready.as_ref().ok_or_else(reject)?;
        ready
            .custody
            .as_ref()
            .ok_or_else(reject)?
            .validate(backing.identity.program, control)
    }

    fn acquire(&self, control: &ExecutionControl) -> Result<Self, ExecutionError> {
        self.validate(control)?;
        let backing = self.backing()?;
        let mut state = backing.lock();
        if state.ready.is_none() || state.leases == 0 {
            return Err(reject());
        }
        state.leases = state.leases.checked_add(1).ok_or_else(capacity)?;
        Ok(Self {
            backing: Some(Arc::clone(backing)),
            live: true,
        })
    }

    fn release_into(&mut self, work: &mut Cleanup) {
        let Some(backing) = self.backing.take() else {
            return;
        };
        let live = std::mem::replace(&mut self.live, false);
        if !live {
            return;
        }
        let ready = {
            let mut state = backing.lock();
            debug_assert!(state.leases != 0);
            state.leases -= 1;
            if state.leases == 0 {
                state.ready.take()
            } else {
                None
            }
        };
        if let Some(ready) = ready {
            work.enqueue(ready);
        }
        // A remaining physical Arc shell is empty if the last semantic lease just closed.
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let mut work = Cleanup::default();
        self.release_into(&mut work);
        work.drain();
    }
}

/// A read/reference/return packet retains both the readable descriptor and any containing
/// descriptor provenance. Keeping only the field's underlying backing would lose the exact
/// selected source loan when a projected view escapes its containing local.
struct Guard {
    lease: Lease,
    provenance: Option<Lease>,
    scope: RuntimeScopeLease,
}

impl Guard {
    fn validate(&self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        self.scope.validate(control)?;
        self.lease.validate(control)?;
        if let Some(provenance) = &self.provenance {
            provenance.validate(control)?;
        }
        Ok(())
    }

    fn acquire(&self, control: &ExecutionControl) -> Result<Self, ExecutionError> {
        self.validate(control)?;
        let lease = self.lease.acquire(control)?;
        let provenance = self
            .provenance
            .as_ref()
            .map(|lease| lease.acquire(control))
            .transpose()?;
        Ok(Self {
            lease,
            provenance,
            scope: self.scope.reborrow(control)?,
        })
    }

    fn release_into(&mut self, work: &mut Cleanup) {
        self.lease.release_into(work);
        if let Some(mut provenance) = self.provenance.take() {
            provenance.release_into(work);
        }
    }
}

#[derive(Default)]
struct Cleanup {
    head: Option<Box<ReadyCleanup>>,
}

impl Cleanup {
    fn enqueue(&mut self, mut node: Box<ReadyCleanup>) {
        debug_assert!(node.next.is_none());
        node.next = self.head.take();
        self.head = Some(node);
    }

    fn drain(&mut self) {
        while let Some(mut node) = self.head.take() {
            self.head = node.next.take();
            // Take the intrusive link before any drop. Every dynamic descriptor/provenance
            // edge becomes inert and queues its ready node; no nested lease Drop traverses it.
            for mut field in node.fields.drain(..) {
                if let StoredValue::View(guard) = &mut field.value {
                    guard.release_into(self);
                }
                // Owned fields can only hold Read-mode tokens, whose Drop releases a loan.
                // Ordinary metadata was fully admitted by the evaluator before publication.
            }
            if let Some(mut custody) = node.custody.take() {
                for mut link in custody.links.drain(..) {
                    link.release_into(self);
                }
                custody.reads.clear();
                // The root packet is flat, contains only Read tokens, and never runs user code.
                drop(custody);
            }
            #[cfg(test)]
            if let Some(finalized) = node.finalized.take() {
                finalized.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        self.drain();
    }
}

struct PendingCleanup(Option<Box<ReadyCleanup>>);

impl Drop for PendingCleanup {
    fn drop(&mut self) {
        if let Some(ready) = self.0.take() {
            let mut work = Cleanup::default();
            work.enqueue(ready);
            work.drain();
        }
    }
}

/// Owns a pre-reserved cleanup node immediately, including while partially initialized.
/// Errors, cancellation and ordinary unwinding use exactly the same iterative cleanup path.
pub(super) struct ViewBuilder {
    construction: Option<ViewConstruction>,
    ready: Option<Box<ReadyCleanup>>,
    guard_capacity: usize,
}

impl ViewBuilder {
    pub(super) fn from_owned(
        construction: ViewConstruction,
        loans: SourceLoans,
        guards: usize,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        validate_construction(&construction, control)?;
        loans.validate(construction.identity.program, control)?;
        if loans.reads[0].origin != construction.source.origin {
            return Err(reject());
        }
        admit(
            admission,
            AdmissionRequest::Source {
                construction: &construction,
                loans: &loans,
            },
            control,
        )?;
        charge::<SourceLoans>(1, control, reserve)?;
        let root = Arc::new(loans);
        Self::reserved(construction, root, guards, control, reserve)
    }

    pub(super) fn from_view(
        construction: ViewConstruction,
        source: &ReadView,
        guards: usize,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        validate_construction(&construction, control)?;
        source.validate(control)?;
        let backing = source.lease.backing()?;
        if construction.identity.program != backing.identity.program {
            return Err(reject());
        }
        construction.scope.within(source.scope()?, control)?;
        admit(
            admission,
            AdmissionRequest::Subview {
                construction: &construction,
                source,
            },
            control,
        )?;
        let root = {
            let source_backing = source
                .provenance
                .as_ref()
                .unwrap_or(&source.lease)
                .backing()?;
            let state = source_backing.lock();
            Arc::clone(
                &state
                    .ready
                    .as_ref()
                    .ok_or_else(reject)?
                    .custody
                    .as_ref()
                    .ok_or_else(reject)?
                    .root,
            )
        };
        let capacity = guards.checked_add(1).ok_or_else(capacity)?;
        let mut builder = Self::reserved(construction, root, capacity, control, reserve)?;
        let parent = source.guard(control)?;
        builder.custody_mut()?.links.push(parent);
        Ok(builder)
    }

    fn reserved(
        construction: ViewConstruction,
        root: Arc<SourceLoans>,
        guards: usize,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        bounded_items(guards)?;
        let fields = reserved_vec(construction.fields, control, reserve)?;
        let links = reserved_vec(guards, control, reserve)?;
        charge_box::<ReadyCleanup>(control, reserve)?;
        let ready = Box::new(ReadyCleanup {
            fields,
            custody: Some(CustodyOnly {
                root,
                links,
                reads: Vec::new(),
            }),
            next: None,
            #[cfg(test)]
            finalized: None,
        });
        Ok(Self {
            construction: Some(construction),
            ready: Some(ready),
            guard_capacity: guards,
        })
    }

    fn construction(&self) -> Result<&ViewConstruction, ExecutionError> {
        self.construction.as_ref().ok_or_else(reject)
    }

    fn custody_mut(&mut self) -> Result<&mut CustodyOnly, ExecutionError> {
        self.ready
            .as_mut()
            .and_then(|ready| ready.custody.as_mut())
            .ok_or_else(reject)
    }

    fn field_slot(&self, field: FieldId, control: &ExecutionControl) -> Result<(), ExecutionError> {
        let construction = self.construction()?;
        validate_construction(construction, control)?;
        let ready = self.ready.as_ref().ok_or_else(reject)?;
        ready
            .custody
            .as_ref()
            .ok_or_else(reject)?
            .validate(construction.identity.program, control)?;
        if ready.fields.len() == construction.fields {
            return Err(capacity());
        }
        for existing in &ready.fields {
            control.check()?;
            if existing.descriptor.field == field {
                return Err(reject());
            }
        }
        Ok(())
    }

    pub(super) fn metadata(
        &mut self,
        field: FieldId,
        ty: TypeObjectDigest,
        value: NormalizedValue,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        self.field_slot(field, control)?;
        // Complete nested metadata/category validation remains the evaluator's obligation.
        if value.memory_form().is_some()
            || matches!(
                value,
                NormalizedValue::Function { .. } | NormalizedValue::Resource(_)
            )
        {
            return Err(reject());
        }
        admit(
            admission,
            AdmissionRequest::Field {
                construction: self.construction()?,
                input: FieldAdmission::Metadata {
                    field,
                    ty,
                    value: &value,
                },
            },
            control,
        )?;
        charge::<NormalizedValue>(1, control, reserve)?;
        self.ready
            .as_mut()
            .ok_or_else(reject)?
            .fields
            .push(StoredField {
                descriptor: FieldDescriptor {
                    field,
                    ty,
                    source: None,
                    kind: FieldKind::Metadata,
                },
                value: StoredValue::Metadata(Arc::new(value)),
            });
        Ok(())
    }

    pub(super) fn owned_reference(
        &mut self,
        field: FieldId,
        ty: TypeObjectDigest,
        source: SourceAnchor,
        value: NormalizedValue,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        self.field_slot(field, control)?;
        let construction = self.construction()?;
        if source.program != construction.identity.program {
            return Err(reject());
        }
        validate_read(&value, source.origin, Some(source.program), control)?;
        admit(
            admission,
            AdmissionRequest::Field {
                construction,
                input: FieldAdmission::Owned {
                    field,
                    ty,
                    source: &source,
                    value: &value,
                },
            },
            control,
        )?;
        self.ready
            .as_mut()
            .ok_or_else(reject)?
            .fields
            .push(StoredField {
                descriptor: FieldDescriptor {
                    field,
                    ty,
                    source: Some(source),
                    kind: FieldKind::OwnedReference,
                },
                value: StoredValue::Owned(value),
            });
        Ok(())
    }

    pub(super) fn view_reference(
        &mut self,
        field: FieldId,
        source: SourceAnchor,
        value: &ReadView,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        self.field_slot(field, control)?;
        value.validate(control)?;
        let construction = self.construction()?;
        if value.identity().program != construction.identity.program
            || source.program != construction.identity.program
        {
            return Err(reject());
        }
        construction.scope.within(value.scope()?, control)?;
        let ty = value.identity().ty;
        admit(
            admission,
            AdmissionRequest::Field {
                construction,
                input: FieldAdmission::View {
                    field,
                    ty,
                    source: &source,
                    value,
                },
            },
            control,
        )?;
        let guard = value.guard(control)?;
        self.ready
            .as_mut()
            .ok_or_else(reject)?
            .fields
            .push(StoredField {
                descriptor: FieldDescriptor {
                    field,
                    ty,
                    source: Some(source),
                    kind: FieldKind::ViewReference,
                },
                value: StoredValue::View(guard),
            });
        Ok(())
    }

    pub(super) fn retain_guard(
        &mut self,
        guard: &ReadView,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        let construction = self.construction()?;
        validate_construction(construction, control)?;
        guard.validate(control)?;
        if guard.identity().program != construction.identity.program {
            return Err(reject());
        }
        construction.scope.within(guard.scope()?, control)?;
        if self
            .ready
            .as_ref()
            .ok_or_else(reject)?
            .custody
            .as_ref()
            .ok_or_else(reject)?
            .links
            .len()
            == self.guard_capacity
        {
            return Err(capacity());
        }
        admit(
            admission,
            AdmissionRequest::Retain {
                construction,
                guard,
            },
            control,
        )?;
        let retained = guard.guard(control)?;
        self.custody_mut()?.links.push(retained);
        Ok(())
    }

    /// Reserve unknown late provenance/return packet depth before retaining its descriptors.
    /// Reallocation uses charged scratch and restores every existing guard on cancellation.
    pub(super) fn reserve_guards(
        &mut self,
        additional: usize,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        validate_construction(self.construction()?, control)?;
        let required = self
            .guard_capacity
            .checked_add(additional)
            .ok_or_else(capacity)?;
        bounded_items(required)?;
        let links = &mut self.custody_mut()?.links;
        grow_reserved(links, required, control, reserve)?;
        let available = links.capacity();
        self.guard_capacity = available;
        Ok(())
    }

    pub(super) fn retain_read(
        &mut self,
        read: HeldRead,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        let construction = self.construction()?;
        validate_construction(construction, control)?;
        validate_read(
            &read.value,
            read.origin,
            Some(construction.identity.program),
            control,
        )?;
        admit(
            admission,
            AdmissionRequest::RetainRead {
                construction,
                read: &read,
            },
            control,
        )?;
        let reads = &mut self.custody_mut()?.reads;
        let required = reads.len().checked_add(1).ok_or_else(capacity)?;
        grow_reserved(reads, required, control, reserve)?;
        reads.push(read);
        Ok(())
    }

    /// Transfer complete projected-return custody into a new descriptor before releasing the
    /// checked input packet. A new borrowed raw token alone does not carry this provenance.
    pub(super) fn retain_projected(
        &mut self,
        guard: &ProjectedOwnedRead,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        let construction = self.construction()?;
        validate_construction(construction, control)?;
        guard.raw(control)?;
        if guard.destination.program != construction.identity.program {
            return Err(reject());
        }
        construction.scope.within(&guard.guard.scope, control)?;
        if self
            .ready
            .as_ref()
            .ok_or_else(reject)?
            .custody
            .as_ref()
            .ok_or_else(reject)?
            .links
            .len()
            == self.guard_capacity
        {
            return Err(capacity());
        }
        admit(
            admission,
            AdmissionRequest::RetainProjected {
                construction,
                guard,
            },
            control,
        )?;
        let retained = guard.guard.acquire(control)?;
        self.custody_mut()?.links.push(retained);
        Ok(())
    }

    pub(super) fn finish(
        mut self,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<ReadView, ExecutionError> {
        let construction = self.construction()?;
        validate_construction(construction, control)?;
        let ready = self.ready.as_ref().ok_or_else(reject)?;
        ready
            .custody
            .as_ref()
            .ok_or_else(reject)?
            .validate(construction.identity.program, control)?;
        if ready.fields.len() != construction.fields {
            return Err(reject());
        }
        admit(
            admission,
            AdmissionRequest::Finish {
                construction,
                fields: &ready.fields,
            },
            control,
        )?;
        charge::<Backing>(1, control, reserve)?;
        let reservation = ReadView::reserve_descriptor(control, reserve)?;
        let construction = self.construction.take().ok_or_else(reject)?;
        let scope = construction.scope.reborrow(control)?;
        let descriptor = reservation.allocate(
            ViewDescriptor {
                identity: construction.identity,
                source: construction.source.clone(),
                scope,
            },
            control,
        )?;
        let backing = Arc::new(Backing {
            identity: construction.identity,
            source: construction.source.clone(),
            scope: construction.scope,
            state: Mutex::new(State {
                leases: 1,
                ready: self.ready.take(),
            }),
        });
        Ok(ReadView {
            descriptor,
            lease: Lease {
                backing: Some(backing),
                live: true,
            },
            provenance: None,
            shared: false,
        })
    }
}

impl Drop for ViewBuilder {
    fn drop(&mut self) {
        if let Some(ready) = self.ready.take() {
            let mut work = Cleanup::default();
            work.enqueue(ready);
            work.drain();
        }
    }
}

// Immutable observations are shared; each handle still owns its distinct affine
// leases below. Metadata sharing neither acquires a lease nor extends source life.
struct ViewDescriptor {
    identity: ViewIdentity,
    source: SourceAnchor,
    scope: RuntimeScopeLease,
}

/// One exact descriptor allocation, reserved before borrowing a live caller frame.
/// This private, non-Clone token cannot mint authority or be used twice.
pub(super) struct DescriptorReservation(());

impl DescriptorReservation {
    fn allocate(
        self,
        descriptor: ViewDescriptor,
        control: &ExecutionControl,
    ) -> Result<Arc<ViewDescriptor>, ExecutionError> {
        control.check()?;
        Ok(Arc::new(descriptor))
    }
}

pub struct ReadView {
    descriptor: Arc<ViewDescriptor>,
    lease: Lease,
    provenance: Option<Lease>,
    shared: bool,
}

impl std::fmt::Debug for ReadView {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ReadView(<opaque affine token>)")
    }
}

/// Matches existing raw memory carriers: Clone conveys an inert observation only, never a
/// second affine/read lease. Even retained inert clones do not prolong source custody.
impl Clone for ReadView {
    fn clone(&self) -> Self {
        Self {
            descriptor: Arc::clone(&self.descriptor),
            lease: Lease {
                backing: self.lease.backing.as_ref().map(Arc::clone),
                live: false,
            },
            provenance: self.provenance.as_ref().map(|lease| Lease {
                backing: lease.backing.as_ref().map(Arc::clone),
                live: false,
            }),
            shared: self.shared,
        }
    }
}

/// Raw carrier identity is observational only, like the existing Owned token equality.
/// Neither equality nor an inert clone grants ordered-argument/provenance permission.
impl PartialEq for ReadView {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor.identity == other.descriptor.identity
            && self.descriptor.source == other.descriptor.source
            && self
                .descriptor
                .scope
                .same_live_scope(&other.descriptor.scope)
            && match (&self.lease.backing, &other.lease.backing) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && match (&self.provenance, &other.provenance) {
                (Some(left), Some(right)) => match (&left.backing, &right.backing) {
                    (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                    (None, None) => true,
                    _ => false,
                },
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for ReadView {}

impl ReadView {
    pub(super) fn reserve_descriptor(
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<DescriptorReservation, ExecutionError> {
        charge::<ViewDescriptor>(1, control, reserve)?;
        Ok(DescriptorReservation(()))
    }
    pub(super) fn identity(&self) -> ViewIdentity {
        self.descriptor.identity
    }
    pub(super) fn source(&self) -> &SourceAnchor {
        &self.descriptor.source
    }
    /// Original admitted readable-construction anchor, distinct from a rebased handle alias
    /// and from the source-scope lease's lifetime anchor. Observation grants no token access.
    pub(super) fn backing_source(&self) -> Result<&SourceAnchor, ExecutionError> {
        Ok(&self.lease.backing()?.source)
    }
    pub(super) fn is_shared(&self) -> bool {
        self.shared
    }
    pub(super) fn scope(&self) -> Result<&RuntimeScopeLease, ExecutionError> {
        self.lease.backing()?;
        Ok(&self.descriptor.scope)
    }

    fn guard(&self, control: &ExecutionControl) -> Result<Guard, ExecutionError> {
        self.validate(control)?;
        let lease = self.lease.acquire(control)?;
        let provenance = self
            .provenance
            .as_ref()
            .map(|lease| lease.acquire(control))
            .transpose()?;
        Ok(Guard {
            lease,
            provenance,
            scope: self.descriptor.scope.reborrow(control)?,
        })
    }

    pub(super) fn validate(&self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        // The effective digest may differ after independently admitted scope narrowing. Only
        // private constructors mint that seal; a raw clone still has no live lease.
        if self.descriptor.source.program != self.descriptor.identity.program
            || self.lease.backing()?.identity.program != self.descriptor.identity.program
            || self.descriptor.scope.source.program != self.descriptor.identity.program
        {
            return Err(reject());
        }
        self.lease.validate(control)?;
        self.descriptor.scope.validate(control)?;
        if let Some(provenance) = &self.provenance {
            provenance.validate(control)?;
        }
        Ok(())
    }

    pub(super) fn reborrow(
        &self,
        destination: SourceAnchor,
        sharing: bool,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reservation: DescriptorReservation,
    ) -> Result<Self, ExecutionError> {
        let target = ViewTarget {
            identity: self.descriptor.identity,
            source: destination,
            scope: self.descriptor.scope.reborrow(control)?,
            sharing,
        };
        self.reborrow_as(target, admission, control, reservation)
    }

    pub(super) fn reborrow_as(
        &self,
        target: ViewTarget,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reservation: DescriptorReservation,
    ) -> Result<Self, ExecutionError> {
        self.validate(control)?;
        if target.identity.program != self.descriptor.identity.program
            || target.source.program != self.descriptor.identity.program
        {
            return Err(reject());
        }
        target.scope.within(&self.descriptor.scope, control)?;
        admit(
            admission,
            AdmissionRequest::Reborrow {
                source: self,
                target: &target,
            },
            control,
        )?;
        let descriptor = reservation.allocate(
            ViewDescriptor {
                identity: target.identity,
                source: target.source,
                scope: target.scope,
            },
            control,
        )?;
        let guard = self.guard(control)?;
        Ok(Self {
            descriptor,
            lease: guard.lease,
            provenance: guard.provenance,
            shared: self.shared || target.sharing,
        })
    }

    fn descriptor(
        &self,
        field: FieldId,
        control: &ExecutionControl,
    ) -> Result<(usize, FieldDescriptor), ExecutionError> {
        self.validate(control)?;
        let state = self.lease.backing()?.lock();
        for (index, candidate) in state
            .ready
            .as_ref()
            .ok_or_else(reject)?
            .fields
            .iter()
            .enumerate()
        {
            control.check()?;
            if candidate.descriptor.field == field {
                return Ok((index, candidate.descriptor.clone()));
            }
        }
        Err(reject())
    }

    pub(super) fn metadata(
        &self,
        field: FieldId,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
    ) -> Result<MetadataRead, ExecutionError> {
        let (index, descriptor) = self.descriptor(field, control)?;
        if descriptor.kind != FieldKind::Metadata {
            return Err(reject());
        }
        let value = {
            let state = self.lease.backing()?.lock();
            control.check()?;
            match &state
                .ready
                .as_ref()
                .ok_or_else(reject)?
                .fields
                .get(index)
                .ok_or_else(reject)?
                .value
            {
                StoredValue::Metadata(value) => Arc::clone(value),
                _ => return Err(reject()),
            }
        };
        admit(
            admission,
            AdmissionRequest::MetadataResult {
                source: self,
                field,
                ty: descriptor.ty,
                value: &value,
            },
            control,
        )?;
        self.validate(control)?;
        Ok(MetadataRead {
            value,
            guard: self.guard(control)?,
            ty: descriptor.ty,
        })
    }

    pub(super) fn project_owned(
        &self,
        field: FieldId,
        destination: SourceAnchor,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
    ) -> Result<ProjectedOwnedRead, ExecutionError> {
        let (index, descriptor) = self.descriptor(field, control)?;
        if descriptor.kind != FieldKind::OwnedReference
            || destination.program != self.descriptor.identity.program
        {
            return Err(reject());
        }
        admit(
            admission,
            AdmissionRequest::Projection {
                source: self,
                field: &descriptor,
                destination: &destination,
                target: None,
            },
            control,
        )?;
        let guard = self.guard(control)?;
        let mut value = {
            let state = self.lease.backing()?.lock();
            control.check()?;
            match &state
                .ready
                .as_ref()
                .ok_or_else(reject)?
                .fields
                .get(index)
                .ok_or_else(reject)?
                .value
            {
                StoredValue::Owned(value) => value.memory_borrow()?,
                _ => return Err(reject()),
            }
        };
        adopt_read(
            &mut value,
            descriptor.source.as_ref().ok_or_else(reject)?.origin,
            destination.origin,
        )?;
        control.check()?;
        guard.validate(control)?;
        Ok(ProjectedOwnedRead {
            value: Some(value),
            guard,
            destination,
            ty: descriptor.ty,
        })
    }

    pub(super) fn project_view_as(
        &self,
        field: FieldId,
        target: ViewTarget,
        admission: &impl ReadViewAdmission,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<ReadView, ExecutionError> {
        let (index, descriptor) = self.descriptor(field, control)?;
        if descriptor.kind != FieldKind::ViewReference
            || target.source.program != self.descriptor.identity.program
            || target.identity.program != self.descriptor.identity.program
        {
            return Err(reject());
        }
        target.scope.within(&self.descriptor.scope, control)?;
        admit(
            admission,
            AdmissionRequest::Projection {
                source: self,
                field: &descriptor,
                destination: &target.source,
                target: Some(&target),
            },
            control,
        )?;
        // Snapshot the pointer without minting permission; the existing handle keeps the
        // containing cell alive. Validation/acquisition happen after its lock is released.
        let observed = {
            let state = self.lease.backing()?.lock();
            control.check()?;
            match &state
                .ready
                .as_ref()
                .ok_or_else(reject)?
                .fields
                .get(index)
                .ok_or_else(reject)?
                .value
            {
                StoredValue::View(guard) => Arc::clone(guard.lease.backing()?),
                _ => return Err(reject()),
            }
        };
        // This reserved provenance cell retains the complete containing/source packet. A
        // projection from a projection queues earlier provenance cells instead of dropping
        // an arbitrarily deep per-handle guard chain recursively.
        let mut links = reserved_vec(1, control, reserve)?;
        charge_box::<ReadyCleanup>(control, reserve)?;
        charge::<Backing>(1, control, reserve)?;
        let scope = target.scope.reborrow(control)?;
        let reservation = ReadView::reserve_descriptor(control, reserve)?;
        let descriptor = reservation.allocate(
            ViewDescriptor {
                identity: target.identity,
                source: target.source,
                scope: target.scope,
            },
            control,
        )?;
        let root = {
            let source_backing = self.provenance.as_ref().unwrap_or(&self.lease).backing()?;
            let state = source_backing.lock();
            Arc::clone(
                &state
                    .ready
                    .as_ref()
                    .ok_or_else(reject)?
                    .custody
                    .as_ref()
                    .ok_or_else(reject)?
                    .root,
            )
        };
        links.push(self.guard(control)?);
        // Build the ready node before acquiring the final field lease. Cancellation/failure
        // after this point still owns a node that can drain all accumulated guards.
        let mut pending = PendingCleanup(Some(Box::new(ReadyCleanup {
            fields: Vec::new(),
            custody: Some(CustodyOnly {
                root,
                links,
                reads: Vec::new(),
            }),
            next: None,
            #[cfg(test)]
            finalized: None,
        })));
        let mut state = observed.lock();
        observed.scope.validate(control)?;
        state
            .ready
            .as_ref()
            .ok_or_else(reject)?
            .custody
            .as_ref()
            .ok_or_else(reject)?
            .validate(observed.identity.program, control)?;
        if state.leases == 0 {
            return Err(reject());
        }
        state.leases = state.leases.checked_add(1).ok_or_else(capacity)?;
        drop(state);
        let lease = Lease {
            backing: Some(Arc::clone(&observed)),
            live: true,
        };
        control.check()?;
        let provenance = Arc::new(Backing {
            identity: observed.identity,
            source: self.descriptor.source.clone(),
            scope,
            state: Mutex::new(State {
                leases: 1,
                ready: pending.0.take(),
            }),
        });
        Ok(ReadView {
            descriptor,
            lease,
            provenance: Some(Lease {
                backing: Some(provenance),
                live: true,
            }),
            shared: self.shared || target.sharing,
        })
    }
}

/// Ordinary metadata is immutable and shared physically. Copying/materializing the raw value
/// is an evaluator operation with its existing complete admission/allocation accounting;
/// storage does not perform an uncharged recursive `NormalizedValue::clone`.
pub(super) struct MetadataRead {
    value: Arc<NormalizedValue>,
    guard: Guard,
    ty: TypeObjectDigest,
}

impl MetadataRead {
    pub(super) fn ty(&self) -> TypeObjectDigest {
        self.ty
    }
    pub(super) fn raw(
        &self,
        control: &ExecutionControl,
    ) -> Result<&NormalizedValue, ExecutionError> {
        self.guard.validate(control)?;
        Ok(&self.value)
    }
}

impl Drop for MetadataRead {
    fn drop(&mut self) {
        let mut work = Cleanup::default();
        self.guard.release_into(&mut work);
        work.drain();
    }
}

/// An Owned ReadFrom(view) result carries both the delegated token and its descriptor guard.
/// Evaluator adapters keep this packet intact until installing an equivalent checked ancestor
/// guard; extracting the raw token alone is deliberately unavailable.
pub(super) struct ProjectedOwnedRead {
    value: Option<NormalizedValue>,
    guard: Guard,
    destination: SourceAnchor,
    ty: TypeObjectDigest,
}

impl ProjectedOwnedRead {
    pub(super) fn ty(&self) -> TypeObjectDigest {
        self.ty
    }
    pub(super) fn source(&self) -> &SourceAnchor {
        &self.destination
    }
    /// Observe the exact retained parent scope before constructing a view from this Owned
    /// read. This does not mint a lease or expose custody; controlled scope operations and
    /// `raw` still validate lifetime/token liveness before granting any access.
    pub(super) fn scope(&self) -> Result<&RuntimeScopeLease, ExecutionError> {
        self.guard.lease.backing()?;
        if let Some(provenance) = &self.guard.provenance {
            provenance.backing()?;
        }
        Ok(&self.guard.scope)
    }
    pub(super) fn raw(
        &self,
        control: &ExecutionControl,
    ) -> Result<&NormalizedValue, ExecutionError> {
        self.guard.validate(control)?;
        let value = self.value.as_ref().ok_or_else(reject)?;
        validate_read(
            value,
            self.destination.origin,
            Some(self.destination.program),
            control,
        )?;
        Ok(value)
    }
}

impl Drop for ProjectedOwnedRead {
    fn drop(&mut self) {
        // Release the descendant read before the parent backing/provenance closure.
        drop(self.value.take());
        let mut work = Cleanup::default();
        self.guard.release_into(&mut work);
        work.drain();
    }
}

fn validate_construction(
    construction: &ViewConstruction,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    control.check()?;
    bounded_items(construction.fields)?;
    if construction.identity.program != construction.source.program
        || construction.identity.program != construction.scope.source.program
    {
        return Err(reject());
    }
    construction.scope.validate(control)
}

fn validate_read(
    value: &NormalizedValue,
    origin: ValueOrigin,
    program: Option<ValueOrigin>,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    control.check()?;
    if !value.memory_is_borrowed() {
        return Err(reject());
    }
    value.memory_validate(origin, false)?;
    if let Some(program) = program {
        control.check()?;
        match value {
            NormalizedValue::ByteBuffer(token) => token.validate_admission(program),
            NormalizedValue::OwnedI64Cell(token) => token.validate_admission(program),
            NormalizedValue::OwnedProduct(token) => token.validate_admission(program),
            NormalizedValue::OwnedChoice(token) => token.validate_admission(program),
            NormalizedValue::OwnedSequence(token) => token.validate_admission(program),
            _ => Err(reject()),
        }?;
    }
    Ok(())
}

fn adopt_read(
    value: &mut NormalizedValue,
    source: ValueOrigin,
    destination: ValueOrigin,
) -> Result<(), ExecutionError> {
    match value {
        NormalizedValue::ByteBuffer(token) => token.adopt_scoped_read(source, destination),
        NormalizedValue::OwnedI64Cell(token) => token.adopt_scoped_read(source, destination),
        NormalizedValue::OwnedProduct(token) => token.adopt_scoped_read(source, destination),
        NormalizedValue::OwnedChoice(token) => token.adopt_scoped_read(source, destination),
        NormalizedValue::OwnedSequence(token) => token.adopt_scoped_read(source, destination),
        _ => Err(reject()),
    }
}

fn admit(
    admission: &impl ReadViewAdmission,
    request: AdmissionRequest<'_>,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    control.check()?;
    admission.validate(request, control)?;
    control.check()
}

fn bounded_items(count: usize) -> Result<(), ExecutionError> {
    if u64::try_from(count).map_err(|_| capacity())? > MAXIMUM_ADMISSION_ITEMS {
        return Err(capacity());
    }
    Ok(())
}

fn reserved_vec<T>(
    count: usize,
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<Vec<T>, ExecutionError> {
    bounded_items(count)?;
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or_else(capacity)?;
    control.check()?;
    reserve(bytes)?;
    control.check()?;
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| capacity())?;
    Ok(values)
}

fn grow_reserved<T>(
    values: &mut Vec<T>,
    required: usize,
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    control.check()?;
    bounded_items(required)?;
    if required <= values.capacity() {
        return Ok(());
    }
    let maximum = usize::try_from(MAXIMUM_ADMISSION_ITEMS).map_err(|_| capacity())?;
    let geometric = values
        .capacity()
        .checked_mul(2)
        .unwrap_or(maximum)
        .min(maximum);
    let target = required.max(geometric);
    // Charge the whole new allocation while the old vector is still retained. This does not
    // hide reallocation scratch behind an incremental live-byte receipt.
    let mut replacement = reserved_vec(target, control, reserve)?;
    while !values.is_empty() {
        if let Err(error) = control.check() {
            replacement.reverse();
            // Original capacity still holds the complete original count. Rollback allocates
            // nothing and must finish even when the caller has cancelled.
            values.append(&mut replacement);
            return Err(error);
        }
        replacement.push(values.pop().expect("nonempty source vector"));
    }
    replacement.reverse();
    *values = replacement;
    Ok(())
}

fn charge<T>(
    count: usize,
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    // Arc storage includes its two strong/weak counters; the referenced T may include Mutex.
    let alignment = std::mem::align_of::<T>().max(std::mem::align_of::<usize>());
    let header = 2 * std::mem::size_of::<usize>();
    let offset = header
        .checked_add(std::mem::align_of::<T>() - 1)
        .map(|bytes| bytes & !(std::mem::align_of::<T>() - 1))
        .ok_or_else(capacity)?;
    let allocation = offset
        .checked_add(std::mem::size_of::<T>())
        .and_then(|bytes| bytes.checked_add(alignment - 1))
        .map(|bytes| bytes & !(alignment - 1))
        .ok_or_else(capacity)?;
    let bytes = allocation
        .checked_mul(count)
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or_else(capacity)?;
    control.check()?;
    reserve(bytes)?;
    control.check()
}

fn charge_box<T>(
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    control.check()?;
    reserve(u64::try_from(std::mem::size_of::<T>()).map_err(|_| capacity())?)?;
    control.check()
}

fn reject() -> ExecutionError {
    ExecutionError::resource(
        "normalized_read_view_token",
        "read-view identity, source scope, invocation, field, or live read authority is invalid",
    )
}

fn capacity() -> ExecutionError {
    ExecutionError::resource(
        "normalized_read_view_capacity",
        "read-view custody or descriptor storage capacity was exhausted",
    )
}
