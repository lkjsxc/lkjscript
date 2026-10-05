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
    // A projected view retains the complete source custody chain. Function
    // borrow parameters have read rights but no invented caller-local ancestor.
    provenance: BTreeMap<LocalValueReference, LocalValueReference>,
    loans: BTreeMap<LocalValueReference, usize>,
}
impl Rights {
    fn root(&self, local: LocalValueReference) -> Option<LocalValueReference> {
        let mut next = local;
        let mut seen = BTreeSet::new();
        loop {
            if !self.readable(next) || !seen.insert(next) {
                return None;
            }
            match self.provenance.get(&next) {
                Some(parent) if self.borrowed.contains(&next) => next = *parent,
                Some(_) => return None,
                None => return Some(next),
            }
        }
    }
    fn readable(&self, local: LocalValueReference) -> bool {
        self.memory.contains(&local)
            && (self.owned.contains(&local) || self.borrowed.contains(&local))
    }
    fn loan(&mut self, source: LocalValueReference) -> Option<Vec<LocalValueReference>> {
        let mut ancestry = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = source;
        loop {
            if !self.readable(next) || !seen.insert(next) {
                return None;
            }
            ancestry.push(next);
            match self.provenance.get(&next) {
                Some(parent) if self.borrowed.contains(&next) => next = *parent,
                Some(_) => return None,
                None => break,
            }
        }
        for local in &ancestry {
            let count = self.loans.entry(*local).or_default();
            *count = count.checked_add(1)?;
        }
        Some(ancestry)
    }
    fn release(&mut self, ancestry: Vec<LocalValueReference>) -> Option<()> {
        for local in ancestry {
            let count = self.loans.get_mut(&local)?;
            *count = count.checked_sub(1)?;
            if *count == 0 {
                self.loans.remove(&local);
            }
        }
        Some(())
    }
    fn consume(&mut self, local: LocalValueReference) -> Option<()> {
        if self.loans.contains_key(&local) || !self.owned.remove(&local) {
            return None;
        }
        Some(())
    }
    fn view(&mut self, local: LocalValueReference, source: LocalValueReference) -> Option<()> {
        if !self.readable(source)
            || !self.memory.insert(local)
            || !self.borrowed.insert(local)
            || self.provenance.insert(local, source).is_some()
        {
            return None;
        }
        Some(())
    }
    fn finish_view(&mut self, local: LocalValueReference) -> Option<()> {
        if self.owned.contains(&local)
            || self.loans.contains_key(&local)
            || !self.memory.remove(&local)
            || !self.borrowed.remove(&local)
            || self.provenance.remove(&local).is_none()
        {
            return None;
        }
        Some(())
    }
}
#[path = "borrowed_result_memory_oracle_tests.rs"]
mod borrowed_result_tests;
#[path = "borrowed_memory_oracle_tests.rs"]
mod borrowed_tests;
#[path = "generic_implementation_memory_oracle_tests.rs"]
mod generic_implementation_tests;
#[path = "implementation_effect_memory_oracle_tests.rs"]
mod implementation_effect_tests;
#[path = "imported_memory_oracle_tests.rs"]
mod imported_tests;
#[path = "parameterized_contract_memory_oracle_tests.rs"]
mod parameterized_contract_tests;
#[path = "sequence_memory_oracle_tests.rs"]
mod sequence_tests;
#[path = "transfer_memory_oracle_tests.rs"]
mod transfer_tests;
#[path = "memory_witness_reference.rs"]
mod witnesses;
struct Oracle<'a>(&'a KernelSnapshot, Option<DeclarationReference>);
#[derive(Clone, Copy)]
enum Demand {
    Data,
    Owner,
    Borrow {
        source: LocalValueReference,
        ty: TypeObjectDigest,
    },
}
impl Oracle<'_> {
    fn type_generation(&self, roots: Vec<TypeObjectDigest>, generation: u16) -> bool {
        if generation >= 25 {
            return true;
        }
        let mut pending = roots;
        let mut visited = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            if !visited.insert(ty) {
                continue;
            }
            let Some(object) = self
                .0
                .types
                .get(&ty)
                .or_else(|| self.0.dependency_types.get(&ty))
            else {
                return false;
            };
            if matches!(object.form, TypeForm::OwnedSequence { .. })
                || generation < 20 && matches!(object.form, TypeForm::OwnedChoice { .. })
                || generation < 19 && matches!(object.form, TypeForm::OwnedProduct { .. })
            {
                return false;
            }
            pending.extend(object.child_types());
        }
        true
    }
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
                    | TypeForm::OwnedSequence { .. }
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
            Some(
                TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. },
            ) => self.product_shape(ty),
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
            let fields = match self.form(ty) {
                Some(TypeForm::OwnedSequence { item }) => {
                    // Dynamic cardinality adds one structural level. Every item
                    // must independently carry exact ownership, including T's scope.
                    match self.form(*item) {
                        Some(
                            TypeForm::OwnedProduct { .. }
                            | TypeForm::OwnedChoice { .. }
                            | TypeForm::OwnedSequence { .. },
                        ) => pending.push((*item, depth + 1)),
                        Some(TypeForm::ByteBuffer | TypeForm::OwnedI64Cell) => {}
                        Some(TypeForm::TypeParameter { .. }) if self.owned_type_in_scope(*item) => {
                        }
                        _ => return false,
                    }
                    continue;
                }
                Some(
                    TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields },
                ) => fields,
                _ => return false,
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
                    Some(
                        TypeForm::OwnedProduct { .. }
                        | TypeForm::OwnedChoice { .. }
                        | TypeForm::OwnedSequence { .. },
                    ) => {
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
    fn result_source(
        &self,
        function: DeclarationReference,
    ) -> Option<Option<crate::platform::semantic_id::ParameterId>> {
        if let Some(signature) = self.function(function) {
            return Some(signature.result_borrow);
        }
        self.signature(function).map(|_| None)
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
    fn exact_application_type(
        &self,
        template: TypeObjectDigest,
        actual: TypeObjectDigest,
        substitutions: &BTreeMap<crate::platform::semantic_id::TypeParameterId, TypeObjectDigest>,
        depth: usize,
        remaining: &mut usize,
    ) -> Option<()> {
        if depth > contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        *remaining = remaining.checked_sub(1)?;
        let expected = self.form(template)?;
        if let TypeForm::TypeParameter { parameter } = expected {
            return (actual == substitutions.get(parameter).copied().unwrap_or(template))
                .then_some(());
        }
        let observed = self.form(actual)?;
        if std::mem::discriminant(expected) != std::mem::discriminant(observed) {
            return None;
        }
        let children = match (expected, observed) {
            (TypeForm::OwnedSequence { item: a }, TypeForm::OwnedSequence { item: b })
            | (TypeForm::List { item: a }, TypeForm::List { item: b })
            | (TypeForm::Option { item: a }, TypeForm::Option { item: b })
            | (TypeForm::Stream { item: a }, TypeForm::Stream { item: b }) => vec![(*a, *b)],
            (TypeForm::OwnedProduct { fields: a }, TypeForm::OwnedProduct { fields: b })
            | (TypeForm::OwnedChoice { cases: a }, TypeForm::OwnedChoice { cases: b })
            | (
                TypeForm::StructuralRecord { fields: a },
                TypeForm::StructuralRecord { fields: b },
            ) => {
                if a.len() != b.len() || a.iter().zip(b).any(|(a, b)| a.name != b.name) {
                    return None;
                }
                a.iter().zip(b).map(|(a, b)| (a.ty, b.ty)).collect()
            }
            (TypeForm::Map { key: a, value: av }, TypeForm::Map { key: b, value: bv })
            | (TypeForm::Result { ok: a, error: av }, TypeForm::Result { ok: b, error: bv }) => {
                vec![(*a, *b), (*av, *bv)]
            }
            (
                TypeForm::Function {
                    parameters: a,
                    result: ar,
                },
                TypeForm::Function {
                    parameters: b,
                    result: br,
                },
            ) => {
                if a.len() != b.len() {
                    return None;
                }
                a.iter()
                    .copied()
                    .zip(b.iter().copied())
                    .chain([(*ar, *br)])
                    .collect()
            }
            (
                TypeForm::TaskFunction {
                    parameters: a,
                    result: ar,
                    effect: ae,
                },
                TypeForm::TaskFunction {
                    parameters: b,
                    result: br,
                    effect: be,
                },
            ) => {
                if a.len() != b.len() || ae != be {
                    return None;
                }
                a.iter()
                    .copied()
                    .zip(b.iter().copied())
                    .chain([(*ar, *br)])
                    .collect()
            }
            (
                TypeForm::Applied {
                    declaration: a,
                    arguments: aa,
                },
                TypeForm::Applied {
                    declaration: b,
                    arguments: ba,
                },
            ) => {
                if a != b || aa.len() != ba.len() {
                    return None;
                }
                aa.iter().copied().zip(ba.iter().copied()).collect()
            }
            _ => return (expected == observed).then_some(()),
        };
        for (template, actual) in children {
            self.exact_application_type(template, actual, substitutions, depth + 1, remaining)?;
        }
        Some(())
    }

    fn application_bindings(
        &self,
        function: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Option<BTreeMap<crate::platform::semantic_id::TypeParameterId, TypeObjectDigest>> {
        let parameters = self.type_parameters(function)?;
        if parameters.len() != arguments.len() {
            return None;
        }
        Some(
            parameters
                .into_iter()
                .zip(arguments.iter().copied())
                .collect(),
        )
    }
    fn borrow_binding(
        &self,
        id: crate::platform::semantic_id::BindingId,
        ty: TypeObjectDigest,
    ) -> bool {
        matches!(
            self.0.owners.get(&OwnerKey::Binding(id)),
            Some(OwnerRecord::Binding(binding))
                if binding.header.owner == OwnerKey::Binding(id)
                    && binding.header.contract_version >= 24
                    && binding.kind == BindingKind::OwnedBorrow
                    && binding.value.is_none()
                    && binding.declared_type == Some(ty)
        )
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
            TypeForm::OwnedSequence { item } => self
                .parallel_shape(*item, bindings, depth + 1, remaining)?
                .then_some(true),
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
        parameters: &[(TypeObjectDigest, TypeObjectDigest, ParameterUse)],
        arguments: &[ExpressionId],
        rights: &mut Rights,
        depth: usize,
        substitutions: &BTreeMap<crate::platform::semantic_id::TypeParameterId, TypeObjectDigest>,
    ) -> Option<()> {
        if parameters.len() != arguments.len() {
            return None;
        }
        let mut moved = BTreeSet::new();
        let mut borrowed = BTreeSet::new();
        let initial_loans = rights.loans.clone();
        let mut loans = Vec::new();
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        for ((template, ty, use_mode), argument) in parameters.iter().zip(arguments) {
            if self.buffer(*ty) {
                let ExpressionOperation::Local { value } = self.expression(*argument)? else {
                    return None;
                };
                // Compare the actual local with the complete generic template
                // without requiring a synthesized type to be retained in source.
                self.exact_application_type(
                    *template,
                    self.local_type(*value)?,
                    substitutions,
                    0,
                    &mut remaining,
                )?;
                match use_mode {
                    ParameterUse::Borrow => {
                        borrowed.insert(*value);
                        loans.push(rights.loan(*value)?);
                    }
                    ParameterUse::Consume => {
                        if !moved.insert(*value) {
                            return None;
                        }
                        rights.consume(*value)?;
                    }
                    ParameterUse::Unrestricted => return None,
                }
            } else if self.run(*argument, rights, false, depth + 1)? {
                return None;
            }
        }
        for loan in loans.into_iter().rev() {
            rights.release(loan)?;
        }
        (moved.is_disjoint(&borrowed) && rights.loans == initial_loans).then_some(())
    }
    // Invocation resolution is reconstructed from canonical signatures. In
    // particular, the source is a parameter position, never an allocation or
    // an interchangeable local with the same type.
    fn borrow_invocation(
        &self,
        call: ExpressionId,
        ty: TypeObjectDigest,
        rights: &mut Rights,
        depth: usize,
    ) -> Option<(LocalValueReference, Vec<LocalValueReference>)> {
        let (parameters, arguments, source, result, substitutions) = match self.expression(call)? {
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
                requirement_arguments,
                effect_arguments,
            } => {
                let f = self.function(*function)?;
                if !f.implementation_parameters.is_empty()
                    || !self.valid_parameters(*function, &f)
                    || !requirement_arguments.is_empty()
                    || !effect_arguments.is_empty()
                {
                    return None;
                }
                let selected = f.result_borrow?;
                let source = f.parameters.iter().position(|p| *p == selected)?;
                let template = self.signature(*function)?;
                let applied = self.applied_signature(*function, type_arguments)?;
                if !self.legal_signature(&applied) {
                    return None;
                }
                (
                    template
                        .0
                        .iter()
                        .zip(&applied.0)
                        .map(|(t, p)| (t.ty, p.ty, p.use_mode))
                        .collect::<Vec<_>>(),
                    arguments,
                    source,
                    f.result,
                    self.application_bindings(*function, type_arguments)?,
                )
            }
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                implementations,
                arguments,
                requirement_arguments,
                effect_arguments,
            } => {
                let f = self.function(*function)?;
                if !self.valid_parameters(*function, &f)
                    || !self.witnesses(*function, type_arguments, implementations)
                    || !requirement_arguments.is_empty()
                    || !effect_arguments.is_empty()
                {
                    return None;
                }
                let selected = f.result_borrow?;
                let source = f.parameters.iter().position(|p| *p == selected)?;
                let template = self.signature(*function)?;
                let applied = self.applied_signature(*function, type_arguments)?;
                if !self.legal_signature(&applied) {
                    return None;
                }
                (
                    template
                        .0
                        .iter()
                        .zip(&applied.0)
                        .map(|(t, p)| (t.ty, p.ty, p.use_mode))
                        .collect::<Vec<_>>(),
                    arguments,
                    source,
                    f.result,
                    self.application_bindings(*function, type_arguments)?,
                )
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
                let (m, substitutions) = self.method(witness.clone(), *contract, *method)?;
                let source = m.result_borrow? as usize;
                (
                    m.parameters
                        .iter()
                        .map(|p| (p.ty, self.method_type(p.ty, &substitutions), p.use_mode))
                        .collect::<Vec<_>>(),
                    arguments,
                    source,
                    m.result,
                    substitutions,
                )
            }
            _ => return None,
        };
        if parameters.get(source)?.2 != ParameterUse::Borrow {
            return None;
        }
        let ExpressionOperation::Local { value: local } =
            self.expression(*arguments.get(source)?)?
        else {
            return None;
        };
        let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
        self.exact_application_type(result, ty, &substitutions, 0, &mut remaining)?;
        self.arguments(&parameters, arguments, rights, depth, &substitutions)?;
        Some((*local, rights.loan(*local)?))
    }
    // Ordinary evaluation distinguishes data and transferred owners. A
    // borrowed-return demand additionally requires every terminal path to
    // establish the exact nominated root, while all preceding operations keep
    // their ordinary consumption rules.
    fn run(&self, id: ExpressionId, rights: &mut Rights, take: bool, depth: usize) -> Option<bool> {
        self.flow(
            id,
            rights,
            if take { Demand::Owner } else { Demand::Data },
            depth,
        )
    }
    fn flow(
        &self,
        id: ExpressionId,
        rights: &mut Rights,
        demand: Demand,
        depth: usize,
    ) -> Option<bool> {
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return None;
        }
        let take = matches!(demand, Demand::Owner);
        if matches!(demand, Demand::Borrow { .. })
            && !matches!(
                self.expression(id)?,
                ExpressionOperation::Local { .. }
                    | ExpressionOperation::Let { .. }
                    | ExpressionOperation::Sequence { .. }
                    | ExpressionOperation::If { .. }
                    | ExpressionOperation::Match { .. }
                    | ExpressionOperation::MatchOwned { .. }
                    | ExpressionOperation::UnpackOwned { .. }
                    | ExpressionOperation::BorrowOwnedItem { .. }
                    | ExpressionOperation::BorrowOwnedField { .. }
                    | ExpressionOperation::MatchBorrowedOwned { .. }
                    | ExpressionOperation::BorrowCall { .. }
            )
        {
            return None;
        }
        let plain = |id, rights: &mut Rights| {
            self.run(id, rights, false, depth + 1)
                .filter(|v| !*v)
                .map(|_| ())
        };
        let output = match self.expression(id)? {
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                if !matches!(
                    self.form(*sequence_type),
                    Some(TypeForm::OwnedSequence { .. })
                ) || !self.product_shape(*sequence_type)
                {
                    return None;
                }
                true
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                if !matches!(
                    self.form(*sequence_type),
                    Some(TypeForm::OwnedSequence { .. })
                ) || !self.product_shape(*sequence_type)
                {
                    return None;
                }
                let ExpressionOperation::Local { value } = self.expression(*source)? else {
                    return None;
                };
                if self.local_type(*value) != Some(*sequence_type) || !rights.readable(*value) {
                    return None;
                }
                false
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                let TypeForm::OwnedSequence { item } = self.form(*sequence_type)? else {
                    return None;
                };
                if !self.product_shape(*sequence_type) {
                    return None;
                }
                let ExpressionOperation::Local { value: element } = self.expression(*value)? else {
                    return None;
                };
                let ExpressionOperation::Local { value: sequence } = self.expression(*source)?
                else {
                    return None;
                };
                if self.local_type(*element) != Some(*item)
                    || self.local_type(*sequence) != Some(*sequence_type)
                {
                    return None;
                }
                // Element consumption precedes source consumption.
                rights.consume(*element)?;
                rights.consume(*sequence)?;
                true
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                let TypeForm::OwnedSequence { item } = self.form(*sequence_type)? else {
                    return None;
                };
                if !self.product_shape(*sequence_type) || !self.product_shape(*result_type) {
                    return None;
                }
                let TypeForm::OwnedChoice { cases } = self.form(*result_type)? else {
                    return None;
                };
                if cases.len() != 2
                    || cases[0].name.as_str() != "empty"
                    || cases[1].name.as_str() != "item"
                    || cases[0].ty != *sequence_type
                {
                    return None;
                }
                let TypeForm::OwnedProduct { fields } = self.form(cases[1].ty)? else {
                    return None;
                };
                if fields.len() != 2
                    || fields[0].name.as_str() != "rest"
                    || fields[1].name.as_str() != "value"
                    || fields[0].ty != *sequence_type
                    || fields[1].ty != *item
                {
                    return None;
                }
                let ExpressionOperation::Local { value } = self.expression(*source)? else {
                    return None;
                };
                if self.local_type(*value) != Some(*sequence_type) {
                    return None;
                }
                rights.consume(*value)?;
                true
            }
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                let TypeForm::OwnedSequence { item } = self.form(*sequence_type)? else {
                    return None;
                };
                if !self.product_shape(*sequence_type) || !self.borrow_binding(*binding, *item) {
                    return None;
                }
                plain(*index, rights)?;
                let ExpressionOperation::Local { value: source } = self.expression(*source)? else {
                    return None;
                };
                if self.local_type(*source) != Some(*sequence_type) {
                    return None;
                }
                let before = rights.clone();
                let ancestry = rights.loan(*source)?;
                let view = LocalValueReference::LexicalBinding(*binding);
                rights.view(view, *source)?;
                let result = self.flow(*body, rights, demand, depth + 1)?;
                rights.finish_view(view)?;
                rights.release(ancestry)?;
                if rights.memory != before.memory
                    || rights.borrowed != before.borrowed
                    || rights.provenance != before.provenance
                    || rights.loans != before.loans
                {
                    return None;
                }
                result
            }
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => {
                if !self.product_shape(*product_type) {
                    return None;
                }
                let TypeForm::OwnedProduct { fields } = self.form(*product_type)? else {
                    return None;
                };
                let selected = fields.iter().find(|item| &item.name == field)?;
                if !self.buffer(selected.ty) || !self.owned_type_in_scope(selected.ty) {
                    return None;
                }
                let ExpressionOperation::Local { value: source } = self.expression(*source)? else {
                    return None;
                };
                if self.local_type(*source) != Some(*product_type)
                    || !self.borrow_binding(*binding, selected.ty)
                {
                    return None;
                }
                let before = rights.clone();
                let ancestry = rights.loan(*source)?;
                let view = LocalValueReference::LexicalBinding(*binding);
                rights.view(view, *source)?;
                let result = self.flow(*body, rights, demand, depth + 1)?;
                rights.finish_view(view)?;
                rights.release(ancestry)?;
                if rights.memory != before.memory
                    || rights.borrowed != before.borrowed
                    || rights.provenance != before.provenance
                    || rights.loans != before.loans
                {
                    return None;
                }
                result
            }
            ExpressionOperation::MatchBorrowedOwned {
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
                let ExpressionOperation::Local { value: source } = self.expression(*source)? else {
                    return None;
                };
                if self.local_type(*source) != Some(*choice_type)
                    || arms.len() != cases.len()
                    || arms.is_empty()
                {
                    return None;
                }
                let before = rights.clone();
                let mut names = BTreeSet::new();
                let mut bindings = BTreeSet::new();
                let mut output = None;
                for arm in arms {
                    let case = cases.iter().find(|case| case.name == arm.name)?;
                    if !names.insert(&arm.name)
                        || !bindings.insert(arm.binding)
                        || !self.borrow_binding(arm.binding, case.ty)
                    {
                        return None;
                    }
                    let mut branch = before.clone();
                    let ancestry = branch.loan(*source)?;
                    let view = LocalValueReference::LexicalBinding(arm.binding);
                    let owned = self.buffer(case.ty);
                    if owned {
                        if !self.owned_type_in_scope(case.ty) {
                            return None;
                        }
                        branch.view(view, *source)?;
                    } else if !self.ordinary(case.ty) {
                        return None;
                    }
                    let result = self.flow(arm.body, &mut branch, demand, depth + 1)?;
                    if owned {
                        branch.finish_view(view)?;
                    }
                    branch.release(ancestry)?;
                    if branch.memory != before.memory
                        || branch.borrowed != before.borrowed
                        || branch.provenance != before.provenance
                        || branch.loans != before.loans
                        || output.is_some_and(|prior| prior != result)
                    {
                        return None;
                    }
                    output = Some(result);
                    rights.owned.retain(|owner| branch.owned.contains(owner));
                }
                output?
            }
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
                    let output = self.flow(arm.body, &mut branch, demand, depth + 1)?;
                    branch.memory.remove(&local);
                    branch.owned.remove(&local);
                    if branch.memory != before.memory
                        || branch.borrowed != before.borrowed
                        || branch.provenance != before.provenance
                        || branch.loans != before.loans
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
                let result = self.flow(*body, rights, demand, depth + 1)?;
                for local in scope {
                    rights.memory.remove(&local);
                    rights.owned.remove(&local);
                }
                result
            }
            ExpressionOperation::Local { value } if rights.memory.contains(value) => {
                if let Demand::Borrow { source, ty } = demand {
                    return (rights.borrowed.contains(value)
                        && rights.root(*value) == Some(source)
                        && self.local_type(*value) == Some(ty))
                    .then_some(false);
                }
                if !take {
                    return None;
                }
                rights.consume(*value)?;
                true
            }
            ExpressionOperation::Let { bindings, body } => {
                let mut scope = vec![];
                for id in bindings {
                    let OwnerRecord::Binding(b) = self.0.owners.get(&OwnerKey::Binding(*id))?
                    else {
                        return None;
                    };
                    if b.kind != BindingKind::Let {
                        return None;
                    }
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
                let result = self.flow(*body, rights, demand, depth + 1)?;
                for r in scope {
                    rights.memory.remove(&r);
                    rights.owned.remove(&r);
                }
                result
            }
            ExpressionOperation::Sequence { items } => {
                if items.is_empty() && matches!(demand, Demand::Borrow { .. }) {
                    return None;
                }
                let mut output = false;
                for (i, e) in items.iter().enumerate() {
                    output = self.flow(
                        *e,
                        rights,
                        if i + 1 < items.len() {
                            Demand::Owner
                        } else {
                            demand
                        },
                        depth + 1,
                    )?;
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
                let av = self.flow(*when_true, &mut a, demand, depth + 1)?;
                let bv = self.flow(*when_false, &mut b, demand, depth + 1)?;
                if av != bv {
                    return None;
                }
                a.owned = a.owned.intersection(&b.owned).copied().collect();
                if a.borrowed != b.borrowed
                    || a.memory != b.memory
                    || a.provenance != b.provenance
                    || a.loans != b.loans
                {
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
                if self.function(*function).is_some_and(|f| {
                    !f.implementation_parameters.is_empty() || f.result_borrow.is_some()
                }) || !self.application(*function, type_arguments)
                {
                    return None;
                }
                let Some(template) = self.signature(*function) else {
                    for a in arguments {
                        plain(*a, rights)?;
                    }
                    return Some(false);
                };
                let s = self.applied_signature(*function, type_arguments)?;
                if !self.legal_signature(&s) || s.0.len() != arguments.len() {
                    return None;
                }
                self.arguments(
                    &template
                        .0
                        .iter()
                        .zip(&s.0)
                        .map(|(t, p)| (t.ty, p.ty, p.use_mode))
                        .collect::<Vec<_>>(),
                    arguments,
                    rights,
                    depth,
                    &self.application_bindings(*function, type_arguments)?,
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
                    || self.result_source(*function) != Some(None)
                {
                    return None;
                }
                let s = self.applied_signature(*function, type_arguments)?;
                let template = self.signature(*function)?;
                if !self.legal_signature(&s) {
                    return None;
                }
                self.arguments(
                    &template
                        .0
                        .iter()
                        .zip(&s.0)
                        .map(|(t, p)| (t.ty, p.ty, p.use_mode))
                        .collect::<Vec<_>>(),
                    arguments,
                    rights,
                    depth,
                    &self.application_bindings(*function, type_arguments)?,
                )?;
                self.buffer(s.1)
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
                let (m, bindings) = self.method(witness.clone(), *contract, *method)?;
                if m.result_borrow.is_some() {
                    return None;
                }
                self.arguments(
                    &m.parameters
                        .iter()
                        .map(|p| (p.ty, self.method_type(p.ty, &bindings), p.use_mode))
                        .collect::<Vec<_>>(),
                    arguments,
                    rights,
                    depth,
                    &bindings,
                )?;
                self.buffer(self.method_type(m.result, &bindings))
            }
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                let OwnerRecord::Binding(record) =
                    self.0.owners.get(&OwnerKey::Binding(*binding))?
                else {
                    return None;
                };
                let ty = record.declared_type?;
                if !self.borrow_binding(*binding, ty) || !self.owned_type_in_scope(ty) {
                    return None;
                }
                let (source, ancestry) = self.borrow_invocation(*call, ty, rights, depth)?;
                let before = rights.clone();
                let view = LocalValueReference::LexicalBinding(*binding);
                rights.view(view, source)?;
                let result = self.flow(*body, rights, demand, depth + 1)?;
                rights.finish_view(view)?;
                if rights.memory != before.memory
                    || rights.borrowed != before.borrowed
                    || rights.provenance != before.provenance
                    || rights.loans != before.loans
                {
                    return None;
                }
                rights.release(ancestry)?;
                result
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                if self.function(*function).is_some_and(|f| {
                    !f.implementation_parameters.is_empty() || f.result_borrow.is_some()
                }) || type_arguments.iter().any(|t| self.contains(*t))
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
                    let v = self.flow(arm.body, &mut path, demand, depth + 1)?;
                    if let Some((prior, pv)) = &mut result {
                        if *pv != v
                            || prior.borrowed != path.borrowed
                            || prior.memory != path.memory
                            || prior.provenance != path.provenance
                            || prior.loans != path.loans
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
            | ExpressionOperation::Constant { .. } => false,
            ExpressionOperation::Local { value } => {
                if matches!(demand, Demand::Borrow { .. })
                    || self.local_type(*value).is_some_and(|ty| self.buffer(ty))
                {
                    return None;
                }
                false
            }
        };
        if output && !take { None } else { Some(output) }
    }
}
pub(crate) fn accepts(snapshot: &KernelSnapshot) -> bool {
    let Some(derived) = witnesses::materialized_witness_types(snapshot) else {
        return false;
    };
    let mut materialized = snapshot.clone();
    materialized.types.extend(derived);
    let snapshot = &materialized;
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
            let generation = snapshot
                .root
                .graph_contract_version
                .min(dependency.graph_contract_version)
                .min(owner.header().contract_version);
            if let PackageInterfaceRecord::Declaration(declaration) = owner
                && match &declaration.payload {
                    PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                        !oracle.contract_generation(c, generation)
                    }
                    PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                        (generation < 26 && !i.type_arguments.is_empty())
                            || (generation < 28
                                && (!i.type_parameters.is_empty()
                                    || i.methods.iter().any(|m| !m.type_arguments.is_empty())))
                    }
                    PackageInterfaceDeclarationPayload::Function(f) => {
                        (generation < 27 && f.result_borrow.is_some())
                            || generation < 26
                                && f.implementation_parameters
                                    .iter()
                                    .any(|p| !p.type_arguments.is_empty())
                    }
                    _ => false,
                }
            {
                return false;
            }
            let mut roots = owner.type_roots();
            if generation < 25
                && let PackageInterfaceRecord::Declaration(declaration) = owner
                && matches!(
                    declaration.payload,
                    PackageInterfaceDeclarationPayload::Function(_)
                        | PackageInterfaceDeclarationPayload::External(_)
                )
            {
                let OwnerKey::Declaration(id) = key else {
                    return false;
                };
                let Some(signature) = oracle.signature(DeclarationReference {
                    package: *package,
                    declaration: *id,
                }) else {
                    return false;
                };
                roots.extend(signature.0.iter().map(|parameter| parameter.ty));
            }
            if !oracle.type_generation(roots, generation) {
                return false;
            }
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
                            if p.constraints == TypeParameterConstraints::Owned
                                && matches!(key, OwnerKey::TypeParameter(id)
                                    if *id == c.self_parameter || c.type_parameters.contains(id)) =>
                            {}
                        PackageInterfaceDeclarationPayload::Function(f)
                            if matches!(key, OwnerKey::TypeParameter(id) if f.type_parameters.contains(id))
                                && (!p.constraints.requires_transfer()
                                    || p.header.contract_version >= 22) => {}
                        PackageInterfaceDeclarationPayload::OwnedImplementation(i)
                            if p.constraints == TypeParameterConstraints::Owned
                                && matches!(key, OwnerKey::TypeParameter(id) if i.type_parameters.contains(id)) =>
                            {}
                        _ => return false,
                    }
                }
                PackageInterfaceRecord::Declaration(d) => {
                    let OwnerKey::Declaration(id) = key else {
                        return false;
                    };
                    if matches!(
                        d.payload,
                        PackageInterfaceDeclarationPayload::OwnedContract(_)
                    ) && !oracle.valid_contract(DeclarationReference {
                        package: *package,
                        declaration: *id,
                    }) {
                        return false;
                    }
                    if let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = &d.payload
                        && !oracle.valid_implementation(i)
                    {
                        return false;
                    }
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
    let mut borrowed_bindings = BTreeSet::new();
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
        if let OwnerRecord::Expression(e) = owner {
            match &e.operation {
                ExpressionOperation::BorrowOwnedField { binding, .. }
                | ExpressionOperation::BorrowOwnedItem { binding, .. }
                | ExpressionOperation::BorrowCall { binding, .. } => {
                    if !product_bindings.insert(*binding) || !borrowed_bindings.insert(*binding) {
                        return false;
                    }
                }
                ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                    for arm in arms {
                        if !product_bindings.insert(arm.binding)
                            || !borrowed_bindings.insert(arm.binding)
                        {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }
        let generation = snapshot
            .root
            .graph_contract_version
            .min(owner.header().contract_version);
        if let OwnerRecord::Declaration(declaration) = owner
            && match &declaration.payload {
                DeclarationPayload::OwnedContract(c) => !oracle.contract_generation(c, generation),
                DeclarationPayload::OwnedImplementation(i) => {
                    (generation < 26 && !i.type_arguments.is_empty())
                        || (generation < 28
                            && (!i.type_parameters.is_empty()
                                || i.methods.iter().any(|m| !m.type_arguments.is_empty())))
                }
                DeclarationPayload::Function(f) => {
                    (generation < 27 && f.result_borrow.is_some())
                        || generation < 26
                            && f.implementation_parameters
                                .iter()
                                .any(|p| !p.type_arguments.is_empty())
                }
                _ => false,
            }
        {
            return false;
        }
        if generation < 28
            && let OwnerRecord::Expression(expression) = owner
            && match &expression.operation {
                ExpressionOperation::ImplementationCall { implementations, .. } => implementations.iter().any(|operand| {
                    matches!(operand, ImplementationOperand::Concrete { type_arguments, .. } if !type_arguments.is_empty())
                }),
                ExpressionOperation::MethodCall { witness, .. } => {
                    matches!(witness, ImplementationOperand::Concrete { type_arguments, .. } if !type_arguments.is_empty())
                },
                _ => false,
            }
        { return false; }
        if generation < 27
            && matches!(owner, OwnerRecord::Expression(e) if matches!(e.operation, ExpressionOperation::BorrowCall { .. }))
        {
            return false;
        }
        if generation < 25
            && matches!(owner, OwnerRecord::Expression(e) if matches!(e.operation,
                ExpressionOperation::SequenceEmpty { .. }
                | ExpressionOperation::SequenceLength { .. }
                | ExpressionOperation::SequencePush { .. }
                | ExpressionOperation::SequencePop { .. }
                | ExpressionOperation::BorrowOwnedItem { .. }))
        {
            return false;
        }
        if generation < 24
            && (matches!(owner, OwnerRecord::Expression(e) if matches!(e.operation,
                ExpressionOperation::BorrowOwnedField { .. } | ExpressionOperation::MatchBorrowedOwned { .. }))
                || matches!(owner, OwnerRecord::Binding(b) if b.kind == BindingKind::OwnedBorrow))
        {
            return false;
        }
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
        if generation >= 25 {
            continue;
        }
        let mut roots = owner.type_roots();
        if let OwnerRecord::Declaration(declaration) = owner
            && matches!(
                declaration.payload,
                DeclarationPayload::Function(_) | DeclarationPayload::External(_)
            )
        {
            let OwnerKey::Declaration(id) = declaration.header.owner else {
                return false;
            };
            let Some(signature) = oracle.signature(DeclarationReference {
                package: snapshot.root.package_id,
                declaration: id,
            }) else {
                return false;
            };
            roots.extend(signature.0.iter().map(|parameter| parameter.ty));
        }
        if !oracle.type_generation(roots, generation) {
            return false;
        }
    }
    if snapshot.owners.iter().any(|(key, owner)| {
        matches!(owner, OwnerRecord::Binding(b) if b.kind == BindingKind::OwnedBorrow)
            && !matches!(key, OwnerKey::Binding(id) if borrowed_bindings.contains(id))
    }) {
        return false;
    }
    if snapshot
        .types
        .values()
        .chain(snapshot.dependency_types.values())
        .any(|t| {
            !matches!(
                t.form,
                TypeForm::OwnedProduct { .. }
                    | TypeForm::OwnedChoice { .. }
                    | TypeForm::OwnedSequence { .. }
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
                        if p.constraints == TypeParameterConstraints::Owned
                            && matches!(key, OwnerKey::TypeParameter(id)
                                if *id == c.self_parameter || c.type_parameters.contains(id)) => {}
                    DeclarationPayload::Function(f) if matches!(key, OwnerKey::TypeParameter(id) if f.type_parameters.contains(id)) =>
                        {}
                    DeclarationPayload::OwnedImplementation(i)
                        if p.constraints == TypeParameterConstraints::Owned
                            && matches!(key, OwnerKey::TypeParameter(id) if i.type_parameters.contains(id)) =>
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
                    let demand = match f.result_borrow {
                        Some(source) => Demand::Borrow {
                            source: LocalValueReference::FunctionParameter(source),
                            ty: f.result,
                        },
                        None if oracle.buffer(f.result) => Demand::Owner,
                        None => Demand::Data,
                    };
                    if oracle.flow(f.body, &mut rights, demand, 0)
                        != Some(f.result_borrow.is_none() && oracle.buffer(f.result))
                        || !rights.provenance.is_empty()
                        || !rights.loans.is_empty()
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
