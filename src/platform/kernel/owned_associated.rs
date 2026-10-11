//! Exact associated-member authority and bounded, occurrence-local resolution.
//!
//! These helpers do not certify method matching or prerequisite construction. The
//! complete callable/witness and finite-flow proofs remain separate obligations.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, TypeParameterId,
};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "owned_associated_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum AssociatedTypeContext {
    ContractSignature(DeclarationReference),
    Lexical(DeclarationReference),
    Forbidden,
}

impl AssociatedTypeContext {
    fn scope(self) -> Option<DeclarationReference> {
        match self {
            Self::ContractSignature(scope) | Self::Lexical(scope) => Some(scope),
            Self::Forbidden => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AssociatedAdmissionLimits {
    pub maximum_steps: usize,
    pub maximum_metadata_bytes: usize,
    pub maximum_depth: usize,
}

impl Default for AssociatedAdmissionLimits {
    fn default() -> Self {
        Self {
            maximum_steps: contract::MAXIMUM_VALIDATION_WORK,
            maximum_metadata_bytes: 64 * 1_048_576,
            maximum_depth: contract::MAXIMUM_TYPE_DEPTH,
        }
    }
}

/// A binding is a template plus simultaneous actual replacements, not an already
/// normalized type. Actuals may retain caller projections and are never revisited
/// under the implementation's lexical scope. No derived object is allocated here.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum AssociatedResolution {
    Binding {
        implementation: DeclarationReference,
        template: TypeObjectDigest,
        bindings: BTreeMap<TypeParameterId, TypeObjectDigest>,
    },
    Lexical(TypeForm),
}

pub(crate) struct OwnedAssociatedAdmission {
    limits: AssociatedAdmissionLimits,
    steps: Cell<usize>,
    metadata_bytes: Cell<usize>,
}

struct AssociatedRead<'a, R: ?Sized> {
    read: &'a R,
    admission: &'a OwnedAssociatedAdmission,
}

fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "kernel_owned_associated",
        message,
    )
}

fn exhausted(message: &'static str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "kernel_owned_associated_capacity",
        message,
    )
}

impl OwnedAssociatedAdmission {
    pub(crate) fn new() -> Self {
        Self::with_limits(AssociatedAdmissionLimits::default())
    }

    pub(crate) fn with_limits(limits: AssociatedAdmissionLimits) -> Self {
        Self {
            limits,
            steps: Cell::new(0),
            metadata_bytes: Cell::new(0),
        }
    }

    pub(crate) fn work(&self) -> (usize, usize) {
        (self.steps.get(), self.metadata_bytes.get())
    }

    fn reserve<T>(&self, count: usize) -> Result<(), Diagnostic> {
        let bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .ok_or_else(|| exhausted("associated type metadata size overflow"))?;
        self.reserve_bytes(bytes)
    }

    fn reserve_bytes(&self, bytes: usize) -> Result<(), Diagnostic> {
        let bytes = self
            .metadata_bytes
            .get()
            .checked_add(bytes)
            .filter(|bytes| *bytes <= self.limits.maximum_metadata_bytes)
            .ok_or_else(|| exhausted("associated type metadata exceeds its storage admission"))?;
        self.metadata_bytes.set(bytes);
        Ok(())
    }

