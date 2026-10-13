//! Mechanism tests with an explicit test admission adapter, not canonical/loader admission.
//! The real evaluators must independently derive the facts this adapter supplies.

use super::*;
use crate::platform::execution::ExecutionFailureClass;
use crate::platform::kernel::{
    DeclarationReference, PackageId, ReadScopeBinder, ReadScopeReference,
};
use crate::platform::semantic_id::{AssociatedTypeMemberId, DeclarationId, ReadScopeParameterId};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

fn fresh() -> ValueOrigin {
    ValueOrigin::fresh().expect("runtime identity capacity")
}
fn field(byte: u8) -> FieldId {
    FieldId::from_bytes([byte; 16]).expect("field identity")
}
fn declaration(byte: u8) -> DeclarationReference {
    DeclarationReference {
        package: PackageId::from_bytes([byte; 16]).expect("package identity"),
        declaration: DeclarationId::from_bytes([byte; 16]).expect("declaration identity"),
    }
}
fn deny() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Trap,
        "test_view_admission",
        "independent test context refuses this request",
    )
}

#[derive(Clone, Copy)]
struct DeclaredField {
    field: FieldId,
    ty: TypeObjectDigest,
    kind: FieldKind,
}

#[derive(Clone)]
struct TestAdmission {
    identity: ViewIdentity,
    source: SourceAnchor,
    scope: CanonicalScopeKey,
    selected_allocation: usize,
    layout: Vec<DeclaredField>,
    destination: Option<SourceAnchor>,
    narrowed: Option<(ViewIdentity, CanonicalScopeKey)>,
    related: Option<ViewIdentity>,
    readable_share: bool,
}

impl TestAdmission {
    fn effective(
        &self,
        identity: ViewIdentity,
        scope: CanonicalScopeKey,
    ) -> Result<(), ExecutionError> {
        if ((identity == self.identity || self.related == Some(identity)) && scope == self.scope)
            || self.narrowed == Some((identity, scope))
        {
            Ok(())
        } else {
            Err(deny())
        }
    }

    fn destination(&self, destination: &SourceAnchor) -> Result<(), ExecutionError> {
        if destination.program != self.identity.program
            || self.destination.as_ref().unwrap_or(&self.source) != destination
        {
            return Err(deny());
        }
        Ok(())
    }

    fn construction(&self, construction: &ViewConstruction) -> Result<(), ExecutionError> {
        if construction.identity != self.identity
            || construction.source != self.source
            || construction.scope.key() != self.scope
            || construction.scope.source() != &self.source
            || construction.fields != self.layout.len()
        {
            return Err(deny());
        }
        Ok(())
    }

    fn declared(
        &self,
        field: FieldId,
        ty: TypeObjectDigest,
        kind: FieldKind,
    ) -> Result<(), ExecutionError> {
        if !self
            .layout
            .iter()
            .any(|entry| entry.field == field && entry.ty == ty && entry.kind == kind)
        {
            return Err(deny());
        }
        Ok(())
    }
}

impl ReadViewAdmission for TestAdmission {
    fn validate(
        &self,
        request: AdmissionRequest<'_>,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        match request {
            AdmissionRequest::Scope {
                key,
                source,
                parent,
            } => {
                if (key != self.scope && self.narrowed.is_none_or(|(_, scope)| scope != key))
                    || source != &self.source
                {
                    return Err(deny());
                }
                if parent.is_some_and(|parent| parent.key() != self.scope) {
                    return Err(deny());
                }
            }
            AdmissionRequest::Source {
                construction,
                loans,
            } => {
                self.construction(construction)?;
                match &loans.reads()[0].value {
                    NormalizedValue::OwnedI64Cell(token)
                        if token.allocation_identity() == self.selected_allocation => {}
                    _ => return Err(deny()),
                }
            }
            AdmissionRequest::Subview {
                construction,
                source,
            } => {
                self.construction(construction)?;
                self.effective(source.identity(), source.scope()?.key())?;
                if source.source() != &self.source {
                    return Err(deny());
                }
            }
            AdmissionRequest::Field {
                construction,
                input,
            } => {
                self.construction(construction)?;
                match input {
                    FieldAdmission::Metadata { field, ty, value } => {
                        self.declared(field, ty, FieldKind::Metadata)?;
                        if !matches!(value, NormalizedValue::I64(_) | NormalizedValue::Unit) {
                            return Err(deny());
                        }
                    }
                    FieldAdmission::Owned {
                        field,
                        ty,
                        source,
                        value,
                    } => {
                        self.declared(field, ty, FieldKind::OwnedReference)?;
                        if source != &self.source {
                            return Err(deny());
                        }
                        match value {
                            NormalizedValue::OwnedI64Cell(token)
                                if token.allocation_identity() == self.selected_allocation => {}
                            _ => return Err(deny()),
                        }
                    }
                    FieldAdmission::View {
                        field,
                        ty,
                        source,
                        value,
                    } => {
                        self.declared(field, ty, FieldKind::ViewReference)?;
                        if source != &self.source
                            || value.identity().ty != ty
                            || value.identity().program != self.identity.program
                            || value.source() != &self.source
                        {
                            return Err(deny());
                        }
                    }
                }
            }
            AdmissionRequest::Finish {
                construction,
                fields,
            } => {
                self.construction(construction)?;
                if fields.len() != self.layout.len() {
                    return Err(deny());
                }
                for (actual, expected) in fields.iter().zip(&self.layout) {
                    control.check()?;
                    let actual = actual.descriptor();
                    if actual.field != expected.field
                        || actual.ty != expected.ty
                        || actual.kind != expected.kind
                    {
                        return Err(deny());
                    }
                }
            }
            AdmissionRequest::Retain {
                construction,
                guard,
            } => {
                self.construction(construction)?;
                if guard.identity() != self.identity || guard.source() != &self.source {
                    return Err(deny());
                }
            }
            AdmissionRequest::RetainProjected {
                construction,
                guard,
            } => {
                self.construction(construction)?;
                if guard.source() != &self.source || guard.ty() != owned_field().ty {
                    return Err(deny());
                }
            }
            AdmissionRequest::RetainRead { construction, read } => {
                self.construction(construction)?;
                if read.origin != self.source.origin {
                    return Err(deny());
                }
                match &read.value {
                    NormalizedValue::OwnedI64Cell(token)
                        if token.allocation_identity() == self.selected_allocation => {}
                    _ => return Err(deny()),
                }
            }
            AdmissionRequest::Reborrow { source, target } => {
                self.effective(source.identity(), source.scope()?.key())?;
                self.effective(target.identity, target.scope.key())?;
                self.destination(&target.source)?;
                // A passive source loan is deliberately absent from this rights test. Every
                // declared accessible reference is checked, even if no projection is taken.
                for entry in &self.layout {
                    control.check()?;
                    if target.sharing && entry.kind != FieldKind::Metadata && !self.readable_share {
                        return Err(deny());
                    }
                }
            }
            AdmissionRequest::Projection {
                source,
                field,
                destination,
                target,
            } => {
                self.effective(source.identity(), source.scope()?.key())?;
                self.declared(field.field, field.ty, field.kind)?;
                self.destination(destination)?;
                if field.source.as_ref() != Some(&self.source) {
                    return Err(deny());
                }
                if source.is_shared() && !self.readable_share {
                    return Err(deny());
                }
                if let Some(target) = target {
                    self.effective(target.identity, target.scope.key())?;
                    if target.sharing && !self.readable_share {
                        return Err(deny());
                    }
                }
            }
            AdmissionRequest::MetadataResult {
                source,
                field,
                ty,
                value,
            } => {
                self.effective(source.identity(), source.scope()?.key())?;
                self.declared(field, ty, FieldKind::Metadata)?;
                if !matches!(value, NormalizedValue::I64(_) | NormalizedValue::Unit) {
                    return Err(deny());
                }
            }
        }
        Ok(())
    }
}

