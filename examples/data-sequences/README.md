# Generalized owned sequences

These literal native requests exercise an affine runtime-sized sequence containing
ordinary immutable data or owned children. The sequence remains owned even when
empty and even when its element is I64, Bool or a nested immutable record.

`library.lkjc` defines generic operations using `T: Transferable` and the native
sequence forms. Its exact transport is admitted and exported before the concrete
consumer exists. `trace.lkjc` runs operation words over I64 sequences and retains
complete event results, reverse drain order and empty-sequence reuse.

`application.lkjc` instantiates the exported generic library with I64, Bool and
records containing nested lists. A read returns an ordinary immutable value that
survives replacement, removal and disposal of its source. Replacement returns an
owned product containing both the remaining sequence and the displaced value.
The bounds and evaluation-order targets use the maintained standard operations
as well as native forms.

`tasks.lkjc` retains an ordinary read across transfer of the owning sequence into
a joined parallel task. Its output contains the earlier read, displaced value,
new value and independently transferred ordinary payload. `owned.lkjc` replaces
an owned cell, consumes the displaced cell independently, reads both remaining
children and disposes their sequence.

The shared-read target joins two tasks borrowing the same sequence. Each task
forwards a whole-sequence borrowed result before reading an ordinary payload.
After both tasks join, the parent replaces and disposes the sequence while
retaining their independent results.

The public acceptance family authors these exact inputs through a copied
executable outside the checkout, retains their transport before consumers exist,
builds deployment artifacts and repeats execution after deleting projects,
request files and transports. Its independent finite model covers all 1,555
operation words of length zero through four over six fixed actions. Additional
traces cross storage-growth boundaries; malformed input, signed index extremes,
active loans, hidden resources and finite fuel refusals must preserve meaning
and expose no partial result. These are designed acceptance witnesses, not a
claim of compiler adoption or measured speed improvement.

Run this family through the maintained checker selector for
`native_declarations::native_data_sequences::`. Set
`LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE=1` to retain copied executables, literal
requests, arguments, deployment descriptors and execution records.