    pub(crate) fn validate_form(
        &self,
        read: &(impl ExpressionRead + ?Sized),
        form: &TypeForm,
        context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<(), Diagnostic> {
        AssociatedRead {
            read,
            admission: self,
        }
        .validate_form(form, context, consumer_generation)
    }

    pub(crate) fn validate_occurrences(
        &self,
        read: &(impl ExpressionRead + ?Sized),
        roots: &[TypeObjectDigest],
        context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<(), Diagnostic> {
        AssociatedRead {
            read,
            admission: self,
        }
        .validate_occurrences(roots, context, consumer_generation)
    }

    pub(crate) fn validate_bindings(
        &self,
        read: &(impl ExpressionRead + ?Sized),
        scope: DeclarationReference,
        implementation: &OwnedImplementation,
    ) -> Result<(), Diagnostic> {
        AssociatedRead {
            read,
            admission: self,
        }
        .validate_bindings(scope, implementation)
    }

    pub(crate) fn resolve_form(
        &self,
        read: &(impl ExpressionRead + ?Sized),
        form: &TypeForm,
        source_context: AssociatedTypeContext,
        operand: &ImplementationOperand,
        caller_context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<AssociatedResolution, Diagnostic> {
        AssociatedRead {
            read,
            admission: self,
        }
        .resolve_form(
            form,
            source_context,
            operand,
            caller_context,
            consumer_generation,
        )
    }

    pub(crate) fn resolve_member(
        &self,
        read: &(impl ExpressionRead + ?Sized),
        reference: DeclarationReference,
        member: AssociatedTypeMemberId,
        operand: &ImplementationOperand,
        caller_context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<AssociatedResolution, Diagnostic> {
        AssociatedRead {
            read,
            admission: self,
        }
        .resolve_member(
            reference,
            member,
            operand,
            caller_context,
            consumer_generation,
        )
    }
}

impl<R: ExpressionRead + ?Sized> AssociatedRead<'_, R> {
    fn reserve<T>(&self, count: usize) -> Result<(), Diagnostic> {
        self.admission.reserve::<T>(count)
    }

    fn dependency(&self, package: PackageId) -> Result<(), Diagnostic> {
        self.validation_work()?;
        if package != self.package_id() && !self.has_dependency(package)? {
            return Err(reject(
                "associated authority is outside the declared package closure",
            ));
        }
        Ok(())
    }

    fn current_generation(&self, generation: u16) -> Result<(), Diagnostic> {
        if generation != contract::GRAPH_CONTRACT_VERSION {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_owned_associated_generation",
                "associated type use requires Graph 33 at its consuming owner",
            ));
        }
        Ok(())
    }

    fn declaration(
        &self,
        scope: DeclarationReference,
    ) -> Result<PackageInterfaceDeclaration, Diagnostic> {
        self.dependency(scope.package)?;
        let record = if scope.package == self.package_id() {
            match self.owner(OwnerKey::Declaration(scope.declaration))? {
                Some(OwnerRecord::Declaration(record)) => local_declaration(record)?,
                _ => return Err(reject("missing exact associated declaration scope")),
            }
        } else {
            match self
                .package_interface_owner(scope.package, OwnerKey::Declaration(scope.declaration))?
            {
                Some(PackageInterfaceRecord::Declaration(record)) => record,
                _ => {
                    return Err(reject(
                        "missing exact imported associated declaration scope",
                    ));
                }
            }
        };
        if record.header.owner != OwnerKey::Declaration(scope.declaration) {
            return Err(reject(
                "associated declaration scope has a foreign owner identity",
            ));
        }
        Ok(record)
    }

    /// Read member inventory only. Revalidating the contract's method signatures
    /// here would recursively revisit the very abstract member being admitted.
    fn member(
        &self,
        reference: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<(), Diagnostic> {
        let record = self.declaration(reference)?;
        self.current_generation(record.header.contract_version)?;
        if record.header.kind != OwnerKind::OwnedContract {
            return Err(reject(
                "associated member authority is not an OwnedContract",
            ));
        }
        let contract = owned_contract::contract_record(self, reference)?;
        if self.member_inventory(&contract)?.contains(&member) {
            Ok(())
        } else {
            Err(reject("member is absent from its exact contract inventory"))
        }
    }

    fn member_inventory(
        &self,
        contract: &OwnedContract,
    ) -> Result<BTreeSet<AssociatedTypeMemberId>, Diagnostic> {
        if contract.associated_types.len() > contract::MAXIMUM_CHILDREN {
            return Err(reject("associated member inventory exceeds its bound"));
        }
        self.reserve::<AssociatedTypeMemberId>(contract.associated_types.len())?;
        self.reserve::<&Name>(contract.associated_types.len())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for entry in &contract.associated_types {
            self.validation_work()?;
            if !ids.insert(entry.id) || !names.insert(&entry.name) {
                return Err(reject(
                    "associated members require distinct identities and names",
                ));
            }
            match entry.bound {
                AssociatedTypeBound::Owned => {}
            }
        }
        Ok(ids)
    }

    fn lexical_parameter(
        &self,
        scope: DeclarationReference,
        parameter: ImplementationParameterId,
        reference: DeclarationReference,
    ) -> Result<ImplementationParameter, Diagnostic> {
        let record = self.declaration(scope)?;
        self.current_generation(record.header.contract_version)?;
        let kind_matches = match &record.payload {
            PackageInterfaceDeclarationPayload::Function(function) => match function.effect {
                FunctionEffect::Pure => record.header.kind == OwnerKind::PureFunction,
                FunctionEffect::Task { .. } => record.header.kind == OwnerKind::TaskFunction,
            },
            PackageInterfaceDeclarationPayload::OwnedImplementation(_) => {
                record.header.kind == OwnerKind::OwnedImplementation
            }
            _ => false,
        };
        if !kind_matches {
            return Err(reject(
                "lexical projections require a function or OwnedImplementation scope",
            ));
        }
        let parameters = owned_contract::implementation_parameters_at(self, scope)?;
        if parameters.len() > contract::MAXIMUM_CHILDREN {
            return Err(exhausted(
                "associated lexical witness inventory exceeds its bound",
            ));
        }
        self.reserve::<ImplementationParameterId>(parameters.len())?;
        self.reserve::<Name>(parameters.len())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut found = None;
        for entry in parameters {
            self.validation_work()?;
            self.admission.reserve_bytes(entry.name.as_str().len())?;
            if !ids.insert(entry.id) || !names.insert(entry.name.clone()) {
                return Err(reject(
                    "lexical witness inventory contains duplicate identities or names",
                ));
            }
            if entry.id == parameter {
                found = Some(entry);
            }
        }
        let parameter =
            found.ok_or_else(|| reject("projection names no exact lexical witness parameter"))?;
        if parameter.contract != reference {
            return Err(reject(
                "projection contract differs from its lexical witness contract",
            ));
        }
        let contract = owned_contract::contract_record(self, reference)?;
        if parameter.type_arguments.len() != contract.type_parameters.len() {
            return Err(reject("lexical witness contract argument arity mismatch"));
        }
        let self_parameter = match self.type_object(parameter.self_type)? {
            Some(TypeObject {
                form: TypeForm::TypeParameter { parameter },
                ..
            }) => parameter,
            _ => {
                return Err(reject(
                    "lexical witness Self requires an exact in-scope Owned parameter",
                ));
            }
        };
        owned_contract::validate_owned_parameter(self, self_parameter, Some(scope))?;
        // Complete argument constraints and signatures belong to the separate
        // witness admission pass; inventory lookup must not recursively admit them.
        Ok(parameter)
    }

    pub(crate) fn validate_form(
        &self,
        form: &TypeForm,
        context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<(), Diagnostic> {
        self.validation_work()?;
        match form {
            TypeForm::AssociatedContractMember { contract, member } => {
                self.current_generation(consumer_generation)?;
                if context != AssociatedTypeContext::ContractSignature(*contract) {
                    return Err(reject(
                        "contract-local member is outside its exact method-signature context",
                    ));
                }
                self.member(*contract, *member)
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                self.current_generation(consumer_generation)?;
                if context != AssociatedTypeContext::Lexical(*scope) {
                    return Err(reject(
                        "lexical projection escaped its exact consuming scope",
                    ));
                }
                self.lexical_parameter(*scope, *parameter, *contract)?;
                self.member(*contract, *member)
            }
            _ => Ok(()),
        }
    }

    /// This is occurrence admission, not ordinary type/category admission. The
    /// same digest in different contexts is checked afresh on each invocation.
    pub(crate) fn validate_occurrences(
        &self,
        roots: &[TypeObjectDigest],
        context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<(), Diagnostic> {
        self.walk(roots, context, consumer_generation, false)
    }

    fn walk(
        &self,
        roots: &[TypeObjectDigest],
        context: AssociatedTypeContext,
        consumer_generation: u16,
        projection_free: bool,
    ) -> Result<(), Diagnostic> {
        type Pending = (
            TypeObjectDigest,
            usize,
            AssociatedTypeContext,
            Option<DeclarationReference>,
            u16,
        );
        self.reserve::<Pending>(roots.len())?;
        let mut pending = roots
            .iter()
            .map(|ty| (*ty, 0, context, context.scope(), consumer_generation))
            .collect::<Vec<_>>();
        let mut depths = BTreeMap::new();
        let mut expanded_nominals = BTreeSet::new();
        while let Some((ty, depth, context, parameter_scope, generation)) = pending.pop() {
            self.validation_work()?;
            if depth > self.admission.limits.maximum_depth {
                return Err(exhausted(
                    "associated type traversal exceeds its depth admission",
                ));
            }
            let key = (ty, context, parameter_scope, generation);
            if depths.get(&key).is_some_and(|previous| *previous >= depth) {
                continue;
            }
            if !depths.contains_key(&key) {
                self.reserve::<(
                    TypeObjectDigest,
                    AssociatedTypeContext,
                    Option<DeclarationReference>,
                    u16,
                    usize,
                )>(1)?;
            }
            depths.insert(key, depth);
            let object = self
                .type_object(ty)?
                .ok_or_else(|| reject("missing associated type closure object"))?;
            object.validate_local()?;
            if projection_free && object.form.is_associated() {
                return Err(reject(
                    "stored associated binding contains an associated type form",
                ));
            }
            self.validate_form(&object.form, context, generation)?;
            if projection_free && let TypeForm::TypeParameter { parameter } = object.form {
                let scope = parameter_scope
                    .ok_or_else(|| reject("binding parameter has no exact scheme scope"))?;
                let property_only = context == AssociatedTypeContext::Forbidden;
                let declaration = if property_only {
                    self.representation_declaration(scope)?
                } else {
                    self.declaration(scope)?
                };
                let record = if property_only {
                    owned_record::parameter(self, scope, parameter)?
                } else {
                    owned_contract::type_parameter_at(self, scope, parameter)?
                };
                let listed = match &declaration.payload {
                    PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                        i.type_parameters.contains(&parameter)
                    }
                    PackageInterfaceDeclarationPayload::OwnedRecord {
                        type_parameters, ..
                    }
                    | PackageInterfaceDeclarationPayload::Record {
                        type_parameters, ..
                    }
                    | PackageInterfaceDeclarationPayload::Variant {
                        type_parameters, ..
                    } => type_parameters.contains(&parameter),
                    _ => false,
                };
                if !listed
                    || !(record.constraints.has_owned() || record.constraints.requires_transfer())
                {
                    return Err(reject(
                        "binding parameter is outside its exact admitted scheme inventory",
                    ));
                }
            }
            self.reserve::<Pending>(object.child_type_count())?;
            for child in object.child_types() {
                pending.push((child, depth + 1, context, parameter_scope, generation));
            }
            if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } =
                object.form
            {
                // Representation absence is a declaration property, not applied
                // layout expansion. Scan each raw definition once, including
                // recursive ordinary definitions; actual syntax still revisits
                // shared DAG nodes at greater occurrence depths above.
                if !expanded_nominals.contains(&declaration) {
                    self.reserve::<DeclarationReference>(1)?;
                    expanded_nominals.insert(declaration);
                    self.nominal_children(declaration, depth + 1, &mut pending)?;
                }
            }
        }
        Ok(())
    }

    /// Raw representation roots are read only for the absence/property proof.
    /// They cannot import lexical witness authority or grant representation access.
    fn nominal_children(
        &self,
        reference: DeclarationReference,
        depth: usize,
        pending: &mut Vec<(
            TypeObjectDigest,
            usize,
            AssociatedTypeContext,
            Option<DeclarationReference>,
            u16,
        )>,
    ) -> Result<(), Diagnostic> {
        self.validation_work()?;
        let payload = self.representation_declaration(reference)?.payload;
        let (fields, cases) = match payload {
            PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. }
            | PackageInterfaceDeclarationPayload::Record { fields, .. } => (fields, Vec::new()),
            PackageInterfaceDeclarationPayload::Variant { cases, .. } => (Vec::new(), cases),
            _ => {
                return Err(reject(
                    "associated type closure nominal reference is not a data declaration",
                ));
            }
        };
        let count = fields
            .len()
            .checked_add(cases.len())
            .filter(|count| *count <= contract::MAXIMUM_CHILDREN)
            .ok_or_else(|| reject("binding nominal inventory exceeds its bound"))?;
        self.reserve::<(
            TypeObjectDigest,
            usize,
            AssociatedTypeContext,
            Option<DeclarationReference>,
            u16,
        )>(count)?;
        for id in fields {
            let field = owned_record::field(
                self,
                FieldReference {
                    package: reference.package,
                    field: id,
                },
            )?;
            if field.header.owner != OwnerKey::Field(id)
                || field.declaration != reference.declaration
            {
                return Err(reject(
                    "binding nominal field belongs to a foreign declaration",
                ));
            }
            pending.push((
                field.ty,
                depth,
                AssociatedTypeContext::Forbidden,
                Some(reference),
                field.header.contract_version,
            ));
        }
        for id in cases {
            self.validation_work()?;
            let case = if reference.package == self.package_id() {
                match self.owner(OwnerKey::Case(id))? {
                    Some(OwnerRecord::Case(case)) => case,
                    _ => return Err(reject("missing binding nominal case")),
                }
            } else {
                match self.package_representation_owner(reference.package, OwnerKey::Case(id))? {
                    Some(PackageInterfaceRecord::Case(case)) => case,
                    _ => return Err(reject("missing imported binding nominal case")),
                }
            };
            if case.header.owner != OwnerKey::Case(id) || case.declaration != reference.declaration
            {
                return Err(reject(
                    "binding nominal case belongs to a foreign declaration",
                ));
            }
            if let Some(ty) = case.payload {
                pending.push((
                    ty,
                    depth,
                    AssociatedTypeContext::Forbidden,
                    Some(reference),
                    case.header.contract_version,
                ));
            }
        }
        Ok(())
    }

    fn representation_declaration(
        &self,
        reference: DeclarationReference,
    ) -> Result<PackageInterfaceDeclaration, Diagnostic> {
        self.validation_work()?;
        let record = if reference.package == self.package_id() {
            match self.owner(OwnerKey::Declaration(reference.declaration))? {
                Some(OwnerRecord::Declaration(record))
                    if record.header.owner == OwnerKey::Declaration(reference.declaration) =>
                {
                    local_declaration(record)?
                }
                _ => return Err(reject("missing exact binding nominal declaration")),
            }
        } else {
            if !self.has_representation_dependency(reference.package)? {
                return Err(reject(
                    "binding nominal type is outside the exact representation closure",
                ));
            }
            match self.package_representation_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
            )? {
                Some(
                    PackageInterfaceRecord::Declaration(record)
                    | PackageInterfaceRecord::RepresentationDeclaration(record),
                ) if record.header.owner == OwnerKey::Declaration(reference.declaration) => record,
                _ => return Err(reject("missing exact imported binding nominal declaration")),
            }
        };
        let kind = match record.payload {
            PackageInterfaceDeclarationPayload::OwnedRecord { .. } => OwnerKind::OwnedRecord,
            PackageInterfaceDeclarationPayload::Record { .. } => OwnerKind::Record,
            PackageInterfaceDeclarationPayload::Variant { .. } => OwnerKind::Variant,
            _ => {
                return Err(reject(
                    "associated representation property requires a data declaration",
                ));
            }
        };
        if record.header.kind != kind {
            return Err(reject(
                "associated representation header disagrees with its data kind",
            ));
        }
        Ok(record)
    }

