# Static eligibility of the fixed numeric edits

This is source inspection and proposed-byte calculation only. No parser/compiler/native command, archive decompression/extraction, or fingerprint reimplementation was run. The initial String edit is retained under v1-rejected-string-edit and is not an executable cohort.

## Exact compiler input change

At Git c887b7c5aca7bb80d70ca26d7dfdf1a44213a163, src/driver.lkj is 120,359 bytes, SHA 333d6ec927416917fa32269730a9a507fcfe31072a2191403a24161b5cf6c412, blob 2de543fa7f32ab3d4d43d8aa9b5302ecb5cdde0f. driver::inline_limit is a nongeneric module-level free function with explicit Result<usize,String> return type. Its body is [18240,18817), 577 bytes. Byte 18378 changes ASCII 0 to 1 only, in `let mut value = 0usize;`. The full after-file SHA is 7acceb488b8a47a328ce269d880134eadfa7b92dee788f7b8ac6751e60ea33c3. The complete before/after bodies are separate text files; their SHAs are 92e13f4d03d042b2e821a32aa04374696337fba6d5c0bb2d7e53a456da6cb2b5 and c4aea53244a222793e35f79516d94e8c8c28449110965c25412ecc39353a87bb.

This changes the decimal parser's initial accumulator in the compiled specimen (for example, its interpretation of a supplied decimal argument). It does not alter A or B, which remain independently pinned executables; the 36 builds always receive --inline-limit 0. No produced specimen image is executed.

## Fingerprint and resume rules

The baseline's src/elaborate/incremental.lkj at e5294c339d9aefbbafc2e951c997f4fefabcc9ad hashes to 760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127. The retained excerpts show that function body bytes are separated from topology, while body coordinates, arena counts, node spans/scopes, type-node raw spelling, and String/Bytes payloads still participate. Other expression payloads use tag 0. Thus the original usage String edit changes topology and is unsuitable. Both revised edits alter a one-byte integer-expression payload with an unchanged suffix, node/span count and all type spelling. There is no new sign, path, include, attribute, signature, local item, or closure.

resume requires identical topology/body-vector length; every changed definition must be a free function without non-lifetime generics or an enclosed Closure expression. It still performs inference, demand validation, exact function skeleton comparison, catalog/types/destructor equality and scoped checking. driver::inline_limit has explicit parameters/return type, no generics, no closure and no local declaration item; its numeric initializer remains usize. edit::target similarly remains a nongeneric free function with explicit u64 argument/return and 1u64→2u64. No proposed change creates a type or destructor. These are static eligibility arguments; actual front_reused>0 and a changed emitted image remain mandatory observations. A refusal/miss must stop the fixed cohort; there is no fallback edit.

The reviewed B revision 31f412a0 changes fingerprint lookups to indexed per-file input/function lookup. The source diff retains the expression-payload rules and resume contract; this proposal does not independently recalculate fingerprints or infer a performance result from that change.

## Include reach and demanded source

The exact fixed Git src/library text scan covered 13 files containing include_str/include_bytes and found 82 literal-call-shaped occurrences, none resolving to src/driver.lkj. This is a deliberately overinclusive textual inventory, not a new parser. Eleven unmatched call-shaped occurrences are all in src/semantic_tests/source_contracts.lkj and were inspected in context: they are escaped source strings or deliberately malformed fixture strings, not calls embedding driver.lkj. The compiler root src/main.lkj does not load semantic_tests. The source loader itself accepts only a single Expr::String argument for these macros (the include-expansion branch of src/source.lkj); nonliteral paths do not form an alternate include route. The actual compiler embedded-source literals target library files, which are unchanged.

The fixed compiler root declares driver and calls driver::main. The retained source occurrences show inline_limit is called by the CLI options parser for depth/limit/bytes/inline-limit/store-bytes. This establishes the intended demanded function from source, not an observed IR instance count. The generated edit is the existing demanded edit::target scale specimen; its before bytes match the retained archive manifest and generator literal.

## Preparation amendment and provenance

The ten original proposal files and static refusal are retained under v1-rejected-string-edit; its manifest SHA is 2725aada954aeee57741cbfc356bd9b1486b9df5f536136f64ba88430e7a46cd. The source population, row order, limits, 2GiB row-store budget, equality checks and stop policy are unchanged. Only the prospective compiler body coordinate and now-reported B provenance change. Read-only gateway requests 348–351 are named in read-provenance.json. numeric-eligibility.read.json retains exact byte calculations, literal include inventory and unresolved fixture contexts.
