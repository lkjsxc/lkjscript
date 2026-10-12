//! Independent raw-canonical adapter for the first-order flow oracle. This lane
//! neither consumes production flow edges nor calls production type substitution.
use super::associated_flow_reference as model;
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionControl;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ExpressionId, ImplementationParameterId, MethodId, TypeParameterId,
};
use std::collections::{BTreeMap, BTreeSet};

const MAXIMUM_REFERENCE_BYTES: usize = 64 * 1_048_576;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ReferenceFlowWork {
    pub contexts: usize,
    pub slots: usize,
    pub edges: usize,
    pub metadata_bytes: usize,
}

/// Complete symbolic source proof, before any concrete preparation queue grows.
/// Callers independently bind the supplied snapshots to their exact closure;
/// this adapter consumes raw canonical owners, never producer selections/edges.
pub(crate) fn validate(
    snapshots: &[&KernelSnapshot],
    control: &ExecutionControl,
    work: &mut usize,
    maximum_work: usize,
) -> std::result::Result<ReferenceFlowWork, Diagnostic> {
    let checkpoint = || {
        control
            .check()
            .map_err(|error| Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message))
    };
    let budget = model::Budget::new(&checkpoint, work, maximum_work, MAXIMUM_REFERENCE_BYTES);
    let result = (|| {
        budget.tick()?;
        let mut associated = false;
        for source in snapshots {
            for owner in source.owners.values() {
                budget.tick()?;
                if let OwnerRecord::Declaration(declaration) = owner {
                    associated |= declaration.payload.has_associated_types();
                }
            }
            for object in source
                .types
                .values()
                .chain(source.dependency_types.values())
            {
                budget.tick()?;
                associated |= matches!(
                    object.form,
                    TypeForm::AssociatedContractMember { .. }
                        | TypeForm::AssociatedWitnessProjection { .. }
                );
            }
        }
        if !associated {
            return Ok(ReferenceFlowWork::default());
        }
        let canonical = CanonicalProgram::read_with_budget(snapshots, &[], &budget)?;
        let proof = model::reference_classify_with_budget(
            &canonical.program,
            model::Limits {
                contexts: maximum_work,
                slots: maximum_work,
                #[cfg(test)]
                concrete_states: model::Limits::default().concrete_states,
            },
            &budget,
        )?;
        if !proof.finite || proof.positive_diagonal.is_some() {
            return Err(model::Refusal::PositiveRecurrence);
        }
        Ok(ReferenceFlowWork {
            contexts: proof.contexts,
            slots: proof.slots.len(),
            edges: proof.edges.len(),
            metadata_bytes: budget.bytes(),
        })
    })();
    result.map_err(refusal_diagnostic)
}

fn refusal_diagnostic(refusal: model::Refusal) -> Diagnostic {
    match refusal {
        model::Refusal::Control(error) => error,
        model::Refusal::Capacity(kind) => Diagnostic::new(
            DiagnosticClass::Resource,
            "kernel_associated_reference_capacity",
            format!("independent associated flow exhausted {kind}"),
        ),
        model::Refusal::PositiveRecurrence => Diagnostic::new(
            DiagnosticClass::Semantic,
            "kernel_associated_reference_growth",
            "independent associated flow found a constructor-growing recursive cycle",
        ),
        model::Refusal::UnsupportedRecursiveWitnessConstruction => Diagnostic::new(
            DiagnosticClass::Source,
            "kernel_associated_reference_witness_construction_unsupported",
            "recursive construction of a witness prerequisite tree is unsupported",
        ),
        other => Diagnostic::new(
            DiagnosticClass::Semantic,
            "kernel_associated_reference_source",
            format!("independent associated flow refused canonical source: {other:?}"),
        ),
    }
}

type Result<T> = std::result::Result<T, model::Refusal>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum DefinitionIdentity {
    Declaration(DeclarationReference),
    MethodMapping(DeclarationReference, MethodId),
    ExecutableOwner(PackageId, OwnerKey),
}

/// Every atom retains its complete non-child fields. Different field names,
/// nominal identities, callable kinds and effect rows cannot collapse to one tag.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ConstructorAtom {
    pub kind: &'static str,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug)]
pub(super) struct CanonicalEntry {
    pub function: DeclarationReference,
    pub arguments: Vec<TypeObjectDigest>,
    pub implementations: Vec<ImplementationOperand>,
}

pub(super) struct CanonicalProgram {
    pub program: model::Program,
    pub definitions: BTreeMap<model::Definition, DefinitionIdentity>,
    pub members: BTreeMap<model::Member, (DeclarationReference, AssociatedTypeMemberId)>,
    pub witness_parameters: BTreeMap<(model::Definition, usize), ImplementationParameterId>,
    pub atoms: BTreeMap<usize, ConstructorAtom>,
    /// Abstract signature members remain exact references, never ground terms.
    pub signature_members: BTreeSet<(DeclarationReference, AssociatedTypeMemberId)>,
    pub signature_types: BTreeMap<(DeclarationReference, TypeObjectDigest), SignatureTerm>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SignatureTerm {
    Ordinary(model::Term),
    Member(DeclarationReference, AssociatedTypeMemberId),
    Constructor(usize, Vec<SignatureTerm>),
}

#[derive(Clone, Copy)]
enum TypeMode {
    Callable,
    Binding,
    Contract,
    Nominal,
    Closed,
}

#[derive(Clone, Copy)]
struct Context {
    source: DeclarationReference,
    target: model::Definition,
    mode: TypeMode,
    generation: u16,
}

struct Adapter<'a, 'b> {
    budget: &'a model::Budget<'b>,
    sources: BTreeMap<PackageId, &'a KernelSnapshot>,
    // Raw exact closure inventory, independent of producer selections. Vectors
    // retain declaration traversal order, including private and unused schemes.
    implementations_by_contract: BTreeMap<DeclarationReference, Vec<DeclarationReference>>,
    ids: BTreeMap<DefinitionIdentity, model::Definition>,
    member_ids: BTreeMap<(DeclarationReference, AssociatedTypeMemberId), model::Member>,
    witness_ids: BTreeMap<(DeclarationReference, ImplementationParameterId), usize>,
    atom_ids: BTreeMap<ConstructorAtom, usize>,
    output: CanonicalProgram,
}

impl CanonicalProgram {
    #[cfg(test)]
    pub(super) fn read(snapshots: &[&KernelSnapshot], entries: &[CanonicalEntry]) -> Result<Self> {
        let checkpoint = || Ok(());
        let mut work = 0;
        let budget = model::Budget::new(&checkpoint, &mut work, usize::MAX, usize::MAX);
        Self::read_with_budget(snapshots, entries, &budget)
    }
    pub(super) fn read_with_budget(
        snapshots: &[&KernelSnapshot],
        entries: &[CanonicalEntry],
        budget: &model::Budget<'_>,
    ) -> Result<Self> {
        budget.reserve::<(PackageId, &KernelSnapshot)>(snapshots.len())?;
        let mut adapter = Adapter {
            budget,
            sources: BTreeMap::new(),
            implementations_by_contract: BTreeMap::new(),
            ids: BTreeMap::new(),
            member_ids: BTreeMap::new(),
            witness_ids: BTreeMap::new(),
            atom_ids: BTreeMap::new(),
            output: Self {
                program: model::Program::default(),
                definitions: BTreeMap::new(),
                members: BTreeMap::new(),
                witness_parameters: BTreeMap::new(),
                atoms: BTreeMap::new(),
                signature_members: BTreeSet::new(),
                signature_types: BTreeMap::new(),
            },
        };
        for snapshot in snapshots {
            if adapter
                .sources
                .insert(snapshot.root.package_id, *snapshot)
                .is_some()
            {
                return Err(model::Refusal::Inventory);
            }
            for (key, owner) in &snapshot.owners {
                budget.tick()?;
                if owner.header().owner != *key {
                    return Err(model::Refusal::Inventory);
                }
            }
            for (digest, object) in snapshot.types.iter().chain(&snapshot.dependency_types) {
                budget.canonical_reserve(object)?;
                if encode_type_object(object)
                    .map_err(|_| model::Refusal::Inventory)?
                    .0
                    != *digest
                {
                    return Err(model::Refusal::Inventory);
                }
            }
        }
        adapter.inventory()?;
        let declarations = adapter.declarations()?;
        for reference in declarations {
            adapter.translate_declaration(reference)?;
        }
        adapter.translate_ports()?;
        budget.reserve::<model::State>(entries.len())?;
        for entry in entries {
            budget.reserve::<model::Term>(entry.arguments.len())?;
            budget.reserve::<model::Witness>(entry.implementations.len())?;
            let context = adapter.context(entry.function, TypeMode::Closed)?;
            let arguments = entry
                .arguments
                .iter()
                .map(|ty| adapter.term(*ty, context, 0))
                .collect::<Result<_>>()?;
            let implementations = entry
                .implementations
                .iter()
                .map(|operand| adapter.closed_witness(operand, context, 0))
                .collect::<Result<_>>()?;
            adapter.output.program.entries.push(model::State {
                function: context.target,
                arguments,
                implementations,
            });
        }
        adapter.output.program.admit_with_budget(budget)?;
        Ok(adapter.output)
    }

