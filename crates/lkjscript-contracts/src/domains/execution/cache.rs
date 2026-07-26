use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, NATIVE_IMAGE_CACHE};

#[allow(clippy::too_many_arguments)]
pub(crate) fn native_image_cache(
    language: ContractDigest,
    source: ContractDigest,
    hir: ContractDigest,
    ssa: ContractDigest,
    bytecode: ContractDigest,
    categories: ContractDigest,
    profiles: ContractDigest,
    package: ContractDigest,
    lock: ContractDigest,
    module: ContractDigest,
    runtime: ContractDigest,
    native: ContractDigest,
) -> ContractDescriptor {
    let mut descriptor = ContractDescriptor {
        name: name(NATIVE_IMAGE_CACHE),
        items: Vec::new(),
        dependencies: Vec::new(),
    };
    for (dependency, digest) in [
        (super::super::LANGUAGE, language),
        (super::super::SOURCE, source),
        (super::super::TYPED_HIR, hir),
        (super::super::VERIFIED_SSA, ssa),
        (super::super::BYTECODE, bytecode),
        (super::super::RESOURCE_CATEGORIES, categories),
        (super::super::RESOURCE_PROFILES, profiles),
        (super::super::PACKAGE_MANIFEST, package),
        (super::super::PACKAGE_LOCK, lock),
        (super::super::MODULE_INTERFACE, module),
        (super::super::RUNTIME_CALLS, runtime),
        (super::super::NATIVE_LAYOUT, native),
    ] {
        descriptor = descriptor.dependency(name(dependency), digest.as_bytes());
    }
    descriptor
        .item(
            ContractItem::new("key", ContractItemKind::Type)
                .semantic_order()
                .fact(fact("framing", "framing", "u64 big-endian complete fields"))
                .fact(fact(
                    "source",
                    "source identity",
                    "entry source module package lock",
                ))
                .fact(fact(
                    "ssa",
                    "SSA identity",
                    "complete freshly verified Program",
                ))
                .fact(fact(
                    "profile",
                    "resource profile",
                    "complete effective identity",
                ))
                .fact(fact(
                    "backend",
                    "backend",
                    "provider limits tier root policy",
                ))
                .fact(fact("target", "target", "Linux x86-64 SysV exact facts")),
        )
        .item(
            ContractItem::new("artifact", ContractItemKind::Section)
                .semantic_order()
                .fact(fact("magic", "magic", "LKJNIC01"))
                .fact(fact("hash", "hash", "full SHA-256 key and file digests"))
                .fact(fact(
                    "image",
                    "image",
                    "canonical complete InstallableImage",
                ))
                .fact(fact(
                    "decode",
                    "decode",
                    "bounded canonical integrity checked",
                ))
                .fact(fact("install", "install", "fresh RW relocate RX mapping")),
        )
        .item(
            ContractItem::new("storage", ContractItemKind::Rule)
                .fact(fact(
                    "root",
                    "root",
                    "verified package target/lkjscript/native-cache",
                ))
                .fact(fact(
                    "bounds",
                    "bounds",
                    "16MiB 64 objects 256MiB 100000 records",
                ))
                .fact(fact(
                    "publication",
                    "publication",
                    "create sync validate rename dir-sync",
                ))
                .fact(fact(
                    "authority",
                    "authority",
                    "misses never grant execution authority",
                )),
        )
}

fn fact(id: &str, name_value: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name_value, value)
}
