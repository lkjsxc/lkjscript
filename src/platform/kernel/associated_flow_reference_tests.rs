//! Independent first-order associated-type growth oracle. Canonical-source adapters
//! can build this explicit model, but must not supply producer-normalized bindings.
//! The oracle substitutes terms/witnesses and uses weighted reachability, without
//! production type substitution, demand extraction, SCC or preparation machinery.
//! Finite concrete closure is evidence for its initial states, never a prefix proof.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Definition {
    pub package: u16,
    pub declaration: u16,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Member {
    pub contract: Definition,
    pub id: u16,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct WitnessPath {
    pub root: u16,
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
    Input {
        scope: Definition,
        parameter: u16,
        instance: u16,
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
    Ground(u8),
    Constructor(u8, Vec<Term>),
    Projection {
        scope: Definition,
        parameter: u16,
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
    pub bindings: BTreeMap<u16, Term>,
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
    pub method_guard: Option<(u16, Definition)>,
    pub untaken: bool,
    pub phantom: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Function {
    pub parameters: usize,
    pub witnesses: Vec<(u16, Definition)>,
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
    pub contracts: BTreeMap<Definition, BTreeSet<u16>>,
    pub schemes: BTreeMap<Definition, Scheme>,
    pub functions: BTreeMap<Definition, Function>,
    pub entries: Vec<State>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Refusal {
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
pub(super) struct Limits {
    pub contexts: usize,
    pub slots: usize,
    pub concrete_states: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            contexts: 128,
            slots: 256,
            concrete_states: 256,
        }
    }
}

struct Environment<'a> {
    scope: Definition,
    declaration: &'a Function,
    arguments: &'a [Term],
    implementations: &'a [Witness],
}

impl Program {
    fn function(&self, id: Definition) -> Result<&Function, Refusal> {
        self.functions.get(&id).ok_or(Refusal::MissingDefinition)
    }
    fn scheme(&self, id: Definition) -> Result<&Scheme, Refusal> {
        self.schemes.get(&id).ok_or(Refusal::MissingDefinition)
    }
    fn member_exists(&self, member: Member) -> Result<(), Refusal> {
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
        witnesses: &[(u16, Definition)],
        binding: bool,
        derived: bool,
    ) -> Result<(), Refusal> {
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
        let mut definitions = BTreeSet::new();
        for id in self
            .contracts
            .keys()
            .chain(self.schemes.keys())
            .chain(self.functions.keys())
        {
            if !definitions.insert(*id) {
                return Err(Refusal::Inventory);
            }
        }
        for (id, scheme) in &self.schemes {
            let members = self
                .contracts
                .get(&scheme.contract)
                .ok_or(Refusal::ForeignContract)?;
            if scheme.bindings.keys().copied().collect::<BTreeSet<_>>() != *members {
                return Err(Refusal::Inventory);
            }
            for contract in &scheme.prerequisite_contracts {
                if !self.contracts.contains_key(contract) {
                    return Err(Refusal::ForeignContract);
                }
            }
            for term in scheme.bindings.values() {
                self.admit_term(term, *id, scheme.parameters, &[], true, false)?;
            }
        }
        for (id, f) in &self.functions {
            if f.witnesses
                .iter()
                .map(|(id, _)| id)
                .collect::<BTreeSet<_>>()
                .len()
                != f.witnesses.len()
            {
                return Err(Refusal::Inventory);
            }
            for (_, contract) in &f.witnesses {
                if !self.contracts.contains_key(contract) {
                    return Err(Refusal::ForeignContract);
                }
            }
            for call in &f.calls {
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
        fn closed(term: &Term) -> Result<(), Refusal> {
            match term {
                Term::Ground(_) => Ok(()),
                Term::Constructor(_, children) => {
                    for child in children {
                        closed(child)?;
                    }
                    Ok(())
                }
                _ => Err(Refusal::AuthoredSymbolicValue),
            }
        }
        fn witness(program: &Program, input: &Witness) -> Result<(), Refusal> {
            match input {
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
                        closed(term)?;
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
                closed(term)?;
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
        let ids: Vec<_> = self.functions.keys().copied().collect();
        let mut reach = vec![vec![false; ids.len()]; ids.len()];
        for (i, id) in ids.iter().enumerate() {
            for call in &self.function(*id)?.calls {
                let j = ids
                    .iter()
                    .position(|id| *id == call.target)
                    .ok_or(Refusal::MissingDefinition)?;
                reach[i][j] = true;
            }
        }
        for k in 0..ids.len() {
            for i in 0..ids.len() {
                for j in 0..ids.len() {
                    let connected = reach[i][k] && reach[k][j];
                    reach[i][j] |= connected;
                }
            }
        }
        fn nested(operand: &Operand, below: bool) -> bool {
            match operand {
                Operand::Parameter { .. } => below,
                Operand::Concrete { prerequisites, .. } => {
                    prerequisites.iter().any(|p| nested(p, true))
                }
            }
        }
        for (i, id) in ids.iter().enumerate() {
            for call in &self.function(*id)?.calls {
                let j = ids
                    .iter()
                    .position(|id| *id == call.target)
                    .ok_or(Refusal::MissingDefinition)?;
                if reach[j][i] && call.implementations.iter().any(|p| nested(p, false)) {
                    return Err(Refusal::UnsupportedRecursiveWitnessConstruction);
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
                arguments.get(*ordinal).cloned().ok_or(Refusal::Arity)
            }
            Term::Ground(_) => Ok(term.clone()),
            Term::Constructor(tag, children) => Ok(Term::Constructor(
                *tag,
                children
                    .iter()
                    .map(|child| self.apply_binding(child, scope, arguments))
                    .collect::<Result<_, _>>()?,
            )),
            _ => Err(Refusal::ProjectionInBinding),
        }
    }
    pub(super) fn project(&self, witness: &Witness, member: Member) -> Result<Term, Refusal> {
        self.member_exists(member)?;
        if self.witness_contract(witness)? != member.contract {
            return Err(Refusal::ForeignContract);
        }
        match witness {
            Witness::Unknown { identity, .. } => Ok(Term::Anchor {
                identity: identity.clone(),
                member,
            }),
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
        match term {
            Term::Parameter { scope, ordinal } => {
                if *scope != env.scope {
                    return Err(Refusal::Scope);
                }
                env.arguments.get(*ordinal).cloned().ok_or(Refusal::Arity)
            }
            Term::Ground(_) => Ok(term.clone()),
            Term::Constructor(tag, children) => Ok(Term::Constructor(
                *tag,
                children
                    .iter()
                    .map(|child| self.evaluate_type(child, env))
                    .collect::<Result<_, _>>()?,
            )),
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
                arguments.get(*ordinal).cloned().ok_or(Refusal::Arity)
            }
            Term::Variable(_) | Term::Anchor { .. } => Err(Refusal::AuthoredSymbolicValue),
        }
    }
    fn evaluate_operand(
        &self,
        operand: &Operand,
        env: &Environment<'_>,
    ) -> Result<Witness, Refusal> {
        match operand {
            Operand::Parameter { scope, path } => {
                if *scope != env.scope {
                    return Err(Refusal::Scope);
                }
                Ok(self.select(env, path)?.clone())
            }
            Operand::Concrete {
                scheme,
                arguments,
                prerequisites,
            } => {
                let s = self.scheme(*scheme)?;
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
        let mut result = Vec::new();
        // Untaken/phantom/unused markers do not remove preparation dependencies.
        for call in &f.calls {
            if let Some((parameter, scheme)) = call.method_guard
                && !matches!(self.select(&env, &WitnessPath { root: parameter, prerequisites: vec![] })?,
                    Witness::Selected { scheme: actual, .. } if *actual == scheme)
            {
                continue;
            }
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
    pub(super) fn concrete_closure(&self, limits: Limits) -> Result<BTreeSet<State>, Refusal> {
        self.admit()?;
        let mut seen = BTreeSet::new();
        let mut pending = self.entries.clone();
        while let Some(state) = pending.pop() {
            if seen.contains(&state) {
                continue;
            }
            if seen.len() >= limits.concrete_states {
                return Err(Refusal::Capacity("concrete states"));
            }
            let next = self.successors(&state)?;
            seen.insert(state);
            pending.extend(next);
        }
        Ok(seen)
    }
    pub(super) fn rebind(
        &self,
        term: &Term,
        callee_scope: Definition,
        caller_scope: Definition,
        actual_types: &[Term],
        actual_witnesses: &[Operand],
    ) -> Result<Term, Refusal> {
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
                actual_types.get(*ordinal).cloned().ok_or(Refusal::Arity)
            }
            Term::Ground(_) => Ok(term.clone()),
            Term::Constructor(tag, children) => Ok(Term::Constructor(
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
            )),
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
fn selection(witness: &Witness) -> Selection {
    match witness {
        Witness::Unknown { contract, .. } => Selection::Unknown(*contract),
        Witness::Selected {
            scheme,
            prerequisites,
            ..
        } => Selection::Scheme(*scheme, prerequisites.iter().map(selection).collect()),
    }
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

struct SubstitutionRelation<'a> {
    program: &'a Program,
    limits: Limits,
    contexts: BTreeMap<ContextKey, State>,
    pending: Vec<State>,
    slots: BTreeSet<Slot>,
    edges: BTreeSet<(Slot, Slot, bool)>,
}
impl SubstitutionRelation<'_> {
    fn slot(&mut self, slot: Slot) -> Result<(), Refusal> {
        if !self.slots.contains(&slot) {
            if self.slots.len() >= self.limits.slots {
                return Err(Refusal::Capacity("reference slots"));
            }
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
        match selected {
            Selection::Unknown(contract) => {
                let members = self
                    .program
                    .contracts
                    .get(contract)
                    .ok_or(Refusal::ForeignContract)?
                    .clone();
                for id in members {
                    self.slot(Slot::UnknownMember {
                        context,
                        path: path.clone(),
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
                let mut arguments = Vec::new();
                for ordinal in 0..parameters {
                    let slot = Slot::WitnessArgument {
                        context,
                        path: path.clone(),
                        ordinal,
                    };
                    self.slot(slot.clone())?;
                    arguments.push(Term::Variable(slot));
                }
                let mut prerequisites = Vec::new();
                for (ordinal, child) in children.iter().enumerate() {
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
        if let Some(state) = self.contexts.get(&key) {
            return Ok(state.clone());
        }
        if self.contexts.len() >= self.limits.contexts {
            return Err(Refusal::Capacity("reference contexts"));
        }
        let context = self.contexts.len();
        let f = self.program.function(key.function)?;
        let parameters = f.parameters;
        let roots: Vec<_> = f.witnesses.iter().map(|(id, _)| *id).collect();
        if roots.len() != key.witnesses.len() {
            return Err(Refusal::Arity);
        }
        let mut arguments = Vec::new();
        for ordinal in 0..parameters {
            let slot = Slot::Ordinary { context, ordinal };
            self.slot(slot.clone())?;
            arguments.push(Term::Variable(slot));
        }
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
        self.contexts.insert(key, state.clone());
        self.pending.push(state.clone());
        Ok(state)
    }
    fn occurrences(&mut self, term: &Term, target: &Slot, positive: bool) -> Result<(), Refusal> {
        let source = match term {
            Term::Variable(slot) => Some(slot.clone()),
            Term::Anchor {
                identity: UnknownIdentity::Symbolic { context, path },
                member,
            } => Some(Slot::UnknownMember {
                context: *context,
                path: path.clone(),
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
            self.slot(source.clone())?;
            self.slot(target.clone())?;
            self.edges.insert((source, target.clone(), positive));
        }
        Ok(())
    }
    fn witness_transfer(&mut self, actual: &Witness, target: &Witness) -> Result<(), Refusal> {
        match target {
            Witness::Unknown {
                identity: UnknownIdentity::Symbolic { context, path },
                contract,
            } => {
                let members = self
                    .program
                    .contracts
                    .get(contract)
                    .ok_or(Refusal::ForeignContract)?
                    .clone();
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
                            path: path.clone(),
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
            _ => return Err(Refusal::AuthoredSymbolicValue),
        }
        Ok(())
    }
}

pub(super) fn reference_classify(program: &Program, limits: Limits) -> Result<Proof, Refusal> {
    program.admit()?;
    let mut relation = SubstitutionRelation {
        program,
        limits,
        contexts: BTreeMap::new(),
        pending: Vec::new(),
        slots: BTreeSet::new(),
        edges: BTreeSet::new(),
    };
    for (id, f) in &program.functions {
        relation.intern(ContextKey {
            function: *id,
            witnesses: f
                .witnesses
                .iter()
                .map(|(_, contract)| Selection::Unknown(*contract))
                .collect(),
        })?;
    }
    for entry in &program.entries {
        relation.intern(ContextKey {
            function: entry.function,
            witnesses: entry.implementations.iter().map(selection).collect(),
        })?;
    }
    let mut cursor = 0;
    while let Some(source) = relation.pending.get(cursor).cloned() {
        cursor += 1;
        for actual in program.successors(&source)? {
            let target = relation.intern(ContextKey {
                function: actual.function,
                witnesses: actual.implementations.iter().map(selection).collect(),
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
    }
    let slots: Vec<_> = relation.slots.iter().cloned().collect();
    let indexes: BTreeMap<_, _> = slots
        .iter()
        .cloned()
        .enumerate()
        .map(|(n, slot)| (slot, n))
        .collect();
    // 0 absent, 1 a zero-constructor path, 2 a path containing a constructor.
    // Transitive weighted closure is independent of the production SCC test.
    let mut paths = vec![vec![0_u8; slots.len()]; slots.len()];
    for (from, to, positive) in &relation.edges {
        let (i, j) = (indexes[from], indexes[to]);
        paths[i][j] = paths[i][j].max(if *positive { 2 } else { 1 });
    }
    for k in 0..slots.len() {
        for i in 0..slots.len() {
            for j in 0..slots.len() {
                if paths[i][k] != 0 && paths[k][j] != 0 {
                    paths[i][j] = paths[i][j].max(paths[i][k]).max(paths[k][j]);
                }
            }
        }
    }
    let positive_diagonal = (0..slots.len())
        .find(|n| paths[*n][*n] == 2)
        .map(|n| slots[n].clone());
    Ok(Proof {
        finite: positive_diagonal.is_none(),
        positive_diagonal,
        contexts: relation.contexts.len(),
        slots,
        edges: relation.edges,
    })
}

fn function_id(n: u16) -> Definition {
    Definition {
        package: 0,
        declaration: n,
    }
}
fn scheme_id(n: u16) -> Definition {
    Definition {
        package: 1,
        declaration: n,
    }
}
fn contract_id(n: u16) -> Definition {
    Definition {
        package: 2,
        declaration: n,
    }
}
fn parameter(scope: Definition, ordinal: usize) -> Term {
    Term::Parameter { scope, ordinal }
}
fn boxed(term: Term) -> Term {
    Term::Constructor(0, vec![term])
}
fn pair(a: Term, b: Term) -> Term {
    Term::Constructor(1, vec![a, b])
}
fn projection(scope: Definition, root: u16, contract: Definition, member: u16) -> Term {
    Term::Projection {
        scope,
        parameter: root,
        member: Member {
            contract,
            id: member,
        },
    }
}
fn out(scope: Definition, root: u16) -> Term {
    projection(scope, root, contract_id(0), 0)
}
fn path(root: u16) -> WitnessPath {
    WitnessPath {
        root,
        prerequisites: vec![],
    }
}
fn forwarded(scope: Definition, root: u16) -> Operand {
    Operand::Parameter {
        scope,
        path: path(root),
    }
}
fn selected(id: Definition, arguments: Vec<Term>) -> Witness {
    Witness::Selected {
        scheme: id,
        arguments,
        prerequisites: vec![],
    }
}
fn applied(id: Definition, arguments: Vec<Term>) -> Operand {
    Operand::Concrete {
        scheme: id,
        arguments,
        prerequisites: vec![],
    }
}
fn application(target: Definition, arguments: Vec<Term>, implementations: Vec<Operand>) -> Call {
    Call {
        target,
        arguments,
        implementations,
        kind: CallKind::Function,
        method_guard: None,
        untaken: false,
        phantom: false,
    }
}
fn unary(binding: Term, actual: Term) -> Program {
    let mut program = Program::default();
    program
        .contracts
        .insert(contract_id(0), [0].into_iter().collect());
    program.schemes.insert(
        scheme_id(0),
        Scheme {
            contract: contract_id(0),
            parameters: 1,
            prerequisite_contracts: vec![],
            bindings: [(0, binding)].into_iter().collect(),
        },
    );
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![applied(scheme_id(0), vec![actual])],
            )],
        },
    );
    program.entries.push(State {
        function: function_id(0),
        arguments: vec![],
        implementations: vec![selected(scheme_id(0), vec![Term::Ground(0)])],
    });
    program
}
fn mutual(binding: Term, projection_actual: Term, relay_actual: Term) -> Program {
    let mut program = unary(binding, Term::Ground(0));
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(1),
                vec![projection_actual],
                vec![forwarded(function_id(0), 7)],
            )],
        },
    );
    program.functions.insert(
        function_id(1),
        Function {
            parameters: 1,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![applied(scheme_id(0), vec![relay_actual])],
            )],
        },
    );
    program
}
fn two_parameter(left: Term, right: Term, first: Term, second: Term) -> Program {
    let mut program = Program::default();
    program
        .contracts
        .insert(contract_id(0), [0, 1].into_iter().collect());
    program.schemes.insert(
        scheme_id(0),
        Scheme {
            contract: contract_id(0),
            parameters: 2,
            prerequisite_contracts: vec![],
            bindings: [(0, left), (1, right)].into_iter().collect(),
        },
    );
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![applied(scheme_id(0), vec![first, second])],
            )],
        },
    );
    program.entries.push(State {
        function: function_id(0),
        arguments: vec![],
        implementations: vec![selected(
            scheme_id(0),
            vec![Term::Ground(0), Term::Ground(1)],
        )],
    });
    program
}
fn alias_permutation() -> Program {
    two_parameter(
        parameter(scheme_id(0), 0),
        parameter(scheme_id(0), 1),
        projection(function_id(0), 7, contract_id(0), 1),
        out(function_id(0), 7),
    )
}
fn prerequisite_permutation() -> Program {
    let mut program = alias_permutation();
    program
        .contracts
        .insert(contract_id(1), [0].into_iter().collect());
    program.schemes.insert(
        scheme_id(1),
        Scheme {
            contract: contract_id(1),
            parameters: 0,
            prerequisite_contracts: vec![contract_id(0)],
            bindings: [(0, Term::Ground(0))].into_iter().collect(),
        },
    );
    let mut entry = application(
        function_id(1),
        vec![],
        vec![Operand::Parameter {
            scope: function_id(0),
            path: WitnessPath {
                root: 7,
                prerequisites: vec![0],
            },
        }],
    );
    entry.kind = CallKind::SelectedMethodEntry;
    entry.method_guard = Some((7, scheme_id(1)));
    program.functions.insert(
        function_id(0),
        Function {
            parameters: 0,
            witnesses: vec![(7, contract_id(1))],
            calls: vec![entry],
        },
    );
    program.functions.insert(
        function_id(1),
        Function {
            parameters: 0,
            witnesses: vec![(11, contract_id(0))],
            calls: vec![application(
                function_id(0),
                vec![],
                vec![Operand::Concrete {
                    scheme: scheme_id(1),
                    arguments: vec![],
                    prerequisites: vec![applied(
                        scheme_id(0),
                        vec![
                            projection(function_id(1), 11, contract_id(0), 1),
                            out(function_id(1), 11),
                        ],
                    )],
                }],
            )],
        },
    );
    program.entries = vec![State {
        function: function_id(0),
        arguments: vec![],
        implementations: vec![Witness::Selected {
            scheme: scheme_id(1),
            arguments: vec![],
            prerequisites: vec![selected(
                scheme_id(0),
                vec![Term::Ground(0), Term::Ground(1)],
            )],
        }],
    }];
    program
}
fn check_finite(program: &Program, maximum_states: usize) {
    let proof = reference_classify(program, Limits::default()).unwrap();
    assert!(
        proof.finite,
        "unexpected positive diagonal {:?}",
        proof.positive_diagonal
    );
    assert_eq!(proof.positive_diagonal, None);
    let closure = program
        .concrete_closure(Limits {
            concrete_states: maximum_states,
            ..Limits::default()
        })
        .unwrap();
    assert!(!closure.is_empty());
    assert!(closure.len() <= maximum_states);
}

#[test]
fn associated_reference_accepts_alias_permutation_and_constant_resets() {
    let f = function_id(0);
    let alias = unary(parameter(scheme_id(0), 0), out(f, 7));
    check_finite(&alias, 1);
    check_finite(&alias_permutation(), 2);
    check_finite(&prerequisite_permutation(), 4);
    check_finite(&unary(Term::Ground(0), boxed(out(f, 7))), 2);
    check_finite(
        &unary(boxed(parameter(scheme_id(0), 0)), Term::Ground(0)),
        1,
    );
    let mut fixed = unary(boxed(parameter(scheme_id(0), 0)), Term::Ground(0));
    fixed.functions.get_mut(&f).unwrap().parameters = 1;
    fixed.functions.get_mut(&f).unwrap().calls =
        vec![application(f, vec![out(f, 7)], vec![forwarded(f, 7)])];
    fixed.entries[0].arguments = vec![Term::Ground(1)];
    check_finite(&fixed, 2);
}

#[test]
fn associated_reference_rejects_witness_mutual_and_ordinary_growth() {
    let f = function_id(0);
    let g = function_id(1);
    let s = scheme_id(0);
    let direct = unary(boxed(parameter(s, 0)), out(f, 7));
    let mixed = mutual(boxed(parameter(s, 0)), out(f, 7), parameter(g, 0));
    let outer = unary(parameter(s, 0), boxed(out(f, 7)));
    let mut ordinary = Program::default();
    ordinary.functions.insert(
        f,
        Function {
            parameters: 1,
            witnesses: vec![],
            calls: vec![application(f, vec![boxed(parameter(f, 0))], vec![])],
        },
    );
    ordinary.entries.push(State {
        function: f,
        arguments: vec![Term::Ground(0)],
        implementations: vec![],
    });
    let mut self_unchanged = mixed.clone();
    self_unchanged.schemes.get_mut(&s).unwrap().parameters = 2;
    self_unchanged
        .schemes
        .get_mut(&s)
        .unwrap()
        .bindings
        .insert(0, boxed(parameter(s, 1)));
    self_unchanged.functions.get_mut(&f).unwrap().parameters = 2;
    self_unchanged.functions.get_mut(&f).unwrap().calls = vec![application(
        g,
        vec![parameter(f, 0), out(f, 7)],
        vec![forwarded(f, 7)],
    )];
    self_unchanged.functions.get_mut(&g).unwrap().parameters = 2;
    self_unchanged.functions.get_mut(&g).unwrap().calls = vec![application(
        f,
        vec![parameter(g, 0), parameter(g, 1)],
        vec![applied(s, vec![parameter(g, 0), parameter(g, 1)])],
    )];
    self_unchanged.entries[0].arguments = vec![Term::Ground(2), Term::Ground(0)];
    self_unchanged.entries[0].implementations =
        vec![selected(s, vec![Term::Ground(2), Term::Ground(0)])];
    // Selected method entry transfers scheme arguments into ordinary parameters
    // even without any projection in that particular recursive application.
    let mut method = direct.clone();
    let mut entry = application(
        g,
        vec![Term::SelectedArgument {
            scope: f,
            path: path(7),
            ordinal: 0,
        }],
        vec![],
    );
    entry.kind = CallKind::SelectedMethodEntry;
    entry.method_guard = Some((7, s));
    method.functions.get_mut(&f).unwrap().calls = vec![entry];
    method.functions.insert(
        g,
        Function {
            parameters: 1,
            witnesses: vec![],
            calls: vec![application(
                f,
                vec![],
                vec![applied(s, vec![boxed(parameter(g, 0))])],
            )],
        },
    );
    for program in [direct, mixed, outer, ordinary, self_unchanged, method] {
        let proof = reference_classify(&program, Limits::default()).unwrap();
        assert!(!proof.finite);
        assert!(proof.positive_diagonal.is_some());
        assert!(proof.edges.iter().any(|(_, _, positive)| *positive));
    }
}

#[test]
fn associated_reference_keeps_exact_roots_paths_and_unknown_anchors() {
    let f = function_id(0);
    let s = scheme_id(0);
    let contract = contract_id(0);
    let mut roots = unary(boxed(parameter(s, 0)), Term::Ground(0));
    roots.functions.get_mut(&f).unwrap().witnesses = vec![(7, contract), (11, contract)];
    roots.functions.get_mut(&f).unwrap().calls = vec![application(
        f,
        vec![],
        vec![
            applied(s, vec![out(f, 11)]),
            applied(s, vec![Term::Ground(0)]),
        ],
    )];
    roots.entries[0].implementations = vec![
        selected(s, vec![Term::Ground(1)]),
        selected(s, vec![Term::Ground(2)]),
    ];
    check_finite(&roots, 3);
    let proof = reference_classify(&roots, Limits::default()).unwrap();
    let mut roots_by_context: BTreeMap<usize, BTreeSet<u16>> = BTreeMap::new();
    for slot in &proof.slots {
        if let Slot::WitnessArgument { context, path, .. } = slot {
            roots_by_context
                .entry(*context)
                .or_default()
                .insert(path.root);
        }
    }
    assert!(
        roots_by_context
            .values()
            .any(|roots| roots == &[7, 11].into_iter().collect())
    );
    let nested = prerequisite_permutation();
    let proof = reference_classify(&nested, Limits::default()).unwrap();
    assert!(proof.slots.iter().any(
        |slot| matches!(slot, Slot::WitnessArgument { path, .. } if path.prerequisites == [0])
    ));
    let a = Witness::Unknown {
        identity: UnknownIdentity::Input {
            scope: f,
            parameter: 7,
            instance: 0,
        },
        contract,
    };
    let b = Witness::Unknown {
        identity: UnknownIdentity::Input {
            scope: Definition { package: 9, ..f },
            parameter: 7,
            instance: 0,
        },
        contract,
    };
    assert_ne!(
        roots.project(&a, Member { contract, id: 0 }).unwrap(),
        roots.project(&b, Member { contract, id: 0 }).unwrap()
    );
    let mut equal = roots.clone();
    equal
        .schemes
        .get_mut(&s)
        .unwrap()
        .bindings
        .insert(0, Term::Ground(0));
    equal
        .schemes
        .insert(scheme_id(1), equal.schemes[&s].clone());
    let a = selected(s, vec![Term::Ground(2)]);
    let b = selected(scheme_id(1), vec![Term::Ground(2)]);
    assert_eq!(
        equal.project(&a, Member { contract, id: 0 }).unwrap(),
        equal.project(&b, Member { contract, id: 0 }).unwrap()
    );
    assert_ne!(
        a, b,
        "equal normalized types do not identify witness authority"
    );
}

#[test]
fn associated_reference_substitution_is_simultaneous_and_capture_free() {
    let caller = function_id(0);
    let callee = function_id(1);
    let s = scheme_id(0);
    let contract = contract_id(0);
    let mut program = unary(boxed(parameter(s, 0)), Term::Ground(0));
    program.functions.insert(
        caller,
        Function {
            parameters: 2,
            witnesses: vec![(7, contract), (11, contract)],
            calls: vec![],
        },
    );
    program.functions.insert(
        callee,
        Function {
            parameters: 2,
            witnesses: vec![(7, contract)],
            calls: vec![],
        },
    );
    let swapped = program
        .rebind(
            &pair(parameter(callee, 0), parameter(callee, 1)),
            callee,
            caller,
            &[parameter(caller, 1), parameter(caller, 0)],
            &[forwarded(caller, 11)],
        )
        .unwrap();
    assert_eq!(swapped, pair(parameter(caller, 1), parameter(caller, 0)));
    assert_eq!(
        program
            .rebind(
                &out(callee, 7),
                callee,
                caller,
                &[parameter(caller, 0), parameter(caller, 1)],
                &[forwarded(caller, 11)]
            )
            .unwrap(),
        out(caller, 11)
    );
    assert_eq!(
        program
            .rebind(
                &out(callee, 7),
                callee,
                caller,
                &[parameter(caller, 0), parameter(caller, 1)],
                &[applied(s, vec![out(caller, 7)])]
            )
            .unwrap(),
        boxed(out(caller, 7))
    );
    assert_eq!(
        program.rebind(
            &out(function_id(9), 7),
            callee,
            caller,
            &[parameter(caller, 0), parameter(caller, 1)],
            &[forwarded(caller, 11)]
        ),
        Err(Refusal::Scope)
    );
    let zero = unary(boxed(parameter(s, 0)), out(caller, 7));
    let env = Environment {
        scope: caller,
        declaration: zero.function(caller).unwrap(),
        arguments: &[],
        implementations: &zero.entries[0].implementations,
    };
    assert_eq!(
        zero.evaluate_type(&out(caller, 7), &env).unwrap(),
        boxed(Term::Ground(0))
    );
}

#[test]
fn associated_reference_checks_unused_meaning_and_capacity_separately() {
    let f = function_id(0);
    let s = scheme_id(0);
    let mut phantom = unary(boxed(parameter(s, 0)), out(f, 7));
    let call = &mut phantom.functions.get_mut(&f).unwrap().calls[0];
    call.kind = CallKind::FunctionValue;
    call.untaken = true;
    call.phantom = true;
    let mut mapping = application(f, vec![], vec![forwarded(f, 7)]);
    mapping.kind = CallKind::UnusedMapping;
    phantom.functions.get_mut(&f).unwrap().calls.push(mapping);
    assert!(
        phantom.functions[&f]
            .calls
            .iter()
            .any(|c| c.untaken && c.phantom)
    );
    let proof = reference_classify(&phantom, Limits::default()).unwrap();
    assert!(!proof.finite);
    let exact = Limits {
        contexts: proof.contexts,
        slots: proof.slots.len(),
        ..Limits::default()
    };
    assert!(!reference_classify(&phantom, exact).unwrap().finite);
    assert!(matches!(
        reference_classify(
            &phantom,
            Limits {
                slots: proof.slots.len() - 1,
                ..exact
            }
        ),
        Err(Refusal::Capacity("reference slots"))
    ));
    assert!(matches!(
        reference_classify(
            &phantom,
            Limits {
                contexts: proof.contexts - 1,
                ..exact
            }
        ),
        Err(Refusal::Capacity("reference contexts"))
    ));
    assert_eq!(
        phantom.concrete_closure(Limits {
            concrete_states: 3,
            ..Limits::default()
        }),
        Err(Refusal::Capacity("concrete states"))
    );
    assert!(
        !reference_classify(&phantom, Limits::default())
            .unwrap()
            .finite,
        "fresh analysis recovery"
    );
    let mut foreign = phantom.clone();
    let Operand::Concrete { arguments, .. } =
        &mut foreign.functions.get_mut(&f).unwrap().calls[0].implementations[0]
    else {
        panic!("fixture concrete");
    };
    arguments[0] = out(function_id(9), 7);
    assert_eq!(foreign.admit(), Err(Refusal::Scope));
    let mut unused = phantom.clone();
    unused.contracts.get_mut(&contract_id(0)).unwrap().insert(1);
    unused
        .schemes
        .get_mut(&s)
        .unwrap()
        .bindings
        .insert(1, out(f, 7));
    assert_eq!(unused.admit(), Err(Refusal::ProjectionInBinding));
    let mut wrong_contract = phantom.clone();
    wrong_contract
        .contracts
        .insert(contract_id(1), [0].into_iter().collect());
    let Operand::Concrete { arguments, .. } =
        &mut wrong_contract.functions.get_mut(&f).unwrap().calls[0].implementations[0]
    else {
        panic!("fixture concrete");
    };
    arguments[0] = projection(f, 7, contract_id(1), 0);
    assert_eq!(wrong_contract.admit(), Err(Refusal::ForeignContract));
    let mut wrap = phantom;
    wrap.schemes.insert(
        scheme_id(1),
        Scheme {
            contract: contract_id(0),
            parameters: 0,
            prerequisite_contracts: vec![contract_id(0)],
            bindings: [(0, Term::Ground(0))].into_iter().collect(),
        },
    );
    wrap.functions.get_mut(&f).unwrap().calls = vec![application(
        f,
        vec![],
        vec![Operand::Concrete {
            scheme: scheme_id(1),
            arguments: vec![],
            prerequisites: vec![forwarded(f, 7)],
        }],
    )];
    let mut foreign_prerequisite = wrap.clone();
    foreign_prerequisite
        .contracts
        .insert(contract_id(1), [0].into_iter().collect());
    foreign_prerequisite
        .schemes
        .get_mut(&scheme_id(1))
        .unwrap()
        .prerequisite_contracts[0] = contract_id(1);
    assert_eq!(foreign_prerequisite.admit(), Err(Refusal::ForeignContract));
    assert_eq!(
        wrap.admit(),
        Err(Refusal::UnsupportedRecursiveWitnessConstruction)
    );
}

#[test]
fn associated_reference_small_grammars_exhaust_every_classified_finite_instance() {
    // Exact bounded grammar from the independently executed ignored prototype.
    // Exhausting these initial-state closures does not prove general soundness;
    // growing classifications come from weighted diagonals, never a finite prefix.
    let f = function_id(0);
    let g = function_id(1);
    let s = scheme_id(0);
    let bindings = [
        parameter(s, 0),
        Term::Ground(0),
        Term::Ground(1),
        boxed(parameter(s, 0)),
        boxed(Term::Ground(0)),
        pair(parameter(s, 0), parameter(s, 0)),
        pair(parameter(s, 0), Term::Ground(0)),
    ];
    let actuals = |owner| {
        [
            out(owner, 7),
            boxed(out(owner, 7)),
            Term::Ground(0),
            Term::Ground(1),
            boxed(Term::Ground(0)),
            pair(out(owner, 7), Term::Ground(0)),
            pair(out(owner, 7), out(owner, 7)),
        ]
    };
    let relays = [
        parameter(g, 0),
        Term::Ground(0),
        Term::Ground(1),
        boxed(parameter(g, 0)),
        boxed(Term::Ground(0)),
        pair(parameter(g, 0), parameter(g, 0)),
        pair(parameter(g, 0), Term::Ground(0)),
    ];
    let mut counts = (0, 0, 0);
    let mut check = |mut program: Program| {
        // The unary grammar starts from a third ground symbol, Cell;
        // constants 0/1 name A/B, matching the retained prototype's alphabet.
        let unary_grammar = program.functions.len() == 1;
        if let Witness::Selected { arguments, .. } = &mut program.entries[0].implementations[0]
            && unary_grammar
            && arguments.len() == 1
        {
            arguments[0] = Term::Ground(2);
        }
        counts.0 += 1;
        let proof = reference_classify(&program, Limits::default()).unwrap();
        if proof.finite {
            counts.1 += 1;
            assert!(
                !program
                    .concrete_closure(Limits::default())
                    .unwrap()
                    .is_empty()
            );
        } else {
            counts.2 += 1;
            assert!(proof.positive_diagonal.is_some());
        }
    };
    for binding in &bindings {
        for actual in actuals(f) {
            check(unary(binding.clone(), actual));
        }
    }
    for binding in &bindings {
        for actual in actuals(f) {
            for relay in &relays {
                check(mutual(binding.clone(), actual.clone(), relay.clone()));
            }
        }
    }
    let two_bindings = [
        parameter(s, 0),
        parameter(s, 1),
        Term::Ground(0),
        boxed(parameter(s, 0)),
        boxed(parameter(s, 1)),
        pair(parameter(s, 0), parameter(s, 1)),
    ];
    let two_actuals = [
        out(f, 7),
        projection(f, 7, contract_id(0), 1),
        Term::Ground(0),
        Term::Ground(1),
        boxed(out(f, 7)),
        boxed(projection(f, 7, contract_id(0), 1)),
    ];
    for left in &two_bindings {
        for right in &two_bindings {
            for first in &two_actuals {
                for second in &two_actuals {
                    check(two_parameter(
                        left.clone(),
                        right.clone(),
                        first.clone(),
                        second.clone(),
                    ));
                }
            }
        }
    }
    assert_eq!(counts, (1688, 958, 730));
}
