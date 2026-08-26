//! Direct AST oracle. It deliberately does not consume bytecode or compiler-local identities.

use super::{
    ExecutionError, ExecutionFailureClass, PreparedFunction, PreparedProgram, RunObservation,
    RunPolicy,
};
use crate::platform::language::{
    Binding, DeclarationReference, Expression, MapEntry, MatchArm, RecordField,
};
use crate::platform::semantic::OwnerId;
use crate::platform::value::{MapKey, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct ReferenceInterpreter<'a> {
    program: &'a PreparedProgram,
    policy: RunPolicy,
}

impl<'a> ReferenceInterpreter<'a> {
    pub fn new(program: &'a PreparedProgram, policy: RunPolicy) -> Self {
        Self { program, policy }
    }

    pub fn invoke(
        &self,
        function: &OwnerId,
        arguments: Vec<Value>,
    ) -> Result<(Value, RunObservation), ExecutionError> {
        let function = self.program.function(function).ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "reference_function_missing",
                "prepared semantic function is absent",
            )
        })?;
        self.invoke_prepared(function, arguments)
    }

    fn invoke_prepared(
        &self,
        function: &PreparedFunction,
        arguments: Vec<Value>,
    ) -> Result<(Value, RunObservation), ExecutionError> {
        let mut machine = Machine {
            program: self.program,
            policy: self.policy,
            fuel: self.policy.instruction_fuel,
            continuations: Vec::new(),
            call_depth: 0,
            observation: RunObservation {
                instructions: 0,
                calls: 0,
                intrinsic_calls: 0,
                maximum_call_depth: 0,
                maximum_value_stack: 0,
                production_tier: "reference_ast_v1",
            },
        };
        let mut control = machine.invoke_prepared(function, arguments)?;
        loop {
            if machine.fuel == 0 {
                return Err(ExecutionError::resource(
                    "execution_fuel",
                    "reference instruction fuel was exhausted",
                ));
            }
            machine.fuel -= 1;
            machine.observation.instructions = machine.observation.instructions.saturating_add(1);
            control = match control {
                Control::Evaluate(expression, context) => machine.evaluate(expression, context)?,
                Control::Value(value) => match machine.continuations.pop() {
                    Some(continuation) => continuation.resume(&mut machine, value)?,
                    None => return Ok((value, machine.observation)),
                },
            };
            machine.observation.maximum_value_stack = machine
                .observation
                .maximum_value_stack
                .max(machine.continuations.len());
            if machine.continuations.len() > machine.policy.maximum_value_stack {
                return Err(ExecutionError::resource(
                    "execution_value_stack",
                    "reference continuation stack exceeded its bound",
                ));
            }
        }
    }
}

#[derive(Clone)]
struct Context {
    locals: BTreeMap<String, Value>,
}

enum Control {
    Evaluate(Expression, Context),
    Value(Value),
}

enum Continuation {
    FunctionReturn,
    If {
        when_true: Expression,
        when_false: Expression,
        context: Context,
    },
    Let {
        bindings: Vec<Binding>,
        completed: usize,
        body: Expression,
        context: Context,
    },
    Do {
        expressions: Vec<Expression>,
        completed: usize,
        context: Context,
    },
    Call {
        function: OwnerId,
        arguments: Vec<Expression>,
        completed: usize,
        values: Vec<Value>,
        context: Context,
    },
    InvokeCallee {
        arguments: Vec<Expression>,
        context: Context,
    },
    Record {
        owner: Option<OwnerId>,
        fields: Vec<RecordField>,
        completed: usize,
        values: Vec<Value>,
        context: Context,
    },
    Variant {
        owner: OwnerId,
        case: String,
    },
    Field(String),
    List {
        items: Vec<Expression>,
        completed: usize,
        values: Vec<Value>,
        context: Context,
    },
    Map {
        entries: Vec<MapEntry>,
        completed: usize,
        key: Option<Value>,
        values: Vec<(Value, Value)>,
        context: Context,
    },
    Match {
        arms: Vec<MatchArm>,
        context: Context,
    },
}

