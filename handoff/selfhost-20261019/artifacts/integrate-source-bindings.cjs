// Run only after the preceding exact candidate has terminal acceptance.
const fs=require('fs');
const root='.artifacts/selfhost/';
for(const name of ['map','state','history','object']) {
 fs.copyFileSync(root+`source-${name}-draft.lkjc`,`tools/selfhost/source-${name}.lkjc`);
 fs.copyFileSync(root+`source-${name}-probe-draft.lkjc`,`tests/fixtures/selfhost-source-${name}-probes.lkjc`);
}
const proposal='tests/support/selfhost_proposals.rs';
let s=fs.readFileSync(proposal,'utf8');
const anchor='    include_str!("../../tools/selfhost/source-owner.lkjc"),\n    "\\n",\n';
if(!s.includes(anchor))throw Error('proposal anchor');
s=s.replace(anchor,anchor+['map','state','history','object'].map(n=>`    include_str!("../../tools/selfhost/source-${n}.lkjc"),\n    "\\n",\n`).join(''));fs.writeFileSync(proposal,s);
const path='src/platform/execution/normalized/selfhost_source_tests.rs';
s=fs.readFileSync(path,'utf8');
const probeAnchor='            include_str!("../../../../tests/fixtures/selfhost-source-record-probes.lkjc"),\n';
if(!s.includes(probeAnchor))throw Error('probe anchor');
s=s.replace(probeAnchor,probeAnchor+['map','state','history','object'].map(n=>`            "\\n",\n            include_str!("../../../../tests/fixtures/selfhost-source-${n}-probes.lkjc"),\n`).join(''));
const old='    let module = if matches!(name, "read-owner" | "read-records") {\n        "source-record-probe"\n    } else {\n        "source-probe"\n    };';
if(!s.includes(old))throw Error('module selector anchor');
s=s.replace(old,`    let module = match name {
        "read-owner" | "read-records" => "source-record-probe",
        "read-map" => "source-map-probe",
        "read-state" => "source-state-probe",
        "read-revision" | "read-dependency" | "read-retirement" => "source-history-probe",
        "bind-type" | "bind-owner" | "bind-dependency" | "bind-retirement" | "read-bindings" => "source-object-probe",
        _ => "source-probe",
    };`);
s+='\n#[path = "selfhost_source_binding_tests.rs"]\nmod binding;\n';fs.writeFileSync(path,s);
const body=['map','state','history','object'].map(n=>fs.readFileSync(root+`source-${n}-tests-draft.rs`,'utf8').replace(/^\/\/ Proposed[^\n]*\n/gm,'')).join('\n');
fs.writeFileSync('src/platform/execution/normalized/selfhost_source_binding_tests.rs','//! Logical source commitments, canonical history, and exact object bindings.\n//! Matching digests do not establish graph semantics or execution permission.\nuse super::*;\n\n'+body);
const verify='tools/selfhost/verify.sh';s=fs.readFileSync(verify,'utf8');
s=s.replace('  src/platform/execution/normalized/selfhost_source_tests.rs \\\n','  src/platform/execution/normalized/selfhost_source_tests.rs \\\n  src/platform/execution/normalized/selfhost_source_binding_tests.rs \\\n');
s=s.replace('  tests/fixtures/selfhost-source-nominal.lkjc \\\n',['map','state','history','object'].map(n=>`  tests/fixtures/selfhost-source-${n}-probes.lkjc \\\n`).join('')+'  tests/fixtures/selfhost-source-nominal.lkjc \\\n');fs.writeFileSync(verify,s);
