#!/usr/bin/env node
// Physical source size is an audit aid, not a proof or weighted security score.
import fs from 'node:fs';
const groups={
 'K contract and safety gate':['src/k.rs','src/check.rs','src/facts.rs'],
 'Lowering, layout and lowered contract':['src/lower.rs','src/lower/ir.rs'],
 'Source runtime and raw-memory binding':['library/runtime.rs','src/runtime.rs'],
 'WebAssembly encoder and metadata':['src/wasm.rs','src/encoding.rs','src/json.rs'],
 'Native encoder and Linux entry':['src/native.rs'],
 'Production host boundary':['bridge/main.rs','bridge/lib.rs','bridge/batch.rs','bridge/library.rs','bridge/compiler_stack.rs','bridge/runtime.rs','bridge/native.rs','bridge/run.mjs','bridge/engine.mjs','bridge/output.mjs'],
 'Independent K and raw-memory models':['src/machine.rs','src/machine/scalar.rs','src/runtime/memory.rs'],
};
for(const [role,paths] of Object.entries(groups)){
 const files=paths.filter(file=>fs.existsSync(file)).map(file=>{const text=fs.readFileSync(file,'utf8');return {file,lines:text.split('\n').length-(text.endsWith('\n')?1:0),bytes:Buffer.byteLength(text)};});
 console.log(JSON.stringify({role,lines:files.reduce((n,f)=>n+f.lines,0),bytes:files.reduce((n,f)=>n+f.bytes,0),files}));
}
console.log('The front end is trusted for intended source meaning; K independently checks accepted concrete safety. The source runtime is checked as ordinary code, but its raw-memory invariants and sealed bindings remain trusted. The K/raw-memory models are oracles, not production prerequisites. Rust/Cargo build the compiler; Linux runs native artifacts; Node/V8 remains the optional WebAssembly host.');
