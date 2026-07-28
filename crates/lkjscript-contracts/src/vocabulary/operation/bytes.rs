use super::{record, OperationCategory as C, OperationVocabularyRecord as R};

pub(super) const RECORDS: &[R] = &[
    record(
        123,
        "bytes-length",
        "bytes-length",
        C::ByteData,
        "read immutable byte length",
    ),
    record(
        124,
        "bytes-byte-at",
        "bytes-byte-at",
        C::ByteData,
        "read one immutable byte",
    ),
    record(
        125,
        "copy-bytes-slice",
        "copy-bytes-slice",
        C::ByteData,
        "copy checked immutable byte range",
    ),
    record(
        126,
        "clone-bytes",
        "clone-bytes",
        C::ByteData,
        "clone immutable bytes",
    ),
    record(
        127,
        "freeze-byte-vector",
        "freeze-byte-vector",
        C::ByteData,
        "consume byte-vector into immutable bytes",
    ),
    record(
        128,
        "thaw-bytes",
        "thaw-bytes",
        C::ByteData,
        "consume or copy immutable bytes into byte-vector",
    ),
];
