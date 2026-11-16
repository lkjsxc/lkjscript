# Narrow preflight repair proposal

Frozen launcher: 47d085bc0cf1b0a48be56fcfda8efcec5cba85162dcdf9031f5da5ba7e6140e6.
Exact extracted preflight: ed1514cb13defccdf1e6fd16cec963152c69d316d6ad3a01b1a8ba962fad9425.
Original cohort remains consumed; none of its files is changed.

In rel(), the last positive regex match receives list context from die_if's
unprototyped argument list through the short-circuit expression. For a valid
relative path the failed match returns (), not a scalar false argument.
die_if therefore receives only "unsafe relative path": it treats that message
as a true condition, then attempts to print its absent second argument.

The instrumented extraction records arity 1 and the call stack at precisely
this valid release-path check. This is a launcher condition/context defect,
not evidence that the release path or producer pins were wrong.

Proposed behavior-preserving repair:

    sub die_if {
        die "E_COPY_RELEASE_GUARD expected condition and message\n"
            unless @_ == 2 && defined($_[1]);
        die "$_[1]\n" if $_[0];
    }

    die_if(scalar(!defined($p) || ref($p) ||
        $p!~m{\A[A-Za-z0-9._/-]+\z} ||
        $p=~m{(?:\A|/)\.\.?(?:/|\z)} || $p=~m{//} || $p=~m{\A/}),
        "unsafe relative path");

The scalar wrapper restores the missing false argument. The arity check makes
a future caller error explicit; it does not relax a release check. A two-scalar
prototype would also force context, but this proposal keeps the existing
calling convention and makes the affected expression explicit.

Focused validation should precede any separately released cohort:

1. Retain the original instrumented failure and the pure argument-context
   control, including valid relative paths and unchanged invalid-path decisions.
2. Compile all embedded Perl blocks and Bash syntax.
3. Run only the corrected extracted preflight against the exact consumed
   release/source evidence, writing every output outside that cohort. Its
   source remains an untracked diagnostic copy; no launcher action follows.
4. Validate a new final release read-only before authorizing a new invocation;
   preserve negative controls for invalid path/field/pin rather than weakening
   the checks to fit the old release. A new cohort needs a new prospective
   record and explicit root release.

No compiler, measured image, profile reader or accounting helper is involved.
