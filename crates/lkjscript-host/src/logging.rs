use std::io::{self, Write};

use crate::{HostError, HostResult, WallTime};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogRecord<'a> {
    pub level: LogLevel,
    pub target: &'a str,
    pub message: &'a str,
    pub wall_time: Option<WallTime>,
}

pub trait Logger: Send + Sync {
    fn log(&self, record: LogRecord<'_>) -> HostResult<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PortableLogger;

impl Logger for PortableLogger {
    fn log(&self, record: LogRecord<'_>) -> HostResult<()> {
        let mut stderr = io::stderr().lock();
        writeln!(
            stderr,
            "{:?} {}: {}",
            record.level, record.target, record.message
        )
        .map_err(|error| HostError::from_io("stderr", error))
    }
}
