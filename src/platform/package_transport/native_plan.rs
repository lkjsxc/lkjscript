//! Checked native topology over one completely admitted exact package closure.
//!
//! Rust owns source identity and bounded projection. The ordinary native program
//! owns component classification and dependency-first stages; there is no host
//! topology fallback. Its own artifact is admitted by the separate artifact reader.

use super::{
    CollectedPackageRevisionInputs, MAXIMUM_PACKAGE_CLOSURE, MAXIMUM_PACKAGE_CLOSURE_EDGES,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionControl;
use crate::platform::execution::normalized::execution_diagnostic;
use crate::platform::json::{JsonLimits, decode_strict};
use crate::platform::kernel::PackageRevisionDigest;
use crate::platform::native_tool::PureTool;
use crate::platform::semantic_id::encode_hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

const ARTIFACT: &[u8] =
    include_bytes!("../../../tools/native-package-plan/generated/package-plan.lkja");
const TARGET: &str = "package-plan";

// Defaults remain unchanged at public command boundaries. These limits admit the
// complete native 10,000-node / 100,000-edge graph and its checked full result.
// Raw typed-value admission, source admission and working storage remain separate.
const JSON_LIMITS: JsonLimits = JsonLimits {
    maximum_bytes: 4 * 1_048_576,
    maximum_depth: 32,
    maximum_items: 500_000,
    maximum_string_bytes: 256,
};

static PROGRAM: PreparationCache<PureTool> = PreparationCache::new();

#[cfg(test)]
std::thread_local! {
    static INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn invocation_count() -> usize {
    INVOCATIONS.with(std::cell::Cell::get)
}

#[derive(Clone, Debug)]
pub(crate) struct DependencyPlan {
    order: Vec<PackageRevisionDigest>,
    evidence: NativePackagePlanEvidence,
}

/// Identity trace of the successful invocation consumed by the compiler.
/// The private construction of `DependencyPlan`, rather than these hashes,
/// establishes that complete checked native planning actually succeeded.
#[derive(Clone, Debug)]
pub(crate) struct NativePackagePlanEvidence {
    pub artifact_sha256: String,
    pub input_sha256: String,
    pub outcome_sha256: String,
    pub order_sha256: String,
    pub packages: usize,
    pub edges: usize,
}

impl DependencyPlan {
    pub(crate) fn evidence(&self) -> &NativePackagePlanEvidence {
        &self.evidence
    }

    pub(crate) fn into_order(self) -> Vec<PackageRevisionDigest> {
        self.order
    }
}

#[derive(Serialize)]
struct Proposal {
    roots: [i64; 1],
    nodes: Vec<Node>,
}

#[derive(Serialize)]
struct Node {
    id: i64,
    successors: Vec<i64>,
}

struct Projection {
    identities: Vec<PackageRevisionDigest>,
    arguments: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "case",
    content = "value",
    rename_all = "lowercase",
    deny_unknown_fields
)]
enum NativeOutcome {
    Valid(NativePlan),
    Invalid(NativeDiagnostic),
    Capacity(String),
    Inconsistent(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeDiagnostic {
    code: String,
    owner: i64,
    target: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativePlan {
    components: Vec<Component>,
    stages: Vec<Vec<i64>>,
    reachable: Vec<i64>,
    unreachable: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Component {
    members: Vec<i64>,
    cyclic: bool,
    dependencies: Vec<i64>,
}

fn contradiction(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, code, message)
}

fn checkpoint(control: &ExecutionControl) -> Result<(), Diagnostic> {
    control.check().map_err(execution_diagnostic)
}

pub(super) fn plan(
    inputs: &CollectedPackageRevisionInputs,
    root: PackageRevisionDigest,
    control: &ExecutionControl,
) -> Result<DependencyPlan, Diagnostic> {
    let projection = project(inputs, root, control)?;
    let output = invoke(&projection.arguments, control)?;
    let order = admit_output(&output, &projection.identities, control)?;
    let mut ordered_identities = Sha256::new();
    for identity in &order {
        checkpoint(control)?;
        // Every exact identity has 32 bytes. This digest uses their raw ordered
        // concatenation, without a host-dependent length, text or delimiter.
        ordered_identities.update(identity.bytes());
    }
    let evidence = NativePackagePlanEvidence {
        artifact_sha256: sha256(ARTIFACT),
        input_sha256: sha256(&projection.arguments),
        outcome_sha256: sha256(&output),
        order_sha256: encode_hex(&ordered_identities.finalize()),
        packages: inputs.revisions.len(),
        edges: inputs.dependency_edges,
    };
    checkpoint(control)?;
    Ok(DependencyPlan { order, evidence })
}

fn sha256(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}

fn project(
    inputs: &CollectedPackageRevisionInputs,
    root: PackageRevisionDigest,
    control: &ExecutionControl,
) -> Result<Projection, Diagnostic> {
    checkpoint(control)?;
    if inputs.revisions.is_empty()
        || inputs.revisions.len() > MAXIMUM_PACKAGE_CLOSURE
        || inputs.dependency_edges > MAXIMUM_PACKAGE_CLOSURE_EDGES
        || inputs.revisions.get(&root) != Some(&inputs.root_revision)
    {
        return Err(contradiction(
            "package_native_plan_input",
            "collected package inputs violate the complete bounded root binding",
        ));
    }
    let identities: Vec<_> = inputs.revisions.keys().copied().collect();
    let positions = identities
        .iter()
        .enumerate()
        .map(|(position, identity)| {
            i64::try_from(position)
                .map(|position| (*identity, position))
                .map_err(|_| {
                    contradiction(
                        "package_native_plan_position",
                        "package position exceeds the native signed integer domain",
                    )
                })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let root_position = *positions.get(&root).ok_or_else(|| {
        contradiction(
            "package_native_plan_root",
            "collected package root disappeared during projection",
        )
    })?;
    let mut nodes = Vec::with_capacity(inputs.revisions.len());
    let mut edges = 0_usize;
    for (identity, revision) in &inputs.revisions {
        checkpoint(control)?;
        edges = edges
            .checked_add(revision.dependencies.len())
            .filter(|count| *count <= MAXIMUM_PACKAGE_CLOSURE_EDGES)
            .ok_or_else(|| {
                contradiction(
                    "package_native_plan_edges",
                    "collected package edges exceed the complete native boundary",
                )
            })?;
        let successors = revision
            .dependencies
            .iter()
            .map(|dependency| {
                checkpoint(control)?;
                positions
                    .get(&dependency.package_revision)
                    .copied()
                    .ok_or_else(|| {
                        contradiction(
                            "package_native_plan_edge",
                            "collected package edge names an identity outside this exact closure",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let id = positions.get(identity).copied().ok_or_else(|| {
            contradiction(
                "package_native_plan_position",
                "collected package identity disappeared during projection",
            )
        })?;
        nodes.push(Node { id, successors });
    }
    if edges != inputs.dependency_edges {
        return Err(contradiction(
            "package_native_plan_edges",
            "complete projected edge count differs from admitted source inventory",
        ));
    }
    let proposal = Proposal {
        roots: [root_position],
        nodes,
    };
    let mut output = BoundedJson::default();
    serde_json::to_writer(&mut output, &[proposal]).map_err(|error| {
        if error.is_io() {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "package_native_plan_input_storage",
                format!("complete package projection storage refused: {error}"),
            )
        } else {
            contradiction(
                "package_native_plan_encoding",
                format!("complete package projection failed encoding: {error}"),
            )
        }
    })?;
    checkpoint(control)?;
    Ok(Projection {
        identities,
        arguments: output.bytes,
    })
}

#[derive(Default)]
struct BoundedJson {
    bytes: Vec<u8>,
}

impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes
            .len()
            .checked_add(bytes.len())
            .filter(|total| *total <= JSON_LIMITS.maximum_bytes)
            .ok_or_else(|| std::io::Error::other("package planning JSON byte ceiling exhausted"))?;
        self.bytes.try_reserve(bytes.len()).map_err(|error| {
            std::io::Error::other(format!("package JSON storage refused: {error}"))
        })?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn invoke(arguments: &[u8], control: &ExecutionControl) -> Result<Vec<u8>, Diagnostic> {
    let program = PROGRAM.get_or_prepare(control, |control| PureTool::load(ARTIFACT, control))?;
    #[cfg(test)]
    INVOCATIONS.with(|count| count.set(count.get() + 1));
    program.run_json(TARGET, arguments, JSON_LIMITS, control)
}

fn decode_outcome(bytes: &[u8], control: &ExecutionControl) -> Result<NativeOutcome, Diagnostic> {
    checkpoint(control)?;
    let value = decode_strict(bytes, JSON_LIMITS).map_err(|error| {
        contradiction(
            "package_native_plan_output",
            format!("native package plan failed complete strict JSON admission: {error}"),
        )
    })?;
    let outcome = serde_json::from_value(value).map_err(|error| {
        contradiction(
            "package_native_plan_output",
            format!("native package plan has an unexpected complete outcome shape: {error}"),
        )
    })?;
    checkpoint(control)?;
    Ok(outcome)
}

fn admit_output(
    bytes: &[u8],
    identities: &[PackageRevisionDigest],
    control: &ExecutionControl,
) -> Result<Vec<PackageRevisionDigest>, Diagnostic> {
    let plan = match decode_outcome(bytes, control)? {
        NativeOutcome::Valid(plan) => plan,
        NativeOutcome::Invalid(diagnostic) => {
            return Err(contradiction(
                "package_native_plan_negative",
                format!(
                    "native planner rejected admitted source as {} at {} -> {}",
                    diagnostic.code, diagnostic.owner, diagnostic.target
                ),
            ));
        }
        NativeOutcome::Capacity(dimension) => {
            return Err(contradiction(
                "package_native_plan_negative",
                format!("native planner refused admitted source capacity {dimension}"),
            ));
        }
        NativeOutcome::Inconsistent(reason) => {
            return Err(contradiction(
                "package_native_plan_inconsistent",
                format!("native package planner contradicted its checker: {reason}"),
            ));
        }
    };
    if identities.is_empty() || identities.len() > MAXIMUM_PACKAGE_CLOSURE {
        return Err(bad_mapping("invalid exact identity inventory"));
    }
    // A source closure collected from its root contains no unreachable package.
    // Verify returned identity coverage without deriving host reachability.
    if !plan.unreachable.is_empty()
        || plan.reachable.len() != identities.len()
        || plan
            .reachable
            .iter()
            .enumerate()
            .any(|(position, id)| usize::try_from(*id).ok() != Some(position))
    {
        return Err(bad_mapping(
            "native reachability inventory differs from the exact closure",
        ));
    }
    let mut membership = vec![false; identities.len()];
    let mut representatives = vec![false; identities.len()];
    let mut previous_representative = None;
    let mut cyclic = false;
    for component in &plan.components {
        checkpoint(control)?;
        let first = component
            .members
            .first()
            .ok_or_else(|| bad_mapping("native component is empty"))?;
        let representative = position(*first, identities.len())?;
        if previous_representative.is_some_and(|previous| previous >= representative) {
            return Err(bad_mapping(
                "native component identities lost their authored order",
            ));
        }
        previous_representative = Some(representative);
        representatives[representative] = true;
        let mut previous_member = None;
        for member in &component.members {
            checkpoint(control)?;
            let member = position(*member, identities.len())?;
            if membership[member] || previous_member.is_some_and(|previous| previous >= member) {
                return Err(bad_mapping(
                    "native component membership is repeated or unordered",
                ));
            }
            membership[member] = true;
            previous_member = Some(member);
        }
        if component.members.len() != 1 && !component.cyclic {
            return Err(bad_mapping(
                "nonsingleton component lacks its cycle classification",
            ));
        }
        cyclic |= component.cyclic;
    }
    if membership.iter().any(|covered| !covered) {
        return Err(bad_mapping(
            "native component inventory omits exact package identities",
        ));
    }
    let mut referenced = vec![0_usize; identities.len()];
    for (rank, component) in plan.components.iter().enumerate() {
        checkpoint(control)?;
        for dependency in &component.dependencies {
            checkpoint(control)?;
            let dependency = position(*dependency, identities.len())?;
            if !representatives[dependency] || referenced[dependency] == rank + 1 {
                return Err(bad_mapping(
                    "native component dependency has no unique representative",
                ));
            }
            referenced[dependency] = rank + 1;
        }
    }
    let mut scheduled = vec![false; identities.len()];
    let mut order = Vec::with_capacity(plan.components.len());
    for stage in &plan.stages {
        checkpoint(control)?;
        if stage.is_empty() {
            return Err(bad_mapping("native package plan contains an empty stage"));
        }
        let mut previous = None;
        for identity in stage {
            checkpoint(control)?;
            let index = position(*identity, identities.len())?;
            if !representatives[index]
                || scheduled[index]
                || previous.is_some_and(|previous| previous >= index)
            {
                return Err(bad_mapping(
                    "native stages contain a foreign, repeated or unordered identity",
                ));
            }
            scheduled[index] = true;
            previous = Some(index);
            order.push(identities[index]);
        }
    }
    if order.len() != plan.components.len() {
        return Err(bad_mapping("native stages omit a component representative"));
    }
    checkpoint(control)?;
    if cyclic {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "package_revision_dependency_cycle",
            "logical package dependency closure contains a cycle",
        ));
    }
    Ok(order)
}

fn position(identity: i64, count: usize) -> Result<usize, Diagnostic> {
    usize::try_from(identity)
        .ok()
        .filter(|position| *position < count)
        .ok_or_else(|| bad_mapping("native package identity escaped this exact input mapping"))
}

fn bad_mapping(message: &'static str) -> Diagnostic {
    contradiction("package_native_plan_mapping", message)
}

// Only successful complete preparation is shared. Waiting callers retain their
// own cancellation controls; a cancelled load or panicking initializer releases
// the preparation slot, rather than permanently caching a failure.
struct PreparationCache<T> {
    state: Mutex<PreparationState<T>>,
    changed: Condvar,
}

struct PreparationState<T> {
    prepared: Option<Arc<T>>,
    preparing: bool,
}

impl<T> PreparationCache<T> {
    const fn new() -> Self {
        Self {
            state: Mutex::new(PreparationState {
                prepared: None,
                preparing: false,
            }),
            changed: Condvar::new(),
        }
    }

    fn state(&self) -> MutexGuard<'_, PreparationState<T>> {
        // Cache state is changed only at complete preparation and slot release.
        // A panic cannot turn an unadmitted program into a prepared cache entry.
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn get_or_prepare(
        &self,
        control: &ExecutionControl,
        prepare: impl FnOnce(&ExecutionControl) -> Result<T, Diagnostic>,
    ) -> Result<Arc<T>, Diagnostic> {
        let mut state = self.state();
        loop {
            checkpoint(control)?;
            if let Some(prepared) = &state.prepared {
                return Ok(Arc::clone(prepared));
            }
            if !state.preparing {
                state.preparing = true;
                break;
            }
            let (next, _) = self
                .changed
                .wait_timeout(state, Duration::from_millis(10))
                .unwrap_or_else(|error| error.into_inner());
            state = next;
        }
        drop(state);
        let slot = PreparationSlot { cache: self };
        let result = prepare(control).and_then(|prepared| {
            checkpoint(control)?;
            Ok(Arc::new(prepared))
        });
        if let Ok(prepared) = &result {
            self.state().prepared = Some(Arc::clone(prepared));
        }
        drop(slot);
        result
    }
}

struct PreparationSlot<'a, T> {
    cache: &'a PreparationCache<T>,
}

impl<T> Drop for PreparationSlot<'_, T> {
    fn drop(&mut self) {
        self.cache.state().preparing = false;
        self.cache.changed.notify_all();
    }
}

#[cfg(test)]
#[path = "native_plan_tests.rs"]
mod tests;
