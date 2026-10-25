//! Logical package-revision identity and separate physical acceptance transport.

pub(crate) mod oracle;
mod read_views;

#[cfg(test)]
#[path = "package_transport/read_view_admission_tests.rs"]
mod read_view_admission_tests;
pub(crate) mod source;

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, FunctionEffect, OwnerKey, OwnerKind, PackageId,
    PackageInterfaceDeclarationPayload, PackageInterfaceDigest, PackageInterfaceRecord,
    PackageRevisionDigest, PackageTransportDigest, SemanticRootDigest, TypeForm, decode_root,
    semantic_state_digest_from_root,
};
use crate::platform::package_interface::{
    PackageInterfaceValidation, validate_package_interface_metered,
};
use crate::platform::persistent_map::{MapAdmission, MapRoot, MapWork};
use crate::platform::publication::RevisionCore;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::witness::{
    ValidationWitnessDigest, ValidationWitnessManifest, encode_witness_manifest,
};
use bincode::de::Decoder;
use bincode::error::DecodeError;
use bincode::{Decode, Encode};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const PACKAGE_REVISION_CONTRACT_IDENTITY: &str = "lkjscript-package-revision";
pub const PACKAGE_REVISION_CONTRACT_VERSION: u16 = crate::platform::canonical_schema::SCHEMA_EPOCH;
pub const PACKAGE_REVISION_MAGIC: [u8; 8] = *b"LKJPKR01";
pub const PACKAGE_REVISION_ENVELOPE_DOMAIN: &str = "lkjscript.package-revision-envelope.v1";
pub const PACKAGE_TRANSPORT_CONTRACT_IDENTITY: &str = "lkjscript-package-transport";
pub const PACKAGE_TRANSPORT_CONTRACT_VERSION: u16 = crate::platform::canonical_schema::SCHEMA_EPOCH;
pub const PACKAGE_TRANSPORT_MAGIC: [u8; 8] = *b"LKJPKT01";
pub const PACKAGE_TRANSPORT_ENVELOPE_DOMAIN: &str = "lkjscript.package-transport-envelope.v1";
pub const MAXIMUM_PACKAGE_REVISION_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_PACKAGE_TRANSPORT_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_PACKAGE_DEPENDENCIES: usize = 10_000;
pub const MAXIMUM_PACKAGE_CLOSURE: usize = 10_000;
pub const MAXIMUM_PACKAGE_CLOSURE_EDGES: usize = 100_000;
pub const MAXIMUM_PACKAGE_TRANSPORT_CANDIDATES: usize = 10_000;

/// Storage-independent identity for one accepted package revision.
#[derive(Clone, Debug, Encode, Eq, PartialEq)]
pub struct PackageRevision {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub package: PackageId,
    pub revision: RevisionCore,
    pub interface: PackageInterfaceDigest,
    pub dependencies: Vec<DependencyRecord>,
}

impl<Context> Decode<Context> for PackageRevision {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let contract_version = u16::decode(decoder)?;
        let graph_contract_version = u16::decode(decoder)?;
        let package = PackageId::decode(decoder)?;
        let revision = RevisionCore::decode(decoder)?;
        let interface = PackageInterfaceDigest::decode(decoder)?;
        let encoded_length = u64::decode(decoder)?;
        let dependency_count = usize::try_from(encoded_length)
            .map_err(|_| DecodeError::OutsideUsizeRange(encoded_length))?;
        if dependency_count > MAXIMUM_PACKAGE_DEPENDENCIES {
            return Err(DecodeError::OtherString(format!(
                "package revision dependency count exceeds {MAXIMUM_PACKAGE_DEPENDENCIES} before allocation"
            )));
        }
        decoder.claim_container_read::<DependencyRecord>(dependency_count)?;
        let mut dependencies = Vec::with_capacity(dependency_count);
        for _ in 0..dependency_count {
            decoder.unclaim_bytes_read(std::mem::size_of::<DependencyRecord>());
            dependencies.push(DependencyRecord::decode(decoder)?);
        }
        Ok(Self {
            contract_version,
            graph_contract_version,
            package,
            revision,
            interface,
            dependencies,
        })
    }
}

impl PackageRevision {
    pub fn encode(&self) -> Result<(PackageRevisionDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            PACKAGE_REVISION_MAGIC,
            PACKAGE_REVISION_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_PACKAGE_REVISION_BYTES,
        )?;
        Ok((PackageRevisionDigest::of(&bytes), bytes))
    }

    pub fn decode(bytes: &[u8], expected: PackageRevisionDigest) -> Result<Self, Diagnostic> {
        if PackageRevisionDigest::of(bytes) != expected {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_revision_digest",
                "package-revision bytes disagree with their exact logical digest",
            ));
        }
        let value: Self = crate::platform::packed::decode(
            bytes,
            PACKAGE_REVISION_MAGIC,
            PACKAGE_REVISION_ENVELOPE_DOMAIN,
            MAXIMUM_PACKAGE_REVISION_BYTES,
        )?;
        value.validate()?;
        Ok(value)
    }

    pub fn matches_dependency(
        &self,
        digest: PackageRevisionDigest,
        dependency: &DependencyRecord,
    ) -> Result<(), Diagnostic> {
        if digest != dependency.package_revision
            || self.package != dependency.package
            || self.revision.revision_id()? != dependency.semantic_revision
        {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_revision_dependency_binding",
                "dependency package and semantic revision disagree with the exact package revision",
            ));
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != PACKAGE_REVISION_CONTRACT_VERSION
            || !crate::platform::kernel::contract::supported_graph_contract(
                self.graph_contract_version,
            )
        {
            return Err(package_error(
                DiagnosticClass::Source,
                "package_revision_contract",
                "package revision uses a predecessor or foreign contract",
            ));
        }
        if self.revision.graph_contract_version != self.graph_contract_version {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_revision_core_contract",
                "package revision and embedded semantic revision core use different graph contracts",
            ));
        }
        let _ = self.revision.revision_id()?;
        validate_dependencies(self.package, &self.dependencies)
    }
}

/// One exact operational selection. It is never semantic package authority.
#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageTransportBinding {
    pub package_revision: PackageRevisionDigest,
    pub transport: PackageTransportDigest,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageTransport {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub package_revision: PackageRevisionDigest,
    pub semantic_root: SemanticRootDigest,
    pub validation_witness: ValidationWitnessDigest,
    pub witness: ValidationWitnessManifest,
    pub interface_owners: MapRoot,
}

impl PackageTransport {
    pub fn encode(&self) -> Result<(PackageTransportDigest, Vec<u8>), Diagnostic> {
        self.validate_local()?;
        let bytes = crate::platform::packed::encode(
            PACKAGE_TRANSPORT_MAGIC,
            PACKAGE_TRANSPORT_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_PACKAGE_TRANSPORT_BYTES,
        )?;
        Ok((PackageTransportDigest::of(&bytes), bytes))
    }

    pub fn decode(bytes: &[u8], expected: PackageTransportDigest) -> Result<Self, Diagnostic> {
        if PackageTransportDigest::of(bytes) != expected {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_transport_digest",
                "package-transport bytes disagree with their exact digest",
            ));
        }
        let value: Self = crate::platform::packed::decode(
            bytes,
            PACKAGE_TRANSPORT_MAGIC,
            PACKAGE_TRANSPORT_ENVELOPE_DOMAIN,
            MAXIMUM_PACKAGE_TRANSPORT_BYTES,
        )?;
        value.validate_local()?;
        Ok(value)
    }

    fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.contract_version != PACKAGE_TRANSPORT_CONTRACT_VERSION
            || !crate::platform::kernel::contract::supported_graph_contract(
                self.graph_contract_version,
            )
        {
            return Err(package_error(
                DiagnosticClass::Source,
                "package_transport_contract",
                "package transport uses a predecessor or foreign contract",
            ));
        }
        // Exact current witness metadata binds the transport. Independent full source
        // admission remains mandatory before the package can supply code or proof.
        let (witness_digest, _) = encode_witness_manifest(&self.witness)?;
        if witness_digest != self.validation_witness {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_transport_witness_digest",
                "package transport witness bytes disagree with their evidence digest",
            ));
        }
        Ok(())
    }
}

fn validate_dependencies(
    package: PackageId,
    dependencies: &[DependencyRecord],
) -> Result<(), Diagnostic> {
    if dependencies.len() > MAXIMUM_PACKAGE_DEPENDENCIES {
        return Err(package_error(
            DiagnosticClass::Resource,
            "package_revision_dependency_count",
            format!(
                "package revision contains more than {MAXIMUM_PACKAGE_DEPENDENCIES} dependencies"
            ),
        ));
    }
    for dependency in dependencies {
        dependency.validate_local()?;
        if dependency.package == package {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_revision_self_dependency",
                "package revision cannot depend on its own package identity",
            ));
        }
    }
    if dependencies
        .windows(2)
        .any(|pair| pair[0].package >= pair[1].package)
    {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_revision_dependency_order",
            "package-revision dependencies must be strictly ordered by package identity",
        ));
    }
    Ok(())
}

