//! Independent nominal layout admission for the symbolic memory oracle.
//! Reads canonical owner inventories and performs its own substitution.
use super::*;
use crate::platform::semantic_id::{FieldId, TypeParameterId};

const REFERENCE_LAYOUT_BYTES: usize = 64 * 1024 * 1024;
type OwnedRecordInventory = (
    Vec<TypeParameterId>,
    Vec<(FieldReference, TypeObjectDigest)>,
);

fn reserve_layout(bytes: &mut usize, count: usize, width: usize) -> Option<()> {
    *bytes = bytes.checked_add(count.checked_mul(width)?)?;
    (*bytes <= REFERENCE_LAYOUT_BYTES).then_some(())
}

impl Oracle<'_> {
    pub(super) fn record_layout_acyclic(
        &self,
        declaration: DeclarationReference,
        active: &mut BTreeSet<DeclarationReference>,
        depth: usize,
        remaining: &mut usize,
    ) -> Option<()> {
        let mut bytes = 0;
        self.record_cycle_walk(
            declaration,
            active,
            &mut BTreeSet::new(),
            depth,
            remaining,
            &mut bytes,
        )
    }

    fn record_cycle_walk(
        &self,
        declaration: DeclarationReference,
        active: &mut BTreeSet<DeclarationReference>,
        admitted: &mut BTreeSet<DeclarationReference>,
        depth: usize,
        remaining: &mut usize,
        bytes: &mut usize,
    ) -> Option<()> {
        if depth > contract::MAXIMUM_TYPE_DEPTH || active.contains(&declaration) {
            return None;
        }
        *remaining = remaining.checked_sub(1)?;
        if admitted.contains(&declaration) {
            return Some(());
        }
        reserve_layout(bytes, 1, 128)?;
        active.insert(declaration);
        let fields = self.record_field_ids(declaration)?;
        if fields.len() > contract::MAXIMUM_CHILDREN {
            return None;
        }
        reserve_layout(
            bytes,
            fields.len(),
            2 * std::mem::size_of::<TypeObjectDigest>(),
        )?;
        let mut pending = Vec::with_capacity(fields.len());
        for field in fields {
            pending.push(self.owned_record_field(declaration.package, *field)?.ty);
        }
        let mut seen = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            if seen.contains(&ty) {
                continue;
            }
            reserve_layout(bytes, 1, 96)?;
            seen.insert(ty);
            *remaining = remaining.checked_sub(1)?;
            let object = self
                .0
                .types
                .get(&ty)
                .or_else(|| self.0.dependency_types.get(&ty))?;
            if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } =
                &object.form
                && self.record_field_ids(*declaration).is_some()
            {
                self.record_cycle_walk(
                    *declaration,
                    active,
                    admitted,
                    depth + 1,
                    remaining,
                    bytes,
                )?;
            }
            reserve_layout(
                bytes,
                object.child_type_count(),
                2 * std::mem::size_of::<TypeObjectDigest>(),
            )?;
            pending.extend(object.child_types());
        }
        active.remove(&declaration);
        reserve_layout(bytes, 1, 128)?;
        admitted.insert(declaration);
        Some(())
    }

    fn record_field_ids(&self, declaration: DeclarationReference) -> Option<&[FieldId]> {
        let key = OwnerKey::Declaration(declaration.declaration);
        if declaration.package == self.0.root.package_id {
            let OwnerRecord::Declaration(record) = self.0.owners.get(&key)? else {
                return None;
            };
            match &record.payload {
                DeclarationPayload::OwnedRecord { fields, .. } => Some(fields),
                _ => None,
            }
        } else {
            let record = match self.representation_foreign(declaration.package, key)? {
                PackageInterfaceRecord::Declaration(record)
                | PackageInterfaceRecord::RepresentationDeclaration(record) => record,
                _ => return None,
            };
            match &record.payload {
                PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. } => Some(fields),
                _ => None,
            }
        }
    }
    pub(super) fn owned_record_inventory(
        &self,
        declaration: DeclarationReference,
    ) -> Option<OwnedRecordInventory> {
        let key = OwnerKey::Declaration(declaration.declaration);
        let (parameters, fields, header) = if declaration.package == self.0.root.package_id {
            let OwnerRecord::Declaration(record) = self.0.owners.get(&key)? else {
                return None;
            };
            let DeclarationPayload::OwnedRecord {
                type_parameters,
                fields,
            } = &record.payload
            else {
                return None;
            };
            (type_parameters, fields, record.header)
        } else {
            let record = match self.representation_foreign(declaration.package, key)? {
                PackageInterfaceRecord::Declaration(record)
                | PackageInterfaceRecord::RepresentationDeclaration(record) => record,
                _ => return None,
            };
            let PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters,
                fields,
            } = &record.payload
            else {
                return None;
            };
            (type_parameters, fields, record.header)
        };
        if header.owner != key
            || header.kind != OwnerKind::OwnedRecord
            || self.representation_generation(declaration)? < 32
            || parameters.len() > contract::MAXIMUM_CHILDREN
            || fields.len() > contract::MAXIMUM_CHILDREN
            || parameters.iter().copied().collect::<BTreeSet<_>>().len() != parameters.len()
            || fields.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return None;
        }
        let mut names = BTreeSet::new();
        for parameter in parameters {
            let record = self.type_parameter(declaration.package, *parameter)?;
            if record.header.owner != OwnerKey::TypeParameter(*parameter)
                || record.header.kind != OwnerKind::TypeParameter
                || record.declaration != declaration.declaration
                || (record.constraints.has_owned() && record.header.contract_version < 18)
                || (record.constraints.requires_transfer() && record.header.contract_version < 22)
                || (record.constraints.requires_share() && record.header.contract_version < 30)
                || (!record.constraints.has_owned() && !record.constraints.requires_transfer())
                || !names.insert(&record.name)
            {
                return None;
            }
        }
        let mut names = BTreeSet::new();
        let mut types = Vec::new();
        for field in fields {
            let record = self.owned_record_field(declaration.package, *field)?;
            if record.header.owner != OwnerKey::Field(*field)
                || record.header.kind != OwnerKind::Field
                || record.declaration != declaration.declaration
                || !names.insert(&record.name)
            {
                return None;
            }
            types.push((
                FieldReference {
                    package: declaration.package,
                    field: *field,
                },
                record.ty,
            ));
        }
        Some((parameters.clone(), types))
    }

    fn owned_record_field(&self, package: PackageId, field: FieldId) -> Option<&FieldRecord> {
        if package == self.0.root.package_id {
            match self.0.owners.get(&OwnerKey::Field(field))? {
                OwnerRecord::Field(record) => Some(record),
                _ => None,
            }
        } else {
            match self.representation_foreign(package, OwnerKey::Field(field))? {
                PackageInterfaceRecord::Field(record) => Some(record),
                _ => None,
            }
        }
    }

    pub(super) fn nominal_owner(&self, ty: TypeObjectDigest) -> Option<DeclarationReference> {
        match self.form(ty)? {
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. }
                if self.owned_record_inventory(*declaration).is_some() =>
            {
                Some(*declaration)
            }
            _ => None,
        }
    }

    pub(super) fn valid_record_application(&self, ty: TypeObjectDigest) -> bool {
        if self.product_shape(ty) {
            return true;
        }
        // A raw inventory also contains generic templates. Admit them only in
        // one actual lexical declaration that owns every free scheme parameter.
        let mut pending = vec![ty];
        let mut seen = BTreeSet::new();
        let mut candidates = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            if !seen.insert(ty) {
                continue;
            }
            if seen.len() > contract::MAXIMUM_VALIDATION_WORK {
                return false;
            }
            let Some(object) = self
                .0
                .types
                .get(&ty)
                .or_else(|| self.0.dependency_types.get(&ty))
            else {
                return false;
            };
            if let TypeForm::TypeParameter { parameter } = object.form {
                if let Some(record) = self.type_parameter(self.0.root.package_id, parameter) {
                    candidates.insert(DeclarationReference {
                        package: self.0.root.package_id,
                        declaration: record.declaration,
                    });
                }
                for package in self
                    .0
                    .dependencies
                    .keys()
                    .chain(self.0.representation_dependencies.keys())
                {
                    if let Some(record) = self.type_parameter(*package, parameter) {
                        candidates.insert(DeclarationReference {
                            package: *package,
                            declaration: record.declaration,
                        });
                    }
                }
            }
            match object.form {
                TypeForm::AssociatedWitnessProjection { scope, .. } => {
                    candidates.insert(scope);
                }
                TypeForm::AssociatedContractMember { contract, .. } => {
                    candidates.insert(contract);
                }
                _ => {}
            }
            pending.extend(object.child_types());
        }
        candidates
            .into_iter()
            .any(|scope| Oracle(self.0, Some(scope)).product_shape(ty))
    }

    pub(super) fn record_fields(
        &self,
        ty: TypeObjectDigest,
    ) -> Option<(
        DeclarationReference,
        Vec<(FieldReference, TypeObjectDigest)>,
    )> {
        let declaration = self.nominal_owner(ty)?;
        let (parameters, fields) = self.owned_record_inventory(declaration)?;
        let arguments = match self.form(ty)? {
            TypeForm::Applied { arguments, .. } => arguments.as_slice(),
            _ => &[],
        };
        if parameters.len() != arguments.len() {
            return None;
        }
        let bindings = parameters
            .into_iter()
            .zip(arguments.iter().copied())
            .collect();
        let fields = fields
            .into_iter()
            .map(|(field, ty)| Some((field, witnesses::substituted_type(self.0, ty, &bindings)?)))
            .collect::<Option<Vec<_>>>()?;
        Some((declaration, fields))
    }

    pub(super) fn record_access(&self, ty: TypeObjectDigest) -> bool {
        let Some(record) = self.nominal_owner(ty) else {
            return false;
        };
        let Some(scope) = self.1 else {
            return false;
        };
        if record.package != self.0.root.package_id || scope.package != record.package {
            return false;
        }
        let Some(OwnerRecord::Declaration(record_owner)) = self
            .0
            .owners
            .get(&OwnerKey::Declaration(record.declaration))
        else {
            return false;
        };
        let Some(OwnerRecord::Declaration(scope_owner)) =
            self.0.owners.get(&OwnerKey::Declaration(scope.declaration))
        else {
            return false;
        };
        record_owner.module == scope_owner.module
    }

    pub(super) fn valid_record_template(&self, declaration: DeclarationReference) -> bool {
        let Some((_, fields)) = self.owned_record_inventory(declaration) else {
            return false;
        };
        let scoped = Oracle(self.0, Some(declaration));
        let mut active = BTreeSet::from([declaration]);
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        fields.into_iter().all(|(_, ty)| {
            scoped
                .layout_shape(ty, &mut active, 0, &mut remaining)
                .is_some()
        })
    }

    pub(super) fn layout_shape(
        &self,
        ty: TypeObjectDigest,
        active: &mut BTreeSet<DeclarationReference>,
        depth: usize,
        remaining: &mut usize,
    ) -> Option<bool> {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        *remaining = remaining.checked_sub(1)?;
        match self.form(ty)? {
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Some(true),
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. } => {
                // Caller projections are opaque actuals. A raw nominal field
                // has Forbidden context, so this cannot authorize stored
                // projections in a representation template.
                self.associated_owned(ty).then_some(true)
            }
            TypeForm::TypeParameter { parameter } => {
                let parameter = self.scoped_parameter(*parameter)?;
                (parameter.constraints.has_owned() || parameter.constraints.requires_transfer())
                    .then_some(parameter.constraints.has_owned())
            }
            TypeForm::OwnedSequence { item } => {
                self.layout_shape(*item, active, depth + 1, remaining)?;
                Some(true)
            }
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                if fields.is_empty()
                    || fields.len() > contract::MAXIMUM_CHILDREN
                    || fields.windows(2).any(|pair| pair[0].name >= pair[1].name)
                {
                    return None;
                }
                let mut owned = false;
                for field in fields {
                    owned |= self.layout_shape(field.ty, active, depth + 1, remaining)?;
                }
                owned.then_some(true)
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. }
                if self.owned_record_inventory(*declaration).is_some() =>
            {
                // Track nominal identity, not an application digest: expanding
                // recursive arguments or a sequence edge must still be rejected.
                let (parameters, templates) = self.owned_record_inventory(*declaration)?;
                let arguments = match self.form(ty)? {
                    TypeForm::Applied { arguments, .. } => arguments.as_slice(),
                    _ => &[],
                };
                if parameters.len() != arguments.len() {
                    return None;
                }
                for (parameter, actual) in parameters.iter().zip(arguments) {
                    let bounds = self
                        .type_parameter(declaration.package, *parameter)?
                        .constraints;
                    let owned = self.layout_shape(*actual, active, depth + 1, remaining)?;
                    if owned != bounds.has_owned()
                        || (bounds.requires_capture_safe() && !self.capture_safe(*actual))
                        || (bounds.requires_transfer()
                            && self.parallel_shape(*actual, &BTreeMap::new(), depth + 1, remaining)
                                != Some(owned))
                        || (bounds.requires_share()
                            && self.read_shape(*actual, &BTreeMap::new(), depth + 1, remaining)
                                != Some(true))
                    {
                        return None;
                    }
                }
                if !active.insert(*declaration) {
                    return None;
                }
                // Independently admit the complete template even for phantom
                // applications; actual substitution cannot rescue an invalid field.
                let scoped = Oracle(self.0, Some(*declaration));
                let (_, fields) = self.record_fields(ty)?;
                for (_, field) in &templates {
                    scoped.layout_shape(*field, active, depth + 1, remaining)?;
                }
                active.remove(declaration);
                for ((_, template), (_, field)) in templates.into_iter().zip(fields) {
                    if template != field {
                        self.layout_shape(field, active, depth + 1, remaining)?;
                    }
                }
                Some(true)
            }
            _ => {
                let mut pending = vec![(ty, depth)];
                while let Some((next, depth)) = pending.pop() {
                    if depth > contract::MAXIMUM_TYPE_DEPTH {
                        return None;
                    }
                    *remaining = remaining.checked_sub(1)?;
                    let object = self
                        .0
                        .types
                        .get(&next)
                        .or_else(|| self.0.dependency_types.get(&next))?;
                    pending.extend(
                        object
                            .child_types()
                            .into_iter()
                            .map(|child| (child, depth + 1)),
                    );
                }
                self.ordinary(ty).then_some(false)
            }
        }
    }
}
