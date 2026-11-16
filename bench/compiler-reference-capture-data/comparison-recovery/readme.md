# Lossless comparison-2 recovery

This archive covers only the completed comparison-2 and its comparison-launch-2 outer capture.
Every ordinary record/control/summary/capture file is stored separately under files/comparison or files/outer.
The fourteen output paths use two stored representatives under families; the inventory retains each original path, mode, size and digest.
A/B executables and producer-B.gz use the exact committed Git seed recipes, whose complete compressed/expanded bytes were checked against the originals.

Recovery is to a new destination, not over either original root:
1. Verify the archive identity and member inventory from the surrounding package.
2. Recreate the thirty-two original directories and their recorded modes.
3. Copy every archive-backed inventory entry to its recorded original relative path; copy aliases independently, then restore each recorded mode.
4. For a git-blob entry, obtain the exact committed seed blob and verify it. For gunzip-git-blob, additionally decompress and verify the full expanded bytes.
5. Verify all 501 recovered regular file sizes/digests and modes against recovery/inventory.json.

No source-preparation tree, benchmark input tree, key/store, mutable canonical compiler or unrelated scope is included.
Filesystem inode allocation, ownership and incidental timestamps are not claimed to be recreated; the requested bytes, file modes and directory layout/modes are complete.
The original roots remain untouched. This package authorizes no cleanup, workload rerun or automatic restoration.
