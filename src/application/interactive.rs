//! Pure foreground interactive application execution.
//!
//! The application owns state transitions, key policy, and full-frame meaning. Native callers
//! own only event adaptation, frame projection, and live-resource lifecycle. Session state is
//! deliberately ephemeral and disappears with the owning process.

use super::*;
use crate::graph::Snapshot;
use crate::ids::NodeId;

pub const INTERACTIVE_PROFILE_VERSION: u16 = 3;
pub const MAXIMUM_INTERACTIVE_ROWS: i64 = 1_000;
pub const MAXIMUM_INTERACTIVE_COLUMNS: i64 = 1_000;
pub const MAXIMUM_INTERACTIVE_FRAME_SCALARS: usize = crate::schema::MAXIMUM_SEQUENCE_ELEMENTS;
pub const MAXIMUM_INTERACTIVE_PASTE_SCALARS: usize = 65_536;
pub const MAXIMUM_INTERACTIVE_STATUS_BYTES: usize = 4_096;
pub const MAXIMUM_INTERACTIVE_JOB_ID: u64 = i64::MAX as u64;
pub const MAXIMUM_INTERACTIVE_STYLE: i64 = 15;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveKeyRoutes {
    pub code: ApplicationTarget,
    pub character_variant: ApplicationTarget,
    pub enter_variant: ApplicationTarget,
    pub backspace_variant: ApplicationTarget,
    pub delete_variant: ApplicationTarget,
    pub left_variant: ApplicationTarget,
    pub right_variant: ApplicationTarget,
    pub up_variant: ApplicationTarget,
    pub down_variant: ApplicationTarget,
    pub home_variant: ApplicationTarget,
    pub end_variant: ApplicationTarget,
    pub escape_variant: ApplicationTarget,
    pub event: ApplicationTarget,
    pub event_code_field: ApplicationTarget,
    pub event_control_field: ApplicationTarget,
    pub event_alt_field: ApplicationTarget,
    pub event_shift_field: ApplicationTarget,
    pub event_repeat_field: ApplicationTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveEventRoutes {
    pub event: ApplicationTarget,
    pub key_variant: ApplicationTarget,
    pub paste_variant: ApplicationTarget,
    pub resize_variant: ApplicationTarget,
    pub mouse_variant: ApplicationTarget,
    pub focus_gained_variant: ApplicationTarget,
    pub focus_lost_variant: ApplicationTarget,
    pub open_variant: ApplicationTarget,
    pub close_variant: ApplicationTarget,
    pub size: ApplicationTarget,
    pub size_rows_field: ApplicationTarget,
    pub size_columns_field: ApplicationTarget,
    pub scalars: ApplicationTarget,
    pub key: InteractiveKeyRoutes,
    pub mouse: InteractiveMouseRoutes,
    pub open: InteractiveOpenRoutes,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveMouseRoutes {
    pub button: ApplicationTarget,
    pub button_none_variant: ApplicationTarget,
    pub button_primary_variant: ApplicationTarget,
    pub button_middle_variant: ApplicationTarget,
    pub button_secondary_variant: ApplicationTarget,
    pub kind: ApplicationTarget,
    pub press_variant: ApplicationTarget,
    pub release_variant: ApplicationTarget,
    pub drag_variant: ApplicationTarget,
    pub scroll_up_variant: ApplicationTarget,
    pub scroll_down_variant: ApplicationTarget,
    pub scroll_left_variant: ApplicationTarget,
    pub scroll_right_variant: ApplicationTarget,
    pub event: ApplicationTarget,
    pub event_button_field: ApplicationTarget,
    pub event_kind_field: ApplicationTarget,
    pub event_row_field: ApplicationTarget,
    pub event_column_field: ApplicationTarget,
    pub event_control_field: ApplicationTarget,
    pub event_alt_field: ApplicationTarget,
    pub event_shift_field: ApplicationTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveOpenRoutes {
    pub event: ApplicationTarget,
    pub event_path_field: ApplicationTarget,
    pub event_directory_field: ApplicationTarget,
    pub event_project_field: ApplicationTarget,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveActionKind {
    None,
    ProjectOrient,
    ProjectSummary,
    ProjectChildren,
    ProjectFunction,
    ProjectCallers,
    ProjectCallees,
    ProjectTargets,
    ProjectBlockers,
    ProjectProposal,
    ProjectHistory,
    ProjectRecord,
    ProjectDiff,
    ProjectValidate,
    ProjectApply,
    ProjectTargetList,
    ProjectTargetTest,
    ProjectTargetBuild,
    ProjectTargetRun,
    FilesystemList,
    FilesystemSearch,
    FilesystemRead,
    FilesystemSave,
    FilesystemReconcile,
}

impl InteractiveActionKind {
    pub const ALL: [Self; 24] = [
        Self::None,
        Self::ProjectOrient,
        Self::ProjectSummary,
        Self::ProjectChildren,
        Self::ProjectFunction,
        Self::ProjectCallers,
        Self::ProjectCallees,
        Self::ProjectTargets,
        Self::ProjectBlockers,
        Self::ProjectProposal,
        Self::ProjectHistory,
        Self::ProjectRecord,
        Self::ProjectDiff,
        Self::ProjectValidate,
        Self::ProjectApply,
        Self::ProjectTargetList,
        Self::ProjectTargetTest,
        Self::ProjectTargetBuild,
        Self::ProjectTargetRun,
        Self::FilesystemList,
        Self::FilesystemSearch,
        Self::FilesystemRead,
        Self::FilesystemSave,
        Self::FilesystemReconcile,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ProjectOrient => "project_orient",
            Self::ProjectSummary => "project_summary",
            Self::ProjectChildren => "project_children",
            Self::ProjectFunction => "project_function",
            Self::ProjectCallers => "project_callers",
            Self::ProjectCallees => "project_callees",
            Self::ProjectTargets => "project_targets",
            Self::ProjectBlockers => "project_blockers",
            Self::ProjectProposal => "project_proposal",
            Self::ProjectHistory => "project_history",
            Self::ProjectRecord => "project_record",
            Self::ProjectDiff => "project_diff",
            Self::ProjectValidate => "project_validate",
            Self::ProjectApply => "project_apply",
            Self::ProjectTargetList => "project_target_list",
            Self::ProjectTargetTest => "project_target_test",
            Self::ProjectTargetBuild => "project_target_build",
            Self::ProjectTargetRun => "project_target_run",
            Self::FilesystemList => "filesystem_list",
            Self::FilesystemSearch => "filesystem_search",
            Self::FilesystemRead => "filesystem_read",
            Self::FilesystemSave => "filesystem_save",
            Self::FilesystemReconcile => "filesystem_reconcile",
        }
    }

    pub(super) const fn stable_tag(self) -> u8 {
        match self {
            Self::None => 1,
            Self::ProjectOrient => 2,
            Self::ProjectSummary => 3,
            Self::ProjectChildren => 4,
            Self::ProjectFunction => 5,
            Self::ProjectCallers => 6,
            Self::ProjectCallees => 7,
            Self::ProjectTargets => 8,
            Self::ProjectBlockers => 9,
            Self::ProjectProposal => 10,
            Self::ProjectHistory => 11,
            Self::ProjectRecord => 12,
            Self::ProjectDiff => 13,
            Self::ProjectValidate => 14,
            Self::ProjectApply => 15,
            Self::ProjectTargetList => 16,
            Self::ProjectTargetTest => 17,
            Self::ProjectTargetBuild => 18,
            Self::ProjectTargetRun => 19,
            Self::FilesystemList => 20,
            Self::FilesystemSearch => 21,
            Self::FilesystemRead => 22,
            Self::FilesystemSave => 23,
            Self::FilesystemReconcile => 24,
        }
    }

    pub(super) const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::None),
            2 => Some(Self::ProjectOrient),
            3 => Some(Self::ProjectSummary),
            4 => Some(Self::ProjectChildren),
            5 => Some(Self::ProjectFunction),
            6 => Some(Self::ProjectCallers),
            7 => Some(Self::ProjectCallees),
            8 => Some(Self::ProjectTargets),
            9 => Some(Self::ProjectBlockers),
            10 => Some(Self::ProjectProposal),
            11 => Some(Self::ProjectHistory),
            12 => Some(Self::ProjectRecord),
            13 => Some(Self::ProjectDiff),
            14 => Some(Self::ProjectValidate),
            15 => Some(Self::ProjectApply),
            16 => Some(Self::ProjectTargetList),
            17 => Some(Self::ProjectTargetTest),
            18 => Some(Self::ProjectTargetBuild),
            19 => Some(Self::ProjectTargetRun),
            20 => Some(Self::FilesystemList),
            21 => Some(Self::FilesystemSearch),
            22 => Some(Self::FilesystemRead),
            23 => Some(Self::FilesystemSave),
            24 => Some(Self::FilesystemReconcile),
            _ => None,
        }
    }

    const fn requires_scalars(self) -> bool {
        !matches!(
            self,
            Self::None
                | Self::ProjectOrient
                | Self::ProjectBlockers
                | Self::ProjectHistory
                | Self::ProjectTargetList
                | Self::FilesystemSave
                | Self::FilesystemReconcile
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveActionRoute {
    pub variant: ApplicationTarget,
    pub kind: InteractiveActionKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveActionRoutes {
    pub action: ApplicationTarget,
    pub update_action_field: ApplicationTarget,
    pub update_action_id_field: ApplicationTarget,
    pub routes: Vec<InteractiveActionRoute>,
    pub file_save_payload: ApplicationTarget,
    pub file_save_origin_field: ApplicationTarget,
    pub file_save_content_field: ApplicationTarget,
    pub file_save_create_field: ApplicationTarget,
    pub file_search_payload: ApplicationTarget,
    pub file_search_start_field: ApplicationTarget,
    pub file_search_query_field: ApplicationTarget,
    pub outcome: ApplicationTarget,
    pub outcome_job_id_field: ApplicationTarget,
    pub outcome_class_field: ApplicationTarget,
    pub outcome_message_field: ApplicationTarget,
    pub outcome_content_field: ApplicationTarget,
    pub outcome_token_field: ApplicationTarget,
    pub resume: ApplicationTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveApplicationProfile {
    pub version: u16,
    pub initialize: ApplicationTarget,
    pub update: ApplicationTarget,
    pub render: ApplicationTarget,
    pub state: ApplicationTarget,
    pub update_result: ApplicationTarget,
    pub update_state_field: ApplicationTarget,
    pub update_changed_field: ApplicationTarget,
    pub update_exit_field: ApplicationTarget,
    pub frame: ApplicationTarget,
    pub frame_rows_field: ApplicationTarget,
    pub frame_columns_field: ApplicationTarget,
    pub frame_scalars_field: ApplicationTarget,
    pub frame_styles_field: ApplicationTarget,
    pub frame_cursor_row_field: ApplicationTarget,
    pub frame_cursor_column_field: ApplicationTarget,
    pub frame_cursor_visible_field: ApplicationTarget,
    pub frame_cursor_shape_field: ApplicationTarget,
    pub frame_status_field: ApplicationTarget,
    pub frame_status_style_field: ApplicationTarget,
    pub events: InteractiveEventRoutes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Box<InteractiveActionRoutes>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveKeyCode {
    Character(u32),
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveKeyEvent {
    pub code: InteractiveKeyCode,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub repeat: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveMouseButton {
    None,
    Primary,
    Middle,
    Secondary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveMouseKind {
    Press,
    Release,
    Drag,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveMouseEvent {
    pub button: InteractiveMouseButton,
    pub kind: InteractiveMouseKind,
    pub row: i64,
    pub column: i64,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveOpenEvent {
    pub path: String,
    pub directory: bool,
    pub project: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum InteractiveEvent {
    Key(InteractiveKeyEvent),
    Paste(Vec<u32>),
    Resize { rows: i64, columns: i64 },
    Mouse(InteractiveMouseEvent),
    FocusGained,
    FocusLost,
    Open(InteractiveOpenEvent),
    Close,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum InteractiveAction {
    ProjectOrient,
    ProjectSummary(String),
    ProjectChildren(String),
    ProjectFunction(String),
    ProjectCallers(String),
    ProjectCallees(String),
    ProjectTargets(String),
    ProjectBlockers,
    ProjectProposal(String),
    ProjectHistory,
    ProjectRecord(String),
    ProjectDiff(String),
    ProjectValidate(String),
    ProjectApply(String),
    ProjectTargetList,
    ProjectTargetTest(String),
    ProjectTargetBuild(String),
    ProjectTargetRun(String),
    FilesystemList(String),
    FilesystemSearch {
        start: String,
        query: String,
    },
    FilesystemRead(String),
    FilesystemSave {
        origin: String,
        content: String,
        create: bool,
    },
    FilesystemReconcile(String),
}

impl InteractiveAction {
    pub const fn kind(&self) -> InteractiveActionKind {
        match self {
            Self::ProjectOrient => InteractiveActionKind::ProjectOrient,
            Self::ProjectSummary(_) => InteractiveActionKind::ProjectSummary,
            Self::ProjectChildren(_) => InteractiveActionKind::ProjectChildren,
            Self::ProjectFunction(_) => InteractiveActionKind::ProjectFunction,
            Self::ProjectCallers(_) => InteractiveActionKind::ProjectCallers,
            Self::ProjectCallees(_) => InteractiveActionKind::ProjectCallees,
            Self::ProjectTargets(_) => InteractiveActionKind::ProjectTargets,
            Self::ProjectBlockers => InteractiveActionKind::ProjectBlockers,
            Self::ProjectProposal(_) => InteractiveActionKind::ProjectProposal,
            Self::ProjectHistory => InteractiveActionKind::ProjectHistory,
            Self::ProjectRecord(_) => InteractiveActionKind::ProjectRecord,
            Self::ProjectDiff(_) => InteractiveActionKind::ProjectDiff,
            Self::ProjectValidate(_) => InteractiveActionKind::ProjectValidate,
            Self::ProjectApply(_) => InteractiveActionKind::ProjectApply,
            Self::ProjectTargetList => InteractiveActionKind::ProjectTargetList,
            Self::ProjectTargetTest(_) => InteractiveActionKind::ProjectTargetTest,
            Self::ProjectTargetBuild(_) => InteractiveActionKind::ProjectTargetBuild,
            Self::ProjectTargetRun(_) => InteractiveActionKind::ProjectTargetRun,
            Self::FilesystemList(_) => InteractiveActionKind::FilesystemList,
            Self::FilesystemSearch { .. } => InteractiveActionKind::FilesystemSearch,
            Self::FilesystemRead(_) => InteractiveActionKind::FilesystemRead,
            Self::FilesystemSave { .. } => InteractiveActionKind::FilesystemSave,
            Self::FilesystemReconcile(_) => InteractiveActionKind::FilesystemReconcile,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveActionRequest {
    pub job_id: u64,
    pub action: InteractiveAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveActionOutcomeClass {
    Succeeded,
    Unchanged,
    Rejected,
    Conflict,
    Unknown,
}

impl InteractiveActionOutcomeClass {
    const fn stable_value(self) -> i64 {
        match self {
            Self::Succeeded => 1,
            Self::Unchanged => 2,
            Self::Rejected => 3,
            Self::Conflict => 4,
            Self::Unknown => 5,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveActionOutcome {
    pub job_id: u64,
    pub class: InteractiveActionOutcomeClass,
    pub message: String,
    pub content: String,
    pub token: String,
}

impl InteractiveActionOutcome {
    pub const fn with_job_id(mut self, job_id: u64) -> Self {
        self.job_id = job_id;
        self
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveCursorShape {
    Block,
    Bar,
    Underline,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveFrame {
    pub rows: i64,
    pub columns: i64,
    pub scalars: Vec<u32>,
    pub styles: Vec<u8>,
    pub cursor_row: i64,
    pub cursor_column: i64,
    pub cursor_visible: bool,
    pub cursor_shape: InteractiveCursorShape,
    pub status: String,
    pub status_style: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveExecutionObservation {
    pub execute_nanoseconds: u64,
    pub public_value_nanoseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveStep {
    pub frame: InteractiveFrame,
    pub changed: bool,
    pub exit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<InteractiveActionRequest>,
    pub observation: InteractiveExecutionObservation,
}

pub struct PreparedInteractiveApplication {
    application: DecodedApplication,
    profile: InteractiveApplicationProfile,
    initialize_entry: NodeId,
    update_entry: NodeId,
    render_entry: NodeId,
    resume_entry: Option<NodeId>,
    initialize_program: interpret::PreparedProgram,
    update_program: interpret::PreparedProgram,
    render_program: interpret::PreparedProgram,
    resume_program: Option<interpret::PreparedProgram>,
}

pub struct InteractiveSession {
    prepared: PreparedInteractiveApplication,
    state: RuntimeValue,
    pending_action: Option<u64>,
    last_action_id: u64,
}

pub(super) fn put_profile(writer: &mut Writer, profile: &InteractiveApplicationProfile) {
    writer.u16(profile.version);
    for target in [
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
        profile.frame_styles_field,
        profile.frame_cursor_row_field,
        profile.frame_cursor_column_field,
        profile.frame_cursor_visible_field,
        profile.frame_cursor_shape_field,
        profile.frame_status_field,
        profile.frame_status_style_field,
        profile.events.event,
        profile.events.key_variant,
        profile.events.paste_variant,
        profile.events.resize_variant,
        profile.events.mouse_variant,
        profile.events.focus_gained_variant,
        profile.events.focus_lost_variant,
        profile.events.open_variant,
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
    ] {
        put_target(writer, target);
    }
    writer.bool(profile.actions.is_some());
    if let Some(actions) = &profile.actions {
        for target in [
            actions.action,
            actions.update_action_field,
            actions.update_action_id_field,
            actions.file_save_payload,
            actions.file_save_origin_field,
            actions.file_save_content_field,
            actions.file_save_create_field,
            actions.file_search_payload,
            actions.file_search_start_field,
            actions.file_search_query_field,
            actions.outcome,
            actions.outcome_job_id_field,
            actions.outcome_class_field,
            actions.outcome_message_field,
            actions.outcome_content_field,
            actions.outcome_token_field,
            actions.resume,
        ] {
            put_target(writer, target);
        }
        writer.u64(u64::try_from(actions.routes.len()).unwrap_or(u64::MAX));
        for route in &actions.routes {
            put_target(writer, route.variant);
            writer.u8(route.kind.stable_tag());
        }
    }
}

pub(super) fn read_profile(reader: &mut Reader<'_>) -> Result<InteractiveApplicationProfile> {
    let version = reader.u16().map_err(application_codec)?;
    let mut next = || read_target(reader);
    let mut profile = InteractiveApplicationProfile {
        version,
        initialize: next()?,
        update: next()?,
        render: next()?,
        state: next()?,
        update_result: next()?,
        update_state_field: next()?,
        update_changed_field: next()?,
        update_exit_field: next()?,
        frame: next()?,
        frame_rows_field: next()?,
        frame_columns_field: next()?,
        frame_scalars_field: next()?,
        frame_styles_field: next()?,
        frame_cursor_row_field: next()?,
        frame_cursor_column_field: next()?,
        frame_cursor_visible_field: next()?,
        frame_cursor_shape_field: next()?,
        frame_status_field: next()?,
        frame_status_style_field: next()?,
        events: InteractiveEventRoutes {
            event: next()?,
            key_variant: next()?,
            paste_variant: next()?,
            resize_variant: next()?,
            mouse_variant: next()?,
            focus_gained_variant: next()?,
            focus_lost_variant: next()?,
            open_variant: next()?,
            close_variant: next()?,
            size: next()?,
            size_rows_field: next()?,
            size_columns_field: next()?,
            scalars: next()?,
            key: InteractiveKeyRoutes {
                code: next()?,
                character_variant: next()?,
                enter_variant: next()?,
                backspace_variant: next()?,
                delete_variant: next()?,
                left_variant: next()?,
                right_variant: next()?,
                up_variant: next()?,
                down_variant: next()?,
                home_variant: next()?,
                end_variant: next()?,
                escape_variant: next()?,
                event: next()?,
                event_code_field: next()?,
                event_control_field: next()?,
                event_alt_field: next()?,
                event_shift_field: next()?,
                event_repeat_field: next()?,
            },
            mouse: InteractiveMouseRoutes {
                button: next()?,
                button_none_variant: next()?,
                button_primary_variant: next()?,
                button_middle_variant: next()?,
                button_secondary_variant: next()?,
                kind: next()?,
                press_variant: next()?,
                release_variant: next()?,
                drag_variant: next()?,
                scroll_up_variant: next()?,
                scroll_down_variant: next()?,
                scroll_left_variant: next()?,
                scroll_right_variant: next()?,
                event: next()?,
                event_button_field: next()?,
                event_kind_field: next()?,
                event_row_field: next()?,
                event_column_field: next()?,
                event_control_field: next()?,
                event_alt_field: next()?,
                event_shift_field: next()?,
            },
            open: InteractiveOpenRoutes {
                event: next()?,
                event_path_field: next()?,
                event_directory_field: next()?,
                event_project_field: next()?,
            },
        },
        actions: None,
    };
    if reader.bool().map_err(application_codec)? {
        let action = read_target(reader)?;
        let update_action_field = read_target(reader)?;
        let update_action_id_field = read_target(reader)?;
        let file_save_payload = read_target(reader)?;
        let file_save_origin_field = read_target(reader)?;
        let file_save_content_field = read_target(reader)?;
        let file_save_create_field = read_target(reader)?;
        let file_search_payload = read_target(reader)?;
        let file_search_start_field = read_target(reader)?;
        let file_search_query_field = read_target(reader)?;
        let outcome = read_target(reader)?;
        let outcome_job_id_field = read_target(reader)?;
        let outcome_class_field = read_target(reader)?;
        let outcome_message_field = read_target(reader)?;
        let outcome_content_field = read_target(reader)?;
        let outcome_token_field = read_target(reader)?;
        let resume = read_target(reader)?;
        let count = reader
            .count(InteractiveActionKind::ALL.len())
            .map_err(application_codec)?;
        let mut routes = Vec::with_capacity(count);
        for _ in 0..count {
            let variant = read_target(reader)?;
            let tag = reader.u8().map_err(application_codec)?;
            let kind = InteractiveActionKind::from_stable_tag(tag).ok_or_else(|| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "application contains an unknown interactive action kind",
                )
            })?;
            routes.push(InteractiveActionRoute { variant, kind });
        }
        profile.actions = Some(Box::new(InteractiveActionRoutes {
            action,
            update_action_field,
            update_action_id_field,
            routes,
            file_save_payload,
            file_save_origin_field,
            file_save_content_field,
            file_save_create_field,
            file_search_payload,
            file_search_start_field,
            file_search_query_field,
            outcome,
            outcome_job_id_field,
            outcome_class_field,
            outcome_message_field,
            outcome_content_field,
            outcome_token_field,
            resume,
        }));
    }
    Ok(profile)
}

impl PreparedInteractiveApplication {
    pub fn start(self, rows: i64, columns: i64) -> Result<(InteractiveSession, InteractiveStep)> {
        validate_dimensions(rows, columns)?;
        let (state, initialize_observation) = self.run_runtime(
            self.initialize_entry,
            &self.initialize_program,
            &[RuntimeValue::I64(rows), RuntimeValue::I64(columns)],
        )?;
        self.validate_runtime_state(&state)?;
        let (frame, render_observation) = self.render_state(&state)?;
        let observation = combine_observations(initialize_observation, render_observation);
        let session = InteractiveSession {
            prepared: self,
            state,
            pending_action: None,
            last_action_id: 0,
        };
        Ok((
            session,
            InteractiveStep {
                frame,
                changed: false,
                exit: false,
                action: None,
                observation,
            },
        ))
    }

    pub fn digest(&self) -> ApplicationDigest {
        self.application.digest
    }

    fn run_runtime(
        &self,
        entry: NodeId,
        program: &interpret::PreparedProgram,
        arguments: &[RuntimeValue],
    ) -> Result<(RuntimeValue, InteractiveExecutionObservation)> {
        let public_started = Instant::now();
        let argument_nanoseconds = elapsed_nanoseconds(public_started);
        let result = interpret::run_prepared(
            &self.application.flattened.snapshot,
            entry,
            program,
            arguments,
            self.application.policy,
        )?;
        let result_started = Instant::now();
        Ok((
            result.value,
            InteractiveExecutionObservation {
                execute_nanoseconds: result.execute_nanoseconds,
                public_value_nanoseconds: argument_nanoseconds
                    .saturating_add(elapsed_nanoseconds(result_started)),
            },
        ))
    }

    fn validate_runtime_state(&self, state: &RuntimeValue) -> Result<()> {
        let expected = self
            .application
            .flattened
            .item(self.profile.state.release, self.profile.state.item)?;
        interpret::validate_runtime_value(
            &self.application.flattened.snapshot,
            state,
            SemanticType::Nominal(expected),
            expected,
        )?;
        Ok(())
    }

    fn render_state(
        &self,
        state: &RuntimeValue,
    ) -> Result<(InteractiveFrame, InteractiveExecutionObservation)> {
        let (value, observation) = self.run_runtime(
            self.render_entry,
            &self.render_program,
            std::slice::from_ref(state),
        )?;
        let value = from_runtime(&self.application.flattened, &value)?;
        Ok((decode_frame(&self.profile, value)?, observation))
    }

    fn decode_runtime_update(
        &self,
        value: RuntimeValue,
    ) -> Result<(RuntimeValue, bool, bool, Option<InteractiveActionRequest>)> {
        let RuntimeValue::Product { ty, fields } = value else {
            return Err(malformed("interactive update result is not a product"));
        };
        let expected = self.application.flattened.item(
            self.profile.update_result.release,
            self.profile.update_result.item,
        )?;
        let expected_count = if self.profile.actions.is_some() { 5 } else { 3 };
        if ty != expected || fields.len() != expected_count {
            return Err(malformed(
                "interactive update result has the wrong exact type or field count",
            ));
        }
        let state_field = self.application.flattened.item(
            self.profile.update_state_field.release,
            self.profile.update_state_field.item,
        )?;
        let changed_field = self.application.flattened.item(
            self.profile.update_changed_field.release,
            self.profile.update_changed_field.item,
        )?;
        let exit_field = self.application.flattened.item(
            self.profile.update_exit_field.release,
            self.profile.update_exit_field.item,
        )?;
        let action_field = self
            .profile
            .actions
            .as_ref()
            .map(|actions| {
                self.application.flattened.item(
                    actions.update_action_field.release,
                    actions.update_action_field.item,
                )
            })
            .transpose()?;
        let action_id_field = self
            .profile
            .actions
            .as_ref()
            .map(|actions| {
                self.application.flattened.item(
                    actions.update_action_id_field.release,
                    actions.update_action_id_field.item,
                )
            })
            .transpose()?;
        let mut state = None;
        let mut changed = None;
        let mut exit = None;
        let mut action = None;
        let mut action_id = None;
        for field in fields {
            if field.field == state_field {
                state = Some(field.value);
            } else if field.field == changed_field {
                let RuntimeValue::Bool(value) = field.value else {
                    return Err(malformed("interactive changed field is not bool"));
                };
                changed = Some(value);
            } else if field.field == exit_field {
                let RuntimeValue::Bool(value) = field.value else {
                    return Err(malformed("interactive exit field is not bool"));
                };
                exit = Some(value);
            } else if action_field == Some(field.field) {
                let actions = self.profile.actions.as_ref().ok_or_else(|| {
                    malformed("interactive action field has no declared action profile")
                })?;
                let public = from_runtime(&self.application.flattened, &field.value)?;
                action = Some(decode_action(&self.profile, actions, public)?);
            } else if action_id_field == Some(field.field) {
                let RuntimeValue::I64(value) = field.value else {
                    return Err(malformed("interactive action id field is not i64"));
                };
                action_id = Some(value);
            } else {
                return Err(malformed(
                    "interactive update result contains an unknown field",
                ));
            }
        }
        let action = match (action.unwrap_or(None), action_id) {
            (None, None) if self.profile.actions.is_none() => None,
            (None, Some(0)) => None,
            (None, Some(_)) => {
                return Err(malformed(
                    "interactive no-action update carries a nonzero job identity",
                ));
            }
            (Some(action), Some(job_id)) => {
                let job_id = u64::try_from(job_id)
                    .map_err(|_| malformed("interactive action job identity must be positive"))?;
                if job_id == 0 || job_id > MAXIMUM_INTERACTIVE_JOB_ID {
                    return Err(malformed(
                        "interactive action job identity exceeds its exact positive domain",
                    ));
                }
                Some(InteractiveActionRequest { job_id, action })
            }
            (Some(_), None) => {
                return Err(malformed(
                    "interactive action update omits its job identity",
                ));
            }
            (None, None) => {
                return Err(malformed(
                    "interactive action update omits its zero job identity",
                ));
            }
        };
        Ok((
            state.ok_or_else(|| malformed("interactive update result omits state"))?,
            changed.ok_or_else(|| malformed("interactive update result omits changed"))?,
            exit.ok_or_else(|| malformed("interactive update result omits exit"))?,
            action,
        ))
    }
}

impl InteractiveSession {
    pub fn step(&mut self, event: InteractiveEvent) -> Result<InteractiveStep> {
        let event = encode_event(&self.prepared.profile, event)?;
        let event = to_runtime(&self.prepared.application.flattened, &event)?;
        let (value, update_observation) = self.prepared.run_runtime(
            self.prepared.update_entry,
            &self.prepared.update_program,
            &[self.state.clone(), event],
        )?;
        let (state, changed, exit, action) = self.prepared.decode_runtime_update(value)?;
        self.prepared.validate_runtime_state(&state)?;
        let (frame, render_observation) = self.prepared.render_state(&state)?;
        let (pending_action, last_action_id) =
            next_action_state(self.pending_action, self.last_action_id, action.as_ref())?;
        self.state = state;
        self.pending_action = pending_action;
        self.last_action_id = last_action_id;
        Ok(InteractiveStep {
            frame,
            changed,
            exit,
            action,
            observation: combine_observations(update_observation, render_observation),
        })
    }

    pub fn resume(&mut self, outcome: InteractiveActionOutcome) -> Result<InteractiveStep> {
        let pending = self.pending_action.ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "interactive session has no pending host action to resume",
            )
        })?;
        if outcome.job_id != pending {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "interactive outcome has a foreign or stale job identity",
            ));
        }
        let actions = self.prepared.profile.actions.as_ref().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "interactive session lost its action profile",
            )
        })?;
        let resume_entry = self.prepared.resume_entry.ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "interactive session lost its resume entry",
            )
        })?;
        let resume_program = self.prepared.resume_program.as_ref().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "interactive session lost its resume program",
            )
        })?;
        let outcome = encode_action_outcome(&self.prepared.profile, actions, outcome)?;
        let outcome = to_runtime(&self.prepared.application.flattened, &outcome)?;
        let (value, update_observation) = self.prepared.run_runtime(
            resume_entry,
            resume_program,
            &[self.state.clone(), outcome],
        )?;
        let (state, changed, exit, action) = self.prepared.decode_runtime_update(value)?;
        self.prepared.validate_runtime_state(&state)?;
        let (frame, render_observation) = self.prepared.render_state(&state)?;
        let (pending_action, last_action_id) =
            next_action_state(None, self.last_action_id, action.as_ref())?;
        self.state = state;
        self.pending_action = pending_action;
        self.last_action_id = last_action_id;
        Ok(InteractiveStep {
            frame,
            changed,
            exit,
            action,
            observation: combine_observations(update_observation, render_observation),
        })
    }

    pub fn frame(&self) -> Result<InteractiveFrame> {
        self.prepared
            .render_state(&self.state)
            .map(|(frame, _)| frame)
    }

    pub fn application_digest(&self) -> ApplicationDigest {
        self.prepared.digest()
    }

    pub const fn pending_action_id(&self) -> Option<u64> {
        self.pending_action
    }
}

fn next_action_state(
    pending: Option<u64>,
    last_action_id: u64,
    action: Option<&InteractiveActionRequest>,
) -> Result<(Option<u64>, u64)> {
    let Some(action) = action else {
        return Ok((pending, last_action_id));
    };
    if pending.is_some() {
        return Err(LkError::new(
            ErrorCode::AuthorityBusy,
            "interactive application requested a second action while one job is pending",
        ));
    }
    if action.job_id <= last_action_id || action.job_id > MAXIMUM_INTERACTIVE_JOB_ID {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "interactive action job identities must increase monotonically without reuse",
        ));
    }
    Ok((Some(action.job_id), action.job_id))
}

pub fn prepare_interactive(bytes: &[u8]) -> Result<PreparedInteractiveApplication> {
    prepare_interactive_observed(bytes, &mut ApplicationLoadObservation::default())
}

pub(crate) fn prepare_interactive_observed(
    bytes: &[u8],
    observation: &mut ApplicationLoadObservation,
) -> Result<PreparedInteractiveApplication> {
    let application = decode_application_observed(bytes, observation)?;
    let InvocationProfile::Interactive(profile) = &application.profile else {
        return Err(LkError::new(
            ErrorCode::RunArgumentMismatch,
            "application does not declare the interactive invocation profile",
        ));
    };
    let profile = profile.as_ref().clone();
    if profile.version != INTERACTIVE_PROFILE_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "interactive execution rejects the predecessor profile",
        ));
    }
    let initialize_entry = application
        .flattened
        .item(profile.initialize.release, profile.initialize.item)?;
    let update_entry = application
        .flattened
        .item(profile.update.release, profile.update.item)?;
    let render_entry = application
        .flattened
        .item(profile.render.release, profile.render.item)?;
    let resume_entry = profile
        .actions
        .as_ref()
        .map(|actions| {
            application
                .flattened
                .item(actions.resume.release, actions.resume.item)
        })
        .transpose()?;
    let initialize_program =
        interpret::prepare_entry(&application.flattened.snapshot, initialize_entry)?.0;
    let update_program = interpret::prepare_entry(&application.flattened.snapshot, update_entry)?.0;
    let render_program = interpret::prepare_entry(&application.flattened.snapshot, render_entry)?.0;
    let resume_program = resume_entry
        .map(|entry| {
            interpret::prepare_entry(&application.flattened.snapshot, entry).map(|value| value.0)
        })
        .transpose()?;
    Ok(PreparedInteractiveApplication {
        application,
        profile,
        initialize_entry,
        update_entry,
        render_entry,
        resume_entry,
        initialize_program,
        update_program,
        render_program,
        resume_program,
    })
}

pub(super) fn validate_profile(
    graph: &ReleaseGraph,
    flattened: &FlattenedGraph,
    _entry: NodeId,
    profile: &InteractiveApplicationProfile,
) -> Result<()> {
    if profile.version != INTERACTIVE_PROFILE_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "interactive application rejects predecessor profile versions",
        ));
    }
    for function in [profile.initialize, profile.update, profile.render] {
        validate_exported_function(graph, function)?;
    }

    let state = item(flattened, profile.state)?;
    if !matches!(
        flattened.snapshot.node(state)?,
        Node::ProductType { .. } | Node::SumType { .. }
    ) {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "interactive state must be a nominal product or sum",
        ));
    }
    let event = item(flattened, profile.events.event)?;
    let update_result = item(flattened, profile.update_result)?;
    let frame = item(flattened, profile.frame)?;
    let scalars = item(flattened, profile.events.scalars)?;
    let size = item(flattened, profile.events.size)?;
    let key_code = item(flattened, profile.events.key.code)?;
    let key_event = item(flattened, profile.events.key.event)?;

    expect_sequence(
        &flattened.snapshot,
        scalars,
        SemanticType::I64,
        "interactive scalars",
    )?;
    let mut update_fields = vec![
        (profile.update_state_field, SemanticType::Nominal(state)),
        (profile.update_changed_field, SemanticType::Bool),
        (profile.update_exit_field, SemanticType::Bool),
    ];
    if let Some(actions) = &profile.actions {
        let action = item(flattened, actions.action)?;
        update_fields.push((actions.update_action_field, SemanticType::Nominal(action)));
        update_fields.push((actions.update_action_id_field, SemanticType::I64));
    }
    expect_product(
        flattened,
        update_result,
        &update_fields,
        "interactive update result",
    )?;
    expect_product(
        flattened,
        frame,
        &[
            (profile.frame_rows_field, SemanticType::I64),
            (profile.frame_columns_field, SemanticType::I64),
            (profile.frame_scalars_field, SemanticType::Nominal(scalars)),
            (profile.frame_styles_field, SemanticType::Nominal(scalars)),
            (profile.frame_cursor_row_field, SemanticType::I64),
            (profile.frame_cursor_column_field, SemanticType::I64),
            (profile.frame_cursor_visible_field, SemanticType::Bool),
            (profile.frame_cursor_shape_field, SemanticType::I64),
            (profile.frame_status_field, SemanticType::Text),
            (profile.frame_status_style_field, SemanticType::I64),
        ],
        "interactive frame",
    )?;
    expect_product(
        flattened,
        size,
        &[
            (profile.events.size_rows_field, SemanticType::I64),
            (profile.events.size_columns_field, SemanticType::I64),
        ],
        "interactive resize payload",
    )?;

    let key = &profile.events.key;
    expect_sum(
        flattened,
        key_code,
        &[
            (key.character_variant, Some(SemanticType::I64)),
            (key.enter_variant, None),
            (key.backspace_variant, None),
            (key.delete_variant, None),
            (key.left_variant, None),
            (key.right_variant, None),
            (key.up_variant, None),
            (key.down_variant, None),
            (key.home_variant, None),
            (key.end_variant, None),
            (key.escape_variant, None),
        ],
        "interactive key code",
    )?;
    expect_product(
        flattened,
        key_event,
        &[
            (key.event_code_field, SemanticType::Nominal(key_code)),
            (key.event_control_field, SemanticType::Bool),
            (key.event_alt_field, SemanticType::Bool),
            (key.event_shift_field, SemanticType::Bool),
            (key.event_repeat_field, SemanticType::Bool),
        ],
        "interactive key event",
    )?;
    let mouse = &profile.events.mouse;
    let mouse_button = item(flattened, mouse.button)?;
    let mouse_kind = item(flattened, mouse.kind)?;
    let mouse_event = item(flattened, mouse.event)?;
    expect_sum(
        flattened,
        mouse_button,
        &[
            (mouse.button_none_variant, None),
            (mouse.button_primary_variant, None),
            (mouse.button_middle_variant, None),
            (mouse.button_secondary_variant, None),
        ],
        "interactive mouse button",
    )?;
    expect_sum(
        flattened,
        mouse_kind,
        &[
            (mouse.press_variant, None),
            (mouse.release_variant, None),
            (mouse.drag_variant, None),
            (mouse.scroll_up_variant, None),
            (mouse.scroll_down_variant, None),
            (mouse.scroll_left_variant, None),
            (mouse.scroll_right_variant, None),
        ],
        "interactive mouse event kind",
    )?;
    expect_product(
        flattened,
        mouse_event,
        &[
            (
                mouse.event_button_field,
                SemanticType::Nominal(mouse_button),
            ),
            (mouse.event_kind_field, SemanticType::Nominal(mouse_kind)),
            (mouse.event_row_field, SemanticType::I64),
            (mouse.event_column_field, SemanticType::I64),
            (mouse.event_control_field, SemanticType::Bool),
            (mouse.event_alt_field, SemanticType::Bool),
            (mouse.event_shift_field, SemanticType::Bool),
        ],
        "interactive mouse event",
    )?;
    let open = &profile.events.open;
    let open_event = item(flattened, open.event)?;
    expect_product(
        flattened,
        open_event,
        &[
            (open.event_path_field, SemanticType::Text),
            (open.event_directory_field, SemanticType::Bool),
            (open.event_project_field, SemanticType::Bool),
        ],
        "interactive deployment-open event",
    )?;
    expect_sum(
        flattened,
        event,
        &[
            (
                profile.events.key_variant,
                Some(SemanticType::Nominal(key_event)),
            ),
            (
                profile.events.paste_variant,
                Some(SemanticType::Nominal(scalars)),
            ),
            (
                profile.events.resize_variant,
                Some(SemanticType::Nominal(size)),
            ),
            (
                profile.events.mouse_variant,
                Some(SemanticType::Nominal(mouse_event)),
            ),
            (profile.events.focus_gained_variant, None),
            (profile.events.focus_lost_variant, None),
            (
                profile.events.open_variant,
                Some(SemanticType::Nominal(open_event)),
            ),
            (profile.events.close_variant, None),
        ],
        "interactive terminal event",
    )?;

    expect_function(
        flattened,
        profile.initialize,
        &[SemanticType::I64, SemanticType::I64],
        SemanticType::Nominal(state),
        "interactive initializer",
    )?;
    expect_function(
        flattened,
        profile.update,
        &[SemanticType::Nominal(state), SemanticType::Nominal(event)],
        SemanticType::Nominal(update_result),
        "interactive update",
    )?;
    expect_function(
        flattened,
        profile.render,
        &[SemanticType::Nominal(state)],
        SemanticType::Nominal(frame),
        "interactive render",
    )?;
    if let Some(actions) = &profile.actions {
        validate_action_routes(
            graph,
            flattened,
            state,
            update_result,
            scalars,
            profile.version,
            actions,
        )?;
    }
    Ok(())
}

