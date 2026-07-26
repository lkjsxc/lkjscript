use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, BYTECODE, LANGUAGE, TYPED_HIR, VERIFIED_SSA};

pub(crate) fn typed_hir(language: ContractDigest) -> ContractDescriptor {
    descriptor(TYPED_HIR)
        .dependency(name(LANGUAGE), language.as_bytes())
        .item(
            ContractItem::new("public-artifact", ContractItemKind::Type)
                .fact(fact(
                    "identity",
                    "identity",
                    "package module source entity definition",
                ))
                .fact(fact("types", "types", "resolved exact semantic types"))
                .fact(fact(
                    "effects",
                    "effects",
                    "derived expression and function effects",
                ))
                .fact(fact(
                    "ownership",
                    "ownership",
                    "places moves borrows and cleanup obligations",
                ))
                .fact(fact(
                    "capabilities",
                    "capabilities",
                    "typed values with exact origins",
                ))
                .fact(fact("control", "control", "structured typed control facts")),
        )
}

pub(crate) fn verified_ssa(hir: ContractDigest) -> ContractDescriptor {
    descriptor(VERIFIED_SSA)
        .dependency(name(TYPED_HIR), hir.as_bytes())
        .item(
            ContractItem::new("program", ContractItemKind::Type)
                .fact(fact(
                    "identity",
                    "identity",
                    "package module and stable direct callees",
                ))
                .fact(fact(
                    "cfg",
                    "control flow",
                    "typed blocks parameters and terminators",
                ))
                .fact(fact(
                    "effects",
                    "effects",
                    "exact instruction and function summaries",
                ))
                .fact(fact(
                    "ownership",
                    "ownership",
                    "affine transfer and explicit cleanup",
                ))
                .fact(fact(
                    "frames",
                    "frames",
                    "exact safepoints frame states and roots",
                ))
                .fact(fact(
                    "charges",
                    "charges",
                    "deterministic logical resource facts",
                )),
        )
}

pub(crate) fn bytecode(ssa: ContractDigest) -> ContractDescriptor {
    descriptor(BYTECODE)
        .dependency(name(VERIFIED_SSA), ssa.as_bytes())
        .item(
            ContractItem::new("chunk", ContractItemKind::Section)
                .semantic_order()
                .fact(fact(
                    "constants",
                    "constants",
                    "bounded typed constant table",
                ))
                .fact(fact("functions", "functions", "bounded function metadata"))
                .fact(fact("code", "code", "validated instruction bytes"))
                .fact(fact("products", "products", "nominal product metadata"))
                .fact(fact("enums", "enums", "nominal enum metadata"))
                .fact(fact(
                    "contracts",
                    "contracts",
                    "full producer contract digests",
                )),
        )
}

fn descriptor(name_value: &str) -> ContractDescriptor {
    ContractDescriptor {
        name: name(name_value),
        items: Vec::new(),
        dependencies: Vec::new(),
    }
}

fn fact(id: &str, name_value: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name_value, value)
}
