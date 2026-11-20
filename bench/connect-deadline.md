# Deadline-aware TCP connection

`Connector.connect_deadline(deadline: usize) -> Result<File, i64>` consumes an existing exact numeric IPv4 endpoint capability. The deadline is an absolute `World.monotonic()` nanosecond value. A nonblocking close-on-exec socket waits for readiness with the remaining deadline and checks `SO_ERROR`; a readiness notification alone is not success. Interrupted waits recompute the budget. Expired deadlines return `ETIMEDOUT` without opening a socket. Every unsuccessful attempt closes its socket. Successful owned Files have blocking mode restored, matching `Connector.connect()`.

Native lowering, the independent interpreter broker and the Rust comparison bridge implement this contract separately. The checked intrinsic requires the exact Connector operand and usize deadline; authority analysis and unit runtime discovery include it. No new entry grant or broader network authority is introduced.

## Executable evidence

Run `bash tools/connect-deadline-test.sh target/lkjscript`. The portable local peer exercises successful readiness and a delayed ordinary read, 96 refused connections, a saturated loopback backlog with 96 bounded timeouts under a 64-descriptor limit, past and maximum deadlines, and closed standard descriptors. The script also checks forged IR, moved/shared authority and cold/warm unit linking. The browser's original deadline probe checks unchanged with this producer.

Six focused cases pass separately on native and interpreter routes. The independent Rust comparison passes one output case and three rejecting verdicts. The permanent full gate and witness gate both run the dedicated script.

## Producer and complete gate

Native SHA-256: `333999758b9b90c74e699615a40ba5c6f976c9f63252a90e5d90178560371ef9`.

Seed gzip SHA-256: `e15e5b7864924ec6091924a6ae3159ade387e826d71df0b860f4a43948f542a5`.

The refreshed-seed bootstrap reproduces the native compiler exactly. The four-worker full gate passes in 993.857 seconds, rerunning all 6,187 case requests, all host suites and 128 IR mutants with zero findings. The standalone real-program test passes the original algorithm, its eight tests on both engines, 18 provenance controls and 7,239 native corpus rows. Producer and seed hashes are unchanged after both commands.

The twelve fresh original-source observations are byte-identical to the previous frontier: nom 17, DOM 63, Sōzu 448 classified plus 34 prior-unclassified rows. This runtime capability does not remove a source-form barrier or admit a large root.

[Complete gate](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/connect-deadline-data/full-gate.log), [standalone real programs](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/connect-deadline-data/standalone-real-programs.log), [fresh census comparison](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/connect-deadline-data/census-comparison.json), and [producer identity](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/connect-deadline-data/producer-and-seed.sha256) retain the local proof. The exact published revision's CI result is retained in the consolidated session report.
