use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::{name, LANGUAGE, SOURCE};

mod vocabulary;

pub(super) fn language() -> ContractDescriptor {
    vocabulary::extend(
        descriptor(LANGUAGE).item(
            ContractItem::new("semantic-forms", ContractItemKind::Rule)
                .fact(fact(
                    "generic-enums",
                    "generic enums",
                    "nominal invariant algebraic data",
                ))
                .fact(fact(
                    "match",
                    "match",
                    "closed exhaustive usefulness-checked patterns",
                ))
                .fact(fact("never", "never", "uninhabited join-only control type"))
                .fact(fact(
                    "control",
                    "structured control",
                    "return loop while break continue",
                ))
                .fact(fact(
                    "numeric",
                    "numeric conversions",
                    "four explicit checked operations",
                ))
                .fact(fact("option", "option", "generic enum some(t)|none"))
                .fact(fact("result", "result", "generic enum ok(t)|err(e)"))
                .fact(fact("errors", "typed errors", "closed nominal error enums"))
                .fact(fact(
                    "effects",
                    "effects",
                    "compiler-derived and not authority",
                ))
                .fact(fact(
                    "ownership",
                    "ownership",
                    "type-derived move borrow and drop facts",
                ))
                .fact(fact(
                    "capabilities",
                    "capabilities",
                    "eight closed explicit unforgeable provider values",
                ))
                .fact(fact(
                    "paths",
                    "path",
                    "opaque absolute byte-preserving Linux pathname value",
                )),
        ),
    )
}

pub(super) fn source(language: ContractDigest) -> ContractDescriptor {
    descriptor(SOURCE)
        .dependency(name(LANGUAGE), language.as_bytes())
        .item(
            ContractItem::new("canonical-grammar", ContractItemKind::Rule)
                .fact(fact("suffix", "source suffix", ".lkjscript"))
                .fact(fact(
                    "projection",
                    "projection",
                    "line-oriented slash-delimited structural forms",
                ))
                .fact(fact("language-selection", "language selection", "absent"))
                .fact(fact(
                    "module",
                    "module identity",
                    "exact package-root-relative source path",
                ))
                .fact(fact(
                    "signatures",
                    "signatures",
                    "structured inputs and output fields without arrow atoms",
                ))
                .fact(fact(
                    "imports",
                    "imports",
                    "structured module path and sorted declaration children",
                ))
                .fact(fact(
                    "visibility",
                    "visibility",
                    "private by default explicit public declaration field",
                ))
                .fact(fact("unknown-fields", "unknown forms", "rejected"))
                .fact(fact(
                    "identity",
                    "source identity",
                    "contract logical-path exact-bytes package",
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
