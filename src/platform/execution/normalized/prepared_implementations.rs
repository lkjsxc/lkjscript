//! Finite closure of callable types and exact applied static witnesses.
//! A caller's type scope is resolved before any witness crosses a call boundary.
use super::super::prepare::{NormalizedFunction, NormalizedImplementationArgument};
use super::*;
use crate::platform::compiler::{CompilationPayload, CompilationUnit};
use crate::platform::kernel::{
    DeclarationReference, ImplementationOperand, OwnedImplementation, OwnerKey, PackageId,
};
use crate::platform::semantic_id::ImplementationParameterId;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AppliedWitness {
    implementation: DeclarationReference,
    type_arguments: Vec<TypeObjectDigest>,
}

type Application = (FunctionIndex, Vec<TypeObjectDigest>, Vec<AppliedWitness>);
type Bindings = BTreeMap<ImplementationParameterId, AppliedWitness>;

struct Closing<'a, 'b> {
    templates: Arc<[NormalizedFunction]>,
    functions: Vec<NormalizedFunction>,
    implementations: BTreeMap<DeclarationReference, &'a OwnedImplementation>,
    targets: BTreeMap<DeclarationReference, FunctionIndex>,
    instances: BTreeMap<Application, FunctionIndex>,
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

    fn operand(
        &mut self,
        operand: &ImplementationOperand,
        scope: Option<DeclarationReference>,
        bindings: &Bindings,
        types: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) -> Result<AppliedWitness, Diagnostic> {
        step(self.work)?;
        match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
            } => {
                let scheme = self
                    .implementations
                    .get(implementation)
                    .ok_or_else(missing)?;
                if scheme.type_parameters.len() != type_arguments.len() {
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
                Ok(AppliedWitness {
                    implementation: *implementation,
                    type_arguments: arguments,
                })
            }
            ImplementationOperand::Parameter {
                function,
                parameter,
            } => {
                if Some(*function) != scope {
                    return Err(missing());
                }
                let witness = bindings.get(parameter).ok_or_else(missing)?;
                self.reserve_witness(witness)?;
                Ok(witness.clone())
            }
        }
    }

    fn application(
        &mut self,
        function: FunctionIndex,
        types: Vec<TypeObjectDigest>,
        implementations: Vec<AppliedWitness>,
    ) -> Result<FunctionIndex, Diagnostic> {
        step(self.work)?;
        let template = self
            .templates
            .get(function.0 as usize)
            .ok_or_else(missing)?;
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
            step(self.work)?;
            let implementation = self
                .implementations
                .get(&witness.implementation)
                .ok_or_else(missing)?;
            if implementation.contract != p.contract
                || implementation.type_parameters.len() != witness.type_arguments.len()
            {
                return Err(missing());
            }
            if implementation.type_arguments.len() != p.type_arguments.len() {
                return Err(missing());
            }
            self.work
                .reserve::<TypeObjectDigest>(implementation.type_arguments.len())?;
            self.work
                .reserve::<TypeObjectDigest>(implementation.type_arguments.len())?;
            self.work
                .reserve::<TypeObjectDigest>(witness.type_arguments.len())?;
            self.work
                .reserve::<TypeObjectDigest>(witness.type_arguments.len())?;
            self.work
                .reserve::<(TypeParameterId, TypeObjectDigest)>(witness.type_arguments.len())?;
            let implementation_bindings = implementation
                .type_parameters
                .iter()
                .copied()
                .zip(witness.type_arguments.iter().copied())
                .collect();
            let self_type = substitute(
                self.types,
                implementation.self_type,
                &implementation_bindings,
                0,
                self.work,
            )?;
            if self_type != substitute(self.types, p.self_type, &function_bindings, 0, self.work)? {
                return Err(missing());
            }
            let mut type_arguments = Vec::new();
            for (actual, expected) in implementation.type_arguments.iter().zip(&p.type_arguments) {
                let actual =
                    substitute(self.types, *actual, &implementation_bindings, 0, self.work)?;
                if actual != substitute(self.types, *expected, &function_bindings, 0, self.work)? {
                    return Err(missing());
                }
                type_arguments.push(actual);
            }
            self.work.reserve::<usize>(4)?;
            supplied.push(NormalizedImplementationArgument {
                implementation: witness.implementation,
                implementation_type_arguments: witness.type_arguments.clone().into(),
                self_type,
                type_arguments: type_arguments.into(),
            });
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
            if let NormalizedInstruction::MethodCall {
                witness: ImplementationOperand::Concrete { type_arguments, .. },
                ..
            } = instruction
            {
                self.work
                    .reserve::<TypeObjectDigest>(type_arguments.len())?;
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
                    let implementation = self
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
                    self.work.reserve::<(TypeParameterId, TypeObjectDigest)>(
                        selected.type_arguments.len(),
                    )?;
                    let implementation_bindings = implementation
                        .type_parameters
                        .iter()
                        .copied()
                        .zip(selected.type_arguments)
                        .collect();
                    let applied = self.arguments(&mapped_types, &implementation_bindings)?;
                    self.work.reserve::<TypeObjectDigest>(applied.len())?;
                    self.work.reserve::<usize>(2)?;
                    let function = self.application(target, applied.clone(), Vec::new())?;
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
    program.functions = closing.functions.into();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
                    implementation: reference,
                    implementation_type_arguments: Arc::from([]),
                    self_type: f.result,
                    type_arguments: Arc::from([]),
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
}
