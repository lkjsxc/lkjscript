//! A native edit may retain body identities only after complete canonical-intent equality.

use super::canonical::{Reader, error};
use super::*;
use crate::platform::change::{AuthoredLiteralUpdate, AuthoredLiteralValue, ChangeBudget};

pub(super) fn function_body(
    reader: &mut Reader<'_>,
    function: DeclarationSelector,
    body: AuthoredExpression,
    mut previous: AuthoredExpression,
    base: RevisionId,
    budget: ChangeBudget,
) -> Result<Option<AuthoredChange>, Diagnostic> {
    let encode = |body| {
        crate::platform::change::canonical_authored_intent_bytes(&AuthoredChangeSet {
            base,
            preconditions: Vec::new(),
            changes: vec![AuthoredChange::ReplaceFunctionBody {
                function: function.clone(),
                body,
            }],
            budget,
        })
    };
    // A proposal can refer to new request-local declarations that this isolated comparison
    // cannot encode. In that case preserve the existing whole-body replacement path.
    let proposed = encode(body.clone()).ok();
    if proposed.as_ref() == Some(&encode(previous.clone())?) {
        return Ok(None);
    }
    let mut literals = Vec::new();
    if proposed.is_some()
        && substitute(reader, &mut previous, &body, &mut literals, 1)?
        && encode(previous).ok() == proposed
    {
        if literals.is_empty() {
            return Err(error("changed literal-only body has no literal updates"));
        }
        Ok(Some(AuthoredChange::SetFunctionLiterals {
            function,
            literals,
        }))
    } else {
        Ok(Some(AuthoredChange::ReplaceFunctionBody { function, body }))
    }
}

fn substitute(
    reader: &Reader<'_>,
    previous: &mut AuthoredExpression,
    proposed: &AuthoredExpression,
    literals: &mut Vec<AuthoredLiteralUpdate>,
    depth: usize,
) -> Result<bool, Diagnostic> {
    reader.check()?;
    if depth > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
        return Err(error(
            "literal comparison exceeds complete body depth admission",
        ));
    }
    if let Some(value) = AuthoredLiteralValue::from_authored(&proposed.operation) {
        let Some(old) = AuthoredLiteralValue::from_authored(&previous.operation) else {
            return Ok(false);
        };
        if std::mem::discriminant(&old) != std::mem::discriminant(&value) {
            return Ok(false);
        }
        if old != value {
            // This identity comes solely from Reader's accepted-body reconstruction,
            // never from a proposed source label or displayed spelling.
            let expression = previous
                .symbol
                .as_deref()
                .and_then(|symbol| symbol.strip_prefix("$e_"))
                .ok_or_else(|| error("canonical literal has no exact expression identity"))?
                .parse()?;
            previous.operation = value.authored();
            literals.push(AuthoredLiteralUpdate { expression, value });
        }
        return Ok(true);
    }
    use AuthoredExpressionOperation as A;
    let mut pairs: Vec<(&mut AuthoredExpression, &AuthoredExpression)> = Vec::new();
    match (&mut previous.operation, &proposed.operation) {
        (A::Unit {}, A::Unit {})
        | (A::Local { .. }, A::Local { .. })
        | (A::Constant { .. }, A::Constant { .. })
        | (A::FunctionValue { .. }, A::FunctionValue { .. }) => {}
        (
            A::If {
                condition: a,
                when_true: b,
                when_false: c,
            },
            A::If {
                condition: x,
                when_true: y,
                when_false: z,
            },
        ) => {
            pairs.extend([
                (a.as_mut(), x.as_ref()),
                (b.as_mut(), y.as_ref()),
                (c.as_mut(), z.as_ref()),
            ]);
        }
        (
            A::Let {
                bindings: a,
                body: b,
            },
            A::Let {
                bindings: x,
                body: y,
            },
        ) => {
            if a.len() != x.len() {
                return Ok(false);
            }
            pairs.extend(a.iter_mut().zip(x).map(|(a, x)| (&mut a.value, &x.value)));
            pairs.push((b, y));
        }
        (A::Sequence { items: a }, A::Sequence { items: x })
        | (A::List { items: a, .. }, A::List { items: x, .. })
        | (A::Call { arguments: a, .. }, A::Call { arguments: x, .. })
        | (A::CapabilityCall { arguments: a, .. }, A::CapabilityCall { arguments: x, .. }) => {
            if a.len() != x.len() {
                return Ok(false);
            }
            pairs.extend(a.iter_mut().zip(x));
        }
        (
            A::Invoke {
                callee: a,
                arguments: b,
            },
            A::Invoke {
                callee: x,
                arguments: y,
            },
        )
        | (
            A::Bind {
                callee: a,
                arguments: b,
            },
            A::Bind {
                callee: x,
                arguments: y,
            },
        ) => {
            if b.len() != y.len() {
                return Ok(false);
            }
            pairs.push((a, x));
            pairs.extend(b.iter_mut().zip(y));
        }
        (A::Record { fields: a, .. }, A::Record { fields: x, .. }) => {
            if a.len() != x.len() {
                return Ok(false);
            }
            pairs.extend(a.iter_mut().zip(x).map(|(a, x)| (&mut a.value, &x.value)));
        }
        (A::Variant { payload: a, .. }, A::Variant { payload: x, .. }) => match (a, x) {
            (Some(a), Some(x)) => pairs.push((a, x)),
            (None, None) => {}
            _ => return Ok(false),
        },
        (A::Field { value: a, .. }, A::Field { value: x, .. }) => pairs.push((a, x)),
        (A::Map { entries: a, .. }, A::Map { entries: x, .. }) => {
            if a.len() != x.len() {
                return Ok(false);
            }
            for (a, x) in a.iter_mut().zip(x) {
                pairs.extend([(&mut a.key, &x.key), (&mut a.value, &x.value)]);
            }
        }
        (A::Match { value: a, arms: b }, A::Match { value: x, arms: y }) => {
            if b.len() != y.len() {
                return Ok(false);
            }
            pairs.push((a, x));
            pairs.extend(b.iter_mut().zip(y).map(|(b, y)| (&mut b.body, &y.body)));
        }
        (A::Transaction { body: a, .. }, A::Transaction { body: x, .. })
        | (A::TransactionOutcome { body: a, .. }, A::TransactionOutcome { body: x, .. }) => {
            pairs.push((a, x))
        }
        _ => return Ok(false),
    }
    for (previous, proposed) in pairs {
        if !substitute(reader, previous, proposed, literals, depth + 1)? {
            return Ok(false);
        }
    }
    // Matching child slots is only a candidate correspondence. function_body must still
    // compare the complete substituted intent, including types, references and binding scope.
    Ok(true)
}
