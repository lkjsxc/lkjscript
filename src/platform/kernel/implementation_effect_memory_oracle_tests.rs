//! Owned witnesses compose with authority operands without changing move rights.
use super::*;

const GENERIC: &str = r#"declarations.begin
(units (use std builtin) (module create effect-owned
  (function create transform (visibility public)
    (type-parameter create T (constraint owned))
    (effect-parameter create E)
    (requirement-parameter create R (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds))
    (implementation-parameter implparam_98000000000000000000000000000001 ops abstraction::Storage T)
    (parameter create callback (type (task-function (I64) I64 (row (parameter E)))))
    (parameter create owner (type T) (use consume))
    (returns T) (effect (task (parameter E) (requirement R)))
    (body (method-call parameter@transform@implparam_98000000000000000000000000000001
      abstraction::Storage method_10000000000000000000000000000002
      (invoke (local callback) (capability-call R std::WallClock::utc-milliseconds))
      (local owner))))
  (function create relay (visibility public)
    (type-parameter create T (constraint owned))
    (effect-parameter create E)
    (requirement-parameter create R (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds))
    (implementation-parameter implparam_98000000000000000000000000000002 ops abstraction::Storage T)
    (parameter create callback (type (task-function (I64) I64 (row (parameter E)))))
    (parameter create owner (type T) (use consume))
    (returns T) (effect (task (parameter E) (requirement R)))
    (body (implementation-call transform (types T) (effects (row (parameter E)))
      (requirements R)
      (implementations parameter@relay@implparam_98000000000000000000000000000002)
      (local callback) (local owner))))))
declarations.end
"#;

const CALLER: &str = r#"declarations.begin
(units (module create effect-caller
  (function create ready (visibility private)
    (returns I64) (effect pure) (body (i64 0)))
  (function create callback (visibility private)
    (parameter create n (type I64)) (returns I64) (effect (task)) (body (local n)))
  (function create apply (visibility public)
    (parameter create owner (type OwnedI64Cell) (use consume)) (returns OwnedI64Cell)
    (effect (task (requirement authority::clock)))
    (body (implementation-call effect-owned::relay (types OwnedI64Cell) (effects (row))
      (requirements authority::clock) (implementations concrete@cell::Scalar)
      (function-value callback) (local owner))))
  (component create authority (visibility private)
    (requirement create clock (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds) (limits (maximum_calls 4 calls)))
    (port create ready (type (function () I64)) (function ready)))))
declarations.end
"#;

fn source() -> KernelSnapshot {
    // Like the maintained task-method memory fixture, use a source-defined
    // interface here. Public package tests separately cover the builtin adapter;
    // this oracle isolates ownership and exact lexical witness forwarding.
    let authored = format!(
        "{}{}{}{}",
        include_str!("../../../tests/fixtures/owned-witness-library.lkjc"),
        GENERIC,
        include_str!("../../../tests/fixtures/owned-witness-cell.lkjc"),
        CALLER
    )
    .replace("(use std builtin)", "")
    .replace("std::WallClock", "memory_clock::WallClock");
    let input = format!(
        "declarations.begin\n(units (module create memory_clock
          (interface create WallClock (visibility public)
            (operation create utc-milliseconds (returns I64)
              (idempotency idempotent) (external-visibility none)))))\ndeclarations.end\n{authored}"
    );
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&input).unwrap()
}

#[test]
fn memory_oracle_admits_effect_and_requirement_witness_forwarding_and_retains_move_rules() {
    let source = source();
    assert!(accepts(&source));
    let (relay, actual) = source
        .owners
        .values()
        .filter_map(|owner| match owner {
            OwnerRecord::Declaration(d) => match &d.payload {
                DeclarationPayload::Function(f) => Some((d, f)),
                _ => None,
            },
            _ => None,
        })
        .fold((None, None), |(relay, actual), (d, f)| {
            (
                if d.name.as_str() == "relay" {
                    Some(f)
                } else {
                    relay
                },
                if d.name.as_str() == "apply" {
                    Some(f)
                } else {
                    actual
                },
            )
        });
    let relay = relay.unwrap();
    let actual = actual.unwrap();
    let mut borrowed = source.clone();
    let OwnerRecord::Parameter(parameter) = borrowed
        .owners
        .get_mut(&OwnerKey::Parameter(actual.parameters[0]))
        .unwrap()
    else {
        unreachable!()
    };
    parameter.use_mode = ParameterUse::Borrow;
    assert!(
        !accepts(&borrowed),
        "borrowed owners cannot satisfy task consumes"
    );

    let mut foreign = source.clone();
    let OwnerRecord::Expression(expression) = foreign
        .owners
        .get_mut(&OwnerKey::Expression(relay.body))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    let ImplementationOperand::Parameter { scope, .. } = &mut implementations[0] else {
        unreachable!()
    };
    scope.declaration = crate::platform::semantic_id::DeclarationId::migrate(b"foreign-scope", 0);
    assert!(
        !accepts(&foreign),
        "matching Self does not authorize a foreign witness formal"
    );
}
