use crate::semantic::schema::SemanticNodeKind as Kind;
use crate::source::SourceNode;

pub(super) fn call(node: &SourceNode, name: &str) -> Option<Kind> {
    Some(match name {
        "variant" if node.children.len() == 1 => Kind::ContextVariant,
        "variant-field"
            if node
                .children
                .get(1)
                .is_some_and(|child| super::type_nodes::call_name(Some(child)) == Some("type")) =>
        {
            Kind::EnumVariantField
        }
        "variant-field" => Kind::VariantValueField,
        "variant-value" => Kind::VariantValue,
        "match" => Kind::Match,
        "arms" => Kind::MatchArms,
        "arm" => Kind::MatchArm,
        "wildcard" => Kind::WildcardPattern,
        "binding" => Kind::BindingPattern,
        "bool-pattern" => Kind::BoolPattern,
        "i64-pattern" => Kind::I64Pattern,
        "variant-pattern" => Kind::VariantPattern,
        "variant-field-pattern" => Kind::VariantFieldPattern,
        "product-pattern" => Kind::ProductPattern,
        "product-field-pattern" => Kind::ProductFieldPattern,
        _ => return None,
    })
}
