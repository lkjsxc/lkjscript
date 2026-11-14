# Native compiler workflow

An installed toolchain compiles source to checked lkjscript IR and IR to static Linux x86-64
ELF using three ordinary lkjscript programs. Bash connects the processes;
coreutils and findutils handle files, and util-linux supplies prlimit and
setsid for optional execution controls and owned process groups. The command path does not invoke Cargo, rustc or Node.

## Use an installed toolchain

The installed command selects the directory containing its own bin and libexec
directories. An explicit --toolchain takes precedence over LKJSCRIPT_TOOLCHAIN, which
takes precedence over that installed location. The checkout's tools/lkjscript also
works when a toolchain is selected explicitly. It never discovers a compiler in
target or falls back to a bootstrap.

~~~sh
/path/to/toolchain/bin/lkjscript check examples/hello.rs
/path/to/toolchain/bin/lkjscript ir examples/hello.rs -o hello.lkjir
/path/to/toolchain/bin/lkjscript build examples/hello.rs -o hello
./hello
/path/to/toolchain/bin/lkjscript run examples/hello.rs -- argument
/path/to/toolchain/bin/lkjscript eval probe.rs
/path/to/toolchain/bin/lkjscript toolchain
/path/to/toolchain/bin/lkjscript verify
~~~

The commands check, build, run and eval accept source, a .lkjsb bundle or direct
.lkjir text. The ir command writes IR from source/bundle, or checks and forwards
direct IR. A successful check is silent. Build requires an output filename;
ir, pack and runtime-ir use stdout unless -o selects a file.

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
build, the selected runtime IR. It does not parse source again to enumerate every
dependency; choose output names outside source/library inputs.

## Declared source, library and runtime

Relative source spelling is preserved. The default source directory capability
is the caller's working directory; --source-dir selects another directory. An
absolute source argument without that option selects its parent directory and
basename. With an explicit source directory, an absolute argument is made
relative to that directory. Reads still obey the World containment rules.

~~~sh
/path/to/toolchain/bin/lkjscript check src/main.rs \
  --source-dir /work/project --library-dir /work/library
/path/to/toolchain/bin/lkjscript ir small.rs --no-library
/path/to/toolchain/bin/lkjscript pack src/main.rs -o program.lkjsb
/path/to/toolchain/bin/lkjscript ir program.lkjsb -o program.lkjir
/path/to/toolchain/bin/lkjscript build program.lkjir \
  --runtime /work/runtime.lkjir --entry main -o program
~~~

The selected installation's library directory and runtime.lkjir are defaults visible
in its manifest. --library-dir and --runtime override those inputs; --no-library
selects no source library. Bundles already declare their library and source
bytes, so source-directory/library options are rejected for bundle and IR inputs.

The frontend uses the production source loader for both bundles and direct
directory reads. The same loader handles inline/file modules, both candidate
module filenames, ambiguity, missing files and disabled modules. Library paths
map to stable $std/... logical names. The pack command records exact UTF-8 bytes
read by that loader, including both candidates when a module is ambiguous.
Syntax/module rejection inputs can be packed for reproduction. I/O failures
cannot be represented by silently omitting a file, and packing reports them.

~~~sh
/path/to/toolchain/bin/lkjscript runtime-ir \
  --runtime-source library/runtime.rs -o runtime.lkjir
~~~

Runtime preparation compiles the selected source without a library, using the
logical root $runtime/runtime.rs. It produces ordinary checked IR. The backend
checks and seals that runtime when emitting an artifact. --check validates
program IR; runtime sealing happens during emission.

## Install explicit native artifacts

The installer takes every compiler/library/runtime input explicitly:

~~~sh
bash tools/install-native.sh /work/toolchains/first \
  --frontend /work/artifacts/frontend.elf \
  --backend /work/artifacts/backend.elf \
  --result /work/artifacts/result.elf \
  --library library --runtime-source library/runtime.rs \
  --source-commit COMMIT_OR_DESCRIPTIVE_LABEL
~~~

The destination must not exist. Installation copies independent regular files
into a private sibling directory, rejects symlinks and special nodes in the
library tree, and prepares runtime IR with the supplied frontend. It then checks
composition by compiling and executing a plain probe and a packet-enabled probe.
The latter must be decoded by the supplied result reader as exactly 42 and a
newline. Only a successful composition is renamed into place.

The installation contains bin/lkjscript, libexec/frontend, libexec/backend, libexec/result,
libexec/native-process.sh, library/, runtime.rs, runtime.lkjir, manifest and SHA256SUMS. The manifest identifies
declared inputs and their hashes, the supplied source label, generated runtime
and composition checks. SHA256SUMS covers all installed files except itself;
lkjscript verify detects later changes. These hashes record local artifact identity,
not an external signature.

## Rebuild from an installed seed

~~~sh
bash tools/rebuild-native.sh /work/toolchains/next \
  --seed /work/toolchains/first --source-dir /work/lkjscript \
  --source-commit COMMIT_OR_DESCRIPTIVE_LABEL
~~~

The caller keeps the source checkout consistent. The script prepares the
checkout's runtime with the explicit seed, builds frontend_main.rs, backend_main.rs
and result_main.rs with the checkout's library/runtime, and calls that checkout's
installer. It builds one generation and inherits process controls. Commands, intermediate IR hashes and
optional GNU time/peak-RSS measurements go to stderr. The destination is
published only after installation checks pass; private intermediate files are
removed on success or failure.

A fixed-point check rebuilds from the same source, library and runtime and
compares libexec/frontend, libexec/backend, libexec/result and runtime.lkjir
byte for byte. Current results are in status.md.

## Results and process controls

The backend's optional --result-descriptor PATH writes metadata from the
same lowering that emits a packet-enabled ELF. Metadata is written completely
before any ELF bytes reach stdout. Default standalone emission remains unchanged.

Eval gives fd3 to the target for its 24-byte native result packet in a private
file. After the target exits, the shell appends its observed process status; the
native result reader checks the packet, descriptor, framing and status agreement.
Program stdout bypasses that channel, and closed descriptors stay closed.
Missing or inconsistent packets are E_HOST failures. Custom IR trap text travels
in the descriptor. The current source panic form lowers to the generic message
"explicit panic"; this wrapper does not change that source behavior.

LKJSCRIPT_RUN_MEMORY_BYTES and LKJSCRIPT_RUN_STACK_BYTES optionally tighten only the run/eval
target's Linux soft limits, preserving inherited hard limits. Unset or unlimited
inherits existing controls, and a requested value cannot raise a tighter
inherited soft limit. The compiler and result reader keep the caller's controls.
Standalone ELFs read Linux limits directly. See limits.md for startup accounting.

TERM and INT sent to a launcher cancel its tracked work and preserve the usual
143/130 status. Compilation and rebuild stages use owned process groups so
wrappers such as GNU time cannot leave a compiler behind. Application targets
keep the caller's terminal group and inherited streams. The shared process helper
restores asynchronous children's INT/QUIT dispositions with Bash builtins; it
introduces no execution deadline. Direct /bin/bash entry points preserve closed
standard descriptors even on hosts whose env utility reopens them. Publication
to stdout checks that descriptor before copying bytes. Relative TMPDIR values are supported, and filesystem input
paths retain the kernel meaning of symlinks and parent components.

## Development checks

~~~sh
bash tests/native_cli.sh tools/lkjscript
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

Rust comparisons, Cargo unit tests, the independent IR interpreter and Node test
orchestration remain development/reference tools. Their continued use does not
supply hidden inputs or fallbacks to the installed compiler.