fn validate_action_routes(
    graph: &ReleaseGraph,
    flattened: &FlattenedGraph,
    state: NodeId,
    update_result: NodeId,
    scalars: NodeId,
    version: u16,
    actions: &InteractiveActionRoutes,
) -> Result<()> {
    if version != INTERACTIVE_PROFILE_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "interactive action routes reject predecessor profile versions",
        ));
    }
    let expected = InteractiveActionKind::ALL.as_slice();
    if actions.routes.len() != expected.len()
        || actions
            .routes
            .iter()
            .map(|route| route.kind)
            .ne(expected.iter().copied())
    {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "interactive action routes must cover every action kind in canonical order",
        ));
    }
    let action = item(flattened, actions.action)?;
    let Node::SumType { variants, .. } = flattened.snapshot.node(action)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "interactive action must be a nominal sum",
        )
        .for_node(action));
    };
    let mapped = actions
        .routes
        .iter()
        .map(|route| item(flattened, route.variant))
        .collect::<Result<Vec<_>>>()?;
    if variants != &mapped {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "interactive action variants do not match their route order",
        )
        .for_node(action));
    }
    let file_save_payload = item(flattened, actions.file_save_payload)?;
    expect_product(
        flattened,
        file_save_payload,
        &[
            (actions.file_save_origin_field, SemanticType::Text),
            (
                actions.file_save_content_field,
                SemanticType::Nominal(scalars),
            ),
            (actions.file_save_create_field, SemanticType::Bool),
        ],
        "interactive file-save action payload",
    )?;
    let file_search_payload = item(flattened, actions.file_search_payload)?;
    expect_product(
        flattened,
        file_search_payload,
        &[
            (actions.file_search_start_field, SemanticType::Text),
            (actions.file_search_query_field, SemanticType::Text),
        ],
        "interactive root-search action payload",
    )?;
    for (route, variant) in actions.routes.iter().zip(mapped) {
        let Node::SumVariant { owner, payload, .. } = flattened.snapshot.node(variant)? else {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "interactive action route does not name a sum variant",
            )
            .for_node(variant));
        };
        let expected = if route.kind == InteractiveActionKind::FilesystemSave {
            Some(SemanticType::Nominal(file_save_payload))
        } else if route.kind == InteractiveActionKind::FilesystemSearch {
            Some(SemanticType::Nominal(file_search_payload))
        } else if route.kind.requires_scalars() {
            Some(SemanticType::Nominal(scalars))
        } else if route.kind == InteractiveActionKind::FilesystemReconcile {
            Some(SemanticType::Text)
        } else {
            None
        };
        if *owner != action || *payload != expected {
            return Err(LkError::new(
                ErrorCode::TypeMismatch,
                "interactive action variant has the wrong exact payload contract",
            )
            .for_node(variant));
        }
    }
    let outcome = item(flattened, actions.outcome)?;
    expect_product(
        flattened,
        outcome,
        &[
            (actions.outcome_job_id_field, SemanticType::I64),
            (actions.outcome_class_field, SemanticType::I64),
            (actions.outcome_message_field, SemanticType::Text),
            (
                actions.outcome_content_field,
                SemanticType::Nominal(scalars),
            ),
            (actions.outcome_token_field, SemanticType::Text),
        ],
        "interactive action outcome",
    )?;
    validate_exported_function(graph, actions.resume)?;
    expect_function(
        flattened,
        actions.resume,
        &[SemanticType::Nominal(state), SemanticType::Nominal(outcome)],
        SemanticType::Nominal(update_result),
        "interactive action resume",
    )
}

