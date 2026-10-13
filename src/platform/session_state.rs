//! Exact closed session-state instances. Binding maps never serve as recursive type identities.
use super::*;
use crate::platform::kernel::{StructuralTypeField, encode_type_object};
use crate::platform::semantic_id::TypeParameterId;

type Bindings = BTreeMap<TypeParameterId, TypeObjectDigest>;

struct Closure<'a, R> {
    read: &'a R,
    derived: BTreeMap<TypeObjectDigest, TypeObject>,
    work: usize,
    bytes: usize,
}

impl<R: SessionShapeRead> Closure<'_, R> {
    fn step(&mut self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()?;
        self.work = self
            .work
            .checked_add(1)
            .filter(|work| *work <= MAXIMUM_SESSION_STATE_NODES)
            .ok_or_else(|| {
                session_resource(
                    "session_state_limit",
                    "retained state type exceeds its bounded closure work",
                )
            })?;
        Ok(())
    }

    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .and_then(|bytes| self.bytes.checked_add(bytes))
            .filter(|bytes| *bytes <= MAXIMUM_SESSION_STATE_BYTES)
            .ok_or_else(|| {
                session_resource(
                    "session_state_limit",
                    "retained state type exceeds its bounded closure storage",
                )
            })?;
        Ok(())
    }

    fn object(&mut self, ty: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        self.step()?;
        if let Some(object) = self.derived.get(&ty) {
            return Ok(object.clone());
        }
        self.read.type_object(ty)
    }

    fn substitute(
        &mut self,
        ty: TypeObjectDigest,
        bindings: &Bindings,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.step()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(session_resource(
                "session_state_depth",
                "retained state argument exceeds structural type depth",
            ));
        }
        let object = self.object(ty)?;
        let next = depth + 1;
        let form = match object.form {
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => {
                return Err(session_semantic(
                    "session_state_live_type",
                    "retained session state cannot contain a read view",
                ));
            }
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. } => {
                return Err(
                    crate::platform::kernel::owned_associated_wire::unsupported_semantics(),
                );
            }
            TypeForm::TypeParameter { parameter } => {
                return bindings.get(&parameter).copied().ok_or_else(|| {
                    session_semantic(
                        "session_state_parameter",
                        "state type parameter is unresolved",
                    )
                });
            }
            TypeForm::Applied {
                declaration,
                arguments,
            } => {
                self.reserve::<TypeObjectDigest>(arguments.len())?;
                let arguments = arguments
                    .into_iter()
                    .map(|ty| self.substitute(ty, bindings, next))
                    .collect::<Result<_, _>>()?;
                TypeForm::Applied {
                    declaration,
                    arguments,
                }
            }
            TypeForm::StructuralRecord { fields } => {
                self.reserve::<StructuralTypeField>(fields.len())?;
                let fields = fields
                    .into_iter()
                    .map(|field| {
                        self.reserve::<u8>(field.name.as_str().len())?;
                        Ok(StructuralTypeField {
                            name: field.name,
                            ty: self.substitute(field.ty, bindings, next)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?;
                TypeForm::StructuralRecord { fields }
            }
            TypeForm::List { item } => TypeForm::List {
                item: self.substitute(item, bindings, next)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: self.substitute(item, bindings, next)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: self.substitute(item, bindings, next)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: self.substitute(key, bindings, next)?,
                value: self.substitute(value, bindings, next)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: self.substitute(ok, bindings, next)?,
                error: self.substitute(error, bindings, next)?,
            },
            TypeForm::Function { parameters, result } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?;
                TypeForm::Function {
                    parameters: parameters
                        .into_iter()
                        .map(|ty| self.substitute(ty, bindings, next))
                        .collect::<Result<_, _>>()?,
                    result: self.substitute(result, bindings, next)?,
                }
            }
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => {
                self.reserve::<TypeObjectDigest>(parameters.len())?;
                TypeForm::TaskFunction {
                    parameters: parameters
                        .into_iter()
                        .map(|ty| self.substitute(ty, bindings, next))
                        .collect::<Result<_, _>>()?,
                    result: self.substitute(result, bindings, next)?,
                    effect,
                }
            }
            form => form,
        };
        let resolved = TypeObject::new(form)?;
        let (digest, bytes) = encode_type_object(&resolved)?;
        self.reserve::<u8>(bytes.len())?;
        if digest != ty && !self.derived.contains_key(&digest) {
            self.reserve::<(TypeObjectDigest, TypeObject)>(1)?;
            self.derived.insert(digest, resolved);
        }
        Ok(digest)
    }

    fn enqueue(
        &mut self,
        pending: &mut Vec<TypeObjectDigest>,
        ty: TypeObjectDigest,
    ) -> Result<(), Diagnostic> {
        self.step()?;
        self.reserve::<TypeObjectDigest>(1)?;
        pending.push(ty);
        Ok(())
    }
}

