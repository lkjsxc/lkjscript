// Value generator v4: random valid programs; rustc (debug, overflow checks on) and lkjscript must agree on the
// returned checksum, or on the kind and source line of the trap. Every function ends by mixing each live variable
// into an order-sensitive hash, so a wrong intermediate value changes the result.
// usage: node value.mjs <lkjscript-binary> <seed> <count>     (writes cases<seed>/ in the current directory)
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { run } from '../process.mjs';
import { runRoutes } from '../batch.mjs';
import { oracleCache } from '../oracle.mjs';
import { createHash } from 'node:crypto';
import path from 'node:path';
const [binary, seedS = '601', countS = '100', output = 'target/generators'] = process.argv.slice(2);
const bin = path.resolve(binary || 'target/debug/lkjscript');
const engines = (process.env.LKJ_ENGINES || process.env.LKJ_ENGINE || 'native').split(',');
const sh = (c, a) => (spawnSync(c, a, { encoding: 'utf8' }).stdout || '').trim();
// Capture identities before execution so later edits cannot relabel this run.
const metadata = { sha256: createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).update(fs.readFileSync(new URL('../process.mjs', import.meta.url))).digest('hex'), binarySha256: createHash('sha256').update(fs.readFileSync(bin)).digest('hex'), compilerDirty: sh('git', ['status', '--porcelain', '--untracked-files=no']) !== '', compiler: sh('git', ['-C', path.dirname(fs.realpathSync(bin)), 'rev-parse', 'HEAD']) || 'unknown', rustc: sh('rustc', ['--version']), node: process.version };
let state = (BigInt(seedS) * 0x9E3779B97F4A7C15n & 0xFFFFFFFFFFFFFFFFn) || 1n;
function rnd(n) { state ^= (state << 13n) & 0xFFFFFFFFFFFFFFFFn; state ^= state >> 7n; state ^= (state << 17n) & 0xFFFFFFFFFFFFFFFFn; return Number(state % BigInt(n)); }
const pick = a => a[rnd(a.length)];
const INTS = ['i64', 'i32', 'u8', 'usize', 'u64', 'i8', 'u32', 'i16'];
const bits = t => t === 'usize' ? 64 : Number(t.slice(1));
const signed = t => t[0] === 'i';
function lit(t) {
  if (t === 'bool') return pick(['true', 'false']);
  if (rnd(14) > 0) return `${rnd(9)}${t}`;
  const b = BigInt(bits(t)); const max = signed(t) ? (1n << (b - 1n)) - 1n : (1n << b) - 1n;
  return `${pick([max, max - 1n, max >> 1n, 100n, 127n])}${t}`;
}
const PRE = `enum En { A(i64), B(u8, bool), C }
struct Hs { v: Vec<i64>, n: i64 }
fn addto(p: &mut i64, q: &i64) { *p = *p + *q; }
fn sumr(w: &Vec<i64>) -> i64 { let mut s: i64 = 0; let mut i = 0; while i < w.len() { s = s + w[i]; i = i + 1; } s }
fn sumv(w: Vec<i64>) -> i64 { sumr(&w) }
fn mv(h: Hs) -> Hs { h }
fn first(w: &Vec<i64>) -> &i64 { &w[0] }
fn bump(w: &mut Vec<i64>) { let n: usize = w.len(); if n > 0usize { w[n - 1usize] = w[n - 1usize] + 1; } w.push(n as i64); }
fn unbox(b: Box<i64>) -> i64 { *b }
fn pickopt(o: Option<Box<i64>>, d: i64) -> i64 { match o { Some(b) => *b, None => d } }
fn inferred_pair() -> (i64, u8) { (40, 2) }
fn replace_mix(p: &mut i64, n: i64) { let old = std::mem::replace(p, n); let next = std::mem::replace(p, old); *p = mix(*p, next); }
fn refill<T>(slot: &mut T, first: T, second: T) -> T { let old = std::mem::replace(slot, first); drop(old); std::mem::replace(slot, second) }
fn borrow<T>(value: &mut T) -> &mut T { value }
struct Slot<'a> { value: &'a mut i64 }
fn projected_mix(slot: &mut i64, n: i64) { let mut held = Slot { value: slot }; replace_mix(held.value, n); *held.value = mix(*held.value, n); }

fn mix(h: i64, x: i64) -> i64 { ((h % 1000003) * 1009 + (x % 1000003) + 2000006) % 1000003 }
fn mixv(h: i64, w: &Vec<i64>) -> i64 { let mut s: i64 = h; let mut i = 0; while i < w.len() { s = mix(s, w[i]); i = i + 1; } mix(s, w.len() as i64) }
`;
function gen() {
  let src = PRE; const fns = []; let uid = 0; const structs = [];
  function expr(t, env, d) {
    const vars = env.filter(v => v.t === t);
    const c = rnd(d <= 0 ? 3 : 26);
    if (c === 0 || (d <= 0 && !vars.length)) return lit(t);
    if (c <= 2 && vars.length) return pick(vars).n;
    if (t === 'bool') {
      if (c < 9) { const it = pick(INTS); return `(${expr(it, env, d - 1)} ${pick(['<', '<=', '>', '>=', '==', '!='])} ${expr(it, env, d - 1)})`; }
      if (c < 14) return `(${expr('bool', env, d - 1)} ${pick(['&&', '||'])} ${expr('bool', env, d - 1)})`;
      if (c < 16) return `(!${expr('bool', env, d - 1)})`;
      return `(if ${expr('bool', env, d - 1)} { ${expr('bool', env, d - 1)} } else { ${expr('bool', env, d - 1)} })`;
    }
    const sub = () => expr(t, env, d - 1), b = () => expr('bool', env, d - 1), us = () => expr('usize', env, d - 1), i64e = () => expr('i64', env, d - 1);
    if (c < 6) return `(${sub()} ${pick(signed(t) ? ['+', '-', '+', '&', '|', '^', '*'] : ['+', '+', '&', '|', '^', '%', '/'])} ${sub()})`;
    if (c < 7) return `(${sub()} ${pick(['<<', '>>'])} ${rnd(7)}${t})`;
    if (c < 9) { const from = pick(INTS.concat(['bool'])); return `(${expr(from, env, d - 1)} as ${t})`; }
    if (c < 10) return `(if ${b()} { ${sub()} } else { ${sub()} })`;
    if (c < 11) { const f = fns.filter(f => f.ret === t); if (f.length) { const g = pick(f); return `${g.name}(${g.params.map(p => expr(p, env, d - 1)).join(', ')})`; } }
    if (c < 12) { const n = `b${uid++}`; const bt = pick(INTS); return `{ let ${n}: ${bt} = ${expr(bt, env, d - 1)}; ${expr(t, env.concat([{ n, t: bt }]), d - 1)} }`; }
    if (c < 13) { const it = pick(INTS); return `(match ${expr(it, env, d - 1)} { ${rnd(5)}${it} => ${sub()}, ${5 + rnd(3)}${it}..=${9 + rnd(9)}${it} => ${sub()}, _ => ${sub()} })`; }
    if (c < 14 && structs.length) { const st = pick(structs); const i = st.fields.indexOf(t); if (i >= 0) return `(${st.name} { ${st.fields.map((f, j) => `f${j}: ${expr(f, env, d - 1)}`).join(', ')} }).f${i}`; }
    if (c < 15) { const n = `w${uid++}`; const annotation = uid % 2 ? `: Vec<${t}>` : ''; return `{ let mut ${n}${annotation} = Vec::new(); ${n}.push(${sub()}); ${n}.push(${sub()}); ${n}[(${us()} % 2usize)] }`; }
    if (c < 16) {
      const n = `x${uid++}`, kind = rnd(3);
      if (kind === 1) return `{ let mut ${n}: Box<${t}> = Box::new(${sub()}); let old = refill(&mut ${n}, Box::new(${sub()}), Box::new(${sub()})); *old + *${n} }`;
      if (kind === 2) return `{ let mut ${n}: Box<${t}> = Box::new(${sub()}); let r: &mut ${t} = borrow(&mut ${n}); *r }`;
      return `{ let ${n}: Box<${t}> = Box::new(${sub()}); *${n} }`;
    }
    if (c < 17) { const n = `t${uid++}`; return `{ let ${n}: (${t}, bool) = (${sub()}, ${b()}); if ${n}.1 { ${n}.0 } else { ${sub()} } }`; }
    if (c < 18) { const n = `a${uid++}`; return `{ let ${n}: [${t}; 3] = [${sub()}, ${sub()}, ${sub()}]; ${n}[(${us()} % 3usize)] }`; }
    if (c < 19) { const n = `o${uid++}`; return `{ let ${n}: Option<${t}> = if ${b()} { Some(${sub()}) } else { None }; match ${n} { Some(y) => y, None => ${sub()} } }`; }
    if (c < 20) { const n = `e${uid++}`; const v = pick([`En::A(${i64e()})`, `En::B(${expr('u8', env, d - 1)}, ${b()})`, 'En::C']); return `{ let ${n}: En = ${v}; match ${n} { En::A(y) => (y as ${t}), En::B(y, z) => if z { (y as ${t}) } else { ${sub()} }, En::C => ${sub()} } }`; }
    if (t === 'i64') {
      const vecs = env.filter(v => v.t === 'vec');
      if (c < 21 && vecs.length) { const w = pick(vecs).n; return pick([`sumr(&${w})`, `(${w}.len() as i64)`, `(if ${w}.len() > 0usize { ${w}[(${us()} % ${w}.len())] } else { ${sub()} })`, `(if ${w}.len() > 0usize { *first(&${w}) } else { ${sub()} })`]); }
      if (c < 22) { const n = `h${uid++}`; return `{ let mut ${n}: Vec<i64> = Vec::new(); ${n}.push(${sub()}); let g: Hs = mv(Hs { v: ${n}, n: ${sub()} }); sumv(g.v) + g.n }`; }
      if (c < 23) return `unbox(Box::new(${sub()}))`;
      if (c < 24) return `pickopt(if ${b()} { Some(Box::new(${sub()})) } else { None }, ${sub()})`;
      if (c < 25) { const n = `m${uid++}`; return `{ let mut ${n}: i64 = ${sub()}; let q: i64 = ${sub()}; ${pick([`addto(&mut ${n}, &q);`, `replace_mix(&mut ${n}, q);`, `projected_mix(&mut ${n}, q);`])} ${n} }`; }
    }
    return lit(t);
  }
  let lastEnv = [];
  function stmts(env, d, n, ret, inLoop) {
    let out = ''; env = env.slice();
    for (let i = 0; i < n; i++) {
      const c = rnd(16);
      if (c < 4) { const t = pick(INTS.concat(['bool'])); const name = `v${uid++}`; const m = rnd(2) === 0; out += `let ${m ? 'mut ' : ''}${name}: ${t} = ${expr(t, env, d)};\n`; env.push({ n: name, t, m }); }
      else if (c < 6) { const ms = env.filter(v => v.m); if (ms.length) { const v = pick(ms); out += v.t === 'bool' || rnd(2) ? `${v.n} = ${expr(v.t, env, d)};\n` : `${v.n} ${pick(['+=', '-=', '*='])} ${expr(v.t, env, d)};\n`; } }
      else if (c < 7 && d > 0) out += `if ${expr('bool', env, d)} {\n${stmts(env, d - 1, 2, ret, inLoop)}}\n`;
      else if (c < 8 && d > 0) out += `if ${expr('bool', env, d)} {\n${stmts(env, d - 1, 2, ret, inLoop)}} else {\n${stmts(env, d - 1, 2, ret, inLoop)}}\n`;
      else if (c < 9 && d > 0) { const k = `k${uid++}`; out += `let mut ${k}: i64 = 0;\nwhile ${k} < ${1 + rnd(4)} {\n${k} = ${k} + 1;\n${stmts(env.concat([{ n: k, t: 'i64' }]), d - 1, 3, ret, true)}}\n`; }
      else if (c < 10 && d > 0) { const k = `k${uid++}`; out += `for ${k} in 0..${1 + rnd(4)}usize {\n${stmts(env.concat([{ n: k, t: 'usize' }]), d - 1, 3, ret, true)}}\n`; }
      else if (c < 11) { const name = `w${uid++}`; out += `let mut ${name}: Vec<i64> = Vec::new();\n`; for (let j = rnd(3); j > 0; j--) out += `${name}.push(${expr('i64', env, 1)});\n`; env.push({ n: name, t: 'vec' }); }
      else if (c < 13) { const vecs = env.filter(v => v.t === 'vec'); if (vecs.length) { const w = pick(vecs).n; out += pick([`${w}.push(${expr('i64', env, d)});\n`, `bump(&mut ${w});\n`, `if ${w}.len() > 0usize { let last: usize = ${w}.len() - 1usize; ${w}[last] = ${expr('i64', env, d)}; }\n`]); } }
      else if (c < 14) { const ms = env.filter(v => v.m && v.t === 'i64'), qs = env.filter(v => v.t === 'i64'); if (ms.length && qs.length) { const p = pick(ms).n, q = pick(qs).n; if (p !== q) out += `addto(&mut ${p}, &${q});\n`; } }
      else if (c < 15 && inLoop) out += `if ${expr('bool', env, 1)} { ${pick(['break', 'continue'])}; }\n`;
      else if (ret && rnd(3) === 0) out += `if ${expr('bool', env, 1)} { return ${expr(ret, env, 1)}; }\n`;
    }
    lastEnv = env;
    return out;
  }
  const tail = (t, e, env) => 'let mut hh: i64 = 0;\nlet inferred = inferred_pair();\nhh = mix(hh, inferred.0);\nhh = mix(hh, inferred.1 as i64);\n' + env.map(x => x.t === 'vec' ? `hh = mixv(hh, &${x.n});\n` : `hh = mix(hh, (${x.n} as i64));\n`).join('') + `let rr: ${t} = ${e};\nhh = mix(hh, (rr as i64));\n${t === 'i64' ? 'hh' : `((hh % 100) as ${t})`}`;
  for (let i = 0; i < 2; i++) { const fields = [pick(INTS), pick(INTS), 'bool']; structs.push({ name: `S${i}`, fields }); src += `struct S${i} { ${fields.map((f, j) => `f${j}: ${f}`).join(', ')} }\n`; }
  for (let i = 0; i < 3; i++) {
    const params = Array.from({ length: rnd(3) }, () => pick(INTS)); const ret = pick(INTS); const name = `f${i}`;
    const env = params.map((t, j) => ({ n: `p${j}`, t }));
    { const body = stmts(env, 2, 4, ret, false); const e2 = lastEnv; src += `fn ${name}(${params.map((t, j) => `p${j}: ${t}`).join(', ')}) -> ${ret} {\n${body}${tail(ret, expr(ret, env, 2), e2)}\n}\n`; }
    fns.push({ name, params, ret });
  }
  { const body = stmts([], 2, 9, 'i64', false); const e2 = lastEnv; src += `pub fn probe() -> i64 {\n${body}${tail('i64', expr('i64', [], 3), e2)}\n}\n`; }
  return src;
}
const dir = path.resolve(output, `value-${seedS}`); fs.mkdirSync(dir, { recursive: true });
const N = Number(countS); const srcs = []; const start = [];
let harness = '#![allow(warnings)]\n'; let line = 2;
for (let i = 0; i < N; i++) { const s = gen(); srcs.push(s); fs.writeFileSync(`${dir}/c${i}.rs`, s); start.push(line); const body = `mod c${i} {\n${s}}\n`; line += body.split('\n').length - 1; harness += body; }
harness += 'fn main() { let n: usize = std::env::args().nth(1).unwrap().parse().unwrap(); let r = match n {\n' + srcs.map((_, i) => `${i} => c${i}::probe(),`).join('\n') + '\n_ => 0 }; println!("{}", r); }\n';
fs.writeFileSync(`${dir}/oracle.rs`, harness);
const FLAGS = ['--edition=2024', '-C', 'overflow-checks=yes', '-C', 'opt-level=0', '-A', 'warnings', '-A', 'arithmetic_overflow', '-A', 'unconditional_panic', '-A', 'overflowing_literals'];
// Outcomes are kept apart: a value, a trap of a given kind at a given line, a compile-time rejection, a crash, a timeout.
const rustKind = m => /with overflow/.test(m) ? 'R0001' : /divide by zero|divisor of zero/.test(m) ? 'R0002' : /index out of bounds/.test(m) ? 'R0003' : /assertion/.test(m) ? 'R0005' : 'R0004';
async function rustRun(i) {
  const r = await run(`${dir}/oracle`, [String(i)], { encoding: 'utf8', timeout: 10000 });
  if (r.error || r.signal) return { k: 'abnormal', text: String(r.error?.code || r.signal) };
  if (r.status === 0) return { k: 'value', v: r.stdout.trim() };
  const m = r.stderr.match(/panicked at [^:\n]+:(\d+):\d+:\n(.*)/);
  if (r.status === 101 && m) return { k: 'trap', kind: rustKind(m[2]), line: Number(m[1]) - start[i], text: m[2] };
  return { k: 'abnormal', text: 'status ' + r.status };
}
function lkRun(l) {
  const text = (l.stdout + l.stderr).trim().split('\n')[0] || '';
  if (l.error || l.signal) return { k: l.error?.code === 'ETIMEDOUT' ? 'timeout' : 'crash', text };
  if (l.status === 0) return { k: 'value', v: text, text };
  const m = text.match(/:(\d+):(?:\d+:)? (E_[A-Z_]+|M_[A-Z_]+|R\d{4})\b/);
  if (m && m[2][0] === 'R') return { k: 'trap', kind: m[2], line: Number(m[1]), text };
  if (m && m[2][0] === 'M') return { k: 'machine-error', code: m[2], text };
  if (m) return { k: 'reject', code: m[2], text };
  return { k: 'crash', text };
}
const references=await oracleCache({generator:metadata.sha256,seed:seedS,count:N,flags:FLAGS,harness},async()=>{
const rc = await run('rustc', [...FLAGS, `${dir}/oracle.rs`, '-o', `${dir}/oracle`], { encoding: 'utf8' });
if (rc.status !== 0) { console.log('oracle failed:\n' + rc.stderr.split('\n').slice(0, 40).join('\n')); process.exit(1); }

const results=[];for(let i=0;i<N;i++){const result=await rustRun(i);if(result.k==='abnormal')throw new Error('abnormal Rust oracle: '+result.text);results.push(result);}return results;
});
const actualRoutes=await runRoutes(bin,srcs.map((_,i)=>({path:dir+'/c'+i+'.rs',entry:'probe'})),{engines,output:dir,timeout:20000});
for(const engine of engines) {
const actualResults=actualRoutes[engine];
const tally = {}; const show = []; const bump = c => { tally[c] = (tally[c] || 0) + 1; };
for (let i = 0; i < N; i++) {
  const r = references[i], l = lkRun(actualResults[i]); let cls;
  if (r.k === 'abnormal') cls = 'oracle-abnormal';
  else if (l.k === 'timeout' || l.k === 'crash') cls = l.k;
  else if (l.k === 'machine-error') cls = 'machine-error:' + l.code;
  else if (l.k === 'reject') cls = (l.code === 'E_UNSUPPORTED' || l.code === 'E_PARSE') ? 'unsupported' : 'rejects-valid:' + l.code;
  else if (r.k === 'value' && l.k === 'value') cls = r.v === l.v ? 'value-equal' : 'VALUE-DIFFERS';
  else if (r.k === 'trap' && l.k === 'trap') cls = r.kind !== l.kind ? 'TRAP-KIND-DIFFERS' : r.line !== l.line ? 'trap-same-kind-other-line' : 'trap-equal';
  else cls = 'VALUE-VS-TRAP';
  bump(cls);
  if (cls !== 'value-equal' && cls !== 'trap-equal') show.push(`${dir}/c${i} ${cls} rust=${r.k === 'value' ? r.v : r.k === 'trap' ? r.kind + '@' + r.line + ' ' + r.text : r.text} lkj=${(l.text || '').replace(/^\S*\.(rs|elf):/, '').slice(0, 150)}`);
}
const record = { generator: 'value', engine, ...metadata, rustFlags: FLAGS.join(' '), seed: seedS, programs: N, ...tally };
fs.mkdirSync(output, {recursive: true});
fs.appendFileSync(path.join(output, 'runs.jsonl'), JSON.stringify(record) + '\n');
console.log(JSON.stringify(record)); fs.writeFileSync(`${dir}/report-${engine}.txt`, show.join('\n') + '\n');

// These generated families are implemented. A valid-source rejection is a
// regression too; keep its outcome class, but never let it silently pass the gate.
const fatal = Object.keys(tally).some(key => key !== 'value-equal' && key !== 'trap-equal');
if (fatal) process.exitCode = 1;
}
