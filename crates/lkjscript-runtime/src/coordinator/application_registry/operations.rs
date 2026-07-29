impl<S: DurableStorage> MachineCoordinator<S> {
    pub(super) fn application_control(
        &mut self,
        operation: &crate::ControlOperation,
    ) -> Result<crate::ControlSuccess, crate::ControlFailure> {
        match operation {
            crate::ControlOperation::ApplicationInstall(request) => self.install_application(request),
            crate::ControlOperation::ApplicationList => self.list_applications(),
            crate::ControlOperation::ApplicationStart { application } => {
                self.start_application(*application)
            }
            crate::ControlOperation::ApplicationStop { application } => {
                self.stop_application(*application)
            }
            crate::ControlOperation::ApplicationRestart { application } => {
                self.restart_application(*application)
            }
            crate::ControlOperation::ApplicationRemove { application } => {
                self.remove_application(*application)
            }
            crate::ControlOperation::ApplicationInvoke {
                application,
                arguments,
            } => self.invoke_application(*application, arguments.clone()),
            _ => Err(crate::ControlFailure::Malformed),
        }
    }

    fn install_application(
        &mut self,
        request: &crate::ApplicationInstallRequest,
    ) -> Result<crate::ControlSuccess, crate::ControlFailure> {
        if self.applications.len() >= MAX_REGISTERED_APPLICATIONS
            || self
                .applications
                .values()
                .any(|application| application.durable.name == request.name)
        {
            return Err(rejected("application registry bound or duplicate name"));
        }
        let id = self.next_application;
        let next = id
            .checked_add(1)
            .ok_or_else(|| rejected("application registry identity exhausted"))?;
        let durable = DurableApplication {
            id,
            name: request.name.clone(),
            package: request.package,
            package_root: PathBuf::from(&request.package_root),
            entry: ApplicationPath::parse(request.entry.clone())
                .map_err(|error| rejected(&error.to_string()))?,
            capabilities: request.capabilities.clone(),
            max_concurrent: request.max_concurrent_invocations,
            max_total: request.max_total_invocations,
            desired_running: false,
        };
        let managed = self
            .install_record(durable)
            .map_err(|error| rejected(&error.to_string()))?;
        if let Err(error) = self
            .store
            .put(NEXT_KEY.into(), next.to_le_bytes().to_vec())
        {
            let _ = self.runtime.remove(managed.runtime);
            return Err(rejected(&error.to_string()));
        }
        let encoded = encode_record(&managed.durable)
            .map_err(|error| rejected(&error.to_string()))?;
        if let Err(error) = self.store.put(record_key(id), encoded) {
            let _ = self.runtime.remove(managed.runtime);
            return Err(rejected(&error.to_string()));
        }
        self.next_application = next;
        self.applications.insert(id, managed);
        self.application_success(id)
    }

    fn list_applications(&self) -> Result<crate::ControlSuccess, crate::ControlFailure> {
        self.applications
            .keys()
            .map(|id| self.application_view(*id))
            .collect::<Result<Vec<_>, _>>()
            .map(crate::ControlSuccess::Applications)
    }

    fn start_application(&mut self, id: u64) -> Result<crate::ControlSuccess, crate::ControlFailure> {
        let runtime = self.runtime_id(id)?;
        let incarnation = self
            .runtime
            .start(runtime)
            .map_err(|error| rejected(&error.to_string()))?;
        if let Err(error) = self.attach_application_database(id, incarnation) {
            let _ = self.runtime.stop(incarnation);
            return Err(rejected(&error.to_string()));
        }
        let mut durable = self.durable(id)?;
        durable.desired_running = true;
        if let Err(error) = self.persist(&durable) {
            let _ = self.abort_application_database(id);
            let _ = self.runtime.stop(incarnation);
            return Err(rejected(&error.to_string()));
        }
        let managed = self.applications.get_mut(&id).ok_or(crate::ControlFailure::NotFound)?;
        managed.durable = durable;
        managed.incarnation = Some(incarnation);
        self.application_success(id)
    }

    fn stop_application(&mut self, id: u64) -> Result<crate::ControlSuccess, crate::ControlFailure> {
        let incarnation = self.incarnation(id)?;
        self.abort_application_database(id)
            .map_err(|error| rejected(&error.to_string()))?;
        self.runtime
            .stop(incarnation)
            .map_err(|error| rejected(&error.to_string()))?;
        let mut durable = self.durable(id)?;
        durable.desired_running = false;
        if let Err(error) = self.persist(&durable) {
            let runtime = self.runtime_id(id)?;
            if let Ok(replacement) = self.runtime.start(runtime) {
                if self.attach_application_database(id, replacement).is_ok() {
                    if let Some(managed) = self.applications.get_mut(&id) {
                        managed.incarnation = Some(replacement);
                    }
                } else {
                    let _ = self.runtime.stop(replacement);
                }
            }
            return Err(rejected(&error.to_string()));
        }
        let managed = self.applications.get_mut(&id).ok_or(crate::ControlFailure::NotFound)?;
        managed.durable = durable;
        managed.incarnation = None;
        self.application_success(id)
    }

    fn restart_application(
        &mut self,
        id: u64,
    ) -> Result<crate::ControlSuccess, crate::ControlFailure> {
        let incarnation = self.incarnation(id)?;
        self.abort_application_database(id)
            .map_err(|error| rejected(&error.to_string()))?;
        let replacement = self
            .runtime
            .restart(incarnation)
            .map_err(|error| rejected(&error.to_string()))?;
        if let Err(error) = self.attach_application_database(id, replacement) {
            let _ = self.runtime.stop(replacement);
            return Err(rejected(&error.to_string()));
        }
        self.applications
            .get_mut(&id)
            .ok_or(crate::ControlFailure::NotFound)?
            .incarnation = Some(replacement);
        self.application_success(id)
    }

    fn remove_application(&mut self, id: u64) -> Result<crate::ControlSuccess, crate::ControlFailure> {
        let runtime = self.runtime_id(id)?;
        let durable = self.durable(id)?;
        if let Some(incarnation) = self.applications.get(&id).and_then(|app| app.incarnation) {
            self.abort_application_database(id)
                .map_err(|error| rejected(&error.to_string()))?;
            self.runtime
                .stop(incarnation)
                .map_err(|error| rejected(&error.to_string()))?;
        }
        self.runtime
            .remove(runtime)
            .map_err(|error| rejected(&error.to_string()))?;
        if let Err(error) = self.store.delete(record_key(id)) {
            if let Ok(mut replacement) = self.install_record(durable.clone()) {
                if durable.desired_running {
                    replacement.incarnation = self.runtime.start(replacement.runtime).ok();
                }
                let incarnation = replacement.incarnation;
                self.applications.insert(id, replacement);
                if let Some(incarnation) = incarnation {
                    if self.attach_application_database(id, incarnation).is_err() {
                        let _ = self.runtime.stop(incarnation);
                    }
                }
            }
            return Err(rejected(&error.to_string()));
        }
        self.applications.remove(&id);
        Ok(crate::ControlSuccess::ApplicationRemoved { application: id })
    }
}

fn rejected(message: &str) -> crate::ControlFailure {
    crate::ControlFailure::Rejected(message.chars().take(4_096).collect())
}
