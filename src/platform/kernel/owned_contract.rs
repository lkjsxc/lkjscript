//! First-order nominal owned-method contracts and explicit static implementation operands.
//! These records are canonical meaning, independent of capability and proof witnesses.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::DeclarationId;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, MethodId, TypeParameterId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::owned_associated::{OwnedAssociatedAdmission, OwnedTypeOccurrence};

#[path = "owned_contract/associated_application.rs"]
mod associated_application;
pub(crate) use associated_application::{
    AdmittedFunctionApplication, AdmittedMethodApplication, FunctionArguments, ScopeApplication,
    admit_function_application, admit_function_application_with_scopes, admit_method_application,
    admit_method_application_with_scopes,
};

#[cfg(test)]
#[path = "owned_prerequisite_tests.rs"]
mod prerequisite_tests;

#[cfg(test)]
#[path = "mixed_contract_tests.rs"]
mod mixed_tests;

fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_owned_contract", message)
}

fn reserve_scope_inventory<S>(
    parameters: &[ReadScopeParameter],
    sources: &[ReadScopeSource<S>],
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    associated.reserve::<crate::platform::semantic_id::ReadScopeParameterId>(parameters.len())?;
    associated.reserve::<&Name>(parameters.len())?;
    associated.reserve::<(S, ReadScopeArgument)>(sources.len())
}
fn admit_dependency(
    read: &(impl ExpressionRead + ?Sized),
    package: PackageId,
) -> Result<(), Diagnostic> {
    if package != read.package_id() && !read.has_dependency(package)? {
        return Err(reject(
            "owned reference is outside the declared package closure",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedContract {
    pub self_parameter: TypeParameterId,
    pub type_parameters: Vec<TypeParameterId>,
    pub methods: Vec<OwnedMethod>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub associated_types: Vec<AssociatedTypeMember>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedMethod {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<OwnedMethodParameter>,
    pub result: CallableResult<u32>,
    pub read_scope_parameters: Vec<ReadScopeParameter>,
    pub read_scope_sources: Vec<ReadScopeSource<u32>>,
    pub effect: FunctionEffect,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedMethodParameter {
    pub ty: TypeObjectDigest,
    pub use_mode: ParameterUse,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedImplementation {
    pub type_parameters: Vec<TypeParameterId>,
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub methods: Vec<OwnedMethodImplementation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub associated_types: Vec<AssociatedTypeBinding>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssociatedTypeMember {
    pub id: AssociatedTypeMemberId,
    pub name: Name,
    pub bound: AssociatedTypeBound,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssociatedTypeBound {
    Owned,
    ReadViewFamily { scope_parameter: ReadScopeParameter },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssociatedTypeBinding {
    pub member: AssociatedTypeMemberId,
    pub value: AssociatedTypeBindingValue,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssociatedTypeBindingValue {
    Owned {
        ty: TypeObjectDigest,
    },
    ReadViewFamily {
        scope_parameter: ReadScopeParameter,
        ty: TypeObjectDigest,
    },
}

impl AssociatedTypeBindingValue {
    pub fn ty(&self) -> TypeObjectDigest {
        match self {
            Self::Owned { ty } | Self::ReadViewFamily { ty, .. } => *ty,
        }
    }
    pub fn ty_mut(&mut self) -> &mut TypeObjectDigest {
        match self {
            Self::Owned { ty } | Self::ReadViewFamily { ty, .. } => ty,
        }
    }
}

impl AssociatedTypeBinding {
    pub fn ty(&self) -> TypeObjectDigest {
        self.value.ty()
    }
}

impl AssociatedTypeBound {
    pub fn scope_arity(&self) -> usize {
        match self {
            Self::Owned => 0,
            Self::ReadViewFamily { .. } => 1,
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedMethodImplementation {
    pub method: MethodId,
    pub function: DeclarationReference,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub implementations: Vec<ImplementationOperand>,
    pub read_scope_arguments: Vec<ReadScopeArgument>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImplementationParameter {
    pub id: ImplementationParameterId,
    pub name: Name,
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub type_arguments: Vec<TypeObjectDigest>,
}

#[derive(Clone, Debug, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImplementationOperand {
    Concrete {
        implementation: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
        implementations: Vec<ImplementationOperand>,
    },
    Parameter {
        scope: DeclarationReference,
        parameter: ImplementationParameterId,
    },
}

impl OwnedContract {
    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        let mut associated_ids = BTreeSet::new();
        let mut associated_names = BTreeSet::new();
        if self.associated_types.len() > contract::MAXIMUM_CHILDREN
            || self.associated_types.iter().any(|member| {
                !associated_ids.insert(member.id) || !associated_names.insert(&member.name)
            })
        {
            return Err(reject(
                "associated members require bounded distinct identities and names",
            ));
        }
        if self.type_parameters.len() >= contract::MAXIMUM_CHILDREN
            || self.type_parameters.contains(&self.self_parameter)
            || self
                .type_parameters
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != self.type_parameters.len()
        {
            return Err(reject(
                "owned contract requires bounded distinct type parameters excluding Self",
            ));
        }
        if self.methods.is_empty() || self.methods.len() > contract::MAXIMUM_CHILDREN {
            return Err(reject(
                "owned contract requires a bounded nonempty method inventory",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for method in &self.methods {
            if !ids.insert(method.id)
                || !names.insert(&method.name)
                || method.parameters.len() > contract::MAXIMUM_CHILDREN
            {
                return Err(reject(
                    "owned methods require distinct identities/names and bounded signatures",
                ));
            }
            let row = method.effect.row();
            row.validate()?;
            if !row.is_closed() {
                return Err(reject("owned methods require closed exact effect rows"));
            }
            validate_read_scope_inventory(
                &method.read_scope_parameters,
                &method.read_scope_sources,
                |position| (position as usize) < method.parameters.len(),
            )?;
            if let Some(position) = method.result.relation.source() {
                let source = method.parameters.get(*position as usize).ok_or_else(|| {
                    reject("borrowed method result source is outside its parameter inventory")
                })?;
                if !matches!(method.effect, FunctionEffect::Pure)
                    || source.use_mode != ParameterUse::Borrow
                {
                    return Err(reject(
                        "borrowed method results require a pure method and borrowed source",
                    ));
                }
            }
            if let ResultRelation::ViewFrom { scope, .. } = method.result.relation {
                validate_formal_scope(scope, &method.read_scope_parameters, None)?;
            }
        }
        Ok(())
    }
    pub(crate) fn type_roots(&self) -> Vec<TypeObjectDigest> {
        self.methods
            .iter()
            .flat_map(|m| m.parameters.iter().map(|p| p.ty).chain([m.result.ty]))
            .collect()
    }
}

impl OwnedImplementation {
    pub(crate) fn requires_prerequisite_generation(&self) -> bool {
        !self.implementation_parameters.is_empty()
            || self
                .methods
                .iter()
                .any(|method| !method.implementations.is_empty())
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.associated_types.len() > contract::MAXIMUM_CHILDREN
            || self
                .associated_types
                .windows(2)
                .any(|pair| pair[0].member >= pair[1].member)
        {
            return Err(reject(
                "associated bindings require a bounded strictly ordered inventory",
            ));
        }
        if self.type_parameters.len() >= contract::MAXIMUM_CHILDREN
            || self
                .type_parameters
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != self.type_parameters.len()
            || self.type_arguments.len() >= contract::MAXIMUM_CHILDREN
            || self.implementation_parameters.len() > contract::MAXIMUM_CHILDREN
            || self.methods.is_empty()
            || self.methods.len() > contract::MAXIMUM_CHILDREN
            || self.methods.windows(2).any(|p| p[0].method >= p[1].method)
            || self
                .methods
                .iter()
                .any(|m| m.type_arguments.len() > contract::MAXIMUM_CHILDREN)
        {
            return Err(reject(
                "implementation method mapping must be complete, unique and canonically ordered",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for parameter in &self.implementation_parameters {
            if !ids.insert(parameter.id)
                || !names.insert(&parameter.name)
                || parameter.type_arguments.len() > contract::MAXIMUM_CHILDREN
            {
                return Err(reject(
                    "implementation prerequisites require distinct identities/names and bounded applications",
                ));
            }
        }
        for method in &self.methods {
            if method.implementations.len() > contract::MAXIMUM_CHILDREN
                || method.read_scope_arguments.len() > contract::MAXIMUM_CHILDREN
            {
                return Err(reject("mapped prerequisite inventory exceeds its bound"));
            }
            for operand in &method.implementations {
                operand.validate_local()?;
            }
        }
        Ok(())
    }

    pub(crate) fn type_roots(&self) -> Vec<TypeObjectDigest> {
        std::iter::once(self.self_type)
            .chain(self.associated_types.iter().map(AssociatedTypeBinding::ty))
            .chain(self.type_arguments.iter().copied())
            .chain(
                self.implementation_parameters.iter().flat_map(|p| {
                    std::iter::once(p.self_type).chain(p.type_arguments.iter().copied())
                }),
            )
            .chain(self.methods.iter().flat_map(|m| {
                m.type_arguments
                    .iter()
                    .copied()
                    .chain(m.implementations.iter().flat_map(|operand| {
                        operand
                            .walk()
                            .flat_map(|operand| operand.type_arguments().iter().copied())
                    }))
            }))
            .collect()
    }
}

impl ImplementationOperand {
    pub(crate) fn type_arguments(&self) -> &[TypeObjectDigest] {
        match self {
            Self::Concrete { type_arguments, .. } => type_arguments,
            Self::Parameter { .. } => &[],
        }
    }

    /// Borrowed preorder traversal never recurses through a producer's witness tree.
    pub(crate) fn walk(&self) -> impl Iterator<Item = &Self> {
        // Keep one iterator per ancestor instead of retaining every sibling.
        // Callers admit the complete shape before semantic use.
        let mut pending = Vec::with_capacity(contract::MAXIMUM_TYPE_DEPTH + 1);
        let mut next = Some(self);
        std::iter::from_fn(move || {
            let operand = loop {
                if let Some(operand) = next.take() {
                    break operand;
                }
                let iter: &mut std::slice::Iter<'_, Self> = pending.last_mut()?;
                if let Some(operand) = iter.next() {
                    break operand;
                }
                pending.pop();
            };
            if let Self::Concrete {
                implementations, ..
            } = operand
                && !implementations.is_empty()
            {
                pending.push(implementations.iter());
            }
            Some(operand)
        })
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        let mut pending = vec![(self, 0usize)];
        let mut nodes = 0usize;
        while let Some((operand, depth)) = pending.pop() {
            nodes += 1;
            if depth > contract::MAXIMUM_TYPE_DEPTH || nodes > contract::MAXIMUM_CHILDREN {
                return Err(reject(
                    "implementation application exceeds its depth or node bound",
                ));
            }
            if let Self::Concrete {
                type_arguments,
                implementations,
                ..
            } = operand
            {
                if type_arguments.len() > contract::MAXIMUM_CHILDREN
                    || implementations.len() > contract::MAXIMUM_CHILDREN
                    || nodes + pending.len() + implementations.len() > contract::MAXIMUM_CHILDREN
                {
                    return Err(reject(
                        "implementation application exceeds its ordered argument bounds",
                    ));
                }
                pending.try_reserve(implementations.len()).map_err(|_| {
                    Diagnostic::new(
                        DiagnosticClass::Resource,
                        "kernel_witness_application_storage",
                        "implementation shape admission could not reserve traversal storage",
                    )
                })?;
                pending.extend(
                    implementations
                        .iter()
                        .rev()
                        .map(|operand| (operand, depth + 1)),
                );
            }
        }
        Ok(())
    }
}

// Derived Decode would recurse without a bound before semantic admission. Retain the
// ordinary enum/Vec framing, but reject excessive depth, breadth and total nodes first.
impl<Context> Decode<Context> for ImplementationOperand {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        fn vector<Context, D: bincode::de::Decoder<Context = Context>, T: Decode<Context>>(
            decoder: &mut D,
        ) -> Result<Vec<T>, bincode::error::DecodeError> {
            let length = usize::decode(decoder)?;
            if length > contract::MAXIMUM_CHILDREN {
                return Err(bincode::error::DecodeError::Other(
                    "implementation argument count exceeds its bound",
                ));
            }
            decoder.claim_container_read::<T>(length)?;
            let mut values = Vec::with_capacity(length);
            for _ in 0..length {
                decoder.unclaim_bytes_read(std::mem::size_of::<T>());
                values.push(T::decode(decoder)?);
            }
            Ok(values)
        }
        fn operand<Context, D: bincode::de::Decoder<Context = Context>>(
            decoder: &mut D,
            depth: usize,
            remaining: &mut usize,
        ) -> Result<ImplementationOperand, bincode::error::DecodeError> {
            if depth > contract::MAXIMUM_TYPE_DEPTH {
                return Err(bincode::error::DecodeError::Other(
                    "implementation application exceeds its depth or node bound",
                ));
            }
            match u32::decode(decoder)? {
                0 => {
                    let implementation = DeclarationReference::decode(decoder)?;
                    let type_arguments = vector(decoder)?;
                    let length = usize::decode(decoder)?;
                    if length > contract::MAXIMUM_CHILDREN || length > *remaining {
                        return Err(bincode::error::DecodeError::Other(
                            "implementation prerequisite count exceeds its bound",
                        ));
                    }
                    *remaining -= length;
                    decoder.claim_container_read::<ImplementationOperand>(length)?;
                    let mut implementations = Vec::with_capacity(length);
                    for _ in 0..length {
                        decoder.unclaim_bytes_read(std::mem::size_of::<ImplementationOperand>());
                        implementations.push(operand(decoder, depth + 1, remaining)?);
                    }
                    Ok(ImplementationOperand::Concrete {
                        implementation,
                        type_arguments,
                        implementations,
                    })
                }
                1 => Ok(ImplementationOperand::Parameter {
                    scope: DeclarationReference::decode(decoder)?,
                    parameter: ImplementationParameterId::decode(decoder)?,
                }),
                _ => Err(bincode::error::DecodeError::Other(
                    "unknown implementation operand tag",
                )),
            }
        }
        let mut remaining = contract::MAXIMUM_CHILDREN - 1;
        operand(decoder, 0, &mut remaining)
    }
}

bincode::impl_borrow_decode!(ImplementationOperand);

/// Disposable application types remain available throughout subsequent validation.
pub(crate) struct AppliedTypeRead<'a, R: ?Sized> {
    pub(crate) read: &'a R,
    pub(crate) types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
}

impl<R: ExpressionRead + ?Sized> ExpressionRead for AppliedTypeRead<'_, R> {
    fn package_id(&self) -> PackageId {
        self.read.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.read.owner(owner)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self
            .read
            .type_object(ty)?
            .or_else(|| self.types.get(&ty).cloned()))
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
        self.read.validation_work()
    }
}

pub(crate) struct AppliedWitnessContract {
    pub(crate) contract: DeclarationReference,
    pub(crate) self_type: TypeObjectDigest,
    pub(crate) type_arguments: Vec<TypeObjectDigest>,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
}

pub(crate) struct AppliedOwnedMethod {
    pub(crate) method: OwnedMethod,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
}

fn requires_task_borrow_generation(
    read: &(impl ExpressionRead + ?Sized),
    contract: &OwnedContract,
) -> Result<bool, Diagnostic> {
    for method in &contract.methods {
        read.validation_work()?;
        if matches!(method.effect, FunctionEffect::Task { .. }) {
            for parameter in &method.parameters {
                read.validation_work()?;
                if parameter.use_mode == ParameterUse::Borrow {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn require_share_generation_at(
    read: &(impl ExpressionRead + ?Sized),
    scope: DeclarationReference,
    subject: &str,
) -> Result<(), Diagnostic> {
    require_generation_at(read, scope, contract::SHARE_GRAPH_CONTRACT_VERSION, subject)
}

fn require_generation_at(
    read: &(impl ExpressionRead + ?Sized),
    scope: DeclarationReference,
    generation: u16,
    subject: &str,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    admit_dependency(read, scope.package)?;
    let key = OwnerKey::Declaration(scope.declaration);
    let header = if scope.package == read.package_id() {
        match read.owner(key)? {
            Some(OwnerRecord::Declaration(declaration)) => declaration.header,
            _ => return Err(reject("missing exact defining declaration generation")),
        }
    } else {
        match read.package_interface_owner(scope.package, key)? {
            Some(PackageInterfaceRecord::Declaration(declaration)) => declaration.header,
            _ => {
                return Err(reject(
                    "missing exact imported defining declaration generation",
                ));
            }
        }
    };
    if header.owner != key || header.contract_version < generation {
        return Err(reject(format!(
            "{subject} requires exact Graph {generation} authority"
        )));
    }
    Ok(())
}

fn defining_occurrence(
    read: &(impl ExpressionRead + ?Sized),
    scope: Option<DeclarationReference>,
) -> Result<OwnedTypeOccurrence, Diagnostic> {
    let Some(scope) = scope else {
        // A closed legacy operation grants no associated scope. Callers admitting
        // authored occurrences must always pass their actual owner generation.
        return Ok(OwnedTypeOccurrence::authored(
            None,
            contract::GRAPH_CONTRACT_VERSION,
        ));
    };
    read.validation_work()?;
    admit_dependency(read, scope.package)?;
    let key = OwnerKey::Declaration(scope.declaration);
    let header = if scope.package == read.package_id() {
        match read.owner(key)? {
            Some(OwnerRecord::Declaration(record)) => record.header,
            _ => return Err(reject("missing exact occurrence declaration")),
        }
    } else {
        match read.package_interface_owner(scope.package, key)? {
            Some(PackageInterfaceRecord::Declaration(record)) => record.header,
            _ => return Err(reject("missing exact imported occurrence declaration")),
        }
    };
    if header.owner != key {
        return Err(reject(
            "occurrence declaration has a foreign owner identity",
        ));
    }
    Ok(OwnedTypeOccurrence::authored(
        Some(scope),
        header.contract_version,
    ))
}

pub(crate) fn contract_record(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
) -> Result<OwnedContract, Diagnostic> {
    read.validation_work()?;
    admit_dependency(read, reference.package)?;
    if reference.package == read.package_id() {
        if let Some(OwnerRecord::Declaration(d)) =
            read.owner(OwnerKey::Declaration(reference.declaration))?
            && let DeclarationPayload::OwnedContract(c) = d.payload
        {
            if d.header.contract_version < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION
                && requires_mixed_generation(read, reference, &c.type_parameters)?
            {
                return Err(reject("ordinary contract parameters require Graph 33"));
            }
            if d.header.contract_version < contract::SHARE_GRAPH_CONTRACT_VERSION
                && requires_task_borrow_generation(read, &c)?
            {
                return Err(reject("borrowed task methods require Graph 30"));
            }
            if d.header.contract_version < 26
                && requires_parameterized_generation(read, reference.package, &c)?
            {
                return Err(reject("structured owned contracts require Graph 26"));
            }
            return Ok(c);
        }
    } else if let Some(PackageInterfaceRecord::Declaration(d)) = read.package_interface_owner(
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? && let PackageInterfaceDeclarationPayload::OwnedContract(c) = d.payload
    {
        if d.header.contract_version < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION
            && requires_mixed_generation(read, reference, &c.type_parameters)?
        {
            return Err(reject(
                "imported ordinary contract parameters require Graph 33",
            ));
        }
        if d.header.contract_version < contract::SHARE_GRAPH_CONTRACT_VERSION
            && requires_task_borrow_generation(read, &c)?
        {
            return Err(reject("imported borrowed task methods require Graph 30"));
        }
        if d.header.contract_version < 26
            && requires_parameterized_generation(read, reference.package, &c)?
        {
            return Err(reject(
                "structured imported owned contracts require Graph 26",
            ));
        }
        return Ok(c);
    }
    Err(reject("missing exact owned contract"))
}

pub(crate) fn implementation_record(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
) -> Result<OwnedImplementation, Diagnostic> {
    read.validation_work()?;
    admit_dependency(read, reference.package)?;
    if reference.package == read.package_id() {
        if let Some(OwnerRecord::Declaration(d)) =
            read.owner(OwnerKey::Declaration(reference.declaration))?
            && let DeclarationPayload::OwnedImplementation(i) = d.payload
        {
            if d.header.contract_version < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION
                && requires_mixed_generation(read, reference, &i.type_parameters)?
            {
                return Err(reject("ordinary implementation schemes require Graph 33"));
            }
            if d.header.contract_version < 29 && i.requires_prerequisite_generation() {
                return Err(reject("implementation prerequisites require Graph 29"));
            }
            return Ok(i);
        }
    } else if let Some(PackageInterfaceRecord::Declaration(d)) = read.package_interface_owner(
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? && let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = d.payload
    {
        if d.header.contract_version < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION
            && requires_mixed_generation(read, reference, &i.type_parameters)?
        {
            return Err(reject(
                "imported ordinary implementation schemes require Graph 33",
            ));
        }
        if d.header.contract_version < 29 && i.requires_prerequisite_generation() {
            return Err(reject(
                "imported implementation prerequisites require Graph 29",
            ));
        }
        return Ok(i);
    }
    Err(reject("missing exact owned implementation"))
}

pub(crate) fn function_contract(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
) -> Result<PackageFunctionSignature, Diagnostic> {
    optional_function_contract(read, reference)?
        .ok_or_else(|| reject("method target must be an exact visible graph function"))
}

fn validate_function_borrow_generation(
    read: &(impl ExpressionRead + ?Sized),
    package: PackageId,
    signature: &PackageFunctionSignature,
    generation: u16,
) -> Result<(), Diagnostic> {
    if !matches!(signature.effect, FunctionEffect::Task { .. }) {
        return Ok(());
    }
    for id in &signature.parameters {
        let parameter = parameter(read, package, *id)?;
        if parameter.use_mode == ParameterUse::Borrow
            && super::memory::direct_in(read, package, parameter.ty)?
            && (generation < contract::SHARE_GRAPH_CONTRACT_VERSION
                || parameter.header.contract_version < contract::SHARE_GRAPH_CONTRACT_VERSION)
        {
            return Err(reject(
                "borrowed task memory parameters and functions require Graph 30",
            ));
        }
    }
    Ok(())
}

pub(crate) fn optional_function_contract(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
) -> Result<Option<PackageFunctionSignature>, Diagnostic> {
    read.validation_work()?;
    admit_dependency(read, reference.package)?;
    if reference.package == read.package_id() {
        if let Some(OwnerRecord::Declaration(d)) =
            read.owner(OwnerKey::Declaration(reference.declaration))?
            && let DeclarationPayload::Function(f) = d.payload
        {
            let kind = match &f.effect {
                FunctionEffect::Pure => OwnerKind::PureFunction,
                FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
            };
            if d.header.owner != OwnerKey::Declaration(reference.declaration)
                || d.header.kind != kind
            {
                return Err(reject(
                    "function signature has a foreign exact owner identity or kind",
                ));
            }
            let signature = PackageFunctionSignature {
                implementation_parameters: f.implementation_parameters,
                requirement_parameters: f.requirement_parameters,
                effect_parameters: f.effect_parameters,
                type_parameters: f.type_parameters,
                parameters: f.parameters,
                result: f.result,
                read_scope_parameters: f.read_scope_parameters,
                read_scope_sources: f.read_scope_sources,
                effect: f.effect,
            };
            validate_function_borrow_generation(
                read,
                reference.package,
                &signature,
                d.header.contract_version,
            )?;
            return Ok(Some(signature));
        }
    } else if let Some(PackageInterfaceRecord::Declaration(d)) = read.package_interface_owner(
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? && let PackageInterfaceDeclarationPayload::Function(f) = d.payload
    {
        let kind = match &f.effect {
            FunctionEffect::Pure => OwnerKind::PureFunction,
            FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
        };
        if d.header.owner != OwnerKey::Declaration(reference.declaration) || d.header.kind != kind {
            return Err(reject(
                "imported function signature has a foreign exact owner identity or kind",
            ));
        }
        validate_function_borrow_generation(
            read,
            reference.package,
            &f,
            d.header.contract_version,
        )?;
        return Ok(Some(f));
    }
    Ok(None)
}

fn parameter(
    read: &(impl ExpressionRead + ?Sized),
    package: PackageId,
    id: crate::platform::semantic_id::ParameterId,
) -> Result<ParameterRecord, Diagnostic> {
    read.validation_work()?;
    if package == read.package_id() {
        if let Some(OwnerRecord::Parameter(p)) = read.owner(OwnerKey::Parameter(id))? {
            return Ok(p);
        }
    } else if let Some(PackageInterfaceRecord::Parameter(p)) =
        read.package_interface_owner(package, OwnerKey::Parameter(id))?
    {
        return Ok(p);
    }
    Err(reject("missing method function parameter"))
}

pub(crate) fn validate_owned_parameter(
    read: &(impl ExpressionRead + ?Sized),
    id: TypeParameterId,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    let scope =
        scope.ok_or_else(|| reject("owned parameter requires an exact declaration scope"))?;
    let p = if scope.package == read.package_id() {
        match read.owner(OwnerKey::TypeParameter(id))? {
            Some(OwnerRecord::TypeParameter(p)) => p,
            _ => return Err(reject("missing exact owned parameter")),
        }
    } else {
        match read.package_interface_owner(scope.package, OwnerKey::TypeParameter(id))? {
            Some(PackageInterfaceRecord::TypeParameter(p)) => p,
            _ => return Err(reject("missing exact imported owned parameter")),
        }
    };
    if p.header.owner != OwnerKey::TypeParameter(id)
        || p.declaration != scope.declaration
        || !p.constraints.has_owned()
    {
        return Err(reject(
            "owned parameter escapes its exact constrained declaration",
        ));
    }
    let listed = if scope.package == read.package_id() {
        match read.owner(OwnerKey::Declaration(scope.declaration))? {
            Some(OwnerRecord::Declaration(d)) => match d.payload {
                DeclarationPayload::Function(f) => {
                    super::transfer::function_parameter_listed(read, &f.type_parameters, id)?
                }
                DeclarationPayload::OwnedContract(c) => {
                    c.self_parameter == id
                        || super::transfer::function_parameter_listed(read, &c.type_parameters, id)?
                }
                DeclarationPayload::OwnedImplementation(i) => {
                    super::transfer::function_parameter_listed(read, &i.type_parameters, id)?
                }
                DeclarationPayload::OwnedRecord {
                    type_parameters, ..
                } => super::transfer::function_parameter_listed(read, &type_parameters, id)?,
                DeclarationPayload::ReadView(view) => {
                    super::transfer::function_parameter_listed(read, &view.type_parameters, id)?
                }
                _ => false,
            },
            _ => false,
        }
    } else {
        match read
            .package_interface_owner(scope.package, OwnerKey::Declaration(scope.declaration))?
        {
            Some(
                PackageInterfaceRecord::Declaration(d)
                | PackageInterfaceRecord::RepresentationDeclaration(d),
            ) => match d.payload {
                PackageInterfaceDeclarationPayload::Function(f) => {
                    super::transfer::function_parameter_listed(read, &f.type_parameters, id)?
                }
                PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                    c.self_parameter == id
                        || super::transfer::function_parameter_listed(read, &c.type_parameters, id)?
                }
                PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                    super::transfer::function_parameter_listed(read, &i.type_parameters, id)?
                }
                PackageInterfaceDeclarationPayload::OwnedRecord {
                    type_parameters, ..
                } => super::transfer::function_parameter_listed(read, &type_parameters, id)?,
                PackageInterfaceDeclarationPayload::ReadView(view) => {
                    super::transfer::function_parameter_listed(read, &view.type_parameters, id)?
                }
                _ => false,
            },
            _ => false,
        }
    };
    if !listed {
        return Err(reject(
            "owned parameter is absent from its declaration's exact parameter inventory",
        ));
    }
    Ok(())
}

pub(crate) fn validate_owned_type(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    match read
        .type_object(ty)?
        .ok_or_else(|| reject("missing owned contract argument type"))?
        .form
    {
        TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Ok(()),
        TypeForm::OwnedProduct { .. }
        | TypeForm::OwnedChoice { .. }
        | TypeForm::OwnedSequence { .. } => {
            super::owned_product::validate_in_scope(read, ty, scope)
        }
        TypeForm::TypeParameter { parameter } => validate_owned_parameter(read, parameter, scope),
        TypeForm::Named { .. } | TypeForm::Applied { .. }
            if super::owned_record::is_owned(
                read,
                scope.map_or(read.package_id(), |scope| scope.package),
                ty,
            )? =>
        {
            super::owned_record::validate_application(read, ty, scope)
        }
        _ => Err(reject("owned contract arguments require exact Owned types")),
    }
}

/// Classify only after admitting the exact occurrence. An abstract member's
/// Owned bound supplies no transfer, share or representation access proof.
pub(crate) fn validate_owned_type_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    associated.validate_occurrence_at(read, &[ty], occurrence)?;
    read.validation_work()?;
    match read
        .type_object(ty)?
        .ok_or_else(|| reject("missing occurrence-owned type"))?
        .form
    {
        TypeForm::AssociatedContractMember { .. }
        | TypeForm::AssociatedWitnessProjection { .. }
        | TypeForm::ByteBuffer
        | TypeForm::OwnedI64Cell => Ok(()),
        TypeForm::TypeParameter { parameter } => {
            associated.validate_owned_parameter_at(read, parameter, occurrence.parameters)
        }
        TypeForm::OwnedProduct { .. }
        | TypeForm::OwnedChoice { .. }
        | TypeForm::OwnedSequence { .. } => {
            super::owned_product::validate_at(read, ty, occurrence, associated)
        }
        TypeForm::Named { .. } | TypeForm::Applied { .. }
            if super::owned_record::is_owned(
                read,
                occurrence
                    .parameters
                    .scope()
                    .map_or(read.package_id(), |scope| scope.package),
                ty,
            )? =>
        {
            super::owned_record::validate_application_at(read, ty, occurrence, associated)
        }
        _ => Err(reject(
            "owned arguments require an admitted exact Owned occurrence",
        )),
    }
}

pub(crate) fn type_parameter_at(
    read: &(impl ExpressionRead + ?Sized),
    scope: DeclarationReference,
    id: TypeParameterId,
) -> Result<TypeParameterRecord, Diagnostic> {
    read.validation_work()?;
    admit_dependency(read, scope.package)?;
    let parameter = if scope.package == read.package_id() {
        match read.owner(OwnerKey::TypeParameter(id))? {
            Some(OwnerRecord::TypeParameter(parameter)) => parameter,
            _ => return Err(reject("missing exact application type parameter")),
        }
    } else {
        match read.package_interface_owner(scope.package, OwnerKey::TypeParameter(id))? {
            Some(PackageInterfaceRecord::TypeParameter(parameter)) => parameter,
            _ => return Err(reject("missing exact imported application type parameter")),
        }
    };
    if parameter.header.owner != OwnerKey::TypeParameter(id)
        || parameter.header.kind != OwnerKind::TypeParameter
        || parameter.declaration != scope.declaration
    {
        return Err(reject(
            "application type parameter escapes its exact defining declaration",
        ));
    }
    Ok(parameter)
}

/// Every actual retains the requirements of its formal, including phantom arguments.
/// The formal's defining scope and the actual's lexical scope are independent.
pub(crate) fn validate_type_argument_constraints(
    read: &(impl ExpressionRead + ?Sized),
    constraints: TypeParameterConstraints,
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    if super::owned_associated::read_view_admission::contains(
        read,
        ty,
        &OwnedAssociatedAdmission::new(),
    )? {
        return Err(reject(
            "read views cannot occur in ordinary or Owned type actuals, including phantom actuals",
        ));
    }
    if constraints.has_owned() {
        validate_owned_type(read, ty, scope)?;
    }
    if constraints.requires_transfer() {
        let owned = super::transfer::admit_in(read, ty, scope)?;
        if owned != constraints.has_owned() {
            return Err(reject(
                "transferable actual must preserve its formal's ordinary or owned category",
            ));
        }
    }
    if constraints.requires_share() {
        super::share::admit_in(read, ty, scope)?;
    }
    Ok(())
}

pub(crate) fn validate_type_argument_constraints_at(
    read: &(impl ExpressionRead + ?Sized),
    constraints: TypeParameterConstraints,
    ty: TypeObjectDigest,
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    associated.validate_occurrence_at(read, &[ty], occurrence)?;
    if super::owned_associated::read_view_admission::contains(read, ty, associated)? {
        return Err(reject(
            "read views cannot occur in ordinary or Owned type actuals, including phantom actuals",
        ));
    }
    if constraints.has_owned() {
        validate_owned_type_at(read, ty, occurrence, associated)?;
    } else if super::memory::contains_at(read, ty, occurrence, associated)? {
        return Err(reject("ordinary formal actual cannot contain owned memory"));
    }
    if constraints.requires_transfer() {
        let owned = super::transfer::admit_at(read, ty, occurrence, associated)?;
        if owned != constraints.has_owned() {
            return Err(reject(
                "transferable actual must preserve its formal's ordinary or owned category",
            ));
        }
    }
    if constraints.requires_share() {
        super::share::admit_at(read, ty, occurrence, associated)?;
    }
    Ok(())
}

fn validate_contract_arguments_at(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    contract: &OwnedContract,
    arguments: &[TypeObjectDigest],
    occurrence: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    if arguments.len() != contract.type_parameters.len() {
        return Err(reject("owned contract type argument arity mismatch"));
    }
    for (id, argument) in contract.type_parameters.iter().zip(arguments) {
        let parameter = type_parameter_at(read, reference, *id)?;
        validate_type_argument_constraints_at(
            read,
            parameter.constraints,
            *argument,
            occurrence,
            associated,
        )?;
    }
    Ok(())
}

fn ordinary_constraint(constraints: TypeParameterConstraints) -> bool {
    matches!(
        constraints,
        TypeParameterConstraints::Transferable | TypeParameterConstraints::CaptureSafeTransferable
    )
}

fn requires_mixed_generation(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    parameters: &[TypeParameterId],
) -> Result<bool, Diagnostic> {
    for id in parameters {
        if ordinary_constraint(type_parameter_at(read, reference, *id)?.constraints) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn require_mixed_generation_at(
    read: &(impl ExpressionRead + ?Sized),
    scope: DeclarationReference,
    subject: &str,
) -> Result<(), Diagnostic> {
    require_generation_at(
        read,
        scope,
        contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION,
        subject,
    )
}

fn mixed_contract_at(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
) -> Result<bool, Diagnostic> {
    let contract = contract_record(read, reference)?;
    requires_mixed_generation(read, reference, &contract.type_parameters)
}

fn mixed_operand(
    read: &(impl ExpressionRead + ?Sized),
    operand: &ImplementationOperand,
) -> Result<bool, Diagnostic> {
    operand.validate_local()?;
    for operand in operand.walk() {
        read.validation_work()?;
        match operand {
            ImplementationOperand::Concrete { implementation, .. } => {
                let record = implementation_record(read, *implementation)?;
                if requires_mixed_generation(read, *implementation, &record.type_parameters)?
                    || mixed_contract_at(read, record.contract)?
                {
                    return Ok(true);
                }
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                for declared in implementation_parameters_at(read, *scope)? {
                    read.validation_work()?;
                    if declared.id == *parameter && mixed_contract_at(read, declared.contract)? {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

/// A preceding owner generation cannot borrow newer application meaning from
/// the exact declarations it references, including unused ordinary scheme arguments.
pub(crate) fn require_expression_generation(
    read: &(impl ExpressionRead + ?Sized),
    expression: &ExpressionRecord,
) -> Result<(), Diagnostic> {
    if expression.contract_version >= contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION {
        return Ok(());
    }
    let mixed = match &expression.operation {
        ExpressionOperation::MethodCall {
            witness, contract, ..
        } => mixed_contract_at(read, *contract)? || mixed_operand(read, witness)?,
        ExpressionOperation::ImplementationCall {
            function,
            implementations,
            ..
        } => {
            let function = function_contract(read, *function)?;
            let mut mixed = false;
            for parameter in &function.implementation_parameters {
                read.validation_work()?;
                mixed |= mixed_contract_at(read, parameter.contract)?;
            }
            for operand in implementations {
                mixed |= mixed_operand(read, operand)?;
            }
            mixed
        }
        _ => false,
    };
    if mixed {
        return Err(reject(
            "mixed contract and scheme applications require Graph 33",
        ));
    }
    Ok(())
}

fn contract_bindings(
    contract: &OwnedContract,
    self_type: TypeObjectDigest,
    arguments: &[TypeObjectDigest],
) -> BTreeMap<TypeParameterId, TypeObjectDigest> {
    std::iter::once((contract.self_parameter, self_type))
        .chain(
            contract
                .type_parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied()),
        )
        .collect()
}

fn substituted_equal(
    derived: &mut super::parallel_types::AppliedTypes<'_, impl ExpressionRead + ?Sized>,
    template: TypeObjectDigest,
    actual: TypeObjectDigest,
    substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
) -> Result<bool, Diagnostic> {
    Ok(derived.substitute(template, substitutions, 0)? == actual)
}

pub(crate) fn type_parameter_ids(
    contract: &OwnedContract,
) -> impl Iterator<Item = TypeParameterId> + '_ {
    std::iter::once(contract.self_parameter).chain(contract.type_parameters.iter().copied())
}

pub(crate) fn requires_parameterized_generation(
    read: &(impl ExpressionRead + ?Sized),
    package: PackageId,
    contract: &OwnedContract,
) -> Result<bool, Diagnostic> {
    if !contract.type_parameters.is_empty() {
        return Ok(true);
    }
    for method in &contract.methods {
        for ty in method
            .parameters
            .iter()
            .map(|p| p.ty)
            .chain([method.result.ty])
        {
            read.validation_work()?;
            if super::memory::direct_in(read, package, ty)?
                && read.type_object(ty)?.map(|t| t.form)
                    != Some(TypeForm::TypeParameter {
                        parameter: contract.self_parameter,
                    })
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
/// Validate every mapping, including methods that no expression invokes.
pub(crate) fn validate_implementation(
    read: &(impl ExpressionRead + ?Sized),
    implementation: &OwnedImplementation,
) -> Result<(), Diagnostic> {
    let scope = implementation
        .type_parameters
        .first()
        .map(|id| match read.owner(OwnerKey::TypeParameter(*id))? {
            Some(OwnerRecord::TypeParameter(p)) => Ok(DeclarationReference {
                package: read.package_id(),
                declaration: p.declaration,
            }),
            _ => Err(reject("missing implementation type parameter")),
        })
        .transpose()?;
    let types = BTreeMap::new();
    let read = AppliedTypeRead {
        read,
        types: &types,
    };
    let associated = OwnedAssociatedAdmission::new();
    validate_implementation_scoped(
        &read,
        scope,
        implementation,
        &mut WitnessAdmission::new(&associated),
    )
}

pub(crate) fn validate_implementation_at(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    implementation: &OwnedImplementation,
) -> Result<(), Diagnostic> {
    validate_implementation_at_with_admission(
        read,
        reference,
        implementation,
        &OwnedAssociatedAdmission::new(),
    )
}

pub(crate) fn validate_implementation_at_with_admission(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    implementation: &OwnedImplementation,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    let types = BTreeMap::new();
    let read = AppliedTypeRead {
        read,
        types: &types,
    };
    validate_implementation_scoped(
        &read,
        Some(reference),
        implementation,
        &mut WitnessAdmission::new(associated),
    )
}

struct WitnessAdmission<'a> {
    active: BTreeSet<DeclarationReference>,
    completed: BTreeSet<DeclarationReference>,
    operand_depth: usize,
    associated: &'a OwnedAssociatedAdmission,
}

fn admit_operand_metadata(
    read: &(impl ExpressionRead + ?Sized),
    operand: &ImplementationOperand,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    operand.validate_local()?;
    for node in operand.walk() {
        read.validation_work()?;
        associated.reserve::<ImplementationOperand>(1)?;
        associated.reserve::<TypeObjectDigest>(node.type_arguments().len())?;
    }
    Ok(())
}

fn extend_application_types(
    read: &(impl ExpressionRead + ?Sized),
    target: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    source: BTreeMap<TypeObjectDigest, TypeObject>,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    for (digest, object) in source {
        read.validation_work()?;
        associated.reserve::<(TypeObjectDigest, TypeObject)>(1)?;
        associated.reserve::<TypeObjectDigest>(object.child_type_count())?;
        match &object.form {
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    read.validation_work()?;
                    associated.reserve_bytes(field.name.as_str().len())?;
                }
            }
            _ => {}
        }
        if target
            .get(&digest)
            .is_some_and(|previous| previous != &object)
        {
            return Err(reject(
                "derived type digest has conflicting canonical metadata",
            ));
        }
        target.insert(digest, object);
    }
    Ok(())
}

impl<'a> WitnessAdmission<'a> {
    fn new(associated: &'a OwnedAssociatedAdmission) -> Self {
        Self {
            active: BTreeSet::new(),
            completed: BTreeSet::new(),
            operand_depth: 0,
            associated,
        }
    }
}

// This authenticates relative interface metadata, not complete supplier source.
// Every accepting publication/load route separately discharges private mapped
// signature equations against canonical supplier source and exact projection.
fn validate_imported_definition_identity(
    read: &dyn ExpressionRead,
    scope: DeclarationReference,
    implementation: &OwnedImplementation,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    associated.step(read)?;
    admit_dependency(read, scope.package)?;
    let key = OwnerKey::Declaration(scope.declaration);
    let Some(PackageInterfaceRecord::Declaration(record)) =
        read.package_interface_owner(scope.package, key)?
    else {
        return Err(reject("missing exact imported implementation descriptor"));
    };
    if !contract::supported_graph_contract(record.header.contract_version)
        || record.header.owner != key
        || record.header.kind != OwnerKind::OwnedImplementation
    {
        return Err(reject(
            "imported implementation descriptor has a foreign owner identity or kind",
        ));
    }
    let generation = record.header.contract_version;
    let PackageInterfaceDeclarationPayload::OwnedImplementation(actual) = record.payload else {
        return Err(reject("missing exact imported implementation descriptor"));
    };
    // Charge both complete bounded descriptors before their equality scan,
    // including nested mapped operands rather than only method identities.
    for definition in [&actual, implementation] {
        for _ in &definition.type_parameters {
            associated.step(read)?;
        }
        for _ in &definition.type_arguments {
            associated.step(read)?;
        }
        for binding in &definition.associated_types {
            associated.step(read)?;
            if let AssociatedTypeBindingValue::ReadViewFamily {
                scope_parameter, ..
            } = &binding.value
            {
                for _ in scope_parameter.name.as_str().bytes() {
                    associated.step(read)?;
                }
            }
        }
        for parameter in &definition.implementation_parameters {
            associated.step(read)?;
            for _ in parameter.name.as_str().bytes() {
                associated.step(read)?;
            }
            for _ in &parameter.type_arguments {
                associated.step(read)?;
            }
        }
        for method in &definition.methods {
            associated.step(read)?;
            for _ in &method.type_arguments {
                associated.step(read)?;
            }
            for _ in &method.read_scope_arguments {
                associated.step(read)?;
            }
            for operand in &method.implementations {
                admit_operand_metadata(read, operand, associated)?;
                for node in operand.walk() {
                    associated.step(read)?;
                    for _ in node.type_arguments() {
                        associated.step(read)?;
                    }
                }
            }
        }
    }
    if generation < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION
        && requires_mixed_generation(read, scope, &actual.type_parameters)?
    {
        return Err(reject(
            "imported ordinary implementation schemes require Graph 33",
        ));
    }
    if generation < 29 && actual.requires_prerequisite_generation() {
        return Err(reject(
            "imported implementation prerequisites require Graph 29",
        ));
    }
    if actual != *implementation {
        return Err(reject(
            "imported implementation descriptor differs from its exact selected inventory",
        ));
    }
    Ok(())
}

fn validate_deferred_private_method_metadata(
    read: &dyn ExpressionRead,
    implementation: &OwnedImplementation,
    method: &OwnedMethod,
    target: &OwnedMethodImplementation,
    occurrence: OwnedTypeOccurrence,
    admission: &mut WitnessAdmission<'_>,
) -> Result<(), Diagnostic> {
    let associated = admission.associated;
    let binder = ReadScopeBinder::ContractMethod {
        contract: implementation.contract,
        method: method.id,
    };
    reserve_scope_inventory(
        &method.read_scope_parameters,
        &method.read_scope_sources,
        associated,
    )?;
    validate_read_scope_inventory_at(
        binder,
        &method.read_scope_parameters,
        &method.read_scope_sources,
        |position| (position as usize) < method.parameters.len(),
    )?;
    if target.read_scope_arguments.len() != method.read_scope_parameters.len() {
        return Err(reject(
            "deferred mapped scopes differ from the exact contract formal inventory",
        ));
    }
    let empty_loans = BTreeSet::new();
    let authority = super::owned_associated::read_view_admission::ScopeAuthority {
        binder: Some(binder),
        lexical: None,
        live_loans: &empty_loans,
    };
    for actual in &target.read_scope_arguments {
        associated.step(read)?;
        authority.validate(read, *actual, associated)?;
    }
    // Hidden formals' bounds remain a supplier-source obligation. Each written
    // actual is still an admitted occurrence, including phantom positions.
    for ty in &target.type_arguments {
        associated.step(read)?;
        associated.validate_occurrence_at(read, &[*ty], occurrence)?;
        if super::owned_associated::read_view_admission::contains(read, *ty, associated)? {
            return Err(reject(
                "read views cannot occur in ordinary or Owned type actuals, including phantom actuals",
            ));
        }
    }
    for operand in &target.implementations {
        admit_operand_metadata(read, operand, associated)?;
        for node in operand.walk() {
            associated.step(read)?;
            if matches!(operand, ImplementationOperand::Concrete { .. })
                && matches!(node, ImplementationOperand::Parameter { .. })
            {
                return Err(reject(
                    "mapped concrete applications must contain no lexical witness operands",
                ));
            }
            for ty in node.type_arguments() {
                associated.step(read)?;
                associated.validate_occurrence_at(read, &[*ty], occurrence)?;
                if super::owned_associated::read_view_admission::contains(read, *ty, associated)? {
                    return Err(reject(
                        "read views cannot occur in ordinary or Owned type actuals, including phantom actuals",
                    ));
                }
            }
        }
        // All exact visible witness/prerequisite relations remain immediate;
        // only the absent callable's formal-to-actual matching is deferred.
        witness_contract_scoped(read, operand, occurrence.parameters.scope(), admission)?;
    }
    Ok(())
}

fn validate_implementation_scoped(
    read: &dyn ExpressionRead,
    scope: Option<DeclarationReference>,
    implementation: &OwnedImplementation,
    admission: &mut WitnessAdmission,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    if let Some(scope) = scope {
        if scope.package != read.package_id() {
            validate_imported_definition_identity(
                read,
                scope,
                implementation,
                admission.associated,
            )?;
        }
        // Definition cycles with closed witness resets can have finite applications.
        // Each active frame still admits all its own maps before it completes.
        if admission.active.contains(&scope) || admission.completed.contains(&scope) {
            return Ok(());
        }
        if admission.active.len() > contract::MAXIMUM_TYPE_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_witness_definition_capacity",
                "implementation definition admission exhausted its finite depth capacity",
            ));
        }
        admission.associated.reserve::<DeclarationReference>(1)?;
        admission.active.insert(scope);
    }
    let result = validate_implementation_definition(read, scope, implementation, admission);
    if let Some(scope) = scope {
        admission.active.remove(&scope);
        if result.is_ok() {
            admission.associated.reserve::<DeclarationReference>(1)?;
            admission.completed.insert(scope);
        }
    }
    result
}

fn validate_implementation_definition(
    read: &dyn ExpressionRead,
    scope: Option<DeclarationReference>,
    implementation: &OwnedImplementation,
    admission: &mut WitnessAdmission,
) -> Result<(), Diagnostic> {
    let occurrence = defining_occurrence(read, scope)?;
    for _ in &implementation.methods {
        read.validation_work()?;
    }
    if !implementation.implementation_parameters.is_empty() {
        let scope =
            scope.ok_or_else(|| reject("implementation prerequisites require an exact scope"))?;
        for prerequisite in &implementation.implementation_parameters {
            validate_witness_parameter(read, prerequisite, scope, admission.associated)?;
        }
    }
    // Account for the local shape check's temporary inventories before it builds
    // them, and for this operation's retained formal-name inventory separately.
    admission
        .associated
        .reserve::<TypeParameterId>(implementation.type_parameters.len())?;
    admission
        .associated
        .reserve::<ImplementationParameterId>(implementation.implementation_parameters.len())?;
    admission
        .associated
        .reserve::<&Name>(implementation.implementation_parameters.len())?;
    implementation.validate_local()?;
    admission
        .associated
        .reserve::<Name>(implementation.type_parameters.len())?;
    let mut names = BTreeSet::new();
    for id in &implementation.type_parameters {
        let scope =
            scope.ok_or_else(|| reject("generic implementation requires an exact scope"))?;
        let p = type_parameter_at(read, scope, *id)?;
        admission.associated.reserve_bytes(p.name.as_str().len())?;
        if (!p.constraints.has_owned() && !ordinary_constraint(p.constraints))
            || p.header.contract_version < 28
            || !names.insert(p.name)
        {
            return Err(reject(
                "implementation schemes require distinct exact owned or ordinary-transferable parameters",
            ));
        }
        if p.constraints.has_owned() {
            validate_owned_parameter(read, *id, Some(scope))?;
        }
        if ordinary_constraint(p.constraints) {
            if p.header.contract_version < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION {
                return Err(reject(
                    "ordinary implementation parameters require Graph 33",
                ));
            }
            require_mixed_generation_at(read, scope, "ordinary implementation schemes")?;
        } else if p.constraints != TypeParameterConstraints::Owned {
            require_share_generation_at(read, scope, "stronger implementation schemes")?;
        }
    }
    validate_owned_type_at(
        read,
        implementation.self_type,
        occurrence,
        admission.associated,
    )?;
    let contract = contract_record(read, implementation.contract)?;
    if !contract.associated_types.is_empty() || !implementation.associated_types.is_empty() {
        admission.associated.validate_bindings(
            read,
            scope.ok_or_else(|| {
                reject("associated implementations require an exact defining scope")
            })?,
            implementation,
        )?;
    }
    validate_contract_at_with(
        read,
        implementation.contract,
        &contract,
        admission.associated,
    )?;
    if requires_task_borrow_generation(read, &contract)? {
        require_share_generation_at(
            read,
            scope.ok_or_else(|| reject("borrowed task method maps require an exact scope"))?,
            "borrowed task method maps",
        )?;
    }
    if requires_mixed_generation(read, implementation.contract, &contract.type_parameters)? {
        require_mixed_generation_at(
            read,
            scope.ok_or_else(|| reject("mixed contract implementations require an exact scope"))?,
            "mixed contract implementations",
        )?;
    }
    validate_contract_arguments_at(
        read,
        implementation.contract,
        &contract,
        &implementation.type_arguments,
        occurrence,
        admission.associated,
    )?;
    if implementation.methods.len() != contract.methods.len() {
        return Err(reject("missing implementation method"));
    }
    admission
        .associated
        .reserve::<(TypeParameterId, TypeObjectDigest)>(1)?;
    admission
        .associated
        .reserve::<(TypeParameterId, TypeObjectDigest)>(contract.type_parameters.len())?;
    let substitutions = contract_bindings(
        &contract,
        implementation.self_type,
        &implementation.type_arguments,
    );
    for method in &contract.methods {
        read.validation_work()?;
        let mut target = None;
        for candidate in &implementation.methods {
            read.validation_work()?;
            if candidate.method == method.id {
                target = Some(candidate);
                break;
            }
        }
        let target = target.ok_or_else(|| reject("wrong implementation method identity"))?;
        let f = match optional_function_contract(read, target.function)? {
            Some(function) => function,
            None if scope.is_some_and(|owner| {
                owner.package != read.package_id() && owner.package == target.function.package
            }) && {
                admission.associated.step(read)?;
                read.package_interface_owner(
                    target.function.package,
                    OwnerKey::Declaration(target.function.declaration),
                )?
                .is_none()
            } =>
            {
                // Only the supplier's own absent private target is relative
                // interface evidence. Complete supplier-local validation is
                // mandatory before outer source/publication/artifact readiness.
                validate_deferred_private_method_metadata(
                    read,
                    implementation,
                    method,
                    target,
                    occurrence,
                    admission,
                )?;
                continue;
            }
            None => {
                return Err(reject(
                    "method target must be an exact visible graph function",
                ));
            }
        };
        if f.type_parameters.len() != target.type_arguments.len()
            || f.read_scope_parameters.len() != method.read_scope_parameters.len()
            || f.read_scope_parameters.len() != target.read_scope_arguments.len()
            || !f.effect_parameters.is_empty()
            || !f.requirement_parameters.is_empty()
            || f.implementation_parameters.len() != target.implementations.len()
            || f.effect != method.effect
            || f.parameters.len() != method.parameters.len()
        {
            return Err(reject(
                "implementation methods require exact graph-function applications and closed callable kind/effect rows",
            ));
        }
        admission
            .associated
            .reserve::<(TypeParameterId, TypeObjectDigest)>(target.type_arguments.len())?;
        let mut target_bindings = BTreeMap::new();
        for (id, ty) in f.type_parameters.iter().zip(&target.type_arguments) {
            read.validation_work()?;
            let p = type_parameter_at(read, target.function, *id)?;
            if (!p.constraints.has_owned() && !ordinary_constraint(p.constraints))
                || target_bindings.insert(*id, *ty).is_some()
            {
                return Err(reject(
                    "mapped function applications require distinct exact owned or ordinary-transferable parameters",
                ));
            }
            if ordinary_constraint(p.constraints) {
                require_mixed_generation_at(
                    read,
                    scope.ok_or_else(|| {
                        reject("ordinary mapped applications require an exact scope")
                    })?,
                    "ordinary mapped function applications",
                )?;
            } else if p.constraints != TypeParameterConstraints::Owned {
                require_share_generation_at(
                    read,
                    scope.ok_or_else(|| {
                        reject("stronger mapped applications require an exact scope")
                    })?,
                    "stronger mapped function applications",
                )?;
            }
            validate_type_argument_constraints_at(
                read,
                p.constraints,
                *ty,
                occurrence,
                admission.associated,
            )?;
        }
        for operand in &target.implementations {
            if matches!(operand, ImplementationOperand::Concrete { .. })
                && operand
                    .walk()
                    .any(|operand| matches!(operand, ImplementationOperand::Parameter { .. }))
            {
                return Err(reject(
                    "mapped concrete applications must contain no lexical witness operands",
                ));
            }
        }
        validate_witness_parameters(
            read,
            &f.implementation_parameters,
            target.function,
            admission.associated,
        )?;
        let method_binder = ReadScopeBinder::ContractMethod {
            contract: implementation.contract,
            method: method.id,
        };
        reserve_scope_inventory(
            &method.read_scope_parameters,
            &method.read_scope_sources,
            admission.associated,
        )?;
        reserve_scope_inventory(
            &f.read_scope_parameters,
            &f.read_scope_sources,
            admission.associated,
        )?;
        validate_read_scope_inventory_at(
            method_binder,
            &method.read_scope_parameters,
            &method.read_scope_sources,
            |position| (position as usize) < method.parameters.len(),
        )?;
        validate_read_scope_inventory_at(
            ReadScopeBinder::Declaration(target.function),
            &f.read_scope_parameters,
            &f.read_scope_sources,
            |parameter| f.parameters.contains(&parameter),
        )?;
        admission
            .associated
            .reserve::<(ReadScopeReference, ReadScopeArgument)>(
                method.read_scope_parameters.len(),
            )?;
        let expected_scopes = method
            .read_scope_parameters
            .iter()
            .map(|parameter| {
                let reference = ReadScopeReference {
                    binder: method_binder,
                    parameter: parameter.id,
                };
                (reference, ReadScopeArgument::Parameter(reference))
            })
            .collect::<BTreeMap<_, _>>();
        let empty_loans = BTreeSet::new();
        let target_scopes = if f.read_scope_parameters.is_empty() {
            BTreeMap::new()
        } else {
            super::owned_associated::read_view_admission::scope_bindings(
                read,
                ReadScopeBinder::Declaration(target.function),
                &target.read_scope_arguments,
                super::owned_associated::read_view_admission::ScopeAuthority {
                    binder: Some(method_binder),
                    lexical: None,
                    live_loans: &empty_loans,
                },
                admission.associated,
            )?
        };
        let application_types = validate_witness_arguments_with_scopes(
            read,
            target.function,
            &f.implementation_parameters,
            &target_bindings,
            &target_scopes,
            &target.implementations,
            occurrence,
            admission,
        )?;
        let overlay = AppliedTypeRead {
            read,
            types: &application_types,
        };
        admission
            .associated
            .reserve::<(ImplementationParameterId, ImplementationOperand)>(
                target.implementations.len(),
            )?;
        let target_witnesses = f
            .implementation_parameters
            .iter()
            .zip(&target.implementations)
            .map(|(parameter, operand)| (parameter.id, operand.clone()))
            .collect::<BTreeMap<_, _>>();
        let empty_witnesses = BTreeMap::new();
        let expected_environment = super::parallel_types::AssociatedTypeEnvironment {
            ordinary: &substitutions,
            scopes: &expected_scopes,
            source: super::owned_associated::AssociatedTypeContext::ContractMethod {
                contract: implementation.contract,
                method: method.id,
            },
            caller: occurrence.associated,
            consumer_generation: occurrence.consumer_generation,
            contract_selection: scope.map(|scope| {
                super::parallel_types::AssociatedContractSelection::Definition {
                    scope,
                    implementation,
                }
            }),
            implementations: &empty_witnesses,
        };
        let target_environment = super::parallel_types::AssociatedTypeEnvironment {
            ordinary: &target_bindings,
            scopes: &target_scopes,
            source: super::owned_associated::AssociatedTypeContext::Lexical(target.function),
            caller: occurrence.associated,
            consumer_generation: occurrence.consumer_generation,
            contract_selection: None,
            implementations: &target_witnesses,
        };
        let mut expected_types = super::parallel_types::WitnessAppliedTypes::with_admission(
            &overlay,
            expected_environment,
            admission.associated,
        );
        let mut target_types = super::parallel_types::WitnessAppliedTypes::with_admission(
            &overlay,
            target_environment,
            admission.associated,
        );
        let source_occurrence = defining_occurrence(read, Some(target.function))?;
        admission
            .associated
            .validate_occurrence_at(read, &[f.result.ty], source_occurrence)?;
        for (id, expected) in f.parameters.iter().zip(&method.parameters) {
            let p = parameter(read, target.function.package, *id)?;
            admission
                .associated
                .validate_occurrence_at(read, &[p.ty], source_occurrence)?;
            admission.associated.validate_occurrence_at(
                read,
                &[p.ty],
                OwnedTypeOccurrence {
                    consumer_generation: p.header.contract_version,
                    ..source_occurrence
                },
            )?;
            if p.parent != ParameterParent::Function(target.function.declaration)
                || p.header.owner != OwnerKey::Parameter(*id)
                || expected_types.substitute(expected.ty, 0)? != target_types.substitute(p.ty, 0)?
                || p.use_mode != expected.use_mode
                || p.resource_requirement.is_some()
            {
                return Err(reject(
                    "implementation parameter type, scope or use mode mismatch",
                ));
            }
        }
        if expected_types.substitute(method.result.ty, 0)?
            != target_types.substitute(f.result.ty, 0)?
        {
            return Err(reject("implementation result mismatch"));
        }
        let mut expected_relation = method.result.relation.try_map_source(|position| {
            f.parameters
                .get(position as usize)
                .copied()
                .ok_or_else(|| reject("implementation borrowed result source is out of range"))
        })?;
        if let ResultRelation::ViewFrom { scope, .. } = &mut expected_relation {
            *scope = expected_types.substitute_scope(*scope)?;
        }
        let mut actual_relation = f.result.relation;
        if let ResultRelation::ViewFrom { scope, .. } = &mut actual_relation {
            *scope = target_types.substitute_scope(*scope)?;
        }
        if actual_relation != expected_relation {
            return Err(reject("implementation borrowed result source mismatch"));
        }
        if method.read_scope_sources.len() != f.read_scope_sources.len() {
            return Err(reject("implementation source-scope inventory mismatch"));
        }
        for (expected, actual) in method.read_scope_sources.iter().zip(&f.read_scope_sources) {
            read.validation_work()?;
            let source = f
                .parameters
                .get(expected.source as usize)
                .copied()
                .ok_or_else(|| {
                    reject("mapped source-scope position is outside the exact parameter inventory")
                })?;
            if source != actual.source
                || expected_types.substitute_scope(expected.scope)?
                    != target_types.substitute_scope(actual.scope)?
            {
                return Err(reject("implementation source-scope relationship mismatch"));
            }
        }
    }
    Ok(())
}

pub(crate) fn witness_contract(
    read: &(impl ExpressionRead + ?Sized),
    operand: &ImplementationOperand,
    scope: Option<DeclarationId>,
) -> Result<AppliedWitnessContract, Diagnostic> {
    let scope = scope.map(|declaration| DeclarationReference {
        package: read.package_id(),
        declaration,
    });
    let types = BTreeMap::new();
    let read = AppliedTypeRead {
        read,
        types: &types,
    };
    let associated = OwnedAssociatedAdmission::new();
    witness_contract_scoped(
        &read,
        operand,
        scope,
        &mut WitnessAdmission::new(&associated),
    )
}

pub(crate) fn implementation_parameters_at(
    read: &(impl ExpressionRead + ?Sized),
    scope: DeclarationReference,
) -> Result<Vec<ImplementationParameter>, Diagnostic> {
    read.validation_work()?;
    admit_dependency(read, scope.package)?;
    if scope.package == read.package_id() {
        if let Some(OwnerRecord::Declaration(declaration)) =
            read.owner(OwnerKey::Declaration(scope.declaration))?
        {
            return match declaration.payload {
                DeclarationPayload::Function(function) => Ok(function.implementation_parameters),
                DeclarationPayload::OwnedImplementation(implementation) => {
                    Ok(implementation.implementation_parameters)
                }
                _ => Err(reject(
                    "witness parameters require an exact function or implementation scope",
                )),
            };
        }
    } else if let Some(PackageInterfaceRecord::Declaration(declaration)) =
        read.package_interface_owner(scope.package, OwnerKey::Declaration(scope.declaration))?
    {
        return match declaration.payload {
            PackageInterfaceDeclarationPayload::Function(function) => {
                Ok(function.implementation_parameters)
            }
            PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) => {
                Ok(implementation.implementation_parameters)
            }
            _ => Err(reject(
                "imported witness parameters require an exact function or implementation scope",
            )),
        };
    }
    Err(reject("missing exact witness declaration scope"))
}

fn validate_witness_parameter(
    read: &(impl ExpressionRead + ?Sized),
    parameter: &ImplementationParameter,
    scope: DeclarationReference,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    read.validation_work()?;
    let contract = contract_record(read, parameter.contract)?;
    validate_contract_at_with(read, parameter.contract, &contract, associated)?;
    if !matches!(
        read.type_object(parameter.self_type)?.map(|ty| ty.form),
        Some(TypeForm::TypeParameter { .. })
    ) {
        return Err(reject(
            "witness Self requires an exact in-scope owned type parameter",
        ));
    }
    let occurrence = defining_occurrence(read, Some(scope))?;
    validate_owned_type_at(read, parameter.self_type, occurrence, associated)?;
    if requires_mixed_generation(read, parameter.contract, &contract.type_parameters)? {
        require_mixed_generation_at(read, scope, "mixed contract prerequisites")?;
    }
    validate_contract_arguments_at(
        read,
        parameter.contract,
        &contract,
        &parameter.type_arguments,
        occurrence,
        associated,
    )
}

fn validate_witness_parameters(
    read: &(impl ExpressionRead + ?Sized),
    parameters: &[ImplementationParameter],
    scope: DeclarationReference,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    if parameters.len() > contract::MAXIMUM_CHILDREN {
        return Err(reject("witness parameter inventory exceeds its bound"));
    }
    associated.reserve::<ImplementationParameterId>(parameters.len())?;
    associated.reserve::<&Name>(parameters.len())?;
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for parameter in parameters {
        if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
            return Err(reject(
                "witness parameters require distinct identities and names",
            ));
        }
        validate_witness_parameter(read, parameter, scope, associated)?;
    }
    Ok(())
}

fn validate_witness_arguments(
    read: &dyn ExpressionRead,
    source: DeclarationReference,
    parameters: &[ImplementationParameter],
    substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    operands: &[ImplementationOperand],
    occurrence: OwnedTypeOccurrence,
    admission: &mut WitnessAdmission,
) -> Result<BTreeMap<TypeObjectDigest, TypeObject>, Diagnostic> {
    validate_witness_arguments_with_scopes(
        read,
        source,
        parameters,
        substitutions,
        &BTreeMap::new(),
        operands,
        occurrence,
        admission,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "complete ordinary, scope and witness application inventories retain independent authority"
)]
fn validate_witness_arguments_with_scopes(
    read: &dyn ExpressionRead,
    source: DeclarationReference,
    parameters: &[ImplementationParameter],
    substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
    scopes: &BTreeMap<ReadScopeReference, ReadScopeArgument>,
    operands: &[ImplementationOperand],
    occurrence: OwnedTypeOccurrence,
    admission: &mut WitnessAdmission,
) -> Result<BTreeMap<TypeObjectDigest, TypeObject>, Diagnostic> {
    if parameters.len() != operands.len() {
        return Err(reject(
            "implementation prerequisite argument arity mismatch",
        ));
    }
    let associated = admission.associated;
    associated.reserve::<AppliedWitnessContract>(operands.len())?;
    associated.reserve::<(ImplementationParameterId, ImplementationOperand)>(operands.len())?;
    let mut types = BTreeMap::new();
    let mut applications = Vec::with_capacity(operands.len());
    let mut witnesses = BTreeMap::new();
    for (parameter, operand) in parameters.iter().zip(operands) {
        read.validation_work()?;
        admit_operand_metadata(read, operand, associated)?;
        let overlay = AppliedTypeRead {
            read,
            types: &types,
        };
        let mut application =
            witness_contract_scoped(&overlay, operand, occurrence.parameters.scope(), admission)?;
        extend_application_types(
            read,
            &mut types,
            std::mem::take(&mut application.types),
            associated,
        )?;
        if witnesses.insert(parameter.id, operand.clone()).is_some() {
            return Err(reject(
                "prerequisite inventory has a duplicate exact witness parameter",
            ));
        }
        applications.push(application);
    }
    let overlay = AppliedTypeRead {
        read,
        types: &types,
    };
    let environment = super::parallel_types::AssociatedTypeEnvironment {
        ordinary: substitutions,
        scopes,
        source: super::owned_associated::AssociatedTypeContext::Lexical(source),
        caller: occurrence.associated,
        consumer_generation: occurrence.consumer_generation,
        contract_selection: None,
        implementations: &witnesses,
    };
    let mut derived = super::parallel_types::WitnessAppliedTypes::with_admission(
        &overlay,
        environment,
        associated,
    );
    for (parameter, application) in parameters.iter().zip(&applications) {
        if application.contract != parameter.contract
            || derived.substitute(parameter.self_type, 0)? != application.self_type
            || application.type_arguments.len() != parameter.type_arguments.len()
        {
            return Err(reject(
                "prerequisite contract, Self or ordered type arguments do not match",
            ));
        }
        for (template, actual) in parameter
            .type_arguments
            .iter()
            .zip(&application.type_arguments)
        {
            if derived.substitute(*template, 0)? != *actual {
                return Err(reject(
                    "prerequisite ordered type argument substitution mismatch",
                ));
            }
        }
    }
    let derived_types = derived.into_types();
    extend_application_types(read, &mut types, derived_types, associated)?;
    Ok(types)
}

fn witness_contract_scoped(
    read: &dyn ExpressionRead,
    operand: &ImplementationOperand,
    scope: Option<DeclarationReference>,
    admission: &mut WitnessAdmission,
) -> Result<AppliedWitnessContract, Diagnostic> {
    read.validation_work()?;
    if admission.operand_depth > contract::MAXIMUM_TYPE_DEPTH {
        return Err(reject(
            "implementation prerequisite application exceeds its depth bound",
        ));
    }
    if admission.operand_depth == 0 {
        operand.validate_local()?;
    }
    admission.operand_depth += 1;
    let result = witness_contract_inner(read, operand, scope, admission);
    admission.operand_depth -= 1;
    result
}

fn witness_contract_inner(
    read: &dyn ExpressionRead,
    operand: &ImplementationOperand,
    scope: Option<DeclarationReference>,
    admission: &mut WitnessAdmission,
) -> Result<AppliedWitnessContract, Diagnostic> {
    let occurrence = defining_occurrence(read, scope)?;
    match operand {
        ImplementationOperand::Concrete {
            implementation,
            type_arguments,
            implementations,
        } => {
            let i = implementation_record(read, *implementation)?;
            validate_implementation_scoped(read, Some(*implementation), &i, admission)?;
            if i.type_parameters.len() != type_arguments.len() {
                return Err(reject("implementation scheme type argument arity mismatch"));
            }
            for (id, ty) in i.type_parameters.iter().zip(type_arguments) {
                let parameter = type_parameter_at(read, *implementation, *id)?;
                validate_type_argument_constraints_at(
                    read,
                    parameter.constraints,
                    *ty,
                    occurrence,
                    admission.associated,
                )?;
            }
            admission
                .associated
                .reserve::<(TypeParameterId, TypeObjectDigest)>(type_arguments.len())?;
            let bindings = i
                .type_parameters
                .iter()
                .copied()
                .zip(type_arguments.iter().copied())
                .collect();
            let mut types = validate_witness_arguments(
                read,
                *implementation,
                &i.implementation_parameters,
                &bindings,
                implementations,
                occurrence,
                admission,
            )?;
            admission
                .associated
                .reserve::<(ImplementationParameterId, ImplementationOperand)>(
                    implementations.len(),
                )?;
            let witnesses = i
                .implementation_parameters
                .iter()
                .zip(implementations)
                .map(|(parameter, operand)| (parameter.id, operand.clone()))
                .collect::<BTreeMap<_, _>>();
            let overlay = AppliedTypeRead {
                read,
                types: &types,
            };
            let empty_scopes = BTreeMap::new();
            let environment = super::parallel_types::AssociatedTypeEnvironment {
                ordinary: &bindings,
                scopes: &empty_scopes,
                source: super::owned_associated::AssociatedTypeContext::Lexical(*implementation),
                caller: occurrence.associated,
                consumer_generation: occurrence.consumer_generation,
                contract_selection: None,
                implementations: &witnesses,
            };
            let mut derived = super::parallel_types::WitnessAppliedTypes::with_admission(
                &overlay,
                environment,
                admission.associated,
            );
            let self_type = derived.substitute(i.self_type, 0)?;
            admission
                .associated
                .reserve::<TypeObjectDigest>(i.type_arguments.len())?;
            let mut arguments = Vec::with_capacity(i.type_arguments.len());
            for ty in &i.type_arguments {
                arguments.push(derived.substitute(*ty, 0)?);
            }
            validate_owned_type_at(&derived, self_type, occurrence, admission.associated)?;
            let contract = contract_record(read, i.contract)?;
            validate_contract_arguments_at(
                &derived,
                i.contract,
                &contract,
                &arguments,
                occurrence,
                admission.associated,
            )?;
            let derived_types = derived.into_types();
            extend_application_types(read, &mut types, derived_types, admission.associated)?;
            Ok(AppliedWitnessContract {
                contract: i.contract,
                self_type,
                type_arguments: arguments,
                types,
            })
        }
        ImplementationOperand::Parameter {
            scope: declaring_scope,
            parameter,
        } => {
            if Some(*declaring_scope) != scope {
                return Err(reject(
                    "implementation parameter escaped its exact declaration scope",
                ));
            }
            let parameters = implementation_parameters_at(read, *declaring_scope)?;
            let mut found = None;
            for p in &parameters {
                read.validation_work()?;
                if p.id == *parameter {
                    found = Some(p);
                    break;
                }
            }
            let p = found.ok_or_else(|| reject("missing implementation parameter"))?;
            validate_witness_parameter(read, p, *declaring_scope, admission.associated)?;
            admission
                .associated
                .reserve::<TypeObjectDigest>(p.type_arguments.len())?;
            Ok(AppliedWitnessContract {
                contract: p.contract,
                self_type: p.self_type,
                type_arguments: p.type_arguments.clone(),
                types: BTreeMap::new(),
            })
        }
    }
}

pub(crate) fn method_signature(
    read: &(impl ExpressionRead + ?Sized),
    operand: &ImplementationOperand,
    reference: DeclarationReference,
    id: MethodId,
    scope: Option<DeclarationId>,
) -> Result<AppliedOwnedMethod, Diagnostic> {
    let application = witness_contract(read, operand, scope)?;
    if application.contract != reference {
        return Err(reject("witness names a different nominal contract"));
    }
    let c = contract_record(read, reference)?;
    validate_contract_at(read, reference, &c)?;
    let mut found = None;
    for method in &c.methods {
        read.validation_work()?;
        if method.id == id {
            found = Some(method.clone());
            break;
        }
    }
    let mut method =
        found.ok_or_else(|| reject("method is absent from the exact witness contract"))?;
    if !method.read_scope_parameters.is_empty() {
        return Err(reject(
            "scope-bearing methods require complete explicit application admission",
        ));
    }
    let substitutions = contract_bindings(&c, application.self_type, &application.type_arguments);
    let overlay = AppliedTypeRead {
        read,
        types: &application.types,
    };
    let mut derived = super::parallel_types::AppliedTypes::new(&overlay);
    for p in &mut method.parameters {
        p.ty = derived.substitute(p.ty, &substitutions, 0)?;
    }
    method.result = method
        .result
        .try_map_type(|ty| derived.substitute(ty, &substitutions, 0))?;
    let mut types = derived.into_types();
    types.extend(application.types);
    Ok(AppliedOwnedMethod { method, types })
}

pub(crate) fn validate_application(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    types: &[TypeObjectDigest],
    operands: &[ImplementationOperand],
    scope: Option<DeclarationId>,
) -> Result<(), Diagnostic> {
    let f = function_contract(read, reference)?;
    if !f.read_scope_parameters.is_empty() {
        return Err(reject(
            "scope-bearing functions require complete explicit application admission",
        ));
    }
    if f.implementation_parameters.len() != operands.len() {
        return Err(reject("implementation witness arity mismatch"));
    }
    if types.len() != f.type_parameters.len() {
        return Err(reject(
            "implementation-bearing calls require exact type arguments",
        ));
    }
    let actual_scope = scope.map(|declaration| DeclarationReference {
        package: read.package_id(),
        declaration,
    });
    let mut substitutions = BTreeMap::new();
    for (id, ty) in f.type_parameters.iter().zip(types) {
        read.validation_work()?;
        let parameter = type_parameter_at(read, reference, *id)?;
        if substitutions.insert(*id, *ty).is_some() {
            return Err(reject(
                "application type parameter inventory is not distinct",
            ));
        }
        validate_type_argument_constraints(read, parameter.constraints, *ty, actual_scope)?;
    }
    let mut derived = super::parallel_types::AppliedTypes::new(read);
    for (p, operand) in f.implementation_parameters.iter().zip(operands) {
        let application = witness_contract(read, operand, scope)?;
        if application.contract != p.contract
            || !substituted_equal(
                &mut derived,
                p.self_type,
                application.self_type,
                &substitutions,
            )?
            || application.type_arguments.len() != p.type_arguments.len()
        {
            return Err(reject(
                "witness contract, Self or type argument substitution mismatch",
            ));
        }
        for (template, actual) in p.type_arguments.iter().zip(application.type_arguments) {
            if !substituted_equal(&mut derived, *template, actual, &substitutions)? {
                return Err(reject(
                    "witness exact contract argument substitution mismatch",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_contract(
    read: &(impl ExpressionRead + ?Sized),
    key: OwnerKey,
    c: &OwnedContract,
) -> Result<(), Diagnostic> {
    let OwnerKey::Declaration(declaration) = key else {
        return Err(reject("contract has a foreign identity domain"));
    };
    validate_contract_at(
        read,
        DeclarationReference {
            package: read.package_id(),
            declaration,
        },
        c,
    )
}
fn validate_contract_at(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    c: &OwnedContract,
) -> Result<(), Diagnostic> {
    validate_contract_at_with(read, reference, c, &OwnedAssociatedAdmission::new())
}

pub(crate) fn validate_contract_at_with(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    c: &OwnedContract,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    let generation = defining_occurrence(read, Some(reference))?.consumer_generation;
    let occurrence = OwnedTypeOccurrence::contract(reference, generation);
    if requires_task_borrow_generation(read, c)? {
        require_share_generation_at(read, reference, "borrowed task methods")?;
    }
    for method in &c.methods {
        read.validation_work()?;
        if let FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = &method.effect
        {
            for _ in requirements {
                read.validation_work()?;
            }
            for _ in effect_parameters {
                read.validation_work()?;
            }
        }
    }
    associated.reserve::<AssociatedTypeMemberId>(c.associated_types.len())?;
    associated.reserve::<&Name>(c.associated_types.len())?;
    associated.reserve::<TypeParameterId>(c.type_parameters.len())?;
    associated.reserve::<MethodId>(c.methods.len())?;
    associated.reserve::<&Name>(c.methods.len())?;
    for method in &c.methods {
        reserve_scope_inventory(
            &method.read_scope_parameters,
            &method.read_scope_sources,
            associated,
        )?;
    }
    c.validate_local()?;
    for member in &c.associated_types {
        match &member.bound {
            AssociatedTypeBound::Owned => associated.validate_form(
                read,
                &TypeForm::AssociatedContractMember {
                    contract: reference,
                    member: member.id,
                },
                occurrence.associated,
                generation,
            )?,
            AssociatedTypeBound::ReadViewFamily { .. } => {
                super::owned_associated::read_view_admission::scope_parameters(
                    read,
                    ReadScopeBinder::AssociatedMember {
                        contract: reference,
                        member: member.id,
                    },
                    associated,
                )?;
            }
        }
    }
    associated.reserve::<Name>(1)?;
    associated.reserve::<Name>(c.type_parameters.len())?;
    let mut names = BTreeSet::new();
    for id in type_parameter_ids(c) {
        read.validation_work()?;
        let p = if reference.package == read.package_id() {
            match read.owner(OwnerKey::TypeParameter(id))? {
                Some(OwnerRecord::TypeParameter(p)) => p,
                _ => return Err(reject("missing owned contract parameter")),
            }
        } else {
            match read.package_interface_owner(reference.package, OwnerKey::TypeParameter(id))? {
                Some(PackageInterfaceRecord::TypeParameter(p)) => p,
                _ => return Err(reject("missing imported owned contract parameter")),
            }
        };
        associated.reserve_bytes(p.name.as_str().len())?;
        if p.header.owner != OwnerKey::TypeParameter(id)
            || p.declaration != reference.declaration
            || if id == c.self_parameter {
                p.constraints != TypeParameterConstraints::Owned
            } else {
                p.constraints != TypeParameterConstraints::Owned
                    && !ordinary_constraint(p.constraints)
            }
            || (id != c.self_parameter && p.header.contract_version < 26)
            || !names.insert(p.name)
        {
            return Err(reject(
                "contract requires exact Owned Self and distinct Owned or ordinary-transferable parameters",
            ));
        }
        if ordinary_constraint(p.constraints) {
            if p.header.contract_version < contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION {
                return Err(reject("ordinary contract parameters require Graph 33"));
            }
            require_mixed_generation_at(read, reference, "mixed storage contracts")?;
        }
    }
    associated.reserve::<TypeParameterId>(1)?;
    associated.reserve::<TypeParameterId>(c.type_parameters.len())?;
    let ordinary_assumptions = super::transfer::ordinary_assumptions_in(read, Some(reference))?;
    for method in &c.methods {
        let occurrence = OwnedTypeOccurrence {
            associated: super::owned_associated::AssociatedTypeContext::ContractMethod {
                contract: reference,
                method: method.id,
            },
            ..occurrence
        };
        reserve_scope_inventory(
            &method.read_scope_parameters,
            &method.read_scope_sources,
            associated,
        )?;
        validate_read_scope_inventory_at(
            ReadScopeBinder::ContractMethod {
                contract: reference,
                method: method.id,
            },
            &method.read_scope_parameters,
            &method.read_scope_sources,
            |position| (position as usize) < method.parameters.len(),
        )?;
        associated.validate_occurrence_at(read, &[method.result.ty], occurrence)?;
        let mut suffix = false;
        for parameter in &method.parameters {
            read.validation_work()?;
            associated.validate_occurrence_at(read, &[parameter.ty], occurrence)?;
            let form = read
                .type_object(parameter.ty)?
                .ok_or_else(|| reject("missing exact method parameter type"))?
                .form;
            if form.is_read_view() {
                if suffix || parameter.use_mode != ParameterUse::Borrow {
                    return Err(reject(
                        "read-view method parameters require exact read-only borrow use in the prefix before Owned parameters",
                    ));
                }
            } else if super::memory::direct_at(read, parameter.ty, occurrence, associated)? {
                validate_owned_type_at(read, parameter.ty, occurrence, associated)?;
                suffix = true;
                if parameter.use_mode == ParameterUse::Unrestricted {
                    return Err(reject("owned method parameters require borrow or consume"));
                }
            } else if suffix
                || parameter.use_mode != ParameterUse::Unrestricted
                || !ordinary_with_assumptions_in(
                    read,
                    parameter.ty,
                    Some(reference),
                    &ordinary_assumptions,
                )?
            {
                return Err(reject(
                    "method parameters require scoped ordinary data followed by scoped owned types",
                ));
            }
        }
        let result_form = read
            .type_object(method.result.ty)?
            .ok_or_else(|| reject("missing exact method result type"))?
            .form;
        if result_form.is_read_view() {
            // Complete occurrence admission above checks the exact family/nominal source scope.
        } else if super::memory::direct_at(read, method.result.ty, occurrence, associated)? {
            validate_owned_type_at(read, method.result.ty, occurrence, associated)?;
        } else if !ordinary_with_assumptions_in(
            read,
            method.result.ty,
            Some(reference),
            &ordinary_assumptions,
        )? {
            return Err(reject(
                "method result requires scoped owned or ordinary data type",
            ));
        }
        associated.reserve::<(u32, TypeObjectDigest, ParameterUse)>(method.parameters.len())?;
        let sources = method
            .parameters
            .iter()
            .enumerate()
            .map(|(position, parameter)| {
                Ok((
                    u32::try_from(position).map_err(|_| {
                        reject("method source position exceeds its canonical bound")
                    })?,
                    parameter.ty,
                    parameter.use_mode,
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        super::owned_associated::read_view_admission::validate_result(
            read,
            &method.result,
            &sources,
            &method.read_scope_sources,
            matches!(method.effect, FunctionEffect::Pure),
            occurrence,
            associated,
        )?;
    }
    Ok(())
}

pub(crate) fn ordinary_closed(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
) -> Result<bool, Diagnostic> {
    ordinary_transfer(read, ty, None)
}

/// First-order ordinary data under exact function-local transfer assumptions.
/// Nominal bodies use their own ordinary formal assumptions, while every actual
/// argument, including phantom arguments, retains its enclosing assumptions.
pub(crate) fn ordinary_transfer(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationId>,
) -> Result<bool, Diagnostic> {
    let assumptions = super::transfer::ordinary_assumptions(read, scope)?;
    ordinary_with_assumptions(read, ty, scope, &assumptions)
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum OrdinaryAssumptionContext {
    Closed,
    Function(DeclarationReference),
    Nominal(DeclarationReference),
}

pub(super) fn ordinary_with_assumptions(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationId>,
    assumptions: &BTreeSet<TypeParameterId>,
) -> Result<bool, Diagnostic> {
    ordinary_with_assumptions_in(
        read,
        ty,
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
        assumptions,
    )
}

pub(super) fn ordinary_with_assumptions_in(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    scope: Option<DeclarationReference>,
    assumptions: &BTreeSet<TypeParameterId>,
) -> Result<bool, Diagnostic> {
    ordinary_with_assumptions_at(
        read,
        ty,
        scope.map_or(
            super::owned_associated::TypeParameterContext::Closed,
            super::owned_associated::TypeParameterContext::Authored,
        ),
        assumptions,
    )
}

pub(super) fn ordinary_with_assumptions_at(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    parameters: super::owned_associated::TypeParameterContext,
    assumptions: &BTreeSet<TypeParameterId>,
) -> Result<bool, Diagnostic> {
    // This property depends on ordinary parameter assumptions, not representation
    // identities. Check every actual argument in its caller scope, and prove each
    // nominal body under its own ordinary formal parameters. Recursive applications
    // then form a finite structural proof without overwriting actual substitutions.
    let context = match parameters {
        super::owned_associated::TypeParameterContext::Closed => OrdinaryAssumptionContext::Closed,
        super::owned_associated::TypeParameterContext::Authored(scope) => {
            OrdinaryAssumptionContext::Function(scope)
        }
        super::owned_associated::TypeParameterContext::NominalRepresentation(scope) => {
            OrdinaryAssumptionContext::Nominal(scope)
        }
    };
    read.validation_work()?;
    let mut todo = vec![(ty, context, copy_assumptions(read, assumptions)?)];
    let mut seen = BTreeSet::new();
    while let Some((ty, context, bindings)) = todo.pop() {
        read.validation_work()?;
        if !seen.insert((ty, context, copy_assumptions(read, &bindings)?)) {
            continue;
        }
        let t = read
            .type_object(ty)?
            .ok_or_else(|| reject("missing method ordinary type"))?;
        match &t.form {
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. } => {
                return Err(super::owned_associated::unsupported_semantics());
            }
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => {
                // Read views have a separate affine category; ordinary assumptions
                // cannot grant their scope, source, or field authority.
                return Ok(false);
            }
            TypeForm::TypeParameter { parameter } => {
                if !bindings.contains(parameter) {
                    return Ok(false);
                }
                let declaration = match context {
                    OrdinaryAssumptionContext::Closed => return Ok(false),
                    OrdinaryAssumptionContext::Function(d)
                    | OrdinaryAssumptionContext::Nominal(d) => d,
                };
                read.validation_work()?;
                let record = if matches!(context, OrdinaryAssumptionContext::Nominal(_)) {
                    super::owned_record::parameter(read, declaration, *parameter)?
                } else if declaration.package == read.package_id() {
                    match read.owner(OwnerKey::TypeParameter(*parameter))? {
                        Some(OwnerRecord::TypeParameter(p)) => p,
                        _ => return Err(reject("missing exact ordinary assumption owner")),
                    }
                } else {
                    match read.package_interface_owner(
                        declaration.package,
                        OwnerKey::TypeParameter(*parameter),
                    )? {
                        Some(PackageInterfaceRecord::TypeParameter(p)) => p,
                        _ => return Err(reject("missing imported ordinary assumption owner")),
                    }
                };
                if record.declaration != declaration.declaration
                    || record.header.owner != OwnerKey::TypeParameter(*parameter)
                    || record.constraints.has_owned()
                    || matches!(context, OrdinaryAssumptionContext::Function(_))
                        && !record.constraints.requires_transfer()
                {
                    return Ok(false);
                }
                continue;
            }
            TypeForm::CapabilityResource { .. }
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. }
            | TypeForm::Secret
            | TypeForm::Stream { .. }
            | TypeForm::Function { .. }
            | TypeForm::TaskFunction { .. } => return Ok(false),
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                if super::owned_record::is_owned(read, declaration.package, ty)? {
                    return Ok(false);
                }
                let (parameters, fields, cases) = nominal_members(read, *declaration)?;
                let arguments = match &t.form {
                    TypeForm::Applied { arguments, .. } => arguments.as_slice(),
                    _ => &[],
                };
                if parameters.len() != arguments.len() {
                    return Err(reject("method nominal type arity mismatch"));
                }
                let mut nested = BTreeSet::new();
                for parameter in parameters {
                    read.validation_work()?;
                    let p = super::owned_record::parameter(read, *declaration, parameter)?;
                    if p.declaration != declaration.declaration
                        || p.constraints.has_owned()
                        || p.constraints.requires_transfer()
                        || p.header.owner != OwnerKey::TypeParameter(parameter)
                    {
                        return Ok(false);
                    }
                    read.validation_work()?;
                    nested.insert(parameter);
                }
                for field in fields {
                    read.validation_work()?;
                    let record = super::owned_record::field(
                        read,
                        FieldReference {
                            package: declaration.package,
                            field,
                        },
                    )?;
                    if record.declaration != declaration.declaration {
                        return Err(reject(
                            "ordinary property field belongs to another declaration",
                        ));
                    }
                    let ty = record.ty;
                    todo.push((
                        ty,
                        OrdinaryAssumptionContext::Nominal(*declaration),
                        copy_assumptions(read, &nested)?,
                    ));
                }
                for case in cases {
                    read.validation_work()?;
                    let record = super::owned_record::case(
                        read,
                        CaseReference {
                            package: declaration.package,
                            case,
                        },
                    )?;
                    if record.declaration != declaration.declaration {
                        return Err(reject(
                            "ordinary property case belongs to another declaration",
                        ));
                    }
                    let payload = record.payload;
                    if let Some(ty) = payload {
                        todo.push((
                            ty,
                            OrdinaryAssumptionContext::Nominal(*declaration),
                            copy_assumptions(read, &nested)?,
                        ));
                    }
                }
            }
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::StructuralRecord { .. }
            | TypeForm::List { .. }
            | TypeForm::Map { .. }
            | TypeForm::Option { .. }
            | TypeForm::Result { .. } => {}
        }
        for child in t.child_types() {
            read.validation_work()?;
            todo.push((child, context, copy_assumptions(read, &bindings)?));
        }
    }
    Ok(true)
}
fn copy_assumptions(
    read: &(impl ExpressionRead + ?Sized),
    assumptions: &BTreeSet<TypeParameterId>,
) -> Result<BTreeSet<TypeParameterId>, Diagnostic> {
    for _ in assumptions {
        read.validation_work()?;
    }
    Ok(assumptions.clone())
}

type NominalMembers = (
    Vec<TypeParameterId>,
    Vec<crate::platform::semantic_id::FieldId>,
    Vec<crate::platform::semantic_id::CaseId>,
);
fn nominal_members(
    read: &(impl ExpressionRead + ?Sized),
    d: DeclarationReference,
) -> Result<NominalMembers, Diagnostic> {
    let property = super::owned_record::property_definition(read, d)?;
    if !matches!(property.header.kind, OwnerKind::Record | OwnerKind::Variant) {
        return Err(reject(
            "ordinary nominal properties require a record or variant",
        ));
    }
    Ok((property.parameters, property.fields, property.cases))
}

pub(crate) fn validate_parameters(
    read: &(impl ExpressionRead + ?Sized),
    declaration: DeclarationId,
    f: &FunctionDeclaration,
) -> Result<(), Diagnostic> {
    if f.implementation_parameters.is_empty() {
        return Ok(());
    }
    let associated = OwnedAssociatedAdmission::new();
    let reference = DeclarationReference {
        package: read.package_id(),
        declaration,
    };
    let occurrence = defining_occurrence(read, Some(reference))?;
    if f.implementation_parameters.len() > contract::MAXIMUM_CHILDREN {
        return Err(reject(
            "witness parameters require bounded graph signatures",
        ));
    }
    associated.reserve::<ImplementationParameterId>(f.implementation_parameters.len())?;
    associated.reserve::<&Name>(f.implementation_parameters.len())?;
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for p in &f.implementation_parameters {
        read.validation_work()?;
        if !ids.insert(p.id) || !names.insert(&p.name) {
            return Err(reject("duplicate implementation parameter"));
        }
        let contract = contract_record(read, p.contract)?;
        validate_contract_at_with(read, p.contract, &contract, &associated)?;
        let Some(TypeObject {
            form: TypeForm::TypeParameter { parameter },
            ..
        }) = read.type_object(p.self_type)?
        else {
            return Err(reject(
                "witness Self requires an in-scope owned type parameter",
            ));
        };
        if !super::transfer::function_parameter_listed(read, &f.type_parameters, parameter)? {
            return Err(reject("witness Self escapes generic scope"));
        }
        let Some(OwnerRecord::TypeParameter(t)) = read.owner(OwnerKey::TypeParameter(parameter))?
        else {
            return Err(reject("missing witness Self type parameter"));
        };
        if t.declaration != declaration || !t.constraints.has_owned() {
            return Err(reject(
                "witness Self requires exact owned generic constraint",
            ));
        }
        validate_owned_parameter(
            read,
            parameter,
            Some(DeclarationReference {
                package: read.package_id(),
                declaration,
            }),
        )?;
        validate_contract_arguments_at(
            read,
            p.contract,
            &contract,
            &p.type_arguments,
            occurrence,
            &associated,
        )?;
        if requires_mixed_generation(read, p.contract, &contract.type_parameters)? {
            require_mixed_generation_at(
                read,
                DeclarationReference {
                    package: read.package_id(),
                    declaration,
                },
                "mixed contract prerequisites",
            )?;
        }
    }
    Ok(())
}