fn expect_sequence(
    snapshot: &Snapshot,
    id: NodeId,
    element: SemanticType,
    label: &str,
) -> Result<()> {
    match snapshot.node(id)? {
        Node::SequenceType {
            element: actual, ..
        } if *actual == element => Ok(()),
        _ => Err(LkError::new(
            ErrorCode::TypeMismatch,
            format!("{label} has the wrong exact sequence element type"),
        )
        .for_node(id)),
    }
}

fn expect_product(
    flattened: &FlattenedGraph,
    product: NodeId,
    expected: &[(ApplicationTarget, SemanticType)],
    label: &str,
) -> Result<()> {
    let Node::ProductType { fields, .. } = flattened.snapshot.node(product)? else {
        return Err(
            LkError::new(ErrorCode::WrongKind, format!("{label} is not a product"))
                .for_node(product),
        );
    };
    let mapped = expected
        .iter()
        .map(|(target, _)| item(flattened, *target))
        .collect::<Result<Vec<_>>>()?;
    if fields != &mapped {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            format!("{label} fields are not in exact declared order"),
        )
        .for_node(product));
    }
    for ((_, expected_type), field) in expected.iter().zip(mapped) {
        match flattened.snapshot.node(field)? {
            Node::ProductField { owner, ty, .. } if *owner == product && *ty == *expected_type => {}
            _ => {
                return Err(LkError::new(
                    ErrorCode::TypeMismatch,
                    format!("{label} field has the wrong owner or exact type"),
                )
                .for_node(field));
            }
        }
    }
    Ok(())
}

