# Native compiler workflow

An installed toolchain compiles source to checked K and K to static Linux x86-64
ELF using three ordinary lkjscript programs. Bash connects the processes; GNU
coreutils and findutils handle files, and util-linux supplies prlimit and
setsid for optional execution controls and owned process groups. The command path does not invoke Cargo, rustc or Node.

## Use an installed toolchain

The installed command selects the directory containing its own bin and libexec
directories. An explicit --toolchain takes precedence over LKJ_TOOLCHAIN, which
takes precedence over that installed location. The checkout's tools/lkj also
works when a toolchain is selected explicitly. It never discovers a compiler in
target or falls back to a bootstrap.

~~~sh
/path/to/toolchain/bin/lkj check examples/hello.rs
/path/to/toolchain/bin/lkj k examples/hello.rs -o hello.k
/path/to/toolchain/bin/lkj build examples/hello.rs -o hello
./hello
/path/to/toolchain/bin/lkj run examples/hello.rs -- argument
/path/to/toolchain/bin/lkj eval probe.rs
/path/to/toolchain/bin/lkj toolchain
/path/to/toolchain/bin/lkj verify
~~~

The commands check, build, run and eval accept source, a .lkjsb bundle or direct
.k text. The k command writes K from source/bundle, or checks and forwards
direct K. A successful check is silent. Build requires an output filename;
k, pack and runtime-k use stdout unless -o selects a file.

Run executes a standalone artifact. It preserves the caller's working directory,
stdin, stdout, stderr, raw argument bytes and requested exit status. A scalar
entry return has no process-stream meaning in this mode. Eval additionally prints
the scalar result through a separately checked result channel. Unit produces no
extra output; booleans, characters and integers produce a value and newline.
Floating results use exact binary64 bits, for example f64:0x3ff0000000000000.

Entry selection is main when present, otherwise probe; --entry selects another
name. The entry takes no arguments or one World and returns unit or a scalar.
Arguments following -- go to the program, including empty arguments and byte
sequences that are not UTF-8. Compiler paths/options themselves must be UTF-8.

Outputs selected by -o are published only after successful compilation, by a
temporary file and rename in the destination directory. Failure preserves an
existing output. The command rejects aliases of the primary input and, for
build, the selected runtime K. It does not parse source again to enumerate every
dependency; choose output names outside source/library inputs.

## Declared source, library and runtime

Relative source spelling is preserved. The default source directory capability
is the caller's working directory; --source-dir selects another directory. An
absolute source argument without that option selects its parent directory and
basename. With an explicit source directory, an absolute argument is made
relative to that directory. Reads still obey the World containment rules.

~~~sh
/path/to/toolchain/bin/lkj check src/main.rs \
  --source-dir /work/project --library-dir /work/library
/path/to/toolchain/bin/lkj k small.rs --no-library
/path/to/toolchain/bin/lkj pack src/main.rs -o program.lkjsb
/path/to/toolchain/bin/lkj k program.lkjsb -o program.k
/path/to/toolchain/bin/lkj build program.k \
  --runtime /work/runtime.k --entry main -o program
~~~

The selected installation's library directory and runtime.k are defaults visible
in its manifest. --library-dir and --runtime override those inputs; --no-library
selects no source library. Bundles already declare their library and source
bytes, so source-directory/library options are rejected for bundle and K inputs.

The frontend uses the production source loader for both bundles and direct
directory reads. The same loader handles inline/file modules, both candidate
module filenames, ambiguity, missing files and disabled modules. Library paths
map to stable $std/... logical names. The pack command records exact UTF-8 bytes
read by that loader, including both candidates when a module is ambiguous.
Syntax/module rejection inputs can be packed for reproduction. I/O failures
cannot be represented by silently omitting a file, and packing reports them.

~~~sh
/path/to/toolchain/bin/lkj runtime-k \
  --runtime-source library/runtime.rs -o runtime.k
~~~

Runtime preparation compiles the selected source without a library, using the
logical root $runtime/runtime.rs. It produces ordinary checked K. The K compiler
checks and seals that runtime when emitting an artifact; --check on K alone
does not establish that a runtime satisfies the raw interface.

## Install explicit native artifacts

The installer takes every compiler/library/runtime input explicitly:

