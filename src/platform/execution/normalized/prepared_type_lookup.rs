//! Positive substitutions already proved during finite exact preparation.
//! Full binding contexts share one retained key; lookup never allocates or grows.
use super::super::value::ValueOrigin;
use super::*;

type Bindings = BTreeMap<TypeParameterId, TypeObjectDigest>;
type ResolvedTypes = BTreeMap<TypeObjectDigest, TypeObjectDigest>;
type Contexts = BTreeMap<Bindings, ResolvedTypes>;

#[derive(Default)]
pub(super) struct TypeLookupBuilder {
    contexts: Contexts,
}

#[derive(Clone, Debug)]
pub(in super::super) struct PreparedTypeLookup {
    origin: ValueOrigin,
    contexts: Contexts,
}

impl PreparedTypeLookup {
    pub(in super::super) fn empty(origin: ValueOrigin) -> Self {
        Self {
            origin,
            contexts: BTreeMap::new(),
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
        }
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