    pub(crate) fn validate_bindings(
        &self,
        scope: DeclarationReference,
        implementation: &OwnedImplementation,
    ) -> Result<(), Diagnostic> {
        self.validation_work()?;
        let record = self.declaration(scope)?;
        if record.header.kind != OwnerKind::OwnedImplementation {
            return Err(reject("binding scope is not an OwnedImplementation"));
        }
        let canonical = owned_contract::implementation_record(self, scope)?;
        if &canonical != implementation {
            return Err(reject(
                "binding inventory differs from its exact implementation owner",
            ));
        }
        implementation.validate_local()?;
        let contract = owned_contract::contract_record(self, implementation.contract)?;
        if !contract.associated_types.is_empty() || !implementation.associated_types.is_empty() {
            self.current_generation(record.header.contract_version)?;
        }
        if contract.associated_types.len() != implementation.associated_types.len() {
            return Err(reject("associated binding inventory is not complete"));
        }
        let members = self.member_inventory(&contract)?;
        for binding in &implementation.associated_types {
            self.validation_work()?;
            if !members.contains(&binding.member) {
                return Err(reject("associated binding names a foreign contract member"));
            }
            self.member(implementation.contract, binding.member)?;
            self.walk(
                &[binding.ty],
                AssociatedTypeContext::Lexical(scope),
                record.header.contract_version,
                true,
            )?;
            owned_contract::validate_owned_type(self, binding.ty, Some(scope))?;
        }
        Ok(())
    }

