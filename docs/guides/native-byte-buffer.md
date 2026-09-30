# Native owned byte buffers

This guide describes the development ByteBuffer increment. Check the selected executable's discovery output; immutable Bytes operations alone do not provide ownership or read borrowing.

The literal native requests build a [generic producer](../../tests/fixtures/owned-buffer-producer.lkjc), [consuming transformer](../../tests/fixtures/owned-buffer-transformer.lkjc) and [borrowed consumer](../../tests/fixtures/owned-buffer-consumer.lkjc). They use ordinary data and callbacks as generic arguments, with concrete ByteBuffer results and final memory parameters. The consumer emits exactly three octets, NUL, 255 and 128, without text conversion. No memory grant is needed.

Use the [native command workflow](native-command.md) to create a command project, bind a request to `status`'s current revision, plan, apply and check it. Import `(use std builtin)` to discover `buffer-empty`, `buffer-push`, `buffer-get`, `buffer-length`, `buffer-freeze` and `buffer-discard`. A local buffer binding and borrow helper are authored directly:

```text
(function create count (visibility private)
  (parameter create b (type ByteBuffer) (use borrow))
  (returns I64) (effect pure)
  (body (call std::buffer-length (local b))))
(function create packet (visibility public) (returns Bytes) (effect pure)
  (body (let
    (binding empty (type ByteBuffer) (call std::buffer-empty))
    (binding next (type ByteBuffer) (call std::buffer-push (i64 255) (local empty)))
    (binding observed (call count (local next)))
    (in (call std::buffer-freeze (local next))))))
```

Push moves the old owner into a new owned result. Count reads without consuming it. Freeze moves the final owner into unrestricted immutable Bytes. Unused owners are dropped at lexical exit. Index errors, invalid octets, quota refusal and cancellation release transient storage rather than publishing a partial result.

Export the producer with `package current export --kind transport --output producer.lkjp`. Stage its exact transport in a second project and add its package, semantic revision and package revision through `add.dependency`. Replace the transformer's `PRODUCER_PACKAGE` and `PRODUCER_PACKAGE_REVISION` placeholders with those exported values. Export the accepted transformer in turn, stage both exact transports in a third project, and bind both dependencies before applying the consumer request. The [public CLI test](../../tests/public_cli/native_byte_buffer.rs) retains this complete workflow, unchanged draft re-entry, source checks, artifact construction and execution after all three source projects have been removed. Detached deployment selects target `memory` with empty grants; JSON output is `{"$bytes":"AP+A"}`.

Borrowing is synchronous and read-only. Buffer signatures remain direct named calls, not first-class function values. ByteBuffer is forbidden as an unrestricted generic argument, including an unused or phantom one, and cannot be stored in ordinary containers or persistence. Task bodies may use locals, but memory task signatures and mixed capability/memory signatures are unsupported. Freeze at raw, adapter and durable boundaries. The [normative scope](../spec/owned-byte-buffers.md) describes exact modes, rejection rules, branch drops and borrowed tail-call lifetimes.
