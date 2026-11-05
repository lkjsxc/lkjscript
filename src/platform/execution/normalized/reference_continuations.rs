//! Canonical-expression continuations for the closed pure reference route.
//!
//! No normalized instruction or VM execution state participates. A finite
//! canonical-body walk selects the complete supported expression closure;
//! broader routes retain their previous evaluator as a whole. Every pending
//! operand, local and result stays in a checked owner. The new flat frontiers
//! use fallible actual storage without changing modeled execution charges.

use super::*;
use crate::platform::kernel::RecordExpressionField;
use std::collections::HashSet;
use std::ops::ControlFlow;

pub(super) fn eligible(target: &AdmittedGraphCall) -> bool {
    #[cfg(test)]
    if FORCE_LEGACY.get() {
        return false;
    }
    matches!(target.function.effect, FunctionEffect::Pure)
        && target.function.effect_parameters.is_empty()
        && target.function.requirement_parameters.is_empty()
        && target.function.implementation_parameters.is_empty()
        && target.function.read_scope_parameters.is_empty()
        && target.function.read_scope_sources.is_empty()
        && matches!(target.function.result.relation, ResultRelation::Value)
        && target.implementations.is_empty()
}

pub(super) fn run(
    state: &mut ReferenceState<'_>,
    target: AdmittedGraphCall,
    tail: bool,
) -> Result<ReferenceStep, ExecutionError> {
    let mut machine = Machine::new(state);
    if !machine.supported(&target)? {
        return machine.state.enter_graph_call_recursive(target, tail);
    }
    machine.drive(target, tail).map(ReferenceStep::Break)
}

pub(super) fn reserve<T>(
    values: &mut Vec<T>,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    reserve_many(values, 1, control)
}

fn reserve_many<T>(
    values: &mut Vec<T>,
    additional: usize,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    control.check()?;
    let required = values
        .len()
        .checked_add(additional)
        .ok_or_else(allocation_error)?;
    if required > values.capacity() {
        values
            .try_reserve(additional)
            .map_err(|_| allocation_error())?;
    }
    Ok(())
}

fn frontier_reserve<T>(
    values: &mut Vec<T>,
    control: &ExecutionControl,
) -> Result<(), ExecutionError> {
    control.check()?;
    if values.len() == values.capacity() {
        reservation_probe()?;
        values.try_reserve(1).map_err(|_| allocation_error())?;
    }
    Ok(())
}

fn allocation_error() -> ExecutionError {
    reference_resource(
        "normalized_reference_continuation_allocation",
        "canonical continuation storage could not be reserved",
    )
}

fn invariant_error() -> ExecutionError {
    reference_error(
        "normalized_reference_continuation_state",
        "canonical continuation state is inconsistent",
    )
}

#[derive(Clone, Copy)]
struct Mode {
    tail: bool,
    demand: bool,
}

impl Mode {
    const TAIL: Self = Self {
        tail: true,
        demand: false,
    };
    const VALUE: Self = Self {
        tail: false,
        demand: false,
    };
}

struct Evaluation {
    expression: ExpressionId,
    mode: Mode,
}

type Next = ControlFlow<CheckedValue, Evaluation>;

fn evaluate(expression: ExpressionId, mode: Mode) -> Next {
    Next::Continue(Evaluation { expression, mode })
}

struct ExpressionContext {
    occurrence: Option<(PackageId, OwnerKey)>,
    demand: bool,
}

struct Activation {
    graph: GraphActivation,
    base: usize,
    occurrence: Option<(PackageId, OwnerKey)>,
    caller: Option<PackageId>,
}

struct LetContinuation {
    bindings: Vec<BindingId>,
    next: usize,
    pending: Option<BindingId>,
    scoped: Vec<LocalValueReference>,
    body: ExpressionId,
    mode: Mode,
}

struct SequenceContinuation {
    items: Vec<ExpressionId>,
    next: usize,
    mode: Mode,
}

struct RecordContinuation {
    nominal_type: Option<DeclarationReference>,
    type_arguments: Vec<TypeObjectDigest>,
    fields: Vec<RecordExpressionField>,
    values: Vec<CheckedValue>,
    next: usize,
}

struct ListContinuation {
    items: Vec<ExpressionId>,
    values: Vec<CheckedValue>,
    next: usize,
}

// Large call/binding contexts have their own flat stacks; continuation tags
// retain indices, never owning links to another continuation.
enum Continuation {
    Expression(ExpressionContext),
    ArgumentUse(ParameterUse),
    Branch {
        when_true: ExpressionId,
        when_false: ExpressionId,
        mode: Mode,
    },
    Let(LetContinuation),
    LetExit(Vec<LocalValueReference>),
    Sequence(SequenceContinuation),
    CallArgument(usize),
    BindCallee(Vec<ExpressionId>),
    BindArgument(usize),
    InvokeCallee(usize),
    InvokeArgument(usize),
    Record(RecordContinuation),
    List(ListContinuation),
    Field(FieldSelector),
}