mod native_plan;
pub(crate) use native_plan::NativePackagePlanEvidence;

/// Exact immutable inputs after complete structural admission, before topology.
/// Only this module can construct an unplanned closure. It cannot confer readiness.
struct CollectedPackageRevisionInputs {
    root_revision: PackageRevision,
    revisions: BTreeMap<PackageRevisionDigest, PackageRevision>,
    dependency_edges: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageRevisionClosureValidation {
    root: PackageRevisionDigest,
    root_revision: PackageRevision,
    revisions: BTreeMap<PackageRevisionDigest, PackageRevision>,
    dependency_edges: usize,
    dependency_plan: native_plan::DependencyPlan,
}

impl PackageRevisionClosureValidation {
    pub(crate) fn revisions(&self) -> &BTreeMap<PackageRevisionDigest, PackageRevision> {
        &self.revisions
    }
}

/// Artifact loading has an independent topology reader. Loading the native
/// planner's own artifact must never recursively invoke that planner.
pub(crate) struct ArtifactPackageRevisionClosureValidation {
    pub root_revision: PackageRevision,
    pub revisions: BTreeMap<PackageRevisionDigest, PackageRevision>,
}

pub(crate) fn validate_package_revision_closure_with_control<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root: PackageRevisionDigest,
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<PackageRevisionClosureValidation, Diagnostic> {
    let collected = collect_package_revision_inputs(store, root, expected, work, control)?;
    let dependency_plan = native_plan::plan(&collected, root, control)?;
    Ok(PackageRevisionClosureValidation {
        root,
        root_revision: collected.root_revision,
        revisions: collected.revisions,
        dependency_edges: collected.dependency_edges,
        dependency_plan,
    })
}

/// Re-admit the complete immutable inputs under this operation's store budget
/// before reusing an earlier successful native plan. Projected transport fields
/// are never the authority for this comparison.
fn revalidate_package_revision_closure_with_control<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root: PackageRevisionDigest,
    expected: Option<&DependencyRecord>,
    prior: &PackageRevisionClosureValidation,
    work: &mut StoreWork,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<PackageRevisionClosureValidation, Diagnostic> {
    let collected = collect_package_revision_inputs(store, root, expected, work, control)?;
    if root != prior.root
        || collected.root_revision != prior.root_revision
        || collected.dependency_edges != prior.dependency_edges
        || collected.revisions.len() != prior.revisions.len()
    {
        return Err(planning_binding_mismatch());
    }
    for (fresh, admitted) in collected.revisions.iter().zip(&prior.revisions) {
        control
            .check()
            .map_err(crate::platform::execution::normalized::execution_diagnostic)?;
        if fresh != admitted {
            return Err(planning_binding_mismatch());
        }
    }
    control
        .check()
        .map_err(crate::platform::execution::normalized::execution_diagnostic)?;
    Ok(PackageRevisionClosureValidation {
        root,
        root_revision: collected.root_revision,
        revisions: collected.revisions,
        dependency_edges: collected.dependency_edges,
        dependency_plan: prior.dependency_plan.clone(),
    })
}

fn planning_binding_mismatch() -> Diagnostic {
    package_error(
        DiagnosticClass::Corrupt,
        "package_revision_plan_binding",
        "re-admitted exact logical closure differs from the private native planning context",
    )
}

pub(crate) fn validate_artifact_package_revision_closure<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root: PackageRevisionDigest,
    work: &mut StoreWork,
) -> Result<ArtifactPackageRevisionClosureValidation, Diagnostic> {
    let collected = collect_package_revision_inputs(
        store,
        root,
        None,
        work,
        &crate::platform::execution::ExecutionControl::uncancelled(),
    )?;
    reject_dependency_cycle(&collected.revisions)?;
    Ok(ArtifactPackageRevisionClosureValidation {
        root_revision: collected.root_revision,
        revisions: collected.revisions,
    })
}

fn collect_package_revision_inputs<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root: PackageRevisionDigest,
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<CollectedPackageRevisionInputs, Diagnostic> {
    let mut pending = VecDeque::from([root]);
    let mut revisions = BTreeMap::new();
    let mut packages = BTreeMap::new();
    let mut dependency_edges = 0_usize;
    while let Some(digest) = pending.pop_front() {
        control
            .check()
            .map_err(crate::platform::execution::normalized::execution_diagnostic)?;
        if revisions.contains_key(&digest) {
            continue;
        }
        if revisions.len() == MAXIMUM_PACKAGE_CLOSURE {
            return Err(package_error(
                DiagnosticClass::Resource,
                "package_revision_closure_count",
                format!("package-revision closure exceeds {MAXIMUM_PACKAGE_CLOSURE} objects"),
            ));
        }
        let revision = read_revision(store, digest, work)?;
        if digest == root
            && let Some(expected) = expected
        {
            revision.matches_dependency(digest, expected)?;
        }
        let semantic_revision = revision.revision.revision_id()?;
        if let Some(previous) = packages.insert(revision.package, (semantic_revision, digest))
            && previous != (semantic_revision, digest)
        {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_revision_closure_package_conflict",
                "one package identity is bound to different exact logical revisions",
            ));
        }
        dependency_edges = reserve_dependency_edges(dependency_edges, revision.dependencies.len())?;
        for dependency in &revision.dependencies {
            if let Some(previous) = packages.get(&dependency.package)
                && *previous != (dependency.semantic_revision, dependency.package_revision)
            {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "package_revision_closure_binding_conflict",
                    "package closure contains conflicting logical bindings for one package",
                ));
            }
            pending.push_back(dependency.package_revision);
        }
        revisions.insert(digest, revision);
    }
    validate_revision_edges(&revisions)?;
    let root_revision = revisions.get(&root).cloned().ok_or_else(|| {
        package_error(
            DiagnosticClass::Corrupt,
            "package_revision_closure_root",
            "validated logical package closure lost its root revision",
        )
    })?;
    Ok(CollectedPackageRevisionInputs {
        root_revision,
        revisions,
        dependency_edges,
    })
}

#[derive(Clone, Debug)]
pub(crate) struct PackageTransportClosureValidation {
    pub selections: Vec<PackageTransportBinding>,
    pub root_transport_digest: PackageTransportDigest,
    pub root_revision: PackageRevision,
    pub root_transport: PackageTransport,
    pub root_interface: PackageInterfaceValidation,
    /// Exact transitive metadata admitted with the root, excluding the root itself.
    /// Presence grants representation proofs, never authored dependency authority.
    pub representation_revisions: BTreeMap<PackageRevisionDigest, PackageRevision>,
    pub representation_interfaces: BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
    pub interface_map_work: MapWork,
    pub dependency_edges: usize,
    /// Complete immutable native planning context, separate from mutable
    /// descriptive projections above. Only exact re-admission can reuse it.
    logical: PackageRevisionClosureValidation,
}

pub(crate) fn validate_package_transport_local<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    selection: PackageTransportBinding,
    revision: &PackageRevision,
    work: &mut StoreWork,
) -> Result<(PackageTransport, PackageInterfaceValidation), Diagnostic> {
    validate_package_transport_local_admitted(
        store,
        selection,
        revision,
        work,
        &mut MapAdmission::unbounded(),
    )
}

pub(crate) fn validate_package_transport_local_admitted<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    selection: PackageTransportBinding,
    revision: &PackageRevision,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
) -> Result<(PackageTransport, PackageInterfaceValidation), Diagnostic> {
    validate_package_transport_local_metered(
        store,
        selection,
        revision,
        work,
        interface_map_admission,
        &mut |_| Ok(()),
    )
}

pub(crate) fn validate_package_transport_local_metered<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    selection: PackageTransportBinding,
    revision: &PackageRevision,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<(PackageTransport, PackageInterfaceValidation), Diagnostic> {
    if selection.package_revision != revision.encode()?.0 {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_selection_revision",
            "physical transport selection does not bind the supplied logical package revision",
        ));
    }
    let transport = read_transport(store, selection.transport, work)?;
    if transport.package_revision != selection.package_revision {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_selection_binding",
            "selected package transport binds another logical package revision",
        ));
    }
    bind_transport(store, &transport, revision, work)?;
    let interface = validate_package_interface_metered(
        revision.package,
        transport.interface_owners,
        store,
        work,
        interface_map_admission,
        visit,
    )?;
    Ok((transport, interface))
}

/// Check exact revisions and descriptive interface closure. This does not admit executable
/// source or publish readiness; installed transports additionally pass `source::admit`.
pub fn validate_package_transport_closure<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root_revision: PackageRevisionDigest,
    selections: &[PackageTransportBinding],
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    validate_package_transport_closure_admitted(
        store,
        root_revision,
        selections,
        expected,
        work,
        &mut MapAdmission::unbounded(),
    )
}

pub(crate) fn validate_package_transport_closure_admitted<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root_revision: PackageRevisionDigest,
    selections: &[PackageTransportBinding],
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    validate_package_transport_closure_admitted_with_control(
        store,
        root_revision,
        selections,
        expected,
        work,
        interface_map_admission,
        &crate::platform::execution::ExecutionControl::uncancelled(),
    )
}

