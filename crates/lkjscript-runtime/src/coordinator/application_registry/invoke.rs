impl<S: DurableStorage> MachineCoordinator<S> {
    fn invoke_application(
    &mut self,
    id: u64,
    arguments: Vec<String>,
) -> Result<crate::ControlSuccess, crate::ControlFailure> {
    let incarnation = self.incarnation(id)?;
    let stdio = self
        .applications
        .get(&id)
        .ok_or(crate::ControlFailure::NotFound)?
        .stdio
        .clone();
    stdio
        .drain_output()
        .map_err(|error| rejected(&error.to_string()))?;
    let outcome = self
        .runtime
        .invoke(incarnation, arguments)
        .map_err(|error| rejected(&error.to_string()))?
        .outcome;
    let (output, _) = stdio
        .drain_output()
        .map_err(|error| rejected(&error.to_string()))?;
    if output.len() > 16 * 1024 {
        return Err(rejected("application output exceeds local-control bound"));
    }
    Ok(crate::ControlSuccess::ApplicationInvoked {
        application: id,
        outcome,
        output,
    })
}

fn application_success(
    &self,
    id: u64,
) -> Result<crate::ControlSuccess, crate::ControlFailure> {
    self.application_view(id)
        .map(crate::ControlSuccess::Application)
}

fn application_view(
    &self,
    id: u64,
) -> Result<crate::ControlledApplication, crate::ControlFailure> {
    let managed = self
        .applications
        .get(&id)
        .ok_or(crate::ControlFailure::NotFound)?;
    let status = self
        .runtime
        .status(managed.runtime)
        .map_err(|error| rejected(&error.to_string()))?;
    let process = match status.process_cell {
        crate::ProcessCellState::Running { process } => Some(process),
        _ => None,
    };
    Ok(crate::ControlledApplication {
        application: id,
        name: managed.durable.name.clone(),
        desired_running: managed.durable.desired_running,
        state: state(&status),
        incarnation: status.incarnation.map(|value| value.incarnation()),
        process,
    })
}

fn runtime_id(&self, id: u64) -> Result<ApplicationId, crate::ControlFailure> {
    self.applications
        .get(&id)
        .map(|application| application.runtime)
        .ok_or(crate::ControlFailure::NotFound)
}

fn incarnation(
    &self,
    id: u64,
) -> Result<crate::ApplicationIncarnationId, crate::ControlFailure> {
    self.applications
        .get(&id)
        .ok_or(crate::ControlFailure::NotFound)?
        .incarnation
        .ok_or_else(|| rejected("application is not running"))
}

fn durable(&self, id: u64) -> Result<DurableApplication, crate::ControlFailure> {
    self.applications
        .get(&id)
        .map(|application| application.durable.clone())
        .ok_or(crate::ControlFailure::NotFound)
}

fn persist(&mut self, durable: &DurableApplication) -> Result<(), CoordinatorError> {
    self.store
        .put(record_key(durable.id), encode_record(durable)?)?;
    Ok(())
}
}