struct CallEvaluation {
    declaration: DeclarationReference,
    types: Vec<TypeObjectDigest>,
    effects: Vec<EffectRow>,
    requirements: Vec<RequirementOperand>,
    scopes: Vec<ReadScopeArgument>,
    expressions: Vec<ExpressionId>,
    uses: Vec<ParameterUse>,
    values: Vec<CheckedValue>,
    next: usize,
    tail: bool,
}

struct InvokeEvaluation {
    invocation: Option<CanonicalInvocation>,
    expressions: Vec<ExpressionId>,
    callee: Option<CheckedValue>,
    values: Vec<CheckedValue>,
    next: usize,
    tail: bool,
}

struct BindEvaluation {
    binding: checked::PendingBind,
    expressions: Vec<ExpressionId>,
    next: usize,
}

struct Machine<'state, 'input> {
    state: &'state mut ReferenceState<'input>,
    activations: Vec<Activation>,
    continuations: Vec<Continuation>,
    calls: Vec<CallEvaluation>,
    invokes: Vec<InvokeEvaluation>,
    binds: Vec<BindEvaluation>,
    shapes: Vec<(DeclarationReference, bool)>,
    transient_call: Option<PackageId>,
}

impl<'state, 'input> Machine<'state, 'input> {
    fn new(state: &'state mut ReferenceState<'input>) -> Self {
        Self {
            state,
            activations: Vec::new(),
            continuations: Vec::new(),
            calls: Vec::new(),
            invokes: Vec::new(),
            binds: Vec::new(),
            shapes: Vec::new(),
            transient_call: None,
        }
    }

    fn supported(&mut self, target: &AdmittedGraphCall) -> Result<bool, ExecutionError> {
        if !eligible(target) {
            return Ok(false);
        }
        let insertion = match self
            .shapes
            .binary_search_by_key(&target.declaration, |row| row.0)
        {
            Ok(index) => return Ok(self.shapes[index].1),
            Err(index) => index,
        };
        let supported = self.body_supported(target.declaration.package, target.function.body)?;
        reserve(&mut self.shapes, self.state.control)?;
        self.shapes
            .insert(insertion, (target.declaration, supported));
        Ok(supported)
    }

