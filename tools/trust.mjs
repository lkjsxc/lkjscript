#!/usr/bin/env node
// Physical source size is an audit aid, not a proof or weighted security score.
import fs from 'node:fs';
const groups={
 'K contract and safety gate':['src/k.rs','src/check.rs','src/check','src/facts.rs'],
 'Lowering, layout and lowered contract':['src/lower.rs','src/lower'],
 'Source runtime and raw-memory binding':['library/runtime.rs','src/runtime.rs'],
 'K text and diagnostics':['src/k_text.rs','src/kc.rs','src/diagnostic.rs'],
 'Native encoder and Linux entry':['src/native.rs','src/native'],
 'Production host boundary':['bridge/main.rs','bridge/lib.rs','bridge/batch.rs','bridge/library.rs','bridge/compiler_stack.rs','bridge/runtime.rs','bridge/native.rs','bridge/process.rs','bridge/world.rs','bridge/output.rs','bridge/kc.rs'],
 'Independent K and raw-memory models':['src/machine.rs','src/machine','src/runtime_memory.rs'],
};
function sourceFiles(entry){
 if(!fs.existsSync(entry))return [];
 if(!fs.statSync(entry).isDirectory())return [entry];
 return fs.readdirSync(entry,{withFileTypes:true}).sort((a,b)=>a.name.localeCompare(b.name))
  .flatMap(child=>child.isDirectory()||child.name.endsWith('.rs')?sourceFiles(entry+'/'+child.name):[]);
}
for(const [role,paths] of Object.entries(groups)){
 const files=paths.flatMap(sourceFiles).map(file=>{const text=fs.readFileSync(file,'utf8');return {file,lines:text.split('\n').length-(text.endsWith('\n')?1:0),bytes:Buffer.byteLength(text)};});
 console.log(JSON.stringify({role,lines:files.reduce((n,f)=>n+f.lines,0),bytes:files.reduce((n,f)=>n+f.bytes,0),files}));
}
console.log('The front end is trusted for intended source meaning; K independently checks accepted concrete safety. The source runtime is checked as ordinary code, but its raw-memory invariants and sealed bindings remain trusted. The K/raw-memory models are oracles, not production prerequisites. Rust/Cargo build the compiler; Linux runs native artifacts; Node currently runs the development generators, not artifacts.');
