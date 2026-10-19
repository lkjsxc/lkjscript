const fs=require('fs'),cp=require('child_process'),crypto=require('crypto');
const dir='.artifacts/selfhost/',prefix=dir+'source-accept-v2-';
const read=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const source=cp.execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
const tree=cp.execFileSync('git',['rev-parse','HEAD^{tree}'],{encoding:'utf8'}).trim();
if(cp.execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim())throw Error('dirty acceptance tree');
const bootstrap=read(prefix+'bootstrap.json'),changed=read(prefix+'changed.json'),full=read(prefix+'full.json'),release=read(prefix+'release-target.json'),harnesses=read(prefix+'harnesses.json');
if(bootstrap.source.source!==source||bootstrap.source.tree!==tree||bootstrap.status!=='passed'||release.status!=='passed')throw Error('source or policy mismatch');
for(const receipt of [changed,full])if(receipt.git_head!==source||receipt.status!=='passed'||!receipt.input_stable||receipt.unrun_gates.length||receipt.gates.some(g=>g.status!=='passed'||g.execution!=='fresh'))throw Error('incomplete official acceptance');
if(changed.worktree_input_digest!==full.worktree_input_digest||full.worktree_input_digest!==full.final_worktree_input_digest)throw Error('snapshot mismatch');
if(harnesses.source!==source||harnesses.harnesses.length!==8||harnesses.harnesses.some(h=>!h.terminal_result.startsWith('test result: ok.')))throw Error('harness terminal mismatch');
const target=dir+'accepted-e-source-records';fs.mkdirSync(target,{recursive:false});
for(const [name,value] of [['bootstrap',bootstrap],['changed',changed],['full',full],['release-target',release],['harnesses',harnesses]])fs.writeFileSync(target+'/'+name+'.json',JSON.stringify(value,null,2)+'\n');
const prior=read(dir+'source-candidate-receipt.json');
const result={...prior,source,tree,parent:cp.execFileSync('git',['rev-parse','HEAD^'],{encoding:'utf8'}).trim(),status:'passed',bootstrap,changed,full,release_target:release,official_harnesses:harnesses,
 previous_attempt:{source:prior.source,status:'failed',receipt:'../source-first-attempt.json',reason:'One storage compaction test immediately reopened GC while an inherited reader lease could still be active. All compiler cases passed. The correction preserves immediate production refusal and adds explicit duplicated-descriptor coverage plus bounded test-only retry after reader drop.'},
 publication:'Accepted locally; normal remote integration pending',
 focused:{...prior.focused,gc_test_correction:'3 focused release storage tests passed; release workspace all-target/all-feature Clippy passed'},
 hygiene:'The final changed/full/policy runs observed this exact clean committed tree. Its status file was written before acceptance; no post-acceptance tracked edits were made.',
 compiler_work:'Native execution remains unmetered. Input bytes, traversal fuel and cumulative modeled allocations are separate quantities; no live heap or RSS reduction is claimed.'};
delete result.failure;
result.expanded_backend.scope='Numeric stage observations are retained from the focused run; all 40 self-host tests also passed in the exact-source official full retry. This remains a Tier 1 fixed point.';
const lib=harnesses.harnesses.find(h=>h.file==='1-run.stdout');
result.validation={library:lib.terminal_result,selfhost_tests:lib.selfhost.length,native_runtime_tests:lib.native.length,gates:full.gates.length,fresh:full.fresh_passed_gates,reused:full.reused_passed_gates,release_target:'passed'};
result.evidence_files=['source-stage-observations.log','source-replay-v6.log','source-gc-focused-v1.log','source-gc-clippy-v1.log'].map(n=>{const b=fs.readFileSync(dir+n);return {path:'../'+n,bytes:b.length,sha256:crypto.createHash('sha256').update(b).digest('hex')}});
fs.writeFileSync(target+'/receipt.json',JSON.stringify(result,null,2)+'\n');
process.stdout.write(JSON.stringify({source,tree,status:result.status,validation:result.validation})+'\n');
