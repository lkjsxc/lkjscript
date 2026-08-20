//! Typed build-target meaning and deterministic lowering from one exact project revision.

use crate::application::{
    ApplicationFieldValue, ApplicationImport, ApplicationTarget, ApplicationTestCase,
    ApplicationTestExpectation, ApplicationTrap, ApplicationTrapCode, ApplicationValue,
    HostInterface, HostOperation, HostOutcomeClass, HostOutcomeRoute, HostRequestRoute,
    InteractiveActionKind, InteractiveActionRoute, InteractiveActionRoutes,
    InteractiveApplicationProfile, InteractiveEventRoutes, InteractiveKeyRoutes,
    InteractiveMouseRoutes, InteractiveOpenRoutes, InvocationProfile, StatefulApplicationProfile,
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
#[serde(deny_unknown_fields)]
pub struct TargetInteractiveKeyRoutes {
    pub code: TargetItem,
    pub character_variant: TargetItem,
    pub enter_variant: TargetItem,
    pub backspace_variant: TargetItem,
    pub delete_variant: TargetItem,
    pub left_variant: TargetItem,
    pub right_variant: TargetItem,
    pub up_variant: TargetItem,
    pub down_variant: TargetItem,
    pub home_variant: TargetItem,
    pub end_variant: TargetItem,
    pub escape_variant: TargetItem,
    pub event: TargetItem,
    pub event_code_field: TargetItem,
    pub event_control_field: TargetItem,
    pub event_alt_field: TargetItem,
    pub event_shift_field: TargetItem,
    pub event_repeat_field: TargetItem,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInteractiveEventRoutes {
    pub event: TargetItem,
    pub key_variant: TargetItem,
    pub paste_variant: TargetItem,
    pub resize_variant: TargetItem,
    pub mouse_variant: TargetItem,
    pub focus_gained_variant: TargetItem,
    pub focus_lost_variant: TargetItem,
    pub open_variant: TargetItem,
    pub close_variant: TargetItem,
    pub size: TargetItem,
    pub size_rows_field: TargetItem,
    pub size_columns_field: TargetItem,
    pub scalars: TargetItem,
    pub key: TargetInteractiveKeyRoutes,
    pub mouse: TargetInteractiveMouseRoutes,
    pub open: TargetInteractiveOpenRoutes,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInteractiveMouseRoutes {
    pub button: TargetItem,
    pub button_none_variant: TargetItem,
    pub button_primary_variant: TargetItem,
    pub button_middle_variant: TargetItem,
    pub button_secondary_variant: TargetItem,
    pub kind: TargetItem,
    pub press_variant: TargetItem,
    pub release_variant: TargetItem,
    pub drag_variant: TargetItem,
    pub scroll_up_variant: TargetItem,
    pub scroll_down_variant: TargetItem,
    pub scroll_left_variant: TargetItem,
    pub scroll_right_variant: TargetItem,
    pub event: TargetItem,
    pub event_button_field: TargetItem,
    pub event_kind_field: TargetItem,
    pub event_row_field: TargetItem,
    pub event_column_field: TargetItem,
    pub event_control_field: TargetItem,
    pub event_alt_field: TargetItem,
    pub event_shift_field: TargetItem,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInteractiveOpenRoutes {
    pub event: TargetItem,
    pub event_path_field: TargetItem,
    pub event_directory_field: TargetItem,
    pub event_project_field: TargetItem,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInteractiveActionRoute {
    pub variant: TargetItem,
    pub kind: InteractiveActionKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInteractiveActionRoutes {
    pub action: TargetItem,
    pub update_action_field: TargetItem,
    pub update_action_id_field: TargetItem,
    pub routes: Vec<TargetInteractiveActionRoute>,
    pub file_save_payload: TargetItem,
    pub file_save_origin_field: TargetItem,
    pub file_save_content_field: TargetItem,
    pub file_save_create_field: TargetItem,
    pub file_search_payload: TargetItem,
    pub file_search_start_field: TargetItem,
    pub file_search_query_field: TargetItem,
    pub outcome: TargetItem,
    pub outcome_job_id_field: TargetItem,
    pub outcome_class_field: TargetItem,
    pub outcome_message_field: TargetItem,
    pub outcome_content_field: TargetItem,
    pub outcome_token_field: TargetItem,
    pub resume: TargetItem,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInteractiveApplicationProfile {
    pub version: u16,
    pub initialize: TargetItem,
    pub update: TargetItem,
    pub render: TargetItem,
    pub state: TargetItem,
    pub update_result: TargetItem,
    pub update_state_field: TargetItem,
    pub update_changed_field: TargetItem,
    pub update_exit_field: TargetItem,
    pub frame: TargetItem,
    pub frame_rows_field: TargetItem,
    pub frame_columns_field: TargetItem,
    pub frame_scalars_field: TargetItem,
    pub frame_styles_field: TargetItem,
    pub frame_cursor_row_field: TargetItem,
    pub frame_cursor_column_field: TargetItem,
    pub frame_cursor_visible_field: TargetItem,
    pub frame_cursor_shape_field: TargetItem,
    pub frame_status_field: TargetItem,
    pub frame_status_style_field: TargetItem,
    pub events: TargetInteractiveEventRoutes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Box<TargetInteractiveActionRoutes>>,
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
    Interactive(Box<TargetInteractiveApplicationProfile>),
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

#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum HistoricalBuildTargetDefinition<'a> {
    Application(HistoricalApplicationTargetDefinition<'a>),
}

#[derive(Serialize)]
struct HistoricalApplicationTargetDefinition<'a> {
    root_release: NodeId,
    entry: TargetItem,
    profile: HistoricalInvocationProfile<'a>,
    policy: &'a RunPolicy,
    tests: &'a [TargetApplicationTestCase],
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum HistoricalInvocationProfile<'a> {
    Interactive(HistoricalInteractiveProfile<'a>),
}

#[derive(Serialize)]
struct HistoricalInteractiveProfile<'a> {
    version: u16,
    initialize: TargetItem,
    update: TargetItem,
    render: TargetItem,
    state: TargetItem,
    update_result: TargetItem,
    update_state_field: TargetItem,
    update_changed_field: TargetItem,
    update_exit_field: TargetItem,
    frame: TargetItem,
    frame_rows_field: TargetItem,
    frame_columns_field: TargetItem,
    frame_scalars_field: TargetItem,
    frame_cursor_row_field: TargetItem,
    frame_cursor_column_field: TargetItem,
    frame_cursor_visible_field: TargetItem,
    frame_status_field: TargetItem,
    events: HistoricalInteractiveEventRoutes<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<HistoricalInteractiveActionRoutes<'a>>,
}

#[derive(Serialize)]
struct HistoricalInteractiveEventRoutes<'a> {
    event: TargetItem,
    key_variant: TargetItem,
    paste_variant: TargetItem,
    resize_variant: TargetItem,
    close_variant: TargetItem,
    size: TargetItem,
    size_rows_field: TargetItem,
    size_columns_field: TargetItem,
    scalars: TargetItem,
    key: &'a TargetInteractiveKeyRoutes,
}

#[derive(Serialize)]
struct HistoricalInteractiveActionRoutes<'a> {
    action: TargetItem,
    update_action_field: TargetItem,
    routes: &'a [TargetInteractiveActionRoute],
    file_save_payload: TargetItem,
    file_save_origin_field: TargetItem,
    file_save_content_field: TargetItem,
    file_save_create_field: TargetItem,
    outcome: TargetItem,
    outcome_class_field: TargetItem,
    outcome_message_field: TargetItem,
    outcome_content_field: TargetItem,
    outcome_token_field: TargetItem,
    resume: TargetItem,
}

pub(crate) fn encode_definition(definition: &BuildTargetDefinition) -> Result<Vec<u8>> {
    let bytes = if let BuildTargetDefinition::Application(application) = definition
        && let TargetInvocationProfile::Interactive(profile) = &application.profile
        && profile.version < crate::application::INTERACTIVE_PROFILE_VERSION
    {
        let actions = profile
            .actions
            .as_ref()
            .map(|actions| HistoricalInteractiveActionRoutes {
                action: actions.action,
                update_action_field: actions.update_action_field,
                routes: &actions.routes,
                file_save_payload: actions.file_save_payload,
                file_save_origin_field: actions.file_save_origin_field,
                file_save_content_field: actions.file_save_content_field,
                file_save_create_field: actions.file_save_create_field,
                outcome: actions.outcome,
                outcome_class_field: actions.outcome_class_field,
                outcome_message_field: actions.outcome_message_field,
                outcome_content_field: actions.outcome_content_field,
                outcome_token_field: actions.outcome_token_field,
                resume: actions.resume,
            });
        serde_json::to_vec(&HistoricalBuildTargetDefinition::Application(
            HistoricalApplicationTargetDefinition {
                root_release: application.root_release,
                entry: application.entry,
                profile: HistoricalInvocationProfile::Interactive(HistoricalInteractiveProfile {
                    version: profile.version,
                    initialize: profile.initialize,
                    update: profile.update,
                    render: profile.render,
                    state: profile.state,
                    update_result: profile.update_result,
                    update_state_field: profile.update_state_field,
                    update_changed_field: profile.update_changed_field,
                    update_exit_field: profile.update_exit_field,
                    frame: profile.frame,
                    frame_rows_field: profile.frame_rows_field,
                    frame_columns_field: profile.frame_columns_field,
                    frame_scalars_field: profile.frame_scalars_field,
                    frame_cursor_row_field: profile.frame_cursor_row_field,
                    frame_cursor_column_field: profile.frame_cursor_column_field,
                    frame_cursor_visible_field: profile.frame_cursor_visible_field,
                    frame_status_field: profile.frame_status_field,
                    events: HistoricalInteractiveEventRoutes {
                        event: profile.events.event,
                        key_variant: profile.events.key_variant,
                        paste_variant: profile.events.paste_variant,
                        resize_variant: profile.events.resize_variant,
                        close_variant: profile.events.close_variant,
                        size: profile.events.size,
                        size_rows_field: profile.events.size_rows_field,
                        size_columns_field: profile.events.size_columns_field,
                        scalars: profile.events.scalars,
                        key: &profile.events.key,
                    },
                    actions,
                }),
                policy: &application.policy,
                tests: &application.tests,
            },
        ))
    } else {
        serde_json::to_vec(definition)
    }
    .map_err(|error| {
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
    let (definition, historical): (BuildTargetDefinition, bool) =
        match serde_json::from_slice(bytes) {
            Ok(definition) => (definition, false),
            Err(current_error) => {
                let mut value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| {
                    LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        format!("canonical build-target decoding failed: {current_error}"),
                    )
                })?;
                if !complete_historical_interactive_v2(&mut value) {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        format!("canonical build-target decoding failed: {current_error}"),
                    ));
                }
                let definition = serde_json::from_value(value).map_err(|error| {
                    LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        format!("historical interactive target decoding failed: {error}"),
                    )
                })?;
                (definition, true)
            }
        };
    if encode_definition(&definition)? != bytes && !historical {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "build-target encoding is not canonical",
        ));
    }
    Ok(definition)
}

