//! Test-only transport of a frozen adversarial graph-14 artifact into current derived envelopes.
//! Canonical source and instructions are untouched. This does not admit or execute the fixture.
use super::*;
use crate::platform::persistent_map::PageStore;

pub(crate) fn current_derived_fixture(bytes: &[u8]) -> Vec<u8> {
    let (magic, domain) = match &bytes[..8] {
        b"LKJART18" => (*b"LKJAMF18", "lkjscript.artifact-manifest-envelope.v18"),
        b"LKJART19" => (*b"LKJAMF19", "lkjscript.artifact-manifest-envelope.v19"),
        b"LKJART20" => (*b"LKJAMF20", "lkjscript.artifact-manifest-envelope.v20"),
        _ => panic!("fixture generation"),
    };
    let number = |at| u64::from_be_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
    let end = 60 + number(12);
    let mut manifest: ArtifactManifest = crate::platform::packed::decode(
        &bytes[60..end],
        magic,
        domain,
        super::super::artifact::MAXIMUM_ARTIFACT_MANIFEST_BYTES,
    )
    .unwrap();
    let mut objects = BTreeMap::new();
    let mut at = end;
    for _ in 0..number(20) {
        let end = at + 40 + number(at);
        let segment = &bytes[at + 40..end];
        let pack = PackMetadata::decode(segment, true).unwrap();
        for entry in &pack.entries {
            let value = pack
                .read(segment, entry.key, entry.key.domain.maximum_bytes())
                .unwrap()
                .unwrap();
            assert!(objects.insert(entry.key, value).is_none());
        }
        at = end;
    }
    assert_eq!(at + 40, bytes.len());
    let mut pages = MemoryPageStore::default();
    for (key, value) in &objects {
        if key.domain == ObjectDomain::MapPage {
            pages
                .write_page(PageDigest::from_bytes(key.digest.bytes()), value)
                .unwrap();
        }
    }
    let mut replacements = BTreeMap::new();
    for (key, value) in objects.clone() {
        if key.domain != ObjectDomain::CompilerUnit {
            continue;
        }
        let mut unit: CompilationUnit = if value.starts_with(b"LKJCUN10") {
            crate::platform::packed::decode::<super::super::wire10::CompilationUnit10>(
                &value,
                *b"LKJCUN10",
                "lkjscript.compiler-unit-envelope.v10",
                super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
            )
            .unwrap()
            .into()
        } else {
            let (magic, domain) = match &value[..8] {
                b"LKJCUN11" => (*b"LKJCUN11", "lkjscript.compiler-unit-envelope.v11"),
                b"LKJCUN12" => (*b"LKJCUN12", "lkjscript.compiler-unit-envelope.v12"),
                b"LKJCUN13" => (*b"LKJCUN13", "lkjscript.compiler-unit-envelope.v13"),
                b"LKJCUN14" => (*b"LKJCUN14", "lkjscript.compiler-unit-envelope.v14"),
                _ => panic!("frozen unit generation"),
            };
            crate::platform::packed::decode::<super::predecessor_units::Unit>(
                &value,
                magic,
                domain,
                super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
            )
            .unwrap()
            .current()
        };
        unit.contract_version = 15;
        unit.graph_contract_version = 18;
        unit.bytecode_contract_version = 11;
        unit.key = CompilationUnitKey::derive(&unit.source, unit.optimization).unwrap();
        let (new, encoded) = unit.encode().unwrap();
        objects.remove(&key).unwrap();
        objects.insert(new, encoded);
        replacements.insert(key, (new, unit.key));
    }
    for package in &mut manifest.packages {
        let old = package.compilation.object_key();
        let mut compilation =
            CompilationManifest::decode(&objects.remove(&old).unwrap(), package.compilation)
                .unwrap();
        let map = PersistentMap::from_root(compilation.units);
        let mut entries = Vec::new();
        map.for_each(&pages, &mut MapWork::default(), |owner, value| {
            let owner_key = crate::platform::kernel::EncodedOwnerKey::decode(owner).unwrap();
            let mut binding = CompilationBinding::decode(value, owner_key).unwrap();
            let (object, key) = replacements[&binding.object.object_key()];
            binding.object = CompilerUnitObjectDigest::from_bytes(object.digest.bytes());
            binding.key = key;
            entries.push((owner.to_vec(), binding.encode(owner_key).unwrap()));
            Ok(())
        })
        .unwrap();
        let mut old_pages = MemoryPageStore::default();
        map.copy_reachable(&pages, &mut old_pages, &mut MapWork::default())
            .unwrap();
        for (digest, _) in old_pages.objects() {
            objects.remove(&ObjectKey::from_digest(
                ObjectDomain::MapPage,
                digest.bytes(),
            ));
        }
        compilation.units = replace_artifact_map(&mut objects, entries);
        compilation.graph_contract_version = 18;
        compilation.compiler_contract_version = 15;
        compilation.bytecode_contract_version = 11;
        let (digest, encoded) = compilation.encode().unwrap();
        package.compilation = digest;
        objects.insert(digest.object_key(), encoded);
    }
    manifest.contract_version = 22;
    manifest.graph_contract_version = 18;
    manifest.compiler_contract_version = 15;
    manifest.bytecode_contract_version = 11;
    let (closure, count, length) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = length;
    nominal_session_tests::hostile_bundle(&manifest, &objects)
}
