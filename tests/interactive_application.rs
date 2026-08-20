#![allow(clippy::expect_used)]

use lkjscript::application::{
    InteractiveAction, InteractiveActionKind, InteractiveActionOutcome,
    InteractiveActionOutcomeClass, InteractiveCursorShape, InteractiveEvent, InteractiveKeyCode,
    InteractiveKeyEvent, InteractiveMouseButton, InteractiveMouseEvent, InteractiveMouseKind,
    InteractiveOpenEvent, MAXIMUM_INTERACTIVE_COLUMNS, MAXIMUM_INTERACTIVE_PASTE_SCALARS,
    prepare_interactive,
};
use lkjscript::error::ErrorCode;
use lkjscript::interactive_runner::{
    HEADLESS_REPLAY_CONTRACT_VERSION, HeadlessReplayRequest, HeadlessReplayTransition,
    run_headless_replay,
};
use std::fs;
use std::path::PathBuf;

fn checked_application() -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("applications/lkjedit/lkjedit.lkja"))
        .expect("checked lkjedit application")
}

fn key(value: char) -> InteractiveEvent {
    modified_key(value, false, false)
}

fn alt(value: char) -> InteractiveEvent {
    modified_key(value, false, true)
}

fn modified_key(value: char, control: bool, alt: bool) -> InteractiveEvent {
    InteractiveEvent::Key(InteractiveKeyEvent {
        code: InteractiveKeyCode::Character(value.into()),
        control,
        alt,
        shift: false,
        repeat: false,
    })
}

fn special(code: InteractiveKeyCode) -> InteractiveEvent {
    InteractiveEvent::Key(InteractiveKeyEvent {
        code,
        control: false,
        alt: false,
        shift: false,
        repeat: false,
    })
}

fn mouse(kind: InteractiveMouseKind, row: i64, column: i64) -> InteractiveEvent {
    InteractiveEvent::Mouse(InteractiveMouseEvent {
        button: InteractiveMouseButton::Primary,
        kind,
        row,
        column,
        control: false,
        alt: false,
        shift: false,
    })
}

fn command(value: &str) -> Vec<HeadlessReplayTransition> {
    std::iter::once(key(':'))
        .chain(value.chars().map(key))
        .chain(std::iter::once(special(InteractiveKeyCode::Enter)))
        .map(HeadlessReplayTransition::Event)
        .collect()
}

fn replay(events: Vec<HeadlessReplayTransition>) -> lkjscript::HeadlessReplayReceipt {
    run_headless_replay(
        &checked_application(),
        &HeadlessReplayRequest {
            version: HEADLESS_REPLAY_CONTRACT_VERSION,
            rows: 40,
            columns: 120,
            transitions: events,
        },
    )
    .expect("headless replay")
}

fn frame_text(receipt: &lkjscript::HeadlessReplayReceipt) -> String {
    receipt
        .final_frame
        .scalars
        .iter()
        .copied()
        .map(char::from_u32)
        .collect::<Option<String>>()
        .expect("valid frame scalars")
}

#[test]
fn checked_lkjedit_runs_vim_owned_insert_search_and_counted_undo() {
    let mut transitions = vec![
        HeadlessReplayTransition::Event(key('i')),
        HeadlessReplayTransition::Event(InteractiveEvent::Paste(
            "one\ntwo\nthree\nλe\u{301}界\n"
                .chars()
                .map(u32::from)
                .collect(),
        )),
        HeadlessReplayTransition::Event(special(InteractiveKeyCode::Escape)),
        HeadlessReplayTransition::Event(key('g')),
        HeadlessReplayTransition::Event(key('g')),
        HeadlessReplayTransition::Event(key('2')),
        HeadlessReplayTransition::Event(key('d')),
        HeadlessReplayTransition::Event(key('d')),
        HeadlessReplayTransition::Event(key('u')),
        HeadlessReplayTransition::Event(key('/')),
    ];
    transitions.extend(
        "three"
            .chars()
            .map(|value| HeadlessReplayTransition::Event(key(value))),
    );
    transitions.extend([
        HeadlessReplayTransition::Event(special(InteractiveKeyCode::Enter)),
        HeadlessReplayTransition::Event(key('n')),
        HeadlessReplayTransition::Event(key('N')),
        HeadlessReplayTransition::Event(InteractiveEvent::Close),
    ]);
    let first = replay(transitions.clone());
    let second = replay(transitions);
    assert_eq!(first.replay_digest, second.replay_digest);
    assert_eq!(first.final_frame_digest, second.final_frame_digest);
    assert_eq!(first.action_count, 0);
    assert_eq!(
        first.final_frame.cursor_shape,
        InteractiveCursorShape::Block
    );
    let final_text = frame_text(&first);
    let visible_lines = final_text.lines().map(str::trim_end).collect::<Vec<_>>();
    assert!(
        visible_lines
            .windows(4)
            .any(|lines| lines == ["one", "two", "three", "λe\u{301}界"]),
        "final frame did not contain the restored text:\n{final_text}"
    );
}

