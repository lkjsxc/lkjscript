# Complete-family census preparation

This separately authorized offline inventory reads only the accepted complete
20-body extraction and the immutable all-row join. It does not rerun a compiler,
profiler, official reader, extractor or raw-row join, and requests no extra page.

Reader families.pl SHA-256:
b4f3df6b6ffc443d7b3baf839b4e1dbdbb664a21f8ff366a437f05c5114113a6.

Inputs:
- /home/coder/workspace/lkjscript/target/compiler-copy-sites-census-output.20261116-1
  manifest 63d59716e15c302d241d9e19ba792a0d715b0a0e25aeda75827b24be0799d480.
- /home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-copy-sites-census-join-20261116/phase-1/output
  manifest b69c1d21e9ac89fde114dda7a0abee592bc16698c94cb902a4de669193ddff31.

The function population/order is identical to the original selection. Every
manifest record is verified. The tool parses all instructions in all 20 complete
bodies and all required type/destructor records, preserving original coordinates.
Explicit operand-position tables cover every current IR operation; unknown
operations refuse. Numeric constants, function indices and variant numbers are
not mistaken for local-variable uses.

Every flat Copy IR-Struct-shaped local is listed, including parameters, projected
reads, assignments, call results and zero/no-sample values. Every Struct
constructor is enumerated, including type-blocked ones and constructors whose
copied payloads are scalar. For a flat-shaped root, the candidate family is closed
over exact same-type whole-local Read copy/move edges. Ambiguous local versions
or multiple definitions block the initial subset; that closure is not a claimed
version-sensitive dataflow proof.

All member definitions/uses, constructor inputs, complete syntactic control/call
sites and exact associated existing copy-row IDs are retained. Roots with no
recorded interval still appear. Interval association includes constructor and
member-event instructions, so scalar field-copy sites are retained. The same
interval may be relevant to multiple families and is not summed across families
as removable time.

Known initial-rule violations are separate from unresolved nominal origin,
initialization/consumption and control/lifetime facts. Whole/partial writes,
aggregate call/return/container use, borrows and disallowed projected moves are
recorded as blockers. A final textual use is never treated as a lifetime proof.
The tool does not issue an eligible verdict. Complete evidence is intended for
the subsequent bounded manual family review.

The flat scalar shape's extent/alignment follows the pinned layout source
f88d1ff9a50ce796d2fc10bfa04cd4e8d73a30d8284d5a9b3bcd3488f724daf0.
Without custom destructors, those scalar fields and their Copy aggregate have no
native cleanup; blocked nonflat shapes are not assigned invented layout/cleanup
facts. IR Struct alone does not distinguish nominal source structs from tuples.

The wrapper exclusively claims phase-1 and runs the copied reader once under
4 GiB address space / 300 seconds / 10 seconds grace, capturing exact script,
argv/cwd, limits, time/status, loads, stdout and stderr. Any failure is retained;
the prior join, captures and warning bytes stay unchanged. No production or
tracked source is edited.