~~~sh
bash tools/install-native.sh /work/toolchains/first \
  --frontend /work/artifacts/frontend.elf \
  --kc /work/artifacts/kc.elf \
  --result /work/artifacts/result.elf \
  --library library --runtime-source library/runtime.rs \
  --source-commit COMMIT_OR_DESCRIPTIVE_LABEL
~~~

The destination must not exist. Installation copies independent regular files
into a private sibling directory, rejects symlinks and special nodes in the
library tree, and prepares runtime K with the supplied frontend. It then checks
composition by compiling and executing a plain probe and a packet-enabled probe.
The latter must be decoded by the supplied result reader as exactly 42 and a
newline. Only a successful composition is renamed into place.

The installation contains bin/lkj, libexec/frontend, libexec/kc, libexec/result,
libexec/native-process.sh, library/, runtime.rs, runtime.k, manifest and SHA256SUMS. The manifest identifies
declared inputs and their hashes, the supplied source label, generated runtime
and composition checks. SHA256SUMS covers all installed files except itself;
lkj verify detects later changes. These hashes record local artifact identity,
not an external signature. Installation adds no memory or stack ceiling.

## Rebuild from an installed seed

~~~sh
bash tools/rebuild-native.sh /work/toolchains/next \
  --seed /work/toolchains/first --source-dir /work/lkjscript \
  --source-commit COMMIT_OR_DESCRIPTIVE_LABEL
~~~

The caller keeps the source checkout consistent. The script prepares the
checkout's runtime with the explicit seed, builds frontend_main.rs, kc_main.rs
and result_main.rs with the checkout's library/runtime, and calls that checkout's
installer. It builds one generation and inherits process controls. Commands, intermediate K hashes and
optional GNU time/peak-RSS measurements go to stderr. The destination is
published only after installation checks pass; private intermediate files are
removed on success or failure.

A fixed-point observation compares generations built from the same source,
library and runtime. Compare libexec/frontend, libexec/kc, libexec/result and
runtime.k byte for byte; manifests can differ because they identify different
input paths. Retain the source label, input hashes, commands and outcomes with
the observation. A seed built by B0 is a bootstrap; equality of subsequent
native generations is the separate self-build evidence.

## Results and process controls

The K compiler's optional --result-descriptor PATH writes metadata from the
same lowering that emits a packet-enabled ELF. Metadata is written completely
before any ELF bytes reach stdout. Default standalone emission remains unchanged.

Eval gives fd3 to the target for its 24-byte native result packet in a private
file. After the target exits, the shell appends its observed process status; the
native result reader checks the packet, descriptor, framing and status agreement.
Program stdout bypasses that channel, and closed descriptors stay closed.
Missing or inconsistent packets are E_HOST failures. Custom K trap text travels
in the descriptor. The current source panic form lowers to the generic message
"explicit panic"; this wrapper does not change that source behavior.

LKJ_RUN_MEMORY_BYTES and LKJ_RUN_STACK_BYTES optionally tighten only the run/eval
target's Linux soft limits, preserving inherited hard limits. Unset or unlimited
inherits existing controls, and a requested value cannot raise a tighter
inherited soft limit. The compiler and result reader keep the caller's controls.
Standalone ELFs read Linux limits directly. See limits.md for startup accounting.

TERM and INT sent to a launcher cancel its tracked work and preserve the usual
143/130 status. Compilation and rebuild stages use owned process groups so
wrappers such as GNU time cannot leave a compiler behind. Application targets
keep the caller's terminal group and inherited streams. The shared process helper
restores asynchronous children's INT/QUIT dispositions with GNU env; it introduces
no execution deadline. Relative TMPDIR values are supported, and filesystem input
paths retain the kernel meaning of symlinks and parent components.

## Development checks

~~~sh
bash tests/native_cli.sh tools/lkj
bash tests/native_install.sh
bash tests/native_rebuild.sh
bash tests/native_workflow.sh /work/toolchains/first /work/new-observation
bash tools/check.sh
~~~

The three shell contract suites use executable stubs and require no compiler.
The real workflow suite takes an installed toolchain and a new output directory,
records process outcomes, and fails if Cargo/rustc/Node are invoked. The full
gate builds native seed artifacts with its declared bootstrap and then exercises
that same installed workflow. This development bootstrap step is separate from
the native compilation/rebuild path.

Rust comparisons, Cargo unit tests, the independent K machine and Node test
orchestration remain development/reference tools. Their continued use does not
supply hidden inputs or fallbacks to the installed compiler.
