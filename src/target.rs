//! Typed build-target meaning and deterministic lowering from one exact project revision.

use crate::application::{
    ApplicationFieldValue, ApplicationImport, ApplicationTarget, ApplicationTestCase,
    ApplicationTestExpectation, ApplicationTrap, ApplicationTrapCode, ApplicationValue,
    HostInterface, HostOperation, HostOutcomeClass, HostOutcomeRoute, HostRequestRoute,
    InvocationProfile, StatefulApplicationProfile,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::interpret::{RunPolicy, RuntimeValue};
use crate::release::{
    PreparedRelease, ReleaseBuildRequest, ReleaseDependencyRequest, ReleaseExportRequest,
    ReleaseImportRequest, ReleaseTestCase,
};
use crate::schema::{ByteString, Node, TextString};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const TARGET_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_TARGETS: usize = 256;
pub const MAXIMUM_TARGET_DEPENDENCIES: usize = 256;
pub const MAXIMUM_TARGET_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildTargetKind {
    Release,
    Application,
    Product,
}

impl BuildTargetKind {
    pub const ALL: [Self; 3] = [Self::Release, Self::Application, Self::Product];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Application => "application",
            Self::Product => "product",
        }
    }

    pub(crate) const fn stable_tag(self) -> u8 {
        match self {
            Self::Release => 1,
            Self::Application => 2,
            Self::Product => 3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetReleaseDependency {
    pub slot: String,
    pub target: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseTargetDefinition {
    pub root: NodeId,
    pub coordinate: String,
    pub user_version: String,
    pub exports: Vec<ReleaseExportRequest>,
    pub dependencies: Vec<TargetReleaseDependency>,
    pub imports: Vec<ReleaseImportRequest>,
    pub tests: Vec<ReleaseTestCase>,
}

/// One exact source item selected through one exact release-target declaration.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetItem {
    pub release_target: NodeId,
    pub item: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetFieldValue {
    pub field: TargetItem,
    pub value: TargetValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetValue {
    Unit,
    Bool(bool),
    I64(i64),
    Bytes(ByteString),
    Text(TextString),
    Product {
        ty: TargetItem,
        fields: Vec<TargetFieldValue>,
    },
    Sum {
        ty: TargetItem,
        variant: TargetItem,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<Box<TargetValue>>,
    },
    Sequence {
        ty: TargetItem,
        elements: Vec<TargetValue>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetHostRequestRoute {
    pub variant: TargetItem,
    pub operation: HostOperation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetHostOutcomeRoute {
    pub operation: HostOperation,
    pub class: HostOutcomeClass,
    pub variant: TargetItem,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetApplicationImport {
    pub slot: String,
    pub interface: HostInterface,
    pub request: TargetItem,
    pub outcome: TargetItem,
    pub command_variant: TargetItem,
    pub outcome_variant: TargetItem,
    pub requests: Vec<TargetHostRequestRoute>,
    pub outcomes: Vec<TargetHostOutcomeRoute>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetStatefulApplicationProfile {
    pub resume: TargetItem,
    pub query_entry: TargetItem,
    pub state: TargetItem,
    pub event: TargetItem,
    pub response: TargetItem,
    pub query: TargetItem,
    pub query_result: TargetItem,
    pub command: TargetItem,
    pub outcome: TargetItem,
    pub decision: TargetItem,
    pub declined_variant: TargetItem,
    pub declined_payload: TargetItem,
    pub declined_response_field: TargetItem,
    pub unchanged_variant: TargetItem,
    pub unchanged_payload: TargetItem,
    pub unchanged_response_field: TargetItem,
    pub completed_variant: TargetItem,
    pub completed_payload: TargetItem,
    pub completed_state_field: TargetItem,
    pub completed_response_field: TargetItem,
    pub suspended_variant: TargetItem,
    pub suspended_payload: TargetItem,
    pub suspended_state_field: TargetItem,
    pub suspended_response_field: TargetItem,
    pub suspended_command_field: TargetItem,
    pub imports: Vec<TargetApplicationImport>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetInvocationProfile {
    Typed,
    BytesStream,
    Stateful(Box<TargetStatefulApplicationProfile>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetTrap {
    pub code: ApplicationTrapCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetTestExpectation {
    Value(TargetValue),
    Trap(TargetTrap),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetApplicationTestCase {
    pub name: String,
    pub target: TargetItem,
    pub arguments: Vec<TargetValue>,
    pub expected: TargetTestExpectation,
    pub policy: RunPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTargetDefinition {
    pub root_release: NodeId,
    pub entry: TargetItem,
    pub profile: TargetInvocationProfile,
    pub policy: RunPolicy,
    pub tests: Vec<TargetApplicationTestCase>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductTargetDefinition {
    pub application: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BuildTargetDefinition {
    Release(ReleaseTargetDefinition),
    Application(Box<ApplicationTargetDefinition>),
    Product(ProductTargetDefinition),
}

impl BuildTargetDefinition {
    pub const fn kind(&self) -> BuildTargetKind {
        match self {
            Self::Release(_) => BuildTargetKind::Release,
            Self::Application(_) => BuildTargetKind::Application,
            Self::Product(_) => BuildTargetKind::Product,
        }
    }
}

pub fn definition_digest(definition: &BuildTargetDefinition) -> Result<String> {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.build-target.definition.v1");
    hasher.update(&[definition.kind().stable_tag()]);
    hasher.update(&encode_definition(definition)?);
    Ok(crate::release::hex(hasher.finalize().as_bytes()))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetSummary {
    pub target: NodeId,
    pub name: String,
    pub kind: BuildTargetKind,
    pub dependencies: Vec<NodeId>,
}

pub(crate) fn encode_definition(definition: &BuildTargetDefinition) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(definition).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("cannot encode canonical build target: {error}"),
        )
    })?;
    if bytes.len() > MAXIMUM_TARGET_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "canonical build target exceeds byte policy",
        ));
    }
    Ok(bytes)
}

pub(crate) fn decode_definition(bytes: &[u8]) -> Result<BuildTargetDefinition> {
    if bytes.len() > MAXIMUM_TARGET_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "canonical build target exceeds byte policy",
        ));
    }
    let definition: BuildTargetDefinition = serde_json::from_slice(bytes).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("canonical build-target decoding failed: {error}"),
        )
    })?;
    if encode_definition(&definition)? != bytes {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "build-target encoding is not canonical",
        ));
    }
    Ok(definition)
}

pub fn summaries(snapshot: &Snapshot) -> Vec<TargetSummary> {
    snapshot
        .nodes()
        .filter_map(|(id, node)| match node {
            Node::BuildTarget {
                name, definition, ..
            } => Some(TargetSummary {
                target: id,
                name: name.clone(),
                kind: definition.kind(),
                dependencies: direct_dependencies(definition),
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn direct_dependencies(definition: &BuildTargetDefinition) -> Vec<NodeId> {
    match definition {
        BuildTargetDefinition::Release(release) => release
            .dependencies
            .iter()
            .map(|item| item.target)
            .collect(),
        BuildTargetDefinition::Application(application) => vec![application.root_release],
        BuildTargetDefinition::Product(product) => vec![product.application],
    }
}

pub(crate) fn validate_snapshot_targets(snapshot: &Snapshot) -> Result<()> {
    let targets = summaries(snapshot);
    if targets.len() > MAXIMUM_TARGETS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "build-target count exceeds policy",
        ));
    }
    let target_ids = targets
        .iter()
        .map(|item| item.target)
        .collect::<BTreeSet<_>>();
    for summary in &targets {
        if summary.dependencies.len() > MAXIMUM_TARGET_DEPENDENCIES {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "build-target dependency count exceeds policy",
            )
            .for_node(summary.target));
        }
        for dependency in &summary.dependencies {
            if !target_ids.contains(dependency) {
                return Err(LkError::new(
                    ErrorCode::NodeNotFound,
                    "build target names a missing target dependency",
                )
                .for_node(*dependency)
                .with_related([summary.target]));
            }
        }
    }
    validate_target_cycles(&targets)?;
    for summary in &targets {
        validate_target(snapshot, summary.target)?;
    }
    for summary in &targets {
        prepare(snapshot, summary.target)?;
    }
    Ok(())
}

fn validate_target_cycles(targets: &[TargetSummary]) -> Result<()> {
    let dependencies = targets
        .iter()
        .map(|target| (target.target, target.dependencies.as_slice()))
        .collect::<BTreeMap<_, _>>();
    for root in dependencies.keys().copied() {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut stack = vec![(root, false)];
        while let Some((target, leaving)) = stack.pop() {
            if leaving {
                visiting.remove(&target);
                visited.insert(target);
                continue;
            }
            if visited.contains(&target) {
                continue;
            }
            if !visiting.insert(target) {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "build-target dependency graph contains a cycle",
                )
                .for_node(target)
                .with_related([root]));
            }
            stack.push((target, true));
            if let Some(children) = dependencies.get(&target) {
                for child in children.iter().rev() {
                    stack.push((*child, false));
                }
            }
        }
    }
    Ok(())
}

fn validate_target(snapshot: &Snapshot, target: NodeId) -> Result<()> {
    let Node::BuildTarget { definition, .. } = snapshot.node(target)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "target selector does not name a build target",
        )
        .for_node(target));
    };
    match definition {
        BuildTargetDefinition::Release(release) => {
            if !matches!(snapshot.node(release.root)?, Node::Package { .. }) {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "release target root must name a package",
                )
                .for_node(release.root)
                .with_related([target]));
            }
            for dependency in &release.dependencies {
                require_target_kind(
                    snapshot,
                    dependency.target,
                    BuildTargetKind::Release,
                    target,
                )?;
            }
        }
        BuildTargetDefinition::Application(application) => {
            require_target_kind(
                snapshot,
                application.root_release,
                BuildTargetKind::Release,
                target,
            )?;
            validate_target_item(snapshot, application.entry, target)?;
            visit_profile_items(&application.profile, |item| {
                validate_target_item(snapshot, item, target)
            })?;
            for case in &application.tests {
                validate_target_item(snapshot, case.target, target)?;
                for argument in &case.arguments {
                    visit_value_items(argument, &mut |item| {
                        validate_target_item(snapshot, item, target)
                    })?;
                }
                if let TargetTestExpectation::Value(value) = &case.expected {
                    visit_value_items(value, &mut |item| {
                        validate_target_item(snapshot, item, target)
                    })?;
                }
            }
        }
        BuildTargetDefinition::Product(product) => {
            require_target_kind(
                snapshot,
                product.application,
                BuildTargetKind::Application,
                target,
            )?;
        }
    }
    Ok(())
}

