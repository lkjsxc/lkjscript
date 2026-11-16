The first source-commit wrapper stopped before creating a commit because the
whole staged whitespace check inspected immutable raw diff/inspection captures.
Their space-prefixed blank context lines and truncated inspection output are
part of the exact captured evidence. No product source violation was reported.
The original output is retained unchanged. The publication check now applies to
the actual source, tools, documentation and authored report; raw evidence bytes
are preserved, not reformatted. No compiler, test or measured cohort was rerun.