/// Completes only the fields needed to inspect immutable profile-1/2 history. The application owner
/// rejects these profiles before execution or current artifact production; this is not a compatibility
/// execution path. Outer snapshot digests continue to bind the exact retained legacy bytes.
fn complete_historical_interactive_v2(value: &mut serde_json::Value) -> bool {
    let Some(profile) = value
        .get_mut("data")
        .and_then(|value| value.get_mut("profile"))
        .and_then(|value| value.get_mut("data"))
        .and_then(serde_json::Value::as_object_mut)
    else {
        return false;
    };
    if !matches!(
        profile.get("version").and_then(serde_json::Value::as_u64),
        Some(1 | 2)
    ) {
        return false;
    }
    let Some(frame_scalars) = profile.get("frame_scalars_field").cloned() else {
        return false;
    };
    let Some(frame_cursor_visible) = profile.get("frame_cursor_visible_field").cloned() else {
        return false;
    };
    let Some(frame_status) = profile.get("frame_status_field").cloned() else {
        return false;
    };
    profile.entry("frame_styles_field").or_insert(frame_scalars);
    profile
        .entry("frame_cursor_shape_field")
        .or_insert(frame_cursor_visible);
    profile
        .entry("frame_status_style_field")
        .or_insert(frame_status);

    let Some(events) = profile
        .get_mut("events")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return false;
    };
    let Some(event) = events.get("event").cloned() else {
        return false;
    };
    let Some(close_variant) = events.get("close_variant").cloned() else {
        return false;
    };
    for field in [
        "mouse_variant",
        "focus_gained_variant",
        "focus_lost_variant",
        "open_variant",
    ] {
        events.entry(field).or_insert(close_variant.clone());
    }
    let mut mouse = serde_json::Map::new();
    for field in [
        "button",
        "button_none_variant",
        "button_primary_variant",
        "button_middle_variant",
        "button_secondary_variant",
        "kind",
        "press_variant",
        "release_variant",
        "drag_variant",
        "scroll_up_variant",
        "scroll_down_variant",
        "scroll_left_variant",
        "scroll_right_variant",
        "event",
        "event_button_field",
        "event_kind_field",
        "event_row_field",
        "event_column_field",
        "event_control_field",
        "event_alt_field",
        "event_shift_field",
    ] {
        mouse.insert(field.to_owned(), event.clone());
    }
    events
        .entry("mouse")
        .or_insert(serde_json::Value::Object(mouse));
    let mut open = serde_json::Map::new();
    for field in [
        "event",
        "event_path_field",
        "event_directory_field",
        "event_project_field",
    ] {
        open.insert(field.to_owned(), event.clone());
    }
    events
        .entry("open")
        .or_insert(serde_json::Value::Object(open));

    let Some(actions) = profile
        .get_mut("actions")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return true;
    };
    let Some(update_action) = actions.get("update_action_field").cloned() else {
        return false;
    };
    let Some(save_payload) = actions.get("file_save_payload").cloned() else {
        return false;
    };
    let Some(save_origin) = actions.get("file_save_origin_field").cloned() else {
        return false;
    };
    let Some(save_content) = actions.get("file_save_content_field").cloned() else {
        return false;
    };
    let Some(outcome_class) = actions.get("outcome_class_field").cloned() else {
        return false;
    };
    actions
        .entry("update_action_id_field")
        .or_insert(update_action);
    actions.entry("file_search_payload").or_insert(save_payload);
    actions
        .entry("file_search_start_field")
        .or_insert(save_origin);
    actions
        .entry("file_search_query_field")
        .or_insert(save_content);
    actions
        .entry("outcome_job_id_field")
        .or_insert(outcome_class);
    true
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
        if !historical_interactive_target(snapshot, summary.target)? {
            prepare(snapshot, summary.target)?;
        }
    }
    Ok(())
}