fn expect_sum(
    flattened: &FlattenedGraph,
    sum: NodeId,
    expected: &[(ApplicationTarget, Option<SemanticType>)],
    label: &str,
) -> Result<()> {
    let Node::SumType { variants, .. } = flattened.snapshot.node(sum)? else {
        return Err(
            LkError::new(ErrorCode::WrongKind, format!("{label} is not a sum")).for_node(sum),
        );
    };
    let mapped = expected
        .iter()
        .map(|(target, _)| item(flattened, *target))
        .collect::<Result<Vec<_>>>()?;
    if variants != &mapped {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            format!("{label} variants are not in exact declared order"),
        )
        .for_node(sum));
    }
    for ((_, payload), variant) in expected.iter().zip(mapped) {
        match flattened.snapshot.node(variant)? {
            Node::SumVariant {
                owner,
                payload: actual,
                ..
            } if *owner == sum && *actual == *payload => {}
            _ => {
                return Err(LkError::new(
                    ErrorCode::TypeMismatch,
                    format!("{label} variant has the wrong owner or payload"),
                )
                .for_node(variant));
            }
        }
    }
    Ok(())
}

fn expect_function(
    flattened: &FlattenedGraph,
    target: ApplicationTarget,
    expected_parameters: &[SemanticType],
    expected_result: SemanticType,
    label: &str,
) -> Result<()> {
    let function = item(flattened, target)?;
    let Node::Function {
        parameters, result, ..
    } = flattened.snapshot.node(function)?
    else {
        return Err(
            LkError::new(ErrorCode::WrongKind, format!("{label} is not a function"))
                .for_node(function),
        );
    };
    let actual = parameters
        .iter()
        .map(|parameter| parameter_type(&flattened.snapshot, *parameter))
        .collect::<Result<Vec<_>>>()?;
    if actual != expected_parameters || *result != expected_result {
        return Err(LkError::new(
            ErrorCode::TypeMismatch,
            format!("{label} has the wrong exact signature"),
        )
        .for_node(function));
    }
    Ok(())
}

