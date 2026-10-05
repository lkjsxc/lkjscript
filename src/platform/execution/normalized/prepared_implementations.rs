//! Finite closure of callable types and exact applied static witnesses.
//! A caller's type scope is resolved before any witness crosses a call boundary.
use super::super::prepare::{
    NormalizedFunction, NormalizedImplementationArgument, NormalizedImplementationConstraint,
};
use super::*;
use crate::platform::compiler::{CompilationPayload, CompilationUnit};
use crate::platform::kernel::{
    DeclarationReference, ImplementationOperand, OwnedImplementation, OwnerKey, PackageId,
};
use crate::platform::semantic_id::ImplementationParameterId;
use std::sync::Arc;

#[derive(Clone, Debug)]
struct AppliedWitness {
    identity: u32,
    depth: usize,
    implementation: DeclarationReference,
    type_arguments: Vec<TypeObjectDigest>,
    implementations: Arc<[AppliedWitness]>,
}

impl PartialEq for AppliedWitness {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}
impl Eq for AppliedWitness {}
impl PartialOrd for AppliedWitness {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AppliedWitness {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identity.cmp(&other.identity)
    }
}

type WitnessIdentity = (DeclarationReference, Vec<TypeObjectDigest>, Vec<u32>);

type Application = (FunctionIndex, Vec<TypeObjectDigest>, Vec<AppliedWitness>);
type Bindings = BTreeMap<ImplementationParameterId, AppliedWitness>;

struct Closing<'a, 'b> {
    templates: Arc<[NormalizedFunction]>,
    functions: Vec<NormalizedFunction>,
    implementations: BTreeMap<DeclarationReference, &'a OwnedImplementation>,
    targets: BTreeMap<DeclarationReference, FunctionIndex>,
    instances: BTreeMap<Application, FunctionIndex>,
    normalized: BTreeMap<AppliedWitness, NormalizedImplementationArgument>,
    witnesses: BTreeMap<WitnessIdentity, u32>,
    pending: Vec<(Application, FunctionIndex)>,
    types: &'a mut BTreeMap<TypeObjectDigest, TypeObject>,
    work: &'a mut Budget<'b>,
}

impl Closing<'_, '_> {
    fn arguments(
        &mut self,
        arguments: &[TypeObjectDigest],
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        self.work.reserve::<TypeObjectDigest>(arguments.len())?;
        arguments
            .iter()
            .map(|ty| substitute(self.types, *ty, bindings, 0, self.work))
            .collect()
    }

    fn reserve_witness(&mut self, witness: &AppliedWitness) -> Result<(), Diagnostic> {
        self.work
            .reserve::<TypeObjectDigest>(witness.type_arguments.len())
    }

