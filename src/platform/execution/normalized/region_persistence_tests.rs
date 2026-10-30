//! Public authoring and exact first-party data authority for slab persistence.
use super::super::data::NormalizedDataAdapter;
use super::super::value::{ComponentIndex, NormalizedRecord};
use super::*;
use crate::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};

fn source(fixture: &str) -> crate::platform::kernel::KernelSnapshot {
    let standard = GraphRepository::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard"),
    )
    .unwrap()
    .view_current()
    .unwrap()
    .reconstruct_full_oracle()
    .unwrap()
    .value;
    let memory = crate::platform::MemProjectStore::new(None).unwrap();
    let repository = GraphRepository::create_memory(&memory, &standard, None)
        .unwrap()
        .repository;
    let member = |parent: &str, name: &str| {
        let parent = declaration_named(&standard, parent).declaration;
        standard
            .owners
            .iter()
            .find_map(|(key, owner)| match (key, owner) {
                (OwnerKey::Operation(id), OwnerRecord::Operation(record))
                    if record.declaration == parent && record.name.as_str() == name =>
                {
                    Some(format!("{}/{id}", standard.root.package_id))
                }
                (OwnerKey::Case(id), OwnerRecord::Case(record))
                    if record.declaration == parent && record.name.as_str() == name =>
                {
                    Some(format!("{}/{id}", standard.root.package_id))
                }
                _ => None,
            })
            .unwrap()
    };
    let mut input =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture))
            .unwrap();
    for (name, value) in [
        (
            "DATA_INTERFACE",
            declaration_named(&standard, "DataStore")
                .declaration
                .to_string(),
        ),
        ("DATA_GET", member("DataStore", "get")),
        ("DATA_PUT", member("DataStore", "put")),
        ("DATA_TRANSACTION", member("DataStore", "transaction")),
        (
            "KEY_PART",
            declaration_named(&standard, "DataKeyPart")
                .declaration
                .to_string(),
        ),
        ("KEY_TEXT", member("DataKeyPart", "Text")),
        ("EXPECT_MISSING", member("DataExpectation", "Missing")),
    ] {
        input = input.replace(name, &value);
    }
    let input = format!(
        "request base={}\n{input}",
        repository.view_current().unwrap().revision()
    );
    let decoded =
        crate::platform::control::decode_compact_change("region-persistence", input.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
    repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

fn capabilities(program: &NormalizedProgram, store: DataStore) -> NormalizedCapabilities {
    let component = ComponentIndex(
        program
            .components
            .iter()
            .position(|component| {
                component
                    .requirements
                    .iter()
                    .any(|index| program.requirements[index.0 as usize].name.as_str() == "slabs")
            })
            .unwrap() as u32,
    );
    let grants = program.components[component.0 as usize]
        .requirements
        .iter()
        .map(|index| {
            let requirement = &program.requirements[index.0 as usize];
            let adapter =
                NormalizedDataAdapter::admit(program, requirement).unwrap()(store.clone());
            NormalizedCapabilityGrant {
                requirement: requirement.reference,
                descriptor: NormalizedCapabilityGrantDescriptor::for_test(
                    requirement.interface,
                    NormalizedAdapterKind::Data,
                    adapter.operations().clone(),
                    exact_grant_limits(requirement, 100),
                ),
                adapter: Arc::new(adapter),
            }
        })
        .collect();
    NormalizedCapabilities::bind(program, component, grants).unwrap()
}

#[test]
fn region_images_round_trip_through_public_expressions_and_exact_data_grants() {
    round_trip("tests/fixtures/region-persistence.lkjc", 81);
}

#[test]
fn region_planner_cyclic_stage_slab_persists_without_node_reconstruction() {
    round_trip("examples/dependency-plan/region-store-witness.lkjc", 2);
}

fn round_trip(fixture: &str, expected: i64) {
    let source = source(fixture);
    let program = prepare_snapshot(&source);
    let mut images = Vec::new();
    for reference in [false, true] {
        let memory = crate::platform::MemProjectStore::new(None).unwrap();
        let store =
            DataStore::open_memory(&memory, "regions".into(), DataLimits::default()).unwrap();
        let capabilities = capabilities(&program, store.clone());
        let invoke = |name, arguments| {
            let declaration = declaration_named(&source, name);
            let control = ExecutionControl::uncancelled();
            if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(declaration, arguments, Some(&capabilities), &control)
                .map(|pair| pair.0)
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(declaration, arguments, Some(&capabilities), &control)
                    .map(|pair| pair.0)
            }
        };
        let NormalizedValue::Record(NormalizedRecord::Structural { fields }) =
            invoke("region-save", vec![]).unwrap()
        else {
            panic!("save result");
        };
        let [(_, digest), (_, root)] = fields.as_slice() else {
            panic!("saved fields");
        };
        assert_eq!(
            invoke("region-load", vec![digest.clone(), root.clone()]).unwrap(),
            NormalizedValue::I64(expected)
        );
        let before = store.current_revision().unwrap();
        assert_eq!(
            invoke("region-save", vec![]).unwrap_err().code,
            "normalized_region_condition"
        );
        assert_eq!(store.current_revision().unwrap(), before);
        assert_eq!(
            invoke(
                "region-load",
                vec![NormalizedValue::bytes([0_u8; 32]), root.clone()]
            )
            .unwrap_err()
            .code,
            "region_image_digest"
        );
        assert_eq!(store.current_revision().unwrap(), before);
        let key = DataKey::new(
            vec![DataKeyPart::Text("cycle".into())],
            &DataLimits::default(),
        )
        .unwrap();
        let transaction = store.begin().unwrap();
        let image = transaction.get_ref("graphs", &key).unwrap().unwrap();
        let actual = crate::platform::execution::region::persistence::digest(
            &image.value,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        assert_eq!(*digest, NormalizedValue::bytes(actual.bytes()));
        let aligned = crate::platform::execution::region::persistence::RegionImage::from_bytes(
            &image.value,
            actual,
            image.value.len(),
            &ExecutionControl::uncancelled(),
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(aligned.image().as_ptr() as usize % 64, 0);
        assert_eq!(aligned.image(), image.value);
        images.push(image.value.clone());
    }
    assert_eq!(
        images[0], images[1],
        "exact slab bytes are independent of evaluator and activation addresses"
    );
}
