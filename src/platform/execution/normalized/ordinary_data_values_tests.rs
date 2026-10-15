//! Literal ordinary payloads and JSON values, with schema-independent type selectors.
use super::*;

fn selected_type(specification: &serde_json::Value) -> TypeObjectDigest {
    let form = match specification["kind"].as_str().unwrap() {
        "unit" | "bool" | "i64" | "bytes" | "text" | "named" => {
            serde_json::from_value(specification.clone()).unwrap()
        }
        "list" => TypeForm::List {
            item: selected_type(&specification["item"]),
        },
        "structural_record" => TypeForm::StructuralRecord {
            fields: specification["fields"]
                .as_array()
                .unwrap()
                .iter()
                .map(|field| StructuralTypeField {
                    name: serde_json::from_value(field["name"].clone()).unwrap(),
                    ty: selected_type(&field["ty"]),
                })
                .collect(),
        },
        kind => panic!("unsupported ordinary fixture selector {kind}"),
    };
    encode_type_object(&TypeObject::new(form).unwrap())
        .unwrap()
        .0
}

fn literal_bytes(hex: &str) -> Vec<u8> {
    let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty());
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn maintained_nominal_nested_data_preserves_all_literal_payloads_in_both_codecs() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/ordinary-data-values.json"
    ))
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let projects = fixture["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    let mut heads = Vec::new();
    let mut snapshots = Vec::new();
    for project in projects {
        let path = root.join(project["project"].as_str().unwrap());
        heads.push(std::fs::read(path.join("HEAD")).unwrap());
        snapshots.push(
            GraphRepository::open(&path)
                .unwrap()
                .view_current()
                .unwrap()
                .reconstruct_full_oracle()
                .unwrap()
                .value,
        );
    }
    let schema =
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct(snapshots.iter())
            .unwrap();
    let mut nominal = 0;
    let mut nested = 0;
    let mut samples = 0;
    for ((project, snapshot), expected_count) in projects.iter().zip(&snapshots).zip([32, 57]) {
        assert_eq!(snapshot.root.package_name.as_str(), project["package_name"]);
        // Current canonical type bytes must roundtrip exactly; no saved type
        // hashes or obsolete format generations participate in the oracle.
        for (ty, object) in &snapshot.types {
            let (digest, bytes) = encode_type_object(object).unwrap();
            assert_eq!(digest, *ty);
            assert_eq!(
                crate::platform::kernel::decode_type_object(&bytes, digest).unwrap(),
                *object,
            );
        }
        let program = NormalizedProgram::prepare(
            load_artifact(
                &std::fs::read(root.join(project["artifact"].as_str().unwrap())).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let cases = project["samples"].as_array().unwrap();
        assert_eq!(cases.len(), expected_count);
        for (index, case) in cases.iter().enumerate() {
            let ty = selected_type(&case["type"]);
            // Samples also cover supplier body layouts that need not appear in
            // this project's imported interface. Require their exact meaning in
            // both the independently admitted canonical union and this artifact.
            let canonical_type = schema.types.get(&ty).unwrap_or_else(|| {
                panic!(
                    "canonical type missing for {} sample {index}: {} ({ty})",
                    project["project"], case["type"],
                )
            });
            assert_eq!(
                program.types.get(&ty),
                Some(canonical_type),
                "artifact type differs for {} sample {index}: {} ({ty})",
                project["project"],
                case["type"],
            );
            let json = serde_json::to_vec(&case["value"]).unwrap();
            let production = decode_typed(&program, &json, ty, JsonLimits::default()).unwrap();
            let reference = decode_typed(&schema, &json, ty, JsonLimits::default()).unwrap();
            let encoded =
                super::super::data_codec::encode_typed(&program, &production, ty).unwrap();
            assert_eq!(
                super::super::data_codec_reference::encode_typed(&schema, &reference, ty).unwrap(),
                encoded,
            );
            assert!(encoded.len() >= 42 + 32);
            assert_eq!(
                &encoded[42..encoded.len() - 32],
                literal_bytes(case["payload"].as_str().unwrap()),
                "literal payload changed for {}: {}",
                project["package_name"],
                case["type"],
            );
            let value = super::super::data_codec::decode_typed(&program, &encoded, ty).unwrap();
            assert_eq!(
                super::super::codec::encode_value(&program, &value, ty, JsonLimits::default())
                    .unwrap(),
                case["value"],
            );
            assert_eq!(
                super::super::data_codec::encode_typed(&program, &value, ty).unwrap(),
                encoded,
            );
            let value =
                super::super::data_codec_reference::decode_typed(&schema, &encoded, ty).unwrap();
            assert_eq!(
                super::super::codec::encode_value(&schema, &value, ty, JsonLimits::default())
                    .unwrap(),
                case["value"],
            );
            assert_eq!(
                super::super::data_codec_reference::encode_typed(&schema, &value, ty).unwrap(),
                encoded,
            );
            nominal += usize::from(matches!(program.types[&ty].form, TypeForm::Named { .. }));
            nested += usize::from(
                case["value"].as_object().is_some_and(|fields| {
                    fields
                        .values()
                        .any(|field| field.is_object() || field.is_array())
                }) || case["value"].is_array(),
            );
            samples += 1;
        }
    }
    assert_eq!(samples, 89);
    assert!(
        nominal >= 10 && nested >= 10,
        "nominal={nominal}, nested={nested}"
    );
    for (project, before) in projects.iter().zip(heads) {
        let path = root.join(project["project"].as_str().unwrap());
        assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), before);
    }
}
