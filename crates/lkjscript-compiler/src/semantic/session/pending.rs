use super::schema::PinnedSession;
use super::SemanticSession;

pub(super) struct PendingExecution {
    pub outcome: crate::semantic::engine::EngineOutcome,
    pub rollback: Option<(PinnedSession, u64)>,
}

impl PendingExecution {
    pub(super) fn new(
        outcome: crate::semantic::engine::EngineOutcome,
        rollback: Option<(PinnedSession, u64)>,
    ) -> Self {
        Self { outcome, rollback }
    }
}

impl SemanticSession {
    pub(super) fn publish_pending(&mut self) -> Result<(), crate::semantic::schema::ProtocolError> {
        let Some(mut pending) = self.pending.take() else {
            return Ok(());
        };
        if let Err(error) = crate::semantic::publish_outcome(&mut pending.outcome) {
            self.restore_pending(pending.rollback);
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn discard_pending(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.restore_pending(pending.rollback);
        }
    }

    fn restore_pending(&mut self, rollback: Option<(PinnedSession, u64)>) {
        if let Some((pinned, revision)) = rollback {
            self.pinned = Some(pinned);
            self.revision = revision;
        }
    }
}