#[test]
fn checked_lkjedit_layout_and_mouse_are_application_owned_and_deterministic() {
    let mut transitions = Vec::new();
    for value in [
        "tabnew",
        "tabnew",
        "tabmoveleft",
        "vsplit",
        "split",
        "tabtonext",
    ] {
        transitions.extend(command(value));
    }
    transitions.extend([
        HeadlessReplayTransition::Event(mouse(InteractiveMouseKind::Press, 0, 3)),
        HeadlessReplayTransition::Event(mouse(InteractiveMouseKind::Drag, 0, 18)),
        HeadlessReplayTransition::Event(mouse(InteractiveMouseKind::Release, 0, 18)),
        HeadlessReplayTransition::Event(mouse(InteractiveMouseKind::Press, 0, 3)),
        HeadlessReplayTransition::Event(mouse(InteractiveMouseKind::Drag, 10, 58)),
        HeadlessReplayTransition::Event(mouse(InteractiveMouseKind::Release, 10, 58)),
        HeadlessReplayTransition::Event(InteractiveEvent::Resize {
            rows: 17,
            columns: 61,
        }),
        HeadlessReplayTransition::Event(InteractiveEvent::Close),
    ]);
    let receipt = replay(transitions);
    assert_eq!(
        receipt.replay_digest,
        "14f72b65103d00e1e008a8784a0e0d1009fb9e289dee5631db966489c590cb11"
    );
    assert_eq!(
        receipt.final_frame_digest,
        "aa271e03aeec0bd40fac2a2287ca45e6bb18a804f6a2b7f2c92c37a2ef69965c"
    );
    assert_eq!(receipt.action_count, 0);
    assert_eq!(receipt.changed_count, 63);
    assert_eq!(
        (receipt.final_frame.rows, receipt.final_frame.columns),
        (17, 61)
    );
}

#[test]
fn checked_lkjedit_keeps_local_input_responsive_during_one_read_job() {
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let (mut session, _) = prepared.start(40, 120).expect("initial state");
    let pending = session
        .step(InteractiveEvent::Open(InteractiveOpenEvent {
            path: String::new(),
            directory: false,
            project: true,
        }))
        .expect("project orientation request");
    let request = pending.action.expect("orientation action");
    assert_eq!(request.job_id, 1);
    assert_eq!(request.action, InteractiveAction::ProjectOrient);

    for event in [
        key('j'),
        mouse(InteractiveMouseKind::ScrollDown, 2, 2),
        special(InteractiveKeyCode::Down),
    ] {
        let local = session.step(event).expect("local input while pending");
        assert!(local.action.is_none());
        assert_eq!(session.pending_action_id(), Some(1));
    }

    let resumed = session
        .resume(InteractiveActionOutcome {
            job_id: 1,
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "orientation revision 121".into(),
            content: "workspace revision 121".into(),
            token: String::new(),
        })
        .expect("matching outcome");
    assert!(resumed.action.is_none());
    assert_eq!(session.pending_action_id(), None);

    let duplicate = session
        .resume(InteractiveActionOutcome {
            job_id: 1,
            class: InteractiveActionOutcomeClass::Succeeded,
            message: String::new(),
            content: String::new(),
            token: String::new(),
        })
        .expect_err("duplicate outcome");
    assert_eq!(duplicate.code, ErrorCode::ProtocolMalformed);
}

#[test]
fn headless_receipt_exposes_bounded_exact_action_identities() {
    let receipt = replay(vec![
        HeadlessReplayTransition::Event(alt('o')),
        HeadlessReplayTransition::Outcome(InteractiveActionOutcome {
            job_id: 1,
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "orientation".into(),
            content: "workspace".into(),
            token: String::new(),
        }),
        HeadlessReplayTransition::Event(alt('k')),
        HeadlessReplayTransition::Outcome(InteractiveActionOutcome {
            job_id: 2,
            class: InteractiveActionOutcomeClass::Succeeded,
            message: "target passed".into(),
            content: "12 passed, 0 failed".into(),
            token: String::new(),
        }),
        HeadlessReplayTransition::Event(InteractiveEvent::Close),
    ]);
    assert_eq!(
        receipt
            .action_trace
            .iter()
            .map(|item| (item.job_id, item.kind))
            .collect::<Vec<_>>(),
        vec![
            (1, InteractiveActionKind::ProjectOrient),
            (2, InteractiveActionKind::ProjectTargetTest),
        ]
    );
    assert!(
        receipt
            .action_trace
            .iter()
            .all(|item| item.payload_digest.len() == 64)
    );
}

#[test]
fn interactive_adapter_rejects_exact_one_over_host_bounds() {
    let prepared = prepare_interactive(&checked_application()).expect("interactive profile");
    let (mut session, _) = prepared.start(1, 1).expect("minimum dimensions");
    let resize = session
        .step(InteractiveEvent::Resize {
            rows: 1,
            columns: MAXIMUM_INTERACTIVE_COLUMNS + 1,
        })
        .expect_err("one-over columns");
    assert_eq!(resize.code, ErrorCode::PolicyExceeded);
    let paste = session
        .step(InteractiveEvent::Paste(vec![
            u32::from('x');
            MAXIMUM_INTERACTIVE_PASTE_SCALARS
                + 1
        ]))
        .expect_err("one-over paste");
    assert_eq!(paste.code, ErrorCode::PolicyExceeded);
}

#[test]
fn missing_and_foreign_outcomes_preserve_authority() {
    let application = checked_application();
    let missing = HeadlessReplayRequest {
        version: HEADLESS_REPLAY_CONTRACT_VERSION,
        rows: 24,
        columns: 80,
        transitions: vec![HeadlessReplayTransition::Event(alt('o'))],
    };
    assert_eq!(
        run_headless_replay(&application, &missing)
            .expect_err("missing result")
            .code,
        ErrorCode::ProtocolMalformed
    );

    let foreign = HeadlessReplayRequest {
        version: HEADLESS_REPLAY_CONTRACT_VERSION,
        rows: 24,
        columns: 80,
        transitions: vec![HeadlessReplayTransition::Outcome(
            InteractiveActionOutcome {
                job_id: 9,
                class: InteractiveActionOutcomeClass::Succeeded,
                message: String::new(),
                content: String::new(),
                token: String::new(),
            },
        )],
    };
    assert_eq!(
        run_headless_replay(&application, &foreign)
            .expect_err("foreign result")
            .code,
        ErrorCode::ProtocolMalformed
    );
}
