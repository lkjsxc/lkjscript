//! Narrow native terminal adaptation for interactive application artifacts.
//!
//! This module owns raw-mode lifecycle, bounded host-event decoding, acknowledged differential
//! projection with a full-frame oracle, one bounded host worker, and cleanup. It does not own key
//! policy, editor state, commands, or frame meaning.

use crate::application::{
    InteractiveAction, InteractiveActionOutcome, InteractiveCursorShape, InteractiveEvent,
    InteractiveFrame, InteractiveKeyCode, InteractiveKeyEvent, InteractiveMouseButton,
    InteractiveMouseEvent, InteractiveMouseKind, MAXIMUM_INTERACTIVE_COLUMNS,
    MAXIMUM_INTERACTIVE_PASTE_SCALARS, MAXIMUM_INTERACTIVE_ROWS, prepare_interactive,
};
use crate::error::{ErrorCode, LkError, Result};
use crossterm::cursor::{Hide, MoveTo, SetCursorStyle, Show};
use crossterm::event::{
    self, DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange,
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::style::{
    Attribute, Color, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, QueueableCommand};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fd::AsFd;
use serde::Serialize;
use signal_hook::SigId;
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use std::io::{self, IsTerminal, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use unicode_width::UnicodeWidthChar;

pub const TERMINAL_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_TERMINAL_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_TERMINAL_ACTIONS: u64 = 10_000;
pub const MAXIMUM_TERMINAL_INITIAL_EVENTS: usize = 4;
pub const TERMINAL_POLL_MILLISECONDS: u64 = 25;

struct ActionWorker {
    requests: Option<SyncSender<crate::application::InteractiveActionRequest>>,
    results: Receiver<Result<InteractiveActionOutcome>>,
    thread: Option<JoinHandle<()>>,
    pending: bool,
}

impl ActionWorker {
    fn start(
        mut handle_action: impl FnMut(InteractiveAction) -> Result<InteractiveActionOutcome>
        + Send
        + 'static,
    ) -> Result<Self> {
        let (request_sender, request_receiver) =
            sync_channel::<crate::application::InteractiveActionRequest>(1);
        let (result_sender, result_receiver) = sync_channel::<Result<InteractiveActionOutcome>>(1);
        let thread = thread::Builder::new()
            .name("lkjedit-host-worker".into())
            .spawn(move || {
                while let Ok(request) = request_receiver.recv() {
                    let job_id = request.job_id;
                    let result =
                        match catch_unwind(AssertUnwindSafe(|| handle_action(request.action))) {
                            Ok(Ok(outcome)) => Ok(outcome.with_job_id(job_id)),
                            Ok(Err(error)) => Err(error),
                            Err(_) => Err(LkError::new(
                                ErrorCode::HostOutcomeUnknown,
                                "bounded host worker panicked; external visibility may be unknown",
                            )),
                        };
                    if result_sender.send(result).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| {
                LkError::new(
                    ErrorCode::TerminalUnavailable,
                    format!("cannot create bounded host worker: {error}"),
                )
            })?;
        Ok(Self {
            requests: Some(request_sender),
            results: result_receiver,
            thread: Some(thread),
            pending: false,
        })
    }

    fn submit(&mut self, request: crate::application::InteractiveActionRequest) -> Result<()> {
        if self.pending {
            return Err(LkError::new(
                ErrorCode::AuthorityBusy,
                "bounded host worker already has one pending job",
            ));
        }
        let sender = self.requests.as_ref().ok_or_else(|| {
            LkError::new(
                ErrorCode::TerminalUnavailable,
                "bounded host worker request channel is closed",
            )
        })?;
        match sender.try_send(request) {
            Ok(()) => {
                self.pending = true;
                Ok(())
            }
            Err(TrySendError::Full(_)) => Err(LkError::new(
                ErrorCode::AuthorityBusy,
                "bounded host worker request queue is full",
            )),
            Err(TrySendError::Disconnected(_)) => Err(LkError::new(
                ErrorCode::TerminalUnavailable,
                "bounded host worker request channel disconnected",
            )),
        }
    }

    fn try_result(&mut self) -> Result<Option<InteractiveActionOutcome>> {
        match self.results.try_recv() {
            Ok(Ok(outcome)) => {
                self.pending = false;
                Ok(Some(outcome))
            }
            Ok(Err(error)) => {
                self.pending = false;
                Err(error)
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(LkError::new(
                ErrorCode::TerminalUnavailable,
                "bounded host worker result channel disconnected",
            )),
        }
    }

    fn shutdown(&mut self, wait: bool) {
        self.requests.take();
        if wait && let Some(thread) = self.thread.take() {
            let _ = thread.join();
        } else {
            self.thread.take();
        }
    }
}

impl Drop for ActionWorker {
    fn drop(&mut self) {
        self.shutdown(false);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalExitReason {
    Application,
    Signal,
    Eof,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalRunReceipt {
    pub version: u16,
    pub application: crate::application::ApplicationDigest,
    pub events: u64,
    pub actions: u64,
    pub frames: u64,
    pub reason: TerminalExitReason,
}

pub fn adapt_terminal_event(event: Event) -> Result<Option<InteractiveEvent>> {
    match event {
        Event::Key(key) => {
            if key
                .modifiers
                .intersects(KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META)
            {
                return Err(LkError::new(
                    ErrorCode::TerminalDecode,
                    "terminal key uses an unsupported super, hyper, or meta modifier",
                ));
            }
            let repeat = match key.kind {
                KeyEventKind::Press => false,
                KeyEventKind::Repeat => true,
                KeyEventKind::Release => return Ok(None),
            };
            let code = match key.code {
                KeyCode::Char(value) => InteractiveKeyCode::Character(value.into()),
                KeyCode::Enter => InteractiveKeyCode::Enter,
                KeyCode::Backspace => InteractiveKeyCode::Backspace,
                KeyCode::Delete => InteractiveKeyCode::Delete,
                KeyCode::Left => InteractiveKeyCode::Left,
                KeyCode::Right => InteractiveKeyCode::Right,
                KeyCode::Up => InteractiveKeyCode::Up,
                KeyCode::Down => InteractiveKeyCode::Down,
                KeyCode::Home => InteractiveKeyCode::Home,
                KeyCode::End => InteractiveKeyCode::End,
                KeyCode::Esc => InteractiveKeyCode::Escape,
                KeyCode::Tab | KeyCode::BackTab => InteractiveKeyCode::Character(u32::from('\t')),
                _ => return Ok(None),
            };
            Ok(Some(InteractiveEvent::Key(InteractiveKeyEvent {
                code,
                control: key.modifiers.contains(KeyModifiers::CONTROL),
                alt: key.modifiers.contains(KeyModifiers::ALT),
                shift: key.modifiers.contains(KeyModifiers::SHIFT),
                repeat,
            })))
        }
        Event::Paste(value) => {
            let scalars = value.chars().map(u32::from).collect::<Vec<_>>();
            if scalars.len() > MAXIMUM_INTERACTIVE_PASTE_SCALARS {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "terminal paste exceeds scalar-count policy",
                ));
            }
            Ok(Some(InteractiveEvent::Paste(scalars)))
        }
        Event::Resize(columns, rows) => {
            let rows = i64::from(rows).clamp(1, MAXIMUM_INTERACTIVE_ROWS);
            let columns = i64::from(columns).clamp(1, MAXIMUM_INTERACTIVE_COLUMNS);
            Ok(Some(InteractiveEvent::Resize { rows, columns }))
        }
        Event::FocusGained => Ok(Some(InteractiveEvent::FocusGained)),
        Event::FocusLost => Ok(Some(InteractiveEvent::FocusLost)),
        Event::Mouse(mouse) => {
            if mouse
                .modifiers
                .intersects(KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META)
            {
                return Err(LkError::new(
                    ErrorCode::TerminalDecode,
                    "terminal mouse event uses an unsupported super, hyper, or meta modifier",
                ));
            }
            let button = |value| match value {
                MouseButton::Left => InteractiveMouseButton::Primary,
                MouseButton::Middle => InteractiveMouseButton::Middle,
                MouseButton::Right => InteractiveMouseButton::Secondary,
            };
            let (kind, button) = match mouse.kind {
                MouseEventKind::Down(value) => (InteractiveMouseKind::Press, button(value)),
                MouseEventKind::Up(value) => (InteractiveMouseKind::Release, button(value)),
                MouseEventKind::Drag(value) => (InteractiveMouseKind::Drag, button(value)),
                MouseEventKind::ScrollUp => {
                    (InteractiveMouseKind::ScrollUp, InteractiveMouseButton::None)
                }
                MouseEventKind::ScrollDown => (
                    InteractiveMouseKind::ScrollDown,
                    InteractiveMouseButton::None,
                ),
                MouseEventKind::ScrollLeft => (
                    InteractiveMouseKind::ScrollLeft,
                    InteractiveMouseButton::None,
                ),
                MouseEventKind::ScrollRight => (
                    InteractiveMouseKind::ScrollRight,
                    InteractiveMouseButton::None,
                ),
                MouseEventKind::Moved => return Ok(None),
            };
            Ok(Some(InteractiveEvent::Mouse(InteractiveMouseEvent {
                button,
                kind,
                row: i64::from(mouse.row).min(MAXIMUM_INTERACTIVE_ROWS - 1),
                column: i64::from(mouse.column).min(MAXIMUM_INTERACTIVE_COLUMNS - 1),
                control: mouse.modifiers.contains(KeyModifiers::CONTROL),
                alt: mouse.modifiers.contains(KeyModifiers::ALT),
                shift: mouse.modifiers.contains(KeyModifiers::SHIFT),
            })))
        }
    }
}

pub fn terminal_frame_bytes(frame: &InteractiveFrame) -> Result<Vec<u8>> {
    let estimated = frame
        .scalars
        .len()
        .checked_mul(char::MAX_LEN_UTF8)
        .and_then(|bytes| bytes.checked_add(4_096))
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "terminal frame output estimate overflows",
            )
        })?;
    let mut output = BoundedOutput::with_capacity(estimated.min(MAXIMUM_TERMINAL_FRAME_BYTES));
    project_frame(&mut output, frame).map_err(terminal_output)?;
    Ok(output.finish())
}

pub fn run_terminal(application_bytes: &[u8]) -> Result<TerminalRunReceipt> {
    run_terminal_with_actions(application_bytes, |_| {
        Err(LkError::new(
            ErrorCode::CapabilityDenied,
            "interactive application requested a host action without an explicit adapter",
        ))
    })
}

pub fn run_terminal_with_actions(
    application_bytes: &[u8],
    handle_action: impl FnMut(InteractiveAction) -> Result<InteractiveActionOutcome> + Send + 'static,
) -> Result<TerminalRunReceipt> {
    run_terminal_with_actions_and_initial_events(application_bytes, Vec::new(), handle_action)
}

/// Runs one interactive application after delivering bounded deployment-selected events.
///
/// Initial events are ordinary application inputs. They let a product translate its launch
/// selection into the same typed `Open` event used by headless and live workflows without giving
/// the native runner any editor policy.
pub fn run_terminal_with_actions_and_initial_events(
    application_bytes: &[u8],
    initial_events: Vec<InteractiveEvent>,
    handle_action: impl FnMut(InteractiveAction) -> Result<InteractiveActionOutcome> + Send + 'static,
) -> Result<TerminalRunReceipt> {
    if initial_events.len() > MAXIMUM_TERMINAL_INITIAL_EVENTS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "terminal initial-event count exceeds policy",
        ));
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    if !stdin.is_terminal() || !stdout.is_terminal() {
        return Err(LkError::new(
            ErrorCode::TerminalUnavailable,
            "interactive execution requires trusted local terminal input and output",
        ));
    }
    let signal = SignalFlag::register()?;
    let (columns, rows) = terminal::size().map_err(|error| {
        LkError::new(
            ErrorCode::TerminalUnavailable,
            format!("cannot inspect terminal dimensions: {error}"),
        )
    })?;
    let rows = i64::from(rows).clamp(1, MAXIMUM_INTERACTIVE_ROWS);
    let columns = i64::from(columns).clamp(1, MAXIMUM_INTERACTIVE_COLUMNS);
    let prepared = prepare_interactive(application_bytes)?;
    let application = prepared.digest();
    let (mut application_session, initial) = prepared.start(rows, columns)?;
    let mut worker = ActionWorker::start(handle_action)?;
    let backend = CrosstermBackend::new(stdout.lock());
    let mut terminal_session = TerminalLease::acquire(backend)?;
    let mut events = 0_u64;
    let mut actions = 0_u64;
    let mut frames = 0_u64;
    let outcome = (|| {
        if present_terminal_step(
            &mut terminal_session,
            &mut worker,
            initial,
            &mut actions,
            &mut frames,
        )? {
            if application_session.pending_action_id().is_some() {
                return Err(LkError::new(
                    ErrorCode::AuthorityBusy,
                    "interactive application requested exit while a host job is pending",
                ));
            }
            return Ok(TerminalExitReason::Application);
        }
        for event in initial_events {
            let step = application_session.step(event)?;
            events = events.saturating_add(1);
            if present_terminal_step(
                &mut terminal_session,
                &mut worker,
                step,
                &mut actions,
                &mut frames,
            )? {
                if application_session.pending_action_id().is_some() {
                    return Err(LkError::new(
                        ErrorCode::AuthorityBusy,
                        "interactive application requested exit while a host job is pending",
                    ));
                }
                return Ok(TerminalExitReason::Application);
            }
        }
        loop {
            if let Some(result) = worker.try_result()? {
                let step = application_session.resume(result)?;
                if present_terminal_step(
                    &mut terminal_session,
                    &mut worker,
                    step,
                    &mut actions,
                    &mut frames,
                )? {
                    if application_session.pending_action_id().is_some() {
                        return Err(LkError::new(
                            ErrorCode::AuthorityBusy,
                            "interactive application requested exit while a host job is pending",
                        ));
                    }
                    return Ok(TerminalExitReason::Application);
                }
                continue;
            }
            if signal.raised() {
                return Ok(TerminalExitReason::Signal);
            }
            if terminal_input_disconnected(&stdin)? {
                return Ok(TerminalExitReason::Eof);
            }
            match event::poll(Duration::from_millis(TERMINAL_POLL_MILLISECONDS)) {
                Ok(false) => continue,
                Ok(true) => {}
                Err(error) if terminal_input_ended(&error) => {
                    return Ok(TerminalExitReason::Eof);
                }
                Err(error) => {
                    return Err(LkError::new(
                        ErrorCode::TerminalDecode,
                        format!("terminal input polling failed: {error}"),
                    ));
                }
            }
            if terminal_input_disconnected(&stdin)? {
                return Ok(TerminalExitReason::Eof);
            }
            let host_event = match event::read() {
                Ok(event) => event,
                Err(error) if terminal_input_ended(&error) => {
                    return Ok(TerminalExitReason::Eof);
                }
                Err(error) => {
                    return Err(LkError::new(
                        ErrorCode::TerminalDecode,
                        format!("terminal input decoding failed: {error}"),
                    ));
                }
            };
            let Some(event) = adapt_terminal_event(host_event)? else {
                continue;
            };
            let step = application_session.step(event)?;
            events = events.saturating_add(1);
            if present_terminal_step(
                &mut terminal_session,
                &mut worker,
                step,
                &mut actions,
                &mut frames,
            )? {
                if application_session.pending_action_id().is_some() {
                    return Err(LkError::new(
                        ErrorCode::AuthorityBusy,
                        "interactive application requested exit while a host job is pending",
                    ));
                }
                return Ok(TerminalExitReason::Application);
            }
        }
    })();
    worker.shutdown(application_session.pending_action_id().is_none());
    let cleanup = terminal_session.close();
    match (outcome, cleanup) {
        (_, Err(cleanup)) => Err(cleanup),
        (Err(error), Ok(())) => Err(error),
        (Ok(reason), Ok(())) => Ok(TerminalRunReceipt {
            version: TERMINAL_CONTRACT_VERSION,
            application,
            events,
            actions,
            frames,
            reason,
        }),
    }
}

fn present_terminal_step<B: TerminalBackend>(
    terminal: &mut TerminalLease<B>,
    worker: &mut ActionWorker,
    step: crate::application::InteractiveStep,
    actions: &mut u64,
    frames: &mut u64,
) -> Result<bool> {
    terminal.write_frame(&step.frame)?;
    *frames = frames.saturating_add(1);
    if step.exit && step.action.is_some() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "interactive application cannot request exit and a host job in one step",
        ));
    }
    if let Some(action) = step.action {
        if *actions >= MAXIMUM_TERMINAL_ACTIONS {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "terminal host-action count exceeds policy",
            ));
        }
        worker.submit(action)?;
        *actions = actions.saturating_add(1);
    }
    Ok(step.exit)
}

