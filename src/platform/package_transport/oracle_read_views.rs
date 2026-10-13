//! Independent interpretation of neutral canonical view/scope/result inventories.
//! No transport/interface admission, projector, source profile gate or prepared data is read.

use super::{Reader, failure};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::*;
use std::collections::{BTreeMap, BTreeSet};

struct Interpretation<'a, 'r> {
    sources: &'a BTreeMap<PackageId, KernelSnapshot>,
    public: &'a BTreeMap<PackageId, BTreeMap<OwnerKey, PackageInterfaceRecord>>,
    revisions: &'a BTreeMap<PackageId, super::PackageRevision>,
    reader: &'a Reader<'r>,
}

fn view_scope(form: &TypeForm) -> Option<ReadScopeArgument> {
    match form {
        TypeForm::ReadViewApplied { read_scope, .. }
        | TypeForm::AssociatedReadViewMember { read_scope, .. }
        | TypeForm::AssociatedReadViewProjection { read_scope, .. } => Some(*read_scope),
        _ => None,
    }
}

fn capacity(message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "package_closure_oracle_read_view_capacity",
        message,
    )
}

impl Interpretation<'_, '_> {
    fn ty(&self, package: PackageId, digest: TypeObjectDigest) -> Result<&TypeObject, Diagnostic> {
        self.reader.charge(1)?;
        let source = self
            .sources
            .get(&package)
            .ok_or_else(|| failure("view oracle type package is absent"))?;
        source
            .types
            .get(&digest)
            .or_else(|| source.dependency_types.get(&digest))
            .ok_or_else(|| failure("view oracle canonical type closure is incomplete"))
    }