    #[cfg(test)]
    pub(super) fn definition(&self, reference: DeclarationReference) -> Result<model::Definition> {
        self.definitions
            .iter()
            .find_map(|(id, identity)| {
                (*identity == DefinitionIdentity::Declaration(reference)).then_some(*id)
            })
            .ok_or(model::Refusal::MissingDefinition)
    }
}

impl Adapter<'_, '_> {
    fn source(&self, package: PackageId) -> Result<&KernelSnapshot> {
        self.budget.tick()?;
        self.sources
            .get(&package)
            .copied()
            .ok_or(model::Refusal::MissingDefinition)
    }

    fn declaration(&self, reference: DeclarationReference) -> Result<DeclarationRecord> {
        match self
            .source(reference.package)?
            .owners
            .get(&OwnerKey::Declaration(reference.declaration))
        {
            Some(OwnerRecord::Declaration(record)) => self.budget.canonical_clone(record),
            _ => Err(model::Refusal::MissingDefinition),
        }
    }

    fn authored(
        &self,
        from: PackageId,
        reference: DeclarationReference,
    ) -> Result<DeclarationRecord> {
        let declaration = self.declaration(reference)?;
        if from != reference.package
            && (!self
                .source(from)?
                .dependencies
                .contains_key(&reference.package)
                || declaration.visibility != DeclarationVisibility::Public)
        {
            return Err(model::Refusal::Scope);
        }
        Ok(declaration)
    }

    fn declarations(&self) -> Result<Vec<DeclarationReference>> {
        let count = self
            .sources
            .values()
            .try_fold(0usize, |n, s| n.checked_add(s.owners.len()))
            .ok_or(model::Refusal::Capacity("canonical owner count"))?;
        self.budget.reserve::<DeclarationReference>(count)?;
        Ok(self
            .sources
            .iter()
            .flat_map(|(package, source)| {
                source.owners.keys().filter_map(move |key| match key {
                    OwnerKey::Declaration(declaration) => Some(DeclarationReference {
                        package: *package,
                        declaration: *declaration,
                    }),
                    _ => None,
                })
            })
            .collect())
    }

    fn inventory(&mut self) -> Result<()> {
        self.budget
            .reserve::<(PackageId, usize)>(self.sources.len())?;
        self.budget
            .reserve::<(PackageId, usize)>(self.sources.len())?;
        let mut package_ids = BTreeMap::new();
        for (ordinal, package) in self.sources.keys().enumerate() {
            package_ids.insert(*package, ordinal);
        }
        let mut next = BTreeMap::<PackageId, usize>::new();
        for reference in self.declarations()? {
            let ordinal = next.entry(reference.package).or_default();
            let id = model::Definition {
                package: package_ids[&reference.package],
                declaration: *ordinal,
            };
            *ordinal = ordinal
                .checked_add(1)
                .ok_or(model::Refusal::Capacity("canonical definition count"))?;
            self.budget
                .reserve::<(DefinitionIdentity, model::Definition)>(1)?;
            self.budget
                .reserve::<(model::Definition, DefinitionIdentity)>(1)?;
            self.ids
                .insert(DefinitionIdentity::Declaration(reference), id);
            self.output
                .definitions
                .insert(id, DefinitionIdentity::Declaration(reference));
            let declaration = self.declaration(reference)?;
            if let DeclarationPayload::OwnedImplementation(implementation) = &declaration.payload {
                if !self
                    .implementations_by_contract
                    .contains_key(&implementation.contract)
                {
                    self.budget
                        .reserve::<(DeclarationReference, Vec<DeclarationReference>)>(1)?;
                }
                self.budget.reserve::<DeclarationReference>(1)?;
                self.implementations_by_contract
                    .entry(implementation.contract)
                    .or_default()
                    .push(reference);
            }
            self.parameters(reference)?;
            let witnesses = self.witnesses(reference)?;
            self.budget
                .reserve::<((DeclarationReference, ImplementationParameterId), usize)>(
                    witnesses.len(),
                )?;
            self.budget.reserve::<Name>(witnesses.len())?;
            let mut seen_names = BTreeSet::new();
            for (n, witness) in witnesses.iter().enumerate() {
                self.budget.reserve_bytes(witness.name.as_str().len())?;
                let id = n;
                if self
                    .witness_ids
                    .insert((reference, witness.id), id)
                    .is_some()
                    || !seen_names.insert(witness.name.clone())
                {
                    return Err(model::Refusal::Inventory);
                }
            }
            if let DeclarationPayload::OwnedContract(contract) = &declaration.payload {
                self.budget
                    .reserve::<usize>(contract.associated_types.len())?;
                self.budget
                    .reserve::<Name>(contract.associated_types.len())?;
                self.budget.reserve::<(
                    (DeclarationReference, AssociatedTypeMemberId),
                    model::Member,
                )>(contract.associated_types.len())?;
                self.budget.reserve::<(
                    model::Member,
                    (DeclarationReference, AssociatedTypeMemberId),
                )>(contract.associated_types.len())?;
                self.budget.reserve::<MethodId>(contract.methods.len())?;
                self.budget
                    .reserve::<(model::Definition, BTreeSet<usize>)>(1)?;
                let mut members = BTreeSet::new();
                let mut names = BTreeSet::new();
                for (n, member) in contract.associated_types.iter().enumerate() {
                    self.budget.reserve_bytes(member.name.as_str().len())?;
                    let member_id = model::Member {
                        contract: id,
                        id: n,
                    };
                    if self
                        .member_ids
                        .insert((reference, member.id), member_id)
                        .is_some()
                        || !names.insert(member.name.clone())
                    {
                        return Err(model::Refusal::Inventory);
                    }
                    self.output
                        .members
                        .insert(member_id, (reference, member.id));
                    members.insert(member_id.id);
                }
                if contract.methods.is_empty()
                    || contract
                        .methods
                        .iter()
                        .map(|method| method.id)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != contract.methods.len()
                {
                    return Err(model::Refusal::Inventory);
                }
                self.output.program.contracts.insert(id, members);
            }
        }
        // Synthetic mapping nodes never reuse the scheme's definition identity.
        for reference in self.declarations()? {
            if let DeclarationPayload::OwnedImplementation(implementation) =
                self.declaration(reference)?.payload
            {
                for mapping in implementation.methods {
                    let ordinal = next.entry(reference.package).or_default();
                    let id = model::Definition {
                        package: package_ids[&reference.package],
                        declaration: *ordinal,
                    };
                    *ordinal = ordinal
                        .checked_add(1)
                        .ok_or(model::Refusal::Capacity("canonical definition count"))?;
                    let identity = DefinitionIdentity::MethodMapping(reference, mapping.method);
                    self.budget
                        .reserve::<(DefinitionIdentity, model::Definition)>(1)?;
                    self.budget
                        .reserve::<(model::Definition, DefinitionIdentity)>(1)?;
                    if self.ids.insert(identity.clone(), id).is_some() {
                        return Err(model::Refusal::Inventory);
                    }
                    self.output.definitions.insert(id, identity);
                }
            }
        }
        for (package, source) in &self.sources {
            for key in source
                .owners
                .keys()
                .filter(|key| matches!(key, OwnerKey::Port(_)))
            {
                let ordinal = next.entry(*package).or_default();
                let id = model::Definition {
                    package: package_ids[package],
                    declaration: *ordinal,
                };
                *ordinal = ordinal
                    .checked_add(1)
                    .ok_or(model::Refusal::Capacity("canonical definition count"))?;
                let identity = DefinitionIdentity::ExecutableOwner(*package, *key);
                self.budget
                    .reserve::<(DefinitionIdentity, model::Definition)>(1)?;
                self.budget
                    .reserve::<(model::Definition, DefinitionIdentity)>(1)?;
                self.ids.insert(identity.clone(), id);
                self.output.definitions.insert(id, identity);
            }
        }
        Ok(())
    }

