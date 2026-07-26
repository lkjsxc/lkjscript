use crate::semantic::schema::{
    ClosedBuiltinOperation, SemanticNodeKind as Kind, SemanticNodeValue as Value,
};

pub(super) fn plain(name: &str) -> (Kind, Option<Value>) {
    if let Some(operation) = crate::hir::Operation::from_name(name) {
        (
            Kind::BuiltinCall,
            Some(Value::BuiltinOperation {
                operation: ClosedBuiltinOperation(operation),
            }),
        )
    } else {
        (
            Kind::UserFunctionCall,
            Some(Value::UserFunction {
                name: name.to_string(),
            }),
        )
    }
}
