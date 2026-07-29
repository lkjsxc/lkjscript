use crate::ResourceKind;

use super::CleanupFailureLimits;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupPhase {
    Ordinary,
    Emergency,
    RuntimeTeardown,
}

impl CleanupPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::Emergency => "emergency",
            Self::RuntimeTeardown => "runtime-teardown",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupSubject {
    UniqueStorage,
    Resource(ResourceKind),
    ResourceTable,
    BorrowedResource(ResourceKind),
    Terminal,
    StandardOutput,
    EvaluatorProvider,
}

impl CleanupSubject {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UniqueStorage => "unique-storage",
            Self::Resource(kind) | Self::BorrowedResource(kind) => kind.as_str(),
            Self::ResourceTable => "resource-table",
            Self::Terminal => "terminal",
            Self::StandardOutput => "standard-output",
            Self::EvaluatorProvider => "evaluator-provider",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupFailure {
    phase: CleanupPhase,
    subject: CleanupSubject,
    message: String,
    omitted_message_bytes: usize,
}

impl CleanupFailure {
    pub const fn phase(&self) -> CleanupPhase {
        self.phase
    }

    pub const fn subject(&self) -> CleanupSubject {
        self.subject
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn omitted_message_bytes(&self) -> usize {
        self.omitted_message_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupFailures {
    limits: CleanupFailureLimits,
    retained: Vec<CleanupFailure>,
    retained_message_bytes: usize,
    omitted_message_bytes: usize,
    omitted_failures: usize,
}

impl Default for CleanupFailures {
    fn default() -> Self {
        Self::new(CleanupFailureLimits::default())
    }
}

impl CleanupFailures {
    pub const fn new(limits: CleanupFailureLimits) -> Self {
        Self {
            limits,
            retained: Vec::new(),
            retained_message_bytes: 0,
            omitted_message_bytes: 0,
            omitted_failures: 0,
        }
    }

    pub fn push(&mut self, phase: CleanupPhase, subject: CleanupSubject, message: impl AsRef<str>) {
        let message = message.as_ref();
        if self.retained.len() >= self.limits.max_failures {
            self.omitted_failures = self.omitted_failures.saturating_add(1);
            self.omitted_message_bytes = self.omitted_message_bytes.saturating_add(message.len());
            return;
        }
        let remaining = self
            .limits
            .max_message_bytes
            .saturating_sub(self.retained_message_bytes);
        let retained_len = utf8_prefix_len(message, remaining);
        let omitted = message.len().saturating_sub(retained_len);
        self.retained.push(CleanupFailure {
            phase,
            subject,
            message: message[..retained_len].to_owned(),
            omitted_message_bytes: omitted,
        });
        self.retained_message_bytes = self.retained_message_bytes.saturating_add(retained_len);
        self.omitted_message_bytes = self.omitted_message_bytes.saturating_add(omitted);
    }

    pub fn append(&mut self, other: Self) {
        let Self {
            retained,
            omitted_message_bytes,
            omitted_failures,
            ..
        } = other;
        for failure in retained {
            self.push(failure.phase, failure.subject, failure.message);
        }
        self.omitted_message_bytes = self
            .omitted_message_bytes
            .saturating_add(omitted_message_bytes);
        self.omitted_failures = self.omitted_failures.saturating_add(omitted_failures);
    }

    pub fn retained(&self) -> &[CleanupFailure] {
        &self.retained
    }

    pub const fn retained_message_bytes(&self) -> usize {
        self.retained_message_bytes
    }

    pub const fn omitted_message_bytes(&self) -> usize {
        self.omitted_message_bytes
    }

    pub const fn omitted_failures(&self) -> usize {
        self.omitted_failures
    }

    pub const fn is_empty(&self) -> bool {
        self.retained.is_empty() && self.omitted_failures == 0
    }
}

include!("cleanup/wire.rs");

fn utf8_prefix_len(message: &str, maximum: usize) -> usize {
    let mut length = message.len().min(maximum);
    while !message.is_char_boundary(length) {
        length -= 1;
    }
    length
}
