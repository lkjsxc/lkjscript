//! Independent first-order associated-type growth oracle. Canonical-source adapters
//! can build this explicit model, but must not supply producer-normalized bindings.
//! The oracle substitutes terms/witnesses and uses weighted reachability, without
//! production type substitution, demand extraction, SCC or preparation machinery.
//! Finite concrete closure is evidence for its initial states, never a prefix proof.
//! Canonical view-scope coordinates are proved separately before entering this
//! algebra. Scope forwarding is zero constructor depth; nominal family binding
//! constructors and every effective witness type actual retain the usual edges.

use crate::platform::admission::{Meter, Observation};
use crate::platform::diagnostic::Diagnostic;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

/// Fixed scalar error context; never an admission token or an additional work debit.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ReferencePhaseLedger {
    pub(super) entry: Observation,
    pub(super) phase: &'static str,
    pub(super) after_presence: Option<Observation>,
    pub(super) after_preflight: Option<Observation>,
    pub(super) after_inventory: Option<Observation>,
    pub(super) after_translation: Option<Observation>,
    pub(super) after_raw_admission: Option<Observation>,
    pub(super) after_classifier_admission: Option<Observation>,
    pub(super) after_context_growth: Option<Observation>,
    pub(super) after_reachability: Option<Observation>,
    pub(super) last_completed_contexts: Option<usize>,
    pub(super) last_completed_slots: Option<usize>,
    pub(super) last_completed_edges: Option<usize>,
}

pub(super) enum ReferencePhaseBoundary {
    Presence,
    Preflight,
    Inventory,
    Translation,
    RawAdmission,
    ClassifierAdmission,
    ContextGrowth,
    Reachability,
}

/// One operation-local pair of optional allowances, shared by raw translation
/// and the independent algebraic proof. Modeled cumulative work and storage are
/// passive observations when the corresponding caller allowance is absent.
pub(super) struct Budget<'a> {
    checkpoint: &'a dyn Fn() -> Result<(), Diagnostic>,
    work: RefCell<Meter>,
    bytes: RefCell<Meter>,
    phase_ledger: Cell<ReferencePhaseLedger>,
}

impl<'a> Budget<'a> {
    pub(super) fn new(
        checkpoint: &'a dyn Fn() -> Result<(), Diagnostic>,
        maximum_work: Option<u64>,
        maximum_bytes: Option<u64>,
    ) -> Self {
        Self {
            checkpoint,
            work: RefCell::new(Meter::new(maximum_work)),
            bytes: RefCell::new(Meter::new(maximum_bytes)),
            phase_ledger: Cell::new(ReferencePhaseLedger {
                phase: "initialization",
                ..ReferencePhaseLedger::default()
            }),
        }
    }
    pub(super) fn consumed(&self) -> Observation {
        self.work.borrow().observation()
    }
    pub(super) fn phase_ledger(&self) -> ReferencePhaseLedger {
        self.phase_ledger.get()
    }
    pub(super) fn observe_phase(&self, phase: &'static str) {
        let mut ledger = self.phase_ledger.get();
        ledger.phase = phase;
        self.phase_ledger.set(ledger);
    }
    pub(super) fn observe_boundary(&self, boundary: ReferencePhaseBoundary) {
        let mut ledger = self.phase_ledger.get();
        let work = Some(self.consumed());
        match boundary {
            ReferencePhaseBoundary::Presence => ledger.after_presence = work,
            ReferencePhaseBoundary::Preflight => ledger.after_preflight = work,
            ReferencePhaseBoundary::Inventory => ledger.after_inventory = work,
            ReferencePhaseBoundary::Translation => ledger.after_translation = work,
            ReferencePhaseBoundary::RawAdmission => ledger.after_raw_admission = work,
            ReferencePhaseBoundary::ClassifierAdmission => ledger.after_classifier_admission = work,
            ReferencePhaseBoundary::ContextGrowth => ledger.after_context_growth = work,
            ReferencePhaseBoundary::Reachability => ledger.after_reachability = work,
        }
        self.phase_ledger.set(ledger);
    }
    pub(super) fn observe_progress(&self, contexts: usize, slots: usize, edges: usize) {
        let mut ledger = self.phase_ledger.get();
        ledger.last_completed_contexts = Some(contexts);
        ledger.last_completed_slots = Some(slots);
        ledger.last_completed_edges = Some(edges);
        self.phase_ledger.set(ledger);
    }
    pub(super) fn tick(&self) -> Result<(), Refusal> {
        (self.checkpoint)().map_err(Refusal::Control)?;
        self.work
            .borrow_mut()
            .charge(Observation::exact(1))
            .map_err(|_| Refusal::Capacity("reference work"))
    }
    pub(super) fn reserve<T>(&self, count: usize) -> Result<(), Refusal> {
        let mut bytes = Observation::product(count, std::mem::size_of::<T>());
        bytes.observe(Observation::product(
            count,
            4 * std::mem::size_of::<usize>(),
        ));
        self.reserve_observation(bytes)
    }
    pub(super) fn reserve_bytes(&self, bytes: usize) -> Result<(), Refusal> {
        self.reserve_observation(Observation::from_usize(bytes))
    }
    fn reserve_observation(&self, bytes: Observation) -> Result<(), Refusal> {
        self.tick()?;
        self.bytes
            .borrow_mut()
            .charge(bytes)
            .map_err(|_| Refusal::Capacity("reference storage"))
    }
    pub(super) fn bytes(&self) -> Observation {
        self.bytes.borrow().observation()
    }