    fn id(&self, reference: DeclarationReference) -> Result<model::Definition> {
        self.ids
            .get(&DefinitionIdentity::Declaration(reference))
            .copied()
            .ok_or(model::Refusal::MissingDefinition)
    }

    fn context(&self, reference: DeclarationReference, mode: TypeMode) -> Result<Context> {
        Ok(Context {
            source: reference,
            target: self.id(reference)?,
            mode,
            generation: self.declaration(reference)?.header.contract_version,
        })
    }

    fn parameters(&self, reference: DeclarationReference) -> Result<Vec<TypeParameterId>> {
        let record = self.declaration(reference)?;
        self.budget
            .reserve::<TypeParameterId>(record.payload.type_parameters().len())?;
        let parameters = match record.payload {
            DeclarationPayload::Function(f) => f.type_parameters,
            DeclarationPayload::External(f) => f.type_parameters,
            DeclarationPayload::OwnedImplementation(i) => i.type_parameters,
            DeclarationPayload::OwnedContract(c) => std::iter::once(c.self_parameter)
                .chain(c.type_parameters)
                .collect(),
            DeclarationPayload::Record {
                type_parameters, ..
            }
            | DeclarationPayload::Variant {
                type_parameters, ..
            }
            | DeclarationPayload::OwnedRecord {
                type_parameters, ..
            } => type_parameters,
            _ => vec![],
        };
        self.budget.reserve::<TypeParameterId>(parameters.len())?;
        if parameters.iter().collect::<BTreeSet<_>>().len() != parameters.len() {
            return Err(model::Refusal::Inventory);
        }
        for parameter in &parameters {
            match self
                .source(reference.package)?
                .owners
                .get(&OwnerKey::TypeParameter(*parameter))
            {
                Some(OwnerRecord::TypeParameter(record))
                    if record.declaration == reference.declaration
                        && record.header.kind == OwnerKind::TypeParameter => {}
                _ => return Err(model::Refusal::Scope),
            }
        }
        Ok(parameters)
    }

    fn witnesses(&self, reference: DeclarationReference) -> Result<Vec<ImplementationParameter>> {
        Ok(match self.declaration(reference)?.payload {
            DeclarationPayload::Function(f) => f.implementation_parameters,
            DeclarationPayload::OwnedImplementation(i) => i.implementation_parameters,
            _ => vec![],
        })
    }

    fn member(
        &self,
        from: PackageId,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<model::Member> {
        self.authored(from, contract)?;
        self.member_ids
            .get(&(contract, member))
            .copied()
            .ok_or(model::Refusal::ForeignMember)
    }

    fn tag(&mut self, kind: &'static str, fields: Vec<String>) -> Result<usize> {
        let atom = ConstructorAtom { kind, fields };
        if let Some(id) = self.atom_ids.get(&atom) {
            return Ok(*id);
        }
        let id = self.atom_ids.len();
        self.budget.reserve::<(ConstructorAtom, usize)>(1)?;
        self.budget.reserve::<(usize, ConstructorAtom)>(1)?;
        self.budget.reserve::<String>(atom.fields.len())?;
        for field in &atom.fields {
            self.budget.reserve_bytes(field.len())?;
        }
        self.atom_ids.insert(atom.clone(), id);
        self.output.atoms.insert(id, atom);
        Ok(id)
    }

    fn object(&self, package: PackageId, digest: TypeObjectDigest) -> Result<TypeObject> {
        let source = self.source(package)?;
        source
            .types
            .get(&digest)
            .or_else(|| source.dependency_types.get(&digest))
            .map(|object| self.budget.canonical_clone(object))
            .transpose()?
            .ok_or(model::Refusal::MissingDefinition)
    }

    fn term(
        &mut self,
        digest: TypeObjectDigest,
        context: Context,
        depth: usize,
    ) -> Result<model::Term> {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(model::Refusal::Capacity("canonical type depth"));
        }
        self.budget.tick()?;
        let object = self.object(context.source.package, digest)?;
        self.budget
            .reserve::<TypeObjectDigest>(object.child_type_count())?;
        self.budget
            .reserve::<model::Term>(object.child_type_count())?;
        self.budget.reserve::<String>(
            object
                .child_type_count()
                .checked_add(2)
                .ok_or(model::Refusal::Capacity("canonical atom fields"))?,
        )?;
        self.budget.canonical_reserve(&object)?;
        let mut children = Vec::new();
        let (kind, atoms) = match object.form {
            TypeForm::TypeParameter { parameter } => {
                if matches!(context.mode, TypeMode::Closed) {
                    return Err(model::Refusal::Scope);
                }
                let parameters = self.parameters(context.source)?;
                let ordinal = parameters
                    .iter()
                    .position(|id| *id == parameter)
                    .ok_or(model::Refusal::Scope)?;
                return Ok(model::Term::Parameter {
                    scope: context.target,
                    ordinal,
                });
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                if matches!(context.mode, TypeMode::Binding) {
                    return Err(model::Refusal::ProjectionInBinding);
                }
                if !matches!(context.mode, TypeMode::Callable)
                    || scope != context.source
                    || context.generation != contract::GRAPH_CONTRACT_VERSION
                {
                    return Err(model::Refusal::Scope);
                }
                let witness = self
                    .witnesses(scope)?
                    .into_iter()
                    .find(|w| w.id == parameter)
                    .ok_or(model::Refusal::Scope)?;
                if witness.contract != contract {
                    return Err(model::Refusal::ForeignContract);
                }
                return Ok(model::Term::Projection {
                    scope: context.target,
                    parameter: self.witness_ids[&(scope, parameter)],
                    member: self.member(scope.package, contract, member)?,
                });
            }
            TypeForm::AssociatedContractMember { .. } => return Err(model::Refusal::Scope),
            TypeForm::Unit => ("unit", vec![]),
            TypeForm::Bool => ("bool", vec![]),
            TypeForm::I64 => ("i64", vec![]),
            TypeForm::F64 => ("f64", vec![]),
            TypeForm::Bytes => ("bytes", vec![]),
            TypeForm::Text => ("text", vec![]),
            TypeForm::StaticText => ("static_text", vec![]),
            TypeForm::Secret => ("secret", vec![]),
            TypeForm::ByteBuffer => ("byte_buffer", vec![]),
            TypeForm::OwnedI64Cell => ("owned_i64_cell", vec![]),
            TypeForm::Named { declaration } => {
                self.authored(context.source.package, declaration)?;
                ("named", vec![format!("{declaration:?}")])
            }
            TypeForm::CapabilityResource { interface } => {
                self.authored(context.source.package, interface)?;
                ("resource", vec![format!("{interface:?}")])
            }
            TypeForm::Applied {
                declaration,
                arguments,
            } => {
                self.authored(context.source.package, declaration)?;
                if self.parameters(declaration)?.len() != arguments.len() {
                    return Err(model::Refusal::Arity);
                }
                children = arguments;
                ("applied", vec![format!("{declaration:?}")])
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                let kind = match &self.object(context.source.package, digest)?.form {
                    TypeForm::StructuralRecord { .. } => "record",
                    TypeForm::OwnedProduct { .. } => "owned_product",
                    _ => "owned_choice",
                };
                let names = fields
                    .iter()
                    .map(|field| field.name.as_str().to_owned())
                    .collect();
                children = fields.into_iter().map(|field| field.ty).collect();
                (kind, names)
            }
            TypeForm::List { item } => {
                children.push(item);
                ("list", vec![])
            }
            TypeForm::Option { item } => {
                children.push(item);
                ("option", vec![])
            }
            TypeForm::Stream { item } => {
                children.push(item);
                ("stream", vec![])
            }
            TypeForm::OwnedSequence { item } => {
                children.push(item);
                ("owned_sequence", vec![])
            }
            TypeForm::Map { key, value } => {
                children.extend([key, value]);
                ("map", vec![])
            }
            TypeForm::Result { ok, error } => {
                children.extend([ok, error]);
                ("result", vec![])
            }
            TypeForm::Function { parameters, result } => {
                children = parameters;
                children.push(result);
                ("function", vec![])
            }
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => {
                children = parameters;
                children.push(result);
                (
                    "task_function",
                    vec![serde_json::to_string(&effect).map_err(|_| model::Refusal::Inventory)?],
                )
            }
        };
        let tag = self.tag(kind, atoms)?;
        if children.is_empty() {
            Ok(model::Term::Ground(tag))
        } else {
            Ok(model::Term::Constructor(
                tag,
                children
                    .into_iter()
                    .map(|ty| self.term(ty, context, depth + 1))
                    .collect::<Result<_>>()?,
            ))
        }
    }

