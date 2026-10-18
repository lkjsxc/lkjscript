# Semantic compiler bootstrap

The target is semantic self-hosting. The current component is an intermediate
backend compiler with Rust source admission and normalized export still required.
It validates bounded LKJCI1 profile 1 and emits standalone Linux x86_64 ELF.
It does not yet emit `.lkja` or compile itself. See the
[contract, schema and dependency inventory](../../docs/spec/selfhosting.md).

`project/` owns accepted meaning. `input.lkjc` and `elf.lkjc` are maintained
authoring proposals. Changes must pass ordinary graph admission before publication.
The Rust test helper exports admitted input programs; both the VM and disjoint
reference interpreter execute the same lkjscript compiler. Tests validate the
generated ELF and execute it in clean child environments, comparing independent
expected results/traps as well as the existing Rust backend.

Run the development bootstrap harness from the repository root:

```sh
bash tools/selfhost/verify.sh
```

This focused harness is not full acceptance. Run the repository's official
changed, fresh full and release-target checks after updating status and all
maintained source. Native compiler execution, full backend closure, semantic
admission, regions and the C2/C3 fixed point remain required work.