struct Fixture {
    cell: super::super::owned_i64_cell::OwnedI64Cell,
    scope_owner: RuntimeScopeOwner,
    scope: RuntimeScopeLease,
    admission: TestAdmission,
    control: ExecutionControl,
}

impl Fixture {
    fn new(layout: Vec<DeclaredField>) -> Self {
        let program = fresh();
        let origin = fresh();
        let cell = super::super::owned_i64_cell::OwnedI64Cell::new(origin, 73);
        cell.establish_admission(program).expect("prepared scalar");
        let source = SourceAnchor {
            program,
            application: fresh(),
            invocation: fresh(),
            coordinate: SourceCoordinate::Parameter(0),
            origin,
            path: Arc::from([]),
        };
        let scope_key = CanonicalScopeKey::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration(1)),
            parameter: ReadScopeParameterId::from_bytes([2; 16]).expect("scope identity"),
        });
        let admission = TestAdmission {
            identity: ViewIdentity {
                ty: TypeObjectDigest::from_bytes([3; 32]),
                program,
            },
            source,
            scope: scope_key,
            selected_allocation: cell.allocation_identity(),
            layout,
            destination: None,
            narrowed: None,
            related: None,
            readable_share: true,
        };
        let control = ExecutionControl::uncancelled();
        let (scope_owner, scope) = RuntimeScopeOwner::open(
            scope_key,
            &admission.source,
            None,
            &admission,
            &control,
            &mut |_| Ok(()),
        )
        .expect("source scope");
        Self {
            cell,
            scope_owner,
            scope,
            admission,
            control,
        }
    }

    fn construction(&self, control: &ExecutionControl) -> Result<ViewConstruction, ExecutionError> {
        Ok(ViewConstruction {
            identity: self.admission.identity,
            source: self.admission.source.clone(),
            scope: self.scope.reborrow(control)?,
            fields: self.admission.layout.len(),
        })
    }

    fn selected(&self) -> HeldRead {
        HeldRead {
            origin: self.admission.source.origin,
            value: NormalizedValue::OwnedI64Cell(self.cell.borrow().expect("source read")),
        }
    }

    fn builder(
        &self,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<ViewBuilder, ExecutionError> {
        let construction = self.construction(control)?;
        let loans = SourceLoans::new(self.selected(), 0, control, reserve)?;
        ViewBuilder::from_owned(construction, loans, 0, &self.admission, control, reserve)
    }

    fn build(
        &self,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<ReadView, ExecutionError> {
        let mut builder = self.builder(control, reserve)?;
        for entry in &self.admission.layout {
            match entry.kind {
                FieldKind::Metadata => builder.metadata(
                    entry.field,
                    entry.ty,
                    NormalizedValue::I64(19),
                    &self.admission,
                    control,
                    reserve,
                )?,
                FieldKind::OwnedReference => builder.owned_reference(
                    entry.field,
                    entry.ty,
                    self.admission.source.clone(),
                    self.selected().value,
                    &self.admission,
                    control,
                )?,
                FieldKind::ViewReference => return Err(deny()),
            }
        }
        builder.finish(&self.admission, control, reserve)
    }

    fn can_consume(&self) -> bool {
        self.cell
            .validate(self.admission.source.origin, true)
            .is_ok()
    }
}

fn owned_field() -> DeclaredField {
    DeclaredField {
        field: field(1),
        ty: TypeObjectDigest::from_bytes([4; 32]),
        kind: FieldKind::OwnedReference,
    }
}
fn metadata_field() -> DeclaredField {
    DeclaredField {
        field: field(2),
        ty: TypeObjectDigest::from_bytes([5; 32]),
        kind: FieldKind::Metadata,
    }
}
// Supply the independently declared fixture referent and actual source lifetime.
// The production boundary likewise requires an explicit effective target.
fn project_declared_view(
    fixture: &Fixture,
    source: &ReadView,
    control: &ExecutionControl,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<ReadView, ExecutionError> {
    let declared = fixture
        .admission
        .layout
        .iter()
        .find(|declared| declared.field == field(3))
        .ok_or_else(deny)?;
    if declared.kind != FieldKind::ViewReference {
        return Err(deny());
    }
    source.project_view_as(
        declared.field,
        ViewTarget {
            identity: ViewIdentity {
                ty: declared.ty,
                program: fixture.admission.identity.program,
            },
            source: fixture.admission.source.clone(),
            scope: fixture.scope.reborrow(control)?,
            sharing: source.is_shared(),
        },
        &fixture.admission,
        control,
        reserve,
    )
}

fn observe(builder: &mut ViewBuilder, counter: &Arc<AtomicUsize>) {
    builder.ready.as_mut().expect("ready cell").finalized = Some(Arc::clone(counter));
}

#[test]
fn read_view_affine_reborrows_and_inert_clones_keep_exact_source_custody() {
    let f = Fixture::new(vec![owned_field()]);
    let view = f.build(&f.control, &mut |_| Ok(())).expect("descriptor");
    let inert = view.clone();
    assert_eq!(
        inert.validate(&f.control).unwrap_err().code,
        "normalized_read_view_token"
    );
    assert!(
        inert
            .reborrow(f.admission.source.clone(), false, &f.admission, &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
            .is_err()
    );
    let read = view
        .reborrow(f.admission.source.clone(), false, &f.admission, &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .expect("explicit lease");
    assert!(!f.can_consume());
    drop(view);
    let projected = read
        .project_owned(
            field(1),
            f.admission.source.clone(),
            &f.admission,
            &f.control,
        )
        .expect("declared Self reference");
    drop(read);
    match projected.raw(&f.control).expect("guarded read") {
        NormalizedValue::OwnedI64Cell(cell) => assert_eq!(cell.read().unwrap(), 73),
        _ => panic!("scalar reference"),
    }
    assert!(!f.can_consume());
    drop(projected);
    // Physical inert clones have no semantic loan and cannot keep the source guarded.
    assert!(f.can_consume());
    assert!(inert.validate(&f.control).is_err());
}

#[test]
fn read_view_source_seals_refuse_owner_inert_alias_invocation_and_scope_mutants() {
    let f = Fixture::new(vec![]);
    let owned = super::super::owned_i64_cell::OwnedI64Cell::new(f.admission.source.origin, 73);
    owned
        .establish_admission(f.admission.identity.program)
        .unwrap();
    assert!(
        SourceLoans::new(
            HeldRead {
                origin: f.admission.source.origin,
                value: NormalizedValue::OwnedI64Cell(owned)
            },
            0,
            &f.control,
            &mut |_| Ok(())
        )
        .is_err()
    );
    let owner = HeldRead {
        origin: f.admission.source.origin,
        value: NormalizedValue::OwnedI64Cell(f.cell.clone()),
    };
    assert!(SourceLoans::new(owner, 0, &f.control, &mut |_| Ok(())).is_err());
    let mut read = f.selected();
    read.value = read.value.clone();
    assert!(SourceLoans::new(read, 0, &f.control, &mut |_| Ok(())).is_err());
    assert!(f.can_consume());

    for change in 0..5 {
        let mut construction = f.construction(&f.control).unwrap();
        match change {
            0 => construction.source.coordinate = SourceCoordinate::Parameter(1), // same allocation, distinct argument
            1 => construction.source.invocation = fresh(),
            2 => construction.source.application = fresh(),
            3 => {
                construction.source.path = Arc::from([ReadPathStep::OwnedIndex {
                    ty: TypeObjectDigest::from_bytes([4; 32]),
                    index: 0,
                }])
            }
            4 => construction.identity.ty = TypeObjectDigest::from_bytes([99; 32]),
            _ => unreachable!(),
        }
        let loans = SourceLoans::new(f.selected(), 0, &f.control, &mut |_| Ok(())).unwrap();
        assert!(
            ViewBuilder::from_owned(
                construction,
                loans,
                0,
                &f.admission,
                &f.control,
                &mut |_| Ok(())
            )
            .is_err()
        );
        assert!(f.can_consume(), "denial releases the tentative exact loan");
    }
    let foreign = super::super::owned_i64_cell::OwnedI64Cell::new(f.admission.source.origin, 73);
    foreign
        .establish_admission(f.admission.identity.program)
        .unwrap();
    let loans = SourceLoans::new(
        HeldRead {
            origin: f.admission.source.origin,
            value: NormalizedValue::OwnedI64Cell(foreign.borrow().unwrap()),
        },
        0,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    assert!(
        ViewBuilder::from_owned(
            f.construction(&f.control).unwrap(),
            loans,
            0,
            &f.admission,
            &f.control,
            &mut |_| Ok(())
        )
        .is_err()
    );
    assert!(foreign.validate(f.admission.source.origin, true).is_ok());
    let mut alias = f.admission.clone();
    alias.source.coordinate = SourceCoordinate::Parameter(1);
    let (_alias_owner, alias_scope) = RuntimeScopeOwner::open(
        alias.scope,
        &alias.source,
        None,
        &alias,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let mut construction = f.construction(&f.control).unwrap();
    construction.scope = alias_scope;
    let loans = SourceLoans::new(f.selected(), 0, &f.control, &mut |_| Ok(())).unwrap();
    assert!(
        ViewBuilder::from_owned(
            construction,
            loans,
            0,
            &f.admission,
            &f.control,
            &mut |_| Ok(())
        )
        .is_err(),
        "a legitimate equal-allocation parameter-1 scope is not parameter-0 authority"
    );
    assert!(f.can_consume());
    assert!(f.build(&f.control, &mut |_| Ok(())).is_ok());

    let mut binding = f.admission.clone();
    binding.identity.ty = TypeObjectDigest::from_bytes([8; 32]);
    binding.scope = CanonicalScopeKey::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::AssociatedBinding {
            implementation: declaration(9),
            member: AssociatedTypeMemberId::from_bytes([10; 16]).unwrap(),
        },
        parameter: ReadScopeParameterId::from_bytes([11; 16]).unwrap(),
    });
    let (_binding_owner, binding_scope) = RuntimeScopeOwner::open(
        binding.scope,
        &binding.source,
        None,
        &binding,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let loans = SourceLoans::new(f.selected(), 0, &f.control, &mut |_| Ok(())).unwrap();
    let bound = ViewBuilder::from_owned(
        ViewConstruction {
            identity: binding.identity,
            source: binding.source.clone(),
            scope: binding_scope,
            fields: 0,
        },
        loans,
        0,
        &binding,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap()
    .finish(&binding, &f.control, &mut |_| Ok(()))
    .unwrap();
    assert_eq!(bound.scope().unwrap().key(), binding.scope);
    drop(bound);
    assert!(f.can_consume());
}

#[test]
fn read_view_metadata_only_sharing_protects_source_without_exposing_it() {
    let mut f = Fixture::new(vec![metadata_field()]);
    f.admission.readable_share = false; // passive chosen source has no granted Share right
    let view = f.build(&f.control, &mut |_| Ok(())).unwrap();
    let child = view
        .reborrow(f.admission.source.clone(), true, &f.admission, &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .expect("metadata-only declaration may share");
    assert!(child.is_shared());
    assert!(!f.can_consume());
    assert!(
        child
            .project_owned(
                field(1),
                f.admission.source.clone(),
                &f.admission,
                &f.control
            )
            .is_err()
    );
    let metadata = child.metadata(field(2), &f.admission, &f.control).unwrap();
    assert_eq!(metadata.ty(), metadata_field().ty);
    assert_eq!(metadata.raw(&f.control).unwrap(), &NormalizedValue::I64(19));
    drop(child);
    drop(view);
    assert!(!f.can_consume());
    drop(metadata);
    assert!(f.can_consume());

    let mut range = Fixture::new(vec![metadata_field(), owned_field()]);
    range.admission.readable_share = false;
    let view = range.build(&range.control, &mut |_| Ok(())).unwrap();
    // No projection is executed; declared accessible Self still requires sharing.
    assert_eq!(
        view.reborrow(
            range.admission.source.clone(),
            true,
            &range.admission,
            &range.control
        , crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&range.control, &mut |_| Ok(())).unwrap())
        .unwrap_err()
        .code,
        "test_view_admission"
    );
    drop(view);
    assert!(range.can_consume());

    let mut empty = Fixture::new(vec![]);
    empty.admission.readable_share = false;
    let zero = empty.build(&empty.control, &mut |_| Ok(())).unwrap();
    let child = zero
        .reborrow(
            empty.admission.source.clone(),
            true,
            &empty.admission,
            &empty.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&empty.control, &mut |_| Ok(())).unwrap())
        .unwrap();
    assert!(
        !empty.can_consume(),
        "zero fields still retain explicit source custody"
    );
    drop(zero);
    drop(child);
    assert!(empty.can_consume());
}

#[test]
fn read_view_child_origins_do_not_rebase_stored_reference_tokens() {
    let mut f = Fixture::new(vec![owned_field()]);
    let view = f.build(&f.control, &mut |_| Ok(())).unwrap();
    let mut first = f.admission.source.clone();
    first.origin = fresh();
    first.invocation = fresh();
    first.coordinate = SourceCoordinate::Parameter(2);
    first.path = Arc::from([ReadPathStep::ViewReference {
        ty: f.admission.identity.ty,
        field: field(1),
    }]);
    f.admission.destination = Some(first.clone());
    let child = view
        .reborrow(first.clone(), true, &f.admission, &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .unwrap();
    assert_eq!(child.backing_source().unwrap(), &f.admission.source);
    let a = child
        .project_owned(field(1), first.clone(), &f.admission, &f.control)
        .unwrap();
    let mut second = first.clone();
    second.origin = fresh();
    second.invocation = fresh();
    second.coordinate = SourceCoordinate::Parameter(3);
    f.admission.destination = Some(second.clone());
    let other = view
        .reborrow(second.clone(), true, &f.admission, &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .unwrap();
    let b = other
        .project_owned(field(1), second.clone(), &f.admission, &f.control)
        .unwrap();
    assert_eq!(a.ty(), owned_field().ty);
    assert_eq!(a.source(), &first);
    assert_eq!(b.source(), &second);
    assert!(
        a.raw(&f.control)
            .unwrap()
            .memory_validate(first.origin, false)
            .is_ok()
    );
    assert!(
        a.raw(&f.control)
            .unwrap()
            .memory_validate(second.origin, false)
            .is_err()
    );
    assert!(
        b.raw(&f.control)
            .unwrap()
            .memory_validate(second.origin, false)
            .is_ok()
    );
    let state = view.lease.backing().unwrap().lock();
    match &state.ready.as_ref().unwrap().fields[0].value {
        StoredValue::Owned(value) => assert!(
            value
                .memory_validate(f.admission.source.origin, false)
                .is_ok()
        ),
        _ => panic!("stored reference"),
    }
    drop(state);
    drop(child);
    drop(other);
    drop(view);
    assert!(!f.can_consume());
    drop(a);
    assert!(!f.can_consume());
    drop(b);
    assert!(f.can_consume());
}

#[test]
fn read_view_returned_subviews_survive_parent_locals_but_not_source_scope() {
    let f = Fixture::new(vec![]);
    let parent = f.build(&f.control, &mut |_| Ok(())).unwrap();
    let child = ViewBuilder::from_view(
        f.construction(&f.control).unwrap(),
        &parent,
        0,
        &f.admission,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap()
    .finish(&f.admission, &f.control, &mut |_| Ok(()))
    .unwrap();
    drop(parent); // represents the defining frame/local ending
    assert!(child.validate(&f.control).is_ok());
    assert!(!f.can_consume());
    drop(f.scope_owner); // ending the actual source scope is a separate boundary
    assert!(child.validate(&f.control).is_err());
    drop(child);
    assert!(f.cell.validate(f.admission.source.origin, true).is_ok());
}

#[test]
fn read_view_scope_narrowing_changes_handle_type_and_guards_without_copying_backing() {
    let mut f = Fixture::new(vec![owned_field(), metadata_field()]);
    let view = f.build(&f.control, &mut |_| Ok(())).unwrap();
    let narrowed = ViewIdentity {
        ty: TypeObjectDigest::from_bytes([7; 32]),
        program: f.admission.identity.program,
    };
    let scope_key = CanonicalScopeKey::Lexical {
        callable: declaration(12),
        binding: BindingId::from_bytes([13; 16]).unwrap(),
    };
    f.admission.narrowed = Some((narrowed, scope_key));
    let (narrow_owner, narrow_scope) = RuntimeScopeOwner::open(
        scope_key,
        &f.admission.source,
        Some(&f.scope),
        &f.admission,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let read = view
        .reborrow_as(
            ViewTarget {
                identity: narrowed,
                source: f.admission.source.clone(),
                scope: narrow_scope.reborrow(&f.control).unwrap(),
                sharing: false,
            },
            &f.admission,
            &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .unwrap();
    assert_eq!(read.identity(), narrowed);
    assert_eq!(read.scope().unwrap().key(), scope_key);
    assert!(
        Arc::ptr_eq(view.lease.backing().unwrap(), read.lease.backing().unwrap()),
        "narrowing does not copy a descriptor or rebase stored tokens"
    );
    assert_eq!(view.lease.backing().unwrap().identity, f.admission.identity);
    assert!(
        read.reborrow_as(
            ViewTarget {
                identity: f.admission.identity,
                source: f.admission.source.clone(),
                scope: f.scope.reborrow(&f.control).unwrap(),
                sharing: false
            },
            &f.admission,
            &f.control
        , crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .is_err(),
        "a live longer root scope cannot widen the shorter handle"
    );
    let wrong = ViewIdentity {
        ty: TypeObjectDigest::from_bytes([99; 32]),
        program: narrowed.program,
    };
    assert_eq!(
        view.reborrow_as(
            ViewTarget {
                identity: wrong,
                source: f.admission.source.clone(),
                scope: narrow_scope.reborrow(&f.control).unwrap(),
                sharing: false
            },
            &f.admission,
            &f.control
        , crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .unwrap_err()
        .code,
        "test_view_admission"
    );
    let owned = read
        .project_owned(
            field(1),
            f.admission.source.clone(),
            &f.admission,
            &f.control,
        )
        .unwrap();
    let metadata = read.metadata(field(2), &f.admission, &f.control).unwrap();
    assert!(
        owned
            .scope()
            .unwrap()
            .same_live_scope(read.scope().unwrap()),
        "projected Owned source exposes its actual retained narrowed scope"
    );
    assert!(owned.scope().unwrap().reborrow(&f.control).is_ok());
    assert!(owned.raw(&f.control).is_ok());
    assert!(metadata.raw(&f.control).is_ok());
    drop(narrow_owner);
    assert!(read.validate(&f.control).is_err());
    assert!(
        owned.raw(&f.control).is_err(),
        "the returned Owned read retains effective narrowing"
    );
    assert!(
        owned.scope().unwrap().reborrow(&f.control).is_err(),
        "scope observation cannot mint a lease after parent expiry"
    );
    assert!(
        RuntimeScopeOwner::open(
            scope_key,
            &f.admission.source,
            Some(owned.scope().unwrap()),
            &f.admission,
            &f.control,
            &mut |_| Ok(())
        )
        .is_err(),
        "a new source scope cannot hide the expired projected parent"
    );
    assert!(metadata.raw(&f.control).is_err());
    assert!(view.validate(&f.control).is_ok());
    drop(read);
    drop(owned);
    drop(metadata);
    drop(view);
    assert!(f.can_consume());
}

#[test]
fn read_view_from_projected_owned_source_retains_descriptor_return_guards() {
    let f = Fixture::new(vec![owned_field()]);
    let finalized = Arc::new(AtomicUsize::new(0));
    let mut builder = f.builder(&f.control, &mut |_| Ok(())).unwrap();
    builder
        .owned_reference(
            field(1),
            owned_field().ty,
            f.admission.source.clone(),
            f.selected().value,
            &f.admission,
            &f.control,
        )
        .unwrap();
    observe(&mut builder, &finalized);
    let parent = builder
        .finish(&f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    let projected = parent
        .project_owned(
            field(1),
            f.admission.source.clone(),
            &f.admission,
            &f.control,
        )
        .unwrap();
    let loans = SourceLoans::new(
        HeldRead {
            origin: projected.source().origin,
            value: projected.raw(&f.control).unwrap().memory_borrow().unwrap(),
        },
        0,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let mut builder = ViewBuilder::from_owned(
        f.construction(&f.control).unwrap(),
        loans,
        1,
        &f.admission,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    builder
        .retain_projected(&projected, &f.admission, &f.control)
        .unwrap();
    builder
        .owned_reference(
            field(1),
            owned_field().ty,
            f.admission.source.clone(),
            projected.raw(&f.control).unwrap().memory_borrow().unwrap(),
            &f.admission,
            &f.control,
        )
        .unwrap();
    let child = builder
        .finish(&f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    drop(parent);
    drop(projected);
    assert_eq!(
        finalized.load(AtomicOrdering::Relaxed),
        0,
        "flattened Owned reads do not replace the containing descriptor provenance"
    );
    assert!(child.validate(&f.control).is_ok());
    assert!(!f.can_consume());
    drop(child);
    assert_eq!(finalized.load(AtomicOrdering::Relaxed), 1);
    assert!(f.can_consume());
}

#[test]
fn read_view_projected_views_retain_the_containing_and_return_provenance_packet() {
    let mut f = Fixture::new(vec![]);
    let underlying = f.build(&f.control, &mut |_| Ok(())).unwrap();
    f.admission.related = Some(underlying.identity());
    f.admission.identity.ty = TypeObjectDigest::from_bytes([6; 32]);
    f.admission.layout = vec![DeclaredField {
        field: field(3),
        ty: underlying.identity().ty,
        kind: FieldKind::ViewReference,
    }];
    let finalized = Arc::new(AtomicUsize::new(0));
    let mut builder = ViewBuilder::from_view(
        f.construction(&f.control).unwrap(),
        &underlying,
        0,
        &f.admission,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    builder
        .view_reference(
            field(3),
            f.admission.source.clone(),
            &underlying,
            &f.admission,
            &f.control,
        )
        .unwrap();
    observe(&mut builder, &finalized);
    let container = builder
        .finish(&f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    let referent = ViewIdentity {
        ty: TypeObjectDigest::from_bytes([7; 32]),
        program: f.admission.identity.program,
    };
    let scope_key = CanonicalScopeKey::Lexical {
        callable: declaration(14),
        binding: BindingId::from_bytes([15; 16]).unwrap(),
    };
    f.admission.narrowed = Some((referent, scope_key));
    let (scope_owner, scope) = RuntimeScopeOwner::open(
        scope_key,
        &f.admission.source,
        Some(&f.scope),
        &f.admission,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let narrower = container
        .project_view_as(
            field(3),
            ViewTarget {
                identity: referent,
                source: f.admission.source.clone(),
                scope,
                sharing: false,
            },
            &f.admission,
            &f.control,
            &mut |_| Ok(()),
        )
        .unwrap();
    assert_eq!(
        narrower.identity(),
        referent,
        "the referent uses its independently derived scope substitution"
    );
    assert!(Arc::ptr_eq(
        narrower.lease.backing().unwrap(),
        underlying.lease.backing().unwrap()
    ));
    drop(scope_owner);
    assert!(narrower.validate(&f.control).is_err());
    assert!(container.validate(&f.control).is_ok());
    drop(narrower);
    let projected = project_declared_view(&f, &container, &f.control, &mut |_| Ok(())).unwrap();
    assert_eq!(
        projected.identity(),
        underlying.identity(),
        "projection retains the declared referent, not the containing view type"
    );
    drop(container);
    drop(underlying);
    assert_eq!(
        finalized.load(AtomicOrdering::Relaxed),
        0,
        "the returned view keeps the containing source packet"
    );
    assert!(projected.validate(&f.control).is_ok());
    assert!(!f.can_consume());
    let forwarded = projected
        .reborrow(f.admission.source.clone(), false, &f.admission, &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
        .unwrap();
    drop(projected);
    assert_eq!(finalized.load(AtomicOrdering::Relaxed), 0);
    drop(forwarded);
    assert_eq!(finalized.load(AtomicOrdering::Relaxed), 1);
    assert!(f.can_consume());
}

#[test]
fn read_view_deep_dynamic_dag_cleanup_is_iterative_and_exactly_once() {
    const DEPTH: usize = 30_000;
    let f = Fixture::new(vec![]);
    let unrelated = super::super::owned_i64_cell::OwnedI64Cell::new(fresh(), 101);
    let finalized = Arc::new(AtomicUsize::new(0));
    let mut builder = f.builder(&f.control, &mut |_| Ok(())).unwrap();
    observe(&mut builder, &finalized);
    let mut current = builder
        .finish(&f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    let inert_root = current.clone();
    for _ in 1..DEPTH {
        let mut builder = ViewBuilder::from_view(
            f.construction(&f.control).unwrap(),
            &current,
            1,
            &f.admission,
            &f.control,
            &mut |_| Ok(()),
        )
        .unwrap();
        // A diamond repeats the same parent through a distinct provenance guard. Arc strong
        // counts alone cannot decide finalization; both semantic leases must close exactly once.
        builder
            .retain_guard(&current, &f.admission, &f.control)
            .unwrap();
        observe(&mut builder, &finalized);
        current = builder
            .finish(&f.admission, &f.control, &mut |_| Ok(()))
            .unwrap();
    }
    assert_eq!(finalized.load(AtomicOrdering::Relaxed), 0);
    drop(current);
    assert_eq!(finalized.load(AtomicOrdering::Relaxed), DEPTH);
    assert!(f.can_consume());
    assert_eq!(unrelated.read().unwrap(), 101);
    assert!(inert_root.validate(&f.control).is_err());
}

#[test]
fn read_view_last_concurrent_lease_claims_ready_cleanup_once() {
    let f = Fixture::new(vec![]);
    let finalized = Arc::new(AtomicUsize::new(0));
    let mut builder = f.builder(&f.control, &mut |_| Ok(())).unwrap();
    observe(&mut builder, &finalized);
    let view = builder
        .finish(&f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    let reads: Vec<_> = (0..16)
        .map(|_| {
            view.reborrow(f.admission.source.clone(), true, &f.admission, &f.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&f.control, &mut |_| Ok(())).unwrap())
                .unwrap()
        })
        .collect();
    drop(view);
    let barrier = Arc::new(std::sync::Barrier::new(reads.len()));
    let workers: Vec<_> = reads
        .into_iter()
        .map(|read| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                drop(read);
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(finalized.load(AtomicOrdering::Relaxed), 1);
    assert!(f.can_consume());
}

#[test]
fn read_view_partial_construction_and_complete_ancestor_packet_failures_release_loans() {
    let f = Fixture::new(vec![owned_field(), metadata_field()]);
    let mut builder = f.builder(&f.control, &mut |_| Ok(())).unwrap();
    builder
        .owned_reference(
            field(1),
            owned_field().ty,
            f.admission.source.clone(),
            f.selected().value,
            &f.admission,
            &f.control,
        )
        .unwrap();
    assert!(
        builder
            .finish(&f.admission, &f.control, &mut |_| Ok(()))
            .is_err(),
        "an omitted declared field cannot publish"
    );
    assert!(f.can_consume());
    let incomplete = SourceLoans::new(f.selected(), 1, &f.control, &mut |_| Ok(())).unwrap();
    assert!(
        ViewBuilder::from_owned(
            f.construction(&f.control).unwrap(),
            incomplete,
            0,
            &f.admission,
            &f.control,
            &mut |_| Ok(())
        )
        .is_err()
    );
    assert!(f.can_consume());
    let mut packet = SourceLoans::new(f.selected(), 1, &f.control, &mut |_| Ok(())).unwrap();
    packet.retain_ancestor(f.selected(), &f.control).unwrap();
    assert!(
        packet.retain_ancestor(f.selected(), &f.control).is_err(),
        "no unreserved guard growth"
    );
    drop(packet);
    assert!(f.can_consume());
    let mut builder = f.builder(&f.control, &mut |_| Ok(())).unwrap();
    builder
        .owned_reference(
            field(1),
            owned_field().ty,
            f.admission.source.clone(),
            f.selected().value,
            &f.admission,
            &f.control,
        )
        .unwrap();
    assert!(
        builder
            .owned_reference(
                field(1),
                owned_field().ty,
                f.admission.source.clone(),
                f.selected().value,
                &f.admission,
                &f.control
            )
            .is_err(),
        "duplicate field is refused before publication"
    );
    drop(builder);
    assert!(f.can_consume());
    drop(f.build(&f.control, &mut |_| Ok(())).unwrap());
    assert!(f.can_consume());
}

#[test]
fn read_view_shared_reservation_exact_and_one_short_preserve_failure_cleanup() {
    let f = Fixture::new(vec![metadata_field(), owned_field()]);
    let mut healthy = 0u64;
    let view = f
        .build(&f.control, &mut |bytes| {
            healthy = healthy.checked_add(bytes).unwrap();
            Ok(())
        })
        .unwrap();
    assert!(healthy > 0);
    drop(view);
    assert!(f.can_consume());
    for limit in [healthy, healthy - 1] {
        let mut observed = 0u64;
        let result = f.build(&f.control, &mut |bytes| {
            let next = observed.checked_add(bytes).ok_or_else(capacity)?;
            if next > limit {
                return Err(ExecutionError::resource(
                    "test_view_quota",
                    "shared allocation limit",
                ));
            }
            observed = next;
            Ok(())
        });
        if limit == healthy {
            drop(result.unwrap());
            assert_eq!(observed, healthy);
        } else {
            assert_eq!(result.unwrap_err().code, "test_view_quota");
        }
        assert!(f.can_consume());
    }
    // The same caller ledger spans two independent constructions; it is never reset by a
    // SourceLoans/Builder/finish phase or by a new descriptor root.
    let mut observed = 0u64;
    let mut reserve = |bytes| {
        let next = observed.checked_add(bytes).unwrap();
        if next > healthy {
            return Err(ExecutionError::resource(
                "test_view_quota",
                "shared allocation limit",
            ));
        }
        observed = next;
        Ok(())
    };
    let first = f.build(&f.control, &mut reserve).unwrap();
    assert_eq!(
        f.build(&f.control, &mut reserve).unwrap_err().code,
        "test_view_quota"
    );
    drop(first);
    assert!(f.can_consume());
}

#[test]
fn read_view_cancellation_at_every_actual_construction_phase_has_healthy_retry() {
    let f = Fixture::new(vec![metadata_field(), owned_field()]);
    let mut completed = false;
    for cutoff in 0..512 {
        let control = ExecutionControl::cancel_after_checks(cutoff);
        let result = f.build(&control, &mut |_| Ok(()));
        match result {
            Ok(view) => {
                assert!(cutoff > 0);
                drop(view);
                assert!(f.can_consume());
                completed = true;
                break;
            }
            Err(error) => {
                assert_eq!(
                    error.class,
                    ExecutionFailureClass::Cancelled,
                    "cutoff {cutoff}: {error:?}"
                );
                assert_eq!(error.code, "execution_cancelled");
                assert!(f.can_consume(), "partial loans survive cutoff {cutoff}");
            }
        }
    }
    assert!(
        completed,
        "bounded deterministic probe must reach first healthy construction"
    );
    let retry = f.build(&f.control, &mut |_| Ok(())).unwrap();
    assert!(retry.validate(&f.control).is_ok());
    drop(retry);
    assert!(f.can_consume());
}

#[test]
fn read_view_projection_quota_and_cancellation_keep_parent_packet_and_retry() {
    let mut f = Fixture::new(vec![]);
    let underlying = f.build(&f.control, &mut |_| Ok(())).unwrap();
    f.admission.related = Some(underlying.identity());
    f.admission.identity.ty = TypeObjectDigest::from_bytes([6; 32]);
    f.admission.layout = vec![DeclaredField {
        field: field(3),
        ty: underlying.identity().ty,
        kind: FieldKind::ViewReference,
    }];
    let mut builder = ViewBuilder::from_view(
        f.construction(&f.control).unwrap(),
        &underlying,
        0,
        &f.admission,
        &f.control,
        &mut |_| Ok(()),
    )
    .unwrap();
    builder
        .view_reference(
            field(3),
            f.admission.source.clone(),
            &underlying,
            &f.admission,
            &f.control,
        )
        .unwrap();
    let container = builder
        .finish(&f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    let mut healthy = 0u64;
    let result = project_declared_view(&f, &container, &f.control, &mut |bytes| {
        healthy += bytes;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        result.identity(),
        underlying.identity(),
        "healthy quota projection retains the independently declared referent"
    );
    drop(result);
    let mut remaining = healthy - 1;
    assert_eq!(
        project_declared_view(&f, &container, &f.control, &mut |bytes| {
            remaining = remaining.checked_sub(bytes).ok_or_else(|| {
                ExecutionError::resource("test_view_quota", "projected guard limit")
            })?;
            Ok(())
        })
        .unwrap_err()
        .code,
        "test_view_quota"
    );
    let mut completed = false;
    for cutoff in 0..512 {
        let control = ExecutionControl::cancel_after_checks(cutoff);
        match project_declared_view(&f, &container, &control, &mut |_| Ok(())) {
            Ok(result) => {
                assert_eq!(result.identity(), underlying.identity());
                drop(result);
                completed = true;
                break;
            }
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                assert!(container.validate(&f.control).is_ok());
                assert!(underlying.validate(&f.control).is_ok());
            }
        }
    }
    assert!(completed);
    let retry = project_declared_view(&f, &container, &f.control, &mut |_| Ok(())).unwrap();
    assert_eq!(
        retry.identity(),
        underlying.identity(),
        "retry preserves the exact declared referent"
    );
    drop(container);
    drop(underlying);
    assert!(retry.validate(&f.control).is_ok());
    drop(retry);
    assert!(f.can_consume());
}

#[test]
fn read_view_late_ancestor_growth_preserves_existing_packets_on_denial_and_cancellation() {
    let f = Fixture::new(vec![owned_field()]);
    let parent = f.build(&f.control, &mut |_| Ok(())).unwrap();
    let make_builder = || {
        let loans = SourceLoans::new(f.selected(), 0, &f.control, &mut |_| Ok(())).unwrap();
        let mut builder = ViewBuilder::from_owned(
            f.construction(&f.control).unwrap(),
            loans,
            1,
            &f.admission,
            &f.control,
            &mut |_| Ok(()),
        )
        .unwrap();
        builder
            .retain_guard(&parent, &f.admission, &f.control)
            .unwrap();
        builder
    };
    let mut measured = make_builder();
    let mut healthy = 0u64;
    measured
        .reserve_guards(1, &f.control, &mut |bytes| {
            healthy += bytes;
            Ok(())
        })
        .unwrap();
    assert!(healthy > 0);
    drop(measured);
    for limit in [healthy, healthy - 1] {
        let mut builder = make_builder();
        let mut remaining = limit;
        let result = builder.reserve_guards(1, &f.control, &mut |bytes| {
            remaining = remaining
                .checked_sub(bytes)
                .ok_or_else(|| ExecutionError::resource("test_view_quota", "late guard scratch"))?;
            Ok(())
        });
        if limit == healthy {
            result.unwrap();
            assert_eq!(remaining, 0);
        } else {
            assert_eq!(result.unwrap_err().code, "test_view_quota");
        }
        assert_eq!(
            builder
                .ready
                .as_ref()
                .unwrap()
                .custody
                .as_ref()
                .unwrap()
                .links
                .len(),
            1
        );
        assert!(parent.validate(&f.control).is_ok());
        drop(builder);
    }

    let mut completed = false;
    for cutoff in 0..512 {
        let mut builder = make_builder();
        let cancelled = ExecutionControl::cancel_after_checks(cutoff);
        let result = builder.reserve_guards(1, &cancelled, &mut |_| Ok(()));
        assert_eq!(
            builder
                .ready
                .as_ref()
                .unwrap()
                .custody
                .as_ref()
                .unwrap()
                .links
                .len(),
            1,
            "old packet prefix must survive cutoff {cutoff}"
        );
        match result {
            Ok(()) => {
                completed = true;
                drop(builder);
                break;
            }
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                assert!(parent.validate(&f.control).is_ok());
                // A failed resize restores the existing packet rather than poisoning or
                // silently shortening it. Its original builder can still publish safely.
                builder
                    .owned_reference(
                        field(1),
                        owned_field().ty,
                        f.admission.source.clone(),
                        f.selected().value,
                        &f.admission,
                        &f.control,
                    )
                    .unwrap();
                drop(
                    builder
                        .finish(&f.admission, &f.control, &mut |_| Ok(()))
                        .unwrap(),
                );
            }
        }
    }
    assert!(completed);

    let mut builder = make_builder();
    builder
        .reserve_guards(1, &f.control, &mut |_| Ok(()))
        .unwrap();
    builder
        .retain_guard(&parent, &f.admission, &f.control)
        .unwrap();
    builder
        .retain_read(f.selected(), &f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    let foreign = super::super::owned_i64_cell::OwnedI64Cell::new(f.admission.source.origin, 11);
    foreign
        .establish_admission(f.admission.identity.program)
        .unwrap();
    assert_eq!(
        builder
            .retain_read(
                HeldRead {
                    origin: f.admission.source.origin,
                    value: NormalizedValue::OwnedI64Cell(foreign.borrow().unwrap())
                },
                &f.admission,
                &f.control,
                &mut |_| Ok(())
            )
            .unwrap_err()
            .code,
        "test_view_admission"
    );
    assert!(foreign.validate(f.admission.source.origin, true).is_ok());
    assert_eq!(
        builder
            .ready
            .as_ref()
            .unwrap()
            .custody
            .as_ref()
            .unwrap()
            .reads
            .len(),
        1
    );
    builder
        .owned_reference(
            field(1),
            owned_field().ty,
            f.admission.source.clone(),
            f.selected().value,
            &f.admission,
            &f.control,
        )
        .unwrap();
    let child = builder
        .finish(&f.admission, &f.control, &mut |_| Ok(()))
        .unwrap();
    drop(parent);
    assert!(child.validate(&f.control).is_ok());
    assert!(!f.can_consume());
    drop(child);
    assert!(f.can_consume());
}

#[test]
fn read_view_compact_metadata_reservations_keep_affine_custody_separate() {
    let f = Fixture::new(vec![owned_field()]);
    let view = f.build(&f.control, &mut |_| Ok(())).unwrap();
    let inert = view.clone();
    assert!(Arc::ptr_eq(&view.descriptor, &inert.descriptor));
    assert!(inert.validate(&f.control).is_err());
    let mut requested = 0;
    let reservation = ReadView::reserve_descriptor(&f.control, &mut |bytes| {
        requested += bytes;
        Ok(())
    })
    .unwrap();
    assert!(requested > std::mem::size_of::<ReadView>() as u64);
    let leases_before = view.lease.backing().unwrap().lock().leases;
    let refused = ReadView::reserve_descriptor(&f.control, &mut |bytes| {
        assert_eq!(bytes, requested);
        if bytes > requested - 1 {
            return Err(ExecutionError::resource(
                "test_descriptor_quota",
                "one byte short",
            ));
        }
        Ok(())
    });
    match refused {
        Err(error) => {
            assert_eq!(error.class, ExecutionFailureClass::Resource);
            assert_eq!(error.code, "test_descriptor_quota");
        }
        Ok(_) => panic!("one-short descriptor reservation"),
    }
    assert_eq!(view.lease.backing().unwrap().lock().leases, leases_before);
    assert!(view.validate(&f.control).is_ok());
    let mut exact = 0;
    let exact_reservation = ReadView::reserve_descriptor(&f.control, &mut |bytes| {
        exact = bytes;
        if bytes > requested {
            return Err(deny());
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(exact, requested);
    let borrowed = view
        .reborrow(
            f.admission.source.clone(),
            false,
            &f.admission,
            &f.control,
            exact_reservation,
        )
        .unwrap();
    assert!(!Arc::ptr_eq(&view.descriptor, &borrowed.descriptor));
    assert_eq!(borrowed.identity(), view.identity());
    assert_eq!(borrowed.source(), view.source());
    let leases_before = view.lease.backing().unwrap().lock().leases;
    let metadata_before = Arc::strong_count(&view.descriptor);
    let mut completed = false;
    for cutoff in 0..512 {
        let paid = ReadView::reserve_descriptor(&f.control, &mut |bytes| {
            assert_eq!(bytes, requested);
            Ok(())
        })
        .unwrap();
        let control = ExecutionControl::cancel_after_checks(cutoff);
        match view.reborrow(
            f.admission.source.clone(),
            false,
            &f.admission,
            &control,
            paid,
        ) {
            Ok(read) => {
                read.validate(&f.control).unwrap();
                drop(read);
                completed = true;
            }
            Err(error) => {
                assert_eq!(
                    error.class,
                    ExecutionFailureClass::Cancelled,
                    "cutoff {cutoff}: {error:?}"
                );
                assert_eq!(error.code, "execution_cancelled");
            }
        }
        assert_eq!(view.lease.backing().unwrap().lock().leases, leases_before);
        assert_eq!(Arc::strong_count(&view.descriptor), metadata_before);
        assert!(view.validate(&f.control).is_ok());
        if completed {
            break;
        }
    }
    assert!(
        completed,
        "paid reborrow sweep must reach healthy completion"
    );
    let retry = view
        .reborrow(
            f.admission.source.clone(),
            false,
            &f.admission,
            &f.control,
            reservation,
        )
        .unwrap();
    assert_eq!(retry.identity(), view.identity());
    drop(retry);
    assert_eq!(view.lease.backing().unwrap().lock().leases, leases_before);
    assert_eq!(Arc::strong_count(&view.descriptor), metadata_before);
    assert!(view.validate(&f.control).is_ok());
    drop(view);
    assert!(!f.can_consume());
    drop(borrowed);
    assert!(
        f.can_consume(),
        "inert metadata aliases do not retain a semantic read"
    );
    assert!(inert.validate(&f.control).is_err());
    assert!(inert.scope().is_err());
}
