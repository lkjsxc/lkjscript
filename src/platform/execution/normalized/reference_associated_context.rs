//! Read-only contexts sealed against independently derived canonical applications.
use super::*;

pub(in super::super::super) struct Context<'a> {
    proof: &'a Proof,
    application: &'a Application,
    function: &'a Function,
}

impl Proof {
    pub(in super::super::super) fn application(
        &self,
        application: &Application,
        control: &ExecutionControl,
    ) -> Result<&Function, ExecutionError> {
        control.check()?;
        self.applications.get(application).ok_or_else(reject)
    }

    pub(in super::super::super) fn context<'a>(
        &'a self,
        application: &Application,
        control: &ExecutionControl,
    ) -> Result<Context<'a>, ExecutionError> {
        control.check()?;
        let (application, function) = self
            .applications
            .get_key_value(application)
            .ok_or_else(reject)?;
        Ok(Context {
            proof: self,
            application,
            function,
        })
    }

    pub(in super::super::super) fn selection(
        &self,
        implementation: DeclarationReference,
        actuals: &[TypeObjectDigest],
        prerequisites: &[usize],
        control: &ExecutionControl,
    ) -> Result<&Arc<Selection>, ExecutionError> {
        control.check()?;
        self.selection_index
            .get(&implementation)
            .and_then(|actuals_index| actuals_index.get(actuals))
            .and_then(|prerequisites_index| prerequisites_index.get(prerequisites))
            .ok_or_else(reject)
    }

    pub(in super::super::super) fn selection_by_id(
        &self,
        identity: usize,
        control: &ExecutionControl,
    ) -> Result<&Arc<Selection>, ExecutionError> {
        control.check()?;
        self.selections
            .get(identity)
            .filter(|selection| selection.identity == identity)
            .ok_or_else(reject)
    }

    pub(in super::super::super) fn closed_root(
        &self,
        application: &Application,
        occurrence: (PackageId, OwnerKey),
        template: TypeObjectDigest,
        control: &ExecutionControl,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.context(application, control)?
            .root(occurrence, template, control)
    }

    pub(in super::super::super) fn call_target(
        &self,
        application: &Application,
        expression: ExpressionId,
        control: &ExecutionControl,
    ) -> Result<&Application, ExecutionError> {
        control.check()?;
        self.context(application, control)?.occurrence(
            (
                application.function.package,
                OwnerKey::Expression(expression),
            ),
            control,
        )?;
        self.application(application, control)?
            .targets
            .get(&expression)
            .filter(|target| self.applications.contains_key(*target))
            .ok_or_else(reject)
    }

    pub(in super::super::super) fn contract_template(
        &self,
        contract: DeclarationReference,
        control: &ExecutionControl,
    ) -> Result<&crate::platform::kernel::OwnedContract, ExecutionError> {
        control.check()?;
        self.contract_templates.get(&contract).ok_or_else(reject)
    }

    pub(in super::super::super) fn mapping_template(
        &self,
        selection: &Selection,
        method: MethodId,
        control: &ExecutionControl,
    ) -> Result<&crate::platform::kernel::OwnedMethodImplementation, ExecutionError> {
        control.check()?;
        let canonical = self.selection_by_id(selection.identity, control)?;
        if !std::ptr::eq(Arc::as_ptr(canonical), selection) {
            return Err(reject());
        }
        self.mapping_templates
            .get(&(selection.implementation, method))
            .ok_or_else(reject)
    }

    pub(in super::super::super) fn method_target(
        &self,
        selection: &Selection,
        method: MethodId,
        control: &ExecutionControl,
    ) -> Result<&Application, ExecutionError> {
        control.check()?;
        let canonical = self.selection_by_id(selection.identity, control)?;
        if !std::ptr::eq(Arc::as_ptr(canonical), selection) {
            return Err(reject());
        }
        self.methods
            .get(&(selection.identity, method))
            .filter(|target| self.applications.contains_key(*target))
            .ok_or_else(reject)
    }
}

impl Context<'_> {
    pub(in super::super::super) fn occurrence(
        &self,
        occurrence: (PackageId, OwnerKey),
        control: &ExecutionControl,
    ) -> Result<u16, ExecutionError> {
        control.check()?;
        if occurrence.0 != self.application.function.package {
            return Err(reject());
        }
        self.function
            .occurrences
            .get(&occurrence.1)
            .copied()
            .ok_or_else(reject)
    }
    pub(in super::super::super) fn root(
        &self,
        occurrence: (PackageId, OwnerKey),
        template: TypeObjectDigest,
        control: &ExecutionControl,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        control.check()?;
        if occurrence.0 != self.application.function.package {
            return Err(reject());
        }
        self.occurrence(occurrence, control)?;
        let result = self
            .function
            .roots
            .get(&(occurrence.1, template))
            .copied()
            .ok_or_else(reject)?;
        if !self.proof.contains_type(&result) {
            return Err(reject());
        }
        Ok(result)
    }
}