    fn reserve_signature(&self, term: &SignatureTerm) -> Result<()> {
        self.budget.tick()?;
        match term {
            SignatureTerm::Ordinary(term) => self.budget.term(term),
            SignatureTerm::Member(_, _) => Ok(()),
            SignatureTerm::Constructor(_, children) => {
                self.budget.reserve::<SignatureTerm>(children.len())?;
                for child in children {
                    self.reserve_signature(child)?;
                }
                Ok(())
            }
        }
    }

    fn contract_type(
        &mut self,
        ty: TypeObjectDigest,
        context: Context,
        depth: usize,
    ) -> Result<SignatureTerm> {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(model::Refusal::Capacity("canonical signature depth"));
        }
        self.budget.tick()?;
        let object = self.object(context.source.package, ty)?;
        self.budget
            .reserve::<TypeObjectDigest>(object.child_type_count())?;
        self.budget
            .reserve::<SignatureTerm>(object.child_type_count())?;
        self.budget.canonical_reserve(&object)?;
        self.budget
            .reserve::<((DeclarationReference, TypeObjectDigest), SignatureTerm)>(1)?;
        if let TypeForm::AssociatedContractMember { contract, member } = object.form {
            if contract != context.source
                || !matches!(context.mode, TypeMode::Contract)
                || context.generation != contract::GRAPH_CONTRACT_VERSION
            {
                return Err(model::Refusal::Scope);
            }
            self.member(context.source.package, contract, member)?;
            self.budget
                .reserve::<(DeclarationReference, AssociatedTypeMemberId)>(1)?;
            self.output.signature_members.insert((contract, member));
            let term = SignatureTerm::Member(contract, member);
            self.reserve_signature(&term)?;
            self.output
                .signature_types
                .insert((context.source, ty), term.clone());
            return Ok(term);
        }
        let children = Self::type_children(&object.form);
        let term = if children.is_empty() {
            SignatureTerm::Ordinary(self.term(ty, context, depth)?)
        } else {
            let (kind, fields) = match &object.form {
                TypeForm::Applied {
                    declaration,
                    arguments,
                } => {
                    self.authored(context.source.package, *declaration)?;
                    if self.parameters(*declaration)?.len() != arguments.len() {
                        return Err(model::Refusal::Arity);
                    }
                    ("applied", vec![format!("{declaration:?}")])
                }
                TypeForm::StructuralRecord { fields } => (
                    "record",
                    fields
                        .iter()
                        .map(|field| field.name.as_str().to_owned())
                        .collect(),
                ),
                TypeForm::OwnedProduct { fields } => (
                    "owned_product",
                    fields
                        .iter()
                        .map(|field| field.name.as_str().to_owned())
                        .collect(),
                ),
                TypeForm::OwnedChoice { cases } => (
                    "owned_choice",
                    cases
                        .iter()
                        .map(|field| field.name.as_str().to_owned())
                        .collect(),
                ),
                TypeForm::Function { .. } => ("function", vec![]),
                TypeForm::TaskFunction { effect, .. } => (
                    "task_function",
                    vec![serde_json::to_string(effect).map_err(|_| model::Refusal::Inventory)?],
                ),
                TypeForm::List { .. } => ("list", vec![]),
                TypeForm::Map { .. } => ("map", vec![]),
                TypeForm::Option { .. } => ("option", vec![]),
                TypeForm::Result { .. } => ("result", vec![]),
                TypeForm::Stream { .. } => ("stream", vec![]),
                TypeForm::OwnedSequence { .. } => ("owned_sequence", vec![]),
                _ => return Err(model::Refusal::Inventory),
            };
            SignatureTerm::Constructor(
                self.tag(kind, fields)?,
                children
                    .into_iter()
                    .map(|child| self.contract_type(child, context, depth + 1))
                    .collect::<Result<_>>()?,
            )
        };
        self.reserve_signature(&term)?;
        self.output
            .signature_types
            .insert((context.source, ty), term.clone());
        Ok(term)
    }

