//! Terminal mode and wait/poll host effects (Linux/Docker first).

use std::io::{self, Read};
use std::thread;
use std::time::{Duration, Instant};

use lkjscript2026_core::{Error, Result, Value};

#[cfg(unix)]
mod tty {
    use std::io::IsTerminal;
    use std::sync::Mutex;

    use rustix::event::{poll, PollFd, PollFlags};
    use rustix::stdio::stdin;
    use rustix::termios::{self, OptionalActions, Termios};

    static STATE: Mutex<Option<Termios>> = Mutex::new(None);

    pub fn term_raw() -> lkjscript2026_core::Result<lkjscript2026_core::Value> {
        if !std::io::stdin().is_terminal() {
            return Ok(lkjscript2026_core::Value::NIL);
        }
        let fd = stdin();
        let mut guard = STATE
            .lock()
            .map_err(|_| lkjscript2026_core::Error::msg("term lock"))?;
        if guard.is_none() {
            let saved = termios::tcgetattr(fd)
                .map_err(|e| lkjscript2026_core::Error::msg(format!("tcgetattr: {e}")))?;
            *guard = Some(saved.clone());
            let mut raw = saved;
            raw.make_raw();
            termios::tcsetattr(fd, OptionalActions::Now, &raw)
                .map_err(|e| lkjscript2026_core::Error::msg(format!("tcsetattr: {e}")))?;
        }
        Ok(lkjscript2026_core::Value::NIL)
    }

    pub fn term_cooked() -> lkjscript2026_core::Result<lkjscript2026_core::Value> {
        let fd = stdin();
        let mut guard = STATE
            .lock()
            .map_err(|_| lkjscript2026_core::Error::msg("term lock"))?;
        if let Some(saved) = guard.take() {
            let _ = termios::tcsetattr(fd, OptionalActions::Now, &saved);
        }
        Ok(lkjscript2026_core::Value::NIL)
    }

    pub fn restore_if_needed() {
        let _ = term_cooked();
    }

    pub fn poll_ready() -> lkjscript2026_core::Result<bool> {
        let fd = stdin();
        let mut pfd = [PollFd::new(&fd, PollFlags::IN | PollFlags::HUP)];
        let n = poll(&mut pfd, 0i32).map_err(|e| lkjscript2026_core::Error::msg(format!("poll: {e}")))?;
        if n <= 0 {
            return Ok(false);
        }
        let ev = pfd[0].revents();
        Ok(ev.contains(PollFlags::IN) || ev.contains(PollFlags::HUP) || ev.contains(PollFlags::ERR))
    }
}

#[cfg(not(unix))]
mod tty {
    pub fn term_raw() -> lkjscript2026_core::Result<lkjscript2026_core::Value> {
        Ok(lkjscript2026_core::Value::NIL)
    }
    pub fn term_cooked() -> lkjscript2026_core::Result<lkjscript2026_core::Value> {
        Ok(lkjscript2026_core::Value::NIL)
    }
    pub fn restore_if_needed() {}
    pub fn poll_ready() -> lkjscript2026_core::Result<bool> {
        Ok(true)
    }
}

static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

pub fn term_raw() -> Result<Value> {
    tty::term_raw()
}

pub fn term_cooked() -> Result<Value> {
    tty::term_cooked()
}

pub fn restore_tty() {
    tty::restore_if_needed();
}

pub fn now_ms() -> Result<Value> {
    let ms = start().elapsed().as_millis() as i64;
    Ok(Value::from_int(ms))
}

pub fn wait_ms(v: Value) -> Result<Value> {
    let n = v
        .as_int()
        .ok_or_else(|| Error::msg("wait-ms expects int"))?;
    let n = n.max(0) as u64;
    thread::sleep(Duration::from_millis(n));
    Ok(Value::NIL)
}

pub fn poll_byte() -> Result<Value> {
    if !tty::poll_ready()? {
        return Ok(Value::from_int(-1));
    }
    let mut buf = [0u8; 1];
    match io::stdin().read(&mut buf) {
        Ok(0) => Ok(Value::from_int(-2)),
        Ok(_) => Ok(Value::from_int(buf[0] as i64)),
        Err(e) => Err(Error::msg(format!("poll-byte: {e}"))),
    }
}