    pub(crate) fn resolve_form(
        &self,
        form: &TypeForm,
        source_context: AssociatedTypeContext,
        operand: &ImplementationOperand,
        caller_context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<AssociatedResolution, Diagnostic> {
        self.validate_form(form, source_context, consumer_generation)?;
        let (contract, member) = match form {
            TypeForm::AssociatedContractMember { contract, member }
            | TypeForm::AssociatedWitnessProjection {
                contract, member, ..
            } => (*contract, *member),
            _ => return Err(reject("member resolution requires an associated type form")),
        };
        self.resolve_member(
            contract,
            member,
            operand,
            caller_context,
            consumer_generation,
        )
    }

    pub(crate) fn resolve_member(
        &self,
        reference: DeclarationReference,
        member: AssociatedTypeMemberId,
        operand: &ImplementationOperand,
        caller_context: AssociatedTypeContext,
        consumer_generation: u16,
    ) -> Result<AssociatedResolution, Diagnostic> {
        self.validation_work()?;
        self.current_generation(consumer_generation)?;
        self.member(reference, member)?;
        operand.validate_local()?;
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                if caller_context != AssociatedTypeContext::Lexical(*scope) {
                    return Err(reject(
                        "selected witness parameter escaped its exact caller scope",
                    ));
                }
                self.lexical_parameter(*scope, *parameter, reference)?;
                Ok(AssociatedResolution::Lexical(
                    TypeForm::AssociatedWitnessProjection {
                        scope: *scope,
                        parameter: *parameter,
                        contract: reference,
                        member,
                    },
                ))
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let value = owned_contract::implementation_record(self, *implementation)?;
                if value.contract != reference {
                    return Err(reject(
                        "selected implementation names a different exact contract",
                    ));
                }
                if value.type_parameters.len() != type_arguments.len()
                    || value.implementation_parameters.len() != implementations.len()
                {
                    return Err(reject("selected implementation application arity mismatch"));
                }
                self.validate_bindings(*implementation, &value)?;
                self.validate_occurrences(type_arguments, caller_context, consumer_generation)?;
                self.reserve::<(TypeParameterId, TypeObjectDigest)>(type_arguments.len())?;
                let mut bindings = BTreeMap::new();
                for (parameter, ty) in value.type_parameters.iter().zip(type_arguments) {
                    let formal =
                        owned_contract::type_parameter_at(self, *implementation, *parameter)?;
                    owned_contract::validate_type_argument_constraints(
                        self,
                        formal.constraints,
                        *ty,
                        caller_context.scope(),
                    )?;
                    bindings.insert(*parameter, *ty);
                }
                let binding = value
                    .associated_types
                    .iter()
                    .find(|binding| binding.member == member)
                    .ok_or_else(|| reject("selected implementation has no exact member binding"))?;
                // Complete prerequisite applications remain checked by ordinary
                // witness admission. Their identities are not inferred from types.
                Ok(AssociatedResolution::Binding {
                    implementation: *implementation,
                    template: binding.ty,
                    bindings,
                })
            }
        }
    }
}

