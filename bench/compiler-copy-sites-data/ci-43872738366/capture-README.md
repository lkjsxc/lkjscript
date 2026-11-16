# CI 43872738366 retained evidence

Read-only capture for root review, completed 2026-11-16. Run attempt 1 at
f9e4581ae815ba041510d31421e6b588d66b4059 and both jobs completed successfully:
witness 133317833494, gate 133317833855. Status polling ended at jobs-14.json;
run-final.json is the one subsequent final run response.

The complete downloaded logs are gate-log-1.log and witness-log-1.log.
Each CLI request retains its raw body, stderr, exit status, command and UTC
capture endpoints. All 20 recorded gh command captures exited zero. The
initial text inspection attempted an unavailable optional file utility;
inspection-1.gateway.json preserves that exact warning and surrounding output.
No request or historical result was overwritten.

summary.json derives the gate total, selected suite timings and their logged
loads, 5,448 rerun requests/zero skips (also independently summed from 12 case
rows), and witness 276 values/output cases/344 verdicts and byte-identity success.
Native gate loads are the one-minute averages printed by the pinned wrapper.
Witness auxiliary suite lines preserve all three load averages.

The compact native log records successful mutation-suite completion, but omits
the detailed mutant/category counter row. The pinned check.sh requests seed
1701, count 128 and four workers; test_generators.lkj returns success exactly
when its findings count is zero. The summary labels that source-contract basis
explicitly. Individual mutant outcomes cannot be reconstructed from this CI
log and remain outside this retained CI evidence.

bootstrap.sh and check.yml are exact Git files at the checked revision, as are
check.sh and test_generators.lkj. The successful bootstrap recipe compares
consecutive current-emitter generations and checks the source library. Its
compact gate row does not print the resulting image digest.

seed-uncompressed-identity.json is a separate data-only decompression/hash of
the Git seed at the checked revision: 32,584,542 bytes,
6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc.
The compressed seed is 6,547,345 bytes,
505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2.
These are independently bound Git-seed identities, not hashes printed by CI.

No compiler, generator, profile, benchmark or test was executed locally.
No CI mutation, push or shared-document edit occurred. Owned captures remain
for root acceptance; no cleanup was performed.
