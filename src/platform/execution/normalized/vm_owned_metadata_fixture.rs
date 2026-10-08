//! Synthetic exact types for adversarial projection tests, not native authoring.
use super::super::super::super::owned_i64_cell::OwnedI64Cell;
use super::super::super::super::owned_product::OwnedProduct;
use super::super::*;
use crate::platform::compiler::load_artifact;
use crate::platform::kernel::{StructuralTypeField, TypeObject, encode_type_object};

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
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("packages/standard/generated/standard.lkja");
        let mut program =
            NormalizedProgram::prepare(load_artifact(&std::fs::read(path).unwrap()).unwrap())
                .unwrap();
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
            allocation_charges: &mut charges,
            items: &mut items,
            admission_bytes: 0,
            admission_items: 0,
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
