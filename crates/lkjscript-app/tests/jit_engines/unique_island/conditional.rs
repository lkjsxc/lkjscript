use super::*;

#[test]
fn conditional_cleanup_executes_in_both_generated_tiers_without_fallback() {
    assert_i64_all_engines(SOURCE, "native-conditional-cleanup.lkjscript", 3, 4, true);
}

const SOURCE: &str = concat!(
    "def/\nname/\nselect-bytes\n/name\npublic\nfn/\n",
    "sig/\ninputs/\nbool\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
    "params/\nflag\nbool\n/params\nlet/\nbind/\nb\nnew-byte-vector/\n1\n",
    "/new-byte-vector\n/bind\nlet/\nbind/\nc\nnew-byte-vector/\n2\n/new-byte-vector\n",
    "/bind\nif/\nflag\nmove/\nb\n/move\nmove/\nc\n/move\n/if\n/let\n/let\n",
    "/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
    "let/\nbind/\na\nselect-bytes/\ntrue\n/select-bytes\n/bind\n",
    "let/\nbind/\nb\nselect-bytes/\nfalse\n/select-bytes\n/bind\nadd/\n",
    "byte-slice-length/\nborrow/\na\n/borrow\n/byte-slice-length\n",
    "byte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/add\n/let\n/let\n/main\n",
);