impl Continuation {
    fn resume(self, machine: &mut Machine<'_>, value: Value) -> Result<Control, ExecutionError> {
        match self {
            Self::FunctionReturn => {
                machine.call_depth = machine.call_depth.saturating_sub(1);
                Ok(Control::Value(value))
            }
            Self::If {
                when_true,
                when_false,
                context,
            } => match value {
                Value::Bool(true) => Ok(Control::Evaluate(when_true, context)),
                Value::Bool(false) => Ok(Control::Evaluate(when_false, context)),
                _ => Err(runtime_type("if condition is not boolean")),
            },
            Self::Let {
                bindings,
                completed,
                body,
                mut context,
            } => {
                let binding = bindings.get(completed).ok_or_else(internal_continuation)?;
                context.locals.insert(binding.name.clone(), value);
                let next = completed + 1;
                if let Some(binding) = bindings.get(next) {
                    let next_value = binding.value.clone();
                    machine.continuations.push(Self::Let {
                        bindings,
                        completed: next,
                        body,
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(next_value, context))
                } else {
                    Ok(Control::Evaluate(body, context))
                }
            }
            Self::Do {
                expressions,
                completed,
                context,
            } => {
                let next = completed + 1;
                if let Some(expression) = expressions.get(next) {
                    machine.continuations.push(Self::Do {
                        expressions: expressions.clone(),
                        completed: next,
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(expression.clone(), context))
                } else {
                    Ok(Control::Value(value))
                }
            }
            Self::Call {
                function,
                arguments,
                completed,
                mut values,
                context,
            } => {
                values.push(value);
                let next = completed + 1;
                if let Some(argument) = arguments.get(next) {
                    machine.continuations.push(Self::Call {
                        function,
                        arguments: arguments.clone(),
                        completed: next,
                        values,
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(argument.clone(), context))
                } else {
                    machine.invoke_function(function, values)
                }
            }
            Self::InvokeCallee { arguments, context } => {
                let Value::Function(function) = value else {
                    return Err(runtime_type("invoke callee is not a function"));
                };
                if let Some(first) = arguments.first() {
                    machine.continuations.push(Self::Call {
                        function,
                        arguments: arguments.clone(),
                        completed: 0,
                        values: Vec::new(),
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(first.clone(), context))
                } else {
                    machine.invoke_function(function, Vec::new())
                }
            }
            Self::Record {
                owner,
                fields,
                completed,
                mut values,
                context,
            } => {
                values.push(value);
                let next = completed + 1;
                if let Some(field) = fields.get(next) {
                    machine.continuations.push(Self::Record {
                        owner,
                        fields: fields.clone(),
                        completed: next,
                        values,
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(field.value.clone(), context))
                } else {
                    Ok(Control::Value(Value::record(
                        owner,
                        fields.into_iter().map(|field| field.name).zip(values),
                    )))
                }
            }
            Self::Variant { owner, case } => {
                Ok(Control::Value(Value::variant(owner, case, Some(value))))
            }
            Self::Field(name) => value
                .field(&name)
                .cloned()
                .map(Control::Value)
                .ok_or_else(|| runtime_type("field selection received a foreign record value")),
            Self::List {
                items,
                completed,
                mut values,
                context,
            } => {
                values.push(value);
                let next = completed + 1;
                if let Some(item) = items.get(next) {
                    machine.continuations.push(Self::List {
                        items: items.clone(),
                        completed: next,
                        values,
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(item.clone(), context))
                } else {
                    Ok(Control::Value(Value::List(Arc::new(values))))
                }
            }
            Self::Map {
                entries,
                completed,
                key,
                mut values,
                context,
            } => match key {
                None => {
                    let entry = entries.get(completed).ok_or_else(internal_continuation)?;
                    let next_value = entry.value.clone();
                    machine.continuations.push(Self::Map {
                        entries,
                        completed,
                        key: Some(value),
                        values,
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(next_value, context))
                }
                Some(key) => {
                    values.push((key, value));
                    let next = completed + 1;
                    if let Some(entry) = entries.get(next) {
                        machine.continuations.push(Self::Map {
                            entries: entries.clone(),
                            completed: next,
                            key: None,
                            values,
                            context: context.clone(),
                        });
                        Ok(Control::Evaluate(entry.key.clone(), context))
                    } else {
                        let mut map = BTreeMap::new();
                        for (key, value) in values {
                            let key = MapKey::from_value(key).map_err(|error| {
                                ExecutionError::new(
                                    ExecutionFailureClass::Infrastructure,
                                    error.code,
                                    error.message,
                                )
                            })?;
                            if map.insert(key, value).is_some() {
                                return Err(ExecutionError::new(
                                    ExecutionFailureClass::Trap,
                                    "map_duplicate_key",
                                    "map expression contains a duplicate key",
                                ));
                            }
                        }
                        Ok(Control::Value(Value::Map(Arc::new(map))))
                    }
                }
            },
            Self::Match { arms, mut context } => {
                let Value::Variant { case, payload, .. } = value else {
                    return Err(runtime_type("match received a foreign non-variant value"));
                };
                let arm = arms
                    .into_iter()
                    .find(|arm| arm.case == case)
                    .ok_or_else(|| {
                        ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "reference_match_case",
                            "validated exhaustive match omitted a runtime case",
                        )
                    })?;
                match (arm.binding, payload) {
                    (Some(binding), Some(payload)) => {
                        context.locals.insert(binding, *payload);
                    }
                    (None, None) => {}
                    _ => {
                        return Err(ExecutionError::new(
                            ExecutionFailureClass::Infrastructure,
                            "reference_match_payload",
                            "runtime variant payload disagrees with validated match",
                        ));
                    }
                }
                Ok(Control::Evaluate(arm.body, context))
            }
        }
    }
}

struct Machine<'a> {
    program: &'a PreparedProgram,
    policy: RunPolicy,
    fuel: u64,
    continuations: Vec<Continuation>,
    call_depth: usize,
    observation: RunObservation,
}

impl Machine<'_> {
    fn evaluate(
        &mut self,
        expression: Expression,
        context: Context,
    ) -> Result<Control, ExecutionError> {
        match expression {
            Expression::Unit(_) => Ok(Control::Value(Value::Unit)),
            Expression::Bool(value, _) => Ok(Control::Value(Value::Bool(value))),
            Expression::I64(value, _) => Ok(Control::Value(Value::I64(value))),
            Expression::Text(value, _) => Ok(Control::Value(Value::text(value))),
            Expression::StaticText(value, _) => Ok(Control::Value(Value::static_text(value))),
            Expression::Variable(name, _) => match context.locals.get(&name).cloned() {
                Some(value) => Ok(Control::Value(value)),
                None => Err(ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "reference_local_missing",
                    format!("validated expression references absent local '{name}'"),
                )),
            },
            Expression::Constant(reference, _) => {
                let owner = self.resolve(&reference)?;
                self.invoke_function(owner, Vec::new())
            }
            Expression::If {
                condition,
                when_true,
                when_false,
                ..
            } => {
                self.continuations.push(Continuation::If {
                    when_true: *when_true,
                    when_false: *when_false,
                    context: context.clone(),
                });
                Ok(Control::Evaluate(*condition, context))
            }
            Expression::Let { bindings, body, .. } => {
                let Some(first) = bindings.first() else {
                    return Ok(Control::Evaluate(*body, context));
                };
                self.continuations.push(Continuation::Let {
                    bindings: bindings.clone(),
                    completed: 0,
                    body: *body,
                    context: context.clone(),
                });
                Ok(Control::Evaluate(first.value.clone(), context))
            }
            Expression::Do { expressions, .. } => {
                let first = expressions.first().cloned().ok_or_else(|| {
                    ExecutionError::new(
                        ExecutionFailureClass::Infrastructure,
                        "reference_do_empty",
                        "validated do expression is empty",
                    )
                })?;
                self.continuations.push(Continuation::Do {
                    expressions,
                    completed: 0,
                    context: context.clone(),
                });
                Ok(Control::Evaluate(first, context))
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let owner = self.resolve(&function)?;
                if let Some(first) = arguments.first() {
                    self.continuations.push(Continuation::Call {
                        function: owner,
                        arguments: arguments.clone(),
                        completed: 0,
                        values: Vec::new(),
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(first.clone(), context))
                } else {
                    self.invoke_function(owner, Vec::new())
                }
            }
            Expression::Invoke {
                callee, arguments, ..
            } => {
                self.continuations.push(Continuation::InvokeCallee {
                    arguments,
                    context: context.clone(),
                });
                Ok(Control::Evaluate(*callee, context))
            }
            Expression::Record { ty, fields, .. } => {
                let owner = ty
                    .as_ref()
                    .map(|reference| self.resolve(reference))
                    .transpose()?;
                if let Some(first) = fields.first() {
                    self.continuations.push(Continuation::Record {
                        owner,
                        fields: fields.clone(),
                        completed: 0,
                        values: Vec::new(),
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(first.value.clone(), context))
                } else {
                    Ok(Control::Value(Value::record(owner, [])))
                }
            }
            Expression::Variant {
                ty, case, payload, ..
            } => {
                let owner = self.resolve(&ty)?;
                if let Some(payload) = payload {
                    self.continuations
                        .push(Continuation::Variant { owner, case });
                    Ok(Control::Evaluate(*payload, context))
                } else {
                    Ok(Control::Value(Value::variant(owner, case, None)))
                }
            }
            Expression::Field { value, field, .. } => {
                self.continuations.push(Continuation::Field(field));
                Ok(Control::Evaluate(*value, context))
            }
            Expression::List { items, .. } => {
                if let Some(first) = items.first() {
                    self.continuations.push(Continuation::List {
                        items: items.clone(),
                        completed: 0,
                        values: Vec::new(),
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(first.clone(), context))
                } else {
                    Ok(Control::Value(Value::List(Arc::new(Vec::new()))))
                }
            }
            Expression::Map { entries, .. } => {
                if let Some(first) = entries.first() {
                    self.continuations.push(Continuation::Map {
                        entries: entries.clone(),
                        completed: 0,
                        key: None,
                        values: Vec::new(),
                        context: context.clone(),
                    });
                    Ok(Control::Evaluate(first.key.clone(), context))
                } else {
                    Ok(Control::Value(Value::Map(Arc::new(BTreeMap::new()))))
                }
            }
            Expression::Match { value, arms, .. } => {
                self.continuations.push(Continuation::Match {
                    arms,
                    context: context.clone(),
                });
                Ok(Control::Evaluate(*value, context))
            }
            Expression::FunctionRef { function, .. } => {
                Ok(Control::Value(Value::Function(self.resolve(&function)?)))
            }
            Expression::Perform { .. } | Expression::Transaction { .. } => {
                Err(ExecutionError::new(
                    ExecutionFailureClass::Capability,
                    "capability_unbound",
                    "reference effect execution requires a capability oracle",
                ))
            }
        }
    }

    fn invoke_function(
        &mut self,
        owner: OwnerId,
        arguments: Vec<Value>,
    ) -> Result<Control, ExecutionError> {
        let function = self.program.function(&owner).ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "reference_function_missing",
                format!("prepared function '{}' is absent", owner.diagnostic_name()),
            )
        })?;
        self.invoke_prepared(function, arguments)
    }

    fn invoke_prepared(
        &mut self,
        function: &PreparedFunction,
        arguments: Vec<Value>,
    ) -> Result<Control, ExecutionError> {
        if arguments.len() != function.parameters.len() {
            return Err(runtime_type("function argument count is foreign"));
        }
        self.observation.calls = self.observation.calls.saturating_add(1);
        if let Some(implementation) = &function.external_implementation {
            self.observation.intrinsic_calls = self.observation.intrinsic_calls.saturating_add(1);
            return self
                .program
                .call_intrinsic(implementation, &function.signature, arguments)
                .map(Control::Value);
        }
        if self.call_depth >= self.policy.maximum_call_depth {
            return Err(ExecutionError::resource(
                "execution_call_depth",
                "maximum reference call depth was exceeded",
            ));
        }
        let source = function.source.clone().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "reference_source_missing",
                "prepared source function has no reference expression",
            )
        })?;
        let locals = function.parameters.iter().cloned().zip(arguments).collect();
        self.call_depth += 1;
        self.observation.maximum_call_depth =
            self.observation.maximum_call_depth.max(self.call_depth);
        self.continuations.push(Continuation::FunctionReturn);
        Ok(Control::Evaluate(source, Context { locals }))
    }

    fn resolve(&self, reference: &DeclarationReference) -> Result<OwnerId, ExecutionError> {
        super::resolve_reference_owner(self.program.artifact(), reference).map_err(|error| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                error.code,
                error.message,
            )
        })
    }
}

fn runtime_type(message: &'static str) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "reference_runtime_type",
        message,
    )
}

fn internal_continuation() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "reference_continuation",
        "reference evaluator continuation is inconsistent",
    )
}