    fn type_children(form: &TypeForm) -> Vec<TypeObjectDigest> {
        match form {
            TypeForm::Applied { arguments, .. } => arguments.clone(),
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                fields.iter().map(|field| field.ty).collect()
            }
            TypeForm::List { item }
            | TypeForm::Option { item }
            | TypeForm::Stream { item }
            | TypeForm::OwnedSequence { item } => vec![*item],
            TypeForm::Map { key, value } => vec![*key, *value],
            TypeForm::Result { ok, error } => vec![*ok, *error],
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => parameters.iter().copied().chain([*result]).collect(),
            _ => vec![],
        }
    }

    fn witness_inventory(&mut self, context: Context) -> Result<Vec<(usize, model::Definition)>> {
        let witnesses = self.witnesses(context.source)?;
        self.budget
            .reserve::<(usize, model::Definition)>(witnesses.len())?;
        self.budget
            .reserve::<((model::Definition, usize), ImplementationParameterId)>(witnesses.len())?;
        let mut result = Vec::new();
        let ordinary = self.parameters(context.source)?;
        for witness in witnesses {
            let DeclarationPayload::OwnedContract(contract) = self
                .authored(context.source.package, witness.contract)?
                .payload
            else {
                return Err(model::Refusal::ForeignContract);
            };
            if witness.type_arguments.len() != contract.type_parameters.len() {
                return Err(model::Refusal::Arity);
            }
            let TypeForm::TypeParameter { parameter } =
                self.object(context.source.package, witness.self_type)?.form
            else {
                return Err(model::Refusal::Scope);
            };
            if !ordinary.contains(&parameter) {
                return Err(model::Refusal::Scope);
            }
            match self
                .source(context.source.package)?
                .owners
                .get(&OwnerKey::TypeParameter(parameter))
            {
                Some(OwnerRecord::TypeParameter(record)) if record.constraints.has_owned() => {}
                _ => return Err(model::Refusal::Scope),
            }
            self.term(witness.self_type, context, 0)?;
            for ty in witness.type_arguments {
                self.term(ty, context, 0)?;
            }
            let id = self.witness_ids[&(context.source, witness.id)];
            self.output
                .witness_parameters
                .insert((context.target, id), witness.id);
            result.push((id, self.id(witness.contract)?));
        }
        Ok(result)
    }

    fn operand(
        &mut self,
        operand: &ImplementationOperand,
        context: Context,
        depth: usize,
    ) -> Result<model::Operand> {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return Err(model::Refusal::Capacity("canonical witness depth"));
        }
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                if !matches!(context.mode, TypeMode::Callable) || *scope != context.source {
                    return Err(model::Refusal::Scope);
                }
                let root = self
                    .witness_ids
                    .get(&(*scope, *parameter))
                    .copied()
                    .ok_or(model::Refusal::Scope)?;
                Ok(model::Operand::Parameter {
                    scope: context.target,
                    path: model::WitnessPath {
                        root,
                        prerequisites: vec![],
                    },
                })
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let DeclarationPayload::OwnedImplementation(scheme) = self
                    .authored(context.source.package, *implementation)?
                    .payload
                else {
                    return Err(model::Refusal::MissingDefinition);
                };
                if type_arguments.len() != scheme.type_parameters.len()
                    || implementations.len() != scheme.implementation_parameters.len()
                {
                    return Err(model::Refusal::Arity);
                }
                self.budget.reserve::<model::Term>(type_arguments.len())?;
                self.budget
                    .reserve::<model::Operand>(implementations.len())?;
                Ok(model::Operand::Concrete {
                    scheme: self.id(*implementation)?,
                    arguments: type_arguments
                        .iter()
                        .map(|ty| self.term(*ty, context, 0))
                        .collect::<Result<_>>()?,
                    prerequisites: implementations
                        .iter()
                        .map(|operand| self.operand(operand, context, depth + 1))
                        .collect::<Result<_>>()?,
                })
            }
        }
    }

    fn closed_witness(
        &mut self,
        operand: &ImplementationOperand,
        context: Context,
        depth: usize,
    ) -> Result<model::Witness> {
        let model::Operand::Concrete {
            scheme,
            arguments,
            prerequisites,
        } = self.operand(operand, context, depth)?
        else {
            return Err(model::Refusal::AuthoredSymbolicValue);
        };
        fn closed(input: model::Operand, budget: &model::Budget<'_>) -> Result<model::Witness> {
            budget.tick()?;
            match input {
                model::Operand::Concrete {
                    scheme,
                    arguments,
                    prerequisites,
                } => {
                    budget.reserve::<model::Witness>(prerequisites.len())?;
                    Ok(model::Witness::Selected {
                        scheme,
                        arguments,
                        prerequisites: prerequisites
                            .into_iter()
                            .map(|p| closed(p, budget))
                            .collect::<Result<_>>()?,
                    })
                }
                _ => Err(model::Refusal::AuthoredSymbolicValue),
            }
        }
        self.budget.reserve::<model::Witness>(prerequisites.len())?;
        Ok(model::Witness::Selected {
            scheme,
            arguments,
            prerequisites: prerequisites
                .into_iter()
                .map(|p| closed(p, self.budget))
                .collect::<Result<_>>()?,
        })
    }

    fn function_target(
        &self,
        from: PackageId,
        reference: DeclarationReference,
    ) -> Result<model::Definition> {
        match self.authored(from, reference)?.payload {
            DeclarationPayload::Function(_) | DeclarationPayload::External(_) => self.id(reference),
            _ => Err(model::Refusal::MissingDefinition),
        }
    }

    fn translate_ports(&mut self) -> Result<()> {
        let count = self
            .sources
            .values()
            .try_fold(0usize, |n, s| n.checked_add(s.owners.len()))
            .ok_or(model::Refusal::Capacity("canonical ports"))?;
        self.budget.reserve::<(PackageId, OwnerKey)>(count)?;
        let ports: Vec<_> = self
            .sources
            .iter()
            .flat_map(|(package, source)| {
                source.owners.iter().filter_map(move |(key, record)| {
                    matches!(record, OwnerRecord::Port(_)).then_some((*package, *key))
                })
            })
            .collect();
        for (package, key) in ports {
            let Some(OwnerRecord::Port(port)) = self
                .source(package)?
                .owners
                .get(&key)
                .map(|r| self.budget.canonical_clone(r))
                .transpose()?
            else {
                return Err(model::Refusal::Inventory);
            };
            let reference = DeclarationReference {
                package,
                declaration: port.declaration,
            };
            let DeclarationPayload::Component { ports, .. } = self.declaration(reference)?.payload
            else {
                return Err(model::Refusal::Scope);
            };
            let OwnerKey::Port(id) = key else {
                return Err(model::Refusal::Inventory);
            };
            if !ports.contains(&id) {
                return Err(model::Refusal::Scope);
            }
            let target = self.ids[&DefinitionIdentity::ExecutableOwner(package, key)];
            let context = Context {
                source: reference,
                target,
                mode: TypeMode::Closed,
                generation: port.header.contract_version,
            };
            self.term(port.function_type, context, 0)?;
            self.budget.reserve::<model::Call>(1)?;
            self.budget
                .reserve::<(model::Definition, model::Function)>(1)?;
            let mut calls = Vec::new();
            match port.implementation {
                PortImplementation::Expression(expression) => {
                    self.expressions(expression, context, &mut calls)?
                }
                PortImplementation::Function(function) => calls.push(model::Call {
                    target: self.function_target(package, function)?,
                    arguments: vec![],
                    implementations: vec![],
                    kind: model::CallKind::Function,
                    method_guard: None,
                    untaken: false,
                    phantom: false,
                }),
            }
            self.output.program.functions.insert(
                target,
                model::Function {
                    parameters: 0,
                    witnesses: vec![],
                    calls,
                },
            );
        }
        Ok(())
    }

    fn translate_declaration(&mut self, reference: DeclarationReference) -> Result<()> {
        self.budget
            .reserve::<(model::Definition, model::Function)>(1)?;
        self.budget
            .reserve::<(model::Definition, model::Scheme)>(1)?;
        match self.declaration(reference)?.payload {
            DeclarationPayload::OwnedContract(contract) => {
                let context = self.context(reference, TypeMode::Contract)?;
                match self
                    .source(reference.package)?
                    .owners
                    .get(&OwnerKey::TypeParameter(contract.self_parameter))
                {
                    Some(OwnerRecord::TypeParameter(record)) if record.constraints.has_owned() => {}
                    _ => return Err(model::Refusal::Scope),
                }
                for method in contract.methods {
                    for parameter in method.parameters {
                        self.contract_type(parameter.ty, context, 0)?;
                    }
                    self.contract_type(method.result, context, 0)?;
                }
            }
            DeclarationPayload::OwnedImplementation(implementation) => {
                let context = self.context(reference, TypeMode::Binding)?;
                let DeclarationPayload::OwnedContract(contract) = self
                    .authored(reference.package, implementation.contract)?
                    .payload
                else {
                    return Err(model::Refusal::ForeignContract);
                };
                self.budget
                    .reserve::<MethodId>(implementation.methods.len())?;
                self.budget.reserve::<MethodId>(contract.methods.len())?;
                if implementation.type_arguments.len() != contract.type_parameters.len()
                    || implementation
                        .methods
                        .iter()
                        .map(|method| method.method)
                        .collect::<BTreeSet<_>>()
                        != contract.methods.iter().map(|method| method.id).collect()
                    || implementation.methods.len() != contract.methods.len()
                {
                    return Err(model::Refusal::Inventory);
                }
                self.term(implementation.self_type, context, 0)?;
                for ty in &implementation.type_arguments {
                    self.term(*ty, context, 0)?;
                }
                let callable = Context {
                    mode: TypeMode::Callable,
                    ..context
                };
                let prerequisites = self.witness_inventory(callable)?;
                self.budget
                    .reserve::<(usize, model::Term)>(implementation.associated_types.len())?;
                self.budget
                    .reserve::<model::Definition>(prerequisites.len())?;
                self.budget
                    .reserve::<MethodId>(implementation.methods.len())?;
                self.budget.reserve::<MethodId>(contract.methods.len())?;
                let mut bindings = BTreeMap::new();
                for binding in implementation.associated_types {
                    let member =
                        self.member(reference.package, implementation.contract, binding.member)?;
                    if bindings
                        .insert(member.id, self.term(binding.ty, context, 0)?)
                        .is_some()
                    {
                        return Err(model::Refusal::Inventory);
                    }
                }
                self.output.program.schemes.insert(
                    context.target,
                    model::Scheme {
                        contract: self.id(implementation.contract)?,
                        parameters: implementation.type_parameters.len(),
                        prerequisite_contracts: prerequisites
                            .iter()
                            .map(|(_, contract)| *contract)
                            .collect(),
                        bindings,
                    },
                );
                for mapping in implementation.methods {
                    let synthetic =
                        self.ids[&DefinitionIdentity::MethodMapping(reference, mapping.method)];
                    let mapped = Context {
                        target: synthetic,
                        mode: TypeMode::Callable,
                        ..context
                    };
                    let witnesses = self.witness_inventory(mapped)?;
                    self.budget
                        .reserve::<model::Term>(mapping.type_arguments.len())?;
                    self.budget
                        .reserve::<model::Operand>(mapping.implementations.len())?;
                    self.budget.reserve::<model::Call>(1)?;
                    self.budget
                        .reserve::<(model::Definition, model::Function)>(1)?;
                    let call = model::Call {
                        target: self.function_target(reference.package, mapping.function)?,
                        arguments: mapping
                            .type_arguments
                            .iter()
                            .map(|ty| self.term(*ty, mapped, 0))
                            .collect::<Result<_>>()?,
                        implementations: mapping
                            .implementations
                            .iter()
                            .map(|operand| self.operand(operand, mapped, 0))
                            .collect::<Result<_>>()?,
                        kind: model::CallKind::UnusedMapping,
                        method_guard: None,
                        untaken: false,
                        phantom: true,
                    };
                    self.output.program.functions.insert(
                        synthetic,
                        model::Function {
                            parameters: implementation.type_parameters.len(),
                            witnesses,
                            calls: vec![call],
                        },
                    );
                }
            }
            DeclarationPayload::Function(function) => {
                let context = self.context(reference, TypeMode::Callable)?;
                let witnesses = self.witness_inventory(context)?;
                self.term(function.result, context, 0)?;
                self.budget
                    .reserve::<crate::platform::semantic_id::ParameterId>(
                        function.parameters.len(),
                    )?;
                let mut parameter_ids = BTreeSet::new();
                for parameter in &function.parameters {
                    if !parameter_ids.insert(*parameter) {
                        return Err(model::Refusal::Inventory);
                    }
                    match self
                        .source(reference.package)?
                        .owners
                        .get(&OwnerKey::Parameter(*parameter))
                        .map(|record| self.budget.canonical_clone(record))
                        .transpose()?
                    {
                        Some(OwnerRecord::Parameter(record))
                            if record.parent
                                == ParameterParent::Function(reference.declaration)
                                && record.header.kind == OwnerKind::Parameter =>
                        {
                            self.term(
                                record.ty,
                                Context {
                                    generation: record.header.contract_version,
                                    ..context
                                },
                                0,
                            )?;
                        }
                        _ => return Err(model::Refusal::Scope),
                    }
                }
                let mut calls = Vec::new();
                self.expressions(function.body, context, &mut calls)?;
                self.output.program.functions.insert(
                    context.target,
                    model::Function {
                        parameters: function.type_parameters.len(),
                        witnesses,
                        calls,
                    },
                );
            }
            DeclarationPayload::External(function) => {
                let context = self.context(reference, TypeMode::Callable)?;
                self.term(function.result, context, 0)?;
                for parameter in function.parameters {
                    match self
                        .source(reference.package)?
                        .owners
                        .get(&OwnerKey::Parameter(parameter))
                        .map(|record| self.budget.canonical_clone(record))
                        .transpose()?
                    {
                        Some(OwnerRecord::Parameter(record))
                            if record.parent
                                == ParameterParent::Function(reference.declaration) =>
                        {
                            self.term(record.ty, context, 0)?;
                        }
                        _ => return Err(model::Refusal::Scope),
                    }
                }
                self.output.program.functions.insert(
                    context.target,
                    model::Function {
                        parameters: function.type_parameters.len(),
                        witnesses: vec![],
                        calls: vec![],
                    },
                );
            }
            DeclarationPayload::Record { fields, .. }
            | DeclarationPayload::OwnedRecord { fields, .. } => {
                let context = self.context(reference, TypeMode::Nominal)?;
                for field in fields {
                    match self
                        .source(reference.package)?
                        .owners
                        .get(&OwnerKey::Field(field))
                        .map(|record| self.budget.canonical_clone(record))
                        .transpose()?
                    {
                        Some(OwnerRecord::Field(record))
                            if record.declaration == reference.declaration =>
                        {
                            self.term(record.ty, context, 0)?;
                        }
                        _ => return Err(model::Refusal::Scope),
                    }
                }
            }
            DeclarationPayload::Variant { cases, .. } => {
                let context = self.context(reference, TypeMode::Nominal)?;
                for case in cases {
                    match self
                        .source(reference.package)?
                        .owners
                        .get(&OwnerKey::Case(case))
                        .map(|record| self.budget.canonical_clone(record))
                        .transpose()?
                    {
                        Some(OwnerRecord::Case(record))
                            if record.declaration == reference.declaration =>
                        {
                            if let Some(ty) = record.payload {
                                self.term(ty, context, 0)?;
                            }
                        }
                        _ => return Err(model::Refusal::Scope),
                    }
                }
            }
            DeclarationPayload::Constant { ty, value } => {
                let context = self.context(reference, TypeMode::Closed)?;
                self.term(ty, context, 0)?;
                let mut calls = Vec::new();
                self.expressions(value, context, &mut calls)?;
                self.output.program.functions.insert(
                    context.target,
                    model::Function {
                        parameters: 0,
                        witnesses: vec![],
                        calls,
                    },
                );
            }
            DeclarationPayload::Test {
                actual, expected, ..
            } => {
                let context = self.context(reference, TypeMode::Closed)?;
                let mut calls = Vec::new();
                self.expressions(actual, context, &mut calls)?;
                self.expressions(expected, context, &mut calls)?;
                self.output.program.functions.insert(
                    context.target,
                    model::Function {
                        parameters: 0,
                        witnesses: vec![],
                        calls,
                    },
                );
            }
            DeclarationPayload::Interface { operations } => {
                let context = self.context(reference, TypeMode::Closed)?;
                for operation in operations {
                    match self
                        .source(reference.package)?
                        .owners
                        .get(&OwnerKey::Operation(operation))
                        .map(|record| self.budget.canonical_clone(record))
                        .transpose()?
                    {
                        Some(OwnerRecord::Operation(record))
                            if record.declaration == reference.declaration =>
                        {
                            self.term(record.result, context, 0)?;
                            for parameter in record.parameters {
                                match self
                                    .source(reference.package)?
                                    .owners
                                    .get(&OwnerKey::Parameter(parameter))
                                    .map(|record| self.budget.canonical_clone(record))
                                    .transpose()?
                                {
                                    Some(OwnerRecord::Parameter(record)) => {
                                        self.term(record.ty, context, 0)?;
                                    }
                                    _ => return Err(model::Refusal::Scope),
                                }
                            }
                        }
                        _ => return Err(model::Refusal::Scope),
                    }
                }
            }
            DeclarationPayload::Component { .. } => {}
        }
        Ok(())
    }

    fn method_calls(
        &mut self,
        witness: &ImplementationOperand,
        contract: DeclarationReference,
        method: MethodId,
        context: Context,
        calls: &mut Vec<model::Call>,
    ) -> Result<()> {
        let DeclarationPayload::OwnedContract(contract_record) =
            self.authored(context.source.package, contract)?.payload
        else {
            return Err(model::Refusal::ForeignContract);
        };
        if !contract_record
            .methods
            .iter()
            .any(|entry| entry.id == method)
        {
            return Err(model::Refusal::Inventory);
        }
        match self.operand(witness, context, 0)? {
            model::Operand::Concrete {
                scheme,
                arguments,
                prerequisites,
            } => {
                let Some(DefinitionIdentity::Declaration(implementation)) =
                    self.output.definitions.get(&scheme).cloned()
                else {
                    return Err(model::Refusal::MissingDefinition);
                };
                let DeclarationPayload::OwnedImplementation(record) =
                    self.declaration(implementation)?.payload
                else {
                    return Err(model::Refusal::MissingDefinition);
                };
                if record.contract != contract {
                    return Err(model::Refusal::ForeignContract);
                }
                self.budget.reserve::<model::Call>(1)?;
                calls.push(model::Call {
                    target: self
                        .ids
                        .get(&DefinitionIdentity::MethodMapping(implementation, method))
                        .copied()
                        .ok_or(model::Refusal::Inventory)?,
                    arguments,
                    implementations: prerequisites,
                    kind: model::CallKind::Function,
                    method_guard: None,
                    untaken: false,
                    phantom: false,
                });
            }
            model::Operand::Parameter { path, .. } => {
                let Some(raw) = self
                    .witnesses(context.source)?
                    .into_iter()
                    .find(|witness| self.witness_ids[&(context.source, witness.id)] == path.root)
                else {
                    return Err(model::Refusal::Scope);
                };
                if raw.contract != contract {
                    return Err(model::Refusal::ForeignContract);
                }
                // Exact method alternatives use raw scheme inventories, not a
                // producer's selected-shape relation or normalized method table.
                for implementation in self
                    .implementations_by_contract
                    .get(&contract)
                    .into_iter()
                    .flatten()
                {
                    self.budget.tick()?;
                    let DeclarationPayload::OwnedImplementation(record) =
                        self.declaration(*implementation)?.payload
                    else {
                        return Err(model::Refusal::Inventory);
                    };
                    if record.contract != contract {
                        return Err(model::Refusal::ForeignContract);
                    }
                    self.budget.reserve::<model::Call>(1)?;
                    self.budget
                        .reserve::<model::Term>(record.type_parameters.len())?;
                    self.budget
                        .reserve::<model::Operand>(record.implementation_parameters.len())?;
                    self.budget
                        .reserve::<usize>(record.implementation_parameters.len())?;
                    let path_bytes = record
                        .type_parameters
                        .len()
                        .checked_mul(path.prerequisites.len())
                        .ok_or(model::Refusal::Capacity("canonical method paths"))?;
                    self.budget.reserve::<usize>(path_bytes)?;
                    calls.push(model::Call {
                        target: self
                            .ids
                            .get(&DefinitionIdentity::MethodMapping(*implementation, method))
                            .copied()
                            .ok_or(model::Refusal::Inventory)?,
                        arguments: (0..record.type_parameters.len())
                            .map(|ordinal| model::Term::SelectedArgument {
                                scope: context.target,
                                path: path.clone(),
                                ordinal,
                            })
                            .collect(),
                        implementations: (0..record.implementation_parameters.len())
                            .map(|ordinal| model::Operand::Parameter {
                                scope: context.target,
                                path: model::WitnessPath {
                                    root: path.root,
                                    prerequisites: vec![ordinal],
                                },
                            })
                            .collect(),
                        kind: model::CallKind::SelectedMethodEntry,
                        method_guard: Some((path.root, self.id(*implementation)?)),
                        untaken: false,
                        phantom: false,
                    });
                }
            }
        }
        Ok(())
    }

    fn expressions(
        &mut self,
        root: ExpressionId,
        context: Context,
        calls: &mut Vec<model::Call>,
    ) -> Result<()> {
        self.budget.reserve::<(ExpressionId, bool)>(1)?;
        let mut pending = vec![(root, false)];
        let mut seen = BTreeSet::new();
        while let Some((id, untaken)) = pending.pop() {
            self.budget.tick()?;
            if seen.contains(&id) {
                continue;
            }
            self.budget.reserve::<ExpressionId>(1)?;
            if !seen.insert(id) {
                continue;
            }
            let Some(OwnerRecord::Expression(expression)) = self
                .source(context.source.package)?
                .owners
                .get(&OwnerKey::Expression(id))
                .map(|record| self.budget.canonical_clone(record))
                .transpose()?
            else {
                return Err(model::Refusal::MissingDefinition);
            };
            self.budget.canonical_reserve(&expression)?;
            let operation = expression.operation;
            let context = Context {
                generation: expression.contract_version,
                ..context
            };
            match &operation {
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    ..
                }
                | ExpressionOperation::FunctionValue {
                    function,
                    type_arguments,
                    ..
                } => {
                    self.budget.reserve::<model::Call>(1)?;
                    self.budget.reserve::<model::Term>(type_arguments.len())?;
                    calls.push(model::Call {
                        target: self.function_target(context.source.package, *function)?,
                        arguments: type_arguments
                            .iter()
                            .map(|ty| self.term(*ty, context, 0))
                            .collect::<Result<_>>()?,
                        implementations: vec![],
                        kind: if matches!(operation, ExpressionOperation::FunctionValue { .. }) {
                            model::CallKind::FunctionValue
                        } else {
                            model::CallKind::Function
                        },
                        method_guard: None,
                        untaken,
                        phantom: false,
                    });
                }
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    implementations,
                    ..
                } => {
                    self.budget
                        .reserve::<model::Operand>(implementations.len())?;
                    self.budget.reserve::<model::Call>(1)?;
                    self.budget.reserve::<model::Term>(type_arguments.len())?;
                    calls.push(model::Call {
                        target: self.function_target(context.source.package, *function)?,
                        arguments: type_arguments
                            .iter()
                            .map(|ty| self.term(*ty, context, 0))
                            .collect::<Result<_>>()?,
                        implementations: implementations
                            .iter()
                            .map(|operand| self.operand(operand, context, 0))
                            .collect::<Result<_>>()?,
                        kind: model::CallKind::Function,
                        method_guard: None,
                        untaken,
                        phantom: false,
                    });
                }
                ExpressionOperation::MethodCall {
                    witness,
                    contract,
                    method,
                    ..
                } => self.method_calls(witness, *contract, *method, context, calls)?,
                _ => {}
            }
            let mut children = Vec::new();
            let mut types = Vec::new();
            let mut bindings = Vec::new();
            // This independent exhaustive constructor table deliberately walks
            // both branches, all arms and all unused annotations/actuals.
            match operation {
                ExpressionOperation::If {
                    condition,
                    when_true,
                    when_false,
                } => {
                    children.extend([condition, when_true]);
                    self.budget.reserve::<(ExpressionId, bool)>(1)?;
                    pending.push((when_false, true));
                }
                ExpressionOperation::Let {
                    bindings: ids,
                    body,
                } => {
                    bindings.extend(ids);
                    children.push(body);
                }
                ExpressionOperation::Sequence { items } => children.extend(items),
                ExpressionOperation::Call { arguments, .. }
                | ExpressionOperation::ImplementationCall { arguments, .. }
                | ExpressionOperation::MethodCall { arguments, .. }
                | ExpressionOperation::CapabilityCall { arguments, .. } => {
                    children.extend(arguments)
                }
                ExpressionOperation::Invoke { callee, arguments }
                | ExpressionOperation::Bind { callee, arguments } => {
                    children.push(callee);
                    children.extend(arguments);
                }
                ExpressionOperation::Record {
                    type_arguments,
                    fields,
                    ..
                } => {
                    types.extend(type_arguments);
                    children.extend(fields.into_iter().map(|field| field.value));
                }
                ExpressionOperation::Variant {
                    type_arguments,
                    payload,
                    ..
                } => {
                    types.extend(type_arguments);
                    children.extend(payload);
                }
                ExpressionOperation::Field { value, .. } => children.push(value),
                ExpressionOperation::List { item_type, items } => {
                    types.push(item_type);
                    children.extend(items);
                }
                ExpressionOperation::Map {
                    key_type,
                    value_type,
                    entries,
                } => {
                    types.extend([key_type, value_type]);
                    for entry in entries {
                        children.extend([entry.key, entry.value]);
                    }
                }
                ExpressionOperation::Match { value, arms } => {
                    children.push(value);
                    for arm in arms {
                        children.push(arm.body);
                        bindings.extend(arm.payload_binding);
                    }
                }
                ExpressionOperation::Transaction { binding, body, .. } => {
                    bindings.push(binding);
                    children.push(body);
                }
                ExpressionOperation::TransactionOutcome {
                    binding,
                    body,
                    type_argument,
                    ..
                } => {
                    bindings.push(binding);
                    children.push(body);
                    types.push(type_argument);
                }
                ExpressionOperation::PackOwned {
                    product_type,
                    fields,
                } => {
                    types.push(product_type);
                    children.extend(fields.into_iter().map(|field| field.value));
                }
                ExpressionOperation::UnpackOwned {
                    product_type,
                    source,
                    fields,
                    body,
                } => {
                    types.push(product_type);
                    children.extend([source, body]);
                    bindings.extend(fields.into_iter().map(|field| field.binding));
                }
                ExpressionOperation::ChooseOwned {
                    choice_type, value, ..
                } => {
                    types.push(choice_type);
                    children.push(value);
                }
                ExpressionOperation::MatchOwned {
                    choice_type,
                    source,
                    arms,
                }
                | ExpressionOperation::MatchBorrowedOwned {
                    choice_type,
                    source,
                    arms,
                } => {
                    types.push(choice_type);
                    children.push(source);
                    for arm in arms {
                        children.push(arm.body);
                        bindings.push(arm.binding);
                    }
                }
                ExpressionOperation::Parallel { left, right } => children.extend([left, right]),
                ExpressionOperation::BorrowOwnedField {
                    product_type,
                    source,
                    binding,
                    body,
                    ..
                } => {
                    types.push(product_type);
                    children.extend([source, body]);
                    bindings.push(binding);
                }
                ExpressionOperation::SequenceEmpty { sequence_type } => types.push(sequence_type),
                ExpressionOperation::SequenceLength {
                    sequence_type,
                    source,
                } => {
                    types.push(sequence_type);
                    children.push(source);
                }
                ExpressionOperation::SequencePush {
                    sequence_type,
                    value,
                    source,
                } => {
                    types.push(sequence_type);
                    children.extend([value, source]);
                }
                ExpressionOperation::SequencePop {
                    sequence_type,
                    result_type,
                    source,
                } => {
                    types.extend([sequence_type, result_type]);
                    children.push(source);
                }
                ExpressionOperation::BorrowOwnedItem {
                    sequence_type,
                    source,
                    index,
                    binding,
                    body,
                } => {
                    types.push(sequence_type);
                    children.extend([source, index, body]);
                    bindings.push(binding);
                }
                ExpressionOperation::BorrowCall {
                    call,
                    binding,
                    body,
                } => {
                    children.extend([call, body]);
                    bindings.push(binding);
                }
                ExpressionOperation::SequenceGet {
                    sequence_type,
                    source,
                    index,
                } => {
                    types.push(sequence_type);
                    children.extend([source, index]);
                }
                ExpressionOperation::SequenceReplace {
                    sequence_type,
                    result_type,
                    index,
                    value,
                    source,
                } => {
                    types.extend([sequence_type, result_type]);
                    children.extend([index, value, source]);
                }
                ExpressionOperation::PackOwnedRecord {
                    record_type,
                    fields,
                } => {
                    types.push(record_type);
                    children.extend(fields.into_iter().map(|field| field.value));
                }
                ExpressionOperation::UnpackOwnedRecord {
                    record_type,
                    source,
                    fields,
                    body,
                } => {
                    types.push(record_type);
                    children.extend([source, body]);
                    bindings.extend(fields.into_iter().map(|field| field.binding));
                }
                ExpressionOperation::BorrowOwnedRecordField {
                    record_type,
                    source,
                    binding,
                    body,
                    ..
                } => {
                    types.push(record_type);
                    children.extend([source, body]);
                    bindings.push(binding);
                }
                ExpressionOperation::Unit {}
                | ExpressionOperation::Bool { .. }
                | ExpressionOperation::I64 { .. }
                | ExpressionOperation::F64 { .. }
                | ExpressionOperation::Text { .. }
                | ExpressionOperation::StaticText { .. }
                | ExpressionOperation::Local { .. }
                | ExpressionOperation::Constant { .. }
                | ExpressionOperation::FunctionValue { .. } => {}
            }
            for ty in types {
                self.term(ty, context, 0)?;
            }
            for binding in bindings {
                let Some(OwnerRecord::Binding(record)) = self
                    .source(context.source.package)?
                    .owners
                    .get(&OwnerKey::Binding(binding))
                    .map(|record| self.budget.canonical_clone(record))
                    .transpose()?
                else {
                    return Err(model::Refusal::MissingDefinition);
                };
                if let Some(ty) = record.declared_type {
                    self.term(ty, context, 0)?;
                }
                self.budget
                    .reserve::<ExpressionId>(usize::from(record.value.is_some()))?;
                children.extend(record.value);
            }
            self.budget
                .reserve::<(ExpressionId, bool)>(children.len())?;
            pending.extend(children.into_iter().map(|child| (child, untaken)));
        }
        Ok(())
    }
}
