//! Scoped canonical signatures. Caller actuals are inserted once and remain opaque.
use super::*;

impl Derivation<'_> {
    pub(super) fn associated_operation(
        &mut self,
        operation: &ExpressionOperation,
        function: DeclarationReference,
    ) -> Result<bool, ExecutionError> {
        let mut associated = self.associated_function(function)?;
        let types = match operation {
            ExpressionOperation::Call { type_arguments, .. }
            | ExpressionOperation::FunctionValue { type_arguments, .. }
            | ExpressionOperation::ImplementationCall { type_arguments, .. } => &type_arguments[..],
            ExpressionOperation::MethodCall { contract, .. } => {
                let DeclarationPayload::OwnedContract(contract) =
                    &self.declaration(*contract, None)?.payload
                else {
                    return Err(reject());
                };
                associated |= !contract.associated_types.is_empty();
                &[][..]
            }
            _ => return Err(reject()),
        };
        for ty in types {
            associated |= self.associated_type(*ty, 0)?;
        }
        Ok(associated)
    }
    pub(super) fn symbolic_member(
        &mut self,
        selected: &SymbolicSelection,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.budget.tick()?;
        if selected.contract != contract {
            return Err(reject());
        }
        match &selected.source {
            SymbolicSelectionSource::Parameter(scope, parameter) => {
                let ty = self.intern_type(TypeForm::AssociatedWitnessProjection {
                    scope: *scope,
                    parameter: *parameter,
                    contract,
                    member,
                })?;
                self.associated_bound(ty)?;
                Ok(ty)
            }
            SymbolicSelectionSource::Concrete { members, .. } => {
                let mut found = None;
                for candidate in members {
                    self.budget.tick()?;
                    if candidate.member == member && found.replace(candidate.ty).is_some() {
                        return Err(reject());
                    }
                }
                found.ok_or_else(reject)
            }
        }
    }

    fn authored_row(
        &mut self,
        effect: &crate::platform::kernel::FunctionEffect,
    ) -> Result<EffectRow, ExecutionError> {
        if let crate::platform::kernel::FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = effect
        {
            self.budget
                .reserve::<RequirementOperand>(requirements.len())?;
            self.budget
                .reserve::<EffectParameterReference>(effect_parameters.len())?;
        }
        Ok(effect.row())
    }

    pub(super) fn scoped_signature(
        &mut self,
        scope: DeclarationReference,
        env: &mut Environment,
    ) -> Result<ScopedSignature, ExecutionError> {
        let owner = self.declaration(scope, None)?;
        let (parameters, result, source, is_task, row) = match &owner.payload {
            DeclarationPayload::Function(function) => (
                &function.parameters[..],
                function.result,
                function.result_borrow,
                matches!(
                    function.effect,
                    crate::platform::kernel::FunctionEffect::Task { .. }
                ),
                self.authored_row(&function.effect)?,
            ),
            DeclarationPayload::External(function) => (
                &function.parameters[..],
                function.result,
                None,
                false,
                EffectRow::default(),
            ),
            _ => return Err(reject()),
        };
        // The row belongs to the callee. Close it before inserting complete caller types.
        let effect = self.row(&row, env)?;
        env.generation = owner.header.contract_version;
        let result = self.ty(result, env, 0)?;
        self.budget
            .reserve::<(TypeObjectDigest, ParameterUse)>(parameters.len())?;
        let mut closed = Vec::new();
        let mut ids = BTreeSet::new();
        for id in parameters {
            self.budget.tick()?;
            if ids.contains(id) {
                return Err(reject());
            }
            self.budget
                .node::<crate::platform::semantic_id::ParameterId>()?;
            ids.insert(*id);
            let OwnerRecord::Parameter(parameter) =
                self.owner(scope.package, OwnerKey::Parameter(*id))?
            else {
                return Err(reject());
            };
            if parameter.parent != ParameterParent::Function(scope.declaration) {
                return Err(reject());
            }
            env.generation = parameter.header.contract_version;
            let ty = self.ty(parameter.ty, env, 0)?;
            env.generation = owner.header.contract_version;
            if self.ty(parameter.ty, env, 0)? != ty {
                return Err(reject());
            }
            closed.push((ty, parameter.use_mode));
        }
        let result_borrow = if let Some(source) = source {
            let mut position = None;
            for (index, candidate) in parameters.iter().enumerate() {
                self.budget.tick()?;
                if *candidate == source {
                    position = Some(u32::try_from(index).map_err(|_| capacity())?);
                }
            }
            Some(position.ok_or_else(reject)?)
        } else {
            None
        };
        if let Some(position) = result_borrow {
            let parameter = closed.get(position as usize).ok_or_else(reject)?;
            if is_task || parameter.1 != ParameterUse::Borrow {
                return Err(reject());
            }
        }
        let mut requires_associated_generation =
            self.associated_function(scope)? || self.associated_type(result, 0)?;
        for (ty, _) in &closed {
            requires_associated_generation |= self.associated_type(*ty, 0)?;
        }
        Ok(ScopedSignature {
            parameters: closed,
            result,
            result_borrow,
            effect,
            is_task,
            requires_associated_generation,
        })
    }

    pub(super) fn scoped_method_signature(
        &mut self,
        selected: &Arc<SymbolicSelection>,
        method: MethodId,
    ) -> Result<ScopedSignature, ExecutionError> {
        let owner = self.declaration(selected.contract, None)?;
        let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
            return Err(reject());
        };
        let mut found = None;
        for candidate in &contract.methods {
            self.budget.tick()?;
            if candidate.id == method && found.replace(candidate).is_some() {
                return Err(reject());
            }
        }
        let method = found.ok_or_else(reject)?;
        if selected.arguments.len() != contract.type_parameters.len() {
            return Err(reject());
        }
        let mut env = Environment {
            scope: Some(selected.contract),
            generation: owner.header.contract_version,
            symbolic: true,
            symbolic_contract: Some(Arc::clone(selected)),
            ..Environment::default()
        };
        self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
        env.ordinary
            .insert(contract.self_parameter, selected.self_type);
        for (formal, actual) in contract.type_parameters.iter().zip(&selected.arguments) {
            self.budget.tick()?;
            self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
            if env.ordinary.insert(*formal, *actual).is_some() {
                return Err(reject());
            }
        }
        let row = self.authored_row(&method.effect)?;
        let effect = self.row(&row, &env)?;
        let mut requires_associated_generation =
            !contract.associated_types.is_empty() || self.associated_type(method.result, 0)?;
        let result = self.ty(method.result, &env, 0)?;
        self.budget
            .reserve::<(TypeObjectDigest, ParameterUse)>(method.parameters.len())?;
        let mut parameters = Vec::new();
        for parameter in &method.parameters {
            self.budget.tick()?;
            requires_associated_generation |= self.associated_type(parameter.ty, 0)?;
            let ty = self.ty(parameter.ty, &env, 0)?;
            requires_associated_generation |= self.associated_type(ty, 0)?;
            parameters.push((ty, parameter.use_mode));
        }
        requires_associated_generation |= self.associated_type(result, 0)?;
        Ok(ScopedSignature {
            parameters,
            result,
            result_borrow: method.result_borrow,
            effect,
            is_task: matches!(
                method.effect,
                crate::platform::kernel::FunctionEffect::Task { .. }
            ),
            requires_associated_generation,
        })
    }

    pub(super) fn compare_scoped_method(
        &mut self,
        expected: &ScopedSignature,
        target: &SymbolicTarget,
    ) -> Result<(), ExecutionError> {
        self.budget.tick()?;
        let actual = &target.signature;
        if expected.result != actual.result
            || expected.result_borrow != actual.result_borrow
            || expected.is_task != actual.is_task
            || expected.parameters.len() != actual.parameters.len()
        {
            return Err(reject());
        }
        for (expected, actual) in expected.parameters.iter().zip(&actual.parameters) {
            self.budget.tick()?;
            if expected != actual {
                return Err(reject());
            }
        }
        for _ in expected
            .effect
            .requirements
            .iter()
            .chain(&actual.effect.requirements)
        {
            self.budget.tick()?;
        }
        for _ in expected
            .effect
            .parameters
            .iter()
            .chain(&actual.effect.parameters)
        {
            self.budget.tick()?;
        }
        if expected.effect != actual.effect {
            return Err(reject());
        }
        let scope = target.function.ok_or_else(reject)?;
        let DeclarationPayload::Function(function) = &self.declaration(scope, None)?.payload else {
            return Err(reject());
        };
        for id in &function.parameters {
            let OwnerRecord::Parameter(parameter) =
                self.owner(scope.package, OwnerKey::Parameter(*id))?
            else {
                return Err(reject());
            };
            if parameter.resource_requirement.is_some() {
                return Err(reject());
            }
        }
        Ok(())
    }
}
