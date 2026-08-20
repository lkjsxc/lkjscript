//! Pure foreground interactive application execution.
//!
//! The application owns state transitions, key policy, and full-frame meaning. Native callers
//! own only event adaptation, frame projection, and live-resource lifecycle. Session state is
//! deliberately ephemeral and disappears with the owning process.

use super::*;
use crate::core_ir::CoreProgram;
use crate::graph::Snapshot;
use crate::ids::NodeId;

pub const INTERACTIVE_PROFILE_VERSION: u16 = 2;
pub const MAXIMUM_INTERACTIVE_ROWS: i64 = 1_000;
pub const MAXIMUM_INTERACTIVE_COLUMNS: i64 = 1_000;
pub const MAXIMUM_INTERACTIVE_FRAME_SCALARS: usize = crate::schema::MAXIMUM_SEQUENCE_ELEMENTS;
pub const MAXIMUM_INTERACTIVE_PASTE_SCALARS: usize = 65_536;
pub const MAXIMUM_INTERACTIVE_STATUS_BYTES: usize = 4_096;

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
    pub close_variant: ApplicationTarget,
    pub size: ApplicationTarget,
    pub size_rows_field: ApplicationTarget,
    pub size_columns_field: ApplicationTarget,
    pub scalars: ApplicationTarget,
    pub key: InteractiveKeyRoutes,
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
    FilesystemRead,
    FilesystemSave,
    FilesystemReconcile,
}

impl InteractiveActionKind {
    pub const ALL: [Self; 23] = [
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
        Self::FilesystemRead,
        Self::FilesystemSave,
        Self::FilesystemReconcile,
    ];