fn item(flattened: &FlattenedGraph, target: ApplicationTarget) -> Result<NodeId> {
    flattened.item(target.release, target.item)
}

fn validate_dimensions(rows: i64, columns: i64) -> Result<()> {
    if !(1..=MAXIMUM_INTERACTIVE_ROWS).contains(&rows)
        || !(1..=MAXIMUM_INTERACTIVE_COLUMNS).contains(&columns)
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "interactive dimensions exceed the 1..=1000 profile policy",
        ));
    }
    Ok(())
}

fn encode_event(
    profile: &InteractiveApplicationProfile,
    event: InteractiveEvent,
) -> Result<ApplicationValue> {
    let events = &profile.events;
    let value = match event {
        InteractiveEvent::Key(key) => {
            let code = match key.code {
                InteractiveKeyCode::Character(value) => {
                    if char::from_u32(value).is_none() {
                        return Err(LkError::new(
                            ErrorCode::RunArgumentMismatch,
                            "interactive character is not a Unicode scalar value",
                        ));
                    }
                    sum(
                        events.key.code,
                        events.key.character_variant,
                        Some(ApplicationValue::I64(i64::from(value))),
                    )
                }
                InteractiveKeyCode::Enter => sum(events.key.code, events.key.enter_variant, None),
                InteractiveKeyCode::Backspace => {
                    sum(events.key.code, events.key.backspace_variant, None)
                }
                InteractiveKeyCode::Delete => sum(events.key.code, events.key.delete_variant, None),
                InteractiveKeyCode::Left => sum(events.key.code, events.key.left_variant, None),
                InteractiveKeyCode::Right => sum(events.key.code, events.key.right_variant, None),
                InteractiveKeyCode::Up => sum(events.key.code, events.key.up_variant, None),
                InteractiveKeyCode::Down => sum(events.key.code, events.key.down_variant, None),
                InteractiveKeyCode::Home => sum(events.key.code, events.key.home_variant, None),
                InteractiveKeyCode::End => sum(events.key.code, events.key.end_variant, None),
                InteractiveKeyCode::Escape => sum(events.key.code, events.key.escape_variant, None),
            };
            let key_value = ApplicationValue::Product {
                ty: events.key.event,
                fields: vec![
                    field(events.key.event_code_field, code),
                    field(
                        events.key.event_control_field,
                        ApplicationValue::Bool(key.control),
                    ),
                    field(events.key.event_alt_field, ApplicationValue::Bool(key.alt)),
                    field(
                        events.key.event_shift_field,
                        ApplicationValue::Bool(key.shift),
                    ),
                    field(
                        events.key.event_repeat_field,
                        ApplicationValue::Bool(key.repeat),
                    ),
                ],
            };
            sum(events.event, events.key_variant, Some(key_value))
        }
        InteractiveEvent::Paste(scalars) => {
            if scalars.len() > MAXIMUM_INTERACTIVE_PASTE_SCALARS
                || scalars
                    .iter()
                    .any(|scalar| char::from_u32(*scalar).is_none())
            {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "interactive paste exceeds policy or contains an invalid Unicode scalar",
                ));
            }
            let value = ApplicationValue::Sequence {
                ty: events.scalars,
                elements: scalars
                    .into_iter()
                    .map(|value| ApplicationValue::I64(i64::from(value)))
                    .collect(),
            };
            sum(events.event, events.paste_variant, Some(value))
        }
        InteractiveEvent::Resize { rows, columns } => {
            validate_dimensions(rows, columns)?;
            let size = ApplicationValue::Product {
                ty: events.size,
                fields: vec![
                    field(events.size_rows_field, ApplicationValue::I64(rows)),
                    field(events.size_columns_field, ApplicationValue::I64(columns)),
                ],
            };
            sum(events.event, events.resize_variant, Some(size))
        }
        InteractiveEvent::Mouse(mouse) => {
            if !(0..MAXIMUM_INTERACTIVE_ROWS).contains(&mouse.row)
                || !(0..MAXIMUM_INTERACTIVE_COLUMNS).contains(&mouse.column)
            {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "interactive mouse coordinates exceed the zero-based profile domain",
                ));
            }
            let routes = &events.mouse;
            let button = match mouse.button {
                InteractiveMouseButton::None => routes.button_none_variant,
                InteractiveMouseButton::Primary => routes.button_primary_variant,
                InteractiveMouseButton::Middle => routes.button_middle_variant,
                InteractiveMouseButton::Secondary => routes.button_secondary_variant,
            };
            let kind = match mouse.kind {
                InteractiveMouseKind::Press => routes.press_variant,
                InteractiveMouseKind::Release => routes.release_variant,
                InteractiveMouseKind::Drag => routes.drag_variant,
                InteractiveMouseKind::ScrollUp => routes.scroll_up_variant,
                InteractiveMouseKind::ScrollDown => routes.scroll_down_variant,
                InteractiveMouseKind::ScrollLeft => routes.scroll_left_variant,
                InteractiveMouseKind::ScrollRight => routes.scroll_right_variant,
            };
            let value = ApplicationValue::Product {
                ty: routes.event,
                fields: vec![
                    field(routes.event_button_field, sum(routes.button, button, None)),
                    field(routes.event_kind_field, sum(routes.kind, kind, None)),
                    field(routes.event_row_field, ApplicationValue::I64(mouse.row)),
                    field(
                        routes.event_column_field,
                        ApplicationValue::I64(mouse.column),
                    ),
                    field(
                        routes.event_control_field,
                        ApplicationValue::Bool(mouse.control),
                    ),
                    field(routes.event_alt_field, ApplicationValue::Bool(mouse.alt)),
                    field(
                        routes.event_shift_field,
                        ApplicationValue::Bool(mouse.shift),
                    ),
                ],
            };
            sum(events.event, events.mouse_variant, Some(value))
        }
        InteractiveEvent::FocusGained => sum(events.event, events.focus_gained_variant, None),
        InteractiveEvent::FocusLost => sum(events.event, events.focus_lost_variant, None),
        InteractiveEvent::Open(open) => {
            let path = TextString::new(open.path).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "interactive deployment-open path exceeds text byte policy",
                )
            })?;
            let routes = &events.open;
            let value = ApplicationValue::Product {
                ty: routes.event,
                fields: vec![
                    field(routes.event_path_field, ApplicationValue::Text(path)),
                    field(
                        routes.event_directory_field,
                        ApplicationValue::Bool(open.directory),
                    ),
                    field(
                        routes.event_project_field,
                        ApplicationValue::Bool(open.project),
                    ),
                ],
            };
            sum(events.event, events.open_variant, Some(value))
        }
        InteractiveEvent::Close => sum(events.event, events.close_variant, None),
    };
    Ok(value)
}

