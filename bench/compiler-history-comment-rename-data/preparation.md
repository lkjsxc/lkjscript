# Static evidence preparation

The first attempt to turn the previously completed scratch screen into this
durable inventory accidentally escaped Git pathspec parentheses. Its assertion
reported 0/0/0 and exited 255 before writing an inventory or shortlist. The
exact preparation script and stderr are retained as screen.prepare-1.*.
The corrected screen uses Git's literal :(glob) / :(exclude) pathspecs, asserts
the same frozen population hash and completes with 204 language diffs, two
shortlisted comment-only revisions and zero pure content-preserving language
pathname-renames. This is static Git evidence preparation, not a compiler run,
semantic measurement, replacement historical case or new equivalence result.
