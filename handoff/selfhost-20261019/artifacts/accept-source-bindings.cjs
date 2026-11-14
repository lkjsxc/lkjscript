const fs = require('fs'), cp = require('child_process'), crypto = require('crypto');
const prefix = process.argv[2];
if (!/^source-accept-v[0-9]+-$/.test(prefix || '')) throw Error('Owned acceptance prefix required');
const directory = '.artifacts/selfhost/';
const read = path => JSON.parse(fs.readFileSync(path, 'utf8'));
const git = (...arguments_) => cp.execFileSync('git', arguments_, {encoding:'utf8'}).trim();
const file = path => {
  const bytes = fs.readFileSync(path);
  return {path, bytes:bytes.length, sha256:crypto.createHash('sha256').update(bytes).digest('hex')};
};
const source = git('rev-parse','HEAD'), tree = git('rev-parse','HEAD^{tree}');
if (git('status','--porcelain')) throw Error('Acceptance requires a clean committed tree');
const receipts = Object.fromEntries(['bootstrap','changed','full','release-target','harnesses'].map(name => [name,read(directory+prefix+name+'.json')]));
const {bootstrap,changed,full,harnesses} = receipts;
if (bootstrap.source.source !== source || bootstrap.source.tree !== tree || bootstrap.status !== 'passed' || receipts['release-target'].status !== 'passed') throw Error('Source or release policy mismatch');
for (const receipt of [changed,full]) {
  if (receipt.git_head !== source || receipt.status !== 'passed' || !receipt.input_stable || receipt.unrun_gates.length || !receipt.gates.length || receipt.gates.some(gate => gate.status !== 'passed' || gate.execution !== 'fresh')) throw Error('Incomplete official acceptance');
}
if (changed.worktree_input_digest !== full.worktree_input_digest || full.worktree_input_digest !== full.final_worktree_input_digest) throw Error('Acceptance snapshot changed');
if (harnesses.source !== source || harnesses.harnesses.length !== 8 || harnesses.harnesses.some(harness => !harness.terminal_result.startsWith('test result: ok.'))) throw Error('Incomplete workspace harness matrix');
const library = harnesses.harnesses.find(harness => harness.selfhost.length > 0);
if (!library || library.selfhost.length !== 48 || library.native.length !== 10 || [...library.selfhost,...library.native].some(line => !line.endsWith(' ... ok'))) throw Error('Compiler or native matrix incomplete');
const development = read(directory+'source-binding-development.json');
const target = directory+'accepted-e-source-bindings';
fs.mkdirSync(target,{recursive:false});
for (const [name,receipt] of Object.entries(receipts)) fs.writeFileSync(target+'/'+name+'.json',JSON.stringify(receipt,null,2)+'\n');
const result = {
  schema:'lkjscript.selfhost.slice.v1', status:'passed',
  milestone:'E logical source identities and exact canonical object bindings; graph semantic admission and normalization remain incomplete',
  tier:1, source, tree, parent:git('rev-parse','HEAD^'),
  authoritative_revision:development.current_authoritative_revision,
  public_authoring:development.meaning_changes,
  source_files:development.files.map(value => file(value.path)),
  functions_added:18, observations:development.observations,
  admission_storage:read(directory+'source-binding-admission-comparison.json'),
  validation:{library:library.terminal_result,selfhost_tests:library.selfhost.length,native_runtime_tests:library.native.length,harnesses:harnesses.harnesses.map(harness => ({file:harness.file,terminal:harness.terminal_result})),changed_gates:changed.gates.length,full_gates:full.gates.length,fresh:full.fresh_passed_gates,reused:full.reused_passed_gates,input_digest:full.worktree_input_digest,release_target:'passed'},
  receipts:Object.keys(receipts).map(name => file(target+'/'+name+'.json')),
  evidence_files:['source-binding-tests-v9-observations.log','source-binding-associated-v9.log','source-binding-decoder-v10.log','source-binding-replay-v10.log','source-binding-clippy-v10.log','source-binding-stage-observations.log'].map(name => file(directory+name)),
  limits:['Current compiler entrypoints still consume Rust-admitted Tier 1 input.','Logical commitments and canonical record bindings do not establish reference existence, ownership, cycles, type correctness, effects or witnesses.','No phase-local Region reclamation or RSS claim.','Tier 2 source-graph normalization and native C2/C3 plus artifact A2/A3 fixed points remain required.'],
  hygiene:'Official changed, fresh full and release target observed this exact clean committed tree; no tracked edits followed acceptance.',
  publication:'Accepted locally; normal remote integration pending'
};
fs.writeFileSync(target+'/receipt.json',JSON.stringify(result,null,2)+'\n');
process.stdout.write(JSON.stringify({source,tree,status:result.status,validation:result.validation})+'\n');