pub(crate) fn validate_package_transport_closure_admitted_with_control<
    S: ImmutableObjectStore + ?Sized,
>(
    store: &S,
    root_revision: PackageRevisionDigest,
    selections: &[PackageTransportBinding],
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    validate_package_transport_closure_metered(
        store,
        root_revision,
        selections,
        expected,
        work,
        interface_map_admission,
        &mut |_| {
            control
                .check()
                .map_err(crate::platform::execution::normalized::execution_diagnostic)
        },
        control,
    )
}

/// Reuse only the operation's already admitted immutable logical closure when
/// selecting its physical transports. No process-wide admission cache is involved.
pub(crate) fn validate_package_transport_closure_planned_admitted<
    S: ImmutableObjectStore + ?Sized,
>(
    store: &S,
    root_revision: PackageRevisionDigest,
    selections: &[PackageTransportBinding],
    logical: PackageRevisionClosureValidation,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    validate_package_transport_closure_planned(
        store,
        root_revision,
        selections,
        logical,
        work,
        interface_map_admission,
        &mut |_| {
            control
                .check()
                .map_err(crate::platform::execution::normalized::execution_diagnostic)
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_package_transport_closure_metered<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root_revision: PackageRevisionDigest,
    selections: &[PackageTransportBinding],
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    let logical = validate_package_revision_closure_with_control(
        store,
        root_revision,
        expected,
        work,
        control,
    )?;
    validate_package_transport_closure_planned(
        store,
        root_revision,
        selections,
        logical,
        work,
        interface_map_admission,
        visit,
    )
}

#[allow(clippy::too_many_arguments)]
fn revalidate_package_transport_closure_metered<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root_revision: PackageRevisionDigest,
    selections: &[PackageTransportBinding],
    prior: &PackageRevisionClosureValidation,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    let logical = revalidate_package_revision_closure_with_control(
        store,
        root_revision,
        None,
        prior,
        work,
        control,
    )?;
    validate_package_transport_closure_planned(
        store,
        root_revision,
        selections,
        logical,
        work,
        interface_map_admission,
        visit,
    )
}

fn validate_package_transport_closure_planned<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root_revision: PackageRevisionDigest,
    selections: &[PackageTransportBinding],
    logical: PackageRevisionClosureValidation,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    visit(0)?;
    if logical.root != root_revision {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_plan_root",
            "transport selection does not bind the planned exact logical root",
        ));
    }
    if selections.len() != logical.revisions.len() {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_selection_count",
            "physical transport selection does not equal the logical package closure",
        ));
    }
    if selections
        .windows(2)
        .any(|pair| pair[0].package_revision >= pair[1].package_revision)
    {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_selection_order",
            "physical transport selections must be unique and ordered by logical revision",
        ));
    }
    let revisions = &logical.revisions;
    let mut transports = BTreeMap::new();
    let mut interfaces = BTreeMap::new();
    let mut interface_map_work = MapWork::default();
    for selection in selections {
        visit(0)?;
        let revision = revisions.get(&selection.package_revision).ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_transport_selection_foreign",
                "physical transport selection names a revision outside the logical closure",
            )
        })?;
        let (transport, interface) = validate_package_transport_local_metered(
            store,
            *selection,
            revision,
            work,
            interface_map_admission,
            visit,
        )?;
        add_map_work(&mut interface_map_work, interface.map_work);
        interfaces.insert(transport.package_revision, interface);
        transports.insert(transport.package_revision, (selection.transport, transport));
    }
    for interface in interfaces.values() {
        visit(interface.type_objects.len() as u64)?;
        for owner in interface.owners.values() {
            visit(crate::platform::package_interface::interface_owner_validation_visits(owner))?;
        }
    }
    validate_interface_dependencies(revisions, &interfaces, visit)?;
    let (root_transport_digest, root_transport) =
        transports.remove(&root_revision).ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_transport_closure_root",
                "validated transport closure lost its root",
            )
        })?;
    let root_revision = logical.root_revision.clone();
    let mut representation_revisions = logical.revisions.clone();
    representation_revisions.remove(&logical.root);
    let root_interface = interfaces
        .remove(&root_transport.package_revision)
        .ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_transport_root_interface",
                "validated transport closure lost its root interface",
            )
        })?;
    Ok(PackageTransportClosureValidation {
        selections: selections.to_vec(),
        root_transport_digest,
        root_revision,
        root_transport,
        root_interface,
        representation_revisions,
        representation_interfaces: interfaces,
        interface_map_work,
        dependency_edges: logical.dependency_edges,
        logical,
    })
}

fn add_map_work(total: &mut MapWork, other: MapWork) {
    total.add_observation(other);
}

fn read_revision<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    digest: PackageRevisionDigest,
    work: &mut StoreWork,
) -> Result<PackageRevision, Diagnostic> {
    let key = ObjectKey::from_digest(ObjectDomain::PackageRevision, digest.bytes());
    let bytes = store
        .read(key, MAXIMUM_PACKAGE_REVISION_BYTES, work)
        .map_err(|error| source_revision_diagnostic(store_diagnostic(error), digest))?
        .ok_or_else(|| {
            source_revision_diagnostic(
                package_error(
                    DiagnosticClass::Semantic,
                    "package_revision_missing",
                    format!("required exact package revision {digest} is not staged"),
                ),
                digest,
            )
        })?;
    PackageRevision::decode(&bytes, digest)
}

pub(crate) fn source_revision_diagnostic(
    mut diagnostic: Diagnostic,
    revision: PackageRevisionDigest,
) -> Diagnostic {
    diagnostic.message = format!(
        "exact dependency source {revision}: {}; restore original canonical material or restage this exact code-complete closure, then replan",
        diagnostic.message
    );
    diagnostic
}

fn read_transport<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    digest: PackageTransportDigest,
    work: &mut StoreWork,
) -> Result<PackageTransport, Diagnostic> {
    let key = ObjectKey::from_digest(ObjectDomain::PackageTransport, digest.bytes());
    let bytes = store
        .read(key, MAXIMUM_PACKAGE_TRANSPORT_BYTES, work)
        .map_err(store_diagnostic)?
        .ok_or_else(|| {
            package_error(
                DiagnosticClass::Semantic,
                "package_transport_missing",
                format!("required exact package transport {digest} is not staged"),
            )
        })?;
    PackageTransport::decode(&bytes, digest)
}

fn bind_transport<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    transport: &PackageTransport,
    revision: &PackageRevision,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    let root_key =
        ObjectKey::from_digest(ObjectDomain::SemanticRoot, transport.semantic_root.bytes());
    let root_bytes = store
        .read(root_key, ObjectDomain::SemanticRoot.maximum_bytes(), work)
        .map_err(store_diagnostic)?
        .ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_transport_semantic_root_missing",
                "package transport omits its exact semantic-root object",
            )
        })?;
    let root = decode_root(&root_bytes, transport.semantic_root)?;
    if root.graph_contract_version != revision.revision.graph_contract_version
        || root.graph_contract_version != transport.graph_contract_version
        || root.repository_id != revision.revision.repository_id
        || root.package_id != revision.package
    {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_semantic_root_binding",
            "physical semantic root belongs to another repository or package",
        ));
    }
    if semantic_state_digest_from_root(&root)? != revision.revision.semantic_state {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_semantic_state_binding",
            "physical semantic root and logical package revision commit to different semantic states",
        ));
    }
    if crate::platform::package_interface::package_interface_digest_for_graph(
        revision.package,
        transport.interface_owners.content_root(),
        revision.graph_contract_version,
    )? != revision.interface
    {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_interface_binding",
            "physical interface root disagrees with the logical package interface commitment",
        ));
    }
    if transport.witness.repository_id != revision.revision.repository_id
        || transport.witness.package_id != revision.package
        || transport.witness.semantic_root != transport.semantic_root
    {
        return Err(package_error(
            DiagnosticClass::Corrupt,
            "package_transport_witness_binding",
            "package revision, physical semantic root, and validation evidence do not form one binding",
        ));
    }
    Ok(())
}

fn reserve_dependency_edges(current: usize, additional: usize) -> Result<usize, Diagnostic> {
    let total = current.checked_add(additional).ok_or_else(|| {
        package_error(
            DiagnosticClass::Resource,
            "package_revision_closure_edge_count",
            "logical package dependency edge count overflowed",
        )
    })?;
    if total > MAXIMUM_PACKAGE_CLOSURE_EDGES {
        return Err(package_error(
            DiagnosticClass::Resource,
            "package_revision_closure_edge_count",
            format!(
                "logical package closure exceeds {MAXIMUM_PACKAGE_CLOSURE_EDGES} dependency edges"
            ),
        ));
    }
    Ok(total)
}

fn validate_revision_edges(
    revisions: &BTreeMap<PackageRevisionDigest, PackageRevision>,
) -> Result<(), Diagnostic> {
    for revision in revisions.values() {
        for dependency in &revision.dependencies {
            let child = revisions.get(&dependency.package_revision).ok_or_else(|| {
                package_error(
                    DiagnosticClass::Corrupt,
                    "package_revision_closure_incomplete",
                    "logical package dependency points outside the validated closure",
                )
            })?;
            child.matches_dependency(dependency.package_revision, dependency)?;
        }
    }
    Ok(())
}

