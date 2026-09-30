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
    pub methods: Vec<OwnedMethod>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedMethod {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<OwnedMethodParameter>,
    pub result: TypeObjectDigest,
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
                || !matches!(method.effect, FunctionEffect::Pure)
                || method.parameters.len() > contract::MAXIMUM_CHILDREN
            {
                return Err(reject(
                    "owned methods require distinct identities/names and pure bounded signatures",
                ));
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
        if self.methods.is_empty()
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
            return Ok(c);
        }
    } else if let Some(PackageInterfaceRecord::Declaration(d)) = read.package_interface_owner(
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? && let PackageInterfaceDeclarationPayload::OwnedContract(c) = d.payload
    {
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

pub(crate) fn substitute_direct(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
) -> Result<TypeObjectDigest, Diagnostic> {
    read.validation_work()?;
    Ok(
        match read
            .type_object(ty)?
            .ok_or_else(|| reject("missing contract type"))?
            .form
        {
            TypeForm::TypeParameter { parameter } => {
                substitutions.get(&parameter).copied().unwrap_or(ty)
            }
            _ => ty,
        },
    )
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
    if !matches!(
        read.type_object(implementation.self_type)?.map(|t| t.form),
        Some(TypeForm::ByteBuffer | TypeForm::OwnedI64Cell)
    ) {
        return Err(reject(
            "implementation Self must be a closed concrete owned primitive",
        ));
    }
    let contract = contract_record(read, implementation.contract)?;
    validate_contract_at(read, implementation.contract, &contract)?;
    if implementation.methods.len() != contract.methods.len() {
        return Err(reject("missing implementation method"));
    }
    let substitutions = BTreeMap::from([(contract.self_parameter, implementation.self_type)]);
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
            || !matches!(f.effect, FunctionEffect::Pure)
            || !matches!(method.effect, FunctionEffect::Pure)
            || f.parameters.len() != method.parameters.len()
        {
            return Err(reject(
                "implementation methods require exact monomorphic pure graph functions",
            ));
        }
        for (id, expected) in f.parameters.iter().zip(&method.parameters) {
            let p = parameter(read, target.function.package, *id)?;
            if p.parent != ParameterParent::Function(target.function.declaration)
                || p.ty != substitute_direct(read, expected.ty, &substitutions)?
                || p.use_mode != expected.use_mode
                || p.resource_requirement.is_some()
            {
                return Err(reject(
                    "implementation parameter type, scope or use mode mismatch",
                ));
            }
        }
        if f.result != substitute_direct(read, method.result, &substitutions)? {
            return Err(reject("implementation result mismatch"));
        }
    }
    Ok(())
}

pub(crate) fn witness_contract(
    read: &(impl ExpressionRead + ?Sized),
    operand: ImplementationOperand,
    scope: Option<DeclarationId>,
) -> Result<(DeclarationReference, TypeObjectDigest), Diagnostic> {
    match operand {
        ImplementationOperand::Concrete { implementation } => {
            let i = implementation_record(read, implementation)?;
            validate_implementation(read, &i)?;
            Ok((i.contract, i.self_type))
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
            Ok((p.contract, p.self_type))
        }
    }
}

pub(crate) fn method_signature(
    read: &(impl ExpressionRead + ?Sized),
    operand: ImplementationOperand,
    reference: DeclarationReference,
    id: MethodId,
    scope: Option<DeclarationId>,
) -> Result<OwnedMethod, Diagnostic> {
    let (actual, self_type) = witness_contract(read, operand, scope)?;
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
    let mut method =
        found.ok_or_else(|| reject("method is absent from the exact witness contract"))?;
    let substitutions = BTreeMap::from([(c.self_parameter, self_type)]);
    for p in &mut method.parameters {
        p.ty = substitute_direct(read, p.ty, &substitutions)?;
    }
    method.result = substitute_direct(read, method.result, &substitutions)?;
    Ok(method)
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
    if types.len() != f.type_parameters.len()
        || !matches!(f.effect, FunctionEffect::Pure)
        || !f.effect_parameters.is_empty()
        || !f.requirement_parameters.is_empty()
    {
        return Err(reject(
            "implementation-bearing calls require first-order pure signatures",
        ));
    }
    for _ in types {
        read.validation_work()?;
    }
    let substitutions = f
        .type_parameters
        .iter()
        .copied()
        .zip(types.iter().copied())
        .collect();
    for (p, operand) in f.implementation_parameters.iter().zip(operands) {
        let (contract, self_type) = witness_contract(read, *operand, scope)?;
        if contract != p.contract
            || self_type != substitute_direct(read, p.self_type, &substitutions)?
        {
            return Err(reject(
                "witness contract or exact Self substitution mismatch",
            ));
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
    for _ in &c.methods {
        read.validation_work()?;
    }
    c.validate_local()?;
    let p = if reference.package == read.package_id() {
        match read.owner(OwnerKey::TypeParameter(c.self_parameter))? {
            Some(OwnerRecord::TypeParameter(p)) => p,
            _ => return Err(reject("missing owned Self parameter")),
        }
    } else {
        match read
            .package_interface_owner(reference.package, OwnerKey::TypeParameter(c.self_parameter))?
        {
            Some(PackageInterfaceRecord::TypeParameter(p)) => p,
            _ => return Err(reject("missing imported owned Self parameter")),
        }
    };
    if p.declaration != reference.declaration || p.constraints != TypeParameterConstraints::Owned {
        return Err(reject("contract requires one exact owned Self parameter"));
    }
    for method in &c.methods {
        let mut suffix = false;
        for parameter in &method.parameters {
            read.validation_work()?;
            let form = read
                .type_object(parameter.ty)?
                .ok_or_else(|| reject("missing method parameter type"))?
                .form;
            if form
                == (TypeForm::TypeParameter {
                    parameter: c.self_parameter,
                })
            {
                suffix = true;
                if parameter.use_mode == ParameterUse::Unrestricted {
                    return Err(reject("Self requires borrow or consume"));
                }
            } else if suffix
                || parameter.use_mode != ParameterUse::Unrestricted
                || !ordinary_closed(read, parameter.ty)?
            {
                return Err(reject(
                    "method parameters require ordinary closed values followed by direct owned Self",
                ));
            }
        }
        if read
            .type_object(method.result)?
            .ok_or_else(|| reject("missing method result type"))?
            .form
            != (TypeForm::TypeParameter {
                parameter: c.self_parameter,
            })
            && !ordinary_closed(read, method.result)?
        {
            return Err(reject(
                "method result requires direct Self or closed ordinary type",
            ));
        }
    }
    Ok(())
}

fn ordinary_closed(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
) -> Result<bool, Diagnostic> {
    // This property depends on ordinary parameter assumptions, not representation
    // identities. Check every actual argument in its caller scope, and prove each
    // nominal body under its own ordinary formal parameters. Recursive applications
    // then form a finite structural proof without overwriting actual substitutions.
    let mut todo = vec![(ty, BTreeSet::<TypeParameterId>::new())];
    let mut seen = BTreeSet::new();
    while let Some((ty, bindings)) = todo.pop() {
        read.validation_work()?;
        if !seen.insert((ty, copy_assumptions(read, &bindings)?)) {
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
                continue;
            }
            TypeForm::CapabilityResource { .. }
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
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
                        || p.constraints == TypeParameterConstraints::Owned
                    {
                        return Ok(false);
                    }
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
                    todo.push((ty, copy_assumptions(read, &nested)?));
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
                        todo.push((ty, copy_assumptions(read, &nested)?));
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
            todo.push((child, copy_assumptions(read, &bindings)?));
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
    if !matches!(f.effect, FunctionEffect::Pure)
        || !f.effect_parameters.is_empty()
        || !f.requirement_parameters.is_empty()
        || f.implementation_parameters.len() > contract::MAXIMUM_CHILDREN
    {
        return Err(reject(
            "witness parameters require bounded first-order pure signatures",
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
        if !f.type_parameters.contains(&parameter) {
            return Err(reject("witness Self escapes generic scope"));
        }
        let Some(OwnerRecord::TypeParameter(t)) = read.owner(OwnerKey::TypeParameter(parameter))?
        else {
            return Err(reject("missing witness Self type parameter"));
        };
        if t.declaration != declaration || t.constraints != TypeParameterConstraints::Owned {
            return Err(reject(
                "witness Self requires exact owned generic constraint",
            ));
        }
    }
    Ok(())
}
