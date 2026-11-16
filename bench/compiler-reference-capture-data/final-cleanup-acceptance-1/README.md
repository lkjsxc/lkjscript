# Final finite cleanup: independent acceptance

Accepted once-only root1600/final1602 against controls at 6114aa74926b4257c8418e6fc4b915348c6100cb and recovery map at 9326dbaab1bf6832b9f35dc156e12d0d62eccd9e.

Verified all 9,367 progress records: begin, complete preflight, 4,532 exact file intent/completion pairs, 145 directory intent/completion pairs, ten root-absence events, and completion. Every intended file matches committed path, byte count and digest. Preflight precedes the first mutation; no pending intent remains. All ten roots and nine literal files are currently ENOENT.

Canonical remains b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db (32,588,975 bytes), matching both captured identities. Actual outer/guard argv, control pins, zero wrapper/guard/GNU exits and empty stderr agree.

Cleanup: wall 7.04 s, CPU 6.93 s (user 4.53 / system 2.40), peak RSS 349436 KiB; load 12.21/6.40/3.39 -> 11.47/6.34/3.38. This is cleanup timing, not compiler performance.

This reads retained metadata/logs and current canonical bytes. It does not reconstruct deleted payloads, rerun recovery checks, execute the cleanup guard, decompress an archive, run a compiler, inspect credentials, or use Python/process/FD observers. Full-byte preflight is established through the exact reviewed guard and its successful event, not through a second read of deleted originals. Cleanup is cooperative and can stop partially, not an atomic multi-root transaction. No failure was injected; success left no residual intent.

acceptance.json records exact pins/counts/order/absences/timing; inputs.json records audit input hashes. The audit and its sole execution capture are retained. No original evidence changed. One earlier tool-publication attempt failed in JavaScript parsing before any remote write or audit execution; that failure remains in the tool transcript.
