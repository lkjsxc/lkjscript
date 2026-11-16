# Prelaunch numeric-stat validation amendment

Root and self-review identified that Perl numeric comparison alone could coerce a malformed JSON string into zero. The only program change adds unsigned_json_integer and uses it before the three existing comparison sites: required zero reuse/inline fields, optional store fields, and emitted_bytes. Undefined values, references/booleans, strings and non-integer numeric encodings cannot satisfy the predicate. No invocation, order, producer/input, resource, capture boundary, output comparison or interpretation policy changes.

The complete V1 preparation is retained under review-v1. Its20-record preparation manifest is add298913344c318bef0600c82c6b454fbfbd652289a1e8fc215a83cf63207ef; execution-control manifest291425ebfe187959ab5cd505d6a163349ef7f214be52da50c7a34cfbe08f3971; helper860c1bc87888c5e9730649587143d97cbb76c2aea5cf894f79c649746326287d. Every V1 manifest entry was rechecked after copying.

This is a preparation-only correction before any trial helper mode or measured command. The exact diff and fresh Bash/Perl syntax-only captures are in syntax-2. No validator test, compiler, native image or benchmark was executed to make this amendment.