fn terminal_input_ended(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::UnexpectedEof
        || error.raw_os_error() == Some(rustix::io::Errno::IO.raw_os_error())
}

fn terminal_input_disconnected(input: &impl AsFd) -> Result<bool> {
    let mut descriptors = [PollFd::new(input, PollFlags::IN)];
    poll(
        &mut descriptors,
        Some(&Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )
    .map_err(|error| {
        LkError::new(
            ErrorCode::TerminalDecode,
            format!("cannot inspect terminal input lifecycle: {error}"),
        )
    })?;
    let ready = descriptors[0].revents();
    if ready.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL) {
        return Ok(true);
    }
    if ready.contains(PollFlags::IN) {
        let bytes = rustix::io::ioctl_fionread(input).map_err(|error| {
            LkError::new(
                ErrorCode::TerminalDecode,
                format!("cannot inspect ready terminal input bytes: {error}"),
            )
        })?;
        return Ok(bytes == 0);
    }
    Ok(false)
}

trait TerminalBackend {
    fn enable_raw(&mut self) -> io::Result<()>;
    fn enter_alternate(&mut self) -> io::Result<()>;
    fn enable_paste(&mut self) -> io::Result<()>;
    fn enable_mouse(&mut self) -> io::Result<()>;
    fn enable_focus(&mut self) -> io::Result<()>;
    fn hide_cursor(&mut self) -> io::Result<()>;
    fn show_cursor(&mut self) -> io::Result<()>;
    fn disable_focus(&mut self) -> io::Result<()>;
    fn disable_mouse(&mut self) -> io::Result<()>;
    fn disable_paste(&mut self) -> io::Result<()>;
    fn leave_alternate(&mut self) -> io::Result<()>;
    fn disable_raw(&mut self) -> io::Result<()>;
    fn write_frame(&mut self, frame: &InteractiveFrame) -> io::Result<()>;
}

