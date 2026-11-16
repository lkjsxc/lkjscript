# Fixed specimen setup

Root reviewed the final setup shell, helper, fixed inputs, syntax checks and
the 22-line amendment that records all five phase outcomes. The independent
review found no blocker. Execute the once-only five-phase setup using controls
manifest edb63ca8c50e2659c12e057225f3adb7a774aa3fa9091ee277e872770b4d2d58.
This permits the two stated generator commands, each with a 4 GiB address
limit, 900 s watchdog, 10 s termination grace and a 3 GiB free-space check.
It does not run or release an A/B measurement, a larger rung or a retry.

Source bd8a9050a3786de9f227c6c4037f5acd43205f71 passed the captured cold
gate. Seed-only commit e5294c339d9aefbbafc2e951c997f4fefabcc9ad
decompresses byte-for-byte to that fixed-point producer. Candidate CI has
not yet been triggered. Baseline CI is not candidate evidence.

The setup must preserve its first failure and stop. Successful inputs and
their verified archives must be committed before any separate measurement.
Text remains the repository projection; no owner decision is changed.

A root JavaScript template parse error occurred before submitting gateway
request v1506; it performed no copy, commit or workload. This v1507 invocation
is the first submission of the reviewed retention step.
