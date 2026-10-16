//! Region identity kind admission. Value custody and lexical liveness are separate proofs.

use super::infer::ExpressionRead;
use super::owned_associated::TypeParameterContext;
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};

#[cfg(test)]
mod tests;

fn reject(message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "kernel_region_kind", message)
}

pub(crate) fn identity(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    context: TypeParameterContext,
) -> Result<bool, Diagnostic> {
    read.validation_work()?;
    let object = read
        .type_object(ty)?
        .ok_or_else(|| reject("region identity type is absent"))?;
    match object.form {
        TypeForm::RegionBrand { callable, binding } => {
            if context != TypeParameterContext::Authored(callable)
                || callable.package != read.package_id()
                || !matches!(read.owner(OwnerKey::Binding(binding))?, Some(OwnerRecord::Binding(b))
                    if b.header.owner == OwnerKey::Binding(binding) && b.kind == BindingKind::Region)
            {
                return Err(reject("region brand is outside its exact lexical callable"));
            }
            Ok(true)
        }
        TypeForm::TypeParameter { parameter } => {
            let Some(scope) = context.scope() else {
                return Err(reject("region kind requires an exact type-parameter scope"));
            };
            let record = match context {
                TypeParameterContext::NominalRepresentation(_) => {
                    super::owned_record::parameter(read, scope, parameter)?
                }
                _ => super::owned_contract::type_parameter_at(read, scope, parameter)?,
            };
            if !record.constraints.is_region() {
                return Ok(false);
            }
            let listed = if scope.package == read.package_id() {
                match read.owner(OwnerKey::Declaration(scope.declaration))? {
                    Some(OwnerRecord::Declaration(d)) => {
                        let mut count = 0;
                        for id in d.payload.type_parameters().iter() {
                            read.validation_work()?;
                            count += usize::from(*id == parameter);
                        }
                        count == 1
                    }
                    _ => false,
                }
            } else {
                match read.package_interface_owner(
                    scope.package,
                    OwnerKey::Declaration(scope.declaration),
                )? {
                    Some(
                        PackageInterfaceRecord::Declaration(d)
                        | PackageInterfaceRecord::RepresentationDeclaration(d),
                    ) => {
                        let parameters = match &d.payload {
                            PackageInterfaceDeclarationPayload::Function(f) => &f.type_parameters,
                            PackageInterfaceDeclarationPayload::Record {
                                type_parameters, ..
                            }
                            | PackageInterfaceDeclarationPayload::Variant {
                                type_parameters, ..
                            }
                            | PackageInterfaceDeclarationPayload::OwnedRecord {
                                type_parameters,
                                ..
                            } => type_parameters,
                            PackageInterfaceDeclarationPayload::ReadView(v) => &v.type_parameters,
                            PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                                &c.type_parameters
                            }
                            PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                                &i.type_parameters
                            }
                            _ => {
                                return Err(reject("region parameter owner has no type inventory"));
                            }
                        };
                        super::transfer::function_parameter_listed(read, parameters, parameter)?
                    }
                    _ => false,
                }
            };
            if !listed {
                return Err(reject(
                    "region parameter is absent from its exact defining inventory",
                ));
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(crate) fn require_identity(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    context: TypeParameterContext,
) -> Result<(), Diagnostic> {
    if identity(read, ty, context)? {
        Ok(())
    } else {
        Err(reject(
            "region position requires a region brand or region-constrained parameter",
        ))
    }
}

pub(crate) fn value_actual(
    read: &(impl ExpressionRead + ?Sized),
    ty: TypeObjectDigest,
    context: TypeParameterContext,
) -> Result<(), Diagnostic> {
    // Exclude the identity kind here; ordinary value validity and declaration
    // scope retain their existing consuming-boundary diagnostics and proofs.
    let object = read
        .type_object(ty)?
        .ok_or_else(|| reject("value type is absent"))?;
    let is_identity = match object.form {
        TypeForm::RegionBrand { .. } => true,
        TypeForm::TypeParameter { parameter } => {
            let package = context
                .scope()
                .map_or(read.package_id(), |scope| scope.package);
            if package == read.package_id() {
                matches!(read.owner(OwnerKey::TypeParameter(parameter))?, Some(OwnerRecord::TypeParameter(p)) if p.constraints.is_region())
            } else if matches!(context, TypeParameterContext::NominalRepresentation(_)) {
                matches!(read.package_representation_owner(package, OwnerKey::TypeParameter(parameter))?, Some(PackageInterfaceRecord::TypeParameter(p)) if p.constraints.is_region())
            } else {
                matches!(read.package_interface_owner(package, OwnerKey::TypeParameter(parameter))?, Some(PackageInterfaceRecord::TypeParameter(p)) if p.constraints.is_region())
            }
        }
        _ => false,
    };
    if is_identity {
        Err(reject(
            "a region identity has no runtime value and cannot fill a value type parameter",
        ))
    } else {
        Ok(())
    }
}
