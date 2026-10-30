//! Synthetic exact types for adversarial projection tests, not native authoring.
use super::super::super::super::owned_i64_cell::OwnedI64Cell;
use super::super::super::super::owned_product::OwnedProduct;
use super::super::*;
use crate::platform::compiler::{
    LoadedArtifact, OptimizationPolicy, build_clean, link_artifact, load_artifact,
};
use crate::platform::kernel::{StructuralTypeField, TypeObject, encode_type_object};
use crate::platform::publication::GraphRepository;
use std::sync::OnceLock;

const SOURCE: &str = r#"declarations.begin
(units (module create checked-value-fixture
  (external create add (visibility public) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (function create unit (visibility public) (effect pure) (returns Unit) (body (unit)))
  (function create flag (visibility public) (effect pure)
    (parameter create value (type Bool)) (returns Bool) (body (local value)))))
declarations.end"#;

fn source_key(source: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.checked-value-test-fixture");
    hasher.update(&crate::platform::canonical_schema::SCHEMA_EPOCH.to_le_bytes());
    hasher.update(&crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH);
    for part in ["owned-byte-buffer", "DeterministicBaseline", source] {
        hasher.update(&(part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    *hasher.finalize().as_bytes()
}

fn base_artifact() -> &'static LoadedArtifact {
    static BASE: OnceLock<([u8; 32], LoadedArtifact)> = OnceLock::new();
    let key = source_key(SOURCE);
    let (bound, artifact) = BASE.get_or_init(|| {
        // These adversarial projection tests need a preparation origin and basic
        // scalar/callable types, rather than the full maintained standard closure.
        // Retain real authoring, compilation and strict loading once per binary.
        let source =
            super::super::super::super::tests::byte_buffer_tests::author_only(SOURCE).unwrap();
        crate::platform::kernel::validate_full(&source).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(&temporary.path().join("checked"), &source, None)
            .unwrap()
            .repository;
        let compilation =
            build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
        let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
        let artifact = load_artifact(&linked.artifact.bytes).unwrap();
        artifact.require_current_admission(|| Ok(())).unwrap();
        assert_eq!(artifact.manifest.packages.len(), 1);
        temporary.close().unwrap();
        (key, artifact)
    });
    assert_eq!(*bound, key);
    artifact
}

pub(in super::super) struct Fixture {
    pub program: NormalizedProgram,
    pub domain: ValueOrigin,
    pub product: TypeObjectDigest,
    pub payload: TypeObjectDigest,
    pub depth: usize,
}
fn add(program: &mut NormalizedProgram, form: TypeForm, ordinary: bool) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    program.types.insert(ty, object);
    if ordinary {
        program.ordinary_types.insert(ty);
        program.application_free_types.insert(ty);
    }
    ty
}
impl Fixture {
    pub fn new(depth: usize) -> Self {
        // Every fixture still owns a fresh preparation, origin and mutable type
        // inventory. Sharing a prepared program here would weaken foreign-origin
        // and table-mutation controls, unlike caching the immutable admitted input.
        let mut program = NormalizedProgram::prepare(base_artifact().clone()).unwrap();
        let integer = add(&mut program, TypeForm::I64, true);
        let mut payload = add(&mut program, TypeForm::List { item: integer }, true);
        for _ in 0..depth {
            payload = add(&mut program, TypeForm::Option { item: payload }, true);
        }
        let cell = add(&mut program, TypeForm::OwnedI64Cell, false);
        let product = add(
            &mut program,
            TypeForm::OwnedProduct {
                fields: vec![
                    StructuralTypeField {
                        name: Name::new("payload").unwrap(),
                        ty: payload,
                    },
                    StructuralTypeField {
                        name: Name::new("rest").unwrap(),
                        ty: cell,
                    },
                ],
            },
            false,
        );
        Self {
            program,
            domain: ValueOrigin::fresh().unwrap(),
            product,
            payload,
            depth,
        }
    }
    pub fn raw(&self, n: usize) -> NormalizedValue {
        let mut raw =
            NormalizedValue::list((0..n).map(|n| NormalizedValue::I64(n as i64)).collect())
                .unwrap();
        for _ in 0..self.depth {
            raw = NormalizedValue::Option(Some(Box::new(raw)));
        }
        raw
    }
    pub fn admit(&self, raw: NormalizedValue) -> Result<Value, ExecutionError> {
        let mut work = ValueWork::default();
        let mut allocated = 0;
        let mut charges = 0;
        let mut items = 0;
        let mut allocated_saturated = false;
        let mut items_saturated = false;
        let resources = NormalizedResourceScope::new().unwrap();
        let bindings = BTreeMap::new();
        let control = ExecutionControl::uncancelled();
        Admission {
            shared_budget: None,
            program: &self.program,
            substitutions: &bindings,
            resources: &resources,
            control: &control,
            policy: super::super::super::NormalizedRunPolicy::foreground(),
            work: &mut work,
            allocated: &mut allocated,
            allocated_saturated: &mut allocated_saturated,
            allocation_charges: &mut charges,
            items: &mut items,
            items_saturated: &mut items_saturated,
            admission_bytes: 0,
            admission_bytes_saturated: false,
        }
        .value(raw, self.payload, None, true)
    }
    pub fn storage(&self, raw: NormalizedValue) -> OwnedProduct {
        OwnedProduct::create(
            self.domain,
            self.product,
            vec![
                raw,
                NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(self.domain, 37)),
            ],
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .unwrap()
    }
    pub fn owner(&self, n: usize) -> Value {
        let checked = self.admit(self.raw(n)).unwrap();
        let token = self.storage(checked.into_raw());
        token
            .establish_admission(self.program.value_origin)
            .unwrap();
        Value::memory(&self.program, NormalizedValue::OwnedProduct(token)).unwrap()
    }
    pub fn read(&self, owner: &Value) -> Value {
        owner.duplicate(ParameterUse::Borrow).unwrap()
    }
    pub fn project(&self, read: &Value) -> Result<Value, ExecutionError> {
        read.owned_metadata(
            &self.program,
            self.domain,
            &Name::new("payload").unwrap(),
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
    }
}
pub(in super::super) fn token(value: &Value) -> &OwnedProduct {
    let NormalizedValue::OwnedProduct(token) = value.raw() else {
        panic!("product")
    };
    token
}

#[test]
fn cached_projection_input_keeps_preparations_origins_and_mutations_independent() {
    let mut first = Fixture::new(8);
    let second = Fixture::new(8);
    assert_ne!(first.program.value_origin, second.program.value_origin);
    assert_ne!(first.domain, second.domain);
    assert_eq!(first.payload, second.payload);
    assert_eq!(
        first.program.artifact().bundle_digest,
        second.program.artifact().bundle_digest
    );
    let value = first.admit(first.raw(3)).unwrap();
    assert!(
        value
            .has_exact_ordinary_type(&first.program, first.payload, &mut ValueWork::default())
            .unwrap()
    );
    assert!(
        value
            .has_exact_ordinary_type(&second.program, second.payload, &mut ValueWork::default())
            .is_err()
    );
    first.program.types.remove(&first.payload).unwrap();
    first.program.ordinary_types.remove(&first.payload);
    assert!(second.program.types.contains_key(&second.payload));
    assert!(second.program.ordinary_types.contains(&second.payload));
    let third = Fixture::new(8);
    assert!(third.program.types.contains_key(&third.payload));
    assert!(third.program.ordinary_types.contains(&third.payload));
    assert_ne!(third.program.value_origin, second.program.value_origin);
    assert_ne!(source_key(SOURCE), source_key(&format!("{SOURCE}\n")));
    base_artifact()
        .require_current_admission(|| Ok(()))
        .unwrap();
}
