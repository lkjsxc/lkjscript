#![allow(clippy::expect_used, clippy::panic)]

use lkjscript::application::{
    InteractiveAction, InteractiveActionOutcome, InteractiveActionOutcomeClass, InteractiveEvent,
    InteractiveKeyCode, InteractiveKeyEvent, MAXIMUM_INTERACTIVE_COLUMNS,
    MAXIMUM_INTERACTIVE_FRAME_SCALARS, MAXIMUM_INTERACTIVE_PASTE_SCALARS, prepare_interactive,
};
use lkjscript::error::ErrorCode;
use lkjscript::interactive_runner::{
    HEADLESS_REPLAY_CONTRACT_VERSION, HeadlessReplayRequest, run_headless_replay,
};
use lkjscript::workbench_host::WorkbenchHost;
use std::fs;
use std::path::PathBuf;

const WORKBENCH_CHROME: &str = "[Explorer] M-O orient M-E children M-I function M-U callers M-D callees M-T targets M-B blockers\n\
[Proposal] M-P open M-V validate M-X apply\n\
[Review] M-W diff M-H history M-N record M-K test M-L build M-Z run\n\
[Files] M-J list M-F open M-S save M-R reconcile\n\
[Editor] ^A select all ^N new ^W close ^Z undo ^Y redo ^Q quit\n";
const MAXIMUM_LKJSTUDIO_CONTENT_SCALARS: usize =
    MAXIMUM_INTERACTIVE_FRAME_SCALARS - WORKBENCH_CHROME.len();

fn expected_scalars(content: &str) -> Vec<u32> {
    WORKBENCH_CHROME
        .chars()
        .chain(content.chars())
        .map(u32::from)
        .collect()
}

fn checked_application() -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("applications/lkjstudio/lkjstudio.lkja"),
    )
    .expect("checked lkjstudio application")
}

fn character(value: char) -> InteractiveEvent {
    InteractiveEvent::Key(InteractiveKeyEvent {
        code: InteractiveKeyCode::Character(value.into()),
        control: false,
        alt: false,
        shift: false,
        repeat: false,
    })
}

fn control_character(value: char) -> InteractiveEvent {
    InteractiveEvent::Key(InteractiveKeyEvent {
        code: InteractiveKeyCode::Character(value.into()),
        control: true,
        alt: false,
        shift: false,
        repeat: false,
    })
}

fn alt_character(value: char) -> InteractiveEvent {
    InteractiveEvent::Key(InteractiveKeyEvent {
        code: InteractiveKeyCode::Character(value.into()),
        control: false,
        alt: true,
        shift: false,
        repeat: false,
    })
}

#[test]
fn checked_lkjstudio_runs_application_owned_interaction() {
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let digest = prepared.digest();
    let (mut session, initial) = prepared.start(24, 80).expect("initial state and frame");
    assert_eq!(session.application_digest(), digest);
    assert_eq!((initial.frame.rows, initial.frame.columns), (24, 80));
    assert_eq!(initial.frame.scalars, expected_scalars(""));
    assert_eq!(initial.frame.status, "ready");
    assert!(!initial.changed);
    assert!(!initial.exit);

    let inserted = session.step(character('A')).expect("character update");
    assert!(inserted.changed);
    assert!(!inserted.exit);
    assert_eq!(inserted.frame.scalars, expected_scalars("A"));
    assert_eq!(
        (inserted.frame.cursor_row, inserted.frame.cursor_column),
        (5, 1)
    );

    let pasted = session
        .step(InteractiveEvent::Paste(vec![
            u32::from('λ'),
            u32::from('!'),
        ]))
        .expect("paste update");
    assert!(pasted.changed);
    assert_eq!(pasted.frame.scalars, expected_scalars("Aλ!"));

    let undone = session
        .step(control_character('z'))
        .expect("editor undo command");
    assert!(undone.action.is_none());
    assert_eq!(undone.frame.scalars, expected_scalars("A"));

    let redone = session
        .step(control_character('y'))
        .expect("editor redo command");
    assert!(redone.action.is_none());
    assert_eq!(redone.frame.scalars, expected_scalars("Aλ!"));

    let resized = session
        .step(InteractiveEvent::Resize {
            rows: 7,
            columns: 19,
        })
        .expect("resize update");
    assert_eq!((resized.frame.rows, resized.frame.columns), (7, 19));

    let closed = session.step(InteractiveEvent::Close).expect("close update");
    assert!(closed.exit);
}