pub(crate) fn historical_interactive_target(snapshot: &Snapshot, target: NodeId) -> Result<bool> {
    let Node::BuildTarget { definition, .. } = snapshot.node(target)? else {
        return Ok(false);
    };
    match definition {
        BuildTargetDefinition::Application(application) => Ok(matches!(
            &application.profile,
            TargetInvocationProfile::Interactive(profile)
                if profile.version < crate::application::INTERACTIVE_PROFILE_VERSION
        )),
        BuildTargetDefinition::Product(product) => {
            historical_interactive_target(snapshot, product.application)
        }
        BuildTargetDefinition::Release(_) => Ok(false),
    }
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
    match profile {
        TargetInvocationProfile::Typed | TargetInvocationProfile::BytesStream => {}
        TargetInvocationProfile::Stateful(profile) => {
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
        }
        TargetInvocationProfile::Interactive(profile) => {
            for item in interactive_profile_items(profile) {
                visit(item)?;
            }
        }
    }
    Ok(())
}

fn interactive_profile_items(profile: &TargetInteractiveApplicationProfile) -> Vec<TargetItem> {
    let mut items = vec![
        profile.initialize,
        profile.update,
        profile.render,
        profile.state,
        profile.update_result,
        profile.update_state_field,
        profile.update_changed_field,
        profile.update_exit_field,
        profile.frame,
        profile.frame_rows_field,
        profile.frame_columns_field,
        profile.frame_scalars_field,
        profile.frame_cursor_row_field,
        profile.frame_cursor_column_field,
        profile.frame_cursor_visible_field,
        profile.frame_status_field,
        profile.events.event,
        profile.events.key_variant,
        profile.events.paste_variant,
        profile.events.resize_variant,
        profile.events.close_variant,
        profile.events.size,
        profile.events.size_rows_field,
        profile.events.size_columns_field,
        profile.events.scalars,
        profile.events.key.code,
        profile.events.key.character_variant,
        profile.events.key.enter_variant,
        profile.events.key.backspace_variant,
        profile.events.key.delete_variant,
        profile.events.key.left_variant,
        profile.events.key.right_variant,
        profile.events.key.up_variant,
        profile.events.key.down_variant,
        profile.events.key.home_variant,
        profile.events.key.end_variant,
        profile.events.key.escape_variant,
        profile.events.key.event,
        profile.events.key.event_code_field,
        profile.events.key.event_control_field,
        profile.events.key.event_alt_field,
        profile.events.key.event_shift_field,
        profile.events.key.event_repeat_field,
    ];
    if profile.version >= crate::application::INTERACTIVE_PROFILE_VERSION {
        items.extend([
            profile.frame_styles_field,
            profile.frame_cursor_shape_field,
            profile.frame_status_style_field,
            profile.events.mouse_variant,
            profile.events.focus_gained_variant,
            profile.events.focus_lost_variant,
            profile.events.open_variant,
            profile.events.mouse.button,
            profile.events.mouse.button_none_variant,
            profile.events.mouse.button_primary_variant,
            profile.events.mouse.button_middle_variant,
            profile.events.mouse.button_secondary_variant,
            profile.events.mouse.kind,
            profile.events.mouse.press_variant,
            profile.events.mouse.release_variant,
            profile.events.mouse.drag_variant,
            profile.events.mouse.scroll_up_variant,
            profile.events.mouse.scroll_down_variant,
            profile.events.mouse.scroll_left_variant,
            profile.events.mouse.scroll_right_variant,
            profile.events.mouse.event,
            profile.events.mouse.event_button_field,
            profile.events.mouse.event_kind_field,
            profile.events.mouse.event_row_field,
            profile.events.mouse.event_column_field,
            profile.events.mouse.event_control_field,
            profile.events.mouse.event_alt_field,
            profile.events.mouse.event_shift_field,
            profile.events.open.event,
            profile.events.open.event_path_field,
            profile.events.open.event_directory_field,
            profile.events.open.event_project_field,
        ]);
    }
    if let Some(actions) = &profile.actions {
        items.extend([
            actions.action,
            actions.update_action_field,
            actions.file_save_payload,
            actions.file_save_origin_field,
            actions.file_save_content_field,
            actions.file_save_create_field,
            actions.outcome,
            actions.outcome_class_field,
            actions.outcome_message_field,
            actions.outcome_content_field,
            actions.outcome_token_field,
            actions.resume,
        ]);
        if profile.version >= crate::application::INTERACTIVE_PROFILE_VERSION {
            items.extend([
                actions.update_action_id_field,
                actions.file_search_payload,
                actions.file_search_start_field,
                actions.file_search_query_field,
                actions.outcome_job_id_field,
            ]);
        }
        items.extend(actions.routes.iter().map(|route| route.variant));
    }
    items
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
        TargetInvocationProfile::Interactive(profile) => {
            let item = |value| lower_item(value, releases);
            InvocationProfile::Interactive(Box::new(InteractiveApplicationProfile {
                version: profile.version,
                initialize: item(profile.initialize)?,
                update: item(profile.update)?,
                render: item(profile.render)?,
                state: item(profile.state)?,
                update_result: item(profile.update_result)?,
                update_state_field: item(profile.update_state_field)?,
                update_changed_field: item(profile.update_changed_field)?,
                update_exit_field: item(profile.update_exit_field)?,
                frame: item(profile.frame)?,
                frame_rows_field: item(profile.frame_rows_field)?,
                frame_columns_field: item(profile.frame_columns_field)?,
                frame_scalars_field: item(profile.frame_scalars_field)?,
                frame_styles_field: item(profile.frame_styles_field)?,
                frame_cursor_row_field: item(profile.frame_cursor_row_field)?,
                frame_cursor_column_field: item(profile.frame_cursor_column_field)?,
                frame_cursor_visible_field: item(profile.frame_cursor_visible_field)?,
                frame_cursor_shape_field: item(profile.frame_cursor_shape_field)?,
                frame_status_field: item(profile.frame_status_field)?,
                frame_status_style_field: item(profile.frame_status_style_field)?,
                events: InteractiveEventRoutes {
                    event: item(profile.events.event)?,
                    key_variant: item(profile.events.key_variant)?,
                    paste_variant: item(profile.events.paste_variant)?,
                    resize_variant: item(profile.events.resize_variant)?,
                    mouse_variant: item(profile.events.mouse_variant)?,
                    focus_gained_variant: item(profile.events.focus_gained_variant)?,
                    focus_lost_variant: item(profile.events.focus_lost_variant)?,
                    open_variant: item(profile.events.open_variant)?,
                    close_variant: item(profile.events.close_variant)?,
                    size: item(profile.events.size)?,
                    size_rows_field: item(profile.events.size_rows_field)?,
                    size_columns_field: item(profile.events.size_columns_field)?,
                    scalars: item(profile.events.scalars)?,
                    key: InteractiveKeyRoutes {
                        code: item(profile.events.key.code)?,
                        character_variant: item(profile.events.key.character_variant)?,
                        enter_variant: item(profile.events.key.enter_variant)?,
                        backspace_variant: item(profile.events.key.backspace_variant)?,
                        delete_variant: item(profile.events.key.delete_variant)?,
                        left_variant: item(profile.events.key.left_variant)?,
                        right_variant: item(profile.events.key.right_variant)?,
                        up_variant: item(profile.events.key.up_variant)?,
                        down_variant: item(profile.events.key.down_variant)?,
                        home_variant: item(profile.events.key.home_variant)?,
                        end_variant: item(profile.events.key.end_variant)?,
                        escape_variant: item(profile.events.key.escape_variant)?,
                        event: item(profile.events.key.event)?,
                        event_code_field: item(profile.events.key.event_code_field)?,
                        event_control_field: item(profile.events.key.event_control_field)?,
                        event_alt_field: item(profile.events.key.event_alt_field)?,
                        event_shift_field: item(profile.events.key.event_shift_field)?,
                        event_repeat_field: item(profile.events.key.event_repeat_field)?,
                    },
                    mouse: InteractiveMouseRoutes {
                        button: item(profile.events.mouse.button)?,
                        button_none_variant: item(profile.events.mouse.button_none_variant)?,
                        button_primary_variant: item(profile.events.mouse.button_primary_variant)?,
                        button_middle_variant: item(profile.events.mouse.button_middle_variant)?,
                        button_secondary_variant: item(
                            profile.events.mouse.button_secondary_variant,
                        )?,
                        kind: item(profile.events.mouse.kind)?,
                        press_variant: item(profile.events.mouse.press_variant)?,
                        release_variant: item(profile.events.mouse.release_variant)?,
                        drag_variant: item(profile.events.mouse.drag_variant)?,
                        scroll_up_variant: item(profile.events.mouse.scroll_up_variant)?,
                        scroll_down_variant: item(profile.events.mouse.scroll_down_variant)?,
                        scroll_left_variant: item(profile.events.mouse.scroll_left_variant)?,
                        scroll_right_variant: item(profile.events.mouse.scroll_right_variant)?,
                        event: item(profile.events.mouse.event)?,
                        event_button_field: item(profile.events.mouse.event_button_field)?,
                        event_kind_field: item(profile.events.mouse.event_kind_field)?,
                        event_row_field: item(profile.events.mouse.event_row_field)?,
                        event_column_field: item(profile.events.mouse.event_column_field)?,
                        event_control_field: item(profile.events.mouse.event_control_field)?,
                        event_alt_field: item(profile.events.mouse.event_alt_field)?,
                        event_shift_field: item(profile.events.mouse.event_shift_field)?,
                    },
                    open: InteractiveOpenRoutes {
                        event: item(profile.events.open.event)?,
                        event_path_field: item(profile.events.open.event_path_field)?,
                        event_directory_field: item(profile.events.open.event_directory_field)?,
                        event_project_field: item(profile.events.open.event_project_field)?,
                    },
                },
                actions: profile
                    .actions
                    .as_ref()
                    .map(|actions| {
                        Ok::<Box<InteractiveActionRoutes>, LkError>(Box::new(
                            InteractiveActionRoutes {
                                action: item(actions.action)?,
                                update_action_field: item(actions.update_action_field)?,
                                update_action_id_field: item(actions.update_action_id_field)?,
                                routes: actions
                                    .routes
                                    .iter()
                                    .map(|route| {
                                        Ok(InteractiveActionRoute {
                                            variant: item(route.variant)?,
                                            kind: route.kind,
                                        })
                                    })
                                    .collect::<Result<Vec<_>>>()?,
                                file_save_payload: item(actions.file_save_payload)?,
                                file_save_origin_field: item(actions.file_save_origin_field)?,
                                file_save_content_field: item(actions.file_save_content_field)?,
                                file_save_create_field: item(actions.file_save_create_field)?,
                                file_search_payload: item(actions.file_search_payload)?,
                                file_search_start_field: item(actions.file_search_start_field)?,
                                file_search_query_field: item(actions.file_search_query_field)?,
                                outcome: item(actions.outcome)?,
                                outcome_job_id_field: item(actions.outcome_job_id_field)?,
                                outcome_class_field: item(actions.outcome_class_field)?,
                                outcome_message_field: item(actions.outcome_message_field)?,
                                outcome_content_field: item(actions.outcome_content_field)?,
                                outcome_token_field: item(actions.outcome_token_field)?,
                                resume: item(actions.resume)?,
                            },
                        ))
                    })
                    .transpose()?,
            }))
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
