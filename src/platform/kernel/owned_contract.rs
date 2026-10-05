//! First-order nominal owned-method contracts and explicit static implementation operands.
//! These records are canonical meaning, independent of capability and proof witnesses.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::DeclarationId;
use crate::platform::semantic_id::{ImplementationParameterId, MethodId, TypeParameterId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn reject(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_owned_contract", message)
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
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedMethod {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<OwnedMethodParameter>,
    pub result: TypeObjectDigest,
    #[serde(default)]
    pub result_borrow: Option<u32>,
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
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub methods: Vec<OwnedMethodImplementation>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedMethodImplementation {
    pub method: MethodId,
    pub function: DeclarationReference,
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

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImplementationOperand {
    Concrete {
        implementation: DeclarationReference,
    },
    Parameter {
        function: DeclarationReference,
        parameter: ImplementationParameterId,
    },
}

impl OwnedContract {
    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
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
            if let Some(position) = method.result_borrow {
                let source = method.parameters.get(position as usize).ok_or_else(|| {
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
        }
        Ok(())
    }
    pub(crate) fn type_roots(&self) -> Vec<TypeObjectDigest> {
        self.methods
            .iter()
            .flat_map(|m| m.parameters.iter().map(|p| p.ty).chain([m.result]))
            .collect()
    }
}

impl OwnedImplementation {
    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.type_arguments.len() >= contract::MAXIMUM_CHILDREN
            || self.methods.is_empty()
            || self.methods.len() > contract::MAXIMUM_CHILDREN
            || self.methods.windows(2).any(|p| p[0].method >= p[1].method)
        {
            return Err(reject(
                "implementation method mapping must be complete, unique and canonically ordered",
            ));
        }
        Ok(())
    }
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
            return Ok(i);
        }
    } else if let Some(PackageInterfaceRecord::Declaration(d)) = read.package_interface_owner(
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? && let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = d.payload
    {
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
            return Ok(Some(PackageFunctionSignature {
                implementation_parameters: f.implementation_parameters,
                requirement_parameters: f.requirement_parameters,
                effect_parameters: f.effect_parameters,
                type_parameters: f.type_parameters,
                parameters: f.parameters,
                result: f.result,
                result_borrow: f.result_borrow,
                effect: f.effect,
            }));
        }
    } else if let Some(PackageInterfaceRecord::Declaration(d)) = read.package_interface_owner(
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? && let PackageInterfaceDeclarationPayload::Function(f) = d.payload
    {
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
                _ => false,
            },
            _ => false,
        }
    } else {
        match read
            .package_interface_owner(scope.package, OwnerKey::Declaration(scope.declaration))?
        {
            Some(PackageInterfaceRecord::Declaration(d)) => match d.payload {
                PackageInterfaceDeclarationPayload::Function(f) => {
                    super::transfer::function_parameter_listed(read, &f.type_parameters, id)?
                }
                PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                    c.self_parameter == id
                        || super::transfer::function_parameter_listed(read, &c.type_parameters, id)?
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

fn validate_owned_type(
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
        _ => Err(reject("owned contract arguments require exact Owned types")),
    }
}

fn validate_contract_arguments(
    read: &(impl ExpressionRead + ?Sized),
    contract: &OwnedContract,
    arguments: &[TypeObjectDigest],
    scope: Option<DeclarationReference>,
) -> Result<(), Diagnostic> {
    if arguments.len() != contract.type_parameters.len() {
        return Err(reject("owned contract type argument arity mismatch"));
    }
    // Every argument is an obligation, even when no method mentions its formal.
    for argument in arguments {
        validate_owned_type(read, *argument, scope)?;
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
            .chain([method.result])
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
    for _ in &implementation.methods {
        read.validation_work()?;
    }
    implementation.validate_local()?;
    validate_owned_type(read, implementation.self_type, None)?;
    let contract = contract_record(read, implementation.contract)?;
    validate_contract_at(read, implementation.contract, &contract)?;
    validate_contract_arguments(read, &contract, &implementation.type_arguments, None)?;
    if implementation.methods.len() != contract.methods.len() {
        return Err(reject("missing implementation method"));
    }
    let substitutions = contract_bindings(
        &contract,
        implementation.self_type,
        &implementation.type_arguments,
    );
    let mut derived = super::parallel_types::AppliedTypes::new(read);
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
        let f = function_contract(read, target.function)?;
        if !f.type_parameters.is_empty()
            || !f.effect_parameters.is_empty()
            || !f.requirement_parameters.is_empty()
            || !f.implementation_parameters.is_empty()
            || f.effect != method.effect
            || f.parameters.len() != method.parameters.len()
        {
            return Err(reject(
                "implementation methods require monomorphic graph functions with exact callable kind and effect row",
            ));
        }
        for (id, expected) in f.parameters.iter().zip(&method.parameters) {
            let p = parameter(read, target.function.package, *id)?;
            if p.parent != ParameterParent::Function(target.function.declaration)
                || !substituted_equal(&mut derived, expected.ty, p.ty, &substitutions)?
                || p.use_mode != expected.use_mode
                || p.resource_requirement.is_some()
            {
                return Err(reject(
                    "implementation parameter type, scope or use mode mismatch",
                ));
            }
        }
        if !substituted_equal(&mut derived, method.result, f.result, &substitutions)? {
            return Err(reject("implementation result mismatch"));
        }
        let expected_source = method
            .result_borrow
            .map(|position| {
                f.parameters
                    .get(position as usize)
                    .copied()
                    .ok_or_else(|| reject("implementation borrowed result source is out of range"))
            })
            .transpose()?;
        if f.result_borrow != expected_source {
            return Err(reject("implementation borrowed result source mismatch"));
        }
    }
    Ok(())
}

pub(crate) fn witness_contract(
    read: &(impl ExpressionRead + ?Sized),
    operand: ImplementationOperand,
    scope: Option<DeclarationId>,
) -> Result<
    (
        DeclarationReference,
        TypeObjectDigest,
        Vec<TypeObjectDigest>,
    ),
    Diagnostic,
> {
    match operand {
        ImplementationOperand::Concrete { implementation } => {
            let i = implementation_record(read, implementation)?;
            validate_implementation(read, &i)?;
            Ok((i.contract, i.self_type, i.type_arguments))
        }
        ImplementationOperand::Parameter {
            function,
            parameter,
        } => {
            if function.package != read.package_id() || Some(function.declaration) != scope {
                return Err(reject(
                    "implementation parameter escaped its exact function scope",
                ));
            }
            let f = function_contract(read, function)?;
            let mut found = None;
            for p in &f.implementation_parameters {
                read.validation_work()?;
                if p.id == parameter {
                    found = Some(p);
                    break;
                }
            }
            let p = found.ok_or_else(|| reject("missing implementation parameter"))?;
            let contract = contract_record(read, p.contract)?;
            validate_contract_at(read, p.contract, &contract)?;
            if !matches!(
                read.type_object(p.self_type)?.map(|t| t.form),
                Some(TypeForm::TypeParameter { .. })
            ) {
                return Err(reject(
                    "witness Self requires an in-scope owned type parameter",
                ));
            }
            validate_owned_type(read, p.self_type, Some(function))?;
            validate_contract_arguments(read, &contract, &p.type_arguments, Some(function))?;
            Ok((p.contract, p.self_type, p.type_arguments.clone()))
        }
    }
}

pub(crate) fn method_signature(
    read: &(impl ExpressionRead + ?Sized),
    operand: ImplementationOperand,
    reference: DeclarationReference,
    id: MethodId,
    scope: Option<DeclarationId>,
) -> Result<(OwnedMethod, BTreeMap<TypeParameterId, TypeObjectDigest>), Diagnostic> {
    let (actual, self_type, arguments) = witness_contract(read, operand, scope)?;
    if actual != reference {
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
    let method = found.ok_or_else(|| reject("method is absent from the exact witness contract"))?;
    let substitutions = contract_bindings(&c, self_type, &arguments);
    Ok((method, substitutions))
}

pub(crate) fn validate_application(
    read: &(impl ExpressionRead + ?Sized),
    reference: DeclarationReference,
    types: &[TypeObjectDigest],
    operands: &[ImplementationOperand],
    scope: Option<DeclarationId>,
) -> Result<(), Diagnostic> {
    let f = function_contract(read, reference)?;
    if f.implementation_parameters.len() != operands.len() {
        return Err(reject("implementation witness arity mismatch"));
    }
    if operands.is_empty() {
        return Ok(());
    }
    if types.len() != f.type_parameters.len() {
        return Err(reject(
            "implementation-bearing calls require exact type arguments",
        ));
    }
    for _ in types {
        read.validation_work()?;
    }
    let substitutions: BTreeMap<_, _> = f
        .type_parameters
        .iter()
        .copied()
        .zip(types.iter().copied())
        .collect();
    let mut derived = super::parallel_types::AppliedTypes::new(read);
    for (p, operand) in f.implementation_parameters.iter().zip(operands) {
        let (contract, self_type, arguments) = witness_contract(read, *operand, scope)?;
        if contract != p.contract
            || !substituted_equal(&mut derived, p.self_type, self_type, &substitutions)?
            || arguments.len() != p.type_arguments.len()
        {
            return Err(reject(
                "witness contract, Self or type argument substitution mismatch",
            ));
        }
        for (template, actual) in p.type_arguments.iter().zip(arguments) {
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
    c.validate_local()?;
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
        if p.header.owner != OwnerKey::TypeParameter(id)
            || p.declaration != reference.declaration
            || p.constraints != TypeParameterConstraints::Owned
            || (id != c.self_parameter && p.header.contract_version < 26)
            || !names.insert(p.name)
        {
            return Err(reject(
                "contract requires distinct exact Owned parameter owners",
            ));
        }
    }
    for method in &c.methods {
        let mut suffix = false;
        for parameter in &method.parameters {
            read.validation_work()?;
            if super::memory::direct_in(read, reference.package, parameter.ty)? {
                validate_owned_type(read, parameter.ty, Some(reference))?;
                suffix = true;
                if parameter.use_mode == ParameterUse::Unrestricted {
                    return Err(reject("owned method parameters require borrow or consume"));
                }
                if !matches!(method.effect, FunctionEffect::Pure)
                    && parameter.use_mode != ParameterUse::Consume
                {
                    return Err(reject(
                        "task method owned parameters must consume ownership",
                    ));
                }
            } else if suffix
                || parameter.use_mode != ParameterUse::Unrestricted
                || !ordinary_closed(read, parameter.ty)?
            {
                return Err(reject(
                    "method parameters require ordinary closed values followed by scoped owned types",
                ));
            }
        }
        if super::memory::direct_in(read, reference.package, method.result)? {
            validate_owned_type(read, method.result, Some(reference))?;
        } else if !ordinary_closed(read, method.result)? {
            return Err(reject(
                "method result requires scoped owned or closed ordinary type",
            ));
        }
        if let Some(position) = method.result_borrow {
            let source = method.parameters.get(position as usize).ok_or_else(|| {
                reject("borrowed method result source is outside its parameter inventory")
            })?;
            if !matches!(method.effect, FunctionEffect::Pure)
                || !super::memory::direct_in(read, reference.package, method.result)?
                || !super::memory::direct_in(read, reference.package, source.ty)?
                || source.use_mode != ParameterUse::Borrow
            {
                return Err(reject(
                    "borrowed method result requires a pure method, direct owned result and exact borrowed source",
                ));
            }
        }
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
    // This property depends on ordinary parameter assumptions, not representation
    // identities. Check every actual argument in its caller scope, and prove each
    // nominal body under its own ordinary formal parameters. Recursive applications
    // then form a finite structural proof without overwriting actual substitutions.
    let context = scope.map_or(OrdinaryAssumptionContext::Closed, |declaration| {
        OrdinaryAssumptionContext::Function(DeclarationReference {
            package: read.package_id(),
            declaration,
        })
    });
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
                let record = if declaration.package == read.package_id() {
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
                    let p = if declaration.package == read.package_id() {
                        match read.owner(OwnerKey::TypeParameter(parameter))? {
                            Some(OwnerRecord::TypeParameter(p)) => p,
                            _ => return Err(reject("missing ordinary nominal parameter")),
                        }
                    } else {
                        match read.package_interface_owner(
                            declaration.package,
                            OwnerKey::TypeParameter(parameter),
                        )? {
                            Some(PackageInterfaceRecord::TypeParameter(p)) => p,
                            _ => return Err(reject("missing imported ordinary nominal parameter")),
                        }
                    };
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
                    let ty = if declaration.package == read.package_id() {
                        match read.owner(OwnerKey::Field(field))? {
                            Some(OwnerRecord::Field(f))
                                if f.declaration == declaration.declaration =>
                            {
                                f.ty
                            }
                            _ => return Err(reject("missing method nominal field")),
                        }
                    } else {
                        match read
                            .package_interface_owner(declaration.package, OwnerKey::Field(field))?
                        {
                            Some(PackageInterfaceRecord::Field(f))
                                if f.declaration == declaration.declaration =>
                            {
                                f.ty
                            }
                            _ => return Err(reject("missing imported method nominal field")),
                        }
                    };
                    todo.push((
                        ty,
                        OrdinaryAssumptionContext::Nominal(*declaration),
                        copy_assumptions(read, &nested)?,
                    ));
                }
                for case in cases {
                    read.validation_work()?;
                    let payload = if declaration.package == read.package_id() {
                        match read.owner(OwnerKey::Case(case))? {
                            Some(OwnerRecord::Case(c))
                                if c.declaration == declaration.declaration =>
                            {
                                c.payload
                            }
                            _ => return Err(reject("missing method nominal case")),
                        }
                    } else {
                        match read
                            .package_interface_owner(declaration.package, OwnerKey::Case(case))?
                        {
                            Some(PackageInterfaceRecord::Case(c))
                                if c.declaration == declaration.declaration =>
                            {
                                c.payload
                            }
                            _ => return Err(reject("missing imported method nominal case")),
                        }
                    };
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
    admit_dependency(read, d.package)?;
    if d.package == read.package_id() {
        match read.owner(OwnerKey::Declaration(d.declaration))? {
            Some(OwnerRecord::Declaration(r)) => match r.payload {
                DeclarationPayload::Record {
                    type_parameters,
                    fields,
                } => Ok((type_parameters, fields, vec![])),
                DeclarationPayload::Variant {
                    type_parameters,
                    cases,
                } => Ok((type_parameters, vec![], cases)),
                _ => Err(reject("method nominal type requires a record or variant")),
            },
            _ => Err(reject("missing method nominal declaration")),
        }
    } else {
        match read.package_interface_owner(d.package, OwnerKey::Declaration(d.declaration))? {
            Some(PackageInterfaceRecord::Declaration(r)) => match r.payload {
                PackageInterfaceDeclarationPayload::Record {
                    type_parameters,
                    fields,
                } => Ok((type_parameters, fields, vec![])),
                PackageInterfaceDeclarationPayload::Variant {
                    type_parameters,
                    cases,
                } => Ok((type_parameters, vec![], cases)),
                _ => Err(reject(
                    "imported method nominal type requires a record or variant",
                )),
            },
            _ => Err(reject("missing imported method nominal declaration")),
        }
    }
}

pub(crate) fn validate_parameters(
    read: &(impl ExpressionRead + ?Sized),
    declaration: DeclarationId,
    f: &FunctionDeclaration,
) -> Result<(), Diagnostic> {
    if f.implementation_parameters.is_empty() {
        return Ok(());
    }
    if f.implementation_parameters.len() > contract::MAXIMUM_CHILDREN {
        return Err(reject(
            "witness parameters require bounded graph signatures",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for p in &f.implementation_parameters {
        read.validation_work()?;
        if !ids.insert(p.id) || !names.insert(&p.name) {
            return Err(reject("duplicate implementation parameter"));
        }
        let contract = contract_record(read, p.contract)?;
        validate_contract_at(read, p.contract, &contract)?;
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
        validate_contract_arguments(
            read,
            &contract,
            &p.type_arguments,
            Some(DeclarationReference {
                package: read.package_id(),
                declaration,
            }),
        )?;
    }
    Ok(())
}