#[test]
fn interactive_adapter_rejects_invalid_or_excessive_host_observations() {
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let (mut session, _) = prepared.start(1, 1).expect("minimum dimensions");

    let excessive_resize = session
        .step(InteractiveEvent::Resize {
            rows: 1,
            columns: MAXIMUM_INTERACTIVE_COLUMNS + 1,
        })
        .expect_err("excessive resize");
    assert_eq!(excessive_resize.code, ErrorCode::PolicyExceeded);

    let invalid_scalar = session
        .step(InteractiveEvent::Paste(vec![0x11_0000]))
        .expect_err("invalid Unicode scalar");
    assert_eq!(invalid_scalar.code, ErrorCode::PolicyExceeded);

    let excessive_paste = session
        .step(InteractiveEvent::Paste(vec![
            u32::from('x');
            MAXIMUM_INTERACTIVE_PASTE_SCALARS
                + 1
        ]))
        .expect_err("excessive paste");
    assert_eq!(excessive_paste.code, ErrorCode::PolicyExceeded);
}

#[test]
fn render_failure_preserves_prior_interactive_state() {
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let (mut session, _) = prepared.start(24, 80).expect("initial state");
    session
        .step(alt_character('o'))
        .expect("semantic project orientation action");
    let exact = session
        .resume(InteractiveActionOutcome {
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "exact frame".into(),
            content: "x".repeat(MAXIMUM_LKJSTUDIO_CONTENT_SCALARS),
            token: String::new(),
        })
        .expect("exact frame boundary");
    assert_eq!(exact.frame.scalars.len(), MAXIMUM_INTERACTIVE_FRAME_SCALARS);

    let excessive = session
        .step(character('x'))
        .expect_err("one-over frame must reject");
    assert_eq!(excessive.code, ErrorCode::PolicyExceeded);
    assert_eq!(
        session.frame().expect("prior state remains renderable"),
        exact.frame
    );
}

#[test]
fn checked_lkjstudio_emits_one_typed_action_and_resumes_through_application_meaning() {
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let (mut session, _) = prepared.start(24, 80).expect("initial state");
    let pending = session
        .step(alt_character('o'))
        .expect("semantic project orientation action");
    assert_eq!(pending.action, Some(InteractiveAction::ProjectOrient));
    assert_eq!(pending.frame.status, "pending:project_orient");

    let busy = session
        .step(character('x'))
        .expect_err("one unresolved action at a time");
    assert_eq!(busy.code, ErrorCode::AuthorityBusy);

    let resumed = session
        .resume(InteractiveActionOutcome {
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "orientation revision 38".into(),
            content: "exact project orientation".into(),
            token: String::new(),
        })
        .expect("application-owned outcome transition");
    assert!(resumed.action.is_none());
    assert_eq!(resumed.frame.status, "orientation revision 38");
    assert_eq!(
        resumed.frame.scalars,
        expected_scalars("exact project orientation")
    );
}

#[test]
fn failed_action_resume_keeps_the_action_pending() {
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let (mut session, _) = prepared.start(24, 80).expect("initial state");
    session
        .step(alt_character('o'))
        .expect("semantic project orientation action");

    let excessive = session
        .resume(InteractiveActionOutcome {
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "oversized orientation".into(),
            content: "x".repeat(MAXIMUM_LKJSTUDIO_CONTENT_SCALARS + 1),
            token: String::new(),
        })
        .expect_err("unrenderable outcome must reject");
    assert_eq!(excessive.code, ErrorCode::PolicyExceeded);

    let recovered = session
        .resume(InteractiveActionOutcome {
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "orientation revision 46".into(),
            content: "bounded orientation".into(),
            token: String::new(),
        })
        .expect("pending action remains resumable");
    assert_eq!(recovered.frame.status, "orientation revision 46");
    assert_eq!(
        recovered.frame.scalars,
        expected_scalars("bounded orientation")
    );
}

