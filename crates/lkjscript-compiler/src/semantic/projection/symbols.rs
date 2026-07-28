use super::type_nodes;
use crate::semantic::schema::{SemanticNodeKind as Kind, SemanticNodeValue as Value};
use crate::source::SourceNode;

pub(super) fn classify(
    name: &str,
    parent: Option<&SourceNode>,
    parent_kind: Option<Kind>,
    index: usize,
) -> (Kind, Option<Value>) {
    let kind = match (parent_kind, index) {
        (Some(Kind::TypedHole), 0) => Kind::HoleIdentity,
        (Some(Kind::TypedHole), _) => Kind::HoleGoal,
        (Some(Kind::Parameters), index) if index.is_multiple_of(2) => Kind::ParameterName,
        (Some(Kind::Parameters), _) => type_nodes::classify(name, parent, index),
        (Some(Kind::ImportDeclarations), _) => Kind::ImportDeclaration,
        (Some(Kind::TypeVariables), _) => Kind::TypeVariable,
        (Some(Kind::TypeCapability), _) => Kind::CapabilityKind,
        (Some(Kind::TypeProduct), _) => Kind::ProductName,
        (Some(Kind::TypeEnum | Kind::TypeList | Kind::TypeOption | Kind::TypeResult), _) => {
            type_nodes::classify(name, parent, index)
        }
        (Some(Kind::Bound), 0) => Kind::TypeVariable,
        (Some(Kind::Bound), _) => Kind::TraitName,
        (Some(Kind::ProductValue), 0) => Kind::ProductName,
        (Some(Kind::ContextVariant), 0) => Kind::VariantName,
        (Some(Kind::ProductValueField | Kind::VariantValueField), 0) => Kind::FieldName,
        (Some(Kind::FieldAccess), 1) | (Some(Kind::WithField), 1) => Kind::FieldName,
        (Some(Kind::Bind), 0) => Kind::BindingName,
        (Some(Kind::Set), 0) => Kind::MutableName,
        (Some(Kind::Quote), _) => Kind::QuotedName,
        (Some(Kind::Move | Kind::Borrow | Kind::BorrowMut), _) => Kind::PlaceName,
        (Some(Kind::ContextTrait), _) => Kind::TraitName,
        (
            Some(
                Kind::Signature
                | Kind::SignatureInputs
                | Kind::SignatureOutput
                | Kind::ContextType
                | Kind::ContextFor
                | Kind::EmptyList
                | Kind::None,
            ),
            _,
        ) => type_nodes::classify(name, parent, index),
        _ => Kind::NameReference,
    };
    (
        kind,
        Some(Value::SourceName {
            name: name.to_string(),
        }),
    )
}
