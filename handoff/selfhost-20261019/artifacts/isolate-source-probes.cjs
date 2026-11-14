const fs=require('fs');
const path='src/platform/execution/normalized/selfhost_source_tests.rs';
let s=fs.readFileSync(path,'utf8');const begin=s.indexOf('fn probes()'),end=s.indexOf('\nfn function(',begin);if(begin<0||end<0)throw Error('probe block missing');
const replacement=`fn probes(module: &str) -> &'static (KernelSnapshot, NormalizedProgram) {
    static PROBES: [OnceLock<(KernelSnapshot, NormalizedProgram)>; 6] =
        [const { OnceLock::new() }; 6];
    let (index, proposal) = match module {
        "source-probe" => (0, include_str!("../../../../tests/fixtures/selfhost-source-probes.lkjc")),
        "source-record-probe" => (1, include_str!("../../../../tests/fixtures/selfhost-source-record-probes.lkjc")),
        "source-map-probe" => (2, include_str!("../../../../tests/fixtures/selfhost-source-map-probes.lkjc")),
        "source-state-probe" => (3, include_str!("../../../../tests/fixtures/selfhost-source-state-probes.lkjc")),
        "source-history-probe" => (4, include_str!("../../../../tests/fixtures/selfhost-source-history-probes.lkjc")),
        "source-object-probe" => (5, include_str!("../../../../tests/fixtures/selfhost-source-object-probes.lkjc")),
        _ => return compiler(),
    };
    // Each independent probe graph retains the entire compiler source. Unrelated
    // test entrypoints do not need to share one finite-admission transaction history.
    PROBES[index].get_or_init(|| {
        let changes = proposals::updates(proposal).unwrap();
        let source = apply_proposals(&compiler().0, changes);
        let program = prepare_snapshot(&source);
        (source, program)
    })
}
`;
s=s.slice(0,begin)+replacement+s.slice(end);s=s.replaceAll('= probes();','= probes(module);');fs.writeFileSync(path,s);
