use crate::{
    CapabilityKind, ContractDescriptor, ContractFact, ContractItem, ContractItemKind,
    OperationCategory, OperationIdentity, OperationOwnership, ResourceKind, RuntimeLowering,
    SemanticConstructor, BUILTIN_ERROR_NAMES, BYTE_TEXT_FOUNDATION_TYPE_NAMES,
    COMPILER_TRAIT_NAMES, CONTEXTUAL_FORM_NAMES, OPERATION_COUNT, PRELUDE_TYPE_NAMES,
    PRELUDE_VARIANT_NAMES, REMOVED_SPELLINGS, RESERVED_WORDS, SIMPLE_TYPE_NAMES,
    TYPE_CONSTRUCTOR_NAMES,
};

pub(super) fn extend(mut descriptor: ContractDescriptor) -> ContractDescriptor {
    descriptor.items.push(
        ContractItem::new("identifier-vocabulary", ContractItemKind::Rule)
            .fact(required(
                "grammar",
                "identifier grammar",
                "[a-z][a-z0-9]*(?:-[a-z0-9]+)*",
            ))
            .fact(required("case", "case", "exact lowercase ASCII bytes"))
            .fact(required(
                "symbolic-semantics",
                "symbolic semantics",
                "rejected",
            )),
    );
    add_names(&mut descriptor, "simple-types", SIMPLE_TYPE_NAMES);
    add_names(
        &mut descriptor,
        "byte-text-foundation-types",
        BYTE_TEXT_FOUNDATION_TYPE_NAMES,
    );
    add_names(&mut descriptor, "type-constructors", TYPE_CONSTRUCTOR_NAMES);
    add_names(&mut descriptor, "builtin-errors", BUILTIN_ERROR_NAMES);
    add_names(&mut descriptor, "compiler-traits", COMPILER_TRAIT_NAMES);
    add_names(&mut descriptor, "prelude-types", PRELUDE_TYPE_NAMES);
    add_names(&mut descriptor, "prelude-variants", PRELUDE_VARIANT_NAMES);
    add_names(&mut descriptor, "contextual-forms", CONTEXTUAL_FORM_NAMES);
    add_names(&mut descriptor, "reserved-words", RESERVED_WORDS);
    add_removed_spellings(&mut descriptor);
    add_capabilities(&mut descriptor);
    add_resources(&mut descriptor);
    add_operations(&mut descriptor);
    descriptor
}

fn add_names(descriptor: &mut ContractDescriptor, id: &str, names: &[&str]) {
    let mut item = ContractItem::new(id, ContractItemKind::Rule).semantic_order();
    for (index, name) in names.iter().enumerate() {
        item.facts.push(required(&index.to_string(), name, name));
    }
    descriptor.items.push(item);
}

fn add_removed_spellings(descriptor: &mut ContractDescriptor) {
    let mut item = ContractItem::new("removed-spellings", ContractItemKind::Rule).semantic_order();
    for (index, record) in REMOVED_SPELLINGS.iter().enumerate() {
        item.facts
            .push(required(&index.to_string(), record.old, record.replacement));
    }
    descriptor.items.push(item);
}

fn add_capabilities(descriptor: &mut ContractDescriptor) {
    let mut item =
        ContractItem::new("capability-kinds", ContractItemKind::Capability).semantic_order();
    for kind in CapabilityKind::ALL {
        item.facts.push(required(
            &(kind as u8).to_string(),
            kind.as_str(),
            kind.as_str(),
        ));
    }
    descriptor.items.push(item);
}

fn add_resources(descriptor: &mut ContractDescriptor) {
    let mut item =
        ContractItem::new("typed-resource-kinds", ContractItemKind::Type).semantic_order();
    for kind in ResourceKind::ALL {
        item.facts.push(required(
            &(kind as u8).to_string(),
            kind.as_str(),
            "affine;copy=false;send=false;sync=false",
        ));
    }
    descriptor.items.push(item);
}

fn add_operations(descriptor: &mut ContractDescriptor) {
    let mut item =
        ContractItem::new("source-operations", ContractItemKind::Operation).semantic_order();
    for index in 0..OPERATION_COUNT {
        let Some(record) = crate::operation_by_id(OperationIdentity::new(index as u16)) else {
            continue;
        };
        let semantics = record.semantics;
        let capabilities = semantics
            .capability_requirements
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let value = format!(
            concat!(
                "category={};arity={};type={};generic={};constraints={};effects={};capabilities={};",
                "ownership={};may-trap={};may-diverge={};lowering={};semantic-constructor={};",
                "legal-constructor={};summary={}"
            ),
            category(record.category),
            semantics.arity,
            semantics.type_scheme,
            semantics.generic_variables.join(","),
            semantics.generic_constraints.join(","),
            semantics.effects.0,
            capabilities,
            ownership(semantics.ownership),
            semantics.may_trap,
            semantics.may_diverge,
            lowering(semantics.runtime_lowering),
            semantic_constructor(semantics.semantic_constructor),
            semantics.legal_constructor_available,
            record.summary,
        );
        item.facts
            .push(required(record.stable_name, record.source_name, &value));
    }
    descriptor.items.push(item);
}

fn required(id: &str, name: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name, value)
}

fn ownership(ownership: OperationOwnership) -> &'static str {
    match ownership {
        OperationOwnership::Observes => "observes",
        OperationOwnership::Allocates => "allocates",
        OperationOwnership::Mutates => "mutates",
        OperationOwnership::ConsumesResource => "consumes-resource",
        OperationOwnership::ConsumesOwner => "consumes-owner",
    }
}

fn lowering(lowering: RuntimeLowering) -> &'static str {
    match lowering {
        RuntimeLowering::Control => "control",
        RuntimeLowering::NumericConversion => "numeric-conversion",
        RuntimeLowering::Enum => "enum",
        RuntimeLowering::RuntimeCall => "runtime-call",
    }
}

fn semantic_constructor(relationship: SemanticConstructor) -> &'static str {
    match relationship {
        SemanticConstructor::BuiltinCall => "builtin-call",
        SemanticConstructor::ControlForm => "control-form",
    }
}

fn category(category: OperationCategory) -> &'static str {
    match category {
        OperationCategory::Arithmetic => "arithmetic",
        OperationCategory::Ordering => "ordering",
        OperationCategory::Equality => "equality",
        OperationCategory::Boolean => "boolean",
        OperationCategory::Bit => "bit",
        OperationCategory::Conversion => "conversion",
        OperationCategory::List => "list",
        OperationCategory::Text => "text",
        OperationCategory::ByteData => "byte-data",
        OperationCategory::Path => "path",
        OperationCategory::Arguments => "arguments",
        OperationCategory::Stdio => "stdio",
        OperationCategory::Resource => "resource",
        OperationCategory::File => "file",
        OperationCategory::Entropy => "entropy",
        OperationCategory::Sqlite => "sqlite",
        OperationCategory::Network => "network",
        OperationCategory::Terminal => "terminal",
        OperationCategory::Control => "control",
        OperationCategory::Variant => "variant",
    }
}
