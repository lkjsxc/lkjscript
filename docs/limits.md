# Current execution implementation

The current backend and Rust host use the representations below.
Their direct-native replacement is milestone 6.1 in [direction](direction.md).

| Area | Implementation |
| --- | --- |
| Values | Indices, extents and addresses use 64-bit words. Integer operations validate their declared width. |
| Native artifact | Static Linux x86-64 ELF, without libc or a dynamic loader. |
| Native control | The current encoder uses absolute branch/call targets. The replacement uses rel32 with located encoding errors. |
| Native memory | The current startup reads physical memory, RLIMIT_AS and RLIMIT_STACK, then reserves a linear heap and a stack. |
| Allocation | Power-of-two payload classes, minimum 16 bytes, with a separate 16-byte header. Freed blocks are reused by class; tip blocks grow in place; other growth copies then releases. |
| Views | Sized references are one word. Slice/text references are address/count pairs of 16 bytes. |
| Interpreter | A separate typed heap, initialization state and loan identities; explicit call/control/drop stacks. The Rust host supplies its execution extents. |
| Failures | Stack exhaustion reports R0008; checked heap refusal reports R0006. Infallible Rust allocation failure is a host failure. |

The current runners accept LKJSCRIPT_RUN_MEMORY_BYTES and
LKJSCRIPT_RUN_STACK_BYTES to tighten inherited process controls.
The test tools also retain LKJSCRIPT_TEST_MEMORY_BYTES.
These settings and startup extent arithmetic are scheduled for deletion with
the direct-native implementation.

## File and stream boundary

World supplies arguments as owned byte vectors and derives stream and directory
capabilities. Directory opens use Linux openat2 with RESOLVE_BENEATH and
RESOLVE_NO_MAGICLINKS. Kernels without that operation return ENOSYS.
Source Results carry positive Linux errno values; raw read/write leaves use
negative errno.

Native artifacts preserve inherited descriptor state. The Rust adapter restores
closed standard descriptors after Rust startup. Program output, requested exit
status and trap control remain separate.
`node tools/world-io.mjs` compares bytes, status, file effects, directory escapes,
handle cleanup and closed/broken streams across execution routes.