/// A private property reader does not apply public interface projection's
/// visibility policy to local canonical declarations.
fn local_declaration(record: DeclarationRecord) -> Result<PackageInterfaceDeclaration, Diagnostic> {
    let payload = match record.payload {
        DeclarationPayload::OwnedContract(value) => {
            PackageInterfaceDeclarationPayload::OwnedContract(value)
        }
        DeclarationPayload::OwnedImplementation(value) => {
            PackageInterfaceDeclarationPayload::OwnedImplementation(value)
        }
        DeclarationPayload::Function(value) => {
            PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                implementation_parameters: value.implementation_parameters,
                requirement_parameters: value.requirement_parameters,
                effect_parameters: value.effect_parameters,
                type_parameters: value.type_parameters,
                parameters: value.parameters,
                result: value.result,
                result_borrow: value.result_borrow,
                effect: value.effect,
            })
        }
        DeclarationPayload::External(value) => {
            PackageInterfaceDeclarationPayload::External(PackageExternalSignature {
                type_parameters: value.type_parameters,
                parameters: value.parameters,
                result: value.result,
            })
        }
        DeclarationPayload::Record {
            type_parameters,
            fields,
        } => PackageInterfaceDeclarationPayload::Record {
            type_parameters,
            fields,
        },
        DeclarationPayload::OwnedRecord {
            type_parameters,
            fields,
        } => PackageInterfaceDeclarationPayload::OwnedRecord {
            type_parameters,
            fields,
        },
        DeclarationPayload::Variant {
            type_parameters,
            cases,
        } => PackageInterfaceDeclarationPayload::Variant {
            type_parameters,
            cases,
        },
        _ => {
            return Err(reject(
                "associated type authority requires an exact contract, lexical scope or data declaration",
            ));
        }
    };
    Ok(PackageInterfaceDeclaration {
        header: record.header,
        name: record.name,
        payload,
    })
}

impl<R: ExpressionRead + ?Sized> ExpressionRead for AssociatedRead<'_, R> {
    fn package_id(&self) -> PackageId {
        self.read.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.read.owner(owner)
    }
    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.read.type_object(digest)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_interface_owner(package, owner)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_dependency(package)
    }
    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_representation_owner(package, owner)
    }
    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_representation_dependency(package)
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.read.validation_work()?;
        let steps = self
            .admission
            .steps
            .get()
            .checked_add(1)
            .filter(|steps| *steps <= self.admission.limits.maximum_steps)
            .ok_or_else(|| exhausted("associated type proof exceeds its work admission"))?;
        self.admission.steps.set(steps);
        Ok(())
    }
}