fn decode_action(
    profile: &InteractiveApplicationProfile,
    actions: &InteractiveActionRoutes,
    value: ApplicationValue,
) -> Result<Option<InteractiveAction>> {
    let ApplicationValue::Sum {
        ty,
        variant,
        payload,
    } = value
    else {
        return Err(malformed("interactive action is not a nominal sum"));
    };
    if ty != actions.action {
        return Err(malformed("interactive action has the wrong exact type"));
    }
    let route = actions
        .routes
        .iter()
        .find(|route| route.variant == variant)
        .ok_or_else(|| malformed("interactive action variant has no declared route"))?;
    if route.kind == InteractiveActionKind::None {
        if payload.is_some() {
            return Err(malformed("interactive no-action variant carries a payload"));
        }
        return Ok(None);
    }
    if route.kind == InteractiveActionKind::FilesystemSave {
        let payload = payload.ok_or_else(|| malformed("interactive file save omits payload"))?;
        let fields = product_fields(
            *payload,
            actions.file_save_payload,
            3,
            "interactive file-save action",
        )?;
        let mut origin = None;
        let mut content = None;
        let mut create = None;
        for field in fields {
            if field.field == actions.file_save_origin_field {
                origin = Some(expect_text(field.value, "interactive file-save origin")?);
            } else if field.field == actions.file_save_content_field {
                content = Some(expect_scalar_text(
                    profile.events.scalars,
                    field.value,
                    "interactive file-save content",
                )?);
            } else if field.field == actions.file_save_create_field {
                create = Some(expect_bool(
                    field.value,
                    "interactive file-save create mode",
                )?);
            } else {
                return Err(malformed(
                    "interactive file-save action contains an unknown field",
                ));
            }
        }
        return Ok(Some(InteractiveAction::FilesystemSave {
            origin: origin.ok_or_else(|| malformed("interactive file-save action omits origin"))?,
            content: content
                .ok_or_else(|| malformed("interactive file-save action omits content"))?,
            create: create
                .ok_or_else(|| malformed("interactive file-save action omits create mode"))?,
        }));
    }
    if route.kind == InteractiveActionKind::FilesystemSearch {
        let payload = payload.ok_or_else(|| malformed("interactive root search omits payload"))?;
        let fields = product_fields(
            *payload,
            actions.file_search_payload,
            2,
            "interactive root-search action",
        )?;
        let mut start = None;
        let mut query = None;
        for field in fields {
            if field.field == actions.file_search_start_field {
                start = Some(expect_text(field.value, "interactive root-search start")?);
            } else if field.field == actions.file_search_query_field {
                query = Some(expect_text(field.value, "interactive root-search query")?);
            } else {
                return Err(malformed(
                    "interactive root-search action contains an unknown field",
                ));
            }
        }
        return Ok(Some(InteractiveAction::FilesystemSearch {
            start: start.ok_or_else(|| malformed("interactive root search omits start"))?,
            query: query.ok_or_else(|| malformed("interactive root search omits query"))?,
        }));
    }
    if route.kind.requires_scalars() || route.kind == InteractiveActionKind::FilesystemReconcile {
        let text = payload.ok_or_else(|| malformed("interactive action omits its text payload"))?;
        let text = if route.kind == InteractiveActionKind::FilesystemReconcile {
            expect_text(*text, "interactive reconciliation token")?
        } else {
            expect_scalar_text(profile.events.scalars, *text, "interactive action payload")?
        };
        return Ok(Some(match route.kind {
            InteractiveActionKind::ProjectSummary => InteractiveAction::ProjectSummary(text),
            InteractiveActionKind::ProjectChildren => InteractiveAction::ProjectChildren(text),
            InteractiveActionKind::ProjectFunction => InteractiveAction::ProjectFunction(text),
            InteractiveActionKind::ProjectCallers => InteractiveAction::ProjectCallers(text),
            InteractiveActionKind::ProjectCallees => InteractiveAction::ProjectCallees(text),
            InteractiveActionKind::ProjectTargets => InteractiveAction::ProjectTargets(text),
            InteractiveActionKind::ProjectProposal => InteractiveAction::ProjectProposal(text),
            InteractiveActionKind::ProjectRecord => InteractiveAction::ProjectRecord(text),
            InteractiveActionKind::ProjectDiff => InteractiveAction::ProjectDiff(text),
            InteractiveActionKind::ProjectValidate => InteractiveAction::ProjectValidate(text),
            InteractiveActionKind::ProjectApply => InteractiveAction::ProjectApply(text),
            InteractiveActionKind::ProjectTargetTest => InteractiveAction::ProjectTargetTest(text),
            InteractiveActionKind::ProjectTargetBuild => {
                InteractiveAction::ProjectTargetBuild(text)
            }
            InteractiveActionKind::ProjectTargetRun => InteractiveAction::ProjectTargetRun(text),
            InteractiveActionKind::FilesystemList => InteractiveAction::FilesystemList(text),
            InteractiveActionKind::FilesystemSearch => {
                return Err(malformed(
                    "interactive root search uses a structured payload",
                ));
            }
            InteractiveActionKind::FilesystemRead => InteractiveAction::FilesystemRead(text),
            InteractiveActionKind::FilesystemReconcile => {
                InteractiveAction::FilesystemReconcile(text)
            }
            _ => {
                return Err(malformed(
                    "interactive action kind has an invalid text-payload contract",
                ));
            }
        }));
    }
    if payload.is_some() {
        return Err(malformed(
            "interactive action unexpectedly carries a payload",
        ));
    }
    Ok(Some(match route.kind {
        InteractiveActionKind::ProjectOrient => InteractiveAction::ProjectOrient,
        InteractiveActionKind::ProjectBlockers => InteractiveAction::ProjectBlockers,
        InteractiveActionKind::ProjectHistory => InteractiveAction::ProjectHistory,
        InteractiveActionKind::ProjectTargetList => InteractiveAction::ProjectTargetList,
        _ => {
            return Err(malformed(
                "interactive action kind has an invalid empty-payload contract",
            ));
        }
    }))
}

