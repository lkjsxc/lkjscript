//! Finite static witness specialization. No witness is a runtime language value.
use super::super::prepare::NormalizedFunction;
use super::*;
use crate::platform::compiler::{CompilationPayload, CompilationUnit};
use crate::platform::kernel::{
    DeclarationReference, ImplementationOperand, OwnedImplementation, OwnerKey, PackageId,
};
use crate::platform::semantic_id::ImplementationParameterId;
use std::sync::Arc;

type Application = (FunctionIndex, Vec<DeclarationReference>);
type Bindings = BTreeMap<ImplementationParameterId, DeclarationReference>;

struct Closing<'a, 'b> {
    templates: Arc<[NormalizedFunction]>,
    functions: Vec<NormalizedFunction>,
    implementations: BTreeMap<DeclarationReference, &'a OwnedImplementation>,
    targets: BTreeMap<DeclarationReference, FunctionIndex>,
    instances: BTreeMap<Application, FunctionIndex>,
    pending: Vec<(Application, FunctionIndex)>,
    work: &'a mut Budget<'b>,
}

impl Closing<'_, '_> {
    fn operand(
        &mut self,
        operand: ImplementationOperand,
        scope: Option<DeclarationReference>,
        bindings: &Bindings,
    ) -> Result<DeclarationReference, Diagnostic> {
        step(self.work)?;
        match operand {
            ImplementationOperand::Concrete { implementation } => {
                if !self.implementations.contains_key(&implementation) {
                    return Err(missing());
                }
                Ok(implementation)
            }
            ImplementationOperand::Parameter {
                function,
                parameter,
            } => {
                if Some(function) != scope {
                    return Err(missing());
                }
                bindings.get(&parameter).copied().ok_or_else(missing)
            }
        }
    }

    fn application(
        &mut self,
        function: FunctionIndex,
        implementations: Vec<DeclarationReference>,
    ) -> Result<FunctionIndex, Diagnostic> {
        step(self.work)?;
        let template = self
            .templates
            .get(function.0 as usize)
            .ok_or_else(missing)?;
        if template.implementation_parameters.len() != implementations.len() {
            return Err(missing());
        }
        self.work
            .reserve::<DeclarationReference>(implementations.len())?;
        let key = (function, implementations);
        if let Some(index) = self.instances.get(&key) {
            return Ok(*index);
        }
        self.work
            .reserve::<(DeclarationReference, TypeObjectDigest)>(key.1.len())?;
        let mut supplied = Vec::new();
        for (p, reference) in template.implementation_parameters.iter().zip(&key.1) {
            step(self.work)?;
            let implementation = self.implementations.get(reference).ok_or_else(missing)?;
            if implementation.contract != p.contract {
                return Err(missing());
            }
            supplied.push((*reference, implementation.self_type));
        }
        self.work.node::<(Application, FunctionIndex)>()?;
        self.work.node::<(Application, FunctionIndex)>()?;
        self.work.reserve::<DeclarationReference>(key.1.len())?;
        let index = if key.1.is_empty() {
            function
        } else {
            self.work.reserve::<NormalizedFunction>(1)?;
            let index = FunctionIndex(
                u32::try_from(self.functions.len()).map_err(|_| missing())?,
                function.1,
            );
            let mut instance = template.clone();
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
    ) -> Result<(), Diagnostic> {
        self.work
            .reserve::<NormalizedInstruction>(code.instructions.len())?;
        for instruction in Arc::make_mut(&mut code.instructions) {
            step(self.work)?;
            match instruction {
                NormalizedInstruction::Parallel {
                    left,
                    left_implementations,
                    right,
                    right_implementations,
                    ..
                } => {
                    for (function, implementations) in
                        [(left, left_implementations), (right, right_implementations)]
                    {
                        self.work
                            .reserve::<DeclarationReference>(implementations.len())?;
                        let mut selected = Vec::new();
                        for operand in implementations.iter() {
                            selected.push(self.operand(*operand, scope, bindings)?);
                        }
                        *function = self.application(*function, selected)?;
                        *implementations = Arc::from([]);
                    }
                }
                NormalizedInstruction::ImplementationCall {
                    function,
                    implementations,
                    type_arguments,
                    arguments,
                } => {
                    self.work
                        .reserve::<DeclarationReference>(implementations.len())?;
                    let mut selected = Vec::new();
                    for operand in implementations.iter() {
                        selected.push(self.operand(*operand, scope, bindings)?);
                    }
                    let function = self.application(*function, selected)?;
                    *instruction = NormalizedInstruction::Call {
                        function,
                        type_arguments: Arc::clone(type_arguments),
                        arguments: *arguments,
                        effect_arguments: Arc::from([]),
                        requirement_arguments: Arc::from([]),
                    };
                }
                NormalizedInstruction::MethodCall {
                    witness,
                    contract,
                    method,
                    arguments,
                } => {
                    let selected = self.operand(*witness, scope, bindings)?;
                    let implementation = self.implementations.get(&selected).ok_or_else(missing)?;
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
                    let function = self.application(target, Vec::new())?;
                    *instruction = NormalizedInstruction::Call {
                        function,
                        type_arguments: Arc::from([]),
                        arguments: *arguments,
                        effect_arguments: Arc::from([]),
                        requirement_arguments: Arc::from([]),
                    };
                }
                NormalizedInstruction::Call { function, .. }
                | NormalizedInstruction::TailCall { function, .. }
                | NormalizedInstruction::FunctionValue { function, .. } => {
                    self.application(*function, Vec::new())?;
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
            left_implementations,
            right_implementations,
            ..
        } = instruction
            && (!left_implementations.is_empty() || !right_implementations.is_empty())
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
        work,
    };
    for i in 0..closing.templates.len() {
        if closing.templates[i].implementation_parameters.is_empty() {
            closing.application(
                FunctionIndex(
                    u32::try_from(i).map_err(|_| missing())?,
                    program.value_origin,
                ),
                Vec::new(),
            )?;
        }
    }
    for test in program.tests.values_mut() {
        closing.code(&mut test.actual, None, &Bindings::new())?;
        closing.code(&mut test.expected, None, &Bindings::new())?;
    }
    closing
        .work
        .reserve::<super::super::prepare::NormalizedPort>(program.ports.len())?;
    for port in Arc::make_mut(&mut program.ports) {
        if let NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) =
            &mut port.entry
        {
            closing.code(code, None, &Bindings::new())?;
        }
    }
    while let Some(((template, selected), index)) = closing.pending.pop() {
        step(closing.work)?;
        let function = &closing.templates[template.0 as usize];
        closing
            .work
            .reserve::<(ImplementationParameterId, DeclarationReference)>(selected.len())?;
        let bindings = function
            .implementation_parameters
            .iter()
            .map(|p| p.id)
            .zip(selected)
            .collect();
        let scope = function.declaration;
        if let NormalizedFunctionBody::Code(mut code) = function.body.clone() {
            closing.code(&mut code, Some(scope), &bindings)?;
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
                implementations: Arc::from([]),
                arguments: 0,
            },
            NormalizedInstruction::MethodCall {
                witness: ImplementationOperand::Concrete {
                    implementation: reference,
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
                f.implementation_arguments = Arc::from([(reference, f.result)]);
            } else {
                f.implementation_parameters =
                    Arc::from([crate::platform::kernel::ImplementationParameter {
                        id: ImplementationParameterId::migrate(b"witness-absence-probe", 0),
                        name: crate::platform::kernel::Name::new("ops").unwrap(),
                        contract: reference,
                        self_type: f.result,
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