struct CrosstermBackend<W> {
    writer: W,
    acknowledged: Option<ProjectedFrame>,
}

impl<W> CrosstermBackend<W> {
    const fn new(writer: W) -> Self {
        Self {
            writer,
            acknowledged: None,
        }
    }
}

impl<W: Write> TerminalBackend for CrosstermBackend<W> {
    fn enable_raw(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()
    }

    fn enter_alternate(&mut self) -> io::Result<()> {
        self.writer.execute(EnterAlternateScreen)?;
        Ok(())
    }

    fn enable_paste(&mut self) -> io::Result<()> {
        self.writer.execute(EnableBracketedPaste)?;
        Ok(())
    }

    fn enable_mouse(&mut self) -> io::Result<()> {
        self.writer
            .write_all(b"\x1b[?1000h\x1b[?1002h\x1b[?1006h")?;
        self.writer.flush()
    }

    fn enable_focus(&mut self) -> io::Result<()> {
        self.writer.execute(EnableFocusChange)?;
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.writer.execute(Hide)?;
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.writer.execute(Show)?;
        Ok(())
    }

    fn disable_focus(&mut self) -> io::Result<()> {
        self.writer.execute(DisableFocusChange)?;
        Ok(())
    }

    fn disable_mouse(&mut self) -> io::Result<()> {
        self.writer
            .write_all(b"\x1b[?1006l\x1b[?1002l\x1b[?1000l")?;
        self.writer.flush()
    }

