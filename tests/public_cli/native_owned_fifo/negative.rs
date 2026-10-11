use super::*;

const PREFIX: &str = r#"declarations.begin
(units
  (use std builtin)
  (module create rejected-fifo
    (type-alias Queue (owned-product (field incoming (owned-sequence ByteBuffer)) (field outgoing (owned-sequence ByteBuffer))))
"#;

#[test]
fn native_owned_fifo_rejects_read_escape_and_protected_consumption_without_publication() {
    let fifo = Fifo::new();
    let public = &fifo.consumer;
    let before = public.revision();
    let cases = [
        (
            "owning-front-call",
            r#"
    (function create bad (visibility private) (effect pure)
      (parameter create queue (type Queue) (use borrow)) (returns ByteBuffer)
      (body (call std::fifo-front (types ByteBuffer) (local queue))))
"#,
        ),
        (
            "owning-view-escape",
            r#"
    (function create bad (visibility private) (effect pure)
      (parameter create queue (type Queue) (use borrow)) (returns ByteBuffer)
      (body (borrow-call (call std::fifo-front (types ByteBuffer) (local queue))
        (binding view (type ByteBuffer)) (in (local view)))))
"#,
        ),
        (
            "consume-protected-queue",
            r#"
    (function create bad (visibility private) (effect pure)
      (parameter create queue (type Queue) (use consume)) (returns Unit)
      (body (borrow-call (call std::fifo-front (types ByteBuffer) (local queue))
        (binding view (type ByteBuffer)) (in (call std::fifo-discard (types ByteBuffer) (local queue))))))
"#,
        ),
        (
            "consume-borrowed-item",
            r#"
    (function create bad (visibility private) (effect pure)
      (parameter create queue (type Queue) (use borrow)) (returns Bytes)
      (body (borrow-call (call std::fifo-front (types ByteBuffer) (local queue))
        (binding view (type ByteBuffer)) (in (call std::buffer-freeze (local view))))))
"#,
        ),
    ];
    for (label, function) in cases {
        let source = format!("request base={before}\n{PREFIX}{function}))\ndeclarations.end\n");
        let input = public.input(&format!("rejected-{label}.lkjc"), &source);
        let records = public.plan(&input, false);
        let diagnostic = compact_record(&records, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "semantic", "{label}");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "kernel_buffer_ownership",
            "{label}"
        );
        assert_eq!(public.revision(), before);
    }
    fifo.expected(
        "fifo-forwarded",
        &json!([42]),
        &json!({"peek":42,"removed":42,"empty":0}),
        false,
    );
    fifo.verify_identity();
}
