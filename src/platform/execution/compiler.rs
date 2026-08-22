use super::resolve_reference_owner;
use crate::platform::artifact::LoadedArtifact;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::language::{Expression, MatchArm, Parameter};
use crate::platform::semantic::OwnerId;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub instructions: Vec<Instruction>,
    pub local_count: usize,
}

#[derive(Clone, Debug)]
pub enum Instruction {
    Unit,
    Bool(bool),
    I64(i64),
    Text(String),
    StaticText(String),
    Function(OwnerId),
    LoadLocal(usize),
    StoreLocal(usize),
    Drop,
    JumpIfFalse(usize),
    Jump(usize),
    Call {
        function: OwnerId,
        arguments: usize,
    },
    Invoke {
        arguments: usize,
    },
    Record {
        owner: Option<OwnerId>,
        fields: Vec<String>,
    },
    Variant {
        owner: OwnerId,
        case: String,
        has_payload: bool,
    },
    Field(String),
    List(usize),
    Map(usize),
    SwitchVariant(Vec<VariantJump>),
    Perform {
        capability: String,
        operation: String,
        arguments: usize,
    },
    BeginTransaction {
        capability: String,
        binding: String,
    },
    CommitTransaction {
        binding: String,
    },
    Return,
}

#[derive(Clone, Debug)]
pub struct VariantJump {
    pub case: String,
    pub target: usize,
    pub binding_local: Option<usize>,
}

pub fn compile_function(
    artifact: &LoadedArtifact,
    parameters: &[Parameter],
    expression: &Expression,
) -> Result<CompiledFunction, Diagnostic> {
    let mut compiler = Compiler {
        artifact,
        instructions: Vec::new(),
        locals: BTreeMap::new(),
        next_local: 0,
    };
    for parameter in parameters {
        compiler.bind(&parameter.name)?;
    }
    compiler.expression(expression)?;
    compiler.instructions.push(Instruction::Return);
    Ok(CompiledFunction {
        instructions: compiler.instructions,
        local_count: compiler.next_local,
    })
}

struct Compiler<'a> {
    artifact: &'a LoadedArtifact,
    instructions: Vec<Instruction>,
    locals: BTreeMap<String, usize>,
    next_local: usize,
}

