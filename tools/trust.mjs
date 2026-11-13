#!/usr/bin/env node
// Physical source size is an audit aid, not a proof or weighted security score.
import fs from 'node:fs';
const groups={
 'K contract and safety gate':['src/k.rs','src/check.rs','src/facts.rs'],
 'Lowering, layout and runtime':['src/lower.rs','src/lower/ir.rs'],
 'Encoding and metadata':['src/wasm.rs','src/encoding.rs','src/json.rs'],
 'Production host boundary':['bridge/main.rs','bridge/lib.rs','bridge/compiler_stack.rs','bridge/run.mjs','bridge/engine.mjs','bridge/worker.mjs','bridge/output.mjs'],
 'Independent K execution model':['src/machine.rs','src/machine/scalar.rs'],
};
if(!fs.existsSync('src/lower.rs')){groups['Lowering, layout and runtime']=['src/wasm.rs'];groups['Encoding and metadata']=['src/encoding.rs','src/json.rs'];}
for(const [role,paths] of Object.entries(groups)){
 const files=paths.filter(file=>fs.existsSync(file)).map(file=>{const text=fs.readFileSync(file,'utf8');return {file,lines:text.split('\n').length-(text.endsWith('\n')?1:0),bytes:Buffer.byteLength(text)};});
 console.log(JSON.stringify({role,lines:files.reduce((n,f)=>n+f.lines,0),bytes:files.reduce((n,f)=>n+f.bytes,0),files}));
}
console.log('The front end is trusted for intended source meaning; K independently checks accepted concrete safety. The machine is an oracle, not a production prerequisite. Rust/Cargo, Node/V8 and Linux remain external dependencies. A source allocator still needs its raw-memory invariants audited.');