    fn disable_paste(&mut self) -> io::Result<()> {
        self.writer.execute(DisableBracketedPaste)?;
        Ok(())
    }

    fn leave_alternate(&mut self) -> io::Result<()> {
        self.writer.execute(LeaveAlternateScreen)?;
        Ok(())
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()
    }

    fn write_frame(&mut self, frame: &InteractiveFrame) -> io::Result<()> {
        let next = projected_frame(frame)?;
        let bytes = differential_projection_bytes(self.acknowledged.as_ref(), &next)?;
        if let Err(error) = self
            .writer
            .write_all(&bytes)
            .and_then(|()| self.writer.flush())
        {
            self.acknowledged = None;
            return Err(error);
        }
        self.acknowledged = Some(next);
        Ok(())
    }
}

struct TerminalLease<B: TerminalBackend> {
    backend: B,
    raw: bool,
    alternate: bool,
    paste: bool,
    mouse: bool,
    focus: bool,
    cursor: bool,
    closed: bool,
}

impl<B: TerminalBackend> TerminalLease<B> {
    fn acquire(backend: B) -> Result<Self> {
        let mut lease = Self {
            backend,
            raw: false,
            alternate: false,
            paste: false,
            mouse: false,
            focus: false,
            cursor: false,
            closed: false,
        };
        lease.backend.enable_raw().map_err(|error| {
            LkError::new(
                ErrorCode::TerminalUnavailable,
                format!("cannot enable terminal raw mode: {error}"),
            )
        })?;
        lease.raw = true;
        lease.backend.enter_alternate().map_err(terminal_output)?;
        lease.alternate = true;
        lease.backend.enable_paste().map_err(terminal_output)?;
        lease.paste = true;
        lease.backend.enable_mouse().map_err(terminal_output)?;
        lease.mouse = true;
        lease.backend.enable_focus().map_err(terminal_output)?;
        lease.focus = true;
        lease.backend.hide_cursor().map_err(terminal_output)?;
        lease.cursor = true;
        Ok(lease)
    }

    fn write_frame(&mut self, frame: &InteractiveFrame) -> Result<()> {
        self.backend.write_frame(frame).map_err(terminal_output)
    }

    fn close(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let mut first = None;
        if self.cursor {
            self.cursor = false;
            record_cleanup(&mut first, self.backend.show_cursor());
        }
        if self.focus {
            self.focus = false;
            record_cleanup(&mut first, self.backend.disable_focus());
        }
        if self.mouse {
            self.mouse = false;
            record_cleanup(&mut first, self.backend.disable_mouse());
        }
        if self.paste {
            self.paste = false;
            record_cleanup(&mut first, self.backend.disable_paste());
        }
        if self.alternate {
            self.alternate = false;
            record_cleanup(&mut first, self.backend.leave_alternate());
        }
        if self.raw {
            self.raw = false;
            record_cleanup(&mut first, self.backend.disable_raw());
        }
        if let Some(error) = first {
            Err(LkError::new(
                ErrorCode::TerminalCleanup,
                format!("terminal cleanup failed after attempting every stage: {error}"),
            ))
        } else {
            Ok(())
        }
    }
}