fn require_target_kind(
    snapshot: &Snapshot,
    selected: NodeId,
    expected: BuildTargetKind,
    related: NodeId,
) -> Result<()> {
    let Node::BuildTarget { definition, .. } = snapshot.node(selected)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "target dependency does not name a build target",
        )
        .for_node(selected)
        .with_related([related]));
    };
    if definition.kind() != expected {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "target dependency has the wrong build-target kind",
        )
        .for_node(selected)
        .with_related([related]));
    }
    Ok(())
}

fn validate_target_item(snapshot: &Snapshot, item: TargetItem, related: NodeId) -> Result<()> {
    require_target_kind(
        snapshot,
        item.release_target,
        BuildTargetKind::Release,
        related,
    )?;
    if item.item.workspace() != snapshot.workspace() || item.item.is_function_local() {
        return Err(LkError::new(
            ErrorCode::WrongWorkspace,
            "target item must be a durable identity in this project",
        )
        .for_node(item.item)
        .with_related([related]));
    }
    snapshot.node(item.item)?;
    Ok(())
}

fn visit_profile_items(
    profile: &TargetInvocationProfile,
    mut visit: impl FnMut(TargetItem) -> Result<()>,
) -> Result<()> {
    let TargetInvocationProfile::Stateful(profile) = profile else {
        return Ok(());
    };
    for item in [
        profile.resume,
        profile.query_entry,
        profile.state,
        profile.event,
        profile.response,
        profile.query,
        profile.query_result,
        profile.command,
        profile.outcome,
        profile.decision,
        profile.declined_variant,
        profile.declined_payload,
        profile.declined_response_field,
        profile.unchanged_variant,
        profile.unchanged_payload,
        profile.unchanged_response_field,
        profile.completed_variant,
        profile.completed_payload,
        profile.completed_state_field,
        profile.completed_response_field,
        profile.suspended_variant,
        profile.suspended_payload,
        profile.suspended_state_field,
        profile.suspended_response_field,
        profile.suspended_command_field,
    ] {
        visit(item)?;
    }
    for import in &profile.imports {
        for item in [
            import.request,
            import.outcome,
            import.command_variant,
            import.outcome_variant,
        ] {
            visit(item)?;
        }
        for route in &import.requests {
            visit(route.variant)?;
        }
        for route in &import.outcomes {
            visit(route.variant)?;
        }
    }
    Ok(())
}

