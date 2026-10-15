//! Positive substitutions already proved during finite exact preparation.
//! Full binding contexts share one retained key; lookup never allocates or grows.
use super::super::value::ValueOrigin;
use super::*;

type Bindings = BTreeMap<TypeParameterId, TypeObjectDigest>;
type ResolvedTypes = BTreeMap<TypeObjectDigest, TypeObjectDigest>;
type Contexts = BTreeMap<Bindings, ResolvedTypes>;
type Applications = BTreeMap<FunctionIndex, Contexts>;
type ScopedApplications = BTreeMap<FunctionIndex, BTreeMap<ScopeBindings, Contexts>>;

#[derive(Default)]
pub(super) struct TypeLookupBuilder {
    contexts: Contexts,
    applications: Applications,
    scoped_applications: ScopedApplications,
}

#[derive(Clone, Debug)]
pub(in super::super) struct PreparedTypeLookup {
    origin: ValueOrigin,
    contexts: Contexts,
    applications: Applications,
    scoped_applications: ScopedApplications,
}

impl PreparedTypeLookup {
    pub(in super::super) fn empty(origin: ValueOrigin) -> Self {
        Self {
            origin,
            contexts: BTreeMap::new(),
            applications: BTreeMap::new(),
            scoped_applications: BTreeMap::new(),
        }
    }

    pub(in super::super) fn get(
        &self,
        origin: ValueOrigin,
        template: TypeObjectDigest,
        bindings: &Bindings,
    ) -> Result<Option<TypeObjectDigest>, ()> {
        if self.origin != origin {
            return Err(());
        }
        Ok(self
            .contexts
            .get(bindings)
            .and_then(|types| types.get(&template))
            .copied())
    }

    /// The exact function index binds the sealed witness application. Ordinary
    /// bindings alone cannot select an associated member, even for equal outputs.
    pub(in super::super) fn get_application(
        &self,
        origin: ValueOrigin,
        function: FunctionIndex,
        template: TypeObjectDigest,
        bindings: &Bindings,
    ) -> Result<Option<TypeObjectDigest>, ()> {
        if self.origin != origin || function.1 != origin {
            return Err(());
        }
        Ok(self
            .applications
            .get(&function)
            .and_then(|contexts| contexts.get(bindings))
            .and_then(|types| types.get(&template))
            .copied())
    }

    pub(in super::super) fn get_scoped_application_arguments(
        &self,
        origin: ValueOrigin,
        function: FunctionIndex,
        declaration: crate::platform::kernel::DeclarationReference,
        template: TypeObjectDigest,
        bindings: &Bindings,
        (parameters, arguments): (
            &[crate::platform::kernel::ReadScopeParameter],
            &[ReadScopeArgument],
        ),
    ) -> Result<Option<TypeObjectDigest>, ()> {
        if self.origin != origin || function.1 != origin || parameters.len() != arguments.len() {
            return Err(());
        }
        let Some(scoped) = self.scoped_applications.get(&function) else {
            return Ok(None);
        };
        let mut result = None;
        for (scopes, contexts) in scoped {
            if scopes.len() != parameters.len() {
                continue;
            }
            let matches = scopes.iter().all(|(reference, actual)| {
                if reference.binder != ReadScopeBinder::Declaration(declaration) {
                    return false;
                }
                let mut positions = parameters
                    .iter()
                    .enumerate()
                    .filter(|(_, parameter)| parameter.id == reference.parameter);
                let Some((position, _)) = positions.next() else {
                    return false;
                };
                positions.next().is_none() && arguments[position] == *actual
            });
            if matches {
                let found = contexts
                    .get(bindings)
                    .and_then(|types| types.get(&template))
                    .copied();
                if result.is_some() && found.is_some() {
                    return Err(());
                }
                result = found.or(result);
            }
        }
        Ok(result)
    }

    #[cfg(test)]
    pub(in super::super) fn ordinary_for_test(
        origin: ValueOrigin,
        template: TypeObjectDigest,
        result: TypeObjectDigest,
        bindings: &Bindings,
        control: &crate::platform::execution::ExecutionControl,
    ) -> Result<Self, Diagnostic> {
        let mut work = Budget::new(control);
        work.remember_type_substitution(template, result, bindings)?;
        Ok(work.finish_type_lookup(origin))
    }