#[test]
fn headless_replay_is_deterministic_and_uses_the_same_event_owner() {
    let request = HeadlessReplayRequest {
        version: HEADLESS_REPLAY_CONTRACT_VERSION,
        rows: 24,
        columns: 80,
        events: vec![
            character('A'),
            InteractiveEvent::Paste(vec![u32::from('λ')]),
            InteractiveEvent::Resize {
                rows: 7,
                columns: 19,
            },
            InteractiveEvent::Close,
        ],
        outcomes: vec![],
    };
    let application = checked_application();
    let first = run_headless_replay(&application, &request).expect("first replay");
    let second = run_headless_replay(&application, &request).expect("second replay");
    assert_eq!(first.replay_digest, second.replay_digest);
    assert_eq!(first.initial_frame_digest, second.initial_frame_digest);
    assert_eq!(first.final_frame_digest, second.final_frame_digest);
    assert_eq!(first.final_frame, second.final_frame);
    assert_eq!(first.event_count, 4);
    assert_eq!(first.action_count, 0);
    assert_eq!(first.exit_event, Some(4));
    assert_eq!(first.changed_count, 3);
}

#[test]
fn headless_replay_binds_action_outcomes_and_rejects_missing_or_extra_outcomes() {
    let request = HeadlessReplayRequest {
        version: HEADLESS_REPLAY_CONTRACT_VERSION,
        rows: 24,
        columns: 80,
        events: vec![alt_character('o'), InteractiveEvent::Close],
        outcomes: vec![InteractiveActionOutcome {
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "orientation revision 38".into(),
            content: "project view".into(),
            token: String::new(),
        }],
    };
    let application = checked_application();
    let receipt = run_headless_replay(&application, &request).expect("action replay");
    assert_eq!(receipt.event_count, 2);
    assert_eq!(receipt.action_count, 1);
    assert_eq!(receipt.exit_event, Some(2));
    assert_eq!(receipt.final_frame.status, "orientation revision 38");
    assert_eq!(
        receipt.final_frame.scalars,
        expected_scalars("project view")
    );

    let mut missing = request.clone();
    missing.outcomes.clear();
    let error = run_headless_replay(&application, &missing).expect_err("missing outcome");
    assert_eq!(error.code, ErrorCode::ProtocolMalformed);

    let mut extra = request;
    extra.events = vec![InteractiveEvent::Close];
    let error = run_headless_replay(&application, &extra).expect_err("extra outcome");
    assert_eq!(error.code, ErrorCode::ProtocolMalformed);
}

#[test]
fn checked_lkjstudio_has_one_closed_alt_action_keymap() {
    let application = checked_application();
    let mut routes = Vec::new();
    for key in 'a'..='z' {
        let prepared = prepare_interactive(&application).expect("interactive profile");
        let (mut session, _) = prepared.start(24, 80).expect("initial state");
        if let Ok(step) = session.step(alt_character(key))
            && let Some(action) = step.action
        {
            routes.push((key, action));
        }
    }
    assert_eq!(
        routes,
        vec![
            ('b', InteractiveAction::ProjectBlockers),
            ('d', InteractiveAction::ProjectCallees(String::new())),
            ('e', InteractiveAction::ProjectChildren(String::new())),
            ('f', InteractiveAction::FilesystemRead(String::new())),
            ('g', InteractiveAction::ProjectSummary(String::new())),
            ('h', InteractiveAction::ProjectHistory),
            ('i', InteractiveAction::ProjectFunction(String::new())),
            ('j', InteractiveAction::FilesystemList(String::new())),
            ('k', InteractiveAction::ProjectTargetTest(String::new())),
            ('l', InteractiveAction::ProjectTargetBuild(String::new())),
            ('n', InteractiveAction::ProjectRecord(String::new())),
            ('o', InteractiveAction::ProjectOrient),
            ('p', InteractiveAction::ProjectProposal(String::new())),
            ('r', InteractiveAction::FilesystemReconcile(String::new())),
            (
                's',
                InteractiveAction::FilesystemSave {
                    origin: String::new(),
                    content: String::new(),
                    create: false,
                },
            ),
            ('t', InteractiveAction::ProjectTargets(String::new())),
            ('u', InteractiveAction::ProjectCallers(String::new())),
            ('v', InteractiveAction::ProjectValidate(String::new())),
            ('w', InteractiveAction::ProjectDiff(String::new())),
            ('x', InteractiveAction::ProjectApply(String::new())),
            ('y', InteractiveAction::ProjectTargetList),
            ('z', InteractiveAction::ProjectTargetRun(String::new())),
        ]
    );
}