fn visit_value_items(
    value: &TargetValue,
    visit: &mut impl FnMut(TargetItem) -> Result<()>,
) -> Result<()> {
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            TargetValue::Unit
            | TargetValue::Bool(_)
            | TargetValue::I64(_)
            | TargetValue::Bytes(_)
            | TargetValue::Text(_) => {}
            TargetValue::Product { ty, fields } => {
                visit(*ty)?;
                for field in fields.iter().rev() {
                    visit(field.field)?;
                    stack.push(&field.value);
                }
            }
            TargetValue::Sum {
                ty,
                variant,
                payload,
            } => {
                visit(*ty)?;
                visit(*variant)?;
                if let Some(payload) = payload {
                    stack.push(payload);
                }
            }
            TargetValue::Sequence { ty, elements } => {
                visit(*ty)?;
                stack.extend(elements.iter().rev());
            }
        }
    }
    Ok(())
}

/// Prepared target output. It is derived from one immutable snapshot and owns no development state.
pub enum PreparedTarget {
    Release(Box<PreparedRelease>),
    Application(Box<crate::application::PreparedApplication>),
}

impl PreparedTarget {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Release(release) => release.bytes(),
            Self::Application(application) => application.bytes(),
        }
    }

    pub fn publish(&self, path: &Path) -> Result<()> {
        match self {
            Self::Release(release) => release.publish(path),
            Self::Application(application) => application.publish(path),
        }
    }
}

