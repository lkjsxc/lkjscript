//! Structured child calls. Only sealed custody enters a fresh child invocation.
use super::super::{parallel, shared_budget::SharedBudget};
use super::*;

struct ChildOutcome {
    result: Result<transfer::TransferResult, ExecutionError>,
    observation: NormalizedRunObservation,
}

#[derive(Clone)]
struct ChildContext {
    program: Arc<NormalizedProgram>,
    executor: StructuredExecutorHandle,
    policy: NormalizedRunPolicy,
    control: ExecutionControl,
    parent_domain: super::super::value::ValueOrigin,
    structured_depth: usize,
    ancestor_depth: usize,
}

impl Machine<'_> {
    /// Retain the complete checked read placement, including leaf and ancestor
    /// guards, until the destination invocation has released its read lease.
    /// The outbound token is a new read alias, never an unchecked raw extraction
    /// of a borrowed-result packet.
    fn prepare_scoped_arguments(
        &mut self,
        values: Vec<CheckedValue>,
    ) -> Result<Vec<transfer::ScopedArgument>, ExecutionError> {
        self.charge_allocation(super::super::value::collection_storage_bytes(
            values.len() as u64,
            std::mem::size_of::<transfer::ScopedArgument>() as u64,
            "normalized_parallel_arguments",
        )?)?;
        let reads = values
            .iter()
            .filter(|value| value.raw().memory_is_borrowed())
            .count();
        self.charge_allocation(super::super::value::collection_storage_bytes(
            reads as u64,
            transfer::ScopedReadCustody::ALLOCATION_BYTES
                + std::mem::size_of::<CheckedValue>() as u64,
            "normalized_parallel_reads",
        )?)?;
        let mut arguments = Vec::new();
        arguments.try_reserve_exact(values.len()).map_err(|_| {
            resource_error(
                "normalized_parallel_arguments",
                "structured capture reservation failed",
            )
        })?;
        for value in values {
            arguments.push(if value.raw().memory_is_borrowed() {
                let read = value.raw().memory_borrow()?;
                transfer::ScopedArgument::Read {
                    value: read,
                    custody: transfer::ScopedReadCustody::new(value),
                }
            } else {
                transfer::ScopedArgument::Value(value.into_raw())
            });
        }
        Ok(arguments)
    }

    /// Child actuals are authored in the caller's scope. Resolve them before the
    /// sealed application certificate is formed, reserving the new vectors first.
    /// A closed caller retains the original shared operand arrays.
    pub(super) fn resolve_parallel_types(
        &mut self,
        types: Arc<[TypeObjectDigest]>,
    ) -> Result<Arc<[TypeObjectDigest]>, ExecutionError> {
        if types.is_empty() || self.current_frame()?.type_arguments.is_empty() {
            return Ok(types);
        }
        self.charge_allocation(super::super::value::collection_storage_bytes(
            types.len() as u64,
            (2 * std::mem::size_of::<TypeObjectDigest>() + 2 * std::mem::size_of::<usize>()) as u64,
            "normalized_parallel_application",
        )?)?;
        let bindings = &self.current_frame()?.type_arguments;
        types
            .iter()
            .map(|ty| transfer::resolve_type(self.program, *ty, bindings, self.control))
            .collect::<Result<Vec<_>, _>>()
            .map(Into::into)
    }

    fn inspect_transfer_data(
        &mut self,
        raw: &NormalizedValue,
        ty: TypeObjectDigest,
    ) -> Result<(), ExecutionError> {
        let class = checked::Admission {
            shared_budget: self.shared_budget.as_deref(),
            substitutions: &BTreeMap::new(),
            program: self.program,
            resources: self.resources,
            control: self.control,
            policy: self.policy,
            admission_bytes: 0,
            admission_items: 0,
            work: &mut self.observation.value_work,
            allocated: &mut self.observation.allocated_bytes,
            allocation_charges: &mut self.observation.allocation_charges,
            items: &mut self.observation.collection_items,
        }
        .inspect(raw, ty, None, true)?;
        if class != Class::Free {
            return Err(type_error("structured child metadata contains authority"));
        }
        Ok(())
    }

    pub(super) fn parallel(
        &mut self,
        left: (FunctionIndex, Arc<[TypeObjectDigest]>),
        left_values: Vec<CheckedValue>,
        right: (FunctionIndex, Arc<[TypeObjectDigest]>),
        right_values: Vec<CheckedValue>,
        result_type: TypeObjectDigest,
    ) -> Result<CheckedValue, ExecutionError> {
        self.admit_task_call(&[])?;
        let left_index = left.0;
        let right_index = right.0;
        let left_types = Arc::clone(&left.1);
        let right_types = Arc::clone(&right.1);
        let left = transfer::TaskApplication::bind(
            self.program,
            left.0,
            left.1,
            self.control,
            &mut |bytes| self.charge_allocation(bytes),
        )?;
        let right = transfer::TaskApplication::bind(
            self.program,
            right.0,
            right.1,
            self.control,
            &mut |bytes| self.charge_allocation(bytes),
        )?;
        let left_type = left.result();
        let right_type = right.result();
        let owned = !self.program.comparable_types.contains(&left_type)
            || !self.program.comparable_types.contains(&right_type);
        let fields = match self.program.types.get(&result_type).map(|ty| &ty.form) {
            Some(TypeForm::OwnedProduct { fields }) if owned => fields,
            Some(TypeForm::StructuralRecord { fields }) if !owned => fields,
            _ => return Err(type_error("structured result has an invalid pair type")),
        };
        if fields.len() != 2
            || fields[0].name.as_str() != "left"
            || fields[0].ty != left_type
            || fields[1].name.as_str() != "right"
            || fields[1].ty != right_type
        {
            return Err(type_error(
                "structured result differs from exact child types",
            ));
        }
        self.observation.parallel_scopes = self.observation.parallel_scopes.saturating_add(1);
        if self.structured_depth >= parallel::MAXIMUM_STRUCTURED_DEPTH {
            return Err(resource_error(
                "normalized_parallel_depth",
                "structured evaluator nesting exceeds its live-stack capacity",
            ));
        }
        if self.shared_budget.is_none() {
            self.charge_allocation(
                (std::mem::size_of::<SharedBudget>() + 2 * std::mem::size_of::<usize>()) as u64,
            )?;
            self.shared_budget = Some(Arc::new(SharedBudget::new(
                self.policy,
                self.remaining_steps,
                self.observation.allocated_bytes,
                self.observation.collection_items,
            )));
        }
        let budget = self
            .shared_budget
            .as_ref()
            .cloned()
            .ok_or_else(|| type_error("missing structured budget"))?;
        let count = left_values
            .len()
            .checked_add(right_values.len())
            .ok_or_else(|| type_error("structured argument count overflow"))?;
        self.charge_allocation(super::super::value::collection_storage_bytes(
            count as u64,
            (std::mem::size_of::<NormalizedValue>()
                + std::mem::size_of::<transfer::ScopedReadLease>()) as u64,
            "normalized_parallel_arguments",
        )?)?;
        let left_domain = super::super::value::ValueOrigin::fresh().ok_or_else(|| {
            resource_error(
                "normalized_parallel_origin",
                "structured destination identity exhausted",
            )
        })?;
        let right_domain = super::super::value::ValueOrigin::fresh().ok_or_else(|| {
            resource_error(
                "normalized_parallel_origin",
                "structured destination identity exhausted",
            )
        })?;
        let program = self.program;
        let control = self.control;
        let source = self.memory_domain;
        let left_values = self.prepare_scoped_arguments(left_values)?;
        let right_values = self.prepare_scoped_arguments(right_values)?;
        let left = transfer::TransferArguments::seal_scoped_applied(
            program,
            source,
            left_domain,
            left,
            left_values,
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let right = transfer::TransferArguments::seal_scoped_applied(
            program,
            source,
            right_domain,
            right,
            right_values,
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let context = ChildContext {
            program: Arc::clone(self.program_owner),
            executor: self.executor.clone(),
            policy: self.policy,
            control: control.clone(),
            parent_domain: source,
            structured_depth: self.structured_depth + 1,
            ancestor_depth: self.ancestor_depth.saturating_add(self.frames.len()),
        };
        let left_budget = Arc::clone(&budget);
        let left_context = context.clone();
        let pair = parallel::run(
            self.executor,
            control,
            move || child(left_context, left_budget, left_domain, left),
            move || {
                let _lists = super::super::list::WorkScope::enter();
                let _maps = super::super::map::WorkScope::enter();
                child(context, budget, right_domain, right)
            },
        )?;
        if pair.dispatched {
            self.observation.parallel_worker_dispatches = self
                .observation
                .parallel_worker_dispatches
                .saturating_add(1);
        } else {
            self.observation.parallel_inline_fallbacks =
                self.observation.parallel_inline_fallbacks.saturating_add(1);
        }
        super::super::list::Work::include_joined(pair.right.observation.value_work.lists);
        super::super::map::Work::include_joined(pair.right.observation.value_work.maps);
        self.observation.include_child(&pair.left.observation);
        self.observation.include_child(&pair.right.observation);
        let (left, right) = super::super::parallel::results(pair.left.result, pair.right.result)?;
        self.control.check()?;
        if owned {
            self.charge_collection(2, std::mem::size_of::<NormalizedValue>())?;
            self.charge_allocation(super::super::owned_product::OwnedProduct::ALLOCATION_BYTES)?;
            let left =
                left.adopt_applied(program, source, left_index, &left_types, left_type, control)?;
            let left = self.product_child(left, left_type)?;
            let right = right.adopt_applied(
                program,
                source,
                right_index,
                &right_types,
                right_type,
                control,
            )?;
            let right = self.product_child(right, right_type)?;
            // The complete wrapper storage was reserved before either adoption.
            let token = super::super::owned_product::OwnedProduct::create(
                source,
                result_type,
                vec![left.into_raw(), right.into_raw()],
                control,
                &mut |_| Ok(()),
            )?;
            token.establish_admission(program.value_origin)?;
            return CheckedValue::memory(program, NormalizedValue::OwnedProduct(token));
        }
        self.charge_collection(2, std::mem::size_of::<(Name, NormalizedValue)>())?;
        self.charge_allocation(9)?;
        let left_name = Name::new("left").map_err(|_| type_error("invalid structured field"))?;
        let right_name = Name::new("right").map_err(|_| type_error("invalid structured field"))?;
        let left =
            left.adopt_applied(program, source, left_index, &left_types, left_type, control)?;
        let left = self.admit(left, left_type, None, true)?;
        let right = right.adopt_applied(
            program,
            source,
            right_index,
            &right_types,
            right_type,
            control,
        )?;
        let right = self.admit(right, right_type, None, true)?;
        CheckedValue::record(
            program,
            None,
            vec![(left_name, left), (right_name, right)],
            &mut self.observation.value_work,
        )
    }
}

fn child(
    context: ChildContext,
    budget: Arc<SharedBudget>,
    memory_domain: super::super::value::ValueOrigin,
    envelope: transfer::TransferArguments,
) -> ChildOutcome {
    let ChildContext {
        program,
        executor,
        policy,
        control,
        parent_domain,
        structured_depth,
        ancestor_depth,
    } = context;
    let program_owner = &program;
    let program = program.as_ref();
    let control = &control;
    let resources = match NormalizedResourceScope::new() {
        Ok(resources) => resources,
        Err(error) => {
            control.cancel();
            return ChildOutcome {
                result: Err(error),
                observation: NormalizedRunObservation::default(),
            };
        }
    };
    let lists = super::super::list::Work::current();
    let maps = super::super::map::Work::current();
    // Declared before the machine so unwind also destroys all child views and
    // activations before releasing the invocation's lending anchors. These
    // leases outlive tail calls, synchronous forwarding and nested groups.
    let mut read_leases = Vec::new();
    let mut machine = Machine {
        shared_budget: Some(budget),
        structured_depth,
        ancestor_depth,
        memory_domain,
        program,
        program_owner,
        executor: &executor,
        root_allowance: Some(Arc::from([])),
        policy,
        host: None,
        capabilities: None,
        resources: &resources,
        control,
        remaining_steps: policy.instruction_steps,
        stack: Vec::new(),
        frames: Vec::new(),
        next_frame: 0,
        next_transaction: 0,
        transactions: BTreeMap::new(),
        calls_by_requirement: BTreeMap::new(),
        observation: NormalizedRunObservation {
            production_tier: "graph14_dense_bytecode_10",
            ..NormalizedRunObservation::default()
        },
    };
    let result = (|| {
        let (application, values, leases) =
            envelope.adopt_scoped_applied(program, memory_domain, control)?;
        read_leases = leases;
        let function = application.function();
        let parameters = &program
            .functions
            .get(function.0 as usize)
            .ok_or_else(|| type_error("missing structured child"))?
            .parameters;
        machine.charge_allocation(super::super::value::collection_storage_bytes(
            values.len() as u64,
            std::mem::size_of::<CheckedValue>() as u64,
            "normalized_parallel_arguments",
        )?)?;
        let arguments = values
            .into_iter()
            .zip(parameters.iter())
            .enumerate()
            .map(|(index, (value, _parameter))| {
                if value.memory_form().is_some() {
                    CheckedValue::memory(program, value)
                } else {
                    machine.admit(value, application.parameter(program, index)?, None, true)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        #[cfg(test)]
        ChildProbe::enter(program.value_origin, memory_domain)?;
        machine.call(function, Arc::clone(application.types()), arguments)?;
        let value = finish_admitted(&mut machine)?;
        transfer::TransferResult::seal_applied(
            program,
            memory_domain,
            parent_domain,
            application,
            value.into_raw(),
            control,
            &mut |raw, ty| machine.inspect_transfer_data(raw, ty),
        )
    })();
    if result.is_err() {
        control.cancel();
    }
    machine.rollback_all();
    machine.clear_execution_values();
    resources.release_all();
    // Returning a child outcome certifies that its read authority has ended.
    drop(read_leases);
    machine.observation.value_work.lists = lists.since();
    machine.observation.value_work.maps = maps.since();
    ChildOutcome {
        result,
        observation: std::mem::take(&mut machine.observation),
    }
}

#[cfg(test)]
struct ProbeState {
    children: std::sync::Mutex<Vec<(std::thread::ThreadId, super::super::value::ValueOrigin)>>,
    changed: std::sync::Condvar,
}
#[cfg(test)]
fn probes() -> &'static std::sync::Mutex<BTreeMap<super::super::value::ValueOrigin, Arc<ProbeState>>>
{
    static PROBES: std::sync::OnceLock<
        std::sync::Mutex<BTreeMap<super::super::value::ValueOrigin, Arc<ProbeState>>>,
    > = std::sync::OnceLock::new();
    PROBES.get_or_init(|| std::sync::Mutex::new(BTreeMap::new()))
}
#[cfg(test)]
pub(in super::super) struct ChildProbe {
    program: super::super::value::ValueOrigin,
    state: Arc<ProbeState>,
}
#[cfg(test)]
impl ChildProbe {
    pub(in super::super) fn start(program: super::super::value::ValueOrigin) -> Self {
        let state = Arc::new(ProbeState {
            children: std::sync::Mutex::new(Vec::new()),
            changed: std::sync::Condvar::new(),
        });
        probes()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(program, Arc::clone(&state));
        Self { program, state }
    }
    fn enter(
        program: super::super::value::ValueOrigin,
        domain: super::super::value::ValueOrigin,
    ) -> Result<(), ExecutionError> {
        let state = probes()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&program)
            .cloned();
        let Some(state) = state else {
            return Ok(());
        };
        let mut children = state
            .children
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        children.push((std::thread::current().id(), domain));
        state.changed.notify_all();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while children.len() < 2 {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let (next, timeout) = state
                .changed
                .wait_timeout(children, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            children = next;
            if timeout.timed_out() && children.len() < 2 {
                return Err(resource_error(
                    "parallel_probe_timeout",
                    "two admitted graph children did not overlap",
                ));
            }
        }
        Ok(())
    }
    pub(in super::super) fn observed(
        &self,
    ) -> Vec<(std::thread::ThreadId, super::super::value::ValueOrigin)> {
        self.state
            .children
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}
#[cfg(test)]
impl Drop for ChildProbe {
    fn drop(&mut self) {
        probes()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.program);
    }
}

impl NormalizedRunObservation {
    fn include_child(&mut self, child: &Self) {
        macro_rules! add { ($($field:ident),* $(,)?) => { $(self.$field = self.$field.saturating_add(child.$field);)* }; }
        macro_rules! maximum { ($($field:ident),* $(,)?) => { $(self.$field = self.$field.max(child.$field);)* }; }
        add!(
            instructions,
            calls,
            external_calls,
            capability_calls,
            allocated_bytes,
            allocation_charges,
            collection_items,
            tail_transfers,
            parallel_scopes,
            parallel_worker_dispatches,
            parallel_inline_fallbacks
        );
        maximum!(
            maximum_call_depth,
            maximum_value_stack,
            maximum_control_frames,
            maximum_live_locals,
            maximum_live_type_bindings,
            maximum_live_allowances,
            maximum_live_transactions
        );
        super::super::parallel::include_value_work(&mut self.value_work, &child.value_work);
    }
}
