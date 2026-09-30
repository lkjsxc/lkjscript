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

pub(super) fn close(
    program: &mut NormalizedProgram,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
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