    /// Exact route order needed only to reconstruct immutable pre-cutover project snapshots.
    /// [`prepare_interactive`] rejects this profile before execution.
    const HISTORICAL_PROFILE_V1: [Self; 19] = [
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
        Self::ProjectValidate,
        Self::ProjectApply,
        Self::ProjectTargetList,
        Self::ProjectTargetTest,
        Self::FilesystemList,
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
            Self::FilesystemRead => 21,
            Self::FilesystemSave => 22,
            Self::FilesystemReconcile => 23,
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
            21 => Some(Self::FilesystemRead),
            22 => Some(Self::FilesystemSave),
            23 => Some(Self::FilesystemReconcile),
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
    pub routes: Vec<InteractiveActionRoute>,
    pub file_save_payload: ApplicationTarget,
    pub file_save_origin_field: ApplicationTarget,
    pub file_save_content_field: ApplicationTarget,
    pub file_save_create_field: ApplicationTarget,
    pub outcome: ApplicationTarget,
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
    pub frame_cursor_row_field: ApplicationTarget,
    pub frame_cursor_column_field: ApplicationTarget,
    pub frame_cursor_visible_field: ApplicationTarget,
    pub frame_status_field: ApplicationTarget,
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
    FilesystemRead(String),
    FilesystemSave {
        origin: String,
        content: String,
        create: bool,
    },
    FilesystemReconcile(String),
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
    pub class: InteractiveActionOutcomeClass,
    pub message: String,
    pub content: String,
    pub token: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveFrame {
    pub rows: i64,
    pub columns: i64,
    pub scalars: Vec<u32>,
    pub cursor_row: i64,
    pub cursor_column: i64,
    pub cursor_visible: bool,
    pub status: String,
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
    pub action: Option<InteractiveAction>,
    pub observation: InteractiveExecutionObservation,
}

pub struct PreparedInteractiveApplication {
    application: DecodedApplication,
    profile: InteractiveApplicationProfile,
    initialize_entry: NodeId,
    update_entry: NodeId,
    render_entry: NodeId,
    resume_entry: Option<NodeId>,
    initialize_program: CoreProgram,
    update_program: CoreProgram,
    render_program: CoreProgram,
    resume_program: Option<CoreProgram>,
}

pub struct InteractiveSession {
    prepared: PreparedInteractiveApplication,
    state: ApplicationValue,
    pending_action: bool,
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
    ] {
        put_target(writer, target);
    }
    writer.bool(profile.actions.is_some());
    if let Some(actions) = &profile.actions {
        for target in [
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
        frame_cursor_row_field: next()?,
        frame_cursor_column_field: next()?,
        frame_cursor_visible_field: next()?,
        frame_status_field: next()?,
        events: InteractiveEventRoutes {
            event: next()?,
            key_variant: next()?,
            paste_variant: next()?,
            resize_variant: next()?,
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
        },
        actions: None,
    };
    if reader.bool().map_err(application_codec)? {
        let action = read_target(reader)?;
        let update_action_field = read_target(reader)?;
        let file_save_payload = read_target(reader)?;
        let file_save_origin_field = read_target(reader)?;
        let file_save_content_field = read_target(reader)?;
        let file_save_create_field = read_target(reader)?;
        let outcome = read_target(reader)?;
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
            routes,
            file_save_payload,
            file_save_origin_field,
            file_save_content_field,
            file_save_create_field,
            outcome,
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
        let (state, initialize_observation) = self.run(
            self.initialize_entry,
            &self.initialize_program,
            &[ApplicationValue::I64(rows), ApplicationValue::I64(columns)],
        )?;
        self.validate_state(&state)?;
        let (frame, render_observation) = self.render_state(&state)?;
        let observation = combine_observations(initialize_observation, render_observation);
        let session = InteractiveSession {
            prepared: self,
            state,
            pending_action: false,
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

    fn run(
        &self,
        entry: NodeId,
        program: &CoreProgram,
        arguments: &[ApplicationValue],
    ) -> Result<(ApplicationValue, InteractiveExecutionObservation)> {
        let public_started = Instant::now();
        let runtime_arguments = arguments
            .iter()
            .map(|value| to_runtime(&self.application.flattened, value))
            .collect::<Result<Vec<_>>>()?;
        let argument_nanoseconds = elapsed_nanoseconds(public_started);
        let result = interpret::run_compiled(
            &self.application.flattened.snapshot,
            entry,
            program,
            &runtime_arguments,
            self.application.policy,
        )?;
        let result_started = Instant::now();
        let value = from_runtime(&self.application.flattened, &result.value)?;
        Ok((
            value,
            InteractiveExecutionObservation {
                execute_nanoseconds: result.execute_nanoseconds,
                public_value_nanoseconds: argument_nanoseconds
                    .saturating_add(elapsed_nanoseconds(result_started)),
            },
        ))
    }

    fn validate_state(&self, state: &ApplicationValue) -> Result<()> {
        validate_public_value_target(state, self.profile.state, "interactive state")
    }

    fn render_state(
        &self,
        state: &ApplicationValue,
    ) -> Result<(InteractiveFrame, InteractiveExecutionObservation)> {
        let (value, observation) = self.run(
            self.render_entry,
            &self.render_program,
            std::slice::from_ref(state),
        )?;
        Ok((decode_frame(&self.profile, value)?, observation))
    }
}

impl InteractiveSession {
    pub fn step(&mut self, event: InteractiveEvent) -> Result<InteractiveStep> {
        if self.pending_action {
            return Err(LkError::new(
                ErrorCode::AuthorityBusy,
                "interactive session has one unresolved host action",
            ));
        }
        let event = encode_event(&self.prepared.profile, event)?;
        let (value, update_observation) = self.prepared.run(
            self.prepared.update_entry,
            &self.prepared.update_program,
            &[self.state.clone(), event],
        )?;
        let (state, changed, exit, action) = decode_update(&self.prepared.profile, value)?;
        self.prepared.validate_state(&state)?;
        let (frame, render_observation) = self.prepared.render_state(&state)?;
        self.state = state;
        self.pending_action = action.is_some();
        Ok(InteractiveStep {
            frame,
            changed,
            exit,
            action,
            observation: combine_observations(update_observation, render_observation),
        })
    }

    pub fn resume(&mut self, outcome: InteractiveActionOutcome) -> Result<InteractiveStep> {
        if !self.pending_action {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "interactive session has no pending host action to resume",
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
        let (value, update_observation) =
            self.prepared
                .run(resume_entry, resume_program, &[self.state.clone(), outcome])?;
        let (state, changed, exit, action) = decode_update(&self.prepared.profile, value)?;
        self.prepared.validate_state(&state)?;
        let (frame, render_observation) = self.prepared.render_state(&state)?;
        self.state = state;
        self.pending_action = action.is_some();
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
        interpret::compile_entry(&application.flattened.snapshot, initialize_entry)?.0;
    let update_program = interpret::compile_entry(&application.flattened.snapshot, update_entry)?.0;
    let render_program = interpret::compile_entry(&application.flattened.snapshot, render_entry)?.0;
    let resume_program = resume_entry
        .map(|entry| {
            interpret::compile_entry(&application.flattened.snapshot, entry).map(|value| value.0)
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
    if profile.version != 1 && profile.version != INTERACTIVE_PROFILE_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "interactive application profile version is unsupported",
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
            (profile.frame_cursor_row_field, SemanticType::I64),
            (profile.frame_cursor_column_field, SemanticType::I64),
            (profile.frame_cursor_visible_field, SemanticType::Bool),
            (profile.frame_status_field, SemanticType::Text),
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
    let expected = if version == 1 {
        InteractiveActionKind::HISTORICAL_PROFILE_V1.as_slice()
    } else {
        InteractiveActionKind::ALL.as_slice()
    };
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
        InteractiveEvent::Close => sum(events.event, events.close_variant, None),
    };
    Ok(value)
}

fn decode_update(
    profile: &InteractiveApplicationProfile,
    value: ApplicationValue,
) -> Result<(ApplicationValue, bool, bool, Option<InteractiveAction>)> {
    let field_count = if profile.actions.is_some() { 4 } else { 3 };
    let fields = product_fields(
        value,
        profile.update_result,
        field_count,
        "interactive update result",
    )?;
    let mut state = None;
    let mut changed = None;
    let mut exit = None;
    let mut action = None;
    for field in fields {
        if field.field == profile.update_state_field {
            state = Some(field.value);
        } else if field.field == profile.update_changed_field {
            changed = Some(expect_bool(field.value, "interactive changed field")?);
        } else if field.field == profile.update_exit_field {
            exit = Some(expect_bool(field.value, "interactive exit field")?);
        } else if let Some(actions) = profile.actions.as_ref()
            && field.field == actions.update_action_field
        {
            action = Some(decode_action(profile, actions, field.value)?);
        } else {
            return Err(malformed(
                "interactive update result contains an unknown field",
            ));
        }
    }
    Ok((
        state.ok_or_else(|| malformed("interactive update result omits state"))?,
        changed.ok_or_else(|| malformed("interactive update result omits changed"))?,
        exit.ok_or_else(|| malformed("interactive update result omits exit"))?,
        action.unwrap_or(None),
    ))
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
    let fields = product_fields(value, profile.frame, 7, "interactive frame")?;
    let mut rows = None;
    let mut columns = None;
    let mut scalars = None;
    let mut cursor_row = None;
    let mut cursor_column = None;
    let mut cursor_visible = None;
    let mut status = None;
    for field in fields {
        if field.field == profile.frame_rows_field {
            rows = Some(expect_i64(field.value, "interactive frame rows")?);
        } else if field.field == profile.frame_columns_field {
            columns = Some(expect_i64(field.value, "interactive frame columns")?);
        } else if field.field == profile.frame_scalars_field {
            scalars = Some(expect_scalars(profile.events.scalars, field.value)?);
        } else if field.field == profile.frame_cursor_row_field {
            cursor_row = Some(expect_i64(field.value, "interactive cursor row")?);
        } else if field.field == profile.frame_cursor_column_field {
            cursor_column = Some(expect_i64(field.value, "interactive cursor column")?);
        } else if field.field == profile.frame_cursor_visible_field {
            cursor_visible = Some(expect_bool(field.value, "interactive cursor visibility")?);
        } else if field.field == profile.frame_status_field {
            status = Some(expect_text(field.value, "interactive frame status")?);
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
    Ok(InteractiveFrame {
        rows,
        columns,
        scalars: scalars.ok_or_else(|| malformed("interactive frame omits scalars"))?,
        cursor_row,
        cursor_column,
        cursor_visible: cursor_visible
            .ok_or_else(|| malformed("interactive frame omits cursor visibility"))?,
        status,
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

fn validate_public_value_target(
    value: &ApplicationValue,
    expected: ApplicationTarget,
    label: &str,
) -> Result<()> {
    let actual = match value {
        ApplicationValue::Product { ty, .. }
        | ApplicationValue::Sum { ty, .. }
        | ApplicationValue::Sequence { ty, .. } => *ty,
        _ => return Err(malformed(format!("{label} is not nominal"))),
    };
    if actual == expected {
        Ok(())
    } else {
        Err(malformed(format!(
            "{label} has the wrong exact nominal type"
        )))
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
