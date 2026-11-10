# Distribution

Automated release seals, receipts, paired implementation trees and the release
controller are retired. Development builds use the pinned Rust toolchain and
locked Cargo dependencies. `docs/spec/verification.md` owns the routine and slow
checks. Publishing a release or changing a running deployment requires an explicit
owner request; ordinary development pushes source only.

Existing runtime installation behavior is described in
`spec/product-surface.md`. Distribution is currently outside the language work.
