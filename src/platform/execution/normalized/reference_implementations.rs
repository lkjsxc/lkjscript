//! Static witness dispatch derived from canonical source, independently of prepared instances.
use super::*;
use crate::platform::kernel::{
    ImplementationOperand, OwnedImplementation, TypeParameterConstraints,
};
use crate::platform::semantic_id::ImplementationParameterId;

use std::collections::BTreeSet;

type Implementations = BTreeMap<ImplementationParameterId, DeclarationReference>;

impl ReferenceState<'_> {
    pub(super) fn resolve_implementation(
        &mut self,
        operand: ImplementationOperand,
    ) -> Result<DeclarationReference, ExecutionError> {
        self.control.check()?;
        match operand {
            ImplementationOperand::Concrete { implementation } => Ok(implementation),
            ImplementationOperand::Parameter {
                function,
                parameter,
            } => {
                let Some((scope, values)) = self.implementation_scopes.last() else {
                    return Err(reference_type_error("witness has no lexical scope"));
                };
                if *scope != function {
                    return Err(reference_type_error("witness belongs to another function"));
                }
                values
                    .get(&parameter)
                    .copied()
                    .ok_or_else(|| reference_type_error("unbound implementation witness"))
            }
        }
    }

    pub(super) fn checked_implementation(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<OwnedImplementation, ExecutionError> {
        let DeclarationPayload::OwnedImplementation(implementation) =
            self.declaration(reference)?.payload
        else {
            return Err(reference_type_error(
                "witness does not select an implementation owner",
            ));
        };
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
                            && p.use_mode != ParameterUse::Consume)
                    {
                        return Err(reference_type_error(
                            "owned method parameters require scoped pure borrowing or consumption",
                        ));
                    }
                } else if suffix || p.use_mode != ParameterUse::Unrestricted {
                    return Err(reference_type_error("method is not first order"));
                }
            }
            self.owned_method_type(method.result, &declared)?;
            let mut mapping = None;
            for candidate in &implementation.methods {
                self.witness_metadata_step()?;
                if candidate.method == method.id && mapping.replace(candidate.function).is_some() {
                    return Err(reference_type_error("duplicate method implementation"));
                }
            }
            let target =
                mapping.ok_or_else(|| reference_type_error("missing method implementation"))?;
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
                || !function.type_parameters.is_empty()
                || !function.effect_parameters.is_empty()
                || !function.requirement_parameters.is_empty()
                || !function.implementation_parameters.is_empty()
                || function.parameters.len() != method.parameters.len()
            {
                return Err(reference_type_error(
                    "method implementation must have exact monomorphic type, callable kind and effects",
                ));
            }
            let parameters = self.parameters(target.package, &function.parameters)?;
            for (actual, expected) in parameters.iter().zip(&method.parameters) {
                if actual.parent
                    != crate::platform::kernel::ParameterParent::Function(target.declaration)
                    || actual.ty != self.method_type(expected.ty, &substitutions)?
                    || actual.use_mode != expected.use_mode
                    || actual.resource_requirement.is_some()
                {
                    return Err(reference_type_error("method parameter contract mismatch"));
                }
            }
            if function.result != self.method_type(method.result, &substitutions)? {
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
        function: &FunctionDeclaration,
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        supplied: &[DeclarationReference],
    ) -> Result<Implementations, ExecutionError> {
        if function.implementation_parameters.len() != supplied.len() {
            return Err(reference_type_error(
                "missing or extra static implementation operands",
            ));
        }
        self.charge_allocation(
            (supplied.len()
                * std::mem::size_of::<(ImplementationParameterId, DeclarationReference)>())
                as u64,
        )?;
        let mut bindings = BTreeMap::new();
        for (parameter, selected) in function.implementation_parameters.iter().zip(supplied) {
            self.witness_metadata_step()?;
            let implementation = self.checked_implementation(*selected)?;
            let Some(TypeForm::TypeParameter {
                parameter: type_parameter,
            }) = self.schema.types.get(&parameter.self_type).map(|t| &t.form)
            else {
                return Err(reference_type_error(
                    "witness Self is not an exact type parameter",
                ));
            };
            if implementation.contract != parameter.contract
                || types.get(type_parameter) != Some(&implementation.self_type)
                || parameter.type_arguments.len() != implementation.type_arguments.len()
                || bindings.insert(parameter.id, *selected).is_some()
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
    ) -> Result<DeclarationReference, ExecutionError> {
        let selected = self.resolve_implementation(witness)?;
        let implementation = self.checked_implementation(selected)?;
        if implementation.contract != contract {
            return Err(reference_type_error("method uses another nominal contract"));
        }
        for m in &implementation.methods {
            self.witness_metadata_step()?;
            if m.method == method {
                return Ok(m.function);
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
            (operands.len() * std::mem::size_of::<DeclarationReference>()) as u64,
        )?;
        let mut selected = Vec::with_capacity(operands.len());
        for operand in operands {
            selected.push(self.resolve_implementation(*operand)?);
        }
        let types = self.resolve_type_arguments(types)?;
        let effects = self.resolve_effect_arguments(effects)?;
        let requirements = self.resolve_requirement_arguments(requirements)?;
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
                types: &types,
                effects: &effects,
                requirements: &requirements,
                implementations: &selected,
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