    fn declaration(
        &self,
        source: PackageId,
        target: DeclarationReference,
    ) -> Result<PackageInterfaceDeclarationPayload, Diagnostic> {
        self.reader.charge(1)?;
        if source == target.package {
            let Some(OwnerRecord::Declaration(record)) = self.sources[&source]
                .owners
                .get(&OwnerKey::Declaration(target.declaration))
            else {
                return Err(failure("view oracle canonical declaration is absent"));
            };
            match &record.payload {
                DeclarationPayload::ReadView(view) => self
                    .reader
                    .charge(1 + view.fields.len() + view.type_parameters.len())?,
                DeclarationPayload::OwnedContract(contract) => {
                    self.reader.charge(
                        contract.associated_types.len()
                            + contract.type_parameters.len()
                            + contract.methods.len(),
                    )?;
                    for method in &contract.methods {
                        self.reader.charge(
                            method.parameters.len()
                                + method.read_scope_parameters.len()
                                + method.read_scope_sources.len(),
                        )?;
                    }
                }
                DeclarationPayload::OwnedRecord {
                    fields,
                    type_parameters,
                }
                | DeclarationPayload::Record {
                    fields,
                    type_parameters,
                } => self.reader.charge(fields.len() + type_parameters.len())?,
                DeclarationPayload::Variant {
                    cases,
                    type_parameters,
                } => self.reader.charge(cases.len() + type_parameters.len())?,
                _ => {}
            }
            return match &record.payload {
                DeclarationPayload::ReadView(view) => {
                    Ok(PackageInterfaceDeclarationPayload::ReadView(view.clone()))
                }
                DeclarationPayload::OwnedContract(contract) => Ok(
                    PackageInterfaceDeclarationPayload::OwnedContract(contract.clone()),
                ),
                DeclarationPayload::OwnedRecord {
                    fields,
                    type_parameters,
                } => Ok(PackageInterfaceDeclarationPayload::OwnedRecord {
                    fields: fields.clone(),
                    type_parameters: type_parameters.clone(),
                }),
                DeclarationPayload::Record {
                    fields,
                    type_parameters,
                } => Ok(PackageInterfaceDeclarationPayload::Record {
                    fields: fields.clone(),
                    type_parameters: type_parameters.clone(),
                }),
                DeclarationPayload::Variant {
                    cases,
                    type_parameters,
                } => Ok(PackageInterfaceDeclarationPayload::Variant {
                    cases: cases.clone(),
                    type_parameters: type_parameters.clone(),
                }),
                _ => Err(failure(
                    "view oracle nominal/member reference names another declaration kind",
                )),
            };
        }
        self.reader
            .charge(self.revisions[&source].dependencies.len())?;
        if !self.revisions[&source]
            .dependencies
            .iter()
            .any(|dependency| dependency.package == target.package)
        {
            return Err(failure(
                "view oracle public reference is outside exact direct dependencies",
            ));
        }
        let Some(
            PackageInterfaceRecord::Declaration(record)
            | PackageInterfaceRecord::RepresentationDeclaration(record),
        ) = self
            .public
            .get(&target.package)
            .and_then(|owners| owners.get(&OwnerKey::Declaration(target.declaration)))
        else {
            return Err(failure(
                "view oracle public/representation nominal inventory is absent",
            ));
        };
        match &record.payload {
            PackageInterfaceDeclarationPayload::ReadView(view) => self
                .reader
                .charge(1 + view.fields.len() + view.type_parameters.len())?,
            PackageInterfaceDeclarationPayload::OwnedContract(contract) => {
                self.reader.charge(
                    contract.associated_types.len()
                        + contract.type_parameters.len()
                        + contract.methods.len(),
                )?;
                for method in &contract.methods {
                    self.reader.charge(
                        method.parameters.len()
                            + method.read_scope_parameters.len()
                            + method.read_scope_sources.len(),
                    )?;
                }
            }
            PackageInterfaceDeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            }
            | PackageInterfaceDeclarationPayload::Record {
                fields,
                type_parameters,
            } => self.reader.charge(fields.len() + type_parameters.len())?,
            PackageInterfaceDeclarationPayload::Variant {
                cases,
                type_parameters,
            } => self.reader.charge(cases.len() + type_parameters.len())?,
            _ => {
                return Err(failure(
                    "view oracle nominal/member reference names another public declaration kind",
                ));
            }
        }
        Ok(record.payload.clone())
    }

    fn owned(&self, package: PackageId, digest: TypeObjectDigest) -> Result<bool, Diagnostic> {
        Ok(match &self.ty(package, digest)?.form {
            TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. } => true,
            TypeForm::TypeParameter { parameter } => {
                matches!(self.sources[&package].owners.get(&OwnerKey::TypeParameter(*parameter)),
                Some(OwnerRecord::TypeParameter(record)) if record.constraints.has_owned())
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => matches!(
                self.declaration(package, *declaration)?,
                PackageInterfaceDeclarationPayload::OwnedRecord { .. }
            ),
            TypeForm::AssociatedContractMember { contract, member }
            | TypeForm::AssociatedWitnessProjection {
                contract, member, ..
            } => {
                let PackageInterfaceDeclarationPayload::OwnedContract(contract) =
                    self.declaration(package, *contract)?
                else {
                    return Err(failure(
                        "view oracle Owned projection has no exact contract",
                    ));
                };
                self.reader.charge(contract.associated_types.len())?;
                contract.associated_types.iter().any(|entry| {
                    entry.id == *member && matches!(entry.bound, AssociatedTypeBound::Owned)
                })
            }
            _ => false,
        })
    }

    fn formal(
        &self,
        actual: ReadScopeArgument,
        binder: ReadScopeBinder,
        formals: &[ReadScopeParameter],
    ) -> Result<(), Diagnostic> {
        self.reader.charge(1 + formals.len())?;
        let ReadScopeArgument::Parameter(reference) = actual else {
            return Err(failure("view oracle signature imports a lexical scope"));
        };
        if reference.binder != binder
            || formals
                .iter()
                .filter(|formal| formal.id == reference.parameter)
                .count()
                != 1
        {
            return Err(failure(
                "view oracle scope formal captures a foreign or absent binder coordinate",
            ));
        }
        Ok(())
    }

    fn occurrences(
        &self,
        package: PackageId,
        ty: TypeObjectDigest,
        root: bool,
        binder: ReadScopeBinder,
        formals: &[ReadScopeParameter],
        projection_free: bool,
    ) -> Result<(), Diagnostic> {
        self.reader.charge(1)?;
        let mut pending = vec![(ty, root, 0_usize)];
        while let Some((ty, permitted, depth)) = pending.pop() {
            if depth > contract::MAXIMUM_TYPE_DEPTH {
                return Err(capacity("view oracle finite type capacity exceeded"));
            }
            let object = self.ty(package, ty)?;
            if projection_free && object.form.is_associated() {
                return Err(failure(
                    "view oracle stored RHS retains associated authority",
                ));
            }
            if let Some(actual) = view_scope(&object.form) {
                if !permitted {
                    return Err(failure(
                        "view oracle hidden view occurs in container/callable/phantom position",
                    ));
                }
                self.formal(actual, binder, formals)?;
                if let TypeForm::AssociatedReadViewProjection {
                    scope,
                    parameter,
                    contract,
                    ..
                } = &object.form
                {
                    if *scope != binder.declaration() {
                        return Err(failure(
                            "view oracle projection imports another lexical witness binder",
                        ));
                    }
                    let Some(OwnerRecord::Declaration(declaration)) = self.sources[&package]
                        .owners
                        .get(&OwnerKey::Declaration(scope.declaration))
                    else {
                        return Err(failure("view oracle projection binder is absent"));
                    };
                    let parameters = match &declaration.payload {
                        DeclarationPayload::Function(function) => {
                            &function.implementation_parameters
                        }
                        DeclarationPayload::OwnedImplementation(implementation) => {
                            &implementation.implementation_parameters
                        }
                        _ => {
                            return Err(failure(
                                "view oracle projection binder has no lexical witness inventory",
                            ));
                        }
                    };
                    self.reader.charge(parameters.len())?;
                    let witness = parameters
                        .iter()
                        .find(|witness| witness.id == *parameter)
                        .ok_or_else(|| {
                            failure("view oracle projection has no exact lexical witness formal")
                        })?;
                    if witness.contract != *contract || !self.owned(package, witness.self_type)? {
                        return Err(failure(
                            "view oracle projection changes its exact witness contract/Self authority",
                        ));
                    }
                    let PackageInterfaceDeclarationPayload::OwnedContract(contract) =
                        self.declaration(package, *contract)?
                    else {
                        return Err(failure("view oracle witness contract has wrong kind"));
                    };
                    if witness.type_arguments.len() != contract.type_parameters.len() {
                        return Err(failure(
                            "view oracle witness contract argument inventory has wrong complete arity",
                        ));
                    }
                }
                if let TypeForm::AssociatedReadViewMember { contract, .. } = &object.form {
                    if *contract != binder.declaration()
                        || !matches!(
                            binder,
                            ReadScopeBinder::ContractMethod { .. }
                                | ReadScopeBinder::AssociatedMember { .. }
                        )
                    {
                        return Err(failure(
                            "view oracle abstract member escaped its exact contract coordinate",
                        ));
                    }
                }
                match &object.form {
                    TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        ..
                    } => {
                        let PackageInterfaceDeclarationPayload::ReadView(view) =
                            self.declaration(package, *declaration)?
                        else {
                            return Err(failure(
                                "view oracle nominal application has wrong declaration kind",
                            ));
                        };
                        if arguments.len() != view.type_parameters.len() {
                            return Err(failure(
                                "view oracle nominal view actual inventory has wrong arity",
                            ));
                        }
                        self.reader.charge(arguments.len())?;
                        for (formal, actual) in view.type_parameters.iter().zip(arguments) {
                            let parameter = if declaration.package == package {
                                match self.sources[&package]
                                    .owners
                                    .get(&OwnerKey::TypeParameter(*formal))
                                {
                                    Some(OwnerRecord::TypeParameter(parameter)) => parameter,
                                    _ => {
                                        return Err(failure(
                                            "view oracle nominal formal is absent",
                                        ));
                                    }
                                }
                            } else {
                                match self.public[&declaration.package]
                                    .get(&OwnerKey::TypeParameter(*formal))
                                {
                                    Some(PackageInterfaceRecord::TypeParameter(parameter)) => {
                                        parameter
                                    }
                                    _ => {
                                        return Err(failure(
                                            "view oracle foreign nominal formal is absent",
                                        ));
                                    }
                                }
                            };
                            if parameter.declaration != declaration.declaration {
                                return Err(failure(
                                    "view oracle nominal formal captures another declaration",
                                ));
                            }
                            if parameter.constraints.has_owned() {
                                if !self.owned(package, *actual)? {
                                    return Err(failure(
                                        "view oracle actual violates exact Owned nominal bound",
                                    ));
                                }
                                if parameter.constraints.requires_transfer()
                                    || parameter.constraints.requires_share()
                                {
                                    self.permissions(
                                        package,
                                        binder.declaration(),
                                        *actual,
                                        parameter.constraints.requires_transfer(),
                                        parameter.constraints.requires_share(),
                                    )?;
                                }
                            } else if parameter.constraints.requires_transfer() {
                                self.metadata(package, binder.declaration(), *actual)?;
                            } else {
                                return Err(failure(
                                    "view oracle nominal formal has no explicit data/Owned category",
                                ));
                            }
                        }
                    }
                    TypeForm::AssociatedReadViewMember {
                        contract, member, ..
                    }
                    | TypeForm::AssociatedReadViewProjection {
                        contract, member, ..
                    } => {
                        let PackageInterfaceDeclarationPayload::OwnedContract(contract) =
                            self.declaration(package, *contract)?
                        else {
                            return Err(failure("view oracle family has no exact contract"));
                        };
                        self.reader.charge(contract.associated_types.len())?;
                        if !contract.associated_types.iter().any(|entry| {
                            entry.id == *member
                                && matches!(entry.bound, AssociatedTypeBound::ReadViewFamily { .. })
                        }) {
                            return Err(failure(
                                "view oracle family coordinate names wrong member kind/arity",
                            ));
                        }
                    }
                    _ => unreachable!("scope is only on view forms"),
                }
            }
            self.reader.charge(object.child_type_count())?;
            pending
                .try_reserve(object.child_type_count())
                .map_err(|_| failure("view oracle child reservation failed"))?;
            pending.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|ty| (ty, false, depth + 1)),
            );
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn signature<S: Copy + Ord>(
        &self,
        package: PackageId,
        binder: ReadScopeBinder,
        parameters: &[(S, TypeObjectDigest, ParameterUse)],
        result: &CallableResult<S>,
        effect: &FunctionEffect,
        formals: &[ReadScopeParameter],
        sources: &[ReadScopeSource<S>],
    ) -> Result<(), Diagnostic> {
        let public_signature = matches!(self.sources[&package].owners.get(&OwnerKey::Declaration(binder.declaration().declaration)),
            Some(OwnerRecord::Declaration(declaration)) if declaration.visibility == DeclarationVisibility::Public && matches!(declaration.payload, DeclarationPayload::Function(_)));
        if public_signature {
            self.reader.charge(parameters.len() + 1)?;
            for ty in parameters.iter().map(|(_, ty, _)| *ty).chain([result.ty]) {
                if let TypeForm::ReadViewApplied { declaration, .. } = self.ty(package, ty)?.form {
                    let private = if declaration.package == package {
                        matches!(self.sources[&package].owners.get(&OwnerKey::Declaration(declaration.declaration)),
                            Some(OwnerRecord::Declaration(record)) if record.visibility != DeclarationVisibility::Public)
                    } else {
                        matches!(
                            self.public
                                .get(&declaration.package)
                                .and_then(|inventory| inventory
                                    .get(&OwnerKey::Declaration(declaration.declaration))),
                            Some(PackageInterfaceRecord::RepresentationDeclaration(_))
                        )
                    };
                    if private {
                        return Err(failure(
                            "view oracle direct public signature exposes private nominal representation",
                        ));
                    }
                }
            }
        }
        if !formals.is_empty()
            || !sources.is_empty()
            || matches!(result.relation, ResultRelation::ViewFrom { .. })
        {
            let Some(OwnerRecord::Declaration(declaration)) = self.sources[&package]
                .owners
                .get(&OwnerKey::Declaration(binder.declaration().declaration))
            else {
                return Err(failure("view oracle scoped signature owner is absent"));
            };
            if declaration.header.contract_version != contract::GRAPH_CONTRACT_VERSION
                || self.sources[&package].root.graph_contract_version
                    != contract::GRAPH_CONTRACT_VERSION
                || self.revisions[&package].revision.graph_contract_version
                    != contract::GRAPH_CONTRACT_VERSION
            {
                return Err(failure(
                    "view oracle scoped signature uses a historical consuming profile",
                ));
            }
        }
        self.reader
            .charge(formals.len() + sources.len() + parameters.len())?;
        let ids: BTreeSet<_> = formals.iter().map(|formal| formal.id).collect();
        let names: BTreeSet<_> = formals.iter().map(|formal| &formal.name).collect();
        if ids.len() != formals.len() || names.len() != formals.len() {
            return Err(failure(
                "view oracle formal inventory has duplicate names/identities",
            ));
        }
        let mut relations = BTreeSet::new();
        for relationship in sources {
            self.formal(relationship.scope, binder, formals)?;
            if !relations.insert((relationship.source, relationship.scope)) {
                return Err(failure("view oracle duplicate source/scope relationship"));
            }
            let (_, ty, mode) = parameters
                .iter()
                .find(|(id, _, _)| *id == relationship.source)
                .ok_or_else(|| {
                    failure("view oracle scope source is outside complete ordered parameters")
                })?;
            if *mode != ParameterUse::Borrow
                || (!self.owned(package, *ty)?
                    && view_scope(&self.ty(package, *ty)?.form).is_none())
            {
                return Err(failure(
                    "view oracle scope source lacks independently borrowed Owned/view rights",
                ));
            }
        }
        for (_, ty, mode) in parameters {
            let is_view = view_scope(&self.ty(package, *ty)?.form).is_some();
            if is_view && *mode != ParameterUse::Borrow {
                return Err(failure(
                    "view oracle view parameter has consuming/copy mode",
                ));
            }
            self.occurrences(package, *ty, is_view, binder, formals, false)?;
        }
        self.occurrences(
            package,
            result.ty,
            matches!(result.relation, ResultRelation::ViewFrom { .. }),
            binder,
            formals,
            false,
        )?;
        let Some(source) = result.relation.source() else {
            return Ok(());
        };
        let (_, source_ty, mode) = parameters
            .iter()
            .find(|(id, _, _)| id == source)
            .ok_or_else(|| {
                failure("view oracle result source is outside complete ordered parameters")
            })?;
        if !matches!(effect, FunctionEffect::Pure) || *mode != ParameterUse::Borrow {
            return Err(failure(
                "view oracle loan result is not pure and source-borrowed",
            ));
        }
        let source_scope = view_scope(&self.ty(package, *source_ty)?.form);
        if source_scope.is_none() && !self.owned(package, *source_ty)? {
            return Err(failure(
                "view oracle loan source has no Owned/view category",
            ));
        }
        match &result.relation {
            ResultRelation::ReadFrom { .. } => {
                if !self.owned(package, result.ty)? {
                    return Err(failure("view oracle ReadFrom result is not Owned"));
                }
            }
            ResultRelation::ViewFrom { source, scope } => {
                self.formal(*scope, binder, formals)?;
                if view_scope(&self.ty(package, result.ty)?.form) != Some(*scope)
                    || (source_scope != Some(*scope) && !relations.contains(&(*source, *scope)))
                {
                    return Err(failure(
                        "view oracle ViewFrom type/scope/source relationship differs",
                    ));
                }
            }
            ResultRelation::Value => unreachable!("ordinary results have no source"),
        }
        Ok(())
    }

    fn metadata(
        &self,
        package: PackageId,
        binder: DeclarationReference,
        ty: TypeObjectDigest,
    ) -> Result<(), Diagnostic> {
        type Environment = BTreeMap<
            (
                DeclarationReference,
                crate::platform::semantic_id::TypeParameterId,
            ),
            (PackageId, DeclarationReference, TypeObjectDigest),
        >;
        let mut pending = vec![(package, binder, ty, 0_usize, Environment::new())];
        let mut seen = BTreeSet::new();
        while let Some((package, binder, ty, depth, environment)) = pending.pop() {
            if depth > contract::MAXIMUM_TYPE_DEPTH {
                return Err(capacity("view oracle metadata capacity exceeded"));
            }
            self.reader.charge(environment.len())?;
            if !seen.insert((package, binder, ty, environment.clone())) {
                continue;
            }
            let object = self.ty(package, ty)?;
            let nominal = match &object.form {
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
                | TypeForm::Result { .. } => None,
                TypeForm::TypeParameter { parameter } => {
                    if let Some((package, binder, ty)) =
                        environment.get(&(binder, *parameter)).copied()
                    {
                        self.reader.charge(1)?;
                        pending.push((package, binder, ty, depth + 1, environment));
                        continue;
                    }
                    let Some(OwnerRecord::TypeParameter(record)) = self.sources[&package]
                        .owners
                        .get(&OwnerKey::TypeParameter(*parameter))
                    else {
                        return Err(failure("view oracle metadata formal is absent"));
                    };
                    if record.declaration != binder.declaration
                        || record.constraints.has_owned()
                        || !record.constraints.requires_transfer()
                    {
                        return Err(failure(
                            "view oracle metadata formal lacks exact ordinary Transferable proof",
                        ));
                    }
                    None
                }
                TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                    Some(*declaration)
                }
                _ => {
                    return Err(failure(
                        "view oracle metadata contains owner/view/resource/secret/callable",
                    ));
                }
            };
            self.reader.charge(object.child_type_count())?;
            pending
                .try_reserve(object.child_type_count())
                .map_err(|_| failure("view oracle metadata reservation failed"))?;
            for child in object.child_types() {
                self.reader.charge(environment.len())?;
                pending.push((package, binder, child, depth + 1, environment.clone()));
            }
            if let Some(reference) = nominal {
                match self.declaration(package, reference)? {
                    PackageInterfaceDeclarationPayload::Record {
                        fields,
                        type_parameters,
                    } => {
                        let actuals: &[TypeObjectDigest] = match &object.form {
                            TypeForm::Applied { arguments, .. } => arguments,
                            _ => &[],
                        };
                        if actuals.len() != type_parameters.len() {
                            return Err(failure("view oracle metadata nominal arity differs"));
                        }
                        self.reader
                            .charge(fields.len() + type_parameters.len() + environment.len())?;
                        let mut field_environment = environment.clone();
                        for (formal, actual) in type_parameters.iter().zip(actuals) {
                            field_environment
                                .insert((reference, *formal), (package, binder, *actual));
                        }
                        for field in fields {
                            let Some(OwnerRecord::Field(field)) = self.sources[&reference.package]
                                .owners
                                .get(&OwnerKey::Field(field))
                            else {
                                return Err(failure("view oracle metadata layout field is absent"));
                            };
                            self.reader.charge(field_environment.len())?;
                            pending.push((
                                reference.package,
                                reference,
                                field.ty,
                                depth + 1,
                                field_environment.clone(),
                            ));
                        }
                    }
                    PackageInterfaceDeclarationPayload::Variant {
                        cases,
                        type_parameters,
                    } => {
                        let actuals: &[TypeObjectDigest] = match &object.form {
                            TypeForm::Applied { arguments, .. } => arguments,
                            _ => &[],
                        };
                        if actuals.len() != type_parameters.len() {
                            return Err(failure("view oracle metadata nominal arity differs"));
                        }
                        self.reader
                            .charge(cases.len() + type_parameters.len() + environment.len())?;
                        let mut field_environment = environment.clone();
                        for (formal, actual) in type_parameters.iter().zip(actuals) {
                            field_environment
                                .insert((reference, *formal), (package, binder, *actual));
                        }
                        for case in cases {
                            let Some(OwnerRecord::Case(case)) = self.sources[&reference.package]
                                .owners
                                .get(&OwnerKey::Case(case))
                            else {
                                return Err(failure("view oracle metadata layout case is absent"));
                            };
                            if let Some(ty) = case.payload {
                                self.reader.charge(field_environment.len())?;
                                pending.push((
                                    reference.package,
                                    reference,
                                    ty,
                                    depth + 1,
                                    field_environment.clone(),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(failure(
                            "view oracle metadata nominal has a non-data layout",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn permissions(
        &self,
        package: PackageId,
        binder: DeclarationReference,
        ty: TypeObjectDigest,
        transfer: bool,
        share: bool,
    ) -> Result<(), Diagnostic> {
        type Environment = BTreeMap<
            (
                DeclarationReference,
                crate::platform::semantic_id::TypeParameterId,
            ),
            (PackageId, DeclarationReference, TypeObjectDigest),
        >;
        let mut terms = vec![(package, binder, ty, Environment::new(), 0_usize)];
        while let Some((package, binder, ty, environment, depth)) = terms.pop() {
            if depth > contract::MAXIMUM_TYPE_DEPTH {
                return Err(capacity("view oracle stronger-bound capacity exceeded"));
            }
            let object = self.ty(package, ty)?;
            match &object.form {
                TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => {}
                TypeForm::TypeParameter { parameter } => {
                    if let Some((package, binder, ty)) =
                        environment.get(&(binder, *parameter)).copied()
                    {
                        self.reader.charge(1)?;
                        terms.push((package, binder, ty, environment, depth + 1));
                    } else {
                        let Some(OwnerRecord::TypeParameter(parameter)) = self.sources[&package]
                            .owners
                            .get(&OwnerKey::TypeParameter(*parameter))
                        else {
                            return Err(failure("view oracle stronger-bound formal is absent"));
                        };
                        if parameter.declaration != binder.declaration
                            || (transfer && !parameter.constraints.requires_transfer())
                            || (share
                                && parameter.constraints.has_owned()
                                && !parameter.constraints.requires_share())
                            || (!parameter.constraints.has_owned()
                                && !parameter.constraints.requires_transfer())
                        {
                            return Err(failure(
                                "view oracle nominal actual lacks its independent stronger permission",
                            ));
                        }
                    }
                }
                TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. } => {
                    self.reader.charge(object.child_type_count())?;
                    for child in object.child_types() {
                        self.reader.charge(environment.len())?;
                        terms.push((package, binder, child, environment.clone(), depth + 1));
                    }
                }
                TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                    match self.declaration(package, *declaration)? {
                        PackageInterfaceDeclarationPayload::OwnedRecord {
                            fields,
                            type_parameters,
                        } => {
                            let actuals: &[TypeObjectDigest] = match &object.form {
                                TypeForm::Applied { arguments, .. } => arguments,
                                _ => &[],
                            };
                            if type_parameters.len() != actuals.len() {
                                return Err(failure(
                                    "view oracle stronger-bound nominal actual arity differs",
                                ));
                            }
                            self.reader
                                .charge(fields.len() + actuals.len() + environment.len())?;
                            let mut fields_environment = environment.clone();
                            for (formal, actual) in type_parameters.iter().zip(actuals) {
                                fields_environment
                                    .insert((*declaration, *formal), (package, binder, *actual));
                                self.reader.charge(environment.len())?;
                                terms.push((
                                    package,
                                    binder,
                                    *actual,
                                    environment.clone(),
                                    depth + 1,
                                ));
                            }
                            for field in fields {
                                let Some(OwnerRecord::Field(field)) = self.sources
                                    [&declaration.package]
                                    .owners
                                    .get(&OwnerKey::Field(field))
                                else {
                                    return Err(failure(
                                        "view oracle stronger-bound nominal field inventory is absent",
                                    ));
                                };
                                self.reader.charge(fields_environment.len())?;
                                terms.push((
                                    declaration.package,
                                    *declaration,
                                    field.ty,
                                    fields_environment.clone(),
                                    depth + 1,
                                ));
                            }
                        }
                        PackageInterfaceDeclarationPayload::Record { .. }
                        | PackageInterfaceDeclarationPayload::Variant { .. } => {
                            self.metadata(package, binder, ty)?
                        }
                        _ => {
                            return Err(failure(
                                "view oracle stronger bound refers to a non-Owned/data nominal",
                            ));
                        }
                    }
                }
                TypeForm::AssociatedContractMember { .. }
                | TypeForm::AssociatedWitnessProjection { .. } => {
                    return Err(failure(
                        "view oracle abstract Owned family grants no sharing/transfer permission",
                    ));
                }
                _ => self.metadata(package, binder, ty)?,
            }
        }
        Ok(())
    }

    fn finite_layouts(&self) -> Result<(), Diagnostic> {
        let mut pending = BTreeSet::new();
        for (package, snapshot) in self.sources {
            self.reader
                .charge(snapshot.owners.len() + snapshot.types.len())?;
            for owner in snapshot.owners.values() {
                if let OwnerRecord::Declaration(DeclarationRecord {
                    header,
                    payload: DeclarationPayload::ReadView(_),
                    ..
                }) = owner
                {
                    let OwnerKey::Declaration(declaration) = header.owner else {
                        return Err(failure("view oracle layout identity is wrong"));
                    };
                    pending.insert(DeclarationReference {
                        package: *package,
                        declaration,
                    });
                }
            }
            for object in snapshot.types.values() {
                if let TypeForm::ReadViewApplied { arguments, .. } = &object.form {
                    self.reader.charge(arguments.len())?;
                    let mut actuals = arguments.clone();
                    while let Some(ty) = actuals.pop() {
                        let object = self.ty(*package, ty)?;
                        if let TypeForm::Named { declaration }
                        | TypeForm::Applied { declaration, .. } = object.form
                        {
                            pending.insert(declaration);
                        }
                        self.reader.charge(object.child_type_count())?;
                        actuals.extend(object.child_types());
                    }
                }
            }
        }
        let mut graph = BTreeMap::<DeclarationReference, BTreeSet<DeclarationReference>>::new();
        while let Some(reference) = pending.pop_first() {
            if graph.contains_key(&reference) {
                continue;
            }
            let payload = self.declaration(reference.package, reference)?;
            let mut types = match payload {
                PackageInterfaceDeclarationPayload::ReadView(view) => {
                    self.reader.charge(view.fields.len())?;
                    view.fields.iter().map(|field| field.ty).collect::<Vec<_>>()
                }
                PackageInterfaceDeclarationPayload::Record { fields, .. }
                | PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. } => {
                    self.reader.charge(fields.len())?;
                    fields
                        .iter()
                        .map(|field| {
                            match self.sources[&reference.package]
                                .owners
                                .get(&OwnerKey::Field(*field))
                            {
                                Some(OwnerRecord::Field(field)) => Ok(field.ty),
                                _ => Err(failure(
                                    "view oracle finite layout field inventory is incomplete",
                                )),
                            }
                        })
                        .collect::<Result<Vec<_>, Diagnostic>>()?
                }
                PackageInterfaceDeclarationPayload::Variant { cases, .. } => {
                    self.reader.charge(cases.len())?;
                    let mut types = Vec::new();
                    for case in cases {
                        let Some(OwnerRecord::Case(case)) = self.sources[&reference.package]
                            .owners
                            .get(&OwnerKey::Case(case))
                        else {
                            return Err(failure(
                                "view oracle finite layout case inventory is incomplete",
                            ));
                        };
                        types.extend(case.payload);
                    }
                    types
                }
                _ => {
                    return Err(failure(
                        "view oracle finite closure names a non-layout declaration",
                    ));
                }
            };
            let mut edges = BTreeSet::new();
            let mut seen = BTreeSet::new();
            while let Some(ty) = types.pop() {
                if !seen.insert(ty) {
                    continue;
                }
                let object = self.ty(reference.package, ty)?;
                match object.form {
                    TypeForm::Named { declaration }
                    | TypeForm::Applied { declaration, .. }
                    | TypeForm::ReadViewApplied { declaration, .. } => {
                        // Check the edge under its own defining package before enqueueing
                        // another declaration. Canonical private records cannot grant access.
                        self.declaration(reference.package, declaration)?;
                        self.reader.charge(2)?;
                        edges.insert(declaration);
                        pending.insert(declaration);
                    }
                    _ => {}
                }
                self.reader.charge(object.child_type_count())?;
                types.extend(object.child_types());
            }
            self.reader.charge(1)?;
            graph.insert(reference, edges);
        }
        // Independent elimination over declaration dependencies, without the production
        // layout stack, type classification, or nominal depth certificate.
        let mut remaining: BTreeMap<_, _> = graph
            .iter()
            .map(|(declaration, edges)| (*declaration, edges.len()))
            .collect();
        let mut parents = BTreeMap::<DeclarationReference, Vec<DeclarationReference>>::new();
        for (declaration, edges) in &graph {
            self.reader.charge(edges.len())?;
            for edge in edges {
                parents.entry(*edge).or_default().push(*declaration);
            }
        }
        self.reader.charge(remaining.len())?;
        let mut leaves: Vec<_> = remaining
            .iter()
            .filter_map(|(declaration, count)| (*count == 0).then_some(*declaration))
            .collect();
        let mut eliminated = 0;
        while let Some(leaf) = leaves.pop() {
            self.reader.charge(1)?;
            eliminated += 1;
            for parent in parents.remove(&leaf).unwrap_or_default() {
                self.reader.charge(1)?;
                let count = remaining
                    .get_mut(&parent)
                    .ok_or_else(|| failure("view oracle nominal parent is absent"))?;
                *count = count
                    .checked_sub(1)
                    .ok_or_else(|| failure("view oracle duplicate nominal edge"))?;
                if *count == 0 {
                    leaves.push(parent);
                }
            }
        }
        if eliminated != graph.len() {
            return Err(failure(
                "view oracle complete nominal field/phantom closure is recursive",
            ));
        }
        Ok(())
    }
}

pub(super) fn admit(
    sources: &BTreeMap<PackageId, KernelSnapshot>,
    public: &BTreeMap<PackageId, BTreeMap<OwnerKey, PackageInterfaceRecord>>,
    revisions: &BTreeMap<PackageId, super::PackageRevision>,
    reader: &Reader<'_>,
) -> Result<(), Diagnostic> {
    let interpretation = Interpretation {
        sources,
        public,
        revisions,
        reader,
    };
    for (package, snapshot) in sources {
        for owner in snapshot.owners.values() {
            reader.charge(1)?;
            let OwnerRecord::Declaration(declaration) = owner else {
                continue;
            };
            let OwnerKey::Declaration(id) = declaration.header.owner else {
                return Err(failure("view oracle declaration identity kind is wrong"));
            };
            let reference = DeclarationReference {
                package: *package,
                declaration: id,
            };
            match &declaration.payload {
                DeclarationPayload::Function(function) => {
                    reader.charge(function.parameters.len())?;
                    let parameters = function
                        .parameters
                        .iter()
                        .map(|id| {
                            let Some(OwnerRecord::Parameter(parameter)) =
                                snapshot.owners.get(&OwnerKey::Parameter(*id))
                            else {
                                return Err(failure(
                                    "view oracle function parameter inventory is incomplete",
                                ));
                            };
                            if parameter.parent != ParameterParent::Function(reference.declaration)
                            {
                                return Err(failure(
                                    "view oracle parameter captures a different function",
                                ));
                            }
                            Ok((*id, parameter.ty, parameter.use_mode))
                        })
                        .collect::<Result<Vec<_>, Diagnostic>>()?;
                    interpretation.signature(
                        *package,
                        ReadScopeBinder::Declaration(reference),
                        &parameters,
                        &function.result,
                        &function.effect,
                        &function.read_scope_parameters,
                        &function.read_scope_sources,
                    )?;
                }
                DeclarationPayload::OwnedContract(contract) => {
                    let mut ids = BTreeSet::new();
                    let mut names = BTreeSet::new();
                    reader.charge(contract.associated_types.len())?;
                    for member in &contract.associated_types {
                        if !ids.insert(member.id) || !names.insert(&member.name) {
                            return Err(failure(
                                "view oracle associated member inventory duplicates identities/names",
                            ));
                        }
                    }
                    for method in &contract.methods {
                        reader.charge(method.parameters.len())?;
                        let parameters = method
                            .parameters
                            .iter()
                            .enumerate()
                            .map(|(position, parameter)| {
                                Ok((
                                    u32::try_from(position).map_err(|_| {
                                        failure("view oracle method position overflow")
                                    })?,
                                    parameter.ty,
                                    parameter.use_mode,
                                ))
                            })
                            .collect::<Result<Vec<_>, Diagnostic>>()?;
                        interpretation.signature(
                            *package,
                            ReadScopeBinder::ContractMethod {
                                contract: reference,
                                method: method.id,
                            },
                            &parameters,
                            &method.result,
                            &method.effect,
                            &method.read_scope_parameters,
                            &method.read_scope_sources,
                        )?;
                    }
                }
                DeclarationPayload::ReadView(view) => {
                    if declaration.header.contract_version != contract::GRAPH_CONTRACT_VERSION
                        || snapshot.root.graph_contract_version != contract::GRAPH_CONTRACT_VERSION
                        || revisions[package].revision.graph_contract_version
                            != contract::GRAPH_CONTRACT_VERSION
                    {
                        return Err(failure(
                            "view oracle containing canonical profile is not current",
                        ));
                    }
                    let binder = ReadScopeBinder::Declaration(reference);
                    let declared = ReadScopeArgument::Parameter(ReadScopeReference {
                        binder,
                        parameter: view.scope_parameter.id,
                    });
                    let mut fields = BTreeSet::new();
                    let mut names = BTreeSet::new();
                    reader.charge(view.fields.len() + view.type_parameters.len())?;
                    for parameter in &view.type_parameters {
                        let Some(OwnerRecord::TypeParameter(parameter)) =
                            snapshot.owners.get(&OwnerKey::TypeParameter(*parameter))
                        else {
                            return Err(failure(
                                "view oracle declaration type inventory is incomplete",
                            ));
                        };
                        if parameter.declaration != reference.declaration
                            || !(parameter.constraints.has_owned()
                                || parameter.constraints.requires_transfer())
                        {
                            return Err(failure(
                                "view oracle declaration type formal lacks exact category admission",
                            ));
                        }
                    }
                    for field in &view.fields {
                        if !fields.insert(field.id) || !names.insert(&field.name) {
                            return Err(failure(
                                "view oracle complete field inventory duplicates identities/names",
                            ));
                        }
                        match field.kind {
                            ReadViewFieldKind::Ordinary => {
                                interpretation.metadata(*package, reference, field.ty)?
                            }
                            ReadViewFieldKind::ReadReference { read_scope } => {
                                if read_scope != declared {
                                    return Err(failure(
                                        "view oracle reference field differs from declaration scope",
                                    ));
                                }
                                let field_scope =
                                    view_scope(&interpretation.ty(*package, field.ty)?.form);
                                if field_scope.is_some()
                                    && (field_scope != Some(declared)
                                        || !matches!(
                                            interpretation.ty(*package, field.ty)?.form,
                                            TypeForm::ReadViewApplied { .. }
                                        ))
                                {
                                    return Err(failure(
                                        "view oracle reference view referent is not exact nominal/scope",
                                    ));
                                }
                                if field_scope.is_none()
                                    && !interpretation.owned(*package, field.ty)?
                                {
                                    return Err(failure(
                                        "view oracle reference field has no Owned/view referent",
                                    ));
                                }
                                interpretation.occurrences(
                                    *package,
                                    field.ty,
                                    field_scope.is_some(),
                                    binder,
                                    std::slice::from_ref(&view.scope_parameter),
                                    false,
                                )?;
                            }
                        }
                    }
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    let PackageInterfaceDeclarationPayload::OwnedContract(contract) =
                        interpretation.declaration(*package, implementation.contract)?
                    else {
                        return Err(failure(
                            "view oracle binding target is not an exact contract",
                        ));
                    };
                    reader.charge(
                        contract.associated_types.len() + implementation.associated_types.len(),
                    )?;
                    let members: BTreeMap<_, _> = contract
                        .associated_types
                        .iter()
                        .map(|member| (member.id, &member.bound))
                        .collect();
                    if members.len() != contract.associated_types.len()
                        || members.len() != implementation.associated_types.len()
                    {
                        return Err(failure(
                            "view oracle bindings do not cover complete member inventory",
                        ));
                    }
                    let mut previous = None;
                    for binding in &implementation.associated_types {
                        if previous.is_some_and(|previous| previous >= binding.member) {
                            return Err(failure(
                                "view oracle binding inventory is not in exact identity order",
                            ));
                        }
                        previous = Some(binding.member);
                        match (members.get(&binding.member).copied(), &binding.value) {
                            (
                                Some(AssociatedTypeBound::Owned),
                                AssociatedTypeBindingValue::Owned { ty },
                            ) => {
                                if !interpretation.owned(*package, *ty)? {
                                    return Err(failure(
                                        "view oracle Owned binding has no independent Owned category",
                                    ));
                                }
                                interpretation.occurrences(
                                    *package,
                                    *ty,
                                    false,
                                    ReadScopeBinder::Declaration(reference),
                                    &[],
                                    true,
                                )?;
                            }
                            (
                                Some(AssociatedTypeBound::ReadViewFamily { .. }),
                                AssociatedTypeBindingValue::ReadViewFamily {
                                    scope_parameter,
                                    ty,
                                },
                            ) => {
                                let binder = ReadScopeBinder::AssociatedBinding {
                                    implementation: reference,
                                    member: binding.member,
                                };
                                if !matches!(
                                    interpretation.ty(*package, *ty)?.form,
                                    TypeForm::ReadViewApplied { .. }
                                ) {
                                    return Err(failure(
                                        "view oracle family binding template is not nominal ReadView",
                                    ));
                                }
                                interpretation.occurrences(
                                    *package,
                                    *ty,
                                    true,
                                    binder,
                                    std::slice::from_ref(scope_parameter),
                                    true,
                                )?;
                            }
                            _ => {
                                return Err(failure(
                                    "view oracle binding/member kind and arity disagree",
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    interpretation.finite_layouts()
}
