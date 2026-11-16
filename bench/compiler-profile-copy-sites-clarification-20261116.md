# Copy-site publication clarification — 2026-11-16

This dated clarification applies to the publication paragraph of
[the prospective copy-site plan](compiler-profile-copy-sites-plan.md),
whose original SHA-256 remains
`b9de2760c582d33eedea3060a59f0502263335ca0667e597c34d4082bc8d9a52`.
The original plan stays unchanged.

Its sentence “A partial publication therefore cannot be accepted as a
corresponding image/IR pair” must be read with the following narrower contract.

The driver retains both output parents, rejects the same parent identity plus
basename, stages both contents, and publishes the companion before the image.
It reports success and publishes a new image memo only after both renames
succeed. A partial publication returns nonzero and publishes no new image memo;
it may leave one destination replaced. This is not a two-file atomic transaction
and does not promise rollback of both paths.

The profile's SHA-256 binds the exact selected-IR companion bytes. The detail
reader refuses missing or different bytes and independently checks the supplied
IR before validating coordinates and layout. Identical companion bytes retained
from an earlier build still satisfy correspondence. Digest agreement does not
prove freshness or a common publication event, and introduces no generation
nonce.

The clarification changes no admission boundary, stored program identity,
sampling scope or optimization claim. The normative user-facing wording belongs
in [the IR profile contract](../docs/ir.md#native-cpu-profiles-and-selected-ir).
