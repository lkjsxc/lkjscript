//! Static witness dispatch derived from canonical source, independently of prepared instances.
use super::*;
use crate::platform::kernel::{
    ImplementationOperand, ImplementationParameter, OwnedImplementation, TypeParameterConstraints,
};
use crate::platform::semantic_id::ImplementationParameterId;

use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub(super) struct AppliedReferenceImplementation {
    pub(super) identity: u64,
    pub(super) depth: usize,
    pub(super) implementation: DeclarationReference,
    pub(super) type_arguments: Arc<[TypeObjectDigest]>,
    pub(super) implementations: Arc<[AppliedReferenceImplementation]>,
}

impl PartialEq for AppliedReferenceImplementation {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}
impl Eq for AppliedReferenceImplementation {}
impl PartialOrd for AppliedReferenceImplementation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AppliedReferenceImplementation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identity.cmp(&other.identity)
    }
}

type WitnessKey = (DeclarationReference, Arc<[TypeObjectDigest]>, Arc<[u64]>);
#[derive(Default)]
pub(super) struct ReferenceWitnessInterner {
    values: BTreeMap<WitnessKey, AppliedReferenceImplementation>,
}

type Implementations = BTreeMap<ImplementationParameterId, AppliedReferenceImplementation>;

impl ReferenceState<'_> {
    pub(super) fn resolve_implementation(
        &mut self,
        operand: ImplementationOperand,
    ) -> Result<AppliedReferenceImplementation, ExecutionError> {
        self.resolve_implementation_in(&operand, None, None, 0, &mut 0)
    }

    fn resolve_implementation_in(
        &mut self,
        operand: &ImplementationOperand,
        witnesses: Option<(DeclarationReference, &Implementations)>,
        types: Option<&BTreeMap<TypeParameterId, TypeObjectDigest>>,
        depth: usize,
        visits: &mut usize,
    ) -> Result<AppliedReferenceImplementation, ExecutionError> {
        self.control.check()?;
        Self::witness_tree_step(depth, visits)?;
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                self.charge_allocation(
                    (type_arguments.len() * std::mem::size_of::<TypeObjectDigest>()
                        + implementations.len()
                            * std::mem::size_of::<AppliedReferenceImplementation>())
                        as u64,
                )?;
                let type_arguments = if let Some(types) = types {
                    type_arguments
                        .iter()
                        .map(|ty| self.method_type(*ty, types))
                        .collect::<Result<_, _>>()?
                } else {
                    self.resolve_type_arguments(type_arguments)?
                };
                let implementations = implementations
                    .iter()
                    .map(|operand| {
                        self.resolve_implementation_in(operand, witnesses, types, depth + 1, visits)
                    })
                    .collect::<Result<_, _>>()?;
                self.intern_implementation(*implementation, type_arguments, implementations)
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let lexical = self
                    .implementation_scopes
                    .last()
                    .map(|(scope, values)| (*scope, values));
                let Some((declaration, values)) = witnesses.or(lexical) else {
                    return Err(reference_type_error("witness has no lexical scope"));
                };
                if declaration != *scope {
                    return Err(reference_type_error(
                        "witness belongs to another declaration",
                    ));
                }
                let selected = values
                    .get(parameter)
                    .ok_or_else(|| reference_type_error("unbound implementation witness"))?;
                Self::witness_tree_step(depth.saturating_add(selected.depth), visits)?;
                Ok(selected.clone())
            }
        }
    }

    pub(super) fn checked_implementation(
        &mut self,
        selected: &AppliedReferenceImplementation,
    ) -> Result<OwnedImplementation, ExecutionError> {
        self.checked_implementation_in(selected, 0, &mut 0, &mut BTreeSet::new())
    }

    fn intern_implementation(
        &mut self,
        implementation: DeclarationReference,
        types: Vec<TypeObjectDigest>,
        children: Vec<AppliedReferenceImplementation>,
    ) -> Result<AppliedReferenceImplementation, ExecutionError> {
        let depth = children
            .iter()
            .map(|child| child.depth.saturating_add(1))
            .max()
            .unwrap_or(0);
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(reference_type_error(
                "implementation DAG exceeds canonical depth",
            ));
        }
        self.charge_allocation((children.len() * std::mem::size_of::<u64>()) as u64)?;
        let children_ids = children
            .iter()
            .map(|child| child.identity)
            .collect::<Vec<_>>();
        let key = (
            implementation,
            Arc::<[TypeObjectDigest]>::from(types),
            Arc::<[u64]>::from(children_ids),
        );
        let interner = Arc::clone(&self.reference_witnesses);
        let mut interner = interner
            .lock()
            .map_err(|_| reference_type_error("reference witness interner is poisoned"))?;
        if let Some(selected) = interner.values.get(&key) {
            return Ok(selected.clone());
        }
        self.charge_allocation(
            (std::mem::size_of::<(WitnessKey, AppliedReferenceImplementation)>()
                + 3 * std::mem::size_of::<usize>()) as u64,
        )?;
        let identity = u64::try_from(interner.values.len())
            .ok()
            .and_then(|id| id.checked_add(1))
            .ok_or_else(|| reference_type_error("reference witness identity overflow"))?;
        let selected = AppliedReferenceImplementation {
            identity,
            depth,
            implementation,
            type_arguments: Arc::clone(&key.1),
            implementations: children.into(),
        };
        interner.values.insert(key, selected.clone());
        Ok(selected)
    }

    fn checked_implementation_in(
        &mut self,
        selected: &AppliedReferenceImplementation,
        depth: usize,
        visits: &mut usize,
        active: &mut BTreeSet<AppliedReferenceImplementation>,
    ) -> Result<OwnedImplementation, ExecutionError> {
        Self::witness_tree_step(depth, visits)?;
        self.charge_witness(selected, depth, &mut 0)?;
        if !active.insert(selected.clone()) {
            // Exact repeated applications close an ordinary mapped callable
            // cycle. The first visit still validates every mapping and formal.
            let DeclarationPayload::OwnedImplementation(mut implementation) =
                self.declaration(selected.implementation)?.payload
            else {
                return Err(reference_type_error(
                    "witness does not select an implementation owner",
                ));
            };
            let bindings = implementation
                .type_parameters
                .iter()
                .copied()
                .zip(selected.type_arguments.iter().copied())
                .collect::<BTreeMap<_, _>>();
            implementation.self_type = self.method_type(implementation.self_type, &bindings)?;
            for argument in &mut implementation.type_arguments {
                *argument = self.method_type(*argument, &bindings)?;
            }
            for mapping in &mut implementation.methods {
                for argument in &mut mapping.type_arguments {
                    *argument = self.method_type(*argument, &bindings)?;
                }
            }
            return Ok(implementation);
        }
        self.checked_implementation_body(selected, depth, visits, active)
    }

    fn checked_implementation_body(
        &mut self,
        selected: &AppliedReferenceImplementation,
        depth: usize,
        visits: &mut usize,
        active: &mut BTreeSet<AppliedReferenceImplementation>,
    ) -> Result<OwnedImplementation, ExecutionError> {
        let reference = selected.implementation;
        let DeclarationPayload::OwnedImplementation(implementation) =
            self.declaration(reference)?.payload
        else {
            return Err(reference_type_error(
                "witness does not select an implementation owner",
            ));
        };
        if implementation.type_parameters.len() != selected.type_arguments.len()
            || implementation.type_parameters.len()
                > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(reference_type_error(
                "implementation type argument arity mismatch",
            ));
        }
        self.charge_allocation(
            (implementation.type_parameters.len()
                * (std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
                    + 6 * std::mem::size_of::<usize>())) as u64,
        )?;
        let mut scheme_parameters = BTreeSet::new();
        let mut scheme_names = BTreeSet::new();
        let mut scheme_bindings = BTreeMap::new();
        for (parameter, actual) in implementation
            .type_parameters
            .iter()
            .zip(selected.type_arguments.iter())
        {
            self.witness_metadata_step()?;
            let Some(OwnerRecord::TypeParameter(owner)) =
                self.owner_in_package(reference.package, OwnerKey::TypeParameter(*parameter))?
            else {
                return Err(reference_type_error(
                    "missing implementation type parameter",
                ));
            };
            if owner.header.owner != OwnerKey::TypeParameter(*parameter)
                || owner.declaration != reference.declaration
                || !owner.constraints.has_owned()
                || (owner.constraints.requires_transfer()
                    && !self.schema.transferable_types.contains(actual))
                || (owner.constraints.requires_share()
                    && !self.schema.shareable_types.contains(actual))
                || !scheme_parameters.insert(*parameter)
                || !scheme_names.insert(owner.name)
                || !self.owned_method_type(*actual, &BTreeSet::new())?
            {
                return Err(reference_type_error(
                    "implementation requires distinct scoped Owned formals and closed Owned arguments",
                ));
            }
            scheme_bindings.insert(*parameter, *actual);
        }
        // Validate the open templates before substitution: an application cannot
        // erase a foreign parameter or an invalid unused mapping argument.
        if !self.owned_method_type(implementation.self_type, &scheme_parameters)? {
            return Err(reference_type_error(
                "implementation template has an invalid Owned scope",
            ));
        }
        for argument in &implementation.type_arguments {
            if !self.owned_method_type(*argument, &scheme_parameters)? {
                return Err(reference_type_error(
                    "implementation template has an invalid Owned scope",
                ));
            }
        }
        let prerequisite_bindings = self.implementation_bindings_in(
            reference,
            &implementation.implementation_parameters,
            &scheme_bindings,
            &selected.implementations,
            depth + 1,
            visits,
            active,
        )?;
        let mut implementation = implementation;
        implementation.self_type = self.method_type(implementation.self_type, &scheme_bindings)?;
        for actual in &mut implementation.type_arguments {
            *actual = self.method_type(*actual, &scheme_bindings)?;
        }
        for mapping in &mut implementation.methods {
            for actual in &mut mapping.type_arguments {
                if !self.owned_method_type(*actual, &scheme_parameters)? {
                    return Err(reference_type_error(
                        "method mapping requires scoped Owned type arguments",
                    ));
                }
                *actual = self.method_type(*actual, &scheme_bindings)?;
            }
        }
        if !self.owned_method_type(implementation.self_type, &BTreeSet::new())? {
            return Err(reference_type_error("implementation has a non-owned Self"));
        }
        let DeclarationPayload::OwnedContract(contract) =
            self.declaration(implementation.contract)?.payload
        else {
            return Err(reference_type_error(
                "implementation names a foreign contract kind",
            ));
        };
        if contract.type_parameters.len() >= crate::platform::kernel::contract::MAXIMUM_CHILDREN
            || contract.type_parameters.len() != implementation.type_arguments.len()
            || contract.methods.len() != implementation.methods.len()
            || contract.methods.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(reference_type_error(
                "implementation does not satisfy the exact owned contract",
            ));
        }
        if contract.methods.is_empty() {
            return Err(reference_type_error("empty implementation methods"));
        }
        self.charge_allocation(
            ((contract.type_parameters.len() + 1)
                * (2 * std::mem::size_of::<TypeParameterId>()
                    + std::mem::size_of::<TypeObjectDigest>()
                    + std::mem::size_of::<crate::platform::kernel::Name>()
                    + 9 * std::mem::size_of::<usize>())) as u64,
        )?;
        let mut declared = BTreeSet::new();
        let mut parameter_names = BTreeSet::new();
        let mut substitutions = BTreeMap::new();
        for (parameter, actual) in
            std::iter::once((contract.self_parameter, implementation.self_type)).chain(
                contract
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(implementation.type_arguments.iter().copied()),
            )
        {
            self.witness_metadata_step()?;
            let Some(OwnerRecord::TypeParameter(owner)) = self.owner_in_package(
                implementation.contract.package,
                OwnerKey::TypeParameter(parameter),
            )?
            else {
                return Err(reference_type_error(
                    "missing owned contract parameter owner",
                ));
            };
            if owner.header.owner != OwnerKey::TypeParameter(parameter)
                || owner.declaration != implementation.contract.declaration
                || owner.constraints != TypeParameterConstraints::Owned
                || (parameter != contract.self_parameter && owner.header.contract_version < 26)
                || !declared.insert(parameter)
                || !parameter_names.insert(owner.name)
                || !self.owned_method_type(actual, &BTreeSet::new())?
            {
                return Err(reference_type_error(
                    "contract application requires distinct exact Owned parameters and closed owned arguments",
                ));
            }
            substitutions.insert(parameter, actual);
        }
        for pair in implementation.methods.windows(2) {
            self.witness_metadata_step()?;
            if pair[0].method >= pair[1].method {
                return Err(reference_type_error("unordered implementation methods"));
            }
        }
        self.charge_allocation(
            (contract.methods.len()
                * (std::mem::size_of::<crate::platform::semantic_id::MethodId>()
                    + std::mem::size_of::<&crate::platform::kernel::Name>()
                    + 6 * std::mem::size_of::<usize>())) as u64,
        )?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for method in &contract.methods {
            self.witness_metadata_step()?;
            if !ids.insert(method.id)
                || !names.insert(&method.name)
                || method.parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
            {
                return Err(reference_type_error(
                    "invalid method identity or callable kind",
                ));
            }
            if let FunctionEffect::Task {
                requirements,
                effect_parameters,
            } = &method.effect
            {
                for _ in requirements {
                    self.witness_metadata_step()?;
                }
                for _ in effect_parameters {
                    self.witness_metadata_step()?;
                }
            }
            let row = method.effect.row();
            if row.validate().is_err() || !row.is_closed() {
                return Err(reference_type_error(
                    "method effect row is not closed and canonical",
                ));
            }
            let mut suffix = false;
            for p in &method.parameters {
                self.witness_metadata_step()?;
                if self.owned_method_type(p.ty, &declared)? {
                    suffix = true;
                    if p.use_mode == ParameterUse::Unrestricted
                        || (!matches!(method.effect, FunctionEffect::Pure)
                            && p.use_mode == ParameterUse::Borrow
                            && self
                                .declaration(implementation.contract)?
                                .header
                                .contract_version
                                < crate::platform::kernel::contract::SHARE_GRAPH_CONTRACT_VERSION)
                    {
                        return Err(reference_type_error(
                            "owned method parameters require scoped borrowing or consumption",
                        ));
                    }
                } else if suffix || p.use_mode != ParameterUse::Unrestricted {
                    return Err(reference_type_error("method is not first order"));
                }
            }
            let result_owned = self.owned_method_type(method.result, &declared)?;
            if let Some(position) = method.result_borrow {
                let source = method.parameters.get(position as usize).ok_or_else(|| {
                    reference_type_error("method result borrows from a foreign parameter position")
                })?;
                if !matches!(method.effect, FunctionEffect::Pure)
                    || !result_owned
                    || source.use_mode != ParameterUse::Borrow
                    || !self.owned_method_type(source.ty, &declared)?
                {
                    return Err(reference_type_error(
                        "method borrowed result lacks an exact pure memory source",
                    ));
                }
            }
            let mut mapping = None;
            for candidate in &implementation.methods {
                self.witness_metadata_step()?;
                if candidate.method == method.id && mapping.replace(candidate).is_some() {
                    return Err(reference_type_error("duplicate method implementation"));
                }
            }
            let mapping =
                mapping.ok_or_else(|| reference_type_error("missing method implementation"))?;
            let target = mapping.function;
            let target_owner = self.declaration(target)?;
            if target.package != reference.package
                && target_owner.visibility != crate::platform::kernel::DeclarationVisibility::Public
            {
                return Err(reference_type_error("method implementation is not visible"));
            }
            let DeclarationPayload::Function(function) = target_owner.payload else {
                return Err(reference_type_error(
                    "method implementation must be a graph function",
                ));
            };
            if function.effect != method.effect
                || function.type_parameters.len() != mapping.type_arguments.len()
                || !function.effect_parameters.is_empty()
                || !function.requirement_parameters.is_empty()
                || function.implementation_parameters.len() != mapping.implementations.len()
                || function.parameters.len() != method.parameters.len()
                || function.result_borrow
                    != method
                        .result_borrow
                        .and_then(|position| function.parameters.get(position as usize).copied())
            {
                return Err(reference_type_error(
                    "method implementation must have exact applied type, callable kind and effects",
                ));
            }
            let mut target_parameters = BTreeSet::new();
            let mut target_names = BTreeSet::new();
            let mut target_bindings = BTreeMap::new();
            self.charge_allocation(
                (function.type_parameters.len()
                    * (2 * std::mem::size_of::<TypeParameterId>()
                        + std::mem::size_of::<TypeObjectDigest>()
                        + std::mem::size_of::<crate::platform::kernel::Name>()
                        + 9 * std::mem::size_of::<usize>())) as u64,
            )?;
            for (parameter, actual) in function.type_parameters.iter().zip(&mapping.type_arguments)
            {
                self.witness_metadata_step()?;
                let Some(OwnerRecord::TypeParameter(owner)) =
                    self.owner_in_package(target.package, OwnerKey::TypeParameter(*parameter))?
                else {
                    return Err(reference_type_error("missing method target type parameter"));
                };
                if owner.header.owner != OwnerKey::TypeParameter(*parameter)
                    || owner.declaration != target.declaration
                    || !owner.constraints.has_owned()
                    || (owner.constraints.requires_transfer()
                        && !self.schema.transferable_types.contains(actual))
                    || (owner.constraints.requires_share()
                        && !self.schema.shareable_types.contains(actual))
                    || !target_parameters.insert(*parameter)
                    || !target_names.insert(owner.name)
                    || !self.owned_method_type(*actual, &BTreeSet::new())?
                {
                    return Err(reference_type_error(
                        "method targets require distinct exact Owned type parameters",
                    ));
                }
                target_bindings.insert(*parameter, *actual);
            }
            self.charge_allocation(
                (mapping.implementations.len()
                    * std::mem::size_of::<AppliedReferenceImplementation>()) as u64,
            )?;
            let mut mapped_witnesses = Vec::with_capacity(mapping.implementations.len());
            for operand in &mapping.implementations {
                self.validate_mapping_operand(operand, &scheme_parameters, true, 0, &mut 0)?;
                mapped_witnesses.push(self.resolve_implementation_in(
                    operand,
                    Some((reference, &prerequisite_bindings)),
                    Some(&scheme_bindings),
                    depth + 1,
                    visits,
                )?);
            }
            self.implementation_bindings_in(
                target,
                &function.implementation_parameters,
                &target_bindings,
                &mapped_witnesses,
                depth + 1,
                visits,
                active,
            )?;
            let parameters = self.parameters(target.package, &function.parameters)?;
            for (actual, expected) in parameters.iter().zip(&method.parameters) {
                self.owned_method_type(actual.ty, &target_parameters)?;
                if actual.parent
                    != crate::platform::kernel::ParameterParent::Function(target.declaration)
                    || self.method_type(actual.ty, &target_bindings)?
                        != self.method_type(expected.ty, &substitutions)?
                    || actual.use_mode != expected.use_mode
                    || actual.resource_requirement.is_some()
                {
                    return Err(reference_type_error("method parameter contract mismatch"));
                }
            }
            self.owned_method_type(function.result, &target_parameters)?;
            if self.method_type(function.result, &target_bindings)?
                != self.method_type(method.result, &substitutions)?
            {
                return Err(reference_type_error("method result contract mismatch"));
            }
        }
        Ok(implementation)
    }

    pub(super) fn method_type(
        &self,
        ty: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.schema
            .transfer_type_identity(ty, substitutions, self.control)
    }

    pub(super) fn implementation_bindings(
        &mut self,
        scope: DeclarationReference,
        parameters: &[ImplementationParameter],
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        supplied: &[AppliedReferenceImplementation],
    ) -> Result<Implementations, ExecutionError> {
        self.implementation_bindings_in(
            scope,
            parameters,
            types,
            supplied,
            0,
            &mut 0,
            &mut BTreeSet::new(),
        )
    }

    // Keep the four semantic binding inputs explicit; depth, visits and active
    // carry one shared traversal ledger through recursive prerequisite admission.
    #[allow(clippy::too_many_arguments)]
    fn implementation_bindings_in(
        &mut self,
        scope: DeclarationReference,
        parameters: &[ImplementationParameter],
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        supplied: &[AppliedReferenceImplementation],
        depth: usize,
        visits: &mut usize,
        active: &mut BTreeSet<AppliedReferenceImplementation>,
    ) -> Result<Implementations, ExecutionError> {
        if parameters.len() != supplied.len()
            || parameters.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(reference_type_error(
                "missing or extra static implementation operands",
            ));
        }
        self.charge_allocation(
            (supplied.len()
                * std::mem::size_of::<(ImplementationParameterId, AppliedReferenceImplementation)>(
                )) as u64,
        )?;
        let mut bindings = BTreeMap::new();
        let mut names = BTreeSet::new();
        let declared = types.keys().copied().collect::<BTreeSet<_>>();
        for (parameter, selected) in parameters.iter().zip(supplied) {
            self.witness_metadata_step()?;
            let implementation = self.checked_implementation_in(selected, depth, visits, active)?;
            let Some(TypeForm::TypeParameter {
                parameter: type_parameter,
            }) = self.schema.types.get(&parameter.self_type).map(|t| &t.form)
            else {
                return Err(reference_type_error(
                    "witness Self is not an exact type parameter",
                ));
            };
            let type_parameter = *type_parameter;
            let Some(OwnerRecord::TypeParameter(owner)) =
                self.owner_in_package(scope.package, OwnerKey::TypeParameter(type_parameter))?
            else {
                return Err(reference_type_error("missing witness Self parameter owner"));
            };
            if owner.header.owner != OwnerKey::TypeParameter(type_parameter)
                || owner.declaration != scope.declaration
                || !owner.constraints.has_owned()
                || !declared.contains(&type_parameter)
                || !names.insert(&parameter.name)
            {
                return Err(reference_type_error(
                    "witness formal has a foreign Owned scope",
                ));
            }
            for expected in &parameter.type_arguments {
                if !self.owned_method_type(*expected, &declared)? {
                    return Err(reference_type_error(
                        "witness argument has a foreign Owned scope",
                    ));
                }
            }
            self.charge_witness(selected, depth, &mut 0)?;
            if implementation.contract != parameter.contract
                || types.get(&type_parameter) != Some(&implementation.self_type)
                || parameter.type_arguments.len() != implementation.type_arguments.len()
                || bindings.insert(parameter.id, selected.clone()).is_some()
            {
                return Err(reference_type_error(
                    "static witness contract or instantiated Self mismatch",
                ));
            }
            for (expected, actual) in parameter
                .type_arguments
                .iter()
                .zip(&implementation.type_arguments)
            {
                self.witness_metadata_step()?;
                if self.method_type(*expected, types)? != *actual {
                    return Err(reference_type_error(
                        "static witness instantiated contract argument mismatch",
                    ));
                }
            }
        }
        Ok(bindings)
    }

    pub(super) fn method_target(
        &mut self,
        witness: ImplementationOperand,
        contract: DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
    ) -> Result<
        (
            DeclarationReference,
            Vec<TypeObjectDigest>,
            Vec<AppliedReferenceImplementation>,
        ),
        ExecutionError,
    > {
        let selected = self.resolve_implementation(witness)?;
        let implementation = self.checked_implementation(&selected)?;
        if implementation.contract != contract {
            return Err(reference_type_error("method uses another nominal contract"));
        }
        let scheme_bindings = implementation
            .type_parameters
            .iter()
            .copied()
            .zip(selected.type_arguments.iter().copied())
            .collect::<BTreeMap<_, _>>();
        let prerequisites = self.implementation_bindings(
            selected.implementation,
            &implementation.implementation_parameters,
            &scheme_bindings,
            &selected.implementations,
        )?;
        for m in &implementation.methods {
            self.witness_metadata_step()?;
            if m.method == method {
                self.charge_allocation(
                    (m.type_arguments.len() * std::mem::size_of::<TypeObjectDigest>()) as u64,
                )?;
                self.charge_allocation(
                    (m.implementations.len()
                        * std::mem::size_of::<AppliedReferenceImplementation>())
                        as u64,
                )?;
                let implementations = m
                    .implementations
                    .iter()
                    .map(|operand| {
                        self.resolve_implementation_in(
                            operand,
                            Some((selected.implementation, &prerequisites)),
                            Some(&scheme_bindings),
                            0,
                            &mut 0,
                        )
                    })
                    .collect::<Result<_, _>>()?;
                return Ok((m.function, m.type_arguments.clone(), implementations));
            }
        }
        Err(reference_type_error("method absent from implementation"))
    }

    pub(super) fn witness_call(
        &mut self,
        function: DeclarationReference,
        types: &[TypeObjectDigest],
        effects: &[EffectRow],
        requirements: &[RequirementOperand],
        operands: &[ImplementationOperand],
        arguments: Vec<CheckedValue>,
    ) -> Result<AdmittedGraphCall, ExecutionError> {
        self.charge_allocation(
            (operands.len() * std::mem::size_of::<AppliedReferenceImplementation>()) as u64,
        )?;
        let mut selected = Vec::with_capacity(operands.len());
        for operand in operands {
            selected.push(self.resolve_implementation(operand.clone())?);
        }
        let types = self.resolve_type_arguments(types)?;
        let effects = self.resolve_effect_arguments(effects)?;
        let requirements = self.resolve_requirement_arguments(requirements)?;
        self.admit_resolved_witness_call(
            function,
            &types,
            &effects,
            &requirements,
            &selected,
            arguments,
        )
    }

    pub(super) fn admit_resolved_witness_call(
        &mut self,
        function: DeclarationReference,
        types: &[TypeObjectDigest],
        effects: &[EffectRow],
        requirements: &[RequirementOperand],
        implementations: &[AppliedReferenceImplementation],
        arguments: Vec<CheckedValue>,
    ) -> Result<AdmittedGraphCall, ExecutionError> {
        let DeclarationPayload::Function(declaration) = self.declaration(function)?.payload else {
            return Err(reference_type_error(
                "witness call requires a graph function",
            ));
        };
        self.admit_graph_call_with_implementations(
            function,
            declaration,
            arguments,
            ReferenceApplication {
                types,
                effects,
                requirements,
                implementations,
            },
        )
    }

    pub(super) fn execute_witness_call(
        &mut self,
        target: AdmittedGraphCall,
    ) -> Result<CheckedValue, ExecutionError> {
        if self.call_depth.saturating_add(self.ancestor_depth) >= self.policy.maximum_call_depth {
            return Err(reference_resource(
                "normalized_reference_call_depth",
                "witness call exceeds call-depth budget",
            ));
        }
        self.call_depth += 1;
        self.observation.maximum_call_depth = self
            .observation
            .maximum_call_depth
            .max(self.call_depth.saturating_add(self.ancestor_depth));
        let package = self.active_package;
        let mut step = self.enter_graph_call(target, false);
        let result = loop {
            match step {
                Ok(ReferenceStep::Value(value)) => break Ok(value),
                Ok(ReferenceStep::Tail(target)) => step = self.enter_graph_call(*target, true),
                Err(error) => break Err(error),
            }
        };
        self.active_package = package;
        self.call_depth -= 1;
        result
    }
}

