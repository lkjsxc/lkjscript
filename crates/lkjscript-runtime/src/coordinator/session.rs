use std::collections::BTreeMap;

use lkjscript_host::{LocalPrincipal, MonotonicTime};

use crate::{ControlFailure, ControlSuccess, ControlledSession, SessionBackend};

const MAX_SESSIONS: usize = 64;
const SESSION_LEASE_NANOS: u64 = 10_000_000_000;

pub(super) struct SessionRegistry {
    sessions: BTreeMap<u64, ControlledSession>,
    next: u64,
}

impl SessionRegistry {
    pub(super) fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            next: 1,
        }
    }

    pub(super) fn register(
        &mut self,
        broker_instance: [u8; 32],
        backend: SessionBackend,
        principal: LocalPrincipal,
        now: MonotonicTime,
    ) -> Result<ControlSuccess, ControlFailure> {
        self.reap(now);
        if principal.process == 0 || broker_instance == [0; 32] {
            return Err(ControlFailure::Malformed);
        }
        if self.sessions.len() == MAX_SESSIONS {
            return Err(ControlFailure::Rejected("session bound reached".into()));
        }
        if self
            .sessions
            .values()
            .any(|session| session.broker_instance == broker_instance)
        {
            return Err(ControlFailure::Rejected(
                "broker instance is already registered".into(),
            ));
        }
        let session = self.next;
        self.next = session.checked_add(1).ok_or(ControlFailure::Internal)?;
        let record = ControlledSession {
            session,
            broker_instance,
            process: principal.process,
            user: principal.user,
            group: principal.group,
            backend,
            lease_deadline: deadline(now)?,
        };
        self.sessions.insert(session, record.clone());
        Ok(ControlSuccess::Session(record))
    }

    pub(super) fn heartbeat(
        &mut self,
        session: u64,
        principal: LocalPrincipal,
        now: MonotonicTime,
    ) -> Result<ControlSuccess, ControlFailure> {
        self.reap(now);
        let record = self
            .sessions
            .get_mut(&session)
            .ok_or(ControlFailure::NotFound)?;
        authorize(record, principal)?;
        record.lease_deadline = deadline(now)?;
        Ok(ControlSuccess::Session(record.clone()))
    }

    pub(super) fn unregister(
        &mut self,
        session: u64,
        principal: LocalPrincipal,
        now: MonotonicTime,
    ) -> Result<ControlSuccess, ControlFailure> {
        self.reap(now);
        let record = self
            .sessions
            .get(&session)
            .ok_or(ControlFailure::NotFound)?;
        authorize(record, principal)?;
        self.sessions.remove(&session);
        Ok(ControlSuccess::SessionUnregistered { session })
    }

    pub(super) fn list(&mut self, now: MonotonicTime) -> ControlSuccess {
        self.reap(now);
        ControlSuccess::Sessions(self.sessions.values().cloned().collect())
    }

    pub(super) fn live_count(&self, now: MonotonicTime) -> usize {
        self.sessions
            .values()
            .filter(|session| session.lease_deadline > now.0)
            .count()
    }

    pub(super) fn clear(&mut self) {
        self.sessions.clear();
    }

    fn reap(&mut self, now: MonotonicTime) {
        self.sessions
            .retain(|_, session| session.lease_deadline > now.0);
    }
}

fn authorize(session: &ControlledSession, principal: LocalPrincipal) -> Result<(), ControlFailure> {
    if session.process == principal.process
        && session.user == principal.user
        && session.group == principal.group
    {
        Ok(())
    } else {
        Err(ControlFailure::Unauthorized)
    }
}

fn deadline(now: MonotonicTime) -> Result<u64, ControlFailure> {
    now.0
        .checked_add(SESSION_LEASE_NANOS)
        .ok_or(ControlFailure::Internal)
}

#[cfg(test)]
mod tests;