fn validate_interface_dependencies(
    revisions: &BTreeMap<PackageRevisionDigest, PackageRevision>,
    interfaces: &BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<(), Diagnostic> {
    let closure = PackageInterfaceClosure {
        packages: revisions
            .iter()
            .map(|(digest, revision)| (revision.package, *digest))
            .collect(),
        interfaces,
        revisions,
    };
    let mut associated =
        crate::platform::package_interface::AssociatedInterfaceAdmission::new(visit);
    // Feature-free callers retain their existing descriptor visit admissions. Discovery is
    // cancellation-aware without changing that legacy allowance; demanded associated analysis
    // charges every observed discovery visit to its shared proof ledger before further growth.
    let mut associated_present = false;
    let mut discovery_visits = 0_usize;
    'discovery: for interface in interfaces.values() {
        for object in interface.type_objects.values() {
            associated.checkpoint()?;
            discovery_visits = discovery_visits.checked_add(1).ok_or_else(|| {
                package_error(
                    DiagnosticClass::Resource,
                    "package_transport_associated_work",
                    "associated discovery count overflow",
                )
            })?;
            if object.form.is_associated() || object.form.is_read_view() {
                associated_present = true;
                break 'discovery;
            }
        }
        for owner in interface.owners.values() {
            associated.checkpoint()?;
            discovery_visits = discovery_visits.checked_add(1).ok_or_else(|| {
                package_error(
                    DiagnosticClass::Resource,
                    "package_transport_associated_work",
                    "associated discovery count overflow",
                )
            })?;
            let present = match &owner.record {
                PackageInterfaceRecord::Declaration(declaration) => match &declaration.payload {
                    PackageInterfaceDeclarationPayload::OwnedContract(contract) => {
                        !contract.associated_types.is_empty()
                    }
                    PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                        !implementation.associated_types.is_empty()
                    }
                    PackageInterfaceDeclarationPayload::ReadView(_) => true,
                    PackageInterfaceDeclarationPayload::Function(function) => {
                        !function.read_scope_parameters.is_empty()
                            || !function.read_scope_sources.is_empty()
                            || matches!(
                                function.result.relation,
                                crate::platform::kernel::ResultRelation::ViewFrom { .. }
                            )
                    }
                    _ => false,
                },
                _ => false,
            };
            if present {
                associated_present = true;
                break 'discovery;
            }
        }
    }
    if associated_present {
        associated.charge(discovery_visits)?;
    }
    for (digest, revision) in revisions {
        let interface = interfaces.get(digest).ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_transport_interface_validation_missing",
                "validated package closure lost one package-interface result",
            )
        })?;
        if associated_present {
            closure.validate_associated_occurrences(revision, interface, &mut associated)?;
        }
        read_views::validate_revision(&closure, revision, interface, &mut associated)?;
        for ty in interface.type_objects.values() {
            if revision.graph_contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH
                && matches!(ty.form, TypeForm::OwnedSequence { .. })
            {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "kernel_sequence_generation",
                    "owned sequence package interface requires the current schema epoch",
                ));
            }
            if revision.graph_contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH
                && matches!(ty.form, TypeForm::OwnedChoice { .. })
            {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "kernel_choice_generation",
                    "owned choice package interface requires the current schema epoch",
                ));
            }
            if revision.graph_contract_version != crate::platform::canonical_schema::SCHEMA_EPOCH
                && matches!(ty.form, TypeForm::OwnedProduct { .. })
            {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "kernel_product_generation",
                    "owned product package interface requires the current schema epoch",
                ));
            }
            if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } = ty.form
            {
                let nominal = closure.require_owner(
                    revision,
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    &[
                        OwnerKind::Record,
                        OwnerKind::Variant,
                        OwnerKind::OwnedRecord,
                    ],
                    "named type",
                )?;
                if declaration.package != revision.package
                    && matches!(
                        nominal.record,
                        PackageInterfaceRecord::RepresentationDeclaration(_)
                    )
                {
                    return Err(package_error(
                        DiagnosticClass::Semantic,
                        "package_transport_interface_private_type",
                        "interface names a private representation declaration in another package",
                    ));
                }
                if nominal.kind() == OwnerKind::OwnedRecord
                    && revision.graph_contract_version
                        != crate::platform::canonical_schema::SCHEMA_EPOCH
                {
                    return Err(package_error(
                        DiagnosticClass::Semantic,
                        "kernel_owned_record_generation",
                        "nominal owned interface types require Graph 32",
                    ));
                }
            }
        }
        for owner in interface.owners.values() {
            match &owner.record {
                PackageInterfaceRecord::Declaration(declaration) => {
                    if let PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) =
                        &declaration.payload
                    {
                        closure.require_owner(
                            revision,
                            implementation.contract.package,
                            OwnerKey::Declaration(implementation.contract.declaration),
                            &[OwnerKind::OwnedContract],
                            "owned implementation contract",
                        )?;
                        for method in &implementation.methods {
                            let omitted_local = method.function.package == revision.package
                                && !interface.owners.contains_key(&OwnerKey::Declaration(
                                    method.function.declaration,
                                ));
                            // The earlier mapped-scope proof checks all visible obligations.
                            // Omitted private local targets require complete canonical source
                            // admission before this descriptive closure can become ready.
                            if !omitted_local {
                                closure.require_owner(
                                    revision,
                                    method.function.package,
                                    OwnerKey::Declaration(method.function.declaration),
                                    &[OwnerKind::PureFunction, OwnerKind::TaskFunction],
                                    "owned implementation method",
                                )?;
                            }
                        }
                    }
                    if let PackageInterfaceDeclarationPayload::OwnedContract(contract) =
                        &declaration.payload
                    {
                        for method in &contract.methods {
                            for requirement in method.effect.row().requirements {
                                closure.require_owner(
                                    revision,
                                    requirement.package(),
                                    requirement.owner(),
                                    &[OwnerKind::Requirement],
                                    "owned method requirement",
                                )?;
                            }
                        }
                    }
                    let PackageInterfaceDeclarationPayload::Function(signature) =
                        &declaration.payload
                    else {
                        continue;
                    };
                    for parameter in &signature.implementation_parameters {
                        closure.require_owner(
                            revision,
                            parameter.contract.package,
                            OwnerKey::Declaration(parameter.contract.declaration),
                            &[OwnerKind::OwnedContract],
                            "implementation parameter contract",
                        )?;
                    }
                    let FunctionEffect::Task {
                        effect_parameters: _,
                        requirements,
                    } = &signature.effect
                    else {
                        continue;
                    };
                    for requirement in requirements {
                        closure.require_owner(
                            revision,
                            requirement.package(),
                            requirement.owner(),
                            &[if requirement.concrete().is_some() {
                                OwnerKind::Requirement
                            } else {
                                OwnerKind::RequirementParameter
                            }],
                            "task function requirement",
                        )?;
                    }
                }
                PackageInterfaceRecord::Requirement(requirement) => validate_requirement_contract(
                    &closure,
                    revision,
                    requirement.interface,
                    &requirement.operations,
                )?,
                PackageInterfaceRecord::RequirementParameter(parameter) => {
                    validate_requirement_contract(
                        &closure,
                        revision,
                        parameter.constraint.interface,
                        &parameter.constraint.operations,
                    )?
                }
                PackageInterfaceRecord::TypeParameter(_)
                | PackageInterfaceRecord::RepresentationDeclaration(_)
                | PackageInterfaceRecord::EffectParameter(_)
                | PackageInterfaceRecord::Field(_)
                | PackageInterfaceRecord::Case(_)
                | PackageInterfaceRecord::Operation(_)
                | PackageInterfaceRecord::Parameter(_)
                | PackageInterfaceRecord::Port(_) => {}
            }
        }
    }
    Ok(())
}

fn validate_requirement_contract(
    closure: &PackageInterfaceClosure<'_>,
    revision: &PackageRevision,
    interface: crate::platform::kernel::DeclarationReference,
    operations: &[crate::platform::kernel::OperationReference],
) -> Result<(), Diagnostic> {
    let interface_owner = closure.require_owner(
        revision,
        interface.package,
        OwnerKey::Declaration(interface.declaration),
        &[OwnerKind::Interface],
        "requirement interface",
    )?;
    if !matches!(
        interface_owner.record,
        PackageInterfaceRecord::Declaration(ref declaration)
            if matches!(
                declaration.payload,
                PackageInterfaceDeclarationPayload::Interface { .. }
            )
    ) {
        return Err(package_error(
            DiagnosticClass::Semantic,
            "package_transport_interface_requirement_kind",
            "requirement interface does not name an interface declaration payload",
        ));
    }
    for operation in operations {
        if operation.package != interface.package {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_transport_interface_operation_package",
                "requirement interface and operation belong to different packages",
            ));
        }
        let operation_owner = closure.require_owner(
            revision,
            operation.package,
            OwnerKey::Operation(operation.operation),
            &[OwnerKind::Operation],
            "requirement operation",
        )?;
        let PackageInterfaceRecord::Operation(operation_record) = &operation_owner.record else {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_transport_interface_operation_variant",
                "validated operation kind disagrees with its interface record",
            ));
        };
        if operation_record.declaration != interface.declaration {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_transport_interface_operation_owner",
                "requirement operation does not belong to its exact interface declaration",
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_package_interface_closure(
    revisions: &BTreeMap<PackageRevisionDigest, PackageRevision>,
    interfaces: &BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
) -> Result<(), Diagnostic> {
    // This legacy structural entry has no caller allowance. The new associated proof still
    // enforces its own finite work/storage bound; metered transport admission shares visits.
    validate_interface_dependencies(revisions, interfaces, &mut |_| Ok(()))
}

struct PackageInterfaceClosure<'a> {
    packages: BTreeMap<PackageId, PackageRevisionDigest>,
    interfaces: &'a BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
    revisions: &'a BTreeMap<PackageRevisionDigest, PackageRevision>,
}

