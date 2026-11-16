# Retrospective standalone-operation command record

This file was written after the two archived operations. The request IDs and cwd were
independently recovered from saved gateway results. Those objects do not expose
submitted argv; the argument list below is reconstructed from the invocation
transcript. It is not a predeclared or contemporaneously saved argv file.

Gateway compiler-container-case-apply-922 (request 1 refusal):
cwd /home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-container-case-migration
producer /home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-container-baseline
producer SHA256 adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7
compiler arguments:
  edit src/query_store_cases.lkj
  --source-dir /home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-container-case-migration
  --library-dir /home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-container-case-migration/library
  --no-store
  --store-key (private external key selected by operation-key.path)
  --request /home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-container-evidence/cases-request-1.json

Gateway compiler-container-case-apply2-928 (request 2 applied):
same cwd, producer, source selector, library selector, no-store and private key;
request /home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-container-evidence/cases-request-2.json

Both used the retained 4,294,967,296-byte address-space limit and 900-second
timeout with 10-second grace. The exact before/after file inventories, request
digests, stdout/stderr, status, timing and loads are retained as cases-*.
No explicit runtime-source was supplied to either edit/check command.
The selected six-file source closure and all 32 library files match the archived
checked closure. Physical diagnostic prefixes differ from the eventual copied
test root; native test execution later uses an explicitly supplied runtime.

No private key contents are stored here. The subsequent case-only correction
has its own contemporaneously retained cases-apply-3.command and evidence.
