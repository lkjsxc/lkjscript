//! Narrow native terminal adaptation for interactive application artifacts.
//!
//! This module owns raw-mode lifecycle, bounded host-event decoding, safe full-frame projection,
//! and cleanup. It does not own key policy, editor state, commands, or frame meaning.

use crate::application::{
    InteractiveAction, InteractiveActionOutcome, InteractiveEvent, InteractiveFrame,
    InteractiveKeyCode, InteractiveKeyEvent, MAXIMUM_INTERACTIVE_COLUMNS,
    MAXIMUM_INTERACTIVE_PASTE_SCALARS, MAXIMUM_INTERACTIVE_ROWS, prepare_interactive,
};
use crate::error::{ErrorCode, LkError, Result};
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, QueueableCommand};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fd::AsFd;
use serde::Serialize;
use signal_hook::SigId;
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use std::io::{self, IsTerminal, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use unicode_width::UnicodeWidthChar;

pub const TERMINAL_CONTRACT_VERSION: u16 = 3;
pub const MAXIMUM_TERMINAL_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_TERMINAL_ACTIONS: u64 = 10_000;
pub const TERMINAL_POLL_MILLISECONDS: u64 = 25;

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
        Event::FocusGained | Event::FocusLost | Event::Mouse(_) => Ok(None),
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
    mut handle_action: impl FnMut(InteractiveAction) -> Result<InteractiveActionOutcome>,
) -> Result<TerminalRunReceipt> {
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
    let backend = CrosstermBackend::new(stdout.lock());
    let mut terminal_session = TerminalLease::acquire(backend)?;
    let mut events = 0_u64;
    let mut actions = 0_u64;
    let mut frames = 0_u64;
    let outcome = (|| {
        terminal_session.write_frame(&initial.frame)?;
        frames = frames.saturating_add(1);
        loop {
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
            let mut step = application_session.step(event)?;
            events = events.saturating_add(1);
            let mut requested_exit = false;
            loop {
                terminal_session.write_frame(&step.frame)?;
                frames = frames.saturating_add(1);
                requested_exit |= step.exit;
                let Some(action) = step.action else {
                    break;
                };
                if actions >= MAXIMUM_TERMINAL_ACTIONS {
                    return Err(LkError::new(
                        ErrorCode::PolicyExceeded,
                        "terminal host-action count exceeds policy",
                    ));
                }
                actions = actions.saturating_add(1);
                step = application_session.resume(handle_action(action)?)?;
            }
            if requested_exit {
                return Ok(TerminalExitReason::Application);
            }
        }
    })();
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
    fn hide_cursor(&mut self) -> io::Result<()>;
    fn show_cursor(&mut self) -> io::Result<()>;
    fn disable_paste(&mut self) -> io::Result<()>;
    fn leave_alternate(&mut self) -> io::Result<()>;
    fn disable_raw(&mut self) -> io::Result<()>;
    fn write_frame(&mut self, frame: &InteractiveFrame) -> io::Result<()>;
}

struct CrosstermBackend<W> {
    writer: W,
}

impl<W> CrosstermBackend<W> {
    const fn new(writer: W) -> Self {
        Self { writer }
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

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.writer.execute(Hide)?;
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.writer.execute(Show)?;
        Ok(())
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
        project_frame(&mut self.writer, frame)?;
        self.writer.flush()
    }
}

struct TerminalLease<B: TerminalBackend> {
    backend: B,
    raw: bool,
    alternate: bool,
    paste: bool,
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
    let rows = u16::try_from(frame.rows)
        .map_err(|_| io::Error::other("frame row count does not fit terminal coordinates"))?;
    let columns = u16::try_from(frame.columns)
        .map_err(|_| io::Error::other("frame column count does not fit terminal coordinates"))?;
    if rows == 0 || columns == 0 {
        return Err(io::Error::other("frame dimensions must be nonzero"));
    }
    writer.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
    let body_rows = rows.saturating_sub(1);
    write_scalars(writer, &frame.scalars, body_rows, columns)?;
    writer.queue(MoveTo(0, rows.saturating_sub(1)))?;
    write_text(writer, &frame.status, columns)?;
    if frame.cursor_visible && body_rows > 0 {
        let (cursor_row, cursor_column) = cursor_cells(frame, body_rows, columns);
        writer
            .queue(MoveTo(cursor_column, cursor_row))?
            .queue(Show)?;
    } else {
        writer.queue(Hide)?;
    }
    Ok(())
}

fn write_scalars(
    writer: &mut impl Write,
    scalars: &[u32],
    rows: u16,
    columns: u16,
) -> io::Result<()> {
    let mut row = 0_u16;
    let mut column = 0_u16;
    for scalar in scalars {
        if row >= rows {
            break;
        }
        let value = char::from_u32(*scalar).unwrap_or(char::REPLACEMENT_CHARACTER);
        if value == '\n' {
            row = row.saturating_add(1);
            column = 0;
            if row < rows {
                writer.queue(MoveTo(0, row))?;
            }
            continue;
        }
        write_cell_character(writer, value, &mut column, columns)?;
    }
    Ok(())
}

fn write_text(writer: &mut impl Write, text: &str, columns: u16) -> io::Result<()> {
    let mut column = 0_u16;
    for value in text.chars() {
        let value = if value == '\n' {
            char::REPLACEMENT_CHARACTER
        } else {
            value
        };
        write_cell_character(writer, value, &mut column, columns)?;
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
    use crossterm::event::{KeyEvent, KeyEventState};
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
        fn hide_cursor(&mut self) -> io::Result<()> {
            self.call("hide_cursor")
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            self.call("show_cursor")
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
        for fail_at in 1..=4 {
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
                "hide_cursor",
                "show_cursor",
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
            fail_at: Some(5),
        };
        let mut lease = TerminalLease::acquire(backend).expect("acquire");
        assert_eq!(
            lease
                .write_frame(&InteractiveFrame {
                    rows: 1,
                    columns: 1,
                    scalars: Vec::new(),
                    cursor_row: 0,
                    cursor_column: 0,
                    cursor_visible: false,
                    status: String::new(),
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
                "hide_cursor",
                "frame",
                "show_cursor",
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
                "hide_cursor",
                "show_cursor",
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
            cursor_row: 0,
            cursor_column: 3,
            cursor_visible: true,
            status: "ok\u{1b}[31m".into(),
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
}
