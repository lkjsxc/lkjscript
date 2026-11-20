#!/usr/bin/env node
// Exact-site evidence controls; no compiler or external specimen is required.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {json,sha} from './rerank.mjs';
const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'lkjscript-rerank-')),tool=path.join(path.dirname(fileURLToPath(import.meta.url)),'rerank.mjs');
let checks=0,sequence=0;
const check=(ok,message)=>{checks++;if(!ok)throw Error('rerank-test: '+message)},read=p=>fs.readFileSync(p),write=(p,s)=>fs.writeFileSync(p,s),writeJson=(p,v)=>write(p,json(v)),clone=structuredClone;
const captured=s=>({text:s,bytes:Buffer.byteLength(s),sha256:sha(s)});
function invoke(after,...extra){
  const output=tmp+'/report-'+sequence+++'.json';writeJson(tmp+'/after.json',after);
  const c=spawnSync(process.execPath,[tool,'--baseline',tmp+'/baseline.json','--before',tmp+'/before.json','--after',tmp+'/after.json',
    '--projection',tmp+'/projection.json','--source',tmp+'/source','--output',output,...extra],{encoding:'utf8',maxBuffer:Infinity});
  return [c.status,fs.existsSync(output)?JSON.parse(read(output)):undefined,c.stderr];
}
try{
  fs.mkdirSync(tmp+'/source/src',{recursive:true});
  const source='fn a() -> impl Value {}\nfn b(x: Qualified) {}\nfn c() {}\n';
  write(tmp+'/source/src/lib.rs',source);
  const manifest={schema:1,kind:'git_tracked_suffix_projection',source:{root:tmp+'/source',commit:'a'.repeat(40)},
    files:[{path:'src/lib.rs',bytes:Buffer.byteLength(source),sha256:sha(source)}],aliases:[{from:'src/lib.rs',to:'src/lib.lkj'}]};
  writeJson(tmp+'/projection.json',manifest);const digest=sha(read(tmp+'/projection.json'));
  const diagnostics=['src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl','src/lib.lkj:2:9: E_PARSE expected an identifier',
    'src/lib.lkj:1:16: E_UNSUPPORTED where predicate on a nonparameter','src/lib.lkj:1:1: E_BLOCKED remaining semantic analysis: source errors'];
  const receipt={schema:1,kind:'unmodified_real_program_root_observations',package:{commit:'a'.repeat(40)},compiler:{path:'/compiler',sha256:'b'.repeat(64)},
    projection_manifest_sha256:digest,verification_before:{manifest_sha256:digest},verification_after:{manifest_sha256:digest},observations:[]};
  for(const route of ['original_suffix','suffix_projection'])receipt.observations.push({route,upstream_entry:'src/lib.rs',entry_sha256:sha(source),entry_bytes:Buffer.byteLength(source),exit_status:1,launcher_signal:0,
    command:['/compiler','check',route==='original_suffix'?'src/lib.rs':'src/lib.lkj','--source-dir',tmp+(route==='original_suffix'?'/source':'/projected')],
    stdout:captured(''),stderr:captured(route==='original_suffix'?'src/lib.rs: E_SOURCE_EXTENSION source files use .lkj\n':diagnostics.join('\n')+'\n'),time_report:captured('0\n')});
  writeJson(tmp+'/before.json',receipt);
  const table={accounting:{unique_primary_sites:2},current_verification:{receipt_sha256:sha(read(tmp+'/before.json'))},causes:[
    {id:'impl_trait',title:'impl Trait',owner:'Language',rank:1,sites:[['src/lib.rs',1,11,null,false]]},
    {id:'qualified',title:'Qualified path',owner:'Language',rank:2,sites:[['src/lib.rs',2,9,null,false]]}],
    recovery_exclusions:[{id:'known_recovery',diagnostic_indices:[2],parent_diagnostic_indices:[0]}]};
  writeJson(tmp+'/baseline.json',table);
  let [status,report,error]=invoke(receipt);
  check(status===0,'unchanged baseline failed: '+error);
  check(report.baseline_site_comparison.length===2&&report.unclassified_unique_diagnostics===0,'unchanged baseline exactly accounts for primary sites');
  check(report.inherited_recovery_unique_diagnostics===1&&report.raw_by_code.E_BLOCKED===1,'recovery and E_BLOCKED stay separate');
  check(report.baseline_site_comparison.every(s=>s.state==='exact_diagnostic_survives'),'both old sites survive unchanged');
  const absolute=clone(receipt);absolute.observations[1].stderr=captured(diagnostics.map(d=>tmp+'/projected/'+d).join('\n')+'\n');
  [status,report,error]=invoke(absolute);check(status===0,'authenticated absolute projected aliases failed: '+error);
  check(report.observed_classified_sites===2&&!report.unclassified_unique_diagnostics&&report.inherited_recovery_unique_diagnostics===1&&report.baseline_site_comparison.every(s=>s.state==='exact_diagnostic_survives'),'absolute aliases retain exact primary and recovery identity');
  for(const file of [tmp+'/projected-other/src/lib.lkj',tmp+'/projected/../projected/src/lib.lkj',tmp+'/source/src/lib.rs',tmp+'/projected/src/untracked.lkj']){
    const after=clone(receipt);after.observations[1].stderr=captured(file+':1:11: E_UNSUPPORTED unsupported construct: impl\n');
    [status,report,error]=invoke(after);check(status!==0&&!report&&/outside the tracked original source/.test(error),'absolute root, traversal or untracked path must refuse: '+file+': '+error);
  }
  const changedRoot=clone(absolute);changedRoot.observations[1].command[4]=tmp+'/forged-projection';
  [status,report,error]=invoke(changedRoot);check(status!==0&&!report&&/command changed/.test(error),'after receipt cannot replace authenticated source-dir');
  const noRoot=clone(receipt);noRoot.observations[1].command.splice(3,2);writeJson(tmp+'/before.json',noRoot);
  const noRootTable=clone(table);noRootTable.current_verification.receipt_sha256=sha(read(tmp+'/before.json'));writeJson(tmp+'/baseline.json',noRootTable);
  const unproven=clone(absolute);unproven.observations[1].command=clone(noRoot.observations[1].command);
  [status,report,error]=invoke(unproven);check(status!==0&&!report&&/outside the tracked original source/.test(error),'legacy receipt cannot authenticate an absolute path');
  writeJson(tmp+'/before.json',receipt);writeJson(tmp+'/baseline.json',table);
  let after=clone(receipt);after.observations[1].stderr=captured(diagnostics[1]+'\n'+diagnostics[3]+'\n');
  [status,report,error]=invoke(after);check(status===0,'removed diagnostic case failed: '+error);
  let impl=report.baseline_site_counts.find(c=>c.id==='impl_trait');
  check(impl.not_observed_sites===1&&impl.exact_surviving_sites===0,'absent old site is not observed rather than accepted');
  after=clone(receipt);after.observations[1].stderr=captured([diagnostics[1],diagnostics[2],diagnostics[3]].join('\n')+'\n');
  [status,report,error]=invoke(after);check(status===0&&report.inherited_recovery_unique_diagnostics===0,'recovery exclusion cannot survive removal of its exact parent');
  check(report.unclassified_unique_diagnostics===1,'formerly recovering diagnostic without parent needs fresh review');
  after=clone(receipt);after.observations[1].stderr=captured('src/lib.lkj:1:11: E_TYPE different frontier\n'+diagnostics[1]+'\n'+diagnostics[3]+'\n');
  [status,report,error]=invoke(after);check(status===0&&report.unclassified_unique_diagnostics===1,'changed diagnostic is unclassified');
  impl=report.baseline_site_counts.find(c=>c.id==='impl_trait');check(impl.changed_diagnostic_sites===1&&!impl.not_observed_sites,'changed coordinate stays distinguished');
  after=clone(receipt);after.observations[1].stderr=captured([...diagnostics,'src/lib.lkj:3:1: E_UNSUPPORTED unsupported construct: impl'].join('\n')+'\n');
  [status,report,error]=invoke(after);check(status===0&&report.unclassified_unique_diagnostics===1,'same words at a new site never inherit a guessed cause');
  const review={schema:1,sites:[{path:'src/lib.rs',line:3,column:1,code:'E_UNSUPPORTED',message:'unsupported construct: impl',file_sha256:sha(source),
    cause:'new_reviewed_form',title:'Reviewed form',owner:'Language',reason:'Fixture source review'}]};
  const reviewed=()=>{writeJson(tmp+'/review.json',review);return invoke(after,'--review',tmp+'/review.json')};
  [status,report,error]=reviewed();check(status===0&&!report.unclassified_unique_diagnostics,'exact reviewed new site is classified');
  review.sites[0].disposition='recovery';review.sites[0].parent={path:'src/lib.rs',line:1,column:11,code:'E_UNSUPPORTED',message:'unsupported construct: impl'};
  [status,report,error]=reviewed();check(status===0&&report.inherited_recovery_unique_diagnostics===2&&!report.unclassified_unique_diagnostics,'new recovery requires an exact surviving parent');
  review.sites[0].parent.message='unobserved parent';[status,report,error]=reviewed();check(status!==0&&/parent does not match/.test(error),'invented recovery parent refuses');
  Object.assign(review.sites[0].parent,{message:'unsupported construct: impl',line:3,column:1});
  [status,report,error]=reviewed();check(status!==0&&/parent does not match/.test(error),'self-referential recovery parent refuses');
  Object.assign(review.sites[0].parent,{line:1,column:11});writeJson(tmp+'/review.json',review);
  const reordered=clone(after);reordered.observations[1].stderr=captured('src/lib.lkj:3:1: E_UNSUPPORTED unsupported construct: impl\n'+diagnostics.join('\n')+'\n');
  [status,report,error]=invoke(reordered,'--review',tmp+'/review.json');check(status!==0&&/parent is not earlier/.test(error),'later recovery parent refuses');
  delete review.sites[0].disposition;delete review.sites[0].parent;review.sites[0].file_sha256='f'.repeat(64);
  [status,report,error]=reviewed();check(status!==0&&/review source hash mismatch/.test(error),'review cannot name unrelated source bytes');
  for(const [name,mutate,reason]of [
    ['stream hash',r=>r.observations[1].stderr.sha256='f'.repeat(64),/stream byte\/hash mismatch/],
    ['signal',r=>r.observations[1].launcher_signal=9,/ended abnormally/],
    ['missing route',r=>r.observations.pop(),/incomplete original\/projection/],
    ['changed command',r=>r.observations[1].command.push('--changed'),/command changed/],
    ['unparsed output',r=>r.observations[1].stderr=captured('unknown error\n'),/unparsed diagnostic/],
    ['bad coordinate',r=>r.observations[1].stderr=captured('src/lib.lkj:1:999: E_PARSE wrong\n'),/outside original source coordinates/],
    ['unknown source',r=>r.observations[1].stderr=captured('other.lkj:1:1: E_PARSE wrong\n'),/outside the tracked original source/]]){
    after=clone(receipt);mutate(after);[status,report,error]=invoke(after);check(status!==0&&!report&&reason.test(error),name+' must refuse: '+error);
  }
  write(tmp+'/source/src/lib.rs',source+'// changed');[status,report,error]=invoke(receipt);
  check(status!==0&&/original source bytes changed/.test(error),'source hashes checked before counting');write(tmp+'/source/src/lib.rs',source);
  const dom=clone(receipt),duplicate='src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl item';
  dom.observations[1].stderr=captured([diagnostics[0],diagnostics[1],duplicate,diagnostics[3]].join('\n')+'\n');writeJson(tmp+'/before.json',dom);
  const domTable={counts:{unique_primary_sites:2},current_verification:{receipt_sha256:sha(read(tmp+'/before.json'))},
    causes:[{id:'impl_trait',cause:'impl Trait',owner:'Language',rank:1},{id:'qualified',cause:'Qualified path',owner:'Language',rank:2}],
    primary_sites:[['src/lib.rs:1:11','impl_trait','E_UNSUPPORTED','observed_cause'],['src/lib.rs:2:9','qualified','E_PARSE','observed_cause']],
    recovery_duplicate_messages:[{coordinate:'src/lib.rs:1:11',primary_code:'E_UNSUPPORTED',primary_message:'unsupported construct: impl',secondary_code:'E_UNSUPPORTED',secondary_message:'unsupported construct: impl item'}]};
  writeJson(tmp+'/baseline.json',domTable);[status,report,error]=invoke(dom);
  check(status===0&&report.observed_classified_sites===2&&report.inherited_recovery_unique_diagnostics===1,'DOM primary/recovery split: '+error);
  dom.observations[1].stderr=captured([diagnostics[1],duplicate,diagnostics[3]].join('\n')+'\n');[status,report,error]=invoke(dom);
  check(status===0&&report.unclassified_unique_diagnostics===1,'same-coordinate duplicate without former primary is not hidden');
  write(tmp+'/source/src/other.rs',source);manifest.files.push({path:'src/other.rs',bytes:Buffer.byteLength(source),sha256:sha(source)});manifest.aliases.push({from:'src/other.rs',to:'src/other.lkj'});writeJson(tmp+'/projection.json',manifest);
  const sozu=clone(receipt);sozu.projection_manifest_sha256=sha(read(tmp+'/projection.json'));sozu.verification_before.manifest_sha256=sozu.projection_manifest_sha256;sozu.verification_after.manifest_sha256=sozu.projection_manifest_sha256;
  sozu.observations[1].stderr=captured(diagnostics[0]+'\n'+diagnostics[1]+'\n');
  for(const o of clone(sozu.observations)){o.upstream_entry='src/other.rs';o.command[2]=o.route==='original_suffix'?'src/other.rs':'src/other.lkj';sozu.observations.push(o)}
  writeJson(tmp+'/before.json',sozu);writeJson(tmp+'/baseline.json',{current_verification:{receipt_sha256:sha(read(tmp+'/before.json'))},groups:{
    impl_trait:{owner:'Language',rank:1,diagnosed_sites:1,independent_gap_sites:1,diagnostic_kinds:['E_UNSUPPORTED: unsupported construct: impl'],sites:[['src/lib.rs',1,11,0,'src/lib.rs']]},
    parse_recovery_or_unclassified:{owner:'Unknown',rank:2,diagnosed_sites:1,independent_gap_sites:0,diagnostic_kinds:['E_PARSE: expected an identifier'],sites:[['src/lib.rs',2,9,0,'src/lib.rs']]}}});
  [status,report,error]=invoke(sozu);check(status===0,'Sozu schema and repeated-root diagnostics: '+error);
  check(report.raw_projected_diagnostic_count===4&&report.observed_classified_sites===1&&report.observed_prior_unclassified_sites===1,'Sozu raw rows, classified and prior unknown counts stay distinct');
  check(report.ranked_observed_causes[0].sites[0].observed_roots.length===2,'deduplicated site retains both observing roots');
  console.log(json({passed:true,checks}));
}catch(e){console.error(e.message);process.exitCode=255}finally{fs.rmSync(tmp,{recursive:true,force:true})}