pub fn prepare(snapshot: &Snapshot, target: NodeId) -> Result<PreparedTarget> {
    let mut releases = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    prepare_target(snapshot, target, &mut releases, &mut visiting)
}

fn prepare_target(
    snapshot: &Snapshot,
    target: NodeId,
    releases: &mut BTreeMap<NodeId, PreparedRelease>,
    visiting: &mut BTreeSet<NodeId>,
) -> Result<PreparedTarget> {
    if !visiting.insert(target) {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "build-target dependency graph contains a cycle",
        )
        .for_node(target));
    }
    let result = match snapshot.node(target)? {
        Node::BuildTarget {
            definition: BuildTargetDefinition::Release(_),
            ..
        } => {
            prepare_release_target(snapshot, target, releases, visiting)?;
            let release = releases.remove(&target).ok_or_else(|| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "prepared release target disappeared",
                )
                .for_node(target)
            })?;
            PreparedTarget::Release(Box::new(release))
        }
        Node::BuildTarget {
            definition: BuildTargetDefinition::Application(application),
            ..
        } => PreparedTarget::Application(Box::new(prepare_application_target(
            snapshot,
            application,
            releases,
            visiting,
        )?)),
        Node::BuildTarget {
            definition: BuildTargetDefinition::Product(product),
            ..
        } => match prepare_target(snapshot, product.application, releases, visiting)? {
            PreparedTarget::Application(application) => PreparedTarget::Application(application),
            PreparedTarget::Release(_) => {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "product target dependency is not an application target",
                )
                .for_node(product.application));
            }
        },
        _ => {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "target selector does not name a build target",
            )
            .for_node(target));
        }
    };
    visiting.remove(&target);
    Ok(result)
}

fn prepare_release_target(
    snapshot: &Snapshot,
    target: NodeId,
    releases: &mut BTreeMap<NodeId, PreparedRelease>,
    visiting: &mut BTreeSet<NodeId>,
) -> Result<()> {
    if releases.contains_key(&target) {
        return Ok(());
    }
    let Node::BuildTarget {
        definition: BuildTargetDefinition::Release(definition),
        ..
    } = snapshot.node(target)?
    else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "release dependency does not name a release target",
        )
        .for_node(target));
    };
    let mut dependency_requests = Vec::with_capacity(definition.dependencies.len());
    let mut dependency_bytes = Vec::new();
    for dependency in &definition.dependencies {
        if !releases.contains_key(&dependency.target) {
            if !visiting.insert(dependency.target) {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "release-target dependency graph contains a cycle",
                )
                .for_node(dependency.target));
            }
            prepare_release_target(snapshot, dependency.target, releases, visiting)?;
            visiting.remove(&dependency.target);
        }
        let prepared = releases.get(&dependency.target).ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "prepared release dependency disappeared",
            )
        })?;
        dependency_requests.push(ReleaseDependencyRequest {
            slot: dependency.slot.clone(),
            release: prepared.release_id(),
        });
        dependency_bytes.push(prepared.bytes().to_vec());
    }
    let request = ReleaseBuildRequest {
        version: crate::release::RELEASE_CONTRACT_VERSION,
        workspace: snapshot.workspace(),
        revision: snapshot.revision(),
        root: definition.root,
        coordinate: definition.coordinate.clone(),
        user_version: definition.user_version.clone(),
        exports: definition.exports.clone(),
        dependencies: dependency_requests,
        imports: definition.imports.clone(),
        tests: definition.tests.clone(),
    };
    let prepared = crate::release::prepare(snapshot, &request, &dependency_bytes)?;
    releases.insert(target, prepared);
    Ok(())
}

