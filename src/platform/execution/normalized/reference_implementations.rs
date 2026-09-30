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
        if !matches!(
            self.schema
                .types
                .get(&implementation.self_type)
                .map(|t| &t.form),
            Some(TypeForm::ByteBuffer | TypeForm::OwnedI64Cell)
        ) {
            return Err(reference_type_error("implementation has a non-owned Self"));
        }
        let DeclarationPayload::OwnedContract(contract) =
            self.declaration(implementation.contract)?.payload
        else {
            return Err(reference_type_error(
                "implementation names a foreign contract kind",
            ));
        };
        let Some(OwnerRecord::TypeParameter(parameter)) = self.owner_in_package(
            implementation.contract.package,
            OwnerKey::TypeParameter(contract.self_parameter),
        )?
        else {
            return Err(reference_type_error("missing contract Self owner"));
        };
        if parameter.declaration != implementation.contract.declaration
            || parameter.constraints != TypeParameterConstraints::Owned
            || contract.methods.len() != implementation.methods.len()
        {
            return Err(reference_type_error(
                "implementation does not satisfy the exact owned contract",
            ));
        }
        if contract.methods.is_empty() {
            return Err(reference_type_error("empty implementation methods"));
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
                || !matches!(method.effect, FunctionEffect::Pure)
            {
                return Err(reference_type_error(
                    "invalid method identity or callable kind",
                ));
            }
            let mut suffix = false;
            for p in &method.parameters {
                self.witness_metadata_step()?;
                if self.schema.types.get(&p.ty).map(|t| &t.form)
                    == Some(&TypeForm::TypeParameter {
                        parameter: contract.self_parameter,
                    })
                {
                    suffix = true;
                    if p.use_mode == ParameterUse::Unrestricted {
                        return Err(reference_type_error(
                            "Self method parameter is unrestricted",
                        ));
                    }
                } else if suffix
                    || p.use_mode != ParameterUse::Unrestricted
                    || !self.ordinary_method_type(p.ty)?
                {
                    return Err(reference_type_error("method is not first order"));
                }
            }
            if self.schema.types.get(&method.result).map(|t| &t.form)
                != Some(&TypeForm::TypeParameter {
                    parameter: contract.self_parameter,
                })
                && !self.ordinary_method_type(method.result)?
            {
                return Err(reference_type_error("method result is not first order"));
            }
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
            if !matches!(function.effect, FunctionEffect::Pure)
                || !function.type_parameters.is_empty()
                || !function.effect_parameters.is_empty()
                || !function.requirement_parameters.is_empty()
                || !function.implementation_parameters.is_empty()
                || function.parameters.len() != method.parameters.len()
            {
                return Err(reference_type_error(
                    "method implementation must be exact monomorphic pure code",
                ));
            }
            let parameters = self.parameters(target.package, &function.parameters)?;
            for (actual, expected) in parameters.iter().zip(&method.parameters) {
                if actual.parent
                    != crate::platform::kernel::ParameterParent::Function(target.declaration)
                    || actual.ty
                        != self.method_type(
                            expected.ty,
                            contract.self_parameter,
                            implementation.self_type,
                        )?
                    || actual.use_mode != expected.use_mode
                    || actual.resource_requirement.is_some()
                {
                    return Err(reference_type_error("method parameter contract mismatch"));
                }
            }
            if function.result
                != self.method_type(
                    method.result,
                    contract.self_parameter,
                    implementation.self_type,
                )?
            {
                return Err(reference_type_error("method result contract mismatch"));
            }
        }
        Ok(implementation)
    }

    pub(super) fn method_type(
        &self,
        ty: TypeObjectDigest,
        parameter: TypeParameterId,
        self_type: TypeObjectDigest,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        let form = &self
            .schema
            .types
            .get(&ty)
            .ok_or_else(|| reference_type_error("missing method type metadata"))?
            .form;
        Ok(if *form == (TypeForm::TypeParameter { parameter }) {
            self_type
        } else {
            ty
        })
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
                || bindings.insert(parameter.id, *selected).is_some()
            {
                return Err(reference_type_error(
                    "static witness contract or instantiated Self mismatch",
                ));
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
                implementations: &selected,
                ..Default::default()
            },
        )
    }

    pub(super) fn execute_witness_call(
        &mut self,
        target: AdmittedGraphCall,
    ) -> Result<CheckedValue, ExecutionError> {
        if self.call_depth >= self.policy.maximum_call_depth {
            return Err(reference_resource(
                "normalized_reference_call_depth",
                "witness call exceeds call-depth budget",
            ));
        }
        self.call_depth += 1;
        self.observation.maximum_call_depth =
            self.observation.maximum_call_depth.max(self.call_depth);
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
                            || record.constraints == TypeParameterConstraints::Owned
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