fn encode_action_outcome(
    profile: &InteractiveApplicationProfile,
    actions: &InteractiveActionRoutes,
    outcome: InteractiveActionOutcome,
) -> Result<ApplicationValue> {
    let text = |value: String, label: &str| {
        TextString::new(value)
            .map(ApplicationValue::Text)
            .map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    format!("interactive action {label} exceeds the text byte policy"),
                )
            })
    };
    let content = outcome
        .content
        .chars()
        .map(|value| ApplicationValue::I64(i64::from(u32::from(value))))
        .collect();
    Ok(ApplicationValue::Product {
        ty: actions.outcome,
        fields: vec![
            field(
                actions.outcome_job_id_field,
                ApplicationValue::I64(i64::try_from(outcome.job_id).map_err(|_| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "interactive action job identity exceeds signed runtime integer policy",
                    )
                })?),
            ),
            field(
                actions.outcome_class_field,
                ApplicationValue::I64(outcome.class.stable_value()),
            ),
            field(
                actions.outcome_message_field,
                text(outcome.message, "message")?,
            ),
            field(
                actions.outcome_content_field,
                ApplicationValue::Sequence {
                    ty: profile.events.scalars,
                    elements: content,
                },
            ),
            field(actions.outcome_token_field, text(outcome.token, "token")?),
        ],
    })
}