fn prepare_application_target(
    snapshot: &Snapshot,
    definition: &ApplicationTargetDefinition,
    releases: &mut BTreeMap<NodeId, PreparedRelease>,
    visiting: &mut BTreeSet<NodeId>,
) -> Result<crate::application::PreparedApplication> {
    prepare_release_target(snapshot, definition.root_release, releases, visiting)?;
    let mut supplied = Vec::new();
    collect_release_closure(
        snapshot,
        definition.root_release,
        releases,
        &mut BTreeSet::new(),
        &mut supplied,
    )?;
    let root = releases.get(&definition.root_release).ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "application root release disappeared",
        )
    })?;
    let request = crate::application::ApplicationBuildRequest {
        version: crate::application::APPLICATION_CONTRACT_VERSION,
        root_release: root.release_id(),
        entry: lower_item(definition.entry, releases)?,
        profile: lower_profile(&definition.profile, releases)?,
        policy: definition.policy,
        tests: definition
            .tests
            .iter()
            .map(|case| lower_test(case, releases))
            .collect::<Result<Vec<_>>>()?,
    };
    crate::application::prepare(&request, &supplied)
}

fn collect_release_closure(
    snapshot: &Snapshot,
    target: NodeId,
    releases: &BTreeMap<NodeId, PreparedRelease>,
    seen: &mut BTreeSet<NodeId>,
    output: &mut Vec<Vec<u8>>,
) -> Result<()> {
    if !seen.insert(target) {
        return Ok(());
    }
    let release = releases.get(&target).ok_or_else(|| {
        LkError::new(ErrorCode::ArtifactCorrupt, "release closure is incomplete").for_node(target)
    })?;
    output.push(release.bytes().to_vec());
    let Node::BuildTarget {
        definition: BuildTargetDefinition::Release(definition),
        ..
    } = snapshot.node(target)?
    else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "release closure contains a non-release target",
        )
        .for_node(target));
    };
    for dependency in &definition.dependencies {
        collect_release_closure(snapshot, dependency.target, releases, seen, output)?;
    }
    Ok(())
}

fn lower_item(
    item: TargetItem,
    releases: &BTreeMap<NodeId, PreparedRelease>,
) -> Result<ApplicationTarget> {
    let release = releases.get(&item.release_target).ok_or_else(|| {
        LkError::new(
            ErrorCode::NodeNotFound,
            "target item names a release outside the application closure",
        )
        .for_node(item.release_target)
    })?;
    Ok(ApplicationTarget {
        release: release.release_id(),
        item: release.source_item(item.item)?,
    })
}