pub(super) fn validate<R: SessionShapeRead>(
    read: &R,
    root: TypeObjectDigest,
) -> Result<(), Diagnostic> {
    let mut closure = Closure {
        read,
        derived: BTreeMap::new(),
        work: 0,
        bytes: 0,
    };
    let root = closure.substitute(root, &Bindings::new(), 0)?;
    let mut pending = Vec::new();
    closure.enqueue(&mut pending, root)?;
    let mut complete = BTreeSet::new();
    while let Some(ty) = pending.pop() {
        closure.step()?;
        if complete.contains(&ty) {
            continue;
        }
        closure.reserve::<TypeObjectDigest>(1)?;
        complete.insert(ty);
        let object = closure.object(ty)?;
        match &object.form {
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. } => {
                return Err(
                    crate::platform::kernel::owned_associated_wire::unsupported_semantics(),
                );
            }
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::Bytes
            | TypeForm::Text => {}
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                let arguments = match &object.form {
                    TypeForm::Applied { arguments, .. } => &arguments[..],
                    _ => &[],
                };
                let parameters = read.nominal_parameters(*declaration)?;
                if arguments.len() != parameters.len() {
                    return Err(session_semantic(
                        "session_state_arity",
                        "state nominal application has wrong arity",
                    ));
                }
                closure.reserve::<(TypeParameterId, TypeObjectDigest)>(parameters.len())?;
                let mut bindings = Bindings::new();
                for (parameter, argument) in parameters.into_iter().zip(arguments) {
                    closure.enqueue(&mut pending, *argument)?;
                    bindings.insert(parameter, *argument);
                }
                let members = match read.nominal_shape(*declaration)? {
                    SessionNominalShape::Record(fields) => fields.into_values().collect::<Vec<_>>(),
                    SessionNominalShape::Variant(cases) => cases.into_values().flatten().collect(),
                };
                closure.reserve::<TypeObjectDigest>(members.len())?;
                for member in members {
                    let member = closure.substitute(member, &bindings, 0)?;
                    closure.enqueue(&mut pending, member)?;
                }
            }
            TypeForm::Map { key, value } => {
                if !matches!(
                    closure.object(*key)?.form,
                    TypeForm::Bool | TypeForm::I64 | TypeForm::Bytes | TypeForm::Text
                ) {
                    return Err(session_semantic(
                        "session_state_map_key",
                        "retained session state map keys must be deterministic primitive values",
                    ));
                }
                closure.enqueue(&mut pending, *value)?;
            }
            TypeForm::StructuralRecord { .. } | TypeForm::List { .. } | TypeForm::Option { .. } => {
                for child in object.child_types() {
                    closure.enqueue(&mut pending, child)?;
                }
            }
            TypeForm::TypeParameter { .. } => {
                return Err(session_semantic(
                    "session_state_parameter",
                    "state type parameter is unresolved",
                ));
            }
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. }
            | TypeForm::StaticText
            | TypeForm::Secret
            | TypeForm::Result { .. }
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. }
            | TypeForm::CapabilityResource { .. }
            | TypeForm::Stream { .. }
            | TypeForm::Function { .. }
            | TypeForm::TaskFunction { .. } => {
                return Err(session_semantic(
                    "session_state_live_type",
                    "retained session state contains a live, callable, static, secret, or unresolved type",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::TypeObjectInterner;
    use crate::platform::kernel::{
        PackageId, ReadScopeArgument, ReadScopeBinder, ReadScopeReference,
    };
    use crate::platform::semantic_id::{
        AssociatedTypeMemberId, DeclarationId, ImplementationParameterId, ReadScopeParameterId,
    };

    struct StateTypes(TypeObjectInterner);

    impl SessionShapeRead for StateTypes {
        fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
            self.0
                .get(digest)
                .cloned()
                .ok_or_else(|| session_semantic("test_state_type", "state test type is absent"))
        }

        fn nominal_parameters(
            &self,
            _: DeclarationReference,
        ) -> Result<Vec<TypeParameterId>, Diagnostic> {
            unreachable!("structural state fixture has no nominal type")
        }

        fn nominal_shape(
            &self,
            _: DeclarationReference,
        ) -> Result<SessionNominalShape, Diagnostic> {
            unreachable!("structural state fixture has no nominal type")
        }
    }

    #[test]
    fn owned_sequence_state_rejects_direct_and_nested_custody() {
        let mut types = TypeObjectInterner::default();
        let cell = types.intern(TypeForm::OwnedI64Cell).unwrap();
        let sequence = types
            .intern(TypeForm::OwnedSequence { item: cell })
            .unwrap();
        let list = types.intern(TypeForm::List { item: sequence }).unwrap();
        let record = types
            .intern(TypeForm::StructuralRecord {
                fields: vec![StructuralTypeField {
                    name: Name::new("pending").unwrap(),
                    ty: list,
                }],
            })
            .unwrap();
        let read = StateTypes(types);
        for root in [sequence, list, record] {
            assert_eq!(
                validate(&read, root).unwrap_err().code,
                "session_state_live_type"
            );
        }
    }

    struct ViewStateTypes {
        types: TypeObjectInterner,
        unit: TypeObjectDigest,
        view: TypeObjectDigest,
        nested: TypeObjectDigest,
        parameter: TypeParameterId,
    }

    fn declaration(ordinal: u64) -> DeclarationReference {
        DeclarationReference {
            package: PackageId::migrate(b"session-view-boundary", 0),
            declaration: DeclarationId::migrate(b"session-view-boundary", ordinal),
        }
    }

    impl SessionShapeRead for ViewStateTypes {
        fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
            self.types
                .get(digest)
                .cloned()
                .ok_or_else(|| session_semantic("test_state_type", "state test type is absent"))
        }
        fn nominal_parameters(
            &self,
            reference: DeclarationReference,
        ) -> Result<Vec<TypeParameterId>, Diagnostic> {
            Ok(if reference == declaration(1) {
                vec![self.parameter]
            } else {
                vec![]
            })
        }
        fn nominal_shape(
            &self,
            reference: DeclarationReference,
        ) -> Result<SessionNominalShape, Diagnostic> {
            Ok(if reference == declaration(1) {
                SessionNominalShape::Record(BTreeMap::from([(
                    Name::new("tag").unwrap(),
                    self.unit,
                )]))
            } else if reference == declaration(2) {
                SessionNominalShape::Variant(BTreeMap::from([
                    (Name::new("safe").unwrap(), None),
                    (Name::new("unused").unwrap(), Some(self.view)),
                ]))
            } else if reference == declaration(3) {
                SessionNominalShape::Record(BTreeMap::from([(
                    Name::new("hidden").unwrap(),
                    self.nested,
                )]))
            } else {
                panic!("unselected fixture declaration")
            })
        }
    }

    #[test]
    fn retained_state_views_reject_direct_nested_phantom_and_inactive_closures() {
        let mut types = TypeObjectInterner::default();
        let unit = types.intern(TypeForm::Unit).unwrap();
        let scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration(0)),
            parameter: ReadScopeParameterId::migrate(b"session-view-boundary", 0),
        });
        let member = AssociatedTypeMemberId::migrate(b"session-view-boundary", 0);
        let view = types
            .intern(TypeForm::ReadViewApplied {
                declaration: declaration(0),
                arguments: vec![],
                read_scope: scope,
            })
            .unwrap();
        let associated = types
            .intern(TypeForm::AssociatedReadViewMember {
                contract: declaration(0),
                member,
                read_scope: scope,
            })
            .unwrap();
        let projected = types
            .intern(TypeForm::AssociatedReadViewProjection {
                scope: declaration(0),
                parameter: ImplementationParameterId::migrate(b"session-view-boundary", 0),
                contract: declaration(0),
                member,
                read_scope: scope,
            })
            .unwrap();
        let optional = types.intern(TypeForm::Option { item: view }).unwrap();
        let nested = types.intern(TypeForm::List { item: optional }).unwrap();
        let phantom = types
            .intern(TypeForm::Applied {
                declaration: declaration(1),
                arguments: vec![view],
            })
            .unwrap();
        let healthy = types
            .intern(TypeForm::Applied {
                declaration: declaration(1),
                arguments: vec![unit],
            })
            .unwrap();
        let inactive = types
            .intern(TypeForm::Named {
                declaration: declaration(2),
            })
            .unwrap();
        let hidden = types
            .intern(TypeForm::Named {
                declaration: declaration(3),
            })
            .unwrap();
        let read = ViewStateTypes {
            types,
            unit,
            view,
            nested,
            parameter: TypeParameterId::migrate(b"session-view-boundary", 0),
        };
        for root in [
            view, associated, projected, optional, nested, phantom, inactive, hidden,
        ] {
            let error = validate(&read, root).unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Semantic);
            assert_eq!(error.code, "session_state_live_type");
        }
        validate(&read, healthy).unwrap();
    }
}