    fn body_supported(
        &mut self,
        package: PackageId,
        body: ExpressionId,
    ) -> Result<bool, ExecutionError> {
        let mut pending = Vec::new();
        reserve(&mut pending, self.state.control)?;
        pending.push(body);
        let mut seen = HashSet::new();
        while let Some(expression) = pending.pop() {
            self.state.control.check()?;
            // Exact owner identity is a leaf key. The set guarantees this routing
            // walk terminates even if a read provider contradicts its admitted
            // containment graph; actual semantic admission is never replaced.
            seen.try_reserve(1).map_err(|_| allocation_error())?;
            if !seen.insert(expression) {
                return Ok(false);
            }
            let Some(OwnerRecord::Expression(record)) = self
                .state
                .owner_in_package(package, OwnerKey::Expression(expression))?
            else {
                return Ok(false);
            };
            let control = self.state.control;
            let mut append = |child| -> Result<(), ExecutionError> {
                reserve(&mut pending, control)?;
                pending.push(child);
                Ok(())
            };
            match record.operation {
                ExpressionOperation::Unit {}
                | ExpressionOperation::Bool { .. }
                | ExpressionOperation::I64 { .. }
                | ExpressionOperation::F64 { .. }
                | ExpressionOperation::Text { .. }
                | ExpressionOperation::StaticText { .. }
                | ExpressionOperation::Local { .. }
                | ExpressionOperation::FunctionValue { .. } => {}
                ExpressionOperation::If {
                    condition,
                    when_true,
                    when_false,
                } => {
                    append(when_false)?;
                    append(when_true)?;
                    append(condition)?;
                }
                ExpressionOperation::Let { bindings, body } => {
                    append(body)?;
                    for binding in bindings.into_iter().rev() {
                        let Some(OwnerRecord::Binding(record)) = self
                            .state
                            .owner_in_package(package, OwnerKey::Binding(binding))?
                        else {
                            return Ok(false);
                        };
                        if record.kind != BindingKind::Let {
                            return Ok(false);
                        }
                        let Some(value) = record.value else {
                            return Ok(false);
                        };
                        append(value)?;
                    }
                }
                ExpressionOperation::Sequence { items }
                | ExpressionOperation::List { items, .. } => {
                    for expression in items.into_iter().rev() {
                        append(expression)?;
                    }
                }
                ExpressionOperation::Call { arguments, .. } => {
                    for expression in arguments.into_iter().rev() {
                        append(expression)?;
                    }
                }
                ExpressionOperation::Bind { callee, arguments }
                | ExpressionOperation::Invoke { callee, arguments } => {
                    for expression in arguments.into_iter().rev() {
                        append(expression)?;
                    }
                    append(callee)?;
                }
                ExpressionOperation::Record { fields, .. } => {
                    for field in fields.into_iter().rev() {
                        append(field.value)?;
                    }
                }
                ExpressionOperation::Field { value, .. } => append(value)?,
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    fn push(&mut self, continuation: Continuation) -> Result<(), ExecutionError> {
        frontier_reserve(&mut self.continuations, self.state.control)?;
        self.continuations.push(continuation);
        Ok(())
    }

    fn start(
        &mut self,
        target: AdmittedGraphCall,
        tail: bool,
        caller: Option<PackageId>,
    ) -> Result<Next, ExecutionError> {
        frontier_reserve(&mut self.activations, self.state.control)?;
        let control_frames = self
            .state
            .control_frames
            .checked_add(1)
            .ok_or_else(allocation_error)?;
        let graph = self.state.begin_graph_activation(target, tail, true)?;
        let body = graph.function.body;
        self.activations.push(Activation {
            graph,
            base: self.continuations.len(),
            occurrence: self.state.type_occurrence,
            caller,
        });
        self.state.control_frames = control_frames;
        self.state.observation.maximum_control_frames = self
            .state
            .observation
            .maximum_control_frames
            .max(control_frames);
        Ok(evaluate(body, Mode::TAIL))
    }

    fn drive(
        &mut self,
        target: AdmittedGraphCall,
        tail: bool,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut next = self.start(target, tail, None)?;
        loop {
            self.state.control.check()?;
            next = match next {
                Next::Continue(evaluation) => self.expression(evaluation)?,
                Next::Break(value) => {
                    if self.activations.is_empty() {
                        return Ok(value);
                    }
                    self.resume(value)?
                }
            };
        }
    }

    fn expression(&mut self, evaluation: Evaluation) -> Result<Next, ExecutionError> {
        let Evaluation { expression, mode } = evaluation;
        if mode.tail {
            self.state.type_occurrence =
                Some((self.state.active_package, OwnerKey::Expression(expression)));
        } else {
            let control_frames = self
                .state
                .control_frames
                .checked_add(1)
                .ok_or_else(allocation_error)?;
            let context = ExpressionContext {
                occurrence: self.state.type_occurrence,
                demand: self.state.borrow_result_demand,
            };
            self.push(Continuation::Expression(context))?;
            self.state.type_occurrence =
                Some((self.state.active_package, OwnerKey::Expression(expression)));
            self.state.borrow_result_demand = mode.demand;
            self.state.control_frames = control_frames;
            self.state.observation.maximum_control_frames = self
                .state
                .observation
                .maximum_control_frames
                .max(control_frames);
        }
        let activation = self.activations.last().ok_or_else(invariant_error)?;
        self.state.observe_locals(&activation.graph.locals);
        let operation = self.state.read_expression(expression)?;
        self.operation(operation, mode)
    }

    fn slot_charge(&mut self) -> Result<(), ExecutionError> {
        self.state.charge_allocation(
            (std::mem::size_of::<CheckedValue>() - std::mem::size_of::<NormalizedValue>()) as u64,
        )
    }

    fn use_expression(
        &mut self,
        expression: ExpressionId,
        use_mode: ParameterUse,
    ) -> Result<Next, ExecutionError> {
        if let Some(local) = self.state.argument_local(expression)? {
            let activation = self.activations.last_mut().ok_or_else(invariant_error)?;
            self.state
                .local_argument_with_use(local, &mut activation.graph.locals, use_mode)
                .map(Next::Break)
        } else {
            self.push(Continuation::ArgumentUse(use_mode))?;
            Ok(evaluate(expression, Mode::VALUE))
        }
    }

    fn operation(
        &mut self,
        operation: ExpressionOperation,
        mode: Mode,
    ) -> Result<Next, ExecutionError> {
        match operation {
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                if !mode.tail {
                    self.slot_charge()?;
                }
                self.push(Continuation::Branch {
                    when_true,
                    when_false,
                    mode,
                })?;
                Ok(evaluate(condition, Mode::VALUE))
            }
            ExpressionOperation::Let { bindings, body } => {
                if !mode.tail {
                    self.slot_charge()?;
                }
                let has_view = bindings.iter().try_fold(false, |found, binding| {
                    Ok::<_, ExecutionError>(found || matches!(self.state.owner(OwnerKey::Binding(*binding))?, Some(OwnerRecord::Binding(record)) if record.kind == BindingKind::ReadView))
                })?;
                if has_view {
                    return Err(invariant_error());
                }
                let mut scoped = Vec::new();
                if !mode.tail {
                    reserve_many(&mut scoped, bindings.len(), self.state.control)?;
                }
                self.advance_let(LetContinuation {
                    bindings,
                    next: 0,
                    pending: None,
                    scoped,
                    body,
                    mode,
                })
            }
            ExpressionOperation::Sequence { items } => {
                if !mode.tail {
                    self.slot_charge()?;
                }
                if items.is_empty() {
                    return Err(reference_error(
                        "normalized_reference_sequence_empty",
                        "canonical sequence has no result expression",
                    ));
                }
                self.advance_sequence(SequenceContinuation {
                    items,
                    next: 0,
                    mode,
                })
            }
            ExpressionOperation::Call {
                read_scope_arguments,
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => {
                if !mode.tail {
                    self.slot_charge()?;
                    self.state.require_owning_result(function)?;
                }
                let uses = self.state.function_parameter_uses(function)?;
                if uses.len() != arguments.len() {
                    return Err(reference_type_error(
                        "call arguments disagree with their exact parameter uses",
                    ));
                }
                let mut values = Vec::new();
                reserve_many(&mut values, arguments.len(), self.state.control)?;
                self.advance_call(CallEvaluation {
                    declaration: function,
                    types: type_arguments,
                    effects: effect_arguments,
                    requirements: requirement_arguments,
                    scopes: read_scope_arguments,
                    expressions: arguments,
                    uses,
                    values,
                    next: 0,
                    tail: mode.tail,
                })
            }
            ExpressionOperation::Invoke { callee, arguments } => {
                if !mode.tail {
                    self.slot_charge()?;
                }
                let invocation = self.state.admit_canonical_invocation(callee, &arguments)?;
                let pending = InvokeEvaluation {
                    invocation,
                    expressions: arguments,
                    callee: None,
                    values: Vec::new(),
                    next: 0,
                    tail: mode.tail,
                };
                frontier_reserve(&mut self.invokes, self.state.control)?;
                frontier_reserve(&mut self.continuations, self.state.control)?;
                let index = self.invokes.len();
                self.invokes.push(pending);
                self.continuations.push(Continuation::InvokeCallee(index));
                Ok(evaluate(callee, Mode::VALUE))
            }
            ExpressionOperation::Bind { callee, arguments } => {
                self.slot_charge()?;
                self.push(Continuation::BindCallee(arguments))?;
                Ok(evaluate(callee, Mode::VALUE))
            }
            ExpressionOperation::Record {
                nominal_type,
                fields,
                type_arguments,
            } => {
                self.slot_charge()?;
                let mut values = Vec::new();
                super::super::child_storage::reserve(&mut values, fields.len())?;
                self.advance_record(RecordContinuation {
                    nominal_type,
                    type_arguments,
                    fields,
                    values,
                    next: 0,
                })
            }
            ExpressionOperation::List { items, .. } => {
                self.slot_charge()?;
                let mut values = Vec::new();
                reserve_many(&mut values, items.len(), self.state.control)?;
                self.advance_list(ListContinuation {
                    items,
                    values,
                    next: 0,
                })
            }
            ExpressionOperation::Field { value, selector } => {
                self.slot_charge()?;
                let product_local = match self.state.owner(OwnerKey::Expression(value))? {
                    Some(OwnerRecord::Expression(record)) => match record.operation {
                        ExpressionOperation::Local { value: local } => self
                            .activations
                            .last()
                            .ok_or_else(invariant_error)?
                            .graph
                            .locals
                            .get(&local)
                            .is_some_and(|value| {
                                matches!(value.raw(), NormalizedValue::OwnedProduct(_))
                            }),
                        _ => false,
                    },
                    _ => false,
                };
                self.push(Continuation::Field(selector))?;
                if product_local {
                    self.use_expression(value, ParameterUse::Borrow)
                } else {
                    Ok(evaluate(value, Mode::VALUE))
                }
            }
            operation @ (ExpressionOperation::Unit {}
            | ExpressionOperation::Bool { .. }
            | ExpressionOperation::I64 { .. }
            | ExpressionOperation::F64 { .. }
            | ExpressionOperation::Text { .. }
            | ExpressionOperation::StaticText { .. }
            | ExpressionOperation::Local { .. }
            | ExpressionOperation::FunctionValue { .. }) => {
                // These leaf owners perform their original checked-slot charge.
                // None invokes a child expression or graph function.
                let activation = self.activations.last_mut().ok_or_else(invariant_error)?;
                self.state
                    .evaluate_operation(operation, &mut activation.graph.locals)
                    .map(Next::Break)
            }
            // A contradictory read provider must not change the chosen execution
            // route in the middle of a selected activation.
            _ => Err(invariant_error()),
        }
    }

    fn advance_let(&mut self, mut pending: LetContinuation) -> Result<Next, ExecutionError> {
        if let Some(&binding) = pending.bindings.get(pending.next) {
            // Keep the canonical binding kind/read boundary, including the old
            // view probe. The whole-body selection has excluded view bindings.
            let view_binding = matches!(self.state.owner(OwnerKey::Binding(binding))?, Some(OwnerRecord::Binding(record)) if record.kind == BindingKind::ReadView);
            if view_binding {
                return Err(invariant_error());
            }
            let record = self.state.binding(binding, BindingKind::Let)?;
            let expression = record.value.ok_or_else(|| {
                reference_error(
                    "normalized_reference_let_value",
                    "canonical let binding has no value expression",
                )
            })?;
            pending.pending = Some(binding);
            self.push(Continuation::Let(pending))?;
            Ok(evaluate(expression, Mode::VALUE))
        } else {
            if !pending.mode.tail {
                self.push(Continuation::LetExit(pending.scoped))?;
            }
            Ok(evaluate(pending.body, pending.mode))
        }
    }

    fn advance_sequence(
        &mut self,
        mut pending: SequenceContinuation,
    ) -> Result<Next, ExecutionError> {
        let expression = *pending
            .items
            .get(pending.next)
            .ok_or_else(invariant_error)?;
        pending.next += 1;
        if pending.next == pending.items.len() {
            Ok(evaluate(expression, pending.mode))
        } else {
            self.push(Continuation::Sequence(pending))?;
            Ok(evaluate(expression, Mode::VALUE))
        }
    }

    fn advance_record(&mut self, mut pending: RecordContinuation) -> Result<Next, ExecutionError> {
        if let Some(field) = pending.fields.get(pending.next) {
            let expression = field.value;
            pending.next += 1;
            self.push(Continuation::Record(pending))?;
            Ok(evaluate(expression, Mode::VALUE))
        } else {
            let types = self.state.resolve_type_arguments(&pending.type_arguments)?;
            self.state
                .record(
                    pending.nominal_type,
                    &types,
                    pending.fields.into_iter().map(|field| field.selector),
                    pending.values,
                )
                .map(Next::Break)
        }
    }

    fn advance_list(&mut self, mut pending: ListContinuation) -> Result<Next, ExecutionError> {
        if let Some(&expression) = pending.items.get(pending.next) {
            pending.next += 1;
            self.push(Continuation::List(pending))?;
            Ok(evaluate(expression, Mode::VALUE))
        } else {
            self.state
                .charge_items(pending.values.len(), std::mem::size_of::<NormalizedValue>())?;
            self.state.list_value(pending.values).map(Next::Break)
        }
    }

    fn advance_call(&mut self, mut pending: CallEvaluation) -> Result<Next, ExecutionError> {
        if let Some(&expression) = pending.expressions.get(pending.next) {
            let use_mode = *pending.uses.get(pending.next).ok_or_else(invariant_error)?;
            pending.next += 1;
            frontier_reserve(&mut self.calls, self.state.control)?;
            frontier_reserve(&mut self.continuations, self.state.control)?;
            let index = self.calls.len();
            self.calls.push(pending);
            self.continuations.push(Continuation::CallArgument(index));
            self.use_expression(expression, use_mode)
        } else {
            self.dispatch(
                pending.declaration,
                ReferenceApplication {
                    types: &pending.types,
                    effects: &pending.effects,
                    requirements: &pending.requirements,
                    implementations: &[],
                    scopes: &pending.scopes,
                },
                pending.values,
                ReferenceCallRoute::Direct,
                pending.tail,
            )
        }
    }

    fn advance_invoke(&mut self, mut pending: InvokeEvaluation) -> Result<Next, ExecutionError> {
        if let Some(&expression) = pending.expressions.get(pending.next) {
            pending.next += 1;
            frontier_reserve(&mut self.invokes, self.state.control)?;
            frontier_reserve(&mut self.continuations, self.state.control)?;
            let index = self.invokes.len();
            self.invokes.push(pending);
            self.continuations.push(Continuation::InvokeArgument(index));
            Ok(evaluate(expression, Mode::VALUE))
        } else {
            let callee = pending.callee.take().ok_or_else(invariant_error)?;
            let (declaration, types, effects, requirements, arguments) =
                self.state.callable_arguments(callee, pending.values)?;
            self.dispatch(
                declaration,
                ReferenceApplication {
                    types: &types,
                    effects: &effects,
                    requirements: &requirements,
                    implementations: &[],
                    scopes: &[],
                },
                arguments,
                ReferenceCallRoute::Invocation(pending.invocation.as_ref()),
                pending.tail,
            )
        }
    }

    fn advance_bind(&mut self, mut pending: BindEvaluation) -> Result<Next, ExecutionError> {
        if let Some(&expression) = pending.expressions.get(pending.next) {
            pending.next += 1;
            frontier_reserve(&mut self.binds, self.state.control)?;
            frontier_reserve(&mut self.continuations, self.state.control)?;
            let index = self.binds.len();
            self.binds.push(pending);
            self.continuations.push(Continuation::BindArgument(index));
            Ok(evaluate(expression, Mode::VALUE))
        } else {
            self.state.finish_bind(pending.binding).map(Next::Break)
        }
    }

    fn resume(&mut self, value: CheckedValue) -> Result<Next, ExecutionError> {
        let base = self.activations.last().ok_or_else(invariant_error)?.base;
        if self.continuations.len() == base {
            return self.complete(value);
        }
        let continuation = self.continuations.pop().ok_or_else(invariant_error)?;
        match continuation {
            Continuation::Expression(context) => {
                let activation = self.activations.last().ok_or_else(invariant_error)?;
                self.state.observe_locals(&activation.graph.locals);
                self.restore_expression(context);
                Ok(Next::Break(value))
            }
            Continuation::ArgumentUse(use_mode) => self
                .state
                .finish_argument_use(value, None, use_mode)
                .map(Next::Break),
            Continuation::Branch {
                when_true,
                when_false,
                mode,
            } => match value.release() {
                NormalizedValue::Bool(true) => Ok(evaluate(when_true, mode)),
                NormalizedValue::Bool(false) => Ok(evaluate(when_false, mode)),
                _ => Err(reference_type_error("if condition is not boolean")),
            },
            Continuation::Let(mut pending) => {
                let binding = pending.pending.take().ok_or_else(invariant_error)?;
                if matches!(value.raw(), NormalizedValue::ReadView(_)) {
                    return Err(reference_type_error(if pending.mode.tail {
                        "ordinary binding cannot conceal a view"
                    } else {
                        "ordinary Let cannot acquire a read-view occurrence"
                    }));
                }
                let local = LocalValueReference::LexicalBinding(binding);
                let activation = self.activations.last_mut().ok_or_else(invariant_error)?;
                reserve(&mut activation.graph.lexical, self.state.control)?;
                if activation.graph.locals.insert(local, value).is_some() {
                    return Err(reference_error(
                        "normalized_reference_local_duplicate",
                        "canonical local identity was bound twice in one scope",
                    ));
                }
                activation.graph.lexical.push(local);
                if !pending.mode.tail {
                    pending.scoped.push(local);
                }
                pending.next += 1;
                self.advance_let(pending)
            }
            Continuation::LetExit(scoped) => {
                let activation = self.activations.last_mut().ok_or_else(invariant_error)?;
                for local in scoped.into_iter().rev() {
                    activation.graph.locals.remove(&local);
                }
                Ok(Next::Break(value))
            }
            Continuation::Sequence(pending) => {
                drop(value);
                self.advance_sequence(pending)
            }
            Continuation::CallArgument(index) => {
                let mut pending = take_pending(&mut self.calls, index)?;
                pending.values.push(value);
                self.advance_call(pending)
            }
            Continuation::BindCallee(expressions) => {
                let binding = self.state.begin_bind(value, expressions.len())?;
                self.advance_bind(BindEvaluation {
                    binding,
                    expressions,
                    next: 0,
                })
            }
            Continuation::BindArgument(index) => {
                let mut pending = take_pending(&mut self.binds, index)?;
                self.state
                    .append_bind_capture(&mut pending.binding, value)?;
                self.advance_bind(pending)
            }
            Continuation::InvokeCallee(index) => {
                let mut pending = take_pending(&mut self.invokes, index)?;
                pending.callee = Some(value);
                reserve_many(
                    &mut pending.values,
                    pending.expressions.len(),
                    self.state.control,
                )?;
                self.advance_invoke(pending)
            }
            Continuation::InvokeArgument(index) => {
                let mut pending = take_pending(&mut self.invokes, index)?;
                pending.values.push(value);
                self.advance_invoke(pending)
            }
            Continuation::Record(mut pending) => {
                pending.values.push(value);
                self.advance_record(pending)
            }
            Continuation::List(mut pending) => {
                pending.values.push(value);
                self.advance_list(pending)
            }
            Continuation::Field(selector) => self.state.field(value, selector).map(Next::Break),
        }
    }

    fn dispatch(
        &mut self,
        declaration: DeclarationReference,
        application: ReferenceApplication<'_>,
        arguments: Vec<CheckedValue>,
        route: ReferenceCallRoute<'_>,
        tail: bool,
    ) -> Result<Next, ExecutionError> {
        if tail {
            // Exactly the old tail admission order. External tails still take
            // an ordinary synchronous call; graph transfers are admitted once
            // against this outgoing canonical scope and invocation proof.
            self.state.require_owning_result(declaration)?;
            let callable = self.state.declaration(declaration)?;
            if let DeclarationPayload::Function(function) = callable.payload {
                let types = self.state.resolve_type_arguments(application.types)?;
                let effects = self.state.resolve_effect_arguments(application.effects)?;
                let requirements = self
                    .state
                    .resolve_requirement_arguments(application.requirements)?;
                let scopes = self
                    .state
                    .resolve_read_scope_arguments(application.scopes)?;
                let target = self.state.admit_graph_call_with_implementations_on_route(
                    declaration,
                    function,
                    arguments,
                    ReferenceApplication {
                        types: &types,
                        effects: &effects,
                        requirements: &requirements,
                        implementations: &[],
                        scopes: &scopes,
                    },
                    route,
                )?;
                return self.transfer(target);
            }
        }
        let resolved = self
            .state
            .begin_declaration_call(declaration, application)?;
        self.transient_call = Some(resolved.previous_package);
        let prepared =
            self.state
                .prepare_activation(declaration, resolved.application(), arguments, route);
        match prepared {
            Ok(ActivationDispatch::Break(value)) => {
                self.finish_transient_call();
                Ok(Next::Break(value))
            }
            Ok(ActivationDispatch::Continue(target)) => {
                self.enter_nested(target, false, resolved.previous_package)
            }
            Err(error) => {
                self.finish_transient_call();
                Err(error)
            }
        }
    }

    // The caller has already incremented call depth. Every branch owns the
    // matching decrement, including routing/allocation failure before entry.
    fn enter_nested(
        &mut self,
        target: AdmittedGraphCall,
        tail: bool,
        caller: PackageId,
    ) -> Result<Next, ExecutionError> {
        self.transient_call = Some(caller);
        let supported = match self.supported(&target) {
            Ok(supported) => supported,
            Err(error) => {
                self.finish_transient_call();
                return Err(error);
            }
        };
        if supported {
            match self.start(target, tail, Some(caller)) {
                Ok(next) => {
                    self.transient_call = None;
                    Ok(next)
                }
                Err(error) => {
                    self.finish_transient_call();
                    Err(error)
                }
            }
        } else {
            let result = self.legacy(target, tail);
            self.finish_transient_call();
            result.map(Next::Break)
        }
    }

    fn legacy(
        &mut self,
        target: AdmittedGraphCall,
        tail: bool,
    ) -> Result<CheckedValue, ExecutionError> {
        let mut step = self.state.enter_graph_call_recursive(target, tail);
        loop {
            match step {
                Ok(ReferenceStep::Break(value)) => return Ok(value),
                Ok(ReferenceStep::Continue(target)) => {
                    step = self.state.enter_graph_call(*target, true);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn transfer(&mut self, target: AdmittedGraphCall) -> Result<Next, ExecutionError> {
        // This is the existing modeled tail-target charge even though this
        // driver's pending target has no recursively linked Box allocation.
        self.state
            .charge_allocation(std::mem::size_of::<AdmittedGraphCall>() as u64)?;
        self.state.control.check()?;
        let activation = self.activations.last().ok_or_else(invariant_error)?;
        let retained = self.state.lexical_loan_scopes != 0
            || target.function.result.relation.source().copied().is_some()
            || self
                .state
                .borrow_result_sources
                .last()
                .is_some_and(Option::is_some)
            || activation
                .graph
                .locals
                .values()
                .any(|value| value.raw().memory_owns_live_loans());
        if retained {
            if self
                .state
                .call_depth
                .saturating_add(self.state.ancestor_depth)
                >= self.state.policy.maximum_call_depth
            {
                return Err(reference_resource(
                    "normalized_reference_call_depth",
                    "retained memory loan scope exceeded call-depth limit",
                ));
            }
            self.state.call_depth += 1;
            self.state.observation.maximum_call_depth =
                self.state.observation.maximum_call_depth.max(
                    self.state
                        .call_depth
                        .saturating_add(self.state.ancestor_depth),
                );
            let caller = self.state.active_package;
            return self.enter_nested(target, false, caller);
        }
        let supported = self.supported(&target)?;
        let outgoing = self.activations.pop().ok_or_else(invariant_error)?;
        if self.continuations.len() != outgoing.base {
            // No non-tail expression can acquire a tail mode. Retain scope
            // ownership for RAII cleanup if an internal invariant is violated.
            self.activations.push(outgoing);
            return Err(invariant_error());
        }
        self.state.control_frames -= 1;
        self.state.type_occurrence = outgoing.occurrence;
        let caller = outgoing.caller;
        self.state.close_graph_activation(outgoing.graph);
        self.transient_call = caller;
        let result = if supported {
            self.start(target, true, caller)
        } else {
            let result = self.legacy(target, true).map(Next::Break);
            self.finish_transient_call();
            return result;
        };
        if result.is_err() {
            self.finish_transient_call();
        } else {
            self.transient_call = None;
        }
        result
    }

    fn complete(&mut self, value: CheckedValue) -> Result<Next, ExecutionError> {
        let activation = self.activations.pop().ok_or_else(invariant_error)?;
        self.state.control_frames -= 1;
        self.state.type_occurrence = activation.occurrence;
        let result = self
            .state
            .finish_graph_activation(activation.graph, Ok(ReferenceStep::Break(value)));
        if let Some(caller) = activation.caller {
            self.state.end_declaration_call(caller);
        }
        match result? {
            ReferenceStep::Break(value) => Ok(Next::Break(value)),
            ReferenceStep::Continue(_) => Err(invariant_error()),
        }
    }

    fn finish_transient_call(&mut self) {
        if let Some(caller) = self.transient_call.take() {
            self.state.end_declaration_call(caller);
        }
    }

    fn restore_expression(&mut self, context: ExpressionContext) {
        self.state.control_frames -= 1;
        self.state.borrow_result_demand = context.demand;
        self.state.type_occurrence = context.occurrence;
    }

    fn discard(
        &mut self,
        continuation: Continuation,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) {
        match continuation {
            Continuation::Expression(context) => self.restore_expression(context),
            Continuation::LetExit(scoped) => {
                for local in scoped.into_iter().rev() {
                    locals.remove(&local);
                }
            }
            Continuation::Let(pending) => {
                for local in pending.scoped.into_iter().rev() {
                    locals.remove(&local);
                }
            }
            Continuation::CallArgument(_) => {
                self.calls.pop();
            }
            Continuation::BindArgument(_) => {
                self.binds.pop();
            }
            Continuation::InvokeCallee(_) | Continuation::InvokeArgument(_) => {
                self.invokes.pop();
            }
            // Other frames own only metadata or checked values whose existing
            // drop protocol drains opaque children without new allocation.
            _ => {}
        }
    }
}

impl Drop for Machine<'_, '_> {
    fn drop(&mut self) {
        // Each activation retains its complete canonical scope until every
        // younger expression/pending-argument owner has been destroyed. This
        // also runs if a host unwinds, and never allocates or checks cancellation.
        self.finish_transient_call();
        while let Some(mut activation) = self.activations.pop() {
            while self.continuations.len() > activation.base {
                if let Some(continuation) = self.continuations.pop() {
                    self.discard(continuation, &mut activation.graph.locals);
                }
            }
            self.state.control_frames -= 1;
            self.state.type_occurrence = activation.occurrence;
            self.state.close_graph_activation(activation.graph);
            if let Some(caller) = activation.caller {
                self.state.end_declaration_call(caller);
            }
        }
        // Publication of a side-stack entry and its continuation tag is atomic
        // after all capacity is reserved; ordinary unwinding empties these.
        while self.continuations.pop().is_some() {}
        while self.calls.pop().is_some() {}
        while self.invokes.pop().is_some() {}
        while self.binds.pop().is_some() {}
    }
}

fn take_pending<T>(pending: &mut Vec<T>, index: usize) -> Result<T, ExecutionError> {
    if index.checked_add(1) != Some(pending.len()) {
        return Err(invariant_error());
    }
    pending.pop().ok_or_else(invariant_error)
}

#[cfg(test)]
thread_local! {
    static FRONTIER_REFUSAL: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) struct FrontierRefusal(Option<usize>);

#[cfg(test)]
pub(crate) fn refuse_frontier_after(allowed_reservations: usize) -> FrontierRefusal {
    FrontierRefusal(FRONTIER_REFUSAL.replace(Some(allowed_reservations)))
}

#[cfg(test)]
impl Drop for FrontierRefusal {
    fn drop(&mut self) {
        FRONTIER_REFUSAL.set(self.0);
    }
}

fn reservation_probe() -> Result<(), ExecutionError> {
    #[cfg(test)]
    if let Some(remaining) = FRONTIER_REFUSAL.get() {
        if remaining == 0 {
            return Err(allocation_error());
        }
        FRONTIER_REFUSAL.set(Some(remaining - 1));
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static FORCE_LEGACY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) struct LegacyRoute(bool);

/// Differential policy controls use the old canonical evaluator in this same
/// binary. This is not a production switch or an alternative program oracle.
#[cfg(test)]
pub(crate) fn force_legacy_route() -> LegacyRoute {
    LegacyRoute(FORCE_LEGACY.replace(true))
}

#[cfg(test)]
impl Drop for LegacyRoute {
    fn drop(&mut self) {
        FORCE_LEGACY.set(self.0);
    }
}