    #[cfg(test)]
    pub(in super::super) fn corrupt_application(
        &mut self,
        function: FunctionIndex,
        template: TypeObjectDigest,
        bindings: &Bindings,
        result: TypeObjectDigest,
    ) {
        *self
            .applications
            .get_mut(&function)
            .expect("existing application")
            .get_mut(bindings)
            .expect("existing complete bindings")
            .get_mut(&template)
            .expect("existing proved root") = result;
    }
}

impl Budget<'_> {
    pub(super) fn remember_type_substitution(
        &mut self,
        template: TypeObjectDigest,
        result: TypeObjectDigest,
        bindings: &Bindings,
    ) -> Result<(), Diagnostic> {
        step(self)?;
        if let Some(existing) = self
            .type_lookup
            .contexts
            .get(bindings)
            .and_then(|types| types.get(&template))
        {
            return if *existing == result {
                Ok(())
            } else {
                Err(missing())
            };
        }
        // Reserve the outer node, full copied binding key, and inner result node
        // before cloning any key or growing either map. Contexts are never made
        // by multiplying the admitted type universe by callable applications.
        if !self.type_lookup.contexts.contains_key(bindings) {
            self.node::<(Bindings, ResolvedTypes)>()?;
            for _ in bindings {
                step(self)?;
                self.node::<(TypeParameterId, TypeObjectDigest)>()?;
            }
        }
        self.node::<(TypeObjectDigest, TypeObjectDigest)>()?;
        step(self)?;
        if let Some(types) = self.type_lookup.contexts.get_mut(bindings) {
            types.insert(template, result);
        } else {
            let mut types = BTreeMap::new();
            types.insert(template, result);
            self.type_lookup.contexts.insert(bindings.clone(), types);
        }
        Ok(())
    }

    pub(super) fn finish_type_lookup(&mut self, origin: ValueOrigin) -> PreparedTypeLookup {
        PreparedTypeLookup {
            origin,
            contexts: std::mem::take(&mut self.type_lookup.contexts),
            applications: std::mem::take(&mut self.type_lookup.applications),
            scoped_applications: std::mem::take(&mut self.type_lookup.scoped_applications),
        }
    }

    pub(super) fn remember_application_substitution(
        &mut self,
        function: FunctionIndex,
        template: TypeObjectDigest,
        result: TypeObjectDigest,
        bindings: &Bindings,
    ) -> Result<(), Diagnostic> {
        step(self)?;
        if let Some(existing) = self
            .type_lookup
            .applications
            .get(&function)
            .and_then(|contexts| contexts.get(bindings))
            .and_then(|types| types.get(&template))
        {
            return if *existing == result {
                Ok(())
            } else {
                Err(missing())
            };
        }
        // Complete reservations and cancellation checks before publishing any
        // application, binding map or result. A failed proof exposes no entry.
        if !self.type_lookup.applications.contains_key(&function) {
            self.node::<(FunctionIndex, Contexts)>()?;
        }
        if !self
            .type_lookup
            .applications
            .get(&function)
            .is_some_and(|contexts| contexts.contains_key(bindings))
        {
            self.node::<(Bindings, ResolvedTypes)>()?;
            for _ in bindings {
                step(self)?;
                self.node::<(TypeParameterId, TypeObjectDigest)>()?;
            }
        }
        self.node::<(TypeObjectDigest, TypeObjectDigest)>()?;
        step(self)?;
        let contexts = self.type_lookup.applications.entry(function).or_default();
        if let Some(types) = contexts.get_mut(bindings) {
            types.insert(template, result);
        } else {
            contexts.insert(bindings.clone(), BTreeMap::from([(template, result)]));
        }
        Ok(())
    }

    pub(super) fn remember_scoped_application_substitution(
        &mut self,
        function: FunctionIndex,
        template: TypeObjectDigest,
        result: TypeObjectDigest,
        bindings: &Bindings,
        scopes: &ScopeBindings,
    ) -> Result<(), Diagnostic> {
        step(self)?;
        if let Some(existing) = self
            .type_lookup
            .scoped_applications
            .get(&function)
            .and_then(|scoped| scoped.get(scopes))
            .and_then(|contexts| contexts.get(bindings))
            .and_then(|types| types.get(&template))
        {
            return if *existing == result {
                Ok(())
            } else {
                Err(missing())
            };
        }
        if !self.type_lookup.scoped_applications.contains_key(&function) {
            self.node::<(FunctionIndex, BTreeMap<ScopeBindings, Contexts>)>()?;
        }
        if !self
            .type_lookup
            .scoped_applications
            .get(&function)
            .is_some_and(|scoped| scoped.contains_key(scopes))
        {
            self.node::<(ScopeBindings, Contexts)>()?;
            for _ in scopes {
                self.node::<(ReadScopeReference, ReadScopeArgument)>()?;
            }
        }
        if !self
            .type_lookup
            .scoped_applications
            .get(&function)
            .and_then(|scoped| scoped.get(scopes))
            .is_some_and(|contexts| contexts.contains_key(bindings))
        {
            self.node::<(Bindings, ResolvedTypes)>()?;
            for _ in bindings {
                self.node::<(TypeParameterId, TypeObjectDigest)>()?;
            }
        }
        self.node::<(TypeObjectDigest, TypeObjectDigest)>()?;
        if let Some(scoped) = self.type_lookup.scoped_applications.get_mut(&function) {
            if let Some(contexts) = scoped.get_mut(scopes) {
                if let Some(types) = contexts.get_mut(bindings) {
                    types.insert(template, result);
                } else {
                    contexts.insert(bindings.clone(), BTreeMap::from([(template, result)]));
                }
            } else {
                scoped.insert(
                    scopes.clone(),
                    BTreeMap::from([(bindings.clone(), BTreeMap::from([(template, result)]))]),
                );
            }
        } else {
            self.type_lookup.scoped_applications.insert(
                function,
                BTreeMap::from([(
                    scopes.clone(),
                    BTreeMap::from([(bindings.clone(), BTreeMap::from([(template, result)]))]),
                )]),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::ExecutionControl;

    fn node_bytes<T>() -> usize {
        std::mem::size_of::<T>() + 3 * std::mem::size_of::<usize>()
    }

    #[test]
    fn prepared_application_lookup_separates_witness_applications_and_full_bindings() {
        let origin = ValueOrigin::fresh().unwrap();
        let first = FunctionIndex(3, origin);
        let second = FunctionIndex(4, origin);
        let template = TypeObjectDigest::from_bytes([101; 32]);
        let a = TypeObjectDigest::from_bytes([103; 32]);
        let b = TypeObjectDigest::from_bytes([107; 32]);
        let parameter = TypeParameterId::migrate(b"application-lookup", 0);
        let bindings = BTreeMap::from([(parameter, a)]);
        let other_bindings = BTreeMap::from([(parameter, b)]);
        let control = ExecutionControl::uncancelled();
        let mut work = Budget::new(&control);
        work.remember_application_substitution(first, template, a, &bindings)
            .unwrap();
        work.remember_application_substitution(second, template, b, &bindings)
            .unwrap();
        work.remember_application_substitution(first, template, b, &other_bindings)
            .unwrap();
        // Equal outputs remain separate application proofs, not witness identities.
        work.remember_application_substitution(second, a, a, &bindings)
            .unwrap();
        let lookup = work.finish_type_lookup(origin);
        assert_eq!(
            lookup.get_application(origin, first, template, &bindings),
            Ok(Some(a))
        );
        assert_eq!(
            lookup.get_application(origin, second, template, &bindings),
            Ok(Some(b))
        );
        assert_eq!(
            lookup.get_application(origin, first, template, &other_bindings),
            Ok(Some(b))
        );
        assert_eq!(
            lookup.get_application(origin, second, template, &other_bindings),
            Ok(None)
        );
        assert_eq!(
            lookup.get_application(origin, FunctionIndex(5, origin), template, &bindings),
            Ok(None)
        );
        assert_eq!(lookup.get(origin, template, &bindings), Ok(None));
        let foreign = ValueOrigin::fresh().unwrap();
        assert!(
            lookup
                .get_application(foreign, first, template, &bindings)
                .is_err()
        );
        assert!(
            lookup
                .get_application(origin, FunctionIndex(first.0, foreign), template, &bindings)
                .is_err()
        );
        assert!(work.type_lookup.applications.is_empty());
    }

    #[test]
    fn prepared_application_lookup_reserves_before_growth_and_rechecks_hits() {
        let origin = ValueOrigin::fresh().unwrap();
        let function = FunctionIndex(7, origin);
        let parameter = TypeParameterId::migrate(b"application-lookup-capacity", 0);
        let template = TypeObjectDigest::from_bytes([109; 32]);
        let result = TypeObjectDigest::from_bytes([113; 32]);
        let bindings = BTreeMap::from([(parameter, result)]);
        let required = node_bytes::<(FunctionIndex, Contexts)>()
            + node_bytes::<(Bindings, ResolvedTypes)>()
            + node_bytes::<(TypeParameterId, TypeObjectDigest)>()
            + node_bytes::<(TypeObjectDigest, TypeObjectDigest)>();
        let control = ExecutionControl::uncancelled();
        let cancelled = ExecutionControl::cancel_after_checks(0);
        for short in [false, true] {
            let mut work = Budget::new(&control);
            work.bytes = MAXIMUM_METADATA_BYTES - required + usize::from(short);
            let outcome =
                work.remember_application_substitution(function, template, result, &bindings);
            if short {
                assert_eq!(outcome.unwrap_err().class, DiagnosticClass::Resource);
                assert!(work.type_lookup.applications.is_empty());
            } else {
                outcome.unwrap();
                assert_eq!(work.bytes, MAXIMUM_METADATA_BYTES);
                work.remember_application_substitution(function, template, result, &bindings)
                    .unwrap();
                assert_eq!(work.bytes, MAXIMUM_METADATA_BYTES);
                assert!(
                    work.remember_application_substitution(function, template, template, &bindings)
                        .is_err()
                );
                work.control = &cancelled;
                assert_eq!(
                    work.remember_application_substitution(function, template, result, &bindings)
                        .unwrap_err()
                        .class,
                    DiagnosticClass::Cancelled
                );
            }
        }
        let mut refused = false;
        let mut succeeded = false;
        for checks in 0..8 {
            let control = ExecutionControl::cancel_after_checks(checks);
            let mut work = Budget::new(&control);
            match work.remember_application_substitution(function, template, result, &bindings) {
                Ok(()) => succeeded = true,
                Err(error) => {
                    refused = true;
                    assert_eq!(error.class, DiagnosticClass::Cancelled);
                    assert!(work.type_lookup.applications.is_empty());
                }
            }
        }
        assert!(refused && succeeded);
    }

    #[test]
    fn prepared_type_lookup_reserves_full_context_and_nodes_before_growth() {
        let context = BTreeMap::from([
            (
                TypeParameterId::migrate(b"lookup-reservation", 0),
                TypeObjectDigest::from_bytes([41; 32]),
            ),
            (
                TypeParameterId::migrate(b"lookup-reservation", 1),
                TypeObjectDigest::from_bytes([43; 32]),
            ),
        ]);
        let template = TypeObjectDigest::from_bytes([47; 32]);
        let result = TypeObjectDigest::from_bytes([53; 32]);
        let required = node_bytes::<(Bindings, ResolvedTypes)>()
            + context.len() * node_bytes::<(TypeParameterId, TypeObjectDigest)>()
            + node_bytes::<(TypeObjectDigest, TypeObjectDigest)>();
        let control = ExecutionControl::uncancelled();
        for short in [false, true] {
            let mut work = Budget::new(&control);
            work.bytes = MAXIMUM_METADATA_BYTES - required + usize::from(short);
            let before = work.bytes;
            let outcome = work.remember_type_substitution(template, result, &context);
            if short {
                assert_eq!(
                    outcome.unwrap_err().code,
                    "normalized_instantiation_storage"
                );
                assert!(work.type_lookup.contexts.is_empty());
            } else {
                outcome.unwrap();
                assert_eq!(work.bytes - before, required);
                assert_eq!(work.bytes, MAXIMUM_METADATA_BYTES);
                assert_eq!(work.type_lookup.contexts[&context][&template], result);
                // Duplicate proofs retain no additional key or node storage.
                work.remember_type_substitution(template, result, &context)
                    .unwrap();
                assert_eq!(work.bytes, MAXIMUM_METADATA_BYTES);
                assert!(
                    work.remember_type_substitution(template, template, &context)
                        .is_err()
                );
            }
        }
        let mut work = Budget::new(&control);
        work.remember_type_substitution(template, result, &context)
            .unwrap();
        let before = work.bytes;
        let other = TypeObjectDigest::from_bytes([59; 32]);
        work.remember_type_substitution(other, result, &context)
            .unwrap();
        assert_eq!(
            work.bytes - before,
            node_bytes::<(TypeObjectDigest, TypeObjectDigest)>()
        );
        assert_eq!(work.type_lookup.contexts.len(), 1);
        assert_eq!(work.type_lookup.contexts[&context].len(), 2);
    }

    #[test]
    fn prepared_type_lookup_cancellation_never_publishes_partial_context() {
        let context = BTreeMap::from([
            (
                TypeParameterId::migrate(b"lookup-cancellation", 0),
                TypeObjectDigest::from_bytes([61; 32]),
            ),
            (
                TypeParameterId::migrate(b"lookup-cancellation", 1),
                TypeObjectDigest::from_bytes([67; 32]),
            ),
        ]);
        let mut failed = false;
        let mut completed = false;
        for checks in 0..12 {
            let control = ExecutionControl::cancel_after_checks(checks);
            let mut work = Budget::new(&control);
            match work.remember_type_substitution(
                TypeObjectDigest::from_bytes([71; 32]),
                TypeObjectDigest::from_bytes([73; 32]),
                &context,
            ) {
                Ok(()) => {
                    completed = true;
                    assert_eq!(work.type_lookup.contexts.len(), 1);
                }
                Err(error) => {
                    failed = true;
                    assert_eq!(error.class, DiagnosticClass::Cancelled);
                    assert!(work.type_lookup.contexts.is_empty());
                }
            }
        }
        assert!(failed && completed);
    }

    #[test]
    fn prepared_type_lookup_records_only_changed_composite_roots() {
        let parameter = TypeParameterId::migrate(b"lookup-root-selection", 0);
        let mut types = BTreeMap::new();
        let mut add = |form| {
            let object = TypeObject::new(form).unwrap();
            let ty = encode_type_object(&object).unwrap().0;
            types.insert(ty, object);
            ty
        };
        let symbolic = add(TypeForm::TypeParameter { parameter });
        let integer = add(TypeForm::I64);
        let sequence = add(TypeForm::OwnedSequence { item: symbolic });
        let bindings = BTreeMap::from([(parameter, integer)]);
        let control = ExecutionControl::uncancelled();
        let mut work = Budget::new(&control);
        assert_eq!(
            substitute(&mut types, integer, &bindings, 0, &mut work).unwrap(),
            integer
        );
        assert_eq!(
            substitute(&mut types, symbolic, &bindings, 0, &mut work).unwrap(),
            integer
        );
        assert!(work.type_lookup.contexts.is_empty());
        let resolved = substitute(&mut types, sequence, &bindings, 0, &mut work).unwrap();
        assert_ne!(resolved, sequence);
        assert_eq!(work.type_lookup.contexts[&bindings].len(), 1);
        assert_eq!(work.type_lookup.contexts[&bindings][&sequence], resolved);
        let origin = ValueOrigin::fresh().unwrap();
        let lookup = work.finish_type_lookup(origin);
        assert!(work.type_lookup.contexts.is_empty());
        assert_eq!(
            lookup.get(origin, sequence, &bindings).unwrap(),
            Some(resolved)
        );
        assert!(
            lookup
                .get(ValueOrigin::fresh().unwrap(), sequence, &bindings)
                .is_err()
        );
    }
}