impl ReferenceState<'_> {
    fn witness_tree_step(depth: usize, visits: &mut usize) -> Result<(), ExecutionError> {
        *visits = visits
            .checked_add(1)
            .ok_or_else(|| reference_type_error("implementation tree size overflow"))?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
            || *visits > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(reference_type_error(
                "implementation tree exceeds canonical bounds",
            ));
        }
        Ok(())
    }

    fn charge_witness(
        &mut self,
        selected: &AppliedReferenceImplementation,
        depth: usize,
        visits: &mut usize,
    ) -> Result<(), ExecutionError> {
        self.witness_metadata_step()?;
        Self::witness_tree_step(depth.saturating_add(selected.depth), visits)?;
        self.charge_allocation(
            (std::mem::size_of::<AppliedReferenceImplementation>()
                + 3 * std::mem::size_of::<usize>()) as u64,
        )
    }

    fn validate_mapping_operand(
        &mut self,
        operand: &ImplementationOperand,
        declared: &BTreeSet<TypeParameterId>,
        parameter_allowed: bool,
        depth: usize,
        visits: &mut usize,
    ) -> Result<(), ExecutionError> {
        self.witness_metadata_step()?;
        Self::witness_tree_step(depth, visits)?;
        match operand {
            ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } => {
                for ty in type_arguments {
                    if !self.owned_method_type(*ty, declared)? {
                        return Err(reference_type_error(
                            "mapping witness has a foreign Owned argument",
                        ));
                    }
                }
                for child in implementations {
                    self.validate_mapping_operand(child, declared, false, depth + 1, visits)?;
                }
            }
            ImplementationOperand::Parameter { .. } if !parameter_allowed => {
                return Err(reference_type_error(
                    "concrete mapping witness captures a scoped prerequisite",
                ));
            }
            ImplementationOperand::Parameter { .. } => {}
        }
        Ok(())
    }

    // Classify the permitted first-order contract grammar from canonical type objects.
    // Every composite member is checked, including ordinary and phantom members.
    fn owned_method_type(
        &mut self,
        ty: TypeObjectDigest,
        declared: &BTreeSet<TypeParameterId>,
    ) -> Result<bool, ExecutionError> {
        self.method_shape(ty, declared, 0, &mut 0)
    }

    fn method_shape(
        &mut self,
        ty: TypeObjectDigest,
        declared: &BTreeSet<TypeParameterId>,
        depth: usize,
        visits: &mut usize,
    ) -> Result<bool, ExecutionError> {
        self.witness_metadata_step()?;
        *visits = visits
            .checked_add(1)
            .filter(|work| *work <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_witness_work",
                    "owned method shape traversal exhausted proof admission",
                )
            })?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(reference_type_error(
                "owned method type exceeds structural depth",
            ));
        }
        let form = &self
            .schema
            .types
            .get(&ty)
            .ok_or_else(|| reference_type_error("missing owned method type metadata"))?
            .form;
        let (sequence, count) = match form {
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => return Ok(true),
            TypeForm::TypeParameter { parameter } => {
                return if declared.contains(parameter) {
                    Ok(true)
                } else {
                    Err(reference_type_error(
                        "method type parameter has a foreign scope",
                    ))
                };
            }
            TypeForm::OwnedSequence { .. } => (true, 1),
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                (false, fields.len())
            }
            _ => {
                return if self.ordinary_method_type(ty)? {
                    Ok(false)
                } else {
                    Err(reference_type_error(
                        "method type is not closed first-order data",
                    ))
                };
            }
        };
        self.charge_allocation((count * std::mem::size_of::<TypeObjectDigest>()) as u64)?;
        let children: Vec<_> = match &self.schema.types[&ty].form {
            TypeForm::OwnedSequence { item } => vec![*item],
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                fields.iter().map(|field| field.ty).collect()
            }
            _ => unreachable!(),
        };
        let mut owned = false;
        for child in children {
            let child_owned = self.method_shape(child, declared, depth + 1, visits)?;
            if sequence && !child_owned {
                return Err(reference_type_error(
                    "owned sequence requires an owned item",
                ));
            }
            owned |= child_owned;
        }
        if !owned {
            return Err(reference_type_error("owned composite has no owned member"));
        }
        Ok(true)
    }

    fn witness_metadata_step(&mut self) -> Result<(), ExecutionError> {
        self.control.check()?;
        self.observation.type_derivation_steps = self
            .observation
            .type_derivation_steps
            .checked_add(1)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_observation_overflow",
                    "static witness work observation overflowed",
                )
            })?;
        Ok(())
    }

    // Walk source nominal contents as well as structural types. Callable fields cannot hide
    // inside a closed named wrapper and make an owned method higher order.
    fn ordinary_method_type(&mut self, ty: TypeObjectDigest) -> Result<bool, ExecutionError> {
        // Independently establish the structural property under nominal formal
        // assumptions. Actual arguments remain children in the enclosing scope.
        type Context = BTreeSet<TypeParameterId>;
        self.charge_allocation(std::mem::size_of::<(TypeObjectDigest, Context)>() as u64)?;
        let mut pending = vec![(ty, Context::new())];
        let mut seen = BTreeSet::new();
        while let Some(key) = pending.pop() {
            self.witness_metadata_step()?;
            if seen.contains(&key) {
                continue;
            }
            let (ty, bindings) = key;
            self.charge_allocation(
                (std::mem::size_of::<(TypeObjectDigest, Context)>()
                    + 3 * std::mem::size_of::<usize>()
                    + bindings.len()
                        * (std::mem::size_of::<TypeParameterId>()
                            + 3 * std::mem::size_of::<usize>())) as u64,
            )?;
            seen.insert((ty, bindings.clone()));
            if seen.len() > crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK {
                return Err(reference_resource(
                    "normalized_reference_witness_work",
                    "method type traversal exhausted proof admission",
                ));
            }
            let object = self
                .schema
                .types
                .get(&ty)
                .ok_or_else(|| reference_type_error("missing ordinary method type"))?;
            let count = match &object.form {
                TypeForm::Function { .. }
                | TypeForm::TaskFunction { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
                | TypeForm::Secret
                | TypeForm::Stream { .. } => return Ok(false),
                TypeForm::StructuralRecord { fields } => fields.len(),
                TypeForm::Applied { arguments, .. } => arguments.len(),
                TypeForm::List { .. } | TypeForm::Option { .. } => 1,
                TypeForm::Result { .. } | TypeForm::Map { .. } => 2,
                _ => 0,
            };
            self.charge_allocation(
                ((2 * count + 1) * std::mem::size_of::<TypeObjectDigest>()) as u64,
            )?;
            let object = &self.schema.types[&ty];
            let children = object.child_types();
            // Only these small variants need retention across owner reads. Ordinary structural
            // fields have already contributed their digest children, without cloning names.
            let form = match &object.form {
                TypeForm::Named { declaration } => TypeForm::Named {
                    declaration: *declaration,
                },
                TypeForm::Applied {
                    declaration,
                    arguments,
                } => TypeForm::Applied {
                    declaration: *declaration,
                    arguments: arguments.clone(),
                },
                TypeForm::TypeParameter { parameter } => TypeForm::TypeParameter {
                    parameter: *parameter,
                },
                _ => TypeForm::Unit,
            };
            match form {
                TypeForm::Function { .. }
                | TypeForm::TaskFunction { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
                | TypeForm::Secret
                | TypeForm::Stream { .. } => return Ok(false),
                TypeForm::TypeParameter { parameter } => {
                    if !bindings.contains(&parameter) {
                        return Ok(false);
                    }
                }
                TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                    let arguments = if let TypeForm::Applied { arguments, .. } = form {
                        arguments
                    } else {
                        vec![]
                    };
                    let owner = self.declaration(declaration)?;
                    let (parameters, fields, cases) = match owner.payload {
                        DeclarationPayload::Record {
                            type_parameters,
                            fields,
                        } => (type_parameters, fields, vec![]),
                        DeclarationPayload::Variant {
                            type_parameters,
                            cases,
                        } => (type_parameters, vec![], cases),
                        _ => return Ok(false),
                    };
                    if parameters.len() != arguments.len() {
                        return Ok(false);
                    }
                    self.charge_allocation(
                        (parameters.len()
                            * (std::mem::size_of::<TypeParameterId>()
                                + 3 * std::mem::size_of::<usize>())) as u64,
                    )?;
                    let mut nested = Context::new();
                    for p in parameters {
                        self.witness_metadata_step()?;
                        let Some(OwnerRecord::TypeParameter(record)) =
                            self.owner_in_package(declaration.package, OwnerKey::TypeParameter(p))?
                        else {
                            return Ok(false);
                        };
                        if record.declaration != declaration.declaration
                            || record.constraints.has_owned()
                        {
                            return Ok(false);
                        }
                        nested.insert(p);
                    }
                    let mut members = vec![];
                    self.charge_allocation(
                        ((fields.len() + cases.len()) * std::mem::size_of::<TypeObjectDigest>())
                            as u64,
                    )?;
                    for field in fields {
                        self.witness_metadata_step()?;
                        let Some(OwnerRecord::Field(field)) =
                            self.owner_in_package(declaration.package, OwnerKey::Field(field))?
                        else {
                            return Ok(false);
                        };
                        if field.declaration != declaration.declaration {
                            return Ok(false);
                        }
                        members.push(field.ty);
                    }
                    for case in cases {
                        self.witness_metadata_step()?;
                        let Some(OwnerRecord::Case(case)) =
                            self.owner_in_package(declaration.package, OwnerKey::Case(case))?
                        else {
                            return Ok(false);
                        };
                        if case.declaration != declaration.declaration {
                            return Ok(false);
                        }
                        members.extend(case.payload);
                    }
                    for ty in members {
                        self.witness_metadata_step()?;
                        self.charge_allocation(
                            (std::mem::size_of::<(TypeObjectDigest, Context)>()
                                + nested.len()
                                    * (std::mem::size_of::<TypeParameterId>()
                                        + 3 * std::mem::size_of::<usize>()))
                                as u64,
                        )?;
                        pending.push((ty, nested.clone()));
                    }
                }
                _ => {}
            }
            for child in children {
                self.witness_metadata_step()?;
                self.charge_allocation(
                    (std::mem::size_of::<(TypeObjectDigest, Context)>()
                        + bindings.len()
                            * (std::mem::size_of::<TypeParameterId>()
                                + 3 * std::mem::size_of::<usize>())) as u64,
                )?;
                pending.push((child, bindings.clone()));
            }
        }
        Ok(true)
    }
}
