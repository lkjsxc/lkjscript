//! Structured child calls. Only sealed custody enters a fresh child invocation.
use super::super::{parallel, shared_budget::SharedBudget};
use super::*;

struct ChildOutcome {
    result: Result<CheckedValue, ExecutionError>,
    observation: NormalizedRunObservation,
}

#[derive(Clone, Copy)]
struct ChildContext<'a> {
    program: &'a NormalizedProgram,
    policy: NormalizedRunPolicy,
    control: &'a ExecutionControl,
    structured_depth: usize,
    ancestor_depth: usize,
}

impl Machine<'_> {
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
        left: FunctionIndex,
        left_values: Vec<CheckedValue>,
        right: FunctionIndex,
        right_values: Vec<CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        self.admit_task_call(&[])?;
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
            std::mem::size_of::<NormalizedValue>() as u64,
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
        let left = transfer::TransferArguments::seal(
            program,
            source,
            left_domain,
            left,
            left_values
                .into_iter()
                .map(CheckedValue::into_raw)
                .collect(),
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let right = transfer::TransferArguments::seal(
            program,
            source,
            right_domain,
            right,
            right_values
                .into_iter()
                .map(CheckedValue::into_raw)
                .collect(),
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let context = ChildContext {
            program,
            policy: self.policy,
            control,
            structured_depth: self.structured_depth + 1,
            ancestor_depth: self.ancestor_depth.saturating_add(self.frames.len()),
        };
        let left_budget = Arc::clone(&budget);
        let pair = parallel::run(
            control,
            move || child(context, left_budget, left_domain, left),
            move || child(context, budget, right_domain, right),
        )?;
        if pair.spawned {
            self.observation.parallel_workers_spawned =
                self.observation.parallel_workers_spawned.saturating_add(1);
            super::super::list::Work::include_joined(pair.right.observation.value_work.lists);
            super::super::map::Work::include_joined(pair.right.observation.value_work.maps);
        }
        self.observation.include_child(&pair.left.observation);
        self.observation.include_child(&pair.right.observation);
        let (left, right) = super::super::parallel::results(pair.left.result, pair.right.result)?;
        self.control.check()?;
        self.charge_collection(2, std::mem::size_of::<(Name, NormalizedValue)>())?;
        self.charge_allocation(9)?;
        let left_name = Name::new("left").map_err(|_| type_error("invalid structured field"))?;
        let right_name = Name::new("right").map_err(|_| type_error("invalid structured field"))?;
        CheckedValue::record(
            program,
            None,
            vec![(left_name, left), (right_name, right)],
            &mut self.observation.value_work,
        )
    }
}

fn child(
    context: ChildContext<'_>,
    budget: Arc<SharedBudget>,
    memory_domain: super::super::value::ValueOrigin,
    envelope: transfer::TransferArguments,
) -> ChildOutcome {
    let ChildContext {
        program,
        policy,
        control,
        structured_depth,
        ancestor_depth,
    } = context;
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
    let mut machine = Machine {
        shared_budget: Some(budget),
        structured_depth,
        ancestor_depth,
        memory_domain,
        program,
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
        let (function, values) = envelope.adopt(program, memory_domain, control)?;
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
            .map(|(value, parameter)| {
                if value.memory_form().is_some() {
                    CheckedValue::memory(program, value)
                } else {
                    machine.admit(value, parameter.ty, None, true)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        #[cfg(test)]
        ChildProbe::enter(program.value_origin, memory_domain)?;
        machine.call(function, Arc::from([]), arguments)?;
        let value = finish_admitted(&mut machine)?;
        if value.class(program, &mut machine.observation.value_work)? != Class::Free {
            return Err(type_error(
                "structured child returned nonordinary ownership",
            ));
        }
        Ok(value)
    })();
    if result.is_err() {
        control.cancel();
    }
    machine.rollback_all();
    machine.frames.clear();
    machine.stack.clear();
    resources.release_all();
    machine.observation.value_work.lists = lists.since();
    machine.observation.value_work.maps = maps.since();
    ChildOutcome {
        result,
        observation: machine.observation,
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
            parallel_workers_spawned
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
