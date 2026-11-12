#!/usr/bin/env node
// Physical source lines, with balanced test-only items separated from Rust core.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {statementSpans} from './generators/reduce.mjs';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const git=(...args)=>execFileSync('git',args,{cwd:root,encoding:'utf8',maxBuffer:Infinity});
const digest=text=>createHash('sha256').update(text).digest('hex');
const labels={
  compiler:'Compiler Rust, excluding embedded tests',
  library:'Source library',
  compiler_tests:'Embedded compiler Rust tests',
  library_tests:'Embedded library Rust tests',
  rust_tests:'Rust test harnesses',
  node_tests:'Node unit tests',
  conformance:'Conformance source programs',
  k_fixtures:'K fixtures and model counterexamples',
  corpus:'Independent corpus source programs',
  reduced:'Retained reduced source programs',
  examples:'Example source programs',
  bridge_rust:'Rust bootstrap bridges, excluding tests',
  bridge_tests:'Embedded Rust bridge tests',
  bridge_node:'Node bootstrap bridges',
  tools:'Verification and measurement tools',
};

function category(file) {
  const rs=file.endsWith('.rs'), node=/\.(?:mjs|js)$/.test(file);
  if(file.startsWith('tests/conformance/')) return rs?'conformance':file.endsWith('.k')?'k_fixtures':undefined;
  if(file.startsWith('tests/corpus/')) return rs?'corpus':undefined;
  if(file.startsWith('tools/model-counterexamples/')) return file.endsWith('.k')?'k_fixtures':undefined;
  if(file.startsWith('tools/generators/reduced/')) return rs?'reduced':undefined;
  if(file.startsWith('examples/')) return rs?'examples':undefined;
  if(file.startsWith('src/')&&rs) return 'compiler';
  if(file.startsWith('library/')&&rs) return 'library';
  if(file.startsWith('bridge/')) return rs?'bridge_rust':node?'bridge_node':undefined;
  if(file.startsWith('tests/')) return rs?'rust_tests':node?'node_tests':undefined;
  if(file.startsWith('tools/')) return /\.test\.mjs$/.test(file)?'node_tests':node||rs||file.endsWith('.sh')?'tools':undefined;
}
// Inspect an item's outer attributes without mistaking comments or doc strings
// for cfg conditions. The shared span reader already excludes quoted item text.
function isTestItem(text,start) {
  const trivia=position=>{
    for(;;){
      while(position<text.length&&/\s/.test(text[position]))position++;
      if(text.startsWith('//',position)){const end=text.indexOf('\n',position+2);position=end<0?text.length:end;continue;}
      if(text.startsWith('/*',position)){
        position+=2;let depth=1;
        while(position<text.length&&depth){if(text.startsWith('/*',position)){depth++;position+=2;}else if(text.startsWith('*/',position)){depth--;position+=2;}else position++;}
        continue;
      }
      return position;
    }
  };
  const literal=position=>{
    const raw=text.slice(position).match(/^(?:br|cr|r)(#*)"/);
    if(raw){const end=text.indexOf('"'+raw[1],position+raw[0].length);return end<0?text.length:end+1+raw[1].length;}
    const quote=text[position];if(quote!=='"'&&quote!=="'")return position;
    if(quote==="'"){const lifetime=text.slice(position).match(/^'[A-Za-z_][A-Za-z0-9_]*/);if(lifetime&&text[position+lifetime[0].length]!=="'")return position;}
    let end=position+1;
    while(end<text.length){if(text[end]==='\\')end+=2;else if(text[end++]===quote)break;}
    return end;
  };
  let position=trivia(start),found=false;
  while(text.startsWith('#[',position)){
    const begin=position+2;position=begin;let depth=1;
    while(position<text.length&&depth){
      position=trivia(position);const end=literal(position);
      if(end!==position){position=end;continue;}
      if(text[position]==='[')depth++;else if(text[position]===']')depth--;
      position++;
    }
    if(depth)throw new Error('unterminated Rust attribute while counting lines');
    for(const tokens of [['cfg','(','test',')'],['test']]){
      let cursor=trivia(begin);
      for(const token of tokens){
        if(!text.startsWith(token,cursor)){cursor=-1;break;}
        cursor=trivia(cursor+token.length);
      }
      if(cursor===position-1)found=true;
    }
    position=trivia(position);
  }
  return found;
}
export function splitRust(text) {
  const lines=text===''?[]:text.split('\n');
  if(lines.at(-1)==='') lines.pop();
  const test=new Uint8Array(lines.length), starts=[0];
  for(let at=0;at<text.length;at++) if(text[at]==='\n') starts.push(at+1);
  const lineAt=position=>{
    let low=0,high=starts.length;
    while(low+1<high){const middle=Math.floor((low+high)/2);if(starts[middle]<=position)low=middle;else high=middle;}
    return low;
  };
  let items=0;
  for(const span of statementSpans(text)) {
    if(!isTestItem(text,span.start)) continue;
    items++;
    for(let line=lineAt(span.start);line<=lineAt(span.end-1);line++) test[line]=1;
  }
  const counts={ordinary:{lines:0,nonblank:0},embedded:{lines:0,nonblank:0},items};
  for(let line=0;line<lines.length;line++){
    const group=test[line]?counts.embedded:counts.ordinary;
    group.lines++;group.nonblank+=Number(lines[line].trim()!=='');
  }
  return counts;
}
function plain(text) {
  const lines=text===''?[]:text.split('\n');
  if(lines.at(-1)==='') lines.pop();
  return {lines:lines.length,nonblank:lines.filter(line=>line.trim()!=='').length};
}
export function countLines(revision) {
  const commit=git('rev-parse',revision||'HEAD').trim();
  const paths=(revision?git('ls-tree','-rz','--name-only',commit):git('ls-files','--cached','--others','--exclude-standard','-z'))
    .split('\0').filter(Boolean);
  const categories=Object.fromEntries(Object.entries(labels).map(([key,label])=>[key,{label,files:0,lines:0,nonblank:0}]));
  const manifest=[], embeddedFiles=[];
  const add=(key,count)=>{if(!count.lines)return;const value=categories[key];value.files++;value.lines+=count.lines;value.nonblank+=count.nonblank;};
  for(const file of [...new Set(paths)].sort()) {
    const key=category(file);if(!key)continue;
    const text=revision?git('show',commit+':'+file):fs.readFileSync(path.join(root,file),'utf8');
    manifest.push(file+'\0'+digest(text));
    const embedded={compiler:'compiler_tests',library:'library_tests',bridge_rust:'bridge_tests'}[key];
    if(embedded){
      const split=splitRust(text);add(key,split.ordinary);add(embedded,split.embedded);
      if(split.embedded.lines) embeddedFiles.push({file,items:split.items,...split.embedded});
    } else add(key,plain(text));
  }
  return {tool:'count-lines',revision:commit,working_tree:!revision,
    dirty:revision?false:git('status','--porcelain','--untracked-files=no').trim()!=='',
    code_manifest_sha256:digest(manifest.join('\n')),tool_sha256:digest(fs.readFileSync(fileURLToPath(import.meta.url))),
    method:'Physical lines include blanks/comments and an unterminated final line. Rust items with a direct #[cfg(test)] or #[test] outer attribute are separated using balanced token-aware spans; preceding attributes and intervening comments stay with the item. Adjacent comments outside the item and other cfg forms remain in the containing category. Strings containing source code remain harness/tool lines. Documents, configuration, generated artifacts and ignored files are excluded. Split-file counts can overlap; line categories do not.',
    categories,embedded_files:embeddedFiles,
    total:{files:manifest.length,lines:Object.values(categories).reduce((sum,c)=>sum+c.lines,0),nonblank:Object.values(categories).reduce((sum,c)=>sum+c.nonblank,0)}};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const args=process.argv.slice(2);
  if(args.includes('--help')){
    console.log('usage: node tools/count-lines.mjs [--revision COMMIT] [--json] [--write RECORD.json]');
  } else {
    const option=name=>{const index=args.indexOf(name);if(index>=0&&!args[index+1])throw new Error('missing '+name+' value');return index<0?undefined:args[index+1];};
    const result=countLines(option('--revision')), output=option('--write');
    if(output){const file=path.resolve(root,output);fs.mkdirSync(path.dirname(file),{recursive:true});fs.writeFileSync(file,JSON.stringify(result,null,2)+'\n');}
    if(args.includes('--json')) console.log(JSON.stringify(result));
    else {
      console.log('Revision '+result.revision+'; '+(result.working_tree?'working tree':'git object')+(result.dirty?' (dirty)':''));
      console.log('Category\tFiles\tPhysical lines\tNonblank lines');
      for(const group of Object.values(result.categories))if(group.lines)console.log(group.label+'\t'+group.files+'\t'+group.lines+'\t'+group.nonblank);
      console.log('TOTAL (unique files)\t'+result.total.files+'\t'+result.total.lines+'\t'+result.total.nonblank);
      console.log(result.method);
    }
  }
}