#[test]
fn checked_lkjstudio_keeps_file_origin_out_of_rendered_status() {
    let temporary = tempfile::tempdir().expect("selected root");
    fs::write(temporary.path().join("note.txt"), "first").expect("initial file");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("applications/lkjstudio");
    let mut host =
        WorkbenchHost::open(Some(&project), Some(temporary.path())).expect("host grants");
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let (mut session, _) = prepared.start(24, 80).expect("initial state");

    session
        .step(InteractiveEvent::Paste(
            "note.txt".chars().map(u32::from).collect(),
        ))
        .expect("file locator proposal");
    let read = session.step(alt_character('f')).expect("file read action");
    assert_eq!(
        read.action,
        Some(InteractiveAction::FilesystemRead("note.txt".into()))
    );
    let opened = host
        .handle(read.action.expect("read action"))
        .expect("host read");
    assert_eq!(opened.class, InteractiveActionOutcomeClass::Succeeded);
    assert_eq!(opened.content, "first");
    assert!(!opened.token.is_empty());
    let origin = opened.token.clone();
    let opened = session.resume(opened).expect("semantic open transition");
    assert_eq!(opened.frame.status, "opened note.txt");
    assert!(!opened.frame.status.contains(&origin));
    assert_eq!(opened.frame.scalars, expected_scalars("first"));

    session.step(character('!')).expect("edit opened file");
    let save = session.step(alt_character('s')).expect("file save action");
    let Some(InteractiveAction::FilesystemSave {
        origin: requested_origin,
        content,
        create,
    }) = save.action.clone()
    else {
        panic!("expected file save action")
    };
    assert_eq!(requested_origin, origin);
    assert_eq!(content, "first!");
    assert!(!create);
    let saved = host
        .handle(save.action.expect("save action"))
        .expect("host save");
    assert_eq!(saved.class, InteractiveActionOutcomeClass::Succeeded);
    let saved_origin = saved.token.clone();
    let saved = session.resume(saved).expect("semantic save transition");
    assert_eq!(saved.frame.status, "saved note.txt");
    assert!(!saved.frame.status.contains(&saved_origin));
    assert_eq!(
        fs::read_to_string(temporary.path().join("note.txt")).expect("saved file"),
        "first!"
    );

    fs::write(temporary.path().join("note.txt"), "external").expect("external change");
    session.step(character('?')).expect("conflicting edit");
    let conflict = session
        .step(alt_character('s'))
        .expect("conflicting save action");
    let conflict = host
        .handle(conflict.action.expect("conflicting save"))
        .expect("host conflict");
    assert_eq!(conflict.class, InteractiveActionOutcomeClass::Conflict);
    let conflict = session
        .resume(conflict)
        .expect("semantic conflict transition");
    assert!(conflict.frame.status.starts_with("file save conflict:"));
    assert_eq!(conflict.frame.scalars, expected_scalars("first!?"));
    assert_eq!(
        fs::read_to_string(temporary.path().join("note.txt")).expect("external file"),
        "external"
    );
}
