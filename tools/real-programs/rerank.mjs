#!/usr/bin/env node
// Recount reviewed exact diagnostic sites; never infer causes from keywords.
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const fail=s=>{throw new Error('real-programs-rerank: '+s)};
const dict=()=>Object.create(null),has=(o,k)=>Object.hasOwn(o,k);
export const sha=s=>createHash('sha256').update(s).digest('hex');
export function json(v,pretty=false){
  const sort=x=>Array.isArray(x)?x.map(sort):x!==null&&typeof x==='object'?Object.fromEntries(Object.keys(x).sort().map(k=>[k,sort(x[k])])):x;
  return JSON.stringify(sort(v),null,pretty?3:undefined).replace(/[\u0080-\uffff]/g,c=>'\\u'+c.charCodeAt(0).toString(16).padStart(4,'0'));
}
const bytes=p=>fs.readFileSync(p);
function text(raw,label){try{return new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(raw)}catch{fail('invalid UTF-8 '+label)}}
function load(p){const raw=bytes(p);try{return [JSON.parse(text(raw,p)),sha(raw)]}catch(e){fail('invalid JSON '+p+': '+e.message)}}
function stream(s){
  if(!s||Array.isArray(s)||typeof s!=='object')fail('missing captured stream');
  let raw;if(has(s,'text')){if(typeof s.text!=='string'||Buffer.from(s.text,'utf8').toString('utf8')!==s.text)fail('invalid UTF-8 diagnostic stream');raw=Buffer.from(s.text)}
  else if(has(s,'base64'))raw=Buffer.from(s.base64,'base64');else fail('captured stream has neither text nor base64');
  if(raw.length!==s.bytes||sha(raw)!==s.sha256)fail('captured stream byte/hash mismatch');
  return text(raw,'diagnostic stream');
}
const coordinate=r=>[r.path,r.line,r.column].join(':'),key=r=>json([r.path,r.line,r.column,r.code,r.message]),observationKey=r=>json([r.route,r.upstream_entry]);
const unique=values=>[...new Set(values)];
function prefixes(receipt){
  const found=new Set();
  for(const o of receipt.observations){
    if(o.route!=='suffix_projection')continue;
    if(!Array.isArray(o.command))fail('missing captured probe command');
    const roots=[];
    for(let i=0;i<o.command.length;i++)if(o.command[i]==='--source-dir'){
      if(++i>=o.command.length)fail('captured source-dir is missing its value');roots.push(o.command[i]);
    }
    if(roots.length>1)fail('ambiguous captured source-dir');
    if(!roots.length)continue;
    let root=roots[0];if(!root.startsWith('/')||/(?:^|\/)\.\.?(?:\/|$)|\0|\/\//.test(root))continue;
    if(root!=='/')root=root.replace(/\/$/,'');found.add(root==='/'?'/':root+'/');
  }
  return [...found].sort((a,b)=>b.length-a.length||(a<b?-1:a>b?1:0));
}
function reader(manifest,root,roots){
  const files=new Map(manifest.files.map(f=>[f.path,f])),aliases=new Map(manifest.aliases.map(a=>[a.to,a.from])),cache=new Map();
  return (path,line,column)=>{
    if(path.startsWith('/'))for(const prefix of roots)if(path.startsWith(prefix)){path=path.slice(prefix.length);break}
    const outside=()=>fail('diagnostic is outside the tracked original source: '+path);
    if(/^\/|(?:^|\/)\.\.?(?:\/|$)|\0/.test(path))outside();
    path=aliases.get(path)??path;
    if(!files.has(path)||/^\/|(?:^|\/)\.\.?(?:\/|$)|\0/.test(path))outside();
    if(!cache.has(path)){
      const raw=bytes(root+'/'+path),expected=files.get(path);
      if(raw.length!==expected.bytes||sha(raw)!==expected.sha256)fail('original source bytes changed: '+path);
      const starts=[0],lines=[];let at=0;
      for(let i=0;i<raw.length;i++)if(raw[i]===10){lines.push(raw.subarray(at,i));at=i+1;starts.push(at)}
      lines.push(raw.subarray(at));cache.set(path,{raw,lines,starts,physical:raw.length?lines.length-(raw.at(-1)===10?1:0):0});
    }
    if(!line&&!column)return {path,line:0,column:0,source:null};
    const f=cache.get(path);
    if(!(line>0&&line<=f.lines.length&&column>0&&column<=f.lines[line-1].length+1))fail('diagnostic is outside original source coordinates: '+path+':'+line+':'+column);
    const raw=f.lines[line-1],expected=files.get(path);
    return {path,line:Number(line),column:Number(column),source:{file_sha256:expected.sha256,file_bytes:expected.bytes,
      file_physical_lines:f.physical,byte_offset:f.starts[line-1]+Number(column)-1,line_sha256:sha(raw),line_text:text(raw,path+':'+line)}};
  };
}
function observations(receipt,manifest,manifestSha,source){
  if(receipt.schema!==1||receipt.kind!=='unmodified_real_program_root_observations')fail('not an unmodified root-probe receipt');
  if(receipt.projection_manifest_sha256!==manifestSha||receipt.verification_before.manifest_sha256!==manifestSha||
     receipt.verification_after.manifest_sha256!==manifestSha||receipt.package.commit!==manifest.source.commit)fail('receipt/projection identity mismatch');
  const files=new Map(manifest.files.map(f=>[f.path,f])),rows=[],annotations=[],observations=dict(),routes=dict();
  for(const o of receipt.observations){
    const id=observationKey(o);if(has(observations,id))fail('duplicate route/entry in receipt');
    observations[id]=o;(routes[o.upstream_entry]??=new Set()).add(o.route);
    const entry=files.get(o.upstream_entry);
    if(!entry||entry.sha256!==o.entry_sha256||entry.bytes!==o.entry_bytes)fail('probe entry does not match a tracked original file');
    stream(o.stdout);const stderr=stream(o.stderr);stream(o.time_report);
    if(o.launcher_signal||(o.exit_status!==0&&o.exit_status!==1))fail('root observation ended abnormally');
    if(o.route==='original_suffix')continue;if(o.route!=='suffix_projection')fail('unknown probe route');
    let index=0;const lines=stderr.split('\n');while(lines.length&&lines.at(-1)==='')lines.pop();
    for(const line of lines){
      const match=line.match(/^(.+):(\d+):(\d+): (E_[A-Z0-9_]+) (.*)$/);
      if(match){
        const [,path,number,column,code,message]=match;
        if((!Number(number)||!Number(column))&&!(code==='E_BLOCKED'&&!Number(number)&&!Number(column)))fail('unlocated primary diagnostic');
        rows.push({...source(path,Number(number),Number(column)),code,message,root:o.upstream_entry,diagnostic_index:index++,raw:line});
      }else if(line.startsWith('LKJSCRIPT_EDITS ')){
        let edits;try{edits=JSON.parse(line.slice(16))}catch{fail('malformed diagnostic edit annotation')}
        if(!edits||Array.isArray(edits)||typeof edits!=='object')fail('malformed diagnostic edit annotation');
        annotations.push({root:o.upstream_entry,raw:line});
      }else fail('unparsed diagnostic line (must be reviewed): '+line);
    }
    if(!o.exit_status&&stderr.length)fail('successful root check emitted diagnostics');
    if(o.exit_status&&!stderr.length)fail('refused root check emitted no diagnostics');
  }
  for(const [entry,r]of Object.entries(routes))if(r.size!==2||!r.has('original_suffix')||!r.has('suffix_projection'))fail('incomplete original/projection observation pair: '+entry);
  if(!Object.keys(routes).length)fail('empty observation cohort');
  return {rows,annotations,observations};
}
function baselineClasses(table,rows){
  const classes=dict(),labels=dict(),primary=dict(),excluded=dict(),byCoordinate=dict();
  for(const row of rows)(byCoordinate[coordinate(row)]??=[]).push(row);
  function add(row,cause,site_id,partition){
    const k=key(row);if(has(classes,k)&&classes[k].cause!==cause)fail('conflicting reviewed cause at exact diagnostic');
    classes[k]={cause,site_id,partition};const p=primary[site_id]??={keys:[]};
    if(!p.keys.includes(k))p.keys.push(k);Object.assign(p,{cause,coordinate:coordinate(row),source:row.source});
  }
  if(has(table,'accounting')){
    if(unique(rows.map(r=>r.root)).length!==1)fail('nom baseline expects exactly one projected root');
    const indices=new Map(rows.map(r=>[r.diagnostic_index,r]));
    for(const exclusion of table.recovery_exclusions){
      if(exclusion.diagnostic_indices.length!==exclusion.parent_diagnostic_indices.length)fail('reviewed recovery requires a parent diagnostic for each row');
      exclusion.diagnostic_indices.forEach((index,i)=>{
        const parent=exclusion.parent_diagnostic_indices[i];
        if(!indices.has(index))fail('missing reviewed recovery diagnostic index '+index);
        if(!indices.has(parent))fail('missing reviewed recovery parent index '+parent);
        excluded[key(indices.get(index))]={reason:exclusion.id,parent_key:key(indices.get(parent))};
      });
    }
    for(const cause of table.causes){
      labels[cause.id]={title:cause.title,owner:cause.owner,baseline_rank:cause.rank};
      for(const site of cause.sites){
        const coord=site.slice(0,3).join(':'),matched=(byCoordinate[coord]??[]).filter(r=>r.code!=='E_BLOCKED'&&!has(excluded,key(r)));
        if(!matched.length)fail('reviewed nom primary coordinate has no exact diagnostic: '+coord);
        for(const row of matched)add(row,cause.id,cause.id+':'+coord,site[4]?'inactive_in_upstream_default':'default_candidate');
      }
    }
    if(Object.keys(primary).length!==table.accounting.unique_primary_sites)fail('nom baseline primary count mismatch');
  }else if(has(table,'primary_sites')){
    for(const cause of table.causes)labels[cause.id]={title:cause.cause,owner:cause.owner,baseline_rank:cause.rank};
    for(const d of table.recovery_duplicate_messages){
      const candidates=byCoordinate[d.coordinate]??[],parents=candidates.filter(r=>r.code===d.primary_code&&r.message===d.primary_message);
      if(!parents.length)fail('reviewed duplicate has no primary diagnostic');
      for(const row of candidates)if(row.code===d.secondary_code&&row.message===d.secondary_message)excluded[key(row)]={reason:'same_coordinate_recovery_duplicate',parent_key:key(parents[0])};
    }
    for(const [coord,cause,code,partition]of table.primary_sites){
      const matched=(byCoordinate[coord]??[]).filter(r=>r.code===code&&!has(excluded,key(r)));
      if(!matched.length)fail('reviewed DOM primary coordinate has no exact diagnostic: '+coord);
      for(const row of matched)add(row,cause,cause+':'+coord,partition);
    }
    if(Object.keys(primary).length!==table.counts.unique_primary_sites)fail('DOM baseline primary count mismatch');
  }else if(has(table,'groups')){
    for(const cause of Object.keys(table.groups).sort()){
      const g=table.groups[cause];labels[cause]={title:cause,owner:g.owner,baseline_rank:g.rank};
      for(const site of g.sites){
        const coord=site.slice(0,3).join(':'),kind=g.diagnostic_kinds[site[3]],matched=(byCoordinate[coord]??[]).filter(r=>r.code+': '+r.message===kind);
        if(!matched.length)fail('reviewed Sozu site has no exact diagnostic: '+coord+' '+kind);
        for(const row of matched)add(row,cause,cause+':'+coord+':'+kind,g.independent_gap_sites?'observed_cause':'unclassified_recovery');
      }
    }
    if(Object.keys(primary).length!==Object.values(table.groups).reduce((n,g)=>n+g.diagnosed_sites,0))fail('Sozu baseline primary count mismatch');
  }else fail('unsupported baseline classification schema');
  for(const row of rows){const k=key(row);if(row.code==='E_BLOCKED')continue;
    if(!has(classes,k)&&!has(excluded,k))fail('unaccounted baseline diagnostic: '+row.raw);
    if(has(classes,k)&&has(excluded,k))fail('baseline primary is also excluded');
  }
  return {classes,labels,primary,excluded};
}
export function main(){
  const args=dict(),allowed=new Set(['baseline','before','after','projection','source','output','review']);
  for(let i=2;i<process.argv.length;i++){
    const match=process.argv[i].match(/^--?([^=]+)(?:=(.*))?$/);if(!match)fail('use --help');
    const [,name,equal]=match;if(name==='help'){args.help=true;continue}
    if(!allowed.has(name))fail('use --help');args[name]=equal??process.argv[++i];if(args[name]===undefined)fail('use --help');
  }
  if(args.help){console.log('Usage: node rerank.mjs --baseline TABLE.json --before OLD_PROBE.json --after NEW_PROBE.json\n       --projection PROJECTION/projection.json [--source ORIGINAL_SOURCE]\n       [--review REVIEWED_NEW_SITES.json] --output NEW_REPORT.json\nClassifications require exact diagnostics and verified original source bytes.\nMissing diagnostics mean not observed, never admitted. The output must be new.');return}
  if(['baseline','before','after','projection','output'].some(k=>!args[k]))fail('missing required arguments; use --help');
  try{fs.lstatSync(args.output);fail('output already exists')}catch(e){if(e.code!=='ENOENT')throw e}
  const [table,tableSha]=load(args.baseline),[before,beforeSha]=load(args.before),[after,afterSha]=load(args.after),[manifest,manifestSha]=load(args.projection);
  if(manifest.schema!==1||manifest.kind!=='git_tracked_suffix_projection')fail('not a tracked suffix-projection manifest');
  if(table.current_verification.receipt_sha256!==beforeSha)fail('before receipt is not the baseline table current verification');
  const source=reader(manifest,args.source??manifest.source.root,prefixes(before)),old=observations(before,manifest,manifestSha,source),current=observations(after,manifest,manifestSha,source);
  if(json(Object.keys(old.observations).sort())!==json(Object.keys(current.observations).sort()))fail('probe root/route scope changed');
  let originalEqual=true;
  for(const k of Object.keys(old.observations)){
    const a=old.observations[k],b=current.observations[k];
    if(a.entry_sha256!==b.entry_sha256||a.entry_bytes!==b.entry_bytes)fail('probe source-entry identity changed');
    if(json(a.command.slice(1))!==json(b.command.slice(1)))fail('probe command changed beyond compiler path');
    if(a.route==='original_suffix')originalEqual&&=json([a.exit_status,a.launcher_signal,a.stdout,a.stderr])===json([b.exit_status,b.launcher_signal,b.stdout,b.stderr]);
  }
  const {classes,labels,primary,excluded}=baselineClasses(table,old.rows),byKey=dict(),atCoordinate=dict();
  for(const row of current.rows){if(row.code==='E_BLOCKED')continue;const k=key(row),r=byKey[k]??={...row,observed_roots:[]};
    if(!r.observed_roots.includes(row.root))r.observed_roots.push(row.root);(atCoordinate[coordinate(row)]??=[]).push(k);
  }
  let reviewSha;
  if(args.review){
    const [review,digest]=load(args.review);reviewSha=digest;
    if(review.schema!==1||!Array.isArray(review.sites))fail('unsupported explicit-review schema');
    for(const entry of review.sites){
      const k=key(entry);if(!has(byKey,k))fail('review does not match an observed diagnostic');
      if(entry.file_sha256!==byKey[k].source.file_sha256)fail('review source hash mismatch');
      if(['cause','title','owner','reason'].some(n=>entry[n]===undefined||!String(entry[n]).length))fail('review needs cause/title/owner/reason');
      if(has(classes,k)||(has(excluded,k)&&has(byKey,excluded[k].parent_key)))fail('explicit review cannot overwrite surviving baseline classification');
      const disposition=entry.disposition??'primary';
      if(disposition==='recovery'){
        if(!entry.parent||Array.isArray(entry.parent)||typeof entry.parent!=='object')fail('reviewed recovery requires a parent diagnostic');
        const parent=key(entry.parent);if(parent===k||!has(byKey,parent))fail('reviewed recovery parent does not match another observed diagnostic');
        for(const row of current.rows.filter(r=>key(r)===k))if(!current.rows.some(r=>key(r)===parent&&r.root===row.root&&r.diagnostic_index<row.diagnostic_index))fail('reviewed recovery parent is not earlier in the same root');
        excluded[k]={reason:entry.cause,parent_key:parent,review_reason:entry.reason};continue;
      }
      if(disposition!=='primary')fail('unknown reviewed disposition');
      delete excluded[k];classes[k]={cause:entry.cause,site_id:'review:'+k,partition:'explicitly_reviewed',review_reason:entry.reason};
      labels[entry.cause]??={title:entry.title,owner:entry.owner};
    }
  }
  const comparison=[],beforeCounts=dict(),survival=dict();
  for(const id of Object.keys(primary).sort()){
    const site=primary[id],surviving=site.keys.filter(k=>has(byKey,k)),at=unique(atCoordinate[site.coordinate]??[]);
    const state=surviving.length?'exact_diagnostic_survives':at.length?'different_diagnostic_at_same_coordinate':'not_observed';
    beforeCounts[site.cause]=(beforeCounts[site.cause]??0)+1;const counts=survival[site.cause]??=dict();counts[state]=(counts[state]??0)+1;
    comparison.push({site_id:id,...site,state,surviving_diagnostic_keys:surviving,current_diagnostic_keys_at_coordinate:at});
  }
  const groups=dict(),rawByCode=dict(),seen=new Set(),unknown=[],recovery=[];let classified=0,priorUnknown=0;
  for(const row of current.rows)rawByCode[row.code]=(rawByCode[row.code]??0)+1;
  for(const k of Object.keys(byKey).sort()){
    const row=byKey[k];if(has(excluded,k)&&has(byKey,excluded[k].parent_key)){recovery.push({...row,exclusion:excluded[k]});continue}
    const c=classes[k];if(!c){unknown.push(row);continue}if(seen.has(c.site_id))continue;seen.add(c.site_id);
    if(c.partition==='unclassified_recovery')priorUnknown++;else classified++;
    const g=groups[c.cause]??={sites:[],files:dict()};g.sites.push({...row,...c});g.files[row.path]=row.source.file_physical_lines;
  }
  const ranked=Object.keys(groups).sort().map(cause=>{
    const g=groups[cause];return {id:cause,...labels[cause],sites:g.sites,observed_sites:g.sites.length,affected_files:Object.keys(g.files).sort(),
      overlapping_file_physical_lines:Object.values(g.files).reduce((a,b)=>a+b,0),baseline_sites:beforeCounts[cause]??0,old_site_survival:survival[cause]??{}};
  }).sort((a,b)=>b.observed_sites-a.observed_sites||(a.id<b.id?-1:a.id>b.id?1:0));
  ranked.forEach((r,i)=>r.count_rank=i+1);
  const counts=Object.keys(beforeCounts).sort().map(id=>({id,...labels[id],baseline_sites:beforeCounts[id],
    exact_surviving_sites:survival[id]?.exact_diagnostic_survives??0,changed_diagnostic_sites:survival[id]?.different_diagnostic_at_same_coordinate??0,not_observed_sites:survival[id]?.not_observed??0}));
  const report={schema:1,kind:'exact_foreign_refusal_comparison',inputs:{baseline:{path:args.baseline,sha256:tableSha},
    before:{path:args.before,sha256:beforeSha,compiler:before.compiler},after:{path:args.after,sha256:afterSha,compiler:after.compiler},
    projection:{path:args.projection,sha256:manifestSha,source:manifest.source},...(args.review?{review:{path:args.review,sha256:reviewSha}}:{})},
    observation_count:Object.keys(current.observations).length,original_suffix_observations_unchanged:originalEqual,
    raw_projected_diagnostic_count:current.rows.length,raw_by_code:rawByCode,observed_classified_sites:classified,observed_prior_unclassified_sites:priorUnknown,
    inherited_recovery_unique_diagnostics:recovery.length,unclassified_unique_diagnostics:unknown.length,baseline_site_counts:counts,baseline_site_comparison:comparison,
    ranked_observed_causes:ranked,unclassified_diagnostics:unknown,inherited_recovery_diagnostics:recovery,diagnostic_edit_annotations:current.annotations,
    interpretation:[
      'Causes are inherited only from an explicitly classified, identical diagnostic at the same unchanged source coordinate; reviewed new sites name the complete code/message and file hash.',
      'Missing old diagnostics are not observed at this frontier. This is not proof of admission, successful execution, or equivalent Cargo feature/test configuration.',
      'New or changed diagnostics stay unclassified until explicitly reviewed. Known recovery exclusions remain exclusions only while both their complete diagnostic identity and their reviewed parent diagnostic survive.',
      'Source columns are one-based UTF-8 byte columns, matching the compiler lexer, not the Unicode-scalar columns of census.mjs.',
      'Count ranking is descriptive. Ownership and implementation priority remain explicit decisions. File coverage overlaps and is not a causal blocked-line estimate.']};
  fs.writeFileSync(args.output,json(report,true)+'\n',{flag:'wx',mode:0o600});
  console.log(json({report:args.output,observations:report.observation_count,old_sites:comparison.length,unclassified_diagnostics:unknown.length,ranked_causes:ranked.length}));
}
if(process.argv[1]&&fs.realpathSync(process.argv[1])===fileURLToPath(import.meta.url)){
  try{main()}catch(e){console.error(e.message.startsWith('real-programs-rerank:')?e.message:'real-programs-rerank: '+e.message);process.exitCode=255}
}