impl<'a> PackageInterfaceClosure<'a> {
    fn associated_owner(
        &self,
        source: &PackageRevision,
        reference: crate::platform::kernel::DeclarationReference,
    ) -> Result<&'a crate::platform::package_interface::PackageInterfaceOwner, Diagnostic> {
        let version = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
        if source.revision.graph_contract_version != version {
            return Err(associated_transport_error(
                "associated consuming semantic revision requires the exact current graph",
            ));
        }
        let target_digest = if reference.package == source.package {
            self.packages
                .get(&source.package)
                .copied()
                .ok_or_else(|| associated_transport_error("associated source revision is absent"))?
        } else {
            let dependency = source
                .dependencies
                .binary_search_by_key(&reference.package, |dependency| dependency.package)
                .ok()
                .map(|index| &source.dependencies[index])
                .ok_or_else(|| {
                    associated_transport_error(
                        "associated contract is outside the authored direct dependencies",
                    )
                })?;
            dependency.package_revision
        };
        let target = self
            .revisions
            .get(&target_digest)
            .ok_or_else(|| associated_transport_error("associated selected revision is absent"))?;
        if target.package != reference.package || target.revision.graph_contract_version != version
        {
            return Err(associated_transport_error(
                "associated selected declaration semantic revision requires the exact current graph",
            ));
        }
        self.require_owner(
            source,
            reference.package,
            OwnerKey::Declaration(reference.declaration),
            &[OwnerKind::OwnedContract, OwnerKind::ReadView],
            "associated contract authority",
        )
    }

    /// Complete descriptive occurrence/binding proof. Executable canonical source is still
    /// independently admitted by source::admit; descriptive evidence grants no execution rights.
    fn validate_associated_occurrences(
        &self,
        revision: &PackageRevision,
        interface: &'a PackageInterfaceValidation,
        admission: &mut crate::platform::package_interface::AssociatedInterfaceAdmission<'_>,
    ) -> Result<(), Diagnostic> {
        // Avoid new traversals for feature-free interfaces. An implementation whose foreign
        // contract has members is checked below even if its own omitted inventory is empty.
        let mut projected = false;
        for object in interface.type_objects.values() {
            admission.step()?;
            if object.form.is_associated() || object.form.is_read_view() {
                projected = true;
                break;
            }
        }
        for (key, owner) in &interface.owners {
            admission.step()?;
            let mut resolve = |reference| self.associated_owner(revision, reference).map(Some);
            if let PackageInterfaceRecord::Declaration(declaration) = &owner.record {
                match &declaration.payload {
                    PackageInterfaceDeclarationPayload::OwnedContract(contract) => {
                        for member in &contract.associated_types {
                            let contract_reference =
                                crate::platform::kernel::DeclarationReference {
                                    package: revision.package,
                                    declaration: match key {
                                        OwnerKey::Declaration(id) => *id,
                                        _ => {
                                            return Err(associated_transport_error(
                                                "associated contract has a foreign identity kind",
                                            ));
                                        }
                                    },
                                };
                            let form = match &member.bound {
                                crate::platform::kernel::AssociatedTypeBound::Owned => TypeForm::AssociatedContractMember {
                                    contract: contract_reference,
                                    member: member.id,
                                },
                                crate::platform::kernel::AssociatedTypeBound::ReadViewFamily { scope_parameter } => TypeForm::AssociatedReadViewMember {
                                    contract: contract_reference,
                                    member: member.id,
                                    read_scope: crate::platform::kernel::ReadScopeArgument::Parameter(crate::platform::kernel::ReadScopeReference {
                                        binder: crate::platform::kernel::ReadScopeBinder::AssociatedMember {
                                            contract: contract_reference,
                                            member: member.id,
                                        },
                                        parameter: scope_parameter.id,
                                    }),
                                },
                            };
                            admission.validate_form(
                                revision.package,
                                *key,
                                &form,
                                &interface.owners,
                                &interface.type_objects,
                                &mut resolve,
                            )?;
                        }
                        if !contract.associated_types.is_empty()
                            && revision.revision.graph_contract_version
                                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                        {
                            return Err(associated_transport_error(
                                "associated contract revision requires the exact current graph",
                            ));
                        }
                    }
                    PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                        let target = self.require_owner(
                            revision,
                            implementation.contract.package,
                            OwnerKey::Declaration(implementation.contract.declaration),
                            &[OwnerKind::OwnedContract],
                            "associated binding contract",
                        )?;
                        let PackageInterfaceRecord::Declaration(contract) = &target.record else {
                            return Err(associated_transport_error(
                                "associated binding contract requires public declaration authority",
                            ));
                        };
                        let PackageInterfaceDeclarationPayload::OwnedContract(contract) =
                            &contract.payload
                        else {
                            return Err(associated_transport_error(
                                "associated binding target is not an OwnedContract",
                            ));
                        };
                        if !implementation.associated_types.is_empty()
                            || !contract.associated_types.is_empty()
                        {
                            self.associated_owner(revision, implementation.contract)?;
                            admission.current(owner, *key)?;
                            admission.current(
                                target,
                                OwnerKey::Declaration(implementation.contract.declaration),
                            )?;
                            if implementation.associated_types.len()
                                != contract.associated_types.len()
                            {
                                return Err(associated_transport_error(
                                    "associated bindings do not cover the complete contract inventory",
                                ));
                            }
                            let mut previous = None;
                            for binding in &implementation.associated_types {
                                admission.step()?;
                                if previous.is_some_and(|member| member >= binding.member) {
                                    return Err(associated_transport_error(
                                        "associated bindings are not in strict member-ID order",
                                    ));
                                }
                                previous = Some(binding.member);
                                let mut selected = None;
                                for member in &contract.associated_types {
                                    admission.step()?;
                                    if member.id == binding.member {
                                        selected = Some(member);
                                    }
                                }
                                let selected = selected.ok_or_else(|| {
                                    associated_transport_error(
                                        "binding names no exact contract member",
                                    )
                                })?;
                                let ty = match &binding.value {
                                    crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty }
                                    | crate::platform::kernel::AssociatedTypeBindingValue::ReadViewFamily { ty, .. } => *ty,
                                };
                                if admission.validate_occurrences(
                                    revision.package,
                                    *key,
                                    &[ty],
                                    true,
                                    &interface.owners,
                                    &interface.type_objects,
                                    &mut resolve,
                                )? {
                                    read_views::discharge_occurrences(
                                        self,
                                        revision,
                                        *key,
                                        &[ty],
                                        interface,
                                        admission,
                                    )?;
                                }
                                match (&selected.bound, &binding.value) {
                                    (crate::platform::kernel::AssociatedTypeBound::Owned,
                                     crate::platform::kernel::AssociatedTypeBindingValue::Owned { .. }) => {
                                        if !self.owned_type(revision, ty)? {
                                            return Err(associated_transport_error("associated binding RHS lacks an Owned bound"));
                                        }
                                    }
                                    (crate::platform::kernel::AssociatedTypeBound::ReadViewFamily { .. },
                                     crate::platform::kernel::AssociatedTypeBindingValue::ReadViewFamily { scope_parameter, .. }) => {
                                        read_views::validate_binding(self, revision, *key, binding.member,
                                            scope_parameter, ty, interface, admission)?;
                                    }
                                    _ => return Err(associated_transport_error("associated binding kind/arity differs from the exact member")),
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if projected {
                admission.reserve_roots(owner)?;
                let roots = owner.type_roots();
                if admission.validate_occurrences(
                    revision.package,
                    *key,
                    &roots,
                    false,
                    &interface.owners,
                    &interface.type_objects,
                    &mut resolve,
                )? {
                    read_views::discharge_occurrences(
                        self, revision, *key, &roots, interface, admission,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn owned_type_at(
        &self,
        source: &PackageRevision,
        owner: OwnerKey,
        ty: crate::platform::kernel::TypeObjectDigest,
        admission: &mut crate::platform::package_interface::AssociatedInterfaceAdmission<'_>,
    ) -> Result<bool, Diagnostic> {
        let interface = self
            .packages
            .get(&source.package)
            .and_then(|digest| self.interfaces.get(digest))
            .ok_or_else(|| associated_transport_error("associated source interface is absent"))?;
        let object = interface
            .type_objects
            .get(&ty)
            .ok_or_else(|| associated_transport_error("associated signature type is absent"))?;
        if !object.form.is_associated() {
            return self.owned_type(source, ty);
        }
        let mut resolve = |reference| self.associated_owner(source, reference).map(Some);
        Ok(admission.validate_form(
            source.package,
            owner,
            &object.form,
            &interface.owners,
            &interface.type_objects,
            &mut resolve,
        )? == crate::platform::package_interface::AssociatedInterfaceProof::Owned)
    }

    fn owned_type(
        &self,
        source: &PackageRevision,
        ty: crate::platform::kernel::TypeObjectDigest,
    ) -> Result<bool, Diagnostic> {
        let interface = self
            .packages
            .get(&source.package)
            .and_then(|digest| self.interfaces.get(digest))
            .ok_or_else(|| {
                package_error(
                    DiagnosticClass::Corrupt,
                    "package_transport_interface_source_lost",
                    "source interface is absent",
                )
            })?;
        let object = interface.type_objects.get(&ty).ok_or_else(|| {
            package_error(
                DiagnosticClass::Semantic,
                "package_transport_interface_borrow_result",
                "borrowed signature type is absent",
            )
        })?;
        Ok(match &object.form {
            TypeForm::RegionCap {
                mode:
                    crate::platform::kernel::RegionMode::Unique
                    | crate::platform::kernel::RegionMode::Frozen
                    | crate::platform::kernel::RegionMode::Pinned,
                ..
            } => true,
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. } => true,
            TypeForm::TypeParameter { parameter } => {
                matches!(interface.owners.get(&OwnerKey::TypeParameter(*parameter)).map(|owner| &owner.record),
                Some(PackageInterfaceRecord::TypeParameter(parameter)) if parameter.constraints.has_owned())
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                self.require_owner(
                    source,
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    &[
                        OwnerKind::Record,
                        OwnerKind::Variant,
                        OwnerKind::OwnedRecord,
                    ],
                    "borrowed signature nominal type",
                )?
                .kind()
                    == OwnerKind::OwnedRecord
            }
            _ => false,
        })
    }

    fn require_owner(
        &self,
        source: &PackageRevision,
        package: PackageId,
        owner: OwnerKey,
        kinds: &[OwnerKind],
        label: &str,
    ) -> Result<&'a crate::platform::package_interface::PackageInterfaceOwner, Diagnostic> {
        let digest = if package == source.package {
            self.packages.get(&package).copied().ok_or_else(|| {
                package_error(
                    DiagnosticClass::Corrupt,
                    "package_transport_interface_source_lost",
                    "validated package interface lost its source package revision",
                )
            })?
        } else {
            let dependency = source
                .dependencies
                .binary_search_by_key(&package, |dependency| dependency.package)
                .ok()
                .map(|index| &source.dependencies[index])
                .ok_or_else(|| {
                    package_error(
                        DiagnosticClass::Semantic,
                        "package_transport_interface_dependency_missing",
                        format!(
                            "{label} names package {package} outside the direct dependency set"
                        ),
                    )
                })?;
            if self.packages.get(&package) != Some(&dependency.package_revision) {
                return Err(package_error(
                    DiagnosticClass::Corrupt,
                    "package_transport_interface_dependency_binding",
                    "interface dependency resolves to another logical package revision",
                ));
            }
            dependency.package_revision
        };
        let target = self.interfaces.get(&digest).ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_transport_interface_dependency_validation",
                "validated package lost its package-interface result",
            )
        })?;
        let value = target.owners.get(&owner).ok_or_else(|| {
            package_error(
                DiagnosticClass::Semantic,
                "package_transport_interface_owner_missing",
                format!("{label} names owner {owner:?} absent from package interface {package}"),
            )
        })?;
        if !kinds.contains(&value.kind()) {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_transport_interface_owner_kind",
                format!(
                    "{label} names package-interface owner kind {:?}",
                    value.kind()
                ),
            ));
        }
        Ok(value)
    }
}

fn associated_transport_error(message: &str) -> Diagnostic {
    package_error(
        DiagnosticClass::Semantic,
        "package_transport_associated_authority",
        message,
    )
}

fn reject_dependency_cycle(
    revisions: &BTreeMap<PackageRevisionDigest, PackageRevision>,
) -> Result<(), Diagnostic> {
    let mut indegree = revisions
        .keys()
        .copied()
        .map(|digest| (digest, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for revision in revisions.values() {
        for dependency in &revision.dependencies {
            let degree = indegree
                .get_mut(&dependency.package_revision)
                .ok_or_else(|| {
                    package_error(
                        DiagnosticClass::Corrupt,
                        "package_revision_closure_edge",
                        "package dependency points outside the validated logical closure",
                    )
                })?;
            *degree = degree.checked_add(1).ok_or_else(|| {
                package_error(
                    DiagnosticClass::Resource,
                    "package_revision_closure_indegree",
                    "package dependency indegree overflowed",
                )
            })?;
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(digest, degree)| (*degree == 0).then_some(*digest))
        .collect::<BTreeSet<_>>();
    let mut visited = 0_usize;
    while let Some(digest) = ready.pop_first() {
        visited = visited.saturating_add(1);
        for dependency in &revisions[&digest].dependencies {
            let degree = indegree
                .get_mut(&dependency.package_revision)
                .ok_or_else(|| {
                    package_error(
                        DiagnosticClass::Corrupt,
                        "package_revision_closure_topology",
                        "package dependency disappeared during cycle validation",
                    )
                })?;
            *degree -= 1;
            if *degree == 0 {
                ready.insert(dependency.package_revision);
            }
        }
    }
    if visited != revisions.len() {
        return Err(package_error(
            DiagnosticClass::Semantic,
            "package_revision_dependency_cycle",
            "logical package dependency closure contains a cycle",
        ));
    }
    Ok(())
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    package_error(class, error.code, error.message)
}

fn package_error(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::change::stage_full_authority;
    use crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    use crate::platform::kernel::{KernelSnapshot, Name, SemanticRoot, semantic_state_digest};
    use crate::platform::package_interface::build_package_interface;
    use crate::platform::persistent_map::{MapContentDigest, PageDigest};
    use crate::platform::publication::contract::REVISION_CONTRACT_VERSION;
    use crate::platform::semantic_id::{RepositoryId, RevisionId};
    use crate::platform::storage::memory::MemoryPackedStore;
    use crate::platform::storage::object::{ImmutableObjectStore, StageOutcome};

    struct Fixture {
        revision: PackageRevision,
        revision_digest: PackageRevisionDigest,
        revision_bytes: Vec<u8>,
        transport: PackageTransport,
        transport_digest: PackageTransportDigest,
        transport_bytes: Vec<u8>,
        semantic_root: SemanticRoot,
        own_objects: BTreeMap<ObjectKey, Vec<u8>>,
        closure_objects: BTreeMap<ObjectKey, Vec<u8>>,
        bindings: BTreeMap<PackageRevisionDigest, PackageTransportDigest>,
    }

    fn fixture(seed: u8, children: &[&Fixture]) -> Fixture {
        let package = PackageId::migrate(b"package-transport-test", u64::from(seed));
        let empty = MapRoot::from_parts(
            PageDigest::from_bytes([seed; 32]),
            0,
            MapContentDigest::from_bytes([seed; 32]),
        );
        let mut dependencies = children
            .iter()
            .map(|child| DependencyRecord {
                graph_contract_version: GRAPH_CONTRACT_VERSION,
                package: child.revision.package,
                semantic_revision: child
                    .revision
                    .revision
                    .revision_id()
                    .expect("child semantic revision"),
                package_revision: child.revision_digest,
            })
            .collect::<Vec<_>>();
        dependencies.sort_by_key(|dependency| dependency.package);
        let logical = KernelSnapshot {
            root: SemanticRoot {
                graph_contract_version: GRAPH_CONTRACT_VERSION,
                repository_id: RepositoryId::migrate(b"package-transport-test", u64::from(seed)),
                package_id: package,
                package_name: Name::new(format!("package_{seed}")).expect("fixture package name"),
                owners: empty,
                dependencies: MapRoot::from_parts(
                    empty.page(),
                    u64::try_from(dependencies.len()).expect("fixture dependency count"),
                    empty.content(),
                ),
                retirements: empty,
            },
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            dependency_interfaces: dependencies
                .iter()
                .map(|dependency| (dependency.package_revision, BTreeMap::new().into()))
                .collect(),
            dependency_types: BTreeMap::new(),
            representation_dependencies: BTreeMap::new(),
            blobs: BTreeMap::new(),
            dependencies: dependencies
                .iter()
                .cloned()
                .map(|dependency| (dependency.package, dependency))
                .collect(),
            retirements: BTreeMap::new(),
        };
        let mut authority_store = MemoryPackedStore::default();
        let mut authority_work = StoreWork::default();
        for child in children {
            for (key, bytes) in &child.closure_objects {
                authority_store
                    .stage(*key, bytes, &mut authority_work)
                    .expect("stage child logical closure");
            }
        }
        let staged = stage_full_authority(&logical, &mut authority_store)
            .expect("stage valid fixture authority");
        assert_eq!(
            staged.binding.semantic.state,
            semantic_state_digest(&logical).expect("fixture logical state")
        );
        let interface = build_package_interface(&BTreeMap::new(), &BTreeMap::new())
            .expect("empty package interface");
        let interface_digest = crate::platform::package_interface::package_interface_digest(
            package,
            interface.root.content_root(),
        )
        .expect("logical interface digest");
        let revision = PackageRevision {
            contract_version: PACKAGE_REVISION_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            package,
            revision: RevisionCore {
                contract_version: REVISION_CONTRACT_VERSION,
                graph_contract_version: GRAPH_CONTRACT_VERSION,
                repository_id: logical.root.repository_id,
                parents: Vec::new(),
                semantic_state: staged.binding.semantic.state,
            },
            interface: interface_digest,
            dependencies: dependencies.clone(),
        };
        let (revision_digest, revision_bytes) = revision.encode().expect("package revision");
        let transport = PackageTransport {
            contract_version: PACKAGE_TRANSPORT_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            package_revision: revision_digest,
            semantic_root: staged.binding.semantic.digest,
            validation_witness: staged.binding.witness.digest,
            witness: staged.binding.witness.manifest,
            interface_owners: interface.root,
        };
        let (transport_digest, transport_bytes) = transport.encode().expect("package transport");
        let mut own_objects = interface.objects;
        own_objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageRevision, revision_digest.bytes()),
            revision_bytes.clone(),
        );
        own_objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageTransport, transport_digest.bytes()),
            transport_bytes.clone(),
        );
        let (semantic_root_digest, semantic_root_bytes) =
            crate::platform::kernel::encode_root(&staged.binding.semantic.root)
                .expect("semantic root");
        assert_eq!(semantic_root_digest, transport.semantic_root);
        own_objects.insert(
            ObjectKey::from_digest(ObjectDomain::SemanticRoot, semantic_root_digest.bytes()),
            semantic_root_bytes,
        );
        let mut closure_objects = BTreeMap::new();
        let mut bindings = BTreeMap::new();
        for child in children {
            closure_objects.extend(child.closure_objects.clone());
            bindings.extend(child.bindings.clone());
        }
        closure_objects.extend(own_objects.clone());
        bindings.insert(revision_digest, transport_digest);
        Fixture {
            revision,
            revision_digest,
            revision_bytes,
            transport,
            transport_digest,
            transport_bytes,
            semantic_root: staged.binding.semantic.root,
            own_objects,
            closure_objects,
            bindings,
        }
    }

    fn selections(fixture: &Fixture) -> Vec<PackageTransportBinding> {
        fixture
            .bindings
            .iter()
            .map(|(package_revision, transport)| PackageTransportBinding {
                package_revision: *package_revision,
                transport: *transport,
            })
            .collect()
    }

    fn revalidated_transport(
        fixture: &Fixture,
        page_byte: u8,
    ) -> (PackageTransport, PackageTransportDigest, Vec<u8>) {
        let mut roots = fixture.transport.witness.roots;
        roots.namespaces = MapRoot::from_parts(
            PageDigest::from_bytes([page_byte; 32]),
            roots.namespaces.entries(),
            roots.namespaces.content(),
        );
        let (witness, validation_witness, _) = crate::platform::witness::bind_witness_manifest(
            fixture.transport.witness.repository_id,
            fixture.transport.witness.package_id,
            fixture.transport.semantic_root,
            roots,
        )
        .expect("revalidated witness");
        let mut transport = fixture.transport.clone();
        transport.witness = witness;
        transport.validation_witness = validation_witness;
        let (digest, bytes) = transport.encode().expect("revalidated transport");
        (transport, digest, bytes)
    }

    fn stage_objects(store: &mut MemoryPackedStore, objects: &BTreeMap<ObjectKey, Vec<u8>>) {
        let mut work = StoreWork::default();
        for (key, bytes) in objects {
            assert!(matches!(
                store
                    .stage(*key, bytes, &mut work)
                    .expect("stage fixture object"),
                StageOutcome::Inserted | StageOutcome::Reused
            ));
        }
    }

    #[test]
    fn package_revision_and_transport_round_trip_and_reject_current_counterfeits() {
        let fixture = fixture(1, &[]);
        assert_eq!(
            PackageRevision::decode(&fixture.revision_bytes, fixture.revision_digest).unwrap(),
            fixture.revision
        );
        assert_eq!(
            PackageTransport::decode(&fixture.transport_bytes, fixture.transport_digest).unwrap(),
            fixture.transport
        );
        let seal = |mut content: Vec<u8>| {
            let mut hasher = blake3::Hasher::new_derive_key(PACKAGE_REVISION_ENVELOPE_DOMAIN);
            hasher.update(&(content.len() as u64).to_be_bytes());
            hasher.update(&content);
            content.extend_from_slice(hasher.finalize().as_bytes());
            content
        };
        for (offset, replacement, expected) in [
            (0, vec![0; 8], "packed_contract"),
            (8, vec![0; 32], "packed_schema"),
        ] {
            let mut content = fixture.revision_bytes[..fixture.revision_bytes.len() - 32].to_vec();
            content[offset..offset + replacement.len()].copy_from_slice(&replacement);
            let counterfeit = seal(content);
            assert_eq!(
                PackageRevision::decode(&counterfeit, PackageRevisionDigest::of(&counterfeit))
                    .expect_err("a matching digest cannot authorize a counterfeit current frame")
                    .code,
                expected
            );
        }
        let mut invalid = fixture.revision;
        invalid.contract_version = 0;
        assert_eq!(
            invalid.encode().unwrap_err().code,
            "package_revision_contract"
        );
        let payload = bincode::encode_to_vec(
            &invalid,
            bincode::config::standard()
                .with_little_endian()
                .with_variable_int_encoding(),
        )
        .unwrap();
        let mut content =
            fixture.revision_bytes[..crate::platform::packed::CANONICAL_HEADER_BYTES].to_vec();
        content[10..18].copy_from_slice(&(payload.len() as u64).to_le_bytes());
        content.extend_from_slice(&payload);
        let counterfeit = seal(content);
        assert_eq!(
            PackageRevision::decode(&counterfeit, PackageRevisionDigest::of(&counterfeit))
                .expect_err("a current frame cannot authorize an invalid revision payload")
                .code,
            "package_revision_contract"
        );
        let mut trailing = fixture.transport_bytes;
        trailing.push(0);
        assert_eq!(
            PackageTransport::decode(&trailing, PackageTransportDigest::of(&trailing))
                .expect_err("a matching object digest cannot authorize trailing transport bytes")
                .code,
            "packed_length_mismatch"
        );
    }

    #[test]
    fn logical_revision_identity_excludes_physical_interface_and_evidence_layout() {
        let fixture = fixture(2, &[]);
        let logical_digest = fixture.revision.encode().unwrap().0;
        let (revalidated, revalidated_digest, _) = revalidated_transport(&fixture, 0xa5);
        assert_eq!(fixture.revision.encode().unwrap().0, logical_digest);
        assert_ne!(fixture.transport_digest, revalidated_digest);
        assert_eq!(
            crate::platform::package_interface::package_interface_digest(
                fixture.revision.package,
                fixture.transport.interface_owners.content_root(),
            )
            .unwrap(),
            crate::platform::package_interface::package_interface_digest(
                fixture.revision.package,
                revalidated.interface_owners.content_root(),
            )
            .unwrap()
        );
    }

    #[test]
    fn transport_closure_requires_flat_exact_selection_and_logical_binding() {
        let child = fixture(3, &[]);
        let root = fixture(4, &[&child]);
        let mut store = MemoryPackedStore::default();
        stage_objects(&mut store, &root.closure_objects);
        let mut work = StoreWork::default();
        let mut incomplete = selections(&root);
        incomplete.retain(|binding| binding.package_revision != child.revision_digest);
        assert_eq!(
            validate_package_transport_closure(
                &store,
                root.revision_digest,
                &incomplete,
                None,
                &mut work,
            )
            .unwrap_err()
            .code,
            "package_transport_selection_count"
        );
        let validated = validate_package_transport_closure(
            &store,
            root.revision_digest,
            &selections(&root),
            None,
            &mut work,
        )
        .expect("complete exact package transport closure");
        assert_eq!(validated.root_revision, root.revision);
        assert_eq!(validated.root_transport, root.transport);

        let foreign = DependencyRecord {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            package: child.revision.package,
            semantic_revision: RevisionId::from_digest([0xee; 32]),
            package_revision: child.revision_digest,
        };
        assert_eq!(
            validate_package_transport_closure(
                &store,
                child.revision_digest,
                &selections(&child),
                Some(&foreign),
                &mut work,
            )
            .unwrap_err()
            .code,
            "package_revision_dependency_binding"
        );
    }

    #[test]
    fn flat_transport_selection_accepts_diamond_with_equivalent_leaf_transport() {
        let leaf = fixture(5, &[]);
        let left = fixture(6, &[&leaf]);
        let right = fixture(7, &[&leaf]);
        let root = fixture(8, &[&left, &right]);
        let (alternate, alternate_digest, alternate_bytes) = revalidated_transport(&leaf, 0xb6);
        let mut objects = root.closure_objects.clone();
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageTransport, alternate_digest.bytes()),
            alternate_bytes,
        );
        let mut store = MemoryPackedStore::default();
        stage_objects(&mut store, &objects);
        let mut alternate_selections = selections(&root);
        let leaf_selection = alternate_selections
            .iter_mut()
            .find(|binding| binding.package_revision == leaf.revision_digest)
            .expect("leaf selection");
        leaf_selection.transport = alternate_digest;
        let mut work = StoreWork::default();
        let validated = validate_package_transport_closure(
            &store,
            root.revision_digest,
            &alternate_selections,
            None,
            &mut work,
        )
        .expect("diamond closure with alternate physical leaf");
        assert_eq!(validated.root_transport_digest, root.transport_digest);
        assert_eq!(alternate.package_revision, leaf.revision_digest);
        assert!(
            store
                .read(
                    ObjectKey::from_digest(
                        ObjectDomain::PackageTransport,
                        leaf.transport_digest.bytes(),
                    ),
                    MAXIMUM_PACKAGE_TRANSPORT_BYTES,
                    &mut StoreWork::default(),
                )
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn transport_closure_rejects_root_state_and_witness_substitution() {
        let local = fixture(5, &[]);
        let foreign = fixture(9, &[]);

        let mut root_substituted = local.transport.clone();
        root_substituted.semantic_root = foreign.transport.semantic_root;
        let (root_digest, root_bytes) = root_substituted.encode().expect("root substitution");
        let mut objects = local.closure_objects.clone();
        objects.extend(foreign.own_objects.clone());
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageTransport, root_digest.bytes()),
            root_bytes,
        );
        let mut store = MemoryPackedStore::default();
        stage_objects(&mut store, &objects);
        let mut work = StoreWork::default();
        assert_eq!(
            validate_package_transport_closure(
                &store,
                local.revision_digest,
                &[PackageTransportBinding {
                    package_revision: local.revision_digest,
                    transport: root_digest,
                }],
                None,
                &mut work,
            )
            .unwrap_err()
            .code,
            "package_transport_semantic_root_binding"
        );

        let mut witness_substituted = local.transport.clone();
        witness_substituted.witness = foreign.transport.witness.clone();
        witness_substituted.validation_witness = foreign.transport.validation_witness;
        let (witness_digest, witness_bytes) =
            witness_substituted.encode().expect("witness substitution");
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageTransport, witness_digest.bytes()),
            witness_bytes,
        );
        let mut witness_store = MemoryPackedStore::default();
        stage_objects(&mut witness_store, &objects);
        assert_eq!(
            validate_package_transport_closure(
                &witness_store,
                local.revision_digest,
                &[PackageTransportBinding {
                    package_revision: local.revision_digest,
                    transport: witness_digest,
                }],
                None,
                &mut StoreWork::default(),
            )
            .unwrap_err()
            .code,
            "package_transport_witness_binding"
        );

        let mut changed_root = local.semantic_root.clone();
        changed_root.package_name = Name::new("changed_meaning").expect("changed package name");
        let (changed_root_digest, changed_root_bytes) =
            crate::platform::kernel::encode_root(&changed_root).expect("changed semantic root");
        let (changed_witness, changed_witness_digest, _) =
            crate::platform::witness::bind_witness_manifest(
                local.transport.witness.repository_id,
                local.transport.witness.package_id,
                changed_root_digest,
                local.transport.witness.roots,
            )
            .expect("changed root witness");
        let mut substituted = local.transport.clone();
        substituted.semantic_root = changed_root_digest;
        substituted.witness = changed_witness;
        substituted.validation_witness = changed_witness_digest;
        let (digest, bytes) = substituted.encode().expect("locally encoded substitution");
        let mut objects = local.closure_objects;
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::SemanticRoot, changed_root_digest.bytes()),
            changed_root_bytes,
        );
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageTransport, digest.bytes()),
            bytes,
        );
        let mut store = MemoryPackedStore::default();
        stage_objects(&mut store, &objects);
        let mut work = StoreWork::default();
        assert_eq!(
            validate_package_transport_closure(
                &store,
                substituted.package_revision,
                &[PackageTransportBinding {
                    package_revision: substituted.package_revision,
                    transport: digest,
                }],
                None,
                &mut work,
            )
            .unwrap_err()
            .code,
            "package_transport_semantic_state_binding"
        );
    }

    #[test]
    fn transport_selection_is_strict_and_edge_budget_exhausts_before_enqueue() {
        let fixture = fixture(10, &[]);
        let selection = source::PackageReadiness {
            bindings: BTreeMap::from([(fixture.revision_digest, fixture.transport_digest)]),
        };
        let bytes = selection.encode().expect("transport selection");
        assert_eq!(source::PackageReadiness::decode(&bytes).unwrap(), selection);
        let mut trailing = bytes;
        trailing.push(0);
        assert!(source::PackageReadiness::decode(&trailing).is_err());
        assert_eq!(
            reserve_dependency_edges(0, MAXIMUM_PACKAGE_CLOSURE_EDGES).unwrap(),
            MAXIMUM_PACKAGE_CLOSURE_EDGES
        );
        assert_eq!(
            reserve_dependency_edges(1, MAXIMUM_PACKAGE_CLOSURE_EDGES)
                .unwrap_err()
                .code,
            "package_revision_closure_edge_count"
        );
    }

    #[test]
    fn exact_logical_resolution_rejects_self_dependency_cycles_missing_edges_and_conflicts() {
        let a = fixture(41, &[]);
        let b = fixture(42, &[]);
        let edge = |target: &Fixture| DependencyRecord {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: target.revision.package,
            semantic_revision: target.revision.revision.revision_id().unwrap(),
            package_revision: target.revision_digest,
        };
        let mut self_dependent = a.revision.clone();
        self_dependent.dependencies.push(edge(&a));
        assert_eq!(
            self_dependent.encode().unwrap_err().code,
            "package_revision_self_dependency"
        );
        // The structural cycle guard is exercised with independently fixed decoded records.
        // Hash-bound public cycles must additionally pass identity and exact-unification checks;
        // these hostile records are not a public authoring path or valid digest preimages.
        let mut a_cycle = a.revision.clone();
        let mut b_cycle = b.revision.clone();
        a_cycle.dependencies.push(edge(&b));
        b_cycle.dependencies.push(edge(&a));
        let mut revisions =
            BTreeMap::from([(a.revision_digest, a_cycle), (b.revision_digest, b_cycle)]);
        assert_eq!(
            reject_dependency_cycle(&revisions).unwrap_err().code,
            "package_revision_dependency_cycle"
        );
        revisions.remove(&b.revision_digest);
        assert_eq!(
            reject_dependency_cycle(&revisions).unwrap_err().code,
            "package_revision_closure_edge"
        );
        let a2 = fixture(41, &[&b]);
        let left = fixture(43, &[&a]);
        let right = fixture(44, &[&a2]);
        let root = fixture(45, &[&left, &right]);
        let mut store = MemoryPackedStore::default();
        stage_objects(&mut store, &root.closure_objects);
        assert_eq!(
            validate_package_transport_closure(
                &store,
                root.revision_digest,
                &selections(&root),
                None,
                &mut StoreWork::default(),
            )
            .unwrap_err()
            .code,
            "package_revision_closure_package_conflict"
        );
    }

    #[test]
    fn package_revision_rejects_dependency_length_before_allocation() {
        struct ExcessDependencies;

        impl Encode for ExcessDependencies {
            fn encode<E: bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> Result<(), bincode::error::EncodeError> {
                u64::try_from(MAXIMUM_PACKAGE_DEPENDENCIES + 1)
                    .expect("test dependency limit fits u64")
                    .encode(encoder)
            }
        }

        #[derive(Encode)]
        struct OversizedPackageRevision {
            contract_version: u16,
            graph_contract_version: u16,
            package: PackageId,
            revision: RevisionCore,
            interface: PackageInterfaceDigest,
            dependencies: ExcessDependencies,
        }

        let fixture = fixture(11, &[]);
        let oversized = OversizedPackageRevision {
            contract_version: fixture.revision.contract_version,
            graph_contract_version: fixture.revision.graph_contract_version,
            package: fixture.revision.package,
            revision: fixture.revision.revision,
            interface: fixture.revision.interface,
            dependencies: ExcessDependencies,
        };
        let bytes = crate::platform::packed::encode(
            PACKAGE_REVISION_MAGIC,
            PACKAGE_REVISION_ENVELOPE_DOMAIN,
            &oversized,
            MAXIMUM_PACKAGE_REVISION_BYTES,
        )
        .expect("encode oversized dependency length without allocating dependencies");
        let error = PackageRevision::decode(&bytes, PackageRevisionDigest::of(&bytes))
            .expect_err("oversized dependency length must reject");
        assert_eq!(error.code, "packed_decode");
        assert!(error.message.contains("before allocation"));
    }
}