fn lower_value(
    value: &TargetValue,
    releases: &BTreeMap<NodeId, PreparedRelease>,
) -> Result<ApplicationValue> {
    Ok(match value {
        TargetValue::Unit => ApplicationValue::Unit,
        TargetValue::Bool(value) => ApplicationValue::Bool(*value),
        TargetValue::I64(value) => ApplicationValue::I64(*value),
        TargetValue::Bytes(value) => ApplicationValue::Bytes(value.clone()),
        TargetValue::Text(value) => ApplicationValue::Text(value.clone()),
        TargetValue::Product { ty, fields } => ApplicationValue::Product {
            ty: lower_item(*ty, releases)?,
            fields: fields
                .iter()
                .map(|field| {
                    Ok(ApplicationFieldValue {
                        field: lower_item(field.field, releases)?,
                        value: lower_value(&field.value, releases)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        },
        TargetValue::Sum {
            ty,
            variant,
            payload,
        } => ApplicationValue::Sum {
            ty: lower_item(*ty, releases)?,
            variant: lower_item(*variant, releases)?,
            payload: payload
                .as_deref()
                .map(|value| lower_value(value, releases).map(Box::new))
                .transpose()?,
        },
        TargetValue::Sequence { ty, elements } => ApplicationValue::Sequence {
            ty: lower_item(*ty, releases)?,
            elements: elements
                .iter()
                .map(|value| lower_value(value, releases))
                .collect::<Result<Vec<_>>>()?,
        },
    })
}

fn lower_profile(
    profile: &TargetInvocationProfile,
    releases: &BTreeMap<NodeId, PreparedRelease>,
) -> Result<InvocationProfile> {
    Ok(match profile {
        TargetInvocationProfile::Typed => InvocationProfile::Typed,
        TargetInvocationProfile::BytesStream => InvocationProfile::BytesStream,
        TargetInvocationProfile::Stateful(profile) => {
            let item = |value| lower_item(value, releases);
            InvocationProfile::Stateful(StatefulApplicationProfile {
                resume: item(profile.resume)?,
                query_entry: item(profile.query_entry)?,
                state: item(profile.state)?,
                event: item(profile.event)?,
                response: item(profile.response)?,
                query: item(profile.query)?,
                query_result: item(profile.query_result)?,
                command: item(profile.command)?,
                outcome: item(profile.outcome)?,
                decision: item(profile.decision)?,
                declined_variant: item(profile.declined_variant)?,
                declined_payload: item(profile.declined_payload)?,
                declined_response_field: item(profile.declined_response_field)?,
                unchanged_variant: item(profile.unchanged_variant)?,
                unchanged_payload: item(profile.unchanged_payload)?,
                unchanged_response_field: item(profile.unchanged_response_field)?,
                completed_variant: item(profile.completed_variant)?,
                completed_payload: item(profile.completed_payload)?,
                completed_state_field: item(profile.completed_state_field)?,
                completed_response_field: item(profile.completed_response_field)?,
                suspended_variant: item(profile.suspended_variant)?,
                suspended_payload: item(profile.suspended_payload)?,
                suspended_state_field: item(profile.suspended_state_field)?,
                suspended_response_field: item(profile.suspended_response_field)?,
                suspended_command_field: item(profile.suspended_command_field)?,
                imports: profile
                    .imports
                    .iter()
                    .map(|import| {
                        Ok(ApplicationImport {
                            slot: import.slot.clone(),
                            interface: import.interface,
                            request: item(import.request)?,
                            outcome: item(import.outcome)?,
                            command_variant: item(import.command_variant)?,
                            outcome_variant: item(import.outcome_variant)?,
                            requests: import
                                .requests
                                .iter()
                                .map(|route| {
                                    Ok(HostRequestRoute {
                                        variant: item(route.variant)?,
                                        operation: route.operation,
                                    })
                                })
                                .collect::<Result<Vec<_>>>()?,
                            outcomes: import
                                .outcomes
                                .iter()
                                .map(|route| {
                                    Ok(HostOutcomeRoute {
                                        operation: route.operation,
                                        class: route.class,
                                        variant: item(route.variant)?,
                                    })
                                })
                                .collect::<Result<Vec<_>>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            })
        }
    })
}

fn lower_test(
    case: &TargetApplicationTestCase,
    releases: &BTreeMap<NodeId, PreparedRelease>,
) -> Result<ApplicationTestCase> {
    Ok(ApplicationTestCase {
        name: case.name.clone(),
        target: lower_item(case.target, releases)?,
        arguments: case
            .arguments
            .iter()
            .map(|value| lower_value(value, releases))
            .collect::<Result<Vec<_>>>()?,
        expected: match &case.expected {
            TargetTestExpectation::Value(value) => {
                ApplicationTestExpectation::Value(lower_value(value, releases)?)
            }
            TargetTestExpectation::Trap(trap) => {
                ApplicationTestExpectation::Trap(ApplicationTrap {
                    code: trap.code,
                    target: trap
                        .target
                        .map(|target| lower_item(target, releases))
                        .transpose()?,
                })
            }
        },
        policy: case.policy,
    })
}

// Retain this import in the target owner: release cases deliberately use core runtime values while
// application cases use artifact-owned nominal values.
const _: Option<RuntimeValue> = None;