impl Compiler<'_> {
    fn expression(&mut self, expression: &Expression) -> Result<(), Diagnostic> {
        match expression {
            Expression::Unit(_) => self.instructions.push(Instruction::Unit),
            Expression::Bool(value, _) => self.instructions.push(Instruction::Bool(*value)),
            Expression::I64(value, _) => self.instructions.push(Instruction::I64(*value)),
            Expression::Text(value, _) => {
                self.instructions.push(Instruction::Text(value.clone()));
            }
            Expression::StaticText(value, _) => {
                self.instructions
                    .push(Instruction::StaticText(value.clone()));
            }
            Expression::Variable(name, _) => {
                let local = self.locals.get(name).copied().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Corrupt,
                        "compile_local_missing",
                        format!("validated expression references absent local '{name}'"),
                    )
                })?;
                self.instructions.push(Instruction::LoadLocal(local));
            }
            Expression::Constant(reference, _) => {
                self.instructions.push(Instruction::Call {
                    function: resolve_reference_owner(self.artifact, reference)?,
                    arguments: 0,
                });
            }
            Expression::If {
                condition,
                when_true,
                when_false,
                ..
            } => {
                self.expression(condition)?;
                let conditional = self.instructions.len();
                self.instructions.push(Instruction::JumpIfFalse(usize::MAX));
                self.expression(when_true)?;
                let jump = self.instructions.len();
                self.instructions.push(Instruction::Jump(usize::MAX));
                let false_target = self.instructions.len();
                self.expression(when_false)?;
                let end = self.instructions.len();
                self.instructions[conditional] = Instruction::JumpIfFalse(false_target);
                self.instructions[jump] = Instruction::Jump(end);
            }
            Expression::Let { bindings, body, .. } => {
                let previous = self.locals.clone();
                for binding in bindings {
                    self.expression(&binding.value)?;
                    let local = self.bind(&binding.name)?;
                    self.instructions.push(Instruction::StoreLocal(local));
                }
                self.expression(body)?;
                self.locals = previous;
            }
            Expression::Do { expressions, .. } => {
                for (index, expression) in expressions.iter().enumerate() {
                    self.expression(expression)?;
                    if index + 1 != expressions.len() {
                        self.instructions.push(Instruction::Drop);
                    }
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                for argument in arguments {
                    self.expression(argument)?;
                }
                self.instructions.push(Instruction::Call {
                    function: resolve_reference_owner(self.artifact, function)?,
                    arguments: arguments.len(),
                });
            }
            Expression::Invoke {
                callee, arguments, ..
            } => {
                self.expression(callee)?;
                for argument in arguments {
                    self.expression(argument)?;
                }
                self.instructions.push(Instruction::Invoke {
                    arguments: arguments.len(),
                });
            }
            Expression::Record { ty, fields, .. } => {
                for field in fields {
                    self.expression(&field.value)?;
                }
                let owner = ty
                    .as_ref()
                    .map(|reference| resolve_reference_owner(self.artifact, reference))
                    .transpose()?;
                self.instructions.push(Instruction::Record {
                    owner,
                    fields: fields.iter().map(|field| field.name.clone()).collect(),
                });
            }
            Expression::Variant {
                ty, case, payload, ..
            } => {
                if let Some(payload) = payload {
                    self.expression(payload)?;
                }
                self.instructions.push(Instruction::Variant {
                    owner: resolve_reference_owner(self.artifact, ty)?,
                    case: case.clone(),
                    has_payload: payload.is_some(),
                });
            }
            Expression::Field { value, field, .. } => {
                self.expression(value)?;
                self.instructions.push(Instruction::Field(field.clone()));
            }
            Expression::List { items, .. } => {
                for item in items {
                    self.expression(item)?;
                }
                self.instructions.push(Instruction::List(items.len()));
            }
            Expression::Map { entries, .. } => {
                for entry in entries {
                    self.expression(&entry.key)?;
                    self.expression(&entry.value)?;
                }
                self.instructions.push(Instruction::Map(entries.len()));
            }
            Expression::Match { value, arms, .. } => {
                self.expression(value)?;
                self.compile_match(arms)?;
            }
            Expression::FunctionRef { function, .. } => {
                self.instructions
                    .push(Instruction::Function(resolve_reference_owner(
                        self.artifact,
                        function,
                    )?));
            }
            Expression::Perform {
                capability,
                operation,
                arguments,
                ..
            } => {
                for argument in arguments {
                    self.expression(argument)?;
                }
                self.instructions.push(Instruction::Perform {
                    capability: capability.clone(),
                    operation: operation.clone(),
                    arguments: arguments.len(),
                });
            }
            Expression::Transaction {
                capability,
                binding,
                body,
                ..
            } => {
                self.instructions.push(Instruction::BeginTransaction {
                    capability: capability.clone(),
                    binding: binding.clone(),
                });
                self.expression(body)?;
                self.instructions.push(Instruction::CommitTransaction {
                    binding: binding.clone(),
                });
            }
        }
        Ok(())
    }

    fn compile_match(&mut self, arms: &[MatchArm]) -> Result<(), Diagnostic> {
        let switch = self.instructions.len();
        self.instructions
            .push(Instruction::SwitchVariant(Vec::new()));
        let previous = self.locals.clone();
        let mut jumps = Vec::with_capacity(arms.len());
        let mut exits = Vec::with_capacity(arms.len());
        for arm in arms {
            self.locals = previous.clone();
            let binding_local = arm
                .binding
                .as_ref()
                .map(|binding| self.bind(binding))
                .transpose()?;
            let target = self.instructions.len();
            jumps.push(VariantJump {
                case: arm.case.clone(),
                target,
                binding_local,
            });
            self.expression(&arm.body)?;
            exits.push(self.instructions.len());
            self.instructions.push(Instruction::Jump(usize::MAX));
        }
        let end = self.instructions.len();
        for exit in exits {
            self.instructions[exit] = Instruction::Jump(end);
        }
        self.instructions[switch] = Instruction::SwitchVariant(jumps);
        self.locals = previous;
        Ok(())
    }

    fn bind(&mut self, name: &str) -> Result<usize, Diagnostic> {
        // The semantic validator has already rejected duplicates in one binding list. A nested
        // let or match payload may deliberately shadow an outer local, so allocate a fresh slot
        // and restore the prior environment when that lexical scope ends.
        let local = self.next_local;
        self.next_local = self.next_local.checked_add(1).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "compile_local_count",
                "compiled function has too many locals",
            )
        })?;
        self.locals.insert(name.to_owned(), local);
        Ok(local)
    }
}