    /// Count borrowed canonical serialization without creating a byte buffer.
    /// This is an allocation model, never an extent used to allocate or encode.
    /// Any selected finite allowance is debited before the caller clones data.
    pub(super) fn canonical_reserve<T: serde::Serialize>(&self, value: &T) -> Result<(), Refusal> {
        struct Counter<'a, 'b> {
            budget: &'a Budget<'b>,
            bytes: Observation,
            refusal: Option<Refusal>,
        }
        impl std::io::Write for Counter<'_, '_> {
            fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
                if let Err(error) = self.budget.tick() {
                    self.refusal = Some(error);
                    return Err(std::io::Error::other(
                        "reference budget refused serialization",
                    ));
                }
                self.bytes.observe(Observation::from_usize(value.len()));
                Ok(value.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut counter = Counter {
            budget: self,
            bytes: Observation::default(),
            refusal: None,
        };
        if serde_json::to_writer(&mut counter, value).is_err() {
            return Err(counter.refusal.unwrap_or(Refusal::Inventory));
        }
        let mut bytes = counter.bytes;
        bytes.observe(counter.bytes);
        bytes.observe(Observation::from_usize(std::mem::size_of::<T>()));
        self.reserve_observation(bytes)
    }
    pub(super) fn canonical_clone<T: serde::Serialize + Clone>(
        &self,
        value: &T,
    ) -> Result<T, Refusal> {
        self.canonical_reserve(value)?;
        Ok(value.clone())
    }
    fn path(&self, path: &WitnessPath) -> Result<(), Refusal> {
        self.reserve::<usize>(path.prerequisites.len())
    }
    fn slot(&self, slot: &Slot) -> Result<(), Refusal> {
        self.tick()?;
        match slot {
            Slot::Ordinary { .. } => Ok(()),
            Slot::WitnessArgument { path, .. } | Slot::UnknownMember { path, .. } => {
                self.path(path)
            }
        }
    }
    fn identity(&self, identity: &UnknownIdentity) -> Result<(), Refusal> {
        self.tick()?;
        match identity {
            #[cfg(test)]
            UnknownIdentity::Input { .. } => Ok(()),
            UnknownIdentity::Symbolic { path, .. } => self.path(path),
        }
    }
    pub(super) fn term(&self, term: &Term) -> Result<(), Refusal> {
        self.tick()?;
        match term {
            Term::Constructor(_, children) => {
                self.reserve::<Term>(children.len())?;
                for child in children {
                    self.term(child)?;
                }
                Ok(())
            }
            Term::SelectedArgument { path, .. } => self.path(path),
            Term::Variable(slot) => self.slot(slot),
            Term::Anchor { identity, .. } => self.identity(identity),
            _ => Ok(()),
        }
    }
    fn witness(&self, witness: &Witness) -> Result<(), Refusal> {
        self.tick()?;
        match witness {
            Witness::Unknown { identity, .. } => self.identity(identity),
            Witness::Selected {
                arguments,
                prerequisites,
                ..
            } => {
                self.reserve::<Term>(arguments.len())?;
                self.reserve::<Witness>(prerequisites.len())?;
                for term in arguments {
                    self.term(term)?;
                }
                for child in prerequisites {
                    self.witness(child)?;
                }
                Ok(())
            }
        }
    }
    fn state(&self, state: &State) -> Result<(), Refusal> {
        self.reserve::<Term>(state.arguments.len())?;
        self.reserve::<Witness>(state.implementations.len())?;
        for term in &state.arguments {
            self.term(term)?;
        }
        for witness in &state.implementations {
            self.witness(witness)?;
        }
        Ok(())
    }
    fn clone_term(&self, term: &Term) -> Result<Term, Refusal> {
        self.term(term)?;
        Ok(term.clone())
    }
    fn clone_state(&self, state: &State) -> Result<State, Refusal> {
        self.state(state)?;
        Ok(state.clone())
    }
    fn clone_slot(&self, slot: &Slot) -> Result<Slot, Refusal> {
        self.slot(slot)?;
        Ok(slot.clone())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Definition {
    pub package: usize,
    pub declaration: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Member {
    pub contract: Definition,
    pub id: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct WitnessPath {
    pub root: usize,
    pub prerequisites: Vec<usize>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Slot {
    Ordinary {
        context: usize,
        ordinal: usize,
    },
    WitnessArgument {
        context: usize,
        path: WitnessPath,
        ordinal: usize,
    },
    UnknownMember {
        context: usize,
        path: WitnessPath,
        member: Member,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum UnknownIdentity {
    #[cfg(test)]
    Input {
        scope: Definition,
        parameter: usize,
        instance: usize,
    },
    Symbolic {
        context: usize,
        path: WitnessPath,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Term {
    Parameter {
        scope: Definition,
        ordinal: usize,
    },
    Ground(usize),
    Constructor(usize, Vec<Term>),
    Projection {
        scope: Definition,
        parameter: usize,
        member: Member,
    },
    // Derived selected-method entry transfer; not a new authored operand.
    SelectedArgument {
        scope: Definition,
        path: WitnessPath,
        ordinal: usize,
    },
    Variable(Slot),
    Anchor {
        identity: UnknownIdentity,
        member: Member,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Witness {
    Unknown {
        identity: UnknownIdentity,
        contract: Definition,
    },
    Selected {
        scheme: Definition,
        arguments: Vec<Term>,
        prerequisites: Vec<Witness>,
    },
}

#[derive(Clone, Debug)]
pub(super) enum Operand {
    Parameter {
        scope: Definition,
        path: WitnessPath,
    },
    Concrete {
        scheme: Definition,
        arguments: Vec<Term>,
        prerequisites: Vec<Operand>,
    },
}

#[derive(Clone, Debug)]
pub(super) struct Scheme {
    pub contract: Definition,
    pub parameters: usize,
    pub prerequisite_contracts: Vec<Definition>,
    pub bindings: BTreeMap<usize, Term>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CallKind {
    Function,
    FunctionValue,
    UnusedMapping,
    SelectedMethodEntry,
}

#[derive(Clone, Debug)]
pub(super) struct Call {
    pub target: Definition,
    pub arguments: Vec<Term>,
    pub implementations: Vec<Operand>,
    pub kind: CallKind,
    pub method_guard: Option<(usize, Definition)>,
    pub untaken: bool,
    pub phantom: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Function {
    pub parameters: usize,
    pub witnesses: Vec<(usize, Definition)>,
    pub calls: Vec<Call>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct State {
    pub function: Definition,
    pub arguments: Vec<Term>,
    pub implementations: Vec<Witness>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Program {
    pub contracts: BTreeMap<Definition, BTreeSet<usize>>,
    pub schemes: BTreeMap<Definition, Scheme>,
    pub functions: BTreeMap<Definition, Function>,
    pub entries: Vec<State>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Refusal {
    Control(Diagnostic),
    PositiveRecurrence,
    MissingDefinition,
    Inventory,
    Arity,
    Scope,
    ForeignContract,
    ForeignMember,
    ProjectionInBinding,
    UnauthorizedDerivedPath,
    AuthoredSymbolicValue,
    UnsupportedRecursiveWitnessConstruction,
    Capacity(&'static str),
}

#[derive(Clone, Copy)]
#[cfg_attr(not(test), derive(Default))]
pub(super) struct Limits {
    pub contexts: Option<u64>,
    pub slots: Option<u64>,
    // Test-only finite exploration is not the symbolic admission contract.
    #[cfg(test)]
    pub concrete_states: usize,
}
// The finite concrete-state fixture allowance differs from the product defaults.
#[cfg(test)]
impl Default for Limits {
    fn default() -> Self {
        Self {
            contexts: None,
            slots: None,
            #[cfg(test)]
            concrete_states: 256,
        }
    }
}

pub(super) struct Environment<'a> {
    pub scope: Definition,
    pub declaration: &'a Function,
    pub arguments: &'a [Term],
    pub implementations: &'a [Witness],
}

struct BoundedProgram<'a, 'b> {
    program: &'a Program,
    budget: &'a Budget<'b>,
}
impl Deref for BoundedProgram<'_, '_> {
    type Target = Program;
    fn deref(&self) -> &Program {
        self.program
    }
}

impl BoundedProgram<'_, '_> {
    fn function(&self, id: Definition) -> Result<&Function, Refusal> {
        self.budget.tick()?;
        self.functions.get(&id).ok_or(Refusal::MissingDefinition)
    }
    fn scheme(&self, id: Definition) -> Result<&Scheme, Refusal> {
        self.budget.tick()?;
        self.schemes.get(&id).ok_or(Refusal::MissingDefinition)
    }
    fn member_exists(&self, member: Member) -> Result<(), Refusal> {
        self.budget.tick()?;
        match self.contracts.get(&member.contract) {
            Some(members) if members.contains(&member.id) => Ok(()),
            Some(_) => Err(Refusal::ForeignMember),
            None => Err(Refusal::ForeignContract),
        }
    }
    fn witness_contract(&self, witness: &Witness) -> Result<Definition, Refusal> {
        match witness {
            Witness::Unknown { contract, .. } => Ok(*contract),
            Witness::Selected { scheme, .. } => Ok(self.scheme(*scheme)?.contract),
        }
    }
    fn operand_contract(&self, operand: &Operand, f: &Function) -> Result<Definition, Refusal> {
        match operand {
            Operand::Parameter { path, .. } => {
                // The guarded method-entry primitive checks its immediate
                // prerequisite paths separately; authored paths stay at a root.
                if !path.prerequisites.is_empty() {
                    return Err(Refusal::UnauthorizedDerivedPath);
                }
                f.witnesses
                    .iter()
                    .find(|(id, _)| *id == path.root)
                    .map(|(_, contract)| *contract)
                    .ok_or(Refusal::Scope)
            }
            Operand::Concrete { scheme, .. } => Ok(self.scheme(*scheme)?.contract),
        }
    }
    fn admit_term(
        &self,
        term: &Term,
        scope: Definition,
        parameters: usize,
        witnesses: &[(usize, Definition)],
        binding: bool,
        derived: bool,
    ) -> Result<(), Refusal> {
        self.budget.tick()?;
        match term {
            Term::Parameter {
                scope: actual,
                ordinal,
            } => {
                if *actual != scope || *ordinal >= parameters {
                    return Err(Refusal::Scope);
                }
            }
            Term::Ground(_) => {}
            Term::Constructor(_, children) => {
                for child in children {
                    self.admit_term(child, scope, parameters, witnesses, binding, derived)?;
                }
            }
            Term::Projection {
                scope: actual,
                parameter,
                member,
            } => {
                if binding {
                    return Err(Refusal::ProjectionInBinding);
                }
                if *actual != scope {
                    return Err(Refusal::Scope);
                }
                let contract = witnesses
                    .iter()
                    .find(|(id, _)| id == parameter)
                    .ok_or(Refusal::Scope)?
                    .1;
                if contract != member.contract {
                    return Err(Refusal::ForeignContract);
                }
                self.member_exists(*member)?;
            }
            Term::SelectedArgument {
                scope: actual,
                path,
                ..
            } => {
                if binding || !derived {
                    return Err(Refusal::UnauthorizedDerivedPath);
                }
                if *actual != scope || !witnesses.iter().any(|(id, _)| *id == path.root) {
                    return Err(Refusal::Scope);
                }
            }
            Term::Variable(_) | Term::Anchor { .. } => return Err(Refusal::AuthoredSymbolicValue),
        }
        Ok(())
    }
    fn admit_operand(
        &self,
        operand: &Operand,
        scope: Definition,
        f: &Function,
        derived: bool,
    ) -> Result<(), Refusal> {
        self.budget.tick()?;
        match operand {
            Operand::Parameter {
                scope: actual,
                path,
            } => {
                if *actual != scope || !f.witnesses.iter().any(|(id, _)| *id == path.root) {
                    return Err(Refusal::Scope);
                }
                if !derived && !path.prerequisites.is_empty() {
                    return Err(Refusal::UnauthorizedDerivedPath);
                }
            }
            Operand::Concrete {
                scheme,
                arguments,
                prerequisites,
            } => {
                let s = self.scheme(*scheme)?;
                if arguments.len() != s.parameters
                    || prerequisites.len() != s.prerequisite_contracts.len()
                {
                    return Err(Refusal::Arity);
                }
                for argument in arguments {
                    self.admit_term(argument, scope, f.parameters, &f.witnesses, false, derived)?;
                }
                for (prerequisite, expected) in prerequisites.iter().zip(&s.prerequisite_contracts)
                {
                    self.admit_operand(prerequisite, scope, f, derived)?;
                    if self.operand_contract(prerequisite, f)? != *expected {
                        return Err(Refusal::ForeignContract);
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn admit(&self) -> Result<(), Refusal> {
        self.budget.tick()?;
        self.budget.reserve::<Definition>(
            self.contracts
                .len()
                .checked_add(self.schemes.len())
                .and_then(|n| n.checked_add(self.functions.len()))
                .ok_or(Refusal::Capacity("definition count"))?,
        )?;
        let mut definitions = BTreeSet::new();
        for id in self
            .contracts
            .keys()
            .chain(self.schemes.keys())
            .chain(self.functions.keys())
        {
            self.budget.tick()?;
            if !definitions.insert(*id) {
                return Err(Refusal::Inventory);
            }
        }
        for (id, scheme) in &self.schemes {
            self.budget.tick()?;
            let members = self
                .contracts
                .get(&scheme.contract)
                .ok_or(Refusal::ForeignContract)?;
            if !scheme.bindings.keys().copied().eq(members.iter().copied()) {
                return Err(Refusal::Inventory);
            }
            for contract in &scheme.prerequisite_contracts {
                self.budget.tick()?;
                if !self.contracts.contains_key(contract) {
                    return Err(Refusal::ForeignContract);
                }
            }
            for term in scheme.bindings.values() {
                self.admit_term(term, *id, scheme.parameters, &[], true, false)?;
            }
        }
        for (id, f) in &self.functions {
            self.budget.reserve::<usize>(f.witnesses.len())?;
            let mut witness_ids = BTreeSet::new();
            for (id, contract) in &f.witnesses {
                self.budget.tick()?;
                if !witness_ids.insert(*id) {
                    return Err(Refusal::Inventory);
                }
                if !self.contracts.contains_key(contract) {
                    return Err(Refusal::ForeignContract);
                }
            }
            for call in &f.calls {
                let _ = (call.untaken, call.phantom);
                let target = self.function(call.target)?;
                if call.arguments.len() != target.parameters
                    || call.implementations.len() != target.witnesses.len()
                {
                    return Err(Refusal::Arity);
                }
                let derived = call.kind == CallKind::SelectedMethodEntry;
                if derived != call.method_guard.is_some() {
                    return Err(Refusal::UnauthorizedDerivedPath);
                }
                if let Some((parameter, scheme)) = call.method_guard {
                    let contract = f
                        .witnesses
                        .iter()
                        .find(|(id, _)| *id == parameter)
                        .ok_or(Refusal::Scope)?
                        .1;
                    let selected = self.scheme(scheme)?;
                    if selected.contract != contract {
                        return Err(Refusal::ForeignContract);
                    }
                    // This primitive stands for the exact selected-method entry:
                    // all scheme arguments and immediate prerequisites transfer
                    // in declared order. It is not arbitrary witness reflection.
                    if target.parameters != selected.parameters
                        || target.witnesses.len() != selected.prerequisite_contracts.len()
                    {
                        return Err(Refusal::Arity);
                    }
                    for (ordinal, term) in call.arguments.iter().enumerate() {
                        if !matches!(term, Term::SelectedArgument { scope, path, ordinal: actual }
                            if *scope == *id && path.root == parameter
                                && path.prerequisites.is_empty() && *actual == ordinal)
                        {
                            return Err(Refusal::UnauthorizedDerivedPath);
                        }
                    }
                    for (ordinal, (operand, contract)) in call
                        .implementations
                        .iter()
                        .zip(&selected.prerequisite_contracts)
                        .enumerate()
                    {
                        if target.witnesses[ordinal].1 != *contract {
                            return Err(Refusal::ForeignContract);
                        }
                        if !matches!(operand, Operand::Parameter { scope, path }
                            if *scope == *id && path.root == parameter
                                && path.prerequisites == [ordinal])
                        {
                            return Err(Refusal::UnauthorizedDerivedPath);
                        }
                    }
                }
                for term in &call.arguments {
                    self.admit_term(term, *id, f.parameters, &f.witnesses, false, derived)?;
                }
                for (operand, (_, expected)) in call.implementations.iter().zip(&target.witnesses) {
                    self.admit_operand(operand, *id, f, derived)?;
                    if !derived && self.operand_contract(operand, f)? != *expected {
                        return Err(Refusal::ForeignContract);
                    }
                }
            }
        }
        fn closed(term: &Term, budget: &Budget<'_>) -> Result<(), Refusal> {
            budget.tick()?;
            match term {
                Term::Ground(_) => Ok(()),
                Term::Constructor(_, children) => {
                    for child in children {
                        closed(child, budget)?;
                    }
                    Ok(())
                }
                _ => Err(Refusal::AuthoredSymbolicValue),
            }
        }
        fn witness(program: &BoundedProgram<'_, '_>, input: &Witness) -> Result<(), Refusal> {
            program.budget.tick()?;
            match input {
                #[cfg(test)]
                Witness::Unknown {
                    identity: UnknownIdentity::Input { .. },
                    contract,
                } => {
                    if program.contracts.contains_key(contract) {
                        Ok(())
                    } else {
                        Err(Refusal::ForeignContract)
                    }
                }
                Witness::Selected {
                    scheme,
                    arguments,
                    prerequisites,
                } => {
                    let s = program.scheme(*scheme)?;
                    if arguments.len() != s.parameters
                        || prerequisites.len() != s.prerequisite_contracts.len()
                    {
                        return Err(Refusal::Arity);
                    }
                    for term in arguments {
                        closed(term, program.budget)?;
                    }
                    for (input, contract) in prerequisites.iter().zip(&s.prerequisite_contracts) {
                        witness(program, input)?;
                        if program.witness_contract(input)? != *contract {
                            return Err(Refusal::ForeignContract);
                        }
                    }
                    Ok(())
                }
                _ => Err(Refusal::AuthoredSymbolicValue),
            }
        }
        for entry in &self.entries {
            let f = self.function(entry.function)?;
            if entry.arguments.len() != f.parameters
                || entry.implementations.len() != f.witnesses.len()
            {
                return Err(Refusal::Arity);
            }
            for term in &entry.arguments {
                closed(term, self.budget)?;
            }
            for (input, (_, contract)) in entry.implementations.iter().zip(&f.witnesses) {
                witness(self, input)?;
                if self.witness_contract(input)? != *contract {
                    return Err(Refusal::ForeignContract);
                }
            }
        }
        self.admit_prerequisite_construction()
    }
    fn admit_prerequisite_construction(&self) -> Result<(), Refusal> {
        fn nested(operand: &Operand, below: bool, budget: &Budget<'_>) -> Result<bool, Refusal> {
            budget.tick()?;
            match operand {
                Operand::Parameter { .. } => Ok(below),
                Operand::Concrete { prerequisites, .. } => {
                    for p in prerequisites {
                        if nested(p, true, budget)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
            }
        }
        // Only witness-construction transfers require a recursion check. Search
        // the raw call relation independently, without producer SCC metadata.
        for (origin, function) in &self.functions {
            for call in &function.calls {
                for operand in &call.implementations {
                    if !nested(operand, false, self.budget)? {
                        continue;
                    }
                    self.budget.reserve::<Definition>(1)?;
                    let mut pending = vec![call.target];
                    let mut seen = BTreeSet::new();
                    while let Some(target) = pending.pop() {
                        self.budget.tick()?;
                        if target == *origin {
                            return Err(Refusal::UnsupportedRecursiveWitnessConstruction);
                        }
                        if seen.contains(&target) {
                            continue;
                        }
                        self.budget.reserve::<Definition>(1)?;
                        seen.insert(target);
                        let next = self.function(target)?;
                        self.budget.reserve::<Definition>(next.calls.len())?;
                        pending.extend(next.calls.iter().map(|call| call.target));
                    }
                }
            }
        }
        Ok(())
    }
    fn select<'a>(
        &self,
        env: &'a Environment<'_>,
        path: &WitnessPath,
    ) -> Result<&'a Witness, Refusal> {
        let ordinal = env
            .declaration
            .witnesses
            .iter()
            .position(|(id, _)| *id == path.root)
            .ok_or(Refusal::Scope)?;
        let mut witness = env.implementations.get(ordinal).ok_or(Refusal::Arity)?;
        for child in &path.prerequisites {
            let Witness::Selected { prerequisites, .. } = witness else {
                return Err(Refusal::UnauthorizedDerivedPath);
            };
            witness = prerequisites.get(*child).ok_or(Refusal::Arity)?;
        }
        Ok(witness)
    }
    fn apply_binding(
        &self,
        term: &Term,
        scope: Definition,
        arguments: &[Term],
    ) -> Result<Term, Refusal> {
        self.budget.tick()?;
        // Even a constant member does not excuse an incomplete applied scheme.
        if self.scheme(scope)?.parameters != arguments.len() {
            return Err(Refusal::Arity);
        }
        match term {
            Term::Parameter {
                scope: actual,
                ordinal,
            } => {
                if *actual != scope {
                    return Err(Refusal::Scope);
                }
                self.budget
                    .clone_term(arguments.get(*ordinal).ok_or(Refusal::Arity)?)
            }
            Term::Ground(_) => self.budget.clone_term(term),
            Term::Constructor(tag, children) => {
                self.budget.reserve::<Term>(children.len())?;
                Ok(Term::Constructor(
                    *tag,
                    children
                        .iter()
                        .map(|child| self.apply_binding(child, scope, arguments))
                        .collect::<Result<_, _>>()?,
                ))
            }
            _ => Err(Refusal::ProjectionInBinding),
        }
    }
    pub(super) fn project(&self, witness: &Witness, member: Member) -> Result<Term, Refusal> {
        self.member_exists(member)?;
        if self.witness_contract(witness)? != member.contract {
            return Err(Refusal::ForeignContract);
        }
        match witness {
            Witness::Unknown { identity, .. } => {
                self.budget.identity(identity)?;
                Ok(Term::Anchor {
                    identity: identity.clone(),
                    member,
                })
            }
            Witness::Selected {
                scheme, arguments, ..
            } => {
                let binding = self
                    .scheme(*scheme)?
                    .bindings
                    .get(&member.id)
                    .ok_or(Refusal::ForeignMember)?;
                self.apply_binding(binding, *scheme, arguments)
            }
        }
    }
    fn evaluate_type(&self, term: &Term, env: &Environment<'_>) -> Result<Term, Refusal> {
        self.budget.tick()?;
        match term {
            Term::Parameter { scope, ordinal } => {
                if *scope != env.scope {
                    return Err(Refusal::Scope);
                }
                self.budget
                    .clone_term(env.arguments.get(*ordinal).ok_or(Refusal::Arity)?)
            }
            Term::Ground(_) => self.budget.clone_term(term),
            Term::Constructor(tag, children) => {
                self.budget.reserve::<Term>(children.len())?;
                Ok(Term::Constructor(
                    *tag,
                    children
                        .iter()
                        .map(|child| self.evaluate_type(child, env))
                        .collect::<Result<_, _>>()?,
                ))
            }
            Term::Projection {
                scope,
                parameter,
                member,
            } => {
                if *scope != env.scope {
                    return Err(Refusal::Scope);
                }
                self.project(
                    self.select(
                        env,
                        &WitnessPath {
                            root: *parameter,
                            prerequisites: vec![],
                        },
                    )?,
                    *member,
                )
            }
            Term::SelectedArgument {
                scope,
                path,
                ordinal,
            } => {
                if *scope != env.scope {
                    return Err(Refusal::Scope);
                }
                let Witness::Selected { arguments, .. } = self.select(env, path)? else {
                    return Err(Refusal::UnauthorizedDerivedPath);
                };
                self.budget
                    .clone_term(arguments.get(*ordinal).ok_or(Refusal::Arity)?)
            }
            Term::Variable(_) | Term::Anchor { .. } => Err(Refusal::AuthoredSymbolicValue),
        }
    }
    fn evaluate_operand(
        &self,
        operand: &Operand,
        env: &Environment<'_>,
    ) -> Result<Witness, Refusal> {
        self.budget.tick()?;
        match operand {
            Operand::Parameter { scope, path } => {
                if *scope != env.scope {
                    return Err(Refusal::Scope);
                }
                let witness = self.select(env, path)?;
                self.budget.witness(witness)?;
                Ok(witness.clone())
            }
            Operand::Concrete {
                scheme,
                arguments,
                prerequisites,
            } => {
                let s = self.scheme(*scheme)?;
                self.budget.reserve::<Term>(arguments.len())?;
                self.budget.reserve::<Witness>(prerequisites.len())?;
                let arguments = arguments
                    .iter()
                    .map(|t| self.evaluate_type(t, env))
                    .collect::<Result<Vec<_>, _>>()?;
                let prerequisites = prerequisites
                    .iter()
                    .map(|p| self.evaluate_operand(p, env))
                    .collect::<Result<Vec<_>, _>>()?;
                if arguments.len() != s.parameters
                    || prerequisites.len() != s.prerequisite_contracts.len()
                {
                    return Err(Refusal::Arity);
                }
                for (w, contract) in prerequisites.iter().zip(&s.prerequisite_contracts) {
                    if self.witness_contract(w)? != *contract {
                        return Err(Refusal::ForeignContract);
                    }
                }
                Ok(Witness::Selected {
                    scheme: *scheme,
                    arguments,
                    prerequisites,
                })
            }
        }
    }
    fn successors(&self, state: &State) -> Result<Vec<State>, Refusal> {
        let f = self.function(state.function)?;
        if state.arguments.len() != f.parameters || state.implementations.len() != f.witnesses.len()
        {
            return Err(Refusal::Arity);
        }
        for (w, (_, contract)) in state.implementations.iter().zip(&f.witnesses) {
            if self.witness_contract(w)? != *contract {
                return Err(Refusal::ForeignContract);
            }
        }
        let env = Environment {
            scope: state.function,
            declaration: f,
            arguments: &state.arguments,
            implementations: &state.implementations,
        };
        self.budget.reserve::<State>(f.calls.len())?;
        let mut result = Vec::new();
        // Untaken/phantom/unused markers do not remove preparation dependencies.
        for call in &f.calls {
            if let Some((parameter, scheme)) = call.method_guard
                && !matches!(self.select(&env, &WitnessPath { root: parameter, prerequisites: vec![] })?,
                    Witness::Selected { scheme: actual, .. } if *actual == scheme)
            {
                continue;
            }
            self.budget.reserve::<Term>(call.arguments.len())?;
            self.budget.reserve::<Witness>(call.implementations.len())?;
            result.push(State {
                function: call.target,
                arguments: call
                    .arguments
                    .iter()
                    .map(|t| self.evaluate_type(t, &env))
                    .collect::<Result<_, _>>()?,
                implementations: call
                    .implementations
                    .iter()
                    .map(|p| self.evaluate_operand(p, &env))
                    .collect::<Result<_, _>>()?,
            });
        }
        Ok(result)
    }
    #[cfg(test)]
    fn concrete_closure(&self, limits: Limits) -> Result<BTreeSet<State>, Refusal> {
        self.admit()?;
        let mut seen = BTreeSet::new();
        self.budget.reserve::<State>(self.entries.len())?;
        for entry in &self.entries {
            self.budget.state(entry)?;
        }
        let mut pending = self.entries.clone();
        while let Some(state) = pending.pop() {
            self.budget.tick()?;
            if seen.contains(&state) {
                continue;
            }
            if seen.len() >= limits.concrete_states {
                return Err(Refusal::Capacity("concrete states"));
            }
            let next = self.successors(&state)?;
            self.budget.reserve::<State>(1)?;
            self.budget.reserve::<State>(next.len())?;
            seen.insert(state);
            pending.extend(next);
        }
        Ok(seen)
    }
    #[cfg(test)]
    fn rebind(
        &self,
        term: &Term,
        callee_scope: Definition,
        caller_scope: Definition,
        actual_types: &[Term],
        actual_witnesses: &[Operand],
    ) -> Result<Term, Refusal> {
        self.budget.tick()?;
        let callee = self.function(callee_scope)?;
        let caller = self.function(caller_scope)?;
        if actual_types.len() != callee.parameters
            || actual_witnesses.len() != callee.witnesses.len()
        {
            return Err(Refusal::Arity);
        }
        for actual in actual_types {
            self.admit_term(
                actual,
                caller_scope,
                caller.parameters,
                &caller.witnesses,
                false,
                false,
            )?;
        }
        for (actual, (_, expected)) in actual_witnesses.iter().zip(&callee.witnesses) {
            self.admit_operand(actual, caller_scope, caller, false)?;
            let contract = match actual {
                Operand::Parameter { path, .. } => {
                    caller
                        .witnesses
                        .iter()
                        .find(|(id, _)| *id == path.root)
                        .ok_or(Refusal::Scope)?
                        .1
                }
                Operand::Concrete { scheme, .. } => self.scheme(*scheme)?.contract,
            };
            if contract != *expected {
                return Err(Refusal::ForeignContract);
            }
        }
        match term {
            Term::Parameter { scope, ordinal } => {
                if *scope != callee_scope {
                    return Err(Refusal::Scope);
                }
                self.budget
                    .clone_term(actual_types.get(*ordinal).ok_or(Refusal::Arity)?)
            }
            Term::Ground(_) => self.budget.clone_term(term),
            Term::Constructor(tag, children) => {
                self.budget.reserve::<Term>(children.len())?;
                Ok(Term::Constructor(
                    *tag,
                    children
                        .iter()
                        .map(|t| {
                            self.rebind(
                                t,
                                callee_scope,
                                caller_scope,
                                actual_types,
                                actual_witnesses,
                            )
                        })
                        .collect::<Result<_, _>>()?,
                ))
            }
            Term::Projection {
                scope,
                parameter,
                member,
            } => {
                if *scope != callee_scope {
                    return Err(Refusal::Scope);
                }
                self.member_exists(*member)?;
                let ordinal = callee
                    .witnesses
                    .iter()
                    .position(|(id, _)| id == parameter)
                    .ok_or(Refusal::Scope)?;
                if callee.witnesses[ordinal].1 != member.contract {
                    return Err(Refusal::ForeignContract);
                }
                match actual_witnesses.get(ordinal).ok_or(Refusal::Arity)? {
                    Operand::Parameter { scope, path } => {
                        if *scope != caller_scope {
                            return Err(Refusal::Scope);
                        }
                        if !path.prerequisites.is_empty() {
                            return Err(Refusal::UnauthorizedDerivedPath);
                        }
                        let contract = caller
                            .witnesses
                            .iter()
                            .find(|(id, _)| *id == path.root)
                            .ok_or(Refusal::Scope)?
                            .1;
                        if contract != member.contract {
                            return Err(Refusal::ForeignContract);
                        }
                        Ok(Term::Projection {
                            scope: caller_scope,
                            parameter: path.root,
                            member: *member,
                        })
                    }
                    Operand::Concrete {
                        scheme, arguments, ..
                    } => {
                        let selected = self.scheme(*scheme)?;
                        if selected.contract != member.contract {
                            return Err(Refusal::ForeignContract);
                        }
                        self.apply_binding(
                            selected
                                .bindings
                                .get(&member.id)
                                .ok_or(Refusal::ForeignMember)?,
                            *scheme,
                            arguments,
                        )
                    }
                }
            }
            _ => Err(Refusal::AuthoredSymbolicValue),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Selection {
    Unknown(Definition),
    Scheme(Definition, Vec<Selection>),
}
fn selection(witness: &Witness, budget: &Budget<'_>) -> Result<Selection, Refusal> {
    budget.tick()?;
    Ok(match witness {
        Witness::Unknown { contract, .. } => Selection::Unknown(*contract),
        Witness::Selected {
            scheme,
            prerequisites,
            ..
        } => {
            budget.reserve::<Selection>(prerequisites.len())?;
            Selection::Scheme(
                *scheme,
                prerequisites
                    .iter()
                    .map(|p| selection(p, budget))
                    .collect::<Result<_, _>>()?,
            )
        }
    })
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ContextKey {
    function: Definition,
    witnesses: Vec<Selection>,
}

#[derive(Debug)]
pub(super) struct Proof {
    pub finite: bool,
    pub contexts: usize,
    pub slots: Vec<Slot>,
    pub edges: BTreeSet<(Slot, Slot, bool)>,
    // Positive weighted diagonal is a finite algebraic cycle certificate.
    pub positive_diagonal: Option<Slot>,
}

struct SubstitutionRelation<'a, 'b> {
    program: BoundedProgram<'a, 'b>,
    limits: Limits,
    contexts: BTreeMap<ContextKey, State>,
    pending: Vec<State>,
    slots: BTreeSet<Slot>,
    edges: BTreeSet<(Slot, Slot, bool)>,
}
impl SubstitutionRelation<'_, '_> {
    fn slot(&mut self, slot: Slot) -> Result<(), Refusal> {
        self.program.budget.tick()?;
        if !self.slots.contains(&slot) {
            if self.limits.slots.is_some_and(|limit| {
                let count = Observation::from_usize(self.slots.len());
                count.saturated || count.consumed >= limit
            }) {
                return Err(Refusal::Capacity("reference slots"));
            }
            self.program.budget.reserve::<Slot>(1)?;
            self.slots.insert(slot);
        }
        Ok(())
    }
    fn symbolic_witness(
        &mut self,
        context: usize,
        path: WitnessPath,
        selected: &Selection,
    ) -> Result<Witness, Refusal> {
        self.program.budget.tick()?;
        match selected {
            Selection::Unknown(contract) => {
                let members = self
                    .program
                    .contracts
                    .get(contract)
                    .ok_or(Refusal::ForeignContract)?;
                self.program.budget.reserve::<usize>(members.len())?;
                let members = members.clone();
                for id in members {
                    self.slot(Slot::UnknownMember {
                        context,
                        path: {
                            self.program.budget.path(&path)?;
                            path.clone()
                        },
                        member: Member {
                            contract: *contract,
                            id,
                        },
                    })?;
                }
                Ok(Witness::Unknown {
                    identity: UnknownIdentity::Symbolic { context, path },
                    contract: *contract,
                })
            }
            Selection::Scheme(id, children) => {
                let parameters = self.program.scheme(*id)?.parameters;
                self.program.budget.reserve::<Term>(parameters)?;
                let mut arguments = Vec::new();
                for ordinal in 0..parameters {
                    let slot = Slot::WitnessArgument {
                        context,
                        path: {
                            self.program.budget.path(&path)?;
                            path.clone()
                        },
                        ordinal,
                    };
                    self.slot(self.program.budget.clone_slot(&slot)?)?;
                    arguments.push(Term::Variable(slot));
                }
                self.program.budget.reserve::<Witness>(children.len())?;
                let mut prerequisites = Vec::new();
                for (ordinal, child) in children.iter().enumerate() {
                    self.program.budget.path(&path)?;
                    self.program.budget.reserve::<usize>(1)?;
                    let mut child_path = path.clone();
                    child_path.prerequisites.push(ordinal);
                    prerequisites.push(self.symbolic_witness(context, child_path, child)?);
                }
                Ok(Witness::Selected {
                    scheme: *id,
                    arguments,
                    prerequisites,
                })
            }
        }
    }
    fn intern(&mut self, key: ContextKey) -> Result<State, Refusal> {
        self.program.budget.tick()?;
        if let Some(state) = self.contexts.get(&key) {
            return self.program.budget.clone_state(state);
        }
        if self.limits.contexts.is_some_and(|limit| {
            let count = Observation::from_usize(self.contexts.len());
            count.saturated || count.consumed >= limit
        }) {
            return Err(Refusal::Capacity("reference contexts"));
        }
        let context = self.contexts.len();
        let f = self.program.function(key.function)?;
        let parameters = f.parameters;
        self.program.budget.reserve::<usize>(f.witnesses.len())?;
        let roots: Vec<_> = f.witnesses.iter().map(|(id, _)| *id).collect();
        if roots.len() != key.witnesses.len() {
            return Err(Refusal::Arity);
        }
        self.program.budget.reserve::<Term>(parameters)?;
        let mut arguments = Vec::new();
        for ordinal in 0..parameters {
            let slot = Slot::Ordinary { context, ordinal };
            self.slot(self.program.budget.clone_slot(&slot)?)?;
            arguments.push(Term::Variable(slot));
        }
        self.program.budget.reserve::<Witness>(roots.len())?;
        let mut implementations = Vec::new();
        for (root, selected) in roots.into_iter().zip(&key.witnesses) {
            implementations.push(self.symbolic_witness(
                context,
                WitnessPath {
                    root,
                    prerequisites: vec![],
                },
                selected,
            )?);
        }
        let state = State {
            function: key.function,
            arguments,
            implementations,
        };
        self.program.budget.reserve::<(ContextKey, State)>(1)?;
        let retained = self.program.budget.clone_state(&state)?;
        self.contexts.insert(key, retained);
        self.program.budget.reserve::<State>(1)?;
        self.pending.push(self.program.budget.clone_state(&state)?);
        Ok(state)
    }
    fn occurrences(&mut self, term: &Term, target: &Slot, positive: bool) -> Result<(), Refusal> {
        self.program.budget.tick()?;
        let source = match term {
            Term::Variable(slot) => Some(self.program.budget.clone_slot(slot)?),
            Term::Anchor {
                identity: UnknownIdentity::Symbolic { context, path },
                member,
            } => Some(Slot::UnknownMember {
                context: *context,
                path: {
                    self.program.budget.path(path)?;
                    path.clone()
                },
                member: *member,
            }),
            Term::Ground(_) => None,
            Term::Constructor(_, children) => {
                for child in children {
                    self.occurrences(child, target, true)?;
                }
                None
            }
            _ => return Err(Refusal::AuthoredSymbolicValue),
        };
        if let Some(source) = source {
            self.slot(self.program.budget.clone_slot(&source)?)?;
            self.slot(self.program.budget.clone_slot(target)?)?;
            self.program.budget.reserve::<(Slot, Slot, bool)>(1)?;
            self.edges
                .insert((source, self.program.budget.clone_slot(target)?, positive));
        }
        Ok(())
    }
    fn witness_transfer(&mut self, actual: &Witness, target: &Witness) -> Result<(), Refusal> {
        self.program.budget.tick()?;
        match target {
            Witness::Unknown {
                identity: UnknownIdentity::Symbolic { context, path },
                contract,
            } => {
                let members = self
                    .program
                    .contracts
                    .get(contract)
                    .ok_or(Refusal::ForeignContract)?;
                self.program.budget.reserve::<usize>(members.len())?;
                let members = members.clone();
                for id in members {
                    let member = Member {
                        contract: *contract,
                        id,
                    };
                    let value = self.program.project(actual, member)?;
                    self.occurrences(
                        &value,
                        &Slot::UnknownMember {
                            context: *context,
                            path: {
                                self.program.budget.path(path)?;
                                path.clone()
                            },
                            member,
                        },
                        false,
                    )?;
                }
            }
            Witness::Selected {
                scheme,
                arguments,
                prerequisites,
            } => {
                let Witness::Selected {
                    scheme: actual_scheme,
                    arguments: actual_arguments,
                    prerequisites: actual_prerequisites,
                } = actual
                else {
                    return Err(Refusal::Arity);
                };
                if actual_scheme != scheme
                    || arguments.len() != actual_arguments.len()
                    || prerequisites.len() != actual_prerequisites.len()
                {
                    return Err(Refusal::Arity);
                }
                for (value, target) in actual_arguments.iter().zip(arguments) {
                    let Term::Variable(slot) = target else {
                        return Err(Refusal::AuthoredSymbolicValue);
                    };
                    self.occurrences(value, slot, false)?;
                }
                for (value, target) in actual_prerequisites.iter().zip(prerequisites) {
                    self.witness_transfer(value, target)?;
                }
            }
            #[cfg(test)]
            Witness::Unknown {
                identity: UnknownIdentity::Input { .. },
                ..
            } => return Err(Refusal::AuthoredSymbolicValue),
        }
        Ok(())
    }
}

pub(super) fn reference_classify_with_budget(
    program: &Program,
    limits: Limits,
    budget: &Budget<'_>,
) -> Result<Proof, Refusal> {
    let raw = program;
    let program = BoundedProgram { program, budget };
    budget.observe_phase("classifier model admission");
    program.admit()?;
    budget.observe_boundary(ReferencePhaseBoundary::ClassifierAdmission);
    budget.observe_phase("classifier context growth");
    let mut relation = SubstitutionRelation {
        program,
        limits,
        contexts: BTreeMap::new(),
        pending: Vec::new(),
        slots: BTreeSet::new(),
        edges: BTreeSet::new(),
    };
    for (id, f) in &raw.functions {
        budget.reserve::<Selection>(f.witnesses.len())?;
        relation.intern(ContextKey {
            function: *id,
            witnesses: f
                .witnesses
                .iter()
                .map(|(_, contract)| Selection::Unknown(*contract))
                .collect(),
        })?;
    }
    for entry in &raw.entries {
        relation.intern(ContextKey {
            function: entry.function,
            witnesses: {
                budget.reserve::<Selection>(entry.implementations.len())?;
                entry
                    .implementations
                    .iter()
                    .map(|w| selection(w, budget))
                    .collect::<Result<_, _>>()?
            },
        })?;
    }
    budget.observe_progress(
        relation.contexts.len(),
        relation.slots.len(),
        relation.edges.len(),
    );
    let mut cursor = 0;
    while let Some(source) = relation.pending.get(cursor) {
        let source = budget.clone_state(source)?;
        cursor += 1;
        for actual in relation.program.successors(&source)? {
            let target = relation.intern(ContextKey {
                function: actual.function,
                witnesses: {
                    budget.reserve::<Selection>(actual.implementations.len())?;
                    actual
                        .implementations
                        .iter()
                        .map(|w| selection(w, budget))
                        .collect::<Result<_, _>>()?
                },
            })?;
            for (value, target) in actual.arguments.iter().zip(&target.arguments) {
                let Term::Variable(slot) = target else {
                    return Err(Refusal::AuthoredSymbolicValue);
                };
                relation.occurrences(value, slot, false)?;
            }
            for (value, target) in actual.implementations.iter().zip(&target.implementations) {
                relation.witness_transfer(value, target)?;
            }
        }
        budget.observe_progress(
            relation.contexts.len(),
            relation.slots.len(),
            relation.edges.len(),
        );
    }
    budget.observe_boundary(ReferencePhaseBoundary::ContextGrowth);
    budget.observe_phase("weighted reachability");
    budget.reserve::<Slot>(relation.slots.len())?;
    let slots: Vec<_> = relation
        .slots
        .iter()
        .map(|s| budget.clone_slot(s))
        .collect::<Result<_, _>>()?;
    budget.reserve::<(Slot, usize)>(slots.len())?;
    for slot in &slots {
        budget.slot(slot)?;
    }
    let indexes: BTreeMap<_, _> = slots
        .iter()
        .cloned()
        .enumerate()
        .map(|(n, slot)| (slot, n))
        .collect();
    // Reachability carries a separate constructor bit. Every positive cycle
    // contains an origin with a positive outgoing edge; zero-only origins need
    // no search allocation. This remains an independent weighted-path proof,
    // not the producer's SCC algorithm or a sampled preparation prefix.
    budget.reserve::<Vec<(usize, bool)>>(slots.len())?;
    let mut successors = vec![Vec::new(); slots.len()];
    for (from, to, positive) in &relation.edges {
        budget.tick()?;
        budget.reserve::<(usize, bool)>(1)?;
        successors[indexes[from]].push((indexes[to], *positive));
    }
    let mut positive_diagonal = None;
    for origin in 0..slots.len() {
        budget.tick()?;
        let mut expanding = false;
        for (_, positive) in &successors[origin] {
            budget.tick()?;
            if *positive {
                expanding = true;
                break;
            }
        }
        if !expanding {
            continue;
        }
        budget.reserve::<[bool; 2]>(slots.len())?;
        let mut visited = vec![[false; 2]; slots.len()];
        let mut pending = Vec::new();
        budget.reserve::<(usize, bool)>(successors[origin].len())?;
        pending.extend(successors[origin].iter().copied());
        while let Some((target, positive)) = pending.pop() {
            budget.tick()?;
            if target == origin && positive {
                positive_diagonal = Some(budget.clone_slot(&slots[origin])?);
                break;
            }
            let weight = usize::from(positive);
            if visited[target][weight] {
                continue;
            }
            visited[target][weight] = true;
            budget.reserve::<(usize, bool)>(successors[target].len())?;
            pending.extend(
                successors[target]
                    .iter()
                    .map(|(child, expanding)| (*child, positive || *expanding)),
            );
        }
        if positive_diagonal.is_some() {
            break;
        }
    }
    budget.observe_boundary(ReferencePhaseBoundary::Reachability);
    Ok(Proof {
        finite: positive_diagonal.is_none(),
        positive_diagonal,
        contexts: relation.contexts.len(),
        slots,
        edges: relation.edges,
    })
}

impl Program {
    #[cfg(test)]
    pub(super) fn function(&self, id: Definition) -> Result<&Function, Refusal> {
        self.functions.get(&id).ok_or(Refusal::MissingDefinition)
    }
    #[cfg(test)]
    pub(super) fn evaluate_type(
        &self,
        term: &Term,
        environment: &Environment<'_>,
    ) -> Result<Term, Refusal> {
        let checkpoint = || Ok(());
        let budget = Budget::new(&checkpoint, None, None);
        BoundedProgram {
            program: self,
            budget: &budget,
        }
        .evaluate_type(term, environment)
    }
    pub(super) fn admit_with_budget(&self, budget: &Budget<'_>) -> Result<(), Refusal> {
        BoundedProgram {
            program: self,
            budget,
        }
        .admit()
    }
    #[cfg(test)]
    pub(super) fn admit(&self) -> Result<(), Refusal> {
        let checkpoint = || Ok(());
        let budget = Budget::new(&checkpoint, None, None);
        self.admit_with_budget(&budget)
    }
    #[cfg(test)]
    pub(super) fn project(&self, witness: &Witness, member: Member) -> Result<Term, Refusal> {
        let checkpoint = || Ok(());
        let budget = Budget::new(&checkpoint, None, None);
        BoundedProgram {
            program: self,
            budget: &budget,
        }
        .project(witness, member)
    }
    #[cfg(test)]
    pub(super) fn concrete_closure(&self, limits: Limits) -> Result<BTreeSet<State>, Refusal> {
        let checkpoint = || Ok(());
        let budget = Budget::new(&checkpoint, None, None);
        BoundedProgram {
            program: self,
            budget: &budget,
        }
        .concrete_closure(limits)
    }
    #[cfg(test)]
    pub(super) fn rebind(
        &self,
        term: &Term,
        callee_scope: Definition,
        caller_scope: Definition,
        actual_types: &[Term],
        actual_witnesses: &[Operand],
    ) -> Result<Term, Refusal> {
        let checkpoint = || Ok(());
        let budget = Budget::new(&checkpoint, None, None);
        BoundedProgram {
            program: self,
            budget: &budget,
        }
        .rebind(
            term,
            callee_scope,
            caller_scope,
            actual_types,
            actual_witnesses,
        )
    }
}
#[cfg(test)]
pub(super) fn reference_classify(program: &Program, limits: Limits) -> Result<Proof, Refusal> {
    let checkpoint = || Ok(());
    let budget = Budget::new(&checkpoint, None, None);
    reference_classify_with_budget(program, limits, &budget)
}