impl<B: TerminalBackend> Drop for TerminalLease<B> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn record_cleanup(first: &mut Option<io::Error>, result: io::Result<()>) {
    if let Err(error) = result
        && first.is_none()
    {
        *first = Some(error);
    }
}

fn project_frame(writer: &mut impl Write, frame: &InteractiveFrame) -> io::Result<()> {
    let projected = projected_frame(frame)?;
    write_full_projection(writer, &projected)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectedFrame {
    rows: u16,
    columns: u16,
    body: Vec<Vec<u8>>,
    status: Vec<u8>,
    cursor: Option<(u16, u16)>,
    cursor_shape: InteractiveCursorShape,
}

fn projected_frame(frame: &InteractiveFrame) -> io::Result<ProjectedFrame> {
    let rows = u16::try_from(frame.rows)
        .map_err(|_| io::Error::other("frame row count does not fit terminal coordinates"))?;
    let columns = u16::try_from(frame.columns)
        .map_err(|_| io::Error::other("frame column count does not fit terminal coordinates"))?;
    if rows == 0 || columns == 0 {
        return Err(io::Error::other("frame dimensions must be nonzero"));
    }
    if frame.styles.len() != frame.scalars.len() {
        return Err(io::Error::other(
            "frame style count must equal frame scalar count",
        ));
    }
    if frame.styles.iter().any(|style| *style > 15) || frame.status_style > 15 {
        return Err(io::Error::other(
            "frame style is outside the closed palette",
        ));
    }
    let body_rows = rows.saturating_sub(1);
    let mut body = Vec::with_capacity(usize::from(body_rows));
    let mut start = 0_usize;
    for index in 0..=frame.scalars.len() {
        let line_end = index == frame.scalars.len() || frame.scalars[index] == u32::from('\n');
        if !line_end {
            continue;
        }
        if body.len() < usize::from(body_rows) {
            body.push(render_scalar_line(
                &frame.scalars[start..index],
                &frame.styles[start..index],
                columns,
            )?);
        }
        start = index.saturating_add(1);
        if body.len() == usize::from(body_rows) {
            break;
        }
    }
    body.resize_with(usize::from(body_rows), Vec::new);
    let status_scalars = frame.status.chars().map(u32::from).collect::<Vec<_>>();
    let status_styles = vec![frame.status_style; status_scalars.len()];
    let status = render_scalar_line(&status_scalars, &status_styles, columns)?;
    let cursor = if frame.cursor_visible && body_rows > 0 {
        Some(cursor_cells(frame, body_rows, columns))
    } else {
        None
    };
    Ok(ProjectedFrame {
        rows,
        columns,
        body,
        status,
        cursor,
        cursor_shape: frame.cursor_shape,
    })
}

fn render_scalar_line(scalars: &[u32], styles: &[u8], columns: u16) -> io::Result<Vec<u8>> {
    let estimated = scalars
        .len()
        .checked_mul(16)
        .and_then(|bytes| bytes.checked_add(64))
        .ok_or_else(|| io::Error::other("terminal row output estimate overflows"))?;
    let mut output = BoundedOutput::with_capacity(estimated.min(MAXIMUM_TERMINAL_FRAME_BYTES));
    let mut column = 0_u16;
    let mut current_style = 0_u8;
    for (scalar, style) in scalars.iter().zip(styles) {
        if column >= columns {
            break;
        }
        if *style != current_style {
            queue_palette_style(&mut output, *style)?;
            current_style = *style;
        }
        let value = char::from_u32(*scalar).unwrap_or(char::REPLACEMENT_CHARACTER);
        let value = if value == '\n' {
            char::REPLACEMENT_CHARACTER
        } else {
            value
        };
        write_cell_character(&mut output, value, &mut column, columns)?;
    }
    if current_style != 0 {
        queue_palette_style(&mut output, 0)?;
    }
    Ok(output.finish())
}

fn queue_palette_style(writer: &mut impl Write, style: u8) -> io::Result<()> {
    writer
        .queue(SetAttribute(Attribute::Reset))?
        .queue(ResetColor)?;
    let (foreground, background, bold) = match style {
        0 => return Ok(()),
        1 => (Color::Black, Some(Color::Cyan), true),
        2 => (Color::DarkGrey, None, false),
        3 => (Color::Yellow, None, true),
        4 => (Color::DarkGrey, None, false),
        5 => (Color::Cyan, None, true),
        6 => (Color::White, Some(Color::DarkBlue), false),
        7 => (Color::Black, Some(Color::Yellow), false),
        8 => (Color::Blue, None, true),
        9 => (Color::Grey, None, false),
        10 => (Color::Black, Some(Color::Green), true),
        11 => (Color::Black, Some(Color::Blue), true),
        12 => (Color::Black, Some(Color::Magenta), true),
        13 => (Color::White, Some(Color::DarkRed), true),
        14 => (Color::Black, Some(Color::DarkYellow), true),
        15 => (Color::Black, Some(Color::DarkCyan), true),
        _ => {
            return Err(io::Error::other(
                "frame style is outside the closed palette",
            ));
        }
    };
    writer.queue(SetForegroundColor(foreground))?;
    if let Some(background) = background {
        writer.queue(SetBackgroundColor(background))?;
    }
    if bold {
        writer.queue(SetAttribute(Attribute::Bold))?;
    }
    Ok(())
}

fn write_full_projection(writer: &mut impl Write, frame: &ProjectedFrame) -> io::Result<()> {
    writer.queue(Clear(ClearType::All))?;
    for (row, bytes) in frame.body.iter().enumerate() {
        writer.queue(MoveTo(0, u16::try_from(row).unwrap_or(u16::MAX)))?;
        writer.write_all(bytes)?;
    }
    writer.queue(MoveTo(0, frame.rows.saturating_sub(1)))?;
    writer.write_all(&frame.status)?;
    write_projected_cursor(writer, frame)
}

fn differential_projection_bytes(
    acknowledged: Option<&ProjectedFrame>,
    next: &ProjectedFrame,
) -> io::Result<Vec<u8>> {
    let mut output = BoundedOutput::with_capacity(4_096);
    let Some(previous) = acknowledged else {
        write_full_projection(&mut output, next)?;
        return Ok(output.finish());
    };
    if previous.rows != next.rows || previous.columns != next.columns {
        write_full_projection(&mut output, next)?;
        return Ok(output.finish());
    }
    let mut moved = false;
    for (row, (before, after)) in previous.body.iter().zip(&next.body).enumerate() {
        if before == after {
            continue;
        }
        output
            .queue(MoveTo(0, u16::try_from(row).unwrap_or(u16::MAX)))?
            .queue(Clear(ClearType::CurrentLine))?;
        output.write_all(after)?;
        moved = true;
    }
    if previous.status != next.status {
        output
            .queue(MoveTo(0, next.rows.saturating_sub(1)))?
            .queue(Clear(ClearType::CurrentLine))?;
        output.write_all(&next.status)?;
        moved = true;
    }
    if moved || previous.cursor != next.cursor || previous.cursor_shape != next.cursor_shape {
        write_projected_cursor(&mut output, next)?;
    }
    Ok(output.finish())
}

fn write_projected_cursor(writer: &mut impl Write, frame: &ProjectedFrame) -> io::Result<()> {
    match frame.cursor_shape {
        InteractiveCursorShape::Block => writer.queue(SetCursorStyle::SteadyBlock)?,
        InteractiveCursorShape::Bar => writer.queue(SetCursorStyle::SteadyBar)?,
        InteractiveCursorShape::Underline => writer.queue(SetCursorStyle::SteadyUnderScore)?,
    };
    if let Some((row, column)) = frame.cursor {
        writer.queue(MoveTo(column, row))?.queue(Show)?;
    } else {
        writer.queue(Hide)?;
    }
    Ok(())
}

fn write_cell_character(
    writer: &mut impl Write,
    value: char,
    column: &mut u16,
    columns: u16,
) -> io::Result<()> {
    if *column >= columns {
        return Ok(());
    }
    if value == '\t' {
        let spaces = 4_u16.saturating_sub(*column % 4).min(columns - *column);
        for _ in 0..spaces {
            writer.write_all(b" ")?;
        }
        *column = column.saturating_add(spaces);
        return Ok(());
    }
    let value = if value.is_control() {
        char::REPLACEMENT_CHARACTER
    } else {
        value
    };
    let width = value.width().unwrap_or(1);
    if width == 0 {
        if *column == 0 {
            writer.write_all("\u{25cc}".as_bytes())?;
            *column = column.saturating_add(1);
        }
        writer.write_all(value.encode_utf8(&mut [0; 4]).as_bytes())?;
        return Ok(());
    }
    let width = u16::try_from(width).unwrap_or(u16::MAX);
    if width > columns - *column {
        writer.write_all(
            char::REPLACEMENT_CHARACTER
                .encode_utf8(&mut [0; 4])
                .as_bytes(),
        )?;
        *column = column.saturating_add(1);
        return Ok(());
    }
    writer.write_all(value.encode_utf8(&mut [0; 4]).as_bytes())?;
    *column = column.saturating_add(width);
    Ok(())
}

fn cursor_cells(frame: &InteractiveFrame, body_rows: u16, columns: u16) -> (u16, u16) {
    let target_row = u16::try_from(frame.cursor_row)
        .unwrap_or(u16::MAX)
        .min(body_rows.saturating_sub(1));
    let target_scalar_column = usize::try_from(frame.cursor_column).unwrap_or(usize::MAX);
    let mut row = 0_u16;
    let mut scalar_column = 0_usize;
    let mut cell_column = 0_u16;
    for scalar in &frame.scalars {
        if row > target_row || (row == target_row && scalar_column >= target_scalar_column) {
            break;
        }
        let value = char::from_u32(*scalar).unwrap_or(char::REPLACEMENT_CHARACTER);
        if value == '\n' {
            if row == target_row {
                break;
            }
            row = row.saturating_add(1);
            scalar_column = 0;
            cell_column = 0;
            continue;
        }
        if row == target_row {
            let width = if value == '\t' {
                usize::from(4_u16.saturating_sub(cell_column % 4))
            } else if value.is_control() {
                1
            } else {
                value
                    .width()
                    .unwrap_or(1)
                    .max(usize::from(cell_column == 0))
            };
            cell_column = cell_column
                .saturating_add(u16::try_from(width).unwrap_or(u16::MAX))
                .min(columns.saturating_sub(1));
            scalar_column = scalar_column.saturating_add(1);
        }
    }
    (target_row, cell_column.min(columns.saturating_sub(1)))
}

struct BoundedOutput {
    bytes: Vec<u8>,
}

impl BoundedOutput {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("terminal frame byte count overflows"))?;
        if next > MAXIMUM_TERMINAL_FRAME_BYTES {
            return Err(io::Error::other(
                "terminal frame exceeds output byte policy",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct SignalFlag {
    raised: Arc<AtomicBool>,
    registrations: Vec<SigId>,
}

impl SignalFlag {
    fn register() -> Result<Self> {
        let raised = Arc::new(AtomicBool::new(false));
        let mut registrations = Vec::with_capacity(4);
        for signal in [SIGINT, SIGTERM, SIGQUIT, SIGHUP] {
            registrations.push(
                signal_hook::flag::register(signal, Arc::clone(&raised)).map_err(|error| {
                    LkError::new(
                        ErrorCode::TerminalUnavailable,
                        format!("cannot register terminal cleanup signal {signal}: {error}"),
                    )
                })?,
            );
        }
        Ok(Self {
            raised,
            registrations,
        })
    }

    fn raised(&self) -> bool {
        self.raised.load(Ordering::Relaxed)
    }
}

impl Drop for SignalFlag {
    fn drop(&mut self) {
        for registration in self.registrations.drain(..) {
            signal_hook::low_level::unregister(registration);
        }
    }
}

fn terminal_output(error: io::Error) -> LkError {
    LkError::new(
        ErrorCode::TerminalOutput,
        format!("terminal output failed: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventState, MouseEvent};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct FakeBackend {
        calls: Rc<RefCell<Vec<&'static str>>>,
        fail_at: Option<usize>,
    }

    impl FakeBackend {
        fn call(&mut self, name: &'static str) -> io::Result<()> {
            let mut calls = self.calls.borrow_mut();
            calls.push(name);
            if self.fail_at == Some(calls.len()) {
                Err(io::Error::other("injected terminal failure"))
            } else {
                Ok(())
            }
        }
    }

    impl TerminalBackend for FakeBackend {
        fn enable_raw(&mut self) -> io::Result<()> {
            self.call("enable_raw")
        }
        fn enter_alternate(&mut self) -> io::Result<()> {
            self.call("enter_alternate")
        }
        fn enable_paste(&mut self) -> io::Result<()> {
            self.call("enable_paste")
        }
        fn enable_mouse(&mut self) -> io::Result<()> {
            self.call("enable_mouse")
        }
        fn enable_focus(&mut self) -> io::Result<()> {
            self.call("enable_focus")
        }
        fn hide_cursor(&mut self) -> io::Result<()> {
            self.call("hide_cursor")
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            self.call("show_cursor")
        }
        fn disable_focus(&mut self) -> io::Result<()> {
            self.call("disable_focus")
        }
        fn disable_mouse(&mut self) -> io::Result<()> {
            self.call("disable_mouse")
        }
        fn disable_paste(&mut self) -> io::Result<()> {
            self.call("disable_paste")
        }
        fn leave_alternate(&mut self) -> io::Result<()> {
            self.call("leave_alternate")
        }
        fn disable_raw(&mut self) -> io::Result<()> {
            self.call("disable_raw")
        }
        fn write_frame(&mut self, _frame: &InteractiveFrame) -> io::Result<()> {
            self.call("frame")
        }
    }

    fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn acquisition_failure_cleans_every_completed_stage_in_reverse_order() {
        for fail_at in 1..=6 {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let backend = FakeBackend {
                calls: Rc::clone(&calls),
                fail_at: Some(fail_at),
            };
            assert!(TerminalLease::acquire(backend).is_err());
            let calls = calls.borrow();
            if fail_at > 1 {
                assert_eq!(calls.last(), Some(&"disable_raw"));
            }
        }
    }

    #[test]
    fn cleanup_attempts_every_stage_and_is_idempotent() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let backend = FakeBackend {
            calls: Rc::clone(&calls),
            fail_at: None,
        };
        let mut lease = TerminalLease::acquire(backend).expect("acquire");
        lease.close().expect("cleanup");
        lease.close().expect("idempotent cleanup");
        assert_eq!(
            calls.borrow().as_slice(),
            [
                "enable_raw",
                "enter_alternate",
                "enable_paste",
                "enable_mouse",
                "enable_focus",
                "hide_cursor",
                "show_cursor",
                "disable_focus",
                "disable_mouse",
                "disable_paste",
                "leave_alternate",
                "disable_raw",
            ]
        );
    }

    #[test]
    fn output_failure_and_drop_cleanup_attempt_every_acquired_stage() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let backend = FakeBackend {
            calls: Rc::clone(&calls),
            fail_at: Some(7),
        };
        let mut lease = TerminalLease::acquire(backend).expect("acquire");
        assert_eq!(
            lease
                .write_frame(&InteractiveFrame {
                    rows: 1,
                    columns: 1,
                    scalars: Vec::new(),
                    styles: Vec::new(),
                    cursor_row: 0,
                    cursor_column: 0,
                    cursor_visible: false,
                    cursor_shape: InteractiveCursorShape::Block,
                    status: String::new(),
                    status_style: 0,
                })
                .expect_err("injected output failure")
                .code,
            ErrorCode::TerminalOutput
        );
        drop(lease);
        assert_eq!(
            calls.borrow().as_slice(),
            [
                "enable_raw",
                "enter_alternate",
                "enable_paste",
                "enable_mouse",
                "enable_focus",
                "hide_cursor",
                "frame",
                "show_cursor",
                "disable_focus",
                "disable_mouse",
                "disable_paste",
                "leave_alternate",
                "disable_raw",
            ]
        );
    }

    #[test]
    fn unwinding_drops_the_lease_and_attempts_every_cleanup_stage() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let backend = FakeBackend {
            calls: Rc::clone(&calls),
            fail_at: None,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _lease = TerminalLease::acquire(backend).expect("acquire");
            std::panic::resume_unwind(Box::new("injected unwind"));
        }));
        assert!(result.is_err());
        assert_eq!(
            calls.borrow().as_slice(),
            [
                "enable_raw",
                "enter_alternate",
                "enable_paste",
                "enable_mouse",
                "enable_focus",
                "hide_cursor",
                "show_cursor",
                "disable_focus",
                "disable_mouse",
                "disable_paste",
                "leave_alternate",
                "disable_raw",
            ]
        );
    }

    #[test]
    fn unix_terminal_end_conditions_have_one_eof_classification() {
        assert!(terminal_input_ended(&io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "injected EOF"
        )));
        assert!(terminal_input_ended(&io::Error::from_raw_os_error(
            rustix::io::Errno::IO.raw_os_error()
        )));
        assert!(!terminal_input_ended(&io::Error::other(
            "unrelated failure"
        )));
    }

    #[test]
    fn disconnected_descriptor_is_a_closed_lifecycle_observation() {
        use std::os::unix::net::UnixStream;

        let (input, peer) = UnixStream::pair().expect("socket pair");
        assert!(!terminal_input_disconnected(&input).expect("connected descriptor"));
        drop(peer);
        assert!(terminal_input_disconnected(&input).expect("disconnected descriptor"));
    }

    #[test]
    fn host_events_are_closed_bounded_and_explicit() {
        let event = adapt_terminal_event(key(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            KeyEventKind::Repeat,
        ))
        .expect("decode")
        .expect("retained key");
        let InteractiveEvent::Key(event) = event else {
            panic!("key expected")
        };
        assert_eq!(event.code, InteractiveKeyCode::Character(u32::from('x')));
        assert!(event.control);
        assert!(event.shift);
        assert!(event.repeat);
        assert!(
            adapt_terminal_event(key(KeyCode::F(1), KeyModifiers::NONE, KeyEventKind::Press))
                .expect("unsupported key")
                .is_none()
        );
        assert!(adapt_terminal_event(Event::Resize(0, u16::MAX)).is_ok());

        let mouse = adapt_terminal_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 19,
            row: 7,
            modifiers: KeyModifiers::CONTROL,
        }))
        .expect("mouse decode")
        .expect("retained mouse");
        assert_eq!(
            mouse,
            InteractiveEvent::Mouse(InteractiveMouseEvent {
                button: InteractiveMouseButton::Primary,
                kind: InteractiveMouseKind::Drag,
                row: 7,
                column: 19,
                control: true,
                alt: false,
                shift: false,
            })
        );
        assert!(
            adapt_terminal_event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Moved,
                column: 1,
                row: 1,
                modifiers: KeyModifiers::NONE,
            }))
            .expect("passive motion decode")
            .is_none()
        );
        assert_eq!(
            adapt_terminal_event(Event::FocusLost).expect("focus decode"),
            Some(InteractiveEvent::FocusLost)
        );
    }

    #[test]
    fn full_frame_projection_clips_wide_and_escapes_control_text() {
        let frame = InteractiveFrame {
            rows: 3,
            columns: 4,
            scalars: vec![
                u32::from('a'),
                0x1b,
                u32::from('界'),
                u32::from('\n'),
                u32::from('z'),
            ],
            styles: vec![0; 5],
            cursor_row: 0,
            cursor_column: 3,
            cursor_visible: true,
            cursor_shape: InteractiveCursorShape::Block,
            status: "ok\u{1b}[31m".into(),
            status_style: 0,
        };
        let projected = terminal_frame_bytes(&frame).expect("frame projection");
        assert!(!projected.windows(5).any(|window| window == b"\x1b[31m"));
        assert!(
            projected
                .windows(3)
                .any(|window| window == "\u{fffd}".as_bytes())
        );
        assert!(projected.len() <= MAXIMUM_TERMINAL_FRAME_BYTES);
    }

    #[test]
    fn row_differential_is_exact_bounded_and_materially_smaller() {
        let mut scalars = Vec::new();
        for row in 0..39_u32 {
            scalars.extend(std::iter::repeat_n(u32::from('a') + row % 20, 120));
            if row != 38 {
                scalars.push(u32::from('\n'));
            }
        }
        let mut frame = InteractiveFrame {
            rows: 40,
            columns: 120,
            styles: vec![0; scalars.len()],
            scalars,
            cursor_row: 20,
            cursor_column: 60,
            cursor_visible: true,
            cursor_shape: InteractiveCursorShape::Block,
            status: "NORMAL  example.txt".into(),
            status_style: 10,
        };
        let first = projected_frame(&frame).expect("first projection");
        let full = terminal_frame_bytes(&frame).expect("full frame");
        assert_eq!(
            differential_projection_bytes(None, &first).expect("cache miss"),
            full
        );
        assert!(
            differential_projection_bytes(Some(&first), &first)
                .expect("unchanged frame")
                .is_empty()
        );

        frame.scalars[120 * 20 + 20] = u32::from('Z');
        frame.styles[120 * 20 + 20] = 7;
        let second = projected_frame(&frame).expect("second projection");
        let delta = differential_projection_bytes(Some(&first), &second).expect("row delta");
        assert!(delta.len() * 5 < full.len() * 4);
        eprintln!(
            "terminal-row-differential full_bytes={} delta_bytes={} reduction_percent_x100={}",
            full.len(),
            delta.len(),
            (full.len() - delta.len()) * 10_000 / full.len()
        );
        assert_eq!(
            differential_projection_bytes(None, &second).expect("cache rebuild"),
            terminal_frame_bytes(&frame).expect("second full frame")
        );
    }

    #[test]
    fn bounded_worker_rejects_a_second_job_and_correlates_the_result() {
        let mut worker = ActionWorker::start(|_| {
            Ok(InteractiveActionOutcome {
                job_id: 0,
                class: crate::application::InteractiveActionOutcomeClass::Succeeded,
                message: "done".into(),
                content: String::new(),
                token: String::new(),
            })
        })
        .expect("worker");
        worker
            .submit(crate::application::InteractiveActionRequest {
                job_id: 41,
                action: InteractiveAction::ProjectOrient,
            })
            .expect("first job");
        assert_eq!(
            worker
                .submit(crate::application::InteractiveActionRequest {
                    job_id: 42,
                    action: InteractiveAction::ProjectOrient,
                })
                .expect_err("second job must reject")
                .code,
            ErrorCode::AuthorityBusy
        );
        let result = loop {
            if let Some(result) = worker.try_result().expect("worker result") {
                break result;
            }
            thread::yield_now();
        };
        assert_eq!(result.job_id, 41);
        worker.shutdown(true);
    }
}
