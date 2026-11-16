# Separate cleanup guard V2 proposal

This is a preparation artifact. It does not release a cleanup invocation. The first attempt remains closed at Perl compilation exit 2 with zero guard cleanup actions; its original guard, bundle, and capture remain unchanged.

## Candidate

- Local candidate: `guard-v2.pl`
- Proposed committed path: `bench/compiler-copy-sites-root-cleanup-guard-v2.pl`
- Candidate bytes: **16,570**
- Candidate SHA-256: **55884264496cbe04ff872883128e65400092c42b32e84b6785a961a5c8197c41**
- Original guard SHA-256: `3001e0e524c6694e56c1ad27cdc162a381493782670ce3a804c92522dbded5e7`

The retained unified diff changes exactly nine existing lines and adds/removes no lines:

1. Seven `}if$class` token boundaries become `} if $class`.
2. The owned result path changes from `cleanup-results` to `cleanup-results-v2`.
3. The self-byte check reads the separately committed V2 path through the supplied bundle revision.

`minimal-change-proof.json` confirms that reversing only those nine replacements reconstructs every original byte. All remaining scope, inventory, retention, recovery, disposal, metadata, hash, unlink, rmdir and ENOENT checks are byte-identical.

## Argument and self-pin review

The required two arguments remain `--execute-after-root-release` plus one exact 40-character revision. The supplied revision must still be in main history.

The manifest prefix stays `bench/compiler-copy-sites-root-cleanup-data`. The original manifest loop, all required original records, and the verification of every original manifest entry are unchanged. In particular, the original `guard.pl` remains one of those authenticated records.

The executing V2 file is additionally compared against the Git blob at:

```text
SUPPLIED_REVISION:bench/compiler-copy-sites-root-cleanup-guard-v2.pl
```

A future release must name the new commit containing that separate blob and the unchanged original 39-entry manifest. The original bundle revision alone does not provide the new self-pin. No runtime preflight was attempted with either revision.

## Syntax-only evidence

The sole check was `perl -c`, captured under the recorded 4 GiB / 300 s / 10 s-grace wrapper. It passed with exit 0 and empty stdout. Stderr contains only the candidate path followed by `syntax OK`.

- UTC: **2026-11-16T15:00:25Z → 2026-11-16T15:00:25Z**
- GNU time: **0.02 s wall, 0.01 s user, 0.00 s system, 13,976 KiB peak RSS**
- Load: **2.80 / 3.33 / 2.53 → 2.80 / 3.33 / 2.53**
- Request: `compiler-copy-sites-cleanup-v2-syntax-20261116-143`

This is syntax validation, not a cleanup or runtime preflight test. The guard's ordinary runtime body was not executed. No deletion, observer, private-key read, compiler/image execution, or cleanup retry occurred.

`syntax-proof.json` records the full argv/cwd, raw capture identities, and fresh hashes of the unchanged original guard, original bundle manifest and failed-attempt manifest. The raw capture files are retained alongside it.

The bounded diff/argument/self-pin inspection found no additional concrete issue in this minimal change. Root review, commitment of the separate V2 path and evidence, and a distinct explicit execution release remain outstanding.