    fn reserve_operand(
        &mut self,
        operand: &ImplementationOperand,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        step(self.work)?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(missing());
        }
        if let ImplementationOperand::Concrete {
            type_arguments,
            implementations,
            ..
        } = operand
        {
            self.work
                .reserve::<TypeObjectDigest>(type_arguments.len())?;
            self.work
                .reserve::<ImplementationOperand>(implementations.len())?;
            for child in implementations {
                self.reserve_operand(child, depth + 1)?;
            }
        }
        Ok(())
    }

    fn operand(
        &mut self,
        operand: &ImplementationOperand,
        scope: Option<DeclarationReference>,
        bindings: &Bindings,
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<AppliedWitness, Diagnostic> {
        self.operand_at(operand, scope, bindings, types, 0)
    }

    fn operand_at(
        &mut self,
        operand: &ImplementationOperand,
        scope: Option<DeclarationReference>,
        bindings: &Bindings,
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Result<AppliedWitness, Diagnostic> {
        step(self.work)?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(missing());
        }
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let scheme = self
                    .implementations
                    .get(implementation)
                    .ok_or_else(missing)?;
                if scheme.type_parameters.len() != type_arguments.len()
                    || scheme.implementation_parameters.len() != implementations.len()
                {
                    return Err(missing());
                }
                let arguments = self.arguments(type_arguments, types)?;
                let mut closed = BTreeMap::new();
                for ty in &arguments {
                    if !matches!(
                        self.types.get(ty).map(|t| &t.form),
                        Some(
                            TypeForm::ByteBuffer
                                | TypeForm::OwnedI64Cell
                                | TypeForm::OwnedProduct { .. }
                                | TypeForm::OwnedChoice { .. }
                                | TypeForm::OwnedSequence { .. }
                        )
                    ) || !instantiated_type(self.types, *ty, &mut closed, 0, self.work)?
                    {
                        return Err(missing());
                    }
                }
                self.work.reserve::<AppliedWitness>(implementations.len())?;
                self.work.reserve::<usize>(2)?;
                let prerequisites = implementations
                    .iter()
                    .map(|operand| self.operand_at(operand, scope, bindings, types, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?;
                let subtree_depth = prerequisites
                    .iter()
                    .map(|witness| witness.depth.saturating_add(1))
                    .max()
                    .unwrap_or(0);
                if depth.saturating_add(subtree_depth)
                    > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
                {
                    return Err(missing());
                }
                self.work.reserve::<TypeObjectDigest>(arguments.len())?;
                self.work.reserve::<u32>(prerequisites.len())?;
                let key = (
                    *implementation,
                    arguments.clone(),
                    prerequisites.iter().map(|w| w.identity).collect(),
                );
                let identity = match self.witnesses.get(&key) {
                    Some(identity) => *identity,
                    None => {
                        self.work.node::<(WitnessIdentity, u32)>()?;
                        let identity =
                            u32::try_from(self.witnesses.len()).map_err(|_| missing())?;
                        self.witnesses.insert(key, identity);
                        identity
                    }
                };
                let selected = AppliedWitness {
                    identity,
                    depth: subtree_depth,
                    implementation: *implementation,
                    type_arguments: arguments,
                    implementations: prerequisites.into(),
                };
                self.normalize_witness(&selected, depth)?;
                Ok(selected)
            }
            ImplementationOperand::Parameter {
                scope: parameter_scope,
                parameter,
            } => {
                if Some(*parameter_scope) != scope {
                    return Err(missing());
                }
                let witness = bindings.get(parameter).ok_or_else(missing)?;
                if depth.saturating_add(witness.depth)
                    > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
                {
                    return Err(missing());
                }
                self.reserve_witness(witness)?;
                Ok(witness.clone())
            }
        }
    }

    fn normalize_witness(
        &mut self,
        witness: &AppliedWitness,
        depth: usize,
    ) -> Result<NormalizedImplementationArgument, Diagnostic> {
        step(self.work)?;
        if depth.saturating_add(witness.depth)
            > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
        {
            return Err(missing());
        }
        if let Some(application) = self.normalized.get(witness) {
            return Ok(application.clone());
        }
        let implementation = *self
            .implementations
            .get(&witness.implementation)
            .ok_or_else(missing)?;
        if implementation.type_parameters.len() != witness.type_arguments.len()
            || implementation.implementation_parameters.len() != witness.implementations.len()
        {
            return Err(missing());
        }
        self.work
            .reserve::<(TypeParameterId, TypeObjectDigest)>(witness.type_arguments.len())?;
        let types = implementation
            .type_parameters
            .iter()
            .copied()
            .zip(witness.type_arguments.iter().copied())
            .collect();
        let self_type = substitute(self.types, implementation.self_type, &types, 0, self.work)?;
        let type_arguments = self.arguments(&implementation.type_arguments, &types)?;
        self.work
            .reserve::<NormalizedImplementationArgument>(witness.implementations.len())?;
        self.work
            .reserve::<NormalizedImplementationConstraint>(witness.implementations.len())?;
        self.work.reserve::<usize>(8)?;
        let mut prerequisites = Vec::new();
        let mut obligations = Vec::new();
        for (parameter, child) in implementation
            .implementation_parameters
            .iter()
            .zip(witness.implementations.iter())
        {
            let application = self.normalize_witness(child, depth + 1)?;
            self.match_parameter(parameter, &application, &types)?;
            let self_type = substitute(self.types, parameter.self_type, &types, 0, self.work)?;
            let type_arguments = self.arguments(&parameter.type_arguments, &types)?;
            self.work.reserve::<usize>(2)?;
            obligations.push(NormalizedImplementationConstraint {
                contract: parameter.contract,
                self_type,
                type_arguments: type_arguments.into(),
            });
            prerequisites.push(application);
        }
        self.work
            .reserve::<TypeObjectDigest>(witness.type_arguments.len())?;
        let application = NormalizedImplementationArgument {
            identity: witness.identity,
            depth: witness.depth,
            implementation: witness.implementation,
            contract: implementation.contract,
            implementation_type_arguments: witness.type_arguments.clone().into(),
            self_type,
            type_arguments: type_arguments.into(),
            implementations: prerequisites.into(),
            prerequisites: obligations.into(),
        };
        self.work
            .node::<(AppliedWitness, NormalizedImplementationArgument)>()?;
        self.reserve_witness(witness)?;
        self.normalized.insert(witness.clone(), application.clone());
        Ok(application)
    }

    fn match_parameter(
        &mut self,
        parameter: &crate::platform::kernel::ImplementationParameter,
        application: &NormalizedImplementationArgument,
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<(), Diagnostic> {
        step(self.work)?;
        if parameter.contract != application.contract
            || substitute(self.types, parameter.self_type, types, 0, self.work)?
                != application.self_type
            || parameter.type_arguments.len() != application.type_arguments.len()
        {
            return Err(missing());
        }
        for (expected, actual) in parameter
            .type_arguments
            .iter()
            .zip(application.type_arguments.iter())
        {
            if substitute(self.types, *expected, types, 0, self.work)? != *actual {
                return Err(missing());
            }
        }
        Ok(())
    }

    fn application(
        &mut self,
        function: FunctionIndex,
        types: Vec<TypeObjectDigest>,
        implementations: Vec<AppliedWitness>,
    ) -> Result<FunctionIndex, Diagnostic> {
        step(self.work)?;
        let templates = Arc::clone(&self.templates);
        let template = templates.get(function.0 as usize).ok_or_else(missing)?;
        if template.implementation_parameters.len() != implementations.len()
            || template.type_parameters.len() != types.len()
        {
            return Err(missing());
        }
        let mut closed = BTreeMap::new();
        for ty in &types {
            if !instantiated_type(self.types, *ty, &mut closed, 0, self.work)? {
                return Err(missing());
            }
        }
        let key = (function, types, implementations);
        if let Some(index) = self.instances.get(&key) {
            return Ok(*index);
        }
        self.work
            .reserve::<NormalizedImplementationArgument>(key.2.len())?;
        self.work
            .reserve::<(TypeParameterId, TypeObjectDigest)>(key.1.len())?;
        let function_bindings: BTreeMap<_, _> = template
            .type_parameters
            .iter()
            .copied()
            .zip(key.1.iter().copied())
            .collect();
        let mut supplied = Vec::new();
        for (p, witness) in template.implementation_parameters.iter().zip(&key.2) {
            let application = self.normalize_witness(witness, 0)?;
            self.match_parameter(p, &application, &function_bindings)?;
            supplied.push(application);
        }
        self.work.node::<(Application, FunctionIndex)>()?;
        self.work.node::<(Application, FunctionIndex)>()?;
        self.work.reserve::<TypeObjectDigest>(key.1.len())?;
        self.work.reserve::<AppliedWitness>(key.2.len())?;
        for witness in &key.2 {
            self.work
                .reserve::<TypeObjectDigest>(witness.type_arguments.len())?;
        }
        let index = if key.1.is_empty() && key.2.is_empty() {
            function
        } else {
            self.work.reserve::<NormalizedFunction>(1)?;
            let index = FunctionIndex(
                u32::try_from(self.functions.len()).map_err(|_| missing())?,
                function.1,
            );
            self.work.cloned_effect(&template.effect)?;
            if let NormalizedFunctionBody::External(name) = &template.body {
                self.work.reserve::<u8>(name.as_str().len())?;
            }
            let mut instance = template.clone();
            self.work.reserve::<TypeObjectDigest>(key.1.len())?;
            self.work.reserve::<usize>(2)?;
            instance.type_arguments = key.1.clone().into();
            instance.implementation_arguments = supplied.into();
            self.functions.push(instance);
            index
        };
        self.instances.insert(key.clone(), index);
        self.pending.push((key, index));
        Ok(index)
    }

    fn code(
        &mut self,
        code: &mut NormalizedCode,
        scope: Option<DeclarationReference>,
        bindings: &Bindings,
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<(), Diagnostic> {
        self.work
            .reserve::<NormalizedInstruction>(code.instructions.len())?;
        // Concrete witness operands own vectors, unlike the Arc-backed call operands.
        // Reserve their copies before Arc::make_mut can clone an instruction array.
        for instruction in code.instructions.iter() {
            step(self.work)?;
            if let NormalizedInstruction::MethodCall { witness, .. } = instruction {
                self.reserve_operand(witness, 0)?;
            }
        }
        for instruction in Arc::make_mut(&mut code.instructions) {
            step(self.work)?;
            match instruction {
                NormalizedInstruction::Parallel {
                    left,
                    left_types,
                    left_implementations,
                    right,
                    right_types,
                    right_implementations,
                    ..
                } => {
                    for (function, arguments, implementations) in [
                        (left, left_types, left_implementations),
                        (right, right_types, right_implementations),
                    ] {
                        self.work.reserve::<AppliedWitness>(implementations.len())?;
                        let mut selected = Vec::new();
                        for operand in implementations.iter() {
                            selected.push(self.operand(operand, scope, bindings, types)?);
                        }
                        let applied = self.arguments(arguments, types)?;
                        self.work.reserve::<TypeObjectDigest>(applied.len())?;
                        self.work.reserve::<usize>(2)?;
                        *arguments = Arc::from(applied.clone());
                        *function = self.application(*function, applied, selected)?;
                        *implementations = Arc::from([]);
                    }
                }
                NormalizedInstruction::ImplementationCall {
                    effect_arguments,
                    requirement_arguments,
                    function,
                    implementations,
                    type_arguments,
                    arguments,
                } => {
                    self.work.reserve::<AppliedWitness>(implementations.len())?;
                    let mut selected = Vec::new();
                    for operand in implementations.iter() {
                        selected.push(self.operand(operand, scope, bindings, types)?);
                    }
                    let applied = self.arguments(type_arguments, types)?;
                    self.work.reserve::<TypeObjectDigest>(applied.len())?;
                    self.work.reserve::<usize>(2)?;
                    let function = self.application(*function, applied.clone(), selected)?;
                    *instruction = NormalizedInstruction::Call {
                        function,
                        type_arguments: Arc::from(applied),
                        arguments: *arguments,
                        effect_arguments: Arc::clone(effect_arguments),
                        requirement_arguments: Arc::clone(requirement_arguments),
                    };
                }
                NormalizedInstruction::MethodCall {
                    witness,
                    contract,
                    method,
                    arguments,
                } => {
                    let selected = self.operand(witness, scope, bindings, types)?;
                    let implementation = *self
                        .implementations
                        .get(&selected.implementation)
                        .ok_or_else(missing)?;
                    if implementation.contract != *contract {
                        return Err(missing());
                    }
                    let mut mapping = None;
                    for candidate in &implementation.methods {
                        step(self.work)?;
                        if candidate.method == *method {
                            mapping = Some(candidate);
                            break;
                        }
                    }
                    let mapping = mapping.ok_or_else(missing)?;
                    let target = *self.targets.get(&mapping.function).ok_or_else(missing)?;
                    self.work
                        .reserve::<TypeObjectDigest>(mapping.type_arguments.len())?;
                    let mapped_types = mapping.type_arguments.clone();
                    self.work
                        .reserve::<ImplementationOperand>(mapping.implementations.len())?;
                    for operand in &mapping.implementations {
                        self.reserve_operand(operand, 0)?;
                    }
                    let mapped_implementations = mapping.implementations.clone();
                    self.work.reserve::<(TypeParameterId, TypeObjectDigest)>(
                        selected.type_arguments.len(),
                    )?;
                    let implementation_bindings = implementation
                        .type_parameters
                        .iter()
                        .copied()
                        .zip(selected.type_arguments.iter().copied())
                        .collect();
                    self.work
                        .reserve::<(ImplementationParameterId, AppliedWitness)>(
                            selected.implementations.len(),
                        )?;
                    for child in selected.implementations.iter() {
                        step(self.work)?;
                        self.work.reserve::<usize>(3)?;
                        self.reserve_witness(child)?;
                    }
                    let implementation_witnesses: Bindings = implementation
                        .implementation_parameters
                        .iter()
                        .map(|p| p.id)
                        .zip(selected.implementations.iter().cloned())
                        .collect();
                    let applied = self.arguments(&mapped_types, &implementation_bindings)?;
                    self.work
                        .reserve::<AppliedWitness>(mapped_implementations.len())?;
                    let mut prerequisites = Vec::new();
                    for operand in &mapped_implementations {
                        prerequisites.push(self.operand(
                            operand,
                            Some(selected.implementation),
                            &implementation_witnesses,
                            &implementation_bindings,
                        )?);
                    }
                    self.work.reserve::<TypeObjectDigest>(applied.len())?;
                    self.work.reserve::<usize>(2)?;
                    let function = self.application(target, applied.clone(), prerequisites)?;
                    *instruction = NormalizedInstruction::Call {
                        function,
                        type_arguments: Arc::from(applied),
                        arguments: *arguments,
                        effect_arguments: Arc::from([]),
                        requirement_arguments: Arc::from([]),
                    };
                }
                NormalizedInstruction::Call {
                    function,
                    type_arguments,
                    ..
                }
                | NormalizedInstruction::TailCall {
                    function,
                    type_arguments,
                    ..
                }
                | NormalizedInstruction::FunctionValue {
                    function,
                    type_arguments,
                    ..
                } => {
                    let applied = self.arguments(type_arguments, types)?;
                    self.work.reserve::<TypeObjectDigest>(applied.len())?;
                    self.work.reserve::<usize>(2)?;
                    *type_arguments = Arc::from(applied.clone());
                    *function = self.application(*function, applied, Vec::new())?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn code_requires_specialization(
    code: &NormalizedCode,
    work: &mut Budget<'_>,
) -> Result<bool, Diagnostic> {
    for instruction in code.instructions.iter() {
        step(work)?;
        if let NormalizedInstruction::Parallel {
            left_types,
            left_implementations,
            right_types,
            right_implementations,
            ..
        } = instruction
            && (!left_types.is_empty()
                || !right_types.is_empty()
                || !left_implementations.is_empty()
                || !right_implementations.is_empty())
        {
            return Ok(true);
        }
        if matches!(
            instruction,
            NormalizedInstruction::ImplementationCall { .. }
                | NormalizedInstruction::MethodCall { .. }
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn requires_specialization(
    program: &NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<bool, Diagnostic> {
    step(work)?;
    for function in program.functions.iter() {
        step(work)?;
        if !function.implementation_parameters.is_empty()
            || !function.implementation_arguments.is_empty()
        {
            return Ok(true);
        }
        if let NormalizedFunctionBody::Code(code) = &function.body
            && code_requires_specialization(code, work)?
        {
            return Ok(true);
        }
    }
    for test in program.tests.values() {
        step(work)?;
        if code_requires_specialization(&test.actual, work)?
            || code_requires_specialization(&test.expected, work)?
        {
            return Ok(true);
        }
    }
    for port in program.ports.iter() {
        step(work)?;
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &port.entry
            && code_requires_specialization(code, work)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn close(
    program: &mut NormalizedProgram,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    // Whole-artifact type, affine and implementation admission already happened.
    // Prove only that this derived pass has no work, including unreachable code
    // and non-function roots. Do not clone ordinary code or spend its allocation
    // budget constructing empty dispatch state. Cancellation and work bounds remain.
    if !requires_specialization(program, work)? {
        return Ok(());
    }
    let mut implementations = BTreeMap::new();
    for ((package, owner), unit) in units {
        step(work)?;
        if let CompilationPayload::OwnedImplementation(i) = &unit.payload {
            let OwnerKey::Declaration(declaration) = owner else {
                return Err(missing());
            };
            work.node::<(DeclarationReference, &OwnedImplementation)>()?;
            implementations.insert(
                DeclarationReference {
                    package: *package,
                    declaration: *declaration,
                },
                i,
            );
        }
    }
    work.reserve::<NormalizedFunction>(program.functions.len())?;
    for function in program.functions.iter() {
        step(work)?;
        work.cloned_effect(&function.effect)?;
        if let NormalizedFunctionBody::External(name) = &function.body {
            work.reserve::<u8>(name.as_str().len())?;
        }
    }
    let mut targets = BTreeMap::new();
    for (i, f) in program.functions.iter().enumerate() {
        step(work)?;
        work.node::<(DeclarationReference, FunctionIndex)>()?;
        targets.insert(
            f.declaration,
            FunctionIndex(
                u32::try_from(i).map_err(|_| missing())?,
                program.value_origin,
            ),
        );
    }
    let mut closing = Closing {
        templates: Arc::clone(&program.functions),
        functions: program.functions.to_vec(),
        implementations,
        targets,
        instances: BTreeMap::new(),
        normalized: BTreeMap::new(),
        witnesses: BTreeMap::new(),
        pending: Vec::new(),
        types: &mut program.types,
        work,
    };
    for i in 0..closing.templates.len() {
        if closing.templates[i].implementation_parameters.is_empty()
            && closing.templates[i].type_parameters.is_empty()
            && closing.templates[i].effect_parameters.is_empty()
            && closing.templates[i].requirement_parameters.is_empty()
        {
            closing.application(
                FunctionIndex(
                    u32::try_from(i).map_err(|_| missing())?,
                    program.value_origin,
                ),
                Vec::new(),
                Vec::new(),
            )?;
        }
    }
    for test in program.tests.values_mut() {
        closing.code(&mut test.actual, None, &Bindings::new(), &BTreeMap::new())?;
        closing.code(&mut test.expected, None, &Bindings::new(), &BTreeMap::new())?;
    }
    closing
        .work
        .reserve::<super::super::prepare::NormalizedPort>(program.ports.len())?;
    for port in Arc::make_mut(&mut program.ports) {
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &mut port.entry
        {
            closing.code(code, None, &Bindings::new(), &BTreeMap::new())?;
        }
    }
    while let Some(((template, arguments, selected), index)) = closing.pending.pop() {
        step(closing.work)?;
        let function = &closing.templates[template.0 as usize];
        closing
            .work
            .reserve::<(ImplementationParameterId, AppliedWitness)>(selected.len())?;
        closing
            .work
            .reserve::<(TypeParameterId, TypeObjectDigest)>(arguments.len())?;
        let type_bindings = function
            .type_parameters
            .iter()
            .copied()
            .zip(arguments)
            .collect();
        let bindings = function
            .implementation_parameters
            .iter()
            .map(|p| p.id)
            .zip(selected)
            .collect();
        let scope = function.declaration;
        if let NormalizedFunctionBody::Code(mut code) = function.body.clone() {
            closing.code(&mut code, Some(scope), &bindings, &type_bindings)?;
            closing.functions[index.0 as usize].body = NormalizedFunctionBody::Code(code);
        }
    }
    closing
        .work
        .reserve::<NormalizedImplementationArgument>(closing.normalized.len())?;
    closing.work.reserve::<usize>(2)?;
    let mut applications = Vec::with_capacity(closing.normalized.len());
    for (witness, application) in &closing.normalized {
        step(closing.work)?;
        if witness.identity as usize != applications.len() {
            return Err(missing());
        }
        applications.push(application.clone());
    }
    program.implementation_applications = applications.into();
    program.functions = closing.functions.into();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::normalized::tests::owned_implementation_scheme_tests::duplicate_dag_fixture;

    fn standard() -> NormalizedProgram {
        let artifact = crate::platform::compiler::load_artifact(include_bytes!(
            "../../../../packages/standard/generated/standard.lkja"
        ))
        .unwrap();
        NormalizedProgram::prepare(artifact).unwrap()
    }

    #[test]
    fn owned_witness_absence_scan_remains_bounded_and_cancellable_before_mutation() {
        let base = standard();
        for cancelled in [false, true] {
            let mut program = base.clone();
            let control = crate::platform::execution::ExecutionControl::uncancelled();
            if cancelled {
                control.cancel();
            }
            let mut work = Budget {
                steps: MAXIMUM_WORK - 1,
                bytes: 0,
                control: &control,
            };
            let error = close(&mut program, &BTreeMap::new(), &mut work).unwrap_err();
            if cancelled {
                assert_eq!(error.class, DiagnosticClass::Cancelled);
                assert_eq!(work.steps, MAXIMUM_WORK - 1);
            } else {
                assert_eq!(error.code, "normalized_instantiation_work");
                assert_eq!(work.steps, MAXIMUM_WORK);
            }
            assert_eq!(work.bytes, 0);
            assert!(Arc::ptr_eq(&base.functions, &program.functions));
            assert!(Arc::ptr_eq(&base.ports, &program.ports));
        }
    }

    #[test]
    fn owned_witness_absence_scan_covers_unreachable_code_and_every_root() {
        use crate::platform::execution::normalized::{
            prepare::NormalizedPort, value::ComponentIndex,
        };
        use crate::platform::kernel::PortReference;
        use crate::platform::semantic_id::{MethodId, PortId};
        let base = standard();
        let reference = base.functions[0].declaration;
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        for instruction in [
            NormalizedInstruction::ImplementationCall {
                function: FunctionIndex(0, base.value_origin),
                type_arguments: Arc::from([]),
                effect_arguments: Arc::from([]),
                requirement_arguments: Arc::from([]),
                implementations: Arc::from([]),
                arguments: 0,
            },
            NormalizedInstruction::MethodCall {
                witness: ImplementationOperand::Concrete {
                    implementation: reference,
                    type_arguments: Vec::new(),
                    implementations: Vec::new(),
                },
                contract: reference,
                method: MethodId::migrate(b"witness-absence-probe", 0),
                arguments: 0,
            },
        ] {
            let code = NormalizedCode {
                parameter_count: 0,
                local_count: 0,
                instructions: Arc::from([
                    NormalizedInstruction::Jump(2),
                    instruction,
                    NormalizedInstruction::Unit,
                ]),
            };
            for root in 0..5 {
                let mut program = base.clone();
                match root {
                    0 => {
                        Arc::make_mut(&mut program.functions)[0].body =
                            NormalizedFunctionBody::Code(code.clone())
                    }
                    1 => program.tests.values_mut().next().unwrap().actual = code.clone(),
                    2 => program.tests.values_mut().next().unwrap().expected = code.clone(),
                    _ => {
                        let ty = program.functions[0].result;
                        program.ports = Arc::from([NormalizedPort {
                            reference: PortReference {
                                package: program.root_package,
                                port: PortId::migrate(b"witness-absence-probe", 0),
                            },
                            name: crate::platform::kernel::Name::new("probe").unwrap(),
                            function_type: ty,
                            component: ComponentIndex(0),
                            entry: if root == 3 {
                                NormalizedEntryPoint::Code(code.clone())
                            } else {
                                NormalizedEntryPoint::PortExpression(code.clone(), ty)
                            },
                        }]);
                    }
                }
                let mut work = Budget::new(&control);
                assert!(
                    requires_specialization(&program, &mut work).unwrap(),
                    "root {root}"
                );
                assert_eq!(work.bytes, 0);
            }
        }
        for selected in [false, true] {
            let mut program = base.clone();
            let f = &mut Arc::make_mut(&mut program.functions)[0];
            if selected {
                f.implementation_arguments = Arc::from([NormalizedImplementationArgument {
                    identity: 0,
                    depth: 0,
                    implementation: reference,
                    contract: reference,
                    implementation_type_arguments: Arc::from([]),
                    self_type: f.result,
                    type_arguments: Arc::from([]),
                    implementations: Arc::from([]),
                    prerequisites: Arc::from([]),
                }]);
            } else {
                f.implementation_parameters =
                    Arc::from([crate::platform::kernel::ImplementationParameter {
                        id: ImplementationParameterId::migrate(b"witness-absence-probe", 0),
                        name: crate::platform::kernel::Name::new("ops").unwrap(),
                        contract: reference,
                        self_type: f.result,
                        type_arguments: Vec::new(),
                    }]);
            }
            assert!(requires_specialization(&program, &mut Budget::new(&control)).unwrap());
        }
    }

    #[test]
    fn owned_witness_absent_pass_preserves_ordinary_code_and_allocation_budget() {
        let mut program = standard();
        let before = program.clone();
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let mut work = Budget::new(&control);
        close(&mut program, &BTreeMap::new(), &mut work).unwrap();
        assert_eq!(
            work.bytes, 0,
            "absence must not allocate specialization state"
        );
        assert!(
            work.steps > 0,
            "absence must be established by bounded reads"
        );
        assert!(Arc::ptr_eq(&before.functions, &program.functions));
        assert!(Arc::ptr_eq(&before.ports, &program.ports));
        for (old, new) in before.functions.iter().zip(program.functions.iter()) {
            if let (NormalizedFunctionBody::Code(a), NormalizedFunctionBody::Code(b)) =
                (&old.body, &new.body)
            {
                assert!(Arc::ptr_eq(&a.instructions, &b.instructions));
            }
        }
        for (key, old) in &before.tests {
            let new = &program.tests[key];
            assert!(Arc::ptr_eq(
                &old.actual.instructions,
                &new.actual.instructions
            ));
            assert!(Arc::ptr_eq(
                &old.expected.instructions,
                &new.expected.instructions
            ));
        }
    }

    #[test]
    fn preparation_interns_depth_24_duplicate_edges_by_exact_child_identity() {
        use crate::platform::kernel::{DeclarationPayload, OwnerRecord};
        let (source, mut program) = duplicate_dag_fixture(1);
        let named = |name: &str| {
            source
                .owners
                .iter()
                .find_map(|(owner, record)| match (owner, record) {
                    (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                        if record.name.as_str() == name =>
                    {
                        Some(DeclarationReference {
                            package: source.root.package_id,
                            declaration: *declaration,
                        })
                    }
                    _ => None,
                })
                .unwrap()
        };
        let both = named("Both");
        let plain = named("Plain");
        let scope = named("dag-0");
        let parameter = program
            .functions
            .iter()
            .find(|function| function.declaration == scope)
            .unwrap()
            .implementation_parameters[0]
            .id;
        let cell = program
            .implementation_applications
            .iter()
            .find(|node| node.implementation == both)
            .unwrap()
            .self_type;
        let implementations = source
            .owners
            .iter()
            .filter_map(|(owner, record)| {
                let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) =
                    (owner, record)
                else {
                    return None;
                };
                let DeclarationPayload::OwnedImplementation(implementation) = &record.payload
                else {
                    return None;
                };
                Some((
                    DeclarationReference {
                        package: source.root.package_id,
                        declaration: *declaration,
                    },
                    implementation,
                ))
            })
            .collect();
        let control = crate::platform::execution::ExecutionControl::uncancelled();
        let mut work = Budget::new(&control);
        let mut closing = Closing {
            templates: Arc::from([]),
            functions: Vec::new(),
            implementations,
            targets: BTreeMap::new(),
            instances: BTreeMap::new(),
            normalized: BTreeMap::new(),
            witnesses: BTreeMap::new(),
            pending: Vec::new(),
            types: &mut program.types,
            work: &mut work,
        };
        let mut selected = closing
            .operand(
                &ImplementationOperand::Concrete {
                    implementation: plain,
                    type_arguments: vec![cell],
                    implementations: Vec::new(),
                },
                None,
                &Bindings::new(),
                &BTreeMap::new(),
            )
            .unwrap();
        let operand = ImplementationOperand::Concrete {
            implementation: both,
            type_arguments: vec![cell],
            implementations: vec![ImplementationOperand::Parameter { scope, parameter }; 2],
        };
        for _ in 0..24 {
            let bindings = Bindings::from([(parameter, selected)]);
            selected = closing
                .operand(&operand, Some(scope), &bindings, &BTreeMap::new())
                .unwrap();
            let repeated = closing
                .operand(&operand, Some(scope), &bindings, &BTreeMap::new())
                .unwrap();
            assert_eq!(selected.identity, repeated.identity);
            assert_eq!(
                selected.implementations[0].identity,
                selected.implementations[1].identity
            );
        }
        assert_eq!(closing.witnesses.len(), 25);
        assert_eq!(closing.normalized.len(), 25);
        assert!(closing.work.steps < 4096);
        assert!(closing.work.bytes < 128 * 1024);

        let changed = ImplementationOperand::Concrete {
            implementation: both,
            type_arguments: vec![cell],
            implementations: vec![
                ImplementationOperand::Parameter { scope, parameter },
                ImplementationOperand::Concrete {
                    implementation: named("CellPlus"),
                    type_arguments: Vec::new(),
                    implementations: Vec::new(),
                },
            ],
        };
        let different = closing
            .operand(
                &changed,
                Some(scope),
                &Bindings::from([(parameter, selected.clone())]),
                &BTreeMap::new(),
            )
            .unwrap();
        assert_ne!(different.identity, selected.identity);
        assert_ne!(
            different.implementations[0].identity,
            different.implementations[1].identity
        );
        let maximum = crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH;
        for _ in 24..maximum {
            let bindings = Bindings::from([(parameter, selected)]);
            selected = closing
                .operand(&operand, Some(scope), &bindings, &BTreeMap::new())
                .unwrap();
        }
        assert_eq!(selected.depth, maximum);
        let before = (closing.witnesses.len(), closing.normalized.len());
        assert!(
            closing
                .operand(
                    &operand,
                    Some(scope),
                    &Bindings::from([(parameter, selected)]),
                    &BTreeMap::new()
                )
                .is_err()
        );
        assert_eq!(
            before,
            (closing.witnesses.len(), closing.normalized.len()),
            "one-over-depth rejection precedes interned metadata publication"
        );
    }
}
