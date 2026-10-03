//! Test-only symbolic memory oracle. It reads canonical inventories, never the production
//! memory checker, compiler, prepared classifications, or runtime tokens.
use super::*;
use crate::platform::semantic_id::ExpressionId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default, Eq, PartialEq)]
struct Rights {
    // Identity survives consumption until lexical exit: a moved owner is not ordinary data.
    memory: BTreeSet<LocalValueReference>,
    owned: BTreeSet<LocalValueReference>,
    borrowed: BTreeSet<LocalValueReference>,
}
#[path = "implementation_effect_memory_oracle_tests.rs"]
mod implementation_effect_tests;
#[path = "imported_memory_oracle_tests.rs"]
mod imported_tests;
#[path = "transfer_memory_oracle_tests.rs"]
mod transfer_tests;
#[path = "memory_witness_reference.rs"]
mod witnesses;
struct Oracle<'a>(&'a KernelSnapshot, Option<DeclarationReference>);
impl Oracle<'_> {
    fn foreign(&self, package: PackageId, key: OwnerKey) -> Option<&PackageInterfaceRecord> {
        let revision = self.0.dependencies.get(&package)?.package_revision;
        self.0.dependency_interfaces.get(&revision)?.get(&key)
    }
    fn buffer(&self, t: TypeObjectDigest) -> bool {
        matches!(
            self.0
                .types
                .get(&t)
                .or_else(|| self.0.dependency_types.get(&t))
                .map(|t| &t.form),
            Some(
                TypeForm::ByteBuffer
                    | TypeForm::OwnedI64Cell
                    | TypeForm::OwnedProduct { .. }
                    | TypeForm::OwnedChoice { .. }
            )
        ) || matches!(self.form(t), Some(TypeForm::TypeParameter { parameter }) if self.type_parameter(self.1.map_or(self.0.root.package_id, |f| f.package), *parameter).is_some_and(|p| p.constraints.has_owned()))
    }
    fn contains(&self, t: TypeObjectDigest) -> bool {
        let mut todo = vec![t];
        let mut seen = BTreeSet::new();
        while let Some(t) = todo.pop() {
            if !seen.insert(t) {
                continue;
            }
            if self.buffer(t) {
                return true;
            }
            let Some(t) = self
                .0
                .types
                .get(&t)
                .or_else(|| self.0.dependency_types.get(&t))
            else {
                return true;
            };
            if matches!(t.form, TypeForm::ByteBuffer | TypeForm::OwnedI64Cell) {
                return true;
            }
            if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } = &t.form
            {
                let Some((_, fields)) = self.nominal(*declaration) else {
                    return true;
                };
                todo.extend(fields);
            }
            todo.extend(t.child_types());
        }
        false
    }
    fn parameter(
        &self,
        package: PackageId,
        id: crate::platform::semantic_id::ParameterId,
    ) -> Option<ParameterRecord> {
        if package == self.0.root.package_id {
            match self.0.owners.get(&OwnerKey::Parameter(id))? {
                OwnerRecord::Parameter(p) => Some(p.clone()),
                _ => None,
            }
        } else {
            match self.foreign(package, OwnerKey::Parameter(id))? {
                PackageInterfaceRecord::Parameter(p) => Some(p.clone()),
                _ => None,
            }
        }
    }
    fn type_parameters(
        &self,
        d: DeclarationReference,
    ) -> Option<Vec<crate::platform::semantic_id::TypeParameterId>> {
        if d.package == self.0.root.package_id {
            let OwnerRecord::Declaration(record) =
                self.0.owners.get(&OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            Some(match &record.payload {
                DeclarationPayload::Function(f) => f.type_parameters.clone(),
                DeclarationPayload::External(f) => f.type_parameters.clone(),
                _ => return None,
            })
        } else {
            let PackageInterfaceRecord::Declaration(record) =
                self.foreign(d.package, OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            Some(match &record.payload {
                PackageInterfaceDeclarationPayload::Function(f) => f.type_parameters.clone(),
                PackageInterfaceDeclarationPayload::External(f) => f.type_parameters.clone(),
                _ => return None,
            })
        }
    }
    fn application(&self, d: DeclarationReference, arguments: &[TypeObjectDigest]) -> bool {
        let Some(parameters) = self.type_parameters(d) else {
            return arguments.iter().all(|t| !self.contains(*t));
        };
        if parameters.len() != arguments.len() {
            return false;
        }
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        parameters.iter().zip(arguments).all(|(id, ty)| {
            let p = if d.package == self.0.root.package_id {
                match self.0.owners.get(&OwnerKey::TypeParameter(*id)) {
                    Some(OwnerRecord::TypeParameter(p)) => Some(p),
                    _ => None,
                }
            } else {
                match self.foreign(d.package, OwnerKey::TypeParameter(*id)) {
                    Some(PackageInterfaceRecord::TypeParameter(p)) => Some(p),
                    _ => None,
                }
            };
            p.is_some_and(|p| {
                p.declaration == d.declaration
                    && (if p.constraints.has_owned() {
                        self.buffer(*ty) && self.owned_type_in_scope(*ty)
                    } else {
                        !self.contains(*ty)
                    })
                    && (!p.constraints.requires_capture_safe() || self.capture_safe(*ty))
                    && (!p.constraints.requires_transfer()
                        || (self.function(d).is_some()
                            && self.parallel_shape(*ty, &BTreeMap::new(), 0, &mut remaining)
                                == Some(p.constraints.has_owned())))
            })
        })
    }
    fn owned_type_in_scope(&self, ty: TypeObjectDigest) -> bool {
        match self.form(ty) {
            Some(TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. }) => {
                self.product_shape(ty)
            }
            Some(TypeForm::TypeParameter { parameter }) => self
                .scoped_parameter(*parameter)
                .is_some_and(|p| p.constraints.has_owned()),
            Some(TypeForm::ByteBuffer | TypeForm::OwnedI64Cell) => true,
            _ => false,
        }
    }
    fn product_shape(&self, ty: TypeObjectDigest) -> bool {
        let mut depths = BTreeMap::new();
        let mut closure = vec![(ty, 0)];
        while let Some((ty, depth)) = closure.pop() {
            if depth > contract::MAXIMUM_TYPE_DEPTH {
                return false;
            }
            if depths.get(&ty).is_some_and(|previous| *previous >= depth) {
                continue;
            }
            depths.insert(ty, depth);
            let Some(object) = self
                .0
                .types
                .get(&ty)
                .or_else(|| self.0.dependency_types.get(&ty))
            else {
                return false;
            };
            closure.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
        let mut pending = vec![(ty, 0)];
        let mut checked = BTreeSet::new();
        while let Some((ty, depth)) = pending.pop() {
            if depth > contract::MAXIMUM_TYPE_DEPTH {
                return false;
            }
            if !checked.insert(ty) {
                continue;
            }
            let Some(TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields }) =
                self.form(ty)
            else {
                return false;
            };
            if fields.is_empty()
                || fields.len() > contract::MAXIMUM_CHILDREN
                || fields.windows(2).any(|f| f[0].name >= f[1].name)
            {
                return false;
            }
            let mut owned = false;
            for field in fields {
                match self.form(field.ty) {
                    Some(TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. }) => {
                        owned = true;
                        pending.push((field.ty, depth + 1));
                    }
                    Some(TypeForm::ByteBuffer | TypeForm::OwnedI64Cell) => owned = true,
                    Some(TypeForm::TypeParameter { .. }) if self.owned_type_in_scope(field.ty) => {
                        owned = true
                    }
                    _ if self.ordinary(field.ty) => {}
                    _ => return false,
                }
            }
            if !owned {
                return false;
            }
        }
        true
    }
    fn local_type(&self, local: LocalValueReference) -> Option<TypeObjectDigest> {
        match local {
            LocalValueReference::FunctionParameter(p) => {
                self.parameter(self.0.root.package_id, p).map(|p| p.ty)
            }
            LocalValueReference::LexicalBinding(b) => {
                match self.0.owners.get(&OwnerKey::Binding(b))? {
                    OwnerRecord::Binding(b) => b.declared_type,
                    _ => None,
                }
            }
            _ => None,
        }
    }
    fn signature(
        &self,
        d: DeclarationReference,
    ) -> Option<(Vec<ParameterRecord>, TypeObjectDigest, bool, Option<String>)> {
        let (parameters, result, pure, implementation) = if d.package == self.0.root.package_id {
            let OwnerRecord::Declaration(d) =
                self.0.owners.get(&OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            match &d.payload {
                DeclarationPayload::Function(f) => (
                    f.parameters.clone(),
                    f.result,
                    matches!(f.effect, FunctionEffect::Pure),
                    None,
                ),
                DeclarationPayload::External(f) => (
                    f.parameters.clone(),
                    f.result,
                    true,
                    Some(f.implementation.as_str().to_owned()),
                ),
                _ => return None,
            }
        } else {
            let PackageInterfaceRecord::Declaration(d) =
                self.foreign(d.package, OwnerKey::Declaration(d.declaration))?
            else {
                return None;
            };
            match &d.payload {
                PackageInterfaceDeclarationPayload::Function(f) => (
                    f.parameters.clone(),
                    f.result,
                    matches!(f.effect, FunctionEffect::Pure),
                    None,
                ),
                PackageInterfaceDeclarationPayload::External(f) => {
                    (f.parameters.clone(), f.result, true, None)
                }
                _ => return None,
            }
        };
        Some((
            parameters
                .into_iter()
                .map(|p| self.parameter(d.package, p))
                .collect::<Option<Vec<_>>>()?,
            result,
            pure,
            implementation,
        ))
    }
    fn legal_signature(
        &self,
        s: &(Vec<ParameterRecord>, TypeObjectDigest, bool, Option<String>),
    ) -> bool {
        let (parameters, result, pure, implementation) = s;
        // Independently classify the complete ordered inventory: data, memory,
        // resource. No production signature classifier participates in this oracle.
        let mut classes = Vec::new();
        for p in parameters {
            let class = if self.buffer(p.ty) {
                if p.resource_requirement.is_some()
                    || p.use_mode == ParameterUse::Unrestricted
                    || (!pure && p.use_mode != ParameterUse::Consume)
                {
                    return false;
                }
                1
            } else if matches!(self.form(p.ty), Some(TypeForm::CapabilityResource { .. })) {
                2
            } else {
                if self.contains(p.ty) {
                    return false;
                }
                0
            };
            classes.push(class);
        }
        if classes.windows(2).any(|pair| pair[0] > pair[1]) {
            return false;
        }
        let memory = classes.contains(&1) || self.buffer(*result);
        if memory && *pure && classes.contains(&2) {
            return false;
        }
        if !self.buffer(*result) && self.contains(*result) {
            return false;
        }
        if let Some(i) = implementation
            && memory
        {
            if i == "core.cell.create" {
                return parameters.len() == 1
                    && matches!(
                        self.0.types.get(&parameters[0].ty).map(|t| &t.form),
                        Some(TypeForm::I64)
                    )
                    && parameters[0].use_mode == ParameterUse::Unrestricted
                    && matches!(
                        self.0.types.get(result).map(|t| &t.form),
                        Some(TypeForm::OwnedI64Cell)
                    );
            }
            let expected = match i.as_str() {
                "core.buffer.empty" => (0, false, true),
                "core.cell.replace" => (2, false, true),
                "core.cell.read" => (1, true, false),
                "core.cell.extract" | "core.cell.discard" => (1, false, false),
                "core.buffer.push" => (2, false, true),
                "core.buffer.get" => (2, true, false),
                "core.buffer.length" => (1, true, false),
                "core.buffer.freeze" | "core.buffer.discard" => (1, false, false),
                _ => return false,
            };
            if parameters.len() != expected.0 || self.buffer(*result) != expected.2 {
                return false;
            }
            if let Some(p) = parameters.last()
                && (!self.buffer(p.ty)
                    || p.use_mode
                        != if expected.1 {
                            ParameterUse::Borrow
                        } else {
                            ParameterUse::Consume
                        })
            {
                return false;
            }
        }
        true
    }
    // Application constraints belong to the callee's exact package, but affine
    // flow belongs to the caller. Resolve direct formals before classifying an
    // imported signature; a foreign Owned parameter is not ordinary caller data.
    // This reconstruction deliberately uses canonical records, not production
    // substitution or memory-classification helpers.
    fn applied_signature(
        &self,
        function: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Option<(Vec<ParameterRecord>, TypeObjectDigest, bool, Option<String>)> {
        if !self.application(function, arguments) {
            return None;
        }
        let parameters = self.type_parameters(function)?;
        let substitutions: BTreeMap<_, _> = parameters
            .into_iter()
            .zip(arguments.iter().copied())
            .collect();
        let substitute = |ty| match self.form(ty) {
            Some(TypeForm::TypeParameter { parameter }) => {
                substitutions.get(parameter).copied().unwrap_or(ty)
            }
            _ => ty,
        };
        let mut signature = self.signature(function)?;
        for parameter in &mut signature.0 {
            if parameter.parent != ParameterParent::Function(function.declaration) {
                return None;
            }
            parameter.ty = substitute(parameter.ty);
        }
        signature.1 = substitute(signature.1);
        Some(signature)
    }
    fn expression(&self, id: ExpressionId) -> Option<&ExpressionOperation> {
        let OwnerRecord::Expression(e) = self.0.owners.get(&OwnerKey::Expression(id))? else {
            return None;
        };
        Some(&e.operation)
    }
    fn parallel_shape(
        &self,
        ty: TypeObjectDigest,
        bindings: &BTreeMap<crate::platform::semantic_id::TypeParameterId, TypeObjectDigest>,
        depth: usize,
        remaining: &mut usize,
    ) -> Option<bool> {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        *remaining = remaining.checked_sub(1)?;
        match self.form(ty)? {
            TypeForm::TypeParameter { parameter } => {
                if let Some(actual) = bindings.get(parameter) {
                    return self.parallel_shape(*actual, &BTreeMap::new(), depth, remaining);
                }
                let p = self.scoped_parameter(*parameter)?;
                p.constraints
                    .requires_transfer()
                    .then_some(p.constraints.has_owned())
            }
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Some(true),
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                if fields.is_empty()
                    || fields.len() > contract::MAXIMUM_CHILDREN
                    || fields.windows(2).any(|f| f[0].name >= f[1].name)
                {
                    return None;
                }
                let mut owned = false;
                for field in fields {
                    owned |= self.parallel_shape(field.ty, bindings, depth + 1, remaining)?;
                }
                owned.then_some(true)
            }
            _ => {
                let ordinary = bindings
                    .iter()
                    .filter_map(|(p, ty)| self.ordinary(*ty).then_some(*p))
                    .collect();
                self.ordinary_assuming(ty, ordinary).then_some(false)
            }
        }
    }
    fn parallel_child(&self, id: ExpressionId) -> Option<bool> {
        let (function, arguments, types, implementations) = match self.expression(id)? {
            ExpressionOperation::Call {
                function,
                arguments,
                type_arguments,
                requirement_arguments,
                effect_arguments,
            } if requirement_arguments.is_empty() && effect_arguments.is_empty() => {
                (*function, arguments, type_arguments, &[][..])
            }
            ExpressionOperation::ImplementationCall {
                function,
                arguments,
                type_arguments,
                implementations,
                requirement_arguments,
                effect_arguments,
            } if requirement_arguments.is_empty() && effect_arguments.is_empty() => (
                *function,
                arguments,
                type_arguments,
                implementations.as_slice(),
            ),
            _ => return None,
        };
        let signature = self.function(function)?;
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        if !self.application(function, types)
            || !self.witnesses(function, types, implementations)
            || types.iter().any(|ty| {
                self.parallel_shape(*ty, &BTreeMap::new(), 0, &mut remaining)
                    .is_none()
            })
            || !signature.requirement_parameters.is_empty()
            || !signature.effect_parameters.is_empty()
            || !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
            || signature.parameters.len() != arguments.len()
        {
            return None;
        }
        let bindings = signature
            .type_parameters
            .into_iter()
            .zip(types.iter().copied())
            .collect();
        let result = self.parallel_shape(signature.result, &bindings, 0, &mut remaining)?;
        let mut owned = false;
        for id in signature.parameters {
            let p = self.parameter(function.package, id)?;
            let memory = self.parallel_shape(p.ty, &bindings, 0, &mut remaining)?;
            if p.parent != ParameterParent::Function(function.declaration)
                || p.resource_requirement.is_some()
                || (owned && !memory)
                || p.use_mode
                    != if memory {
                        ParameterUse::Consume
                    } else {
                        ParameterUse::Unrestricted
                    }
            {
                return None;
            }
            owned |= memory;
        }
        Some(result)
    }
    fn arguments(
        &self,
        parameters: &[(TypeObjectDigest, ParameterUse)],
        arguments: &[ExpressionId],
        rights: &mut Rights,
        depth: usize,
    ) -> Option<()> {
        if parameters.len() != arguments.len() {
            return None;
        }
        let mut moved = BTreeSet::new();
        let mut borrowed = BTreeSet::new();
        for ((ty, use_mode), argument) in parameters.iter().zip(arguments) {
            if self.buffer(*ty) {
                let ExpressionOperation::Local { value } = self.expression(*argument)? else {
                    return None;
                };
                match use_mode {
                    ParameterUse::Borrow => {
                        if !rights.owned.contains(value) && !rights.borrowed.contains(value) {
                            return None;
                        }
                        borrowed.insert(*value);
                    }
                    ParameterUse::Consume => {
                        if !moved.insert(*value) || !rights.owned.remove(value) {
                            return None;
                        }
                    }
                    ParameterUse::Unrestricted => return None,
                }
            } else if self.run(*argument, rights, false, depth + 1)? {
                return None;
            }
        }
        moved.is_disjoint(&borrowed).then_some(())
    }
    // A result is either ordinary data or a moved owner. Reads never produce owners.
    fn run(&self, id: ExpressionId, rights: &mut Rights, take: bool, depth: usize) -> Option<bool> {
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return None;
        }
        let plain = |id, rights: &mut Rights| {
            self.run(id, rights, false, depth + 1)
                .filter(|v| !*v)
                .map(|_| ())
        };
        let output = match self.expression(id)? {
            ExpressionOperation::ChooseOwned {
                choice_type,
                case,
                value,
            } => {
                if !self.product_shape(*choice_type) {
                    return None;
                }
                let TypeForm::OwnedChoice { cases } = self.form(*choice_type)? else {
                    return None;
                };
                let selected = cases.iter().find(|item| &item.name == case)?;
                if self.buffer(selected.ty) {
                    let ExpressionOperation::Local { value: local } = self.expression(*value)?
                    else {
                        return None;
                    };
                    if self.local_type(*local) != Some(selected.ty)
                        || !self.run(*value, rights, true, depth + 1)?
                    {
                        return None;
                    }
                } else {
                    plain(*value, rights)?;
                }
                true
            }
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            } => {
                if !self.product_shape(*choice_type) {
                    return None;
                }
                let TypeForm::OwnedChoice { cases } = self.form(*choice_type)? else {
                    return None;
                };
                let ExpressionOperation::Local { value } = self.expression(*source)? else {
                    return None;
                };
                if self.local_type(*value) != Some(*choice_type)
                    || arms.len() != cases.len()
                    || arms.is_empty()
                    || !self.run(*source, rights, true, depth + 1)?
                {
                    return None;
                }
                let before = rights.clone();
                let mut names = BTreeSet::new();
                let mut bindings = BTreeSet::new();
                let mut outcome = None;
                for arm in arms {
                    let case = cases.iter().find(|case| case.name == arm.name)?;
                    let OwnerRecord::Binding(binding) =
                        self.0.owners.get(&OwnerKey::Binding(arm.binding))?
                    else {
                        return None;
                    };
                    if !names.insert(&arm.name)
                        || !bindings.insert(arm.binding)
                        || binding.kind != BindingKind::OwnedChoicePayload
                        || binding.value.is_some()
                        || binding.declared_type != Some(case.ty)
                    {
                        return None;
                    }
                    let local = LocalValueReference::LexicalBinding(arm.binding);
                    if before.memory.contains(&local) {
                        return None;
                    }
                    let mut branch = before.clone();
                    if self.buffer(case.ty) {
                        branch.memory.insert(local);
                        branch.owned.insert(local);
                    }
                    let output = self.run(arm.body, &mut branch, take, depth + 1)?;
                    branch.memory.remove(&local);
                    branch.owned.remove(&local);
                    if branch.memory != before.memory
                        || branch.borrowed != before.borrowed
                        || outcome.is_some_and(|prior| prior != output)
                    {
                        return None;
                    }
                    outcome = Some(output);
                    rights.owned.retain(|owner| branch.owned.contains(owner));
                }
                outcome?
            }
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                if !self.product_shape(*product_type) {
                    return None;
                }
                let TypeForm::OwnedProduct { fields: expected } = self.form(*product_type)? else {
                    return None;
                };
                if fields.len() != expected.len() {
                    return None;
                }
                let mut seen = BTreeSet::new();
                for field in fields {
                    let expected = expected.iter().find(|f| f.name == field.name)?;
                    if !seen.insert(&field.name) {
                        return None;
                    }
                    if self.buffer(expected.ty) {
                        let ExpressionOperation::Local { value } = self.expression(field.value)?
                        else {
                            return None;
                        };
                        if self.local_type(*value) != Some(expected.ty)
                            || !self.run(field.value, rights, true, depth + 1)?
                        {
                            return None;
                        }
                    } else {
                        plain(field.value, rights)?;
                    }
                }
                true
            }
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                if !self.product_shape(*product_type) {
                    return None;
                }
                let TypeForm::OwnedProduct { fields: expected } = self.form(*product_type)? else {
                    return None;
                };
                let ExpressionOperation::Local { value } = self.expression(*source)? else {
                    return None;
                };
                if self.local_type(*value) != Some(*product_type)
                    || !self.run(*source, rights, true, depth + 1)?
                    || fields.len() != expected.len()
                {
                    return None;
                }
                let mut scope = BTreeSet::new();
                for (field, expected) in fields.iter().zip(expected) {
                    let OwnerRecord::Binding(b) =
                        self.0.owners.get(&OwnerKey::Binding(field.binding))?
                    else {
                        return None;
                    };
                    if field.name != expected.name
                        || b.kind != BindingKind::OwnedUnpack
                        || b.value.is_some()
                        || b.declared_type != Some(expected.ty)
                    {
                        return None;
                    }
                    let local = LocalValueReference::LexicalBinding(field.binding);
                    if !scope.insert(local) || rights.memory.contains(&local) {
                        return None;
                    }
                    if self.buffer(expected.ty) {
                        rights.memory.insert(local);
                        rights.owned.insert(local);
                    }
                }
                let result = self.run(*body, rights, take, depth + 1)?;
                for local in scope {
                    rights.memory.remove(&local);
                    rights.owned.remove(&local);
                }
                result
            }
            ExpressionOperation::Local { value } if rights.memory.contains(value) => {
                if !take {
                    return None;
                }
                if !rights.owned.remove(value) {
                    return None;
                }
                true
            }
            ExpressionOperation::Let { bindings, body } => {
                let mut scope = vec![];
                for id in bindings {
                    let OwnerRecord::Binding(b) = self.0.owners.get(&OwnerKey::Binding(*id))?
                    else {
                        return None;
                    };
                    let memory = b.declared_type.is_some_and(|t| self.buffer(t));
                    if memory && !self.owned_type_in_scope(b.declared_type?) {
                        return None;
                    }
                    if self.run(b.value?, rights, memory, depth + 1)? != memory {
                        return None;
                    }
                    if memory {
                        let r = LocalValueReference::LexicalBinding(*id);
                        if !rights.memory.insert(r) || !rights.owned.insert(r) {
                            return None;
                        }
                        scope.push(r);
                    }
                }
                let result = self.run(*body, rights, take, depth + 1)?;
                for r in scope {
                    rights.memory.remove(&r);
                    rights.owned.remove(&r);
                }
                result
            }
            ExpressionOperation::Sequence { items } => {
                let mut output = false;
                for (i, e) in items.iter().enumerate() {
                    output = self.run(*e, rights, i + 1 < items.len() || take, depth + 1)?;
                }
                output
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                plain(*condition, rights)?;
                let mut a = rights.clone();
                let mut b = rights.clone();
                let av = self.run(*when_true, &mut a, take, depth + 1)?;
                let bv = self.run(*when_false, &mut b, take, depth + 1)?;
                if av != bv {
                    return None;
                }
                a.owned = a.owned.intersection(&b.owned).copied().collect();
                if a.borrowed != b.borrowed || a.memory != b.memory {
                    return None;
                }
                *rights = a;
                av
            }
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
                ..
            } => {
                if self
                    .function(*function)
                    .is_some_and(|f| !f.implementation_parameters.is_empty())
                    || !self.application(*function, type_arguments)
                {
                    return None;
                }
                if self.signature(*function).is_none() {
                    for a in arguments {
                        plain(*a, rights)?;
                    }
                    return Some(false);
                }
                let s = self.applied_signature(*function, type_arguments)?;
                if !self.legal_signature(&s) || s.0.len() != arguments.len() {
                    return None;
                }
                self.arguments(
                    &s.0.iter().map(|p| (p.ty, p.use_mode)).collect::<Vec<_>>(),
                    arguments,
                    rights,
                    depth,
                )?;
                self.buffer(s.1)
            }
            ExpressionOperation::Parallel { left, right } => {
                if let Some(function) = self.1
                    && self
                        .function(function)
                        .is_none_or(|f| matches!(f.effect, FunctionEffect::Pure))
                {
                    return None;
                }
                let left_owned = self.parallel_child(*left)?;
                let right_owned = self.parallel_child(*right)?;
                // Both argument inventories consume the same parent rights in
                // authored order. Child results become fields of one owned pair
                // whenever either child returns an owner.
                if self.run(*left, rights, true, depth + 1)? != left_owned
                    || self.run(*right, rights, true, depth + 1)? != right_owned
                {
                    return None;
                }
                left_owned || right_owned
            }
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                implementations,
                arguments,
                ..
            } => {
                if !self.application(*function, type_arguments)
                    || !self.witnesses(*function, type_arguments, implementations)
                {
                    return None;
                }
                let s = self.applied_signature(*function, type_arguments)?;
                if !self.legal_signature(&s) {
                    return None;
                }
                self.arguments(
                    &s.0.iter().map(|p| (p.ty, p.use_mode)).collect::<Vec<_>>(),
                    arguments,
                    rights,
                    depth,
                )?;
                self.buffer(s.1)
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
                let m = self.method(*witness, *contract, *method)?;
                self.arguments(
                    &m.parameters
                        .iter()
                        .map(|p| (p.ty, p.use_mode))
                        .collect::<Vec<_>>(),
                    arguments,
                    rights,
                    depth,
                )?;
                self.buffer(m.result)
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                if self
                    .function(*function)
                    .is_some_and(|f| !f.implementation_parameters.is_empty())
                    || type_arguments.iter().any(|t| self.contains(*t))
                {
                    return None;
                }
                let s = self.applied_signature(*function, type_arguments)?;
                if self.contains(s.1) || s.0.iter().any(|p| self.contains(p.ty)) {
                    return None;
                }
                false
            }
            ExpressionOperation::Invoke { callee, arguments }
            | ExpressionOperation::Bind { callee, arguments } => {
                plain(*callee, rights)?;
                for a in arguments {
                    plain(*a, rights)?;
                }
                false
            }
            ExpressionOperation::Record { fields, .. } => {
                for f in fields {
                    plain(f.value, rights)?;
                }
                false
            }
            ExpressionOperation::Variant { payload, .. } => {
                if let Some(e) = payload {
                    plain(*e, rights)?;
                }
                false
            }
            ExpressionOperation::List { items, .. } => {
                for e in items {
                    plain(*e, rights)?;
                }
                false
            }
            ExpressionOperation::Map { entries, .. } => {
                for e in entries {
                    plain(e.key, rights)?;
                    plain(e.value, rights)?;
                }
                false
            }
            ExpressionOperation::Field { value, selector } => {
                if let ExpressionOperation::Local { value: local } = self.expression(*value)?
                    && rights.memory.contains(local)
                {
                    if !rights.owned.contains(local) && !rights.borrowed.contains(local) {
                        return None;
                    }
                    let ty = self.local_type(*local)?;
                    if !self.product_shape(ty) {
                        return None;
                    }
                    let (TypeForm::OwnedProduct { fields }, FieldSelector::Structural(name)) =
                        (self.form(ty)?, selector)
                    else {
                        return None;
                    };
                    let field = fields.iter().find(|f| &f.name == name)?;
                    if !self.ordinary(field.ty) {
                        return None;
                    }
                    // A read grants no new owner and leaves the original rights intact.
                } else {
                    plain(*value, rights)?;
                }
                false
            }
            ExpressionOperation::CapabilityCall { arguments, .. } => {
                for e in arguments {
                    plain(*e, rights)?;
                }
                false
            }
            ExpressionOperation::Transaction { body, .. }
            | ExpressionOperation::TransactionOutcome { body, .. } => {
                self.run(*body, rights, take, depth + 1)?
            }
            ExpressionOperation::Match { value, arms } => {
                plain(*value, rights)?;
                let mut result: Option<(Rights, bool)> = None;
                for arm in arms {
                    let mut path = rights.clone();
                    let v = self.run(arm.body, &mut path, take, depth + 1)?;
                    if let Some((prior, pv)) = &mut result {
                        if *pv != v
                            || prior.borrowed != path.borrowed
                            || prior.memory != path.memory
                        {
                            return None;
                        }
                        prior.owned = prior.owned.intersection(&path.owned).copied().collect();
                    } else {
                        result = Some((path, v));
                    }
                }
                let (r, v) = result?;
                *rights = r;
                v
            }
            ExpressionOperation::Unit {}
            | ExpressionOperation::Bool { .. }
            | ExpressionOperation::I64 { .. }
            | ExpressionOperation::F64 { .. }
            | ExpressionOperation::Text { .. }
            | ExpressionOperation::StaticText { .. }
            | ExpressionOperation::Local { .. }
            | ExpressionOperation::Constant { .. } => false,
        };
        if output && !take { None } else { Some(output) }
    }
}
pub(crate) fn accepts(snapshot: &KernelSnapshot) -> bool {
    let oracle = Oracle(snapshot, None);
    // Imported parameter identities live in their declaration's package. Check
    // every exported template, even when no root expression calls it.
    for (package, dependency) in &snapshot.dependencies {
        let Some(interface) = snapshot
            .dependency_interfaces
            .get(&dependency.package_revision)
        else {
            return false;
        };
        for (key, owner) in interface {
            match owner {
                PackageInterfaceRecord::TypeParameter(p)
                    if p.constraints.has_owned() || p.constraints.requires_transfer() =>
                {
                    let Some(PackageInterfaceRecord::Declaration(d)) =
                        interface.get(&OwnerKey::Declaration(p.declaration))
                    else {
                        return false;
                    };
                    match &d.payload {
                        PackageInterfaceDeclarationPayload::OwnedContract(c)
                            if !p.constraints.requires_transfer()
                                && *key == OwnerKey::TypeParameter(c.self_parameter) => {}
                        PackageInterfaceDeclarationPayload::Function(f)
                            if matches!(key, OwnerKey::TypeParameter(id) if f.type_parameters.contains(id))
                                && (!p.constraints.requires_transfer()
                                    || p.header.contract_version >= 22) => {}
                        _ => return false,
                    }
                }
                PackageInterfaceRecord::Declaration(d) => {
                    let OwnerKey::Declaration(id) = key else {
                        return false;
                    };
                    if let PackageInterfaceDeclarationPayload::Function(f) = &d.payload {
                        let function = DeclarationReference {
                            package: *package,
                            declaration: *id,
                        };
                        let imported = Oracle(snapshot, Some(function));
                        if !imported.valid_parameters(function, f)
                            || imported.signature(function).is_none_or(|s| {
                                !imported.legal_signature(&s)
                                    || s.0.iter().map(|p| p.ty).chain([s.1]).any(|ty| {
                                        imported.buffer(ty) && !imported.owned_type_in_scope(ty)
                                    })
                                    || s.0
                                        .iter()
                                        .any(|p| p.parent != ParameterParent::Function(*id))
                            })
                        {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut product_bindings = BTreeSet::new();
    for owner in snapshot.owners.values() {
        if let OwnerRecord::Expression(e) = owner
            && let ExpressionOperation::UnpackOwned { fields, .. } = &e.operation
            && fields
                .iter()
                .any(|field| !product_bindings.insert(field.binding))
        {
            return false;
        }
        if let OwnerRecord::Expression(e) = owner
            && let ExpressionOperation::MatchOwned { arms, .. } = &e.operation
            && arms.iter().any(|arm| !product_bindings.insert(arm.binding))
        {
            return false;
        }
        let generation = snapshot
            .root
            .graph_contract_version
            .min(owner.header().contract_version);
        if generation < 21
            && matches!(owner, OwnerRecord::Expression(e) if matches!(e.operation, ExpressionOperation::Parallel { .. }))
        {
            return false;
        }
        if generation < 22
            && matches!(owner, OwnerRecord::TypeParameter(p) if p.constraints.requires_transfer())
        {
            return false;
        }
        if generation >= 20 {
            continue;
        }
        let mut pending = owner.type_roots();
        let mut visited = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            if !visited.insert(ty) {
                continue;
            }
            let Some(object) = snapshot
                .types
                .get(&ty)
                .or_else(|| snapshot.dependency_types.get(&ty))
            else {
                return false;
            };
            if matches!(object.form, TypeForm::OwnedChoice { .. })
                || generation < 19 && matches!(object.form, TypeForm::OwnedProduct { .. })
            {
                return false;
            }
            pending.extend(object.child_types());
        }
    }
    if snapshot
        .types
        .values()
        .chain(snapshot.dependency_types.values())
        .any(|t| {
            !matches!(
                t.form,
                TypeForm::OwnedProduct { .. } | TypeForm::OwnedChoice { .. }
            ) && t.child_types().iter().any(|t| oracle.contains(*t))
        })
    {
        return false;
    }
    for (key, owner) in &snapshot.owners {
        match owner {
            OwnerRecord::TypeParameter(p)
                if p.constraints.has_owned() || p.constraints.requires_transfer() =>
            {
                let Some(OwnerRecord::Declaration(d)) =
                    snapshot.owners.get(&OwnerKey::Declaration(p.declaration))
                else {
                    return false;
                };
                match &d.payload {
                    DeclarationPayload::OwnedContract(c)
                        if !p.constraints.requires_transfer()
                            && *key == OwnerKey::TypeParameter(c.self_parameter) => {}
                    DeclarationPayload::Function(f) if matches!(key, OwnerKey::TypeParameter(id) if f.type_parameters.contains(id)) =>
                        {}
                    _ => return false,
                }
            }
            OwnerRecord::Port(p) => {
                if oracle.contains(p.function_type) {
                    return false;
                }
                if let PortImplementation::Expression(e) = p.implementation
                    && oracle.run(e, &mut Rights::default(), false, 0) != Some(false)
                {
                    return false;
                }
            }
            OwnerRecord::Field(f) if oracle.contains(f.ty) => return false,
            OwnerRecord::Case(c) if c.payload.is_some_and(|t| oracle.contains(t)) => return false,
            OwnerRecord::Operation(o) if oracle.contains(o.result) => return false,
            OwnerRecord::Parameter(p)
                if oracle.buffer(p.ty) && !matches!(p.parent, ParameterParent::Function(_)) =>
            {
                return false;
            }
            OwnerRecord::Declaration(d) => match &d.payload {
                DeclarationPayload::OwnedContract(_) => {
                    let OwnerKey::Declaration(id) = key else {
                        return false;
                    };
                    if !oracle.valid_contract(DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *id,
                    }) {
                        return false;
                    }
                }
                DeclarationPayload::OwnedImplementation(i) => {
                    if !oracle.valid_implementation(i) {
                        return false;
                    }
                }
                DeclarationPayload::Constant { ty, value } => {
                    if oracle.contains(*ty)
                        || oracle.run(*value, &mut Rights::default(), false, 0) != Some(false)
                    {
                        return false;
                    }
                }
                DeclarationPayload::Test {
                    actual, expected, ..
                } => {
                    if oracle.run(*actual, &mut Rights::default(), false, 0) != Some(false)
                        || oracle.run(*expected, &mut Rights::default(), false, 0) != Some(false)
                    {
                        return false;
                    }
                }
                DeclarationPayload::Function(f) => {
                    let OwnerKey::Declaration(id) = key else {
                        return false;
                    };
                    let oracle = Oracle(
                        snapshot,
                        Some(DeclarationReference {
                            package: snapshot.root.package_id,
                            declaration: *id,
                        }),
                    );
                    let function = DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *id,
                    };
                    if oracle
                        .function(function)
                        .is_none_or(|f| !oracle.valid_parameters(function, &f))
                    {
                        return false;
                    }
                    let Some(s) = oracle.signature(DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *id,
                    }) else {
                        return false;
                    };
                    if !oracle.legal_signature(&s) {
                        return false;
                    }
                    if s.0
                        .iter()
                        .map(|p| p.ty)
                        .chain([s.1])
                        .any(|ty| oracle.buffer(ty) && !oracle.owned_type_in_scope(ty))
                    {
                        return false;
                    }
                    let mut rights = Rights::default();
                    for p in &s.0 {
                        if oracle.buffer(p.ty) {
                            let OwnerKey::Parameter(id) = p.header.owner else {
                                return false;
                            };
                            let r = LocalValueReference::FunctionParameter(id);
                            if !rights.memory.insert(r) {
                                return false;
                            }
                            if p.use_mode == ParameterUse::Borrow {
                                rights.borrowed.insert(r);
                            } else {
                                rights.owned.insert(r);
                            }
                        }
                    }
                    if oracle.run(f.body, &mut rights, oracle.buffer(f.result), 0)
                        != Some(oracle.buffer(f.result))
                    {
                        return false;
                    }
                }
                DeclarationPayload::External(_) => {
                    let OwnerKey::Declaration(id) = key else {
                        return false;
                    };
                    if !oracle
                        .signature(DeclarationReference {
                            package: snapshot.root.package_id,
                            declaration: *id,
                        })
                        .is_some_and(|s| oracle.legal_signature(&s))
                    {
                        return false;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    true
}
