use lkjscript_host::{DurableStorage, LocalPrincipal};

use crate::{ControlFailure, ControlIdentity, ControlOperation, ControlRequest, ControlSuccess};

use super::MachineCoordinator;

impl<S: DurableStorage> MachineCoordinator<S> {
    pub fn handle_control(
        &mut self,
        request: &ControlRequest,
        principal: LocalPrincipal,
    ) -> Result<ControlSuccess, ControlFailure> {
        match &request.operation {
            ControlOperation::Describe => self.description(),
            ControlOperation::Status => self.control_status(),
            ControlOperation::Shutdown => Ok(ControlSuccess::ShutdownAccepted),
            ControlOperation::SessionRegister {
                broker_instance,
                backend,
            } => self.sessions.register(
                *broker_instance,
                *backend,
                principal,
                self.clock.monotonic_time(),
            ),
            ControlOperation::SessionList => Ok(self.sessions.list(self.clock.monotonic_time())),
            ControlOperation::SessionHeartbeat { session } => {
                self.sessions
                    .heartbeat(*session, principal, self.clock.monotonic_time())
            }
            ControlOperation::SessionUnregister { session } => {
                self.sessions
                    .unregister(*session, principal, self.clock.monotonic_time())
            }
            operation => self.application_control(operation),
        }
    }

    fn description(&self) -> Result<ControlSuccess, ControlFailure> {
        let identity = ControlIdentity::current().map_err(|_| ControlFailure::Internal)?;
        Ok(ControlSuccess::Description {
            platform_revision: identity.platform_revision,
            contract_digest: identity.contract_digest,
            product: "lkjscript runtime".to_string(),
        })
    }

    fn control_status(&self) -> Result<ControlSuccess, ControlFailure> {
        let status = self.status().map_err(|_| ControlFailure::Internal)?;
        Ok(ControlSuccess::Status {
            coordinator: status.identity.get(),
            clean_shutdown: status.previous_shutdown_clean,
            control_sequence: status.control_sequence,
            applications: u32::try_from(status.applications)
                .map_err(|_| ControlFailure::Internal)?,
            sessions: u16::try_from(status.sessions).map_err(|_| ControlFailure::Internal)?,
        })
    }
}