fn decode_frame(
    profile: &InteractiveApplicationProfile,
    value: ApplicationValue,
) -> Result<InteractiveFrame> {
    let fields = product_fields(value, profile.frame, 10, "interactive frame")?;
    let mut rows = None;
    let mut columns = None;
    let mut scalars = None;
    let mut styles = None;
    let mut cursor_row = None;
    let mut cursor_column = None;
    let mut cursor_visible = None;
    let mut cursor_shape = None;
    let mut status = None;
    let mut status_style = None;
    for field in fields {
        if field.field == profile.frame_rows_field {
            rows = Some(expect_i64(field.value, "interactive frame rows")?);
        } else if field.field == profile.frame_columns_field {
            columns = Some(expect_i64(field.value, "interactive frame columns")?);
        } else if field.field == profile.frame_scalars_field {
            scalars = Some(expect_scalars(profile.events.scalars, field.value)?);
        } else if field.field == profile.frame_styles_field {
            styles = Some(expect_styles(profile.events.scalars, field.value)?);
        } else if field.field == profile.frame_cursor_row_field {
            cursor_row = Some(expect_i64(field.value, "interactive cursor row")?);
        } else if field.field == profile.frame_cursor_column_field {
            cursor_column = Some(expect_i64(field.value, "interactive cursor column")?);
        } else if field.field == profile.frame_cursor_visible_field {
            cursor_visible = Some(expect_bool(field.value, "interactive cursor visibility")?);
        } else if field.field == profile.frame_cursor_shape_field {
            cursor_shape = Some(match expect_i64(field.value, "interactive cursor shape")? {
                0 => InteractiveCursorShape::Block,
                1 => InteractiveCursorShape::Bar,
                2 => InteractiveCursorShape::Underline,
                _ => {
                    return Err(malformed(
                        "interactive cursor shape is outside its closed set",
                    ));
                }
            });
        } else if field.field == profile.frame_status_field {
            status = Some(expect_text(field.value, "interactive frame status")?);
        } else if field.field == profile.frame_status_style_field {
            status_style = Some(expect_style(field.value, "interactive status style")?);
        } else {
            return Err(malformed("interactive frame contains an unknown field"));
        }
    }
    let rows = rows.ok_or_else(|| malformed("interactive frame omits rows"))?;
    let columns = columns.ok_or_else(|| malformed("interactive frame omits columns"))?;
    validate_dimensions(rows, columns)?;
    let cursor_row = cursor_row.ok_or_else(|| malformed("interactive frame omits cursor row"))?;
    let cursor_column =
        cursor_column.ok_or_else(|| malformed("interactive frame omits cursor column"))?;
    if cursor_row < 0 || cursor_column < 0 || cursor_row >= rows || cursor_column >= columns {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "interactive cursor lies outside the declared frame",
        ));
    }
    let status = status.ok_or_else(|| malformed("interactive frame omits status"))?;
    if status.len() > MAXIMUM_INTERACTIVE_STATUS_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "interactive status exceeds byte policy",
        ));
    }
    let scalars = scalars.ok_or_else(|| malformed("interactive frame omits scalars"))?;
    let styles = styles.ok_or_else(|| malformed("interactive frame omits styles"))?;
    if styles.len() != scalars.len() {
        return Err(malformed(
            "interactive frame styles must correspond one-for-one with scalars",
        ));
    }
    Ok(InteractiveFrame {
        rows,
        columns,
        scalars,
        styles,
        cursor_row,
        cursor_column,
        cursor_visible: cursor_visible
            .ok_or_else(|| malformed("interactive frame omits cursor visibility"))?,
        cursor_shape: cursor_shape
            .ok_or_else(|| malformed("interactive frame omits cursor shape"))?,
        status,
        status_style: status_style
            .ok_or_else(|| malformed("interactive frame omits status style"))?,
    })
}

fn product_fields(
    value: ApplicationValue,
    expected: ApplicationTarget,
    count: usize,
    label: &str,
) -> Result<Vec<ApplicationFieldValue>> {
    let ApplicationValue::Product { ty, fields } = value else {
        return Err(malformed(format!("{label} is not a product")));
    };
    if ty != expected || fields.len() != count {
        return Err(malformed(format!(
            "{label} has the wrong exact type or field count"
        )));
    }
    Ok(fields)
}

fn expect_scalars(expected: ApplicationTarget, value: ApplicationValue) -> Result<Vec<u32>> {
    let ApplicationValue::Sequence { ty, elements } = value else {
        return Err(malformed("interactive frame scalars are not a sequence"));
    };
    if ty != expected || elements.len() > MAXIMUM_INTERACTIVE_FRAME_SCALARS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "interactive frame scalars have the wrong type or exceed policy",
        ));
    }
    elements
        .into_iter()
        .map(|value| {
            let scalar = expect_i64(value, "interactive frame scalar")?;
            let scalar = u32::try_from(scalar)
                .map_err(|_| malformed("interactive frame scalar is negative or too large"))?;
            char::from_u32(scalar)
                .ok_or_else(|| malformed("interactive frame contains an invalid Unicode scalar"))?;
            Ok(scalar)
        })
        .collect()
}

fn expect_styles(expected: ApplicationTarget, value: ApplicationValue) -> Result<Vec<u8>> {
    let ApplicationValue::Sequence { ty, elements } = value else {
        return Err(malformed("interactive frame styles are not a sequence"));
    };
    if ty != expected || elements.len() > MAXIMUM_INTERACTIVE_FRAME_SCALARS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "interactive frame styles have the wrong type or exceed policy",
        ));
    }
    elements
        .into_iter()
        .map(|value| expect_style(value, "interactive frame style"))
        .collect()
}

fn expect_style(value: ApplicationValue, label: &str) -> Result<u8> {
    let value = expect_i64(value, label)?;
    if !(0..=MAXIMUM_INTERACTIVE_STYLE).contains(&value) {
        return Err(malformed(format!(
            "{label} is outside the closed style palette"
        )));
    }
    u8::try_from(value).map_err(|_| malformed(format!("{label} does not fit its style domain")))
}

fn expect_scalar_text(
    expected: ApplicationTarget,
    value: ApplicationValue,
    label: &str,
) -> Result<String> {
    expect_scalars(expected, value)?
        .into_iter()
        .map(|scalar| {
            char::from_u32(scalar)
                .ok_or_else(|| malformed(format!("{label} contains an invalid Unicode scalar")))
        })
        .collect()
}

fn expect_bool(value: ApplicationValue, label: &str) -> Result<bool> {
    match value {
        ApplicationValue::Bool(value) => Ok(value),
        _ => Err(malformed(format!("{label} is not bool"))),
    }
}

fn expect_i64(value: ApplicationValue, label: &str) -> Result<i64> {
    match value {
        ApplicationValue::I64(value) => Ok(value),
        _ => Err(malformed(format!("{label} is not i64"))),
    }
}

fn expect_text(value: ApplicationValue, label: &str) -> Result<String> {
    match value {
        ApplicationValue::Text(value) => Ok(value.as_str().to_owned()),
        _ => Err(malformed(format!("{label} is not text"))),
    }
}

fn sum(
    ty: ApplicationTarget,
    variant: ApplicationTarget,
    payload: Option<ApplicationValue>,
) -> ApplicationValue {
    ApplicationValue::Sum {
        ty,
        variant,
        payload: payload.map(Box::new),
    }
}

fn field(field: ApplicationTarget, value: ApplicationValue) -> ApplicationFieldValue {
    ApplicationFieldValue { field, value }
}

fn combine_observations(
    lhs: InteractiveExecutionObservation,
    rhs: InteractiveExecutionObservation,
) -> InteractiveExecutionObservation {
    InteractiveExecutionObservation {
        execute_nanoseconds: lhs
            .execute_nanoseconds
            .saturating_add(rhs.execute_nanoseconds),
        public_value_nanoseconds: lhs
            .public_value_nanoseconds
            .saturating_add(rhs.public_value_nanoseconds),
    }
}

fn malformed(message: impl Into<String>) -> LkError {
    LkError::new(ErrorCode::ArtifactCorrupt, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_and_frame_policies_are_bounded() {
        assert_eq!(MAXIMUM_INTERACTIVE_ROWS, 1_000);
        assert_eq!(MAXIMUM_INTERACTIVE_COLUMNS, 1_000);
        assert_eq!(MAXIMUM_INTERACTIVE_FRAME_SCALARS, 131_072);
        assert_eq!(MAXIMUM_INTERACTIVE_PASTE_SCALARS, 65_536);
        assert!(std::mem::size_of::<InteractiveEvent>() <= 64);
    }
}
