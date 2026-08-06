use super::{Expression, ExpressionCounts};

// Measurements traverse a materialized host tree; checked u64 arithmetic documents the invariant.
#[allow(clippy::expect_used)]
impl Expression {
    pub(crate) fn measure(&self, depth: u64, counts: &mut ExpressionCounts) {
        counts.nodes = counts
            .nodes
            .checked_add(1)
            .expect("host-addressable expression trees fit u64");
        counts.depth = counts.depth.max(depth);
        self.measure_strings(counts);
        let next = depth
            .checked_add(1)
            .expect("host-addressable expression depth fits u64");
        match self {
            Self::EmptyList { element } => element.measure(next, counts),
            Self::None { value_type } => value_type.measure(next, counts),
            Self::Let { bindings, body } => {
                for binding in bindings {
                    binding.value.measure(next, counts);
                }
                body.measure(next, counts);
            }
            Self::Var {
                value_type,
                initial,
                body,
                ..
            } => {
                value_type.measure(next, counts);
                initial.measure(next, counts);
                body.measure(next, counts);
            }
            Self::Set { value, .. } | Self::Field { value, .. } => value.measure(next, counts),
            Self::If {
                condition,
                then_branch,
                else_branch,
            } => {
                condition.measure(next, counts);
                then_branch.measure(next, counts);
                else_branch.measure(next, counts);
            }
            Self::While { condition, body } => {
                condition.measure(next, counts);
                measure_many(body, next, counts);
            }
            Self::Loop { result_type, body } => {
                result_type.measure(next, counts);
                measure_many(body, next, counts);
            }
            Self::Return { value }
            | Self::Break { value }
            | Self::Trap { value }
            | Self::Exit { code: value } => value.measure(next, counts),
            Self::Do { expressions } => measure_many(expressions, next, counts),
            Self::ProductValue { fields, .. } | Self::VariantValue { fields, .. } => {
                for field in fields {
                    field.value.measure(next, counts);
                }
                if let Self::VariantValue { value_type, .. } = self {
                    value_type.measure(next, counts);
                }
            }
            Self::Match { scrutinee, arms } => {
                scrutinee.measure(next, counts);
                super::matching::measure_arms(arms, next, counts);
            }
            Self::WithField {
                value, replacement, ..
            } => {
                value.measure(next, counts);
                replacement.measure(next, counts);
            }
            Self::BuiltinCall { arguments, .. } | Self::UserCall { arguments, .. } => {
                measure_many(arguments, next, counts);
            }
            _ => {}
        }
    }

    fn measure_strings(&self, counts: &mut ExpressionCounts) {
        let bytes = match self {
            Self::F64 { value } | Self::String { value } => value.len(),
            Self::TypedHole { identity, goal } => {
                identity.len() + goal.as_ref().map_or(0, String::len)
            }
            Self::NameReference { name }
            | Self::Quote { name }
            | Self::Move { name }
            | Self::Borrow { name }
            | Self::BorrowMut { name }
            | Self::Set { name, .. }
            | Self::Var { name, .. }
            | Self::UserCall { name, .. } => name.len(),
            Self::ProductValue { product, fields } => {
                product.len() + fields.iter().map(|field| field.name.len()).sum::<usize>()
            }
            Self::VariantValue {
                variant, fields, ..
            } => variant.len() + fields.iter().map(|field| field.name.len()).sum::<usize>(),
            Self::Field { field, .. } | Self::WithField { field, .. } => field.len(),
            Self::Let { bindings, .. } => bindings.iter().map(|binding| binding.name.len()).sum(),
            _ => 0,
        };
        counts.string_bytes = counts
            .string_bytes
            .checked_add(u64::try_from(bytes).expect("host string bytes fit u64"))
            .expect("materialized expression strings fit u64");
    }
}

fn measure_many(values: &[Expression], depth: u64, counts: &mut ExpressionCounts) {
    for value in values {
        value.measure(depth, counts);
    }
}
