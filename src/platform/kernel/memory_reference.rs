//! Test-only symbolic memory oracle. It reads canonical inventories, never the production
//! memory checker, compiler, prepared classifications, or runtime tokens.
use super::*;
use crate::platform::semantic_id::ExpressionId;
use std::collections::BTreeSet;

#[derive(Clone, Default, Eq, PartialEq)]
struct Rights {
    // Identity survives consumption until lexical exit: a moved owner is not ordinary data.
    memory: BTreeSet<LocalValueReference>,
    owned: BTreeSet<LocalValueReference>,
    borrowed: BTreeSet<LocalValueReference>,
}
struct Oracle<'a>(&'a KernelSnapshot);
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
            Some(TypeForm::ByteBuffer)
        )
    }
    fn contains(&self, t: TypeObjectDigest) -> bool {
        let mut todo = vec![t];
        let mut seen = BTreeSet::new();
        while let Some(t) = todo.pop() {
            if !seen.insert(t) {
                continue;
            }
            let Some(t) = self
                .0
                .types
                .get(&t)
                .or_else(|| self.0.dependency_types.get(&t))
            else {
                return true;
            };
            if matches!(t.form, TypeForm::ByteBuffer) {
                return true;
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
        let mut suffix = false;
        for p in parameters {
            if self.buffer(p.ty) {
                suffix = true;
                if !pure
                    || p.resource_requirement.is_some()
                    || p.use_mode == ParameterUse::Unrestricted
                {
                    return false;
                }
            } else if suffix || self.contains(p.ty) {
                return false;
            }
        }
        let memory = suffix || self.buffer(*result);
        if memory && (!pure || parameters.iter().any(|p| p.resource_requirement.is_some())) {
            return false;
        }
        if !self.buffer(*result) && self.contains(*result) {
            return false;
        }
        if let Some(i) = implementation
            && memory
        {
            let expected = match i.as_str() {
                "core.buffer.empty" => (0, false, true),
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
    fn expression(&self, id: ExpressionId) -> Option<&ExpressionOperation> {
        let OwnerRecord::Expression(e) = self.0.owners.get(&OwnerKey::Expression(id))? else {
            return None;
        };
        Some(&e.operation)
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
                if type_arguments.iter().any(|t| self.contains(*t)) {
                    return None;
                }
                let Some(s) = self.signature(*function) else {
                    for a in arguments {
                        plain(*a, rights)?;
                    }
                    return Some(false);
                };
                if !self.legal_signature(&s) || s.0.len() != arguments.len() {
                    return None;
                }
                let mut moved = BTreeSet::new();
                let mut read = BTreeSet::new();
                for (p, a) in s.0.iter().zip(arguments) {
                    if self.buffer(p.ty) {
                        let ExpressionOperation::Local { value } = self.expression(*a)? else {
                            return None;
                        };
                        if p.use_mode == ParameterUse::Borrow {
                            if !rights.owned.contains(value) && !rights.borrowed.contains(value) {
                                return None;
                            }
                            read.insert(*value);
                        } else {
                            if !moved.insert(*value) || !rights.owned.remove(value) {
                                return None;
                            }
                        }
                    } else {
                        self.run(*a, rights, false, depth + 1).filter(|v| !*v)?;
                    }
                }
                if !moved.is_disjoint(&read) {
                    return None;
                }
                self.buffer(s.1)
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                if type_arguments.iter().any(|t| self.contains(*t)) {
                    return None;
                }
                let s = self.signature(*function)?;
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
            ExpressionOperation::Field { value, .. } => {
                plain(*value, rights)?;
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
            _ => false,
        };
        if output && !take { None } else { Some(output) }
    }
}
pub(crate) fn accepts(snapshot: &KernelSnapshot) -> bool {
    let oracle = Oracle(snapshot);
    if snapshot
        .types
        .values()
        .chain(snapshot.dependency_types.values())
        .any(|t| t.child_types().iter().any(|t| oracle.contains(*t)))
    {
        return false;
    }
    for (key, owner) in &snapshot.owners {
        match owner {
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
                    let Some(s) = oracle.signature(DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *id,
                    }) else {
                        return false;
                    };
                    if !oracle.legal_signature(&s) {
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
