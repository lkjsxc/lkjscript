// Destructor-order generator. Every value of type M writes one byte when it is created and another when it is
// dropped, so the program's output is the exact order of evaluation and destruction. Programs are valid Rust by
// construction (the generator tracks moves); rustc's output is the reference for both lkjscript routes.
// usage: node drops.mjs <lkjscript-binary> <seed> <count> [out-dir]      env ONE_SIDED=<n>: 1 in n `if`s is unbalanced (default 6)
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { run } from '../process.mjs';
import { runRoutes } from '../batch.mjs';
import { oracleCache, packResult, unpackResult, worldOutcome } from '../oracle.mjs';
const [binArg, seedS = '1', countS = '100', outArg = 'target/generators'] = process.argv.slice(2);
const bin = path.resolve(binArg); const repo = path.resolve(path.dirname(fs.realpathSync(bin)), '../..');
const ONE_SIDED = Number(process.env.ONE_SIDED || 6);
const engines=(process.env.LKJSCRIPT_ENGINES || 'native,interpreter').split(',');
const generatorHash=createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).digest('hex');
const binarySha256=createHash('sha256').update(fs.readFileSync(bin)).digest('hex');
let state = (BigInt(seedS) * 0x9E3779B97F4A7C15n & 0xFFFFFFFFFFFFFFFFn) || 1n;
function rnd(n) { state ^= (state << 13n) & 0xFFFFFFFFFFFFFFFFn; state ^= state >> 7n; state ^= (state << 17n) & 0xFFFFFFFFFFFFFFFFn; return Number(state % BigInt(n)); }
const pick = a => a[rnd(a.length)];
const PRE = `struct M { out: Stdout, b: u8 }
impl Drop for M { fn drop(&mut self) { let a: [u8; 1] = [self.b]; self.out.write(a.as_slice()); } }
struct Pair { a: M, b: M }
struct Loud { m: M, tag: u8, out: Stdout }
impl Drop for Loud { fn drop(&mut self) { let a: [u8; 1] = [self.tag]; self.out.write(a.as_slice()); } }
enum E { One(M), Two(M, M), Zero }
fn mk(world: &mut World, b: u8) -> M { let mut o: Stdout = world.stdout(); let a: [u8; 1] = [b + 100u8]; o.write(a.as_slice()); M { out: o, b: b } }
fn tick(world: &mut World, b: u8) { let mut o: Stdout = world.stdout(); let a: [u8; 1] = [b]; o.write(a.as_slice()); }
fn eat(m: M) {}
fn eatp(p: Pair) {}
fn pass(m: M) -> M { m }
fn two(x: M, y: M) {}
fn c(s: &mut u64) -> bool { *s = (*s * 75u64 + 74u64) % 65537u64; (*s % 2u64) == 1u64 }
`;
function gen() {
  let uid = 0, marks = 0, ticks = 0, blocks = 0;
  const nm = p => `${p}${uid++}`;
  const clone = env => env.map(v => ({ ...v, f: v.f ? { ...v.f } : undefined }));
  const freeze = env => clone(env).map(v => ({ ...v, frozen: true }));
  const join = (p, q) => p === q ? p : 'maybe';
  const add = (env, blk, t, pre, mut = false, f) => { const n = nm(pre); env.push({ id: uid++, n, t, st: 'live', mut, frozen: false, whole: true, f, blk }); return n; };
  const live = (env, t) => env.filter(v => v.t === t && v.st === 'live' && !v.frozen && v.whole && (!v.f || (v.f.a === 'live' && v.f.b === 'live')));
  const gone = v => { v.st = 'moved'; v.whole = false; if (v.f) { v.f.a = 'moved'; v.f.b = 'moved'; } };
  function merge(env, x, y) { // write the join of two branch environments back into env
    for (const v of env) {
      const p = x.find(e => e.id === v.id), q = y.find(e => e.id === v.id);
      v.st = join(p.st, q.st); v.whole = p.whole && q.whole;
      if (v.f) { v.f.a = join(p.f.a, q.f.a); v.f.b = join(p.f.b, q.f.b); }
    }
  }
  function block(env, d, n, ctx) {
    const blk = blocks++; let out = '';
    const W = ctx.helper ? 'world' : '&mut world', S = ctx.helper ? 's' : '&mut s';
    const mk = () => marks < 96 ? `mk(${W}, ${marks++}u8)` : null;
    const tick = () => `tick(${W}, ${200 + (ticks++ % 50)}u8);\n`;
    function consume(e, v) { // one statement that takes ownership of the live M v, in environment e
      gone(v); const k = rnd(9);
      if (k === 0) return `eat(${v.n});\n`;
      if (k === 1) return `drop(${v.n});\n`;
      if (k === 2) { const y = add(e, blk, 'M', 'y'); return `let ${y}: M = ${v.n};\n`; }
      if (k === 3) { const y = add(e, blk, 'M', 'y', true); return `let mut ${y}: M = pass(${v.n});\n`; }
      if (k === 4) { const m = mk(); if (m) return pick([`two(${v.n}, ${m});\n`, `two(${m}, ${v.n});\n`]); return `eat(${v.n});\n`; }
      if (k === 5) { const vs = live(e, 'Vec'); if (vs.length) return `${pick(vs).n}.push(${v.n});\n`; const bx = add(e, blk, 'Box', 'bx'); return `let ${bx}: Box<M> = Box::new(${v.n});\n`; }
      if (k === 6) { const o = add(e, blk, 'Opt', 'o'); return `let ${o}: Option<M> = Some(${v.n});\n`; }
      if (k === 7) { const m = mk(); if (m) { const p = add(e, blk, 'Pair', 'p', true, { a: 'live', b: 'live' }); return pick([`let mut ${p}: Pair = Pair { a: ${v.n}, b: ${m} };\n`, `let mut ${p}: Pair = Pair { b: ${m}, a: ${v.n} };\n`]); } return `drop(${v.n});\n`; }
      const y = add(e, blk, 'M', 'y'); return ctx.h1 ? `let ${y}: M = ${v.n};\n` : `let ${y}: M = h1(${W}, ${S}, ${v.n});\n`;
    }
    function scoped(d2, n2, ctx2) { const before = new Set(env.map(v => v.id)); const text = block(env, d2, n2, ctx2); for (let i = env.length - 1; i >= 0; i--) if (!before.has(env[i].id)) env.splice(i, 1); return text; }
    for (let step = 0; step < n; step++) {
      const k = rnd(30); const ms = live(env, 'M');
      if (k < 4) { const m = mk(); if (m) { const mut = rnd(2) === 0; const x = add(env, blk, 'M', 'x', mut); out += `let ${mut ? 'mut ' : ''}${x}: M = ${m};\n`; } }
      else if (k === 4) { const p1 = mk(), p2 = mk(); if (p1 && p2) { const p = add(env, blk, 'Pair', 'p', true, { a: 'live', b: 'live' }); out += rnd(2) ? `let mut ${p}: Pair = Pair { a: ${p1}, b: ${p2} };\n` : `let mut ${p}: Pair = Pair { b: ${p1}, a: ${p2} };\n`; } }
      else if (k === 5) { const m = mk(); if (m) { const d2 = add(env, blk, 'Loud', 'd'); out += `let ${d2}: Loud = Loud { m: ${m}, tag: ${150 + rnd(40)}u8, out: world.stdout() };\n`; } }
      else if (k === 6) { const m = mk(); if (m) { const o = add(env, blk, 'Opt', 'o'); out += `let ${o}: Option<M> = if c(${S}) { Some(${m}) } else { None };\n`; } }
      else if (k === 7) { const m = mk(); if (m) { const bx = add(env, blk, 'Box', 'bx'); out += `let ${bx}: Box<M> = Box::new(${m});\n`; } }
      else if (k === 8) { const v = add(env, blk, 'Vec', 'v', true); out += `let mut ${v}: Vec<M> = Vec::new();\n`; for (let j = rnd(3); j > 0; j--) { const m = mk(); if (m) out += `${v}.push(${m});\n`; } }
      else if (k === 9) { const p1 = mk(), p2 = mk(); if (p1 && p2) { const t = add(env, blk, 'Tup', 't', true, { a: 'live', b: 'live' }); out += `let mut ${t}: (M, M) = (${p1}, ${p2});\n`; } }
      else if (k === 10) { const p1 = mk(), p2 = mk(), p3 = mk(); if (p1 && p2 && p3) { const e = add(env, blk, 'E', 'e'); out += `let ${e}: E = if c(${S}) { E::One(${p1}) } else { if c(${S}) { E::Two(${p2}, ${p3}) } else { E::Zero } };\n`; } }
      else if (k < 14) { out += ms.length ? consume(env, pick(ms)) : tick(); }
      else if (k === 14) out += tick();
      else if (k === 15) { const p1 = mk(), p2 = mk(); if (p1 && p2) out += pick([`${p1};\n`, `let _ = ${p1};\n`, `eat(${p1});\n`, `two(${p1}, ${p2});\n`, `drop(${p1});\n`]); }
      else if (k < 18 && d > 0) {
        if (rnd(ONE_SIDED) === 0) { // unbalanced: each branch moves what it likes, and the join is "maybe"
          const x = clone(env), y = clone(env); const t1 = block(x, d - 1, 3, ctx); const hasElse = rnd(2) === 0; const t2 = hasElse ? block(y, d - 1, 3, ctx) : '';
          out += `if c(${S}) {\n${t1}}${hasElse ? ` else {\n${t2}}` : ''}\n`; merge(env, x, y);
        } else { // balanced: both branches consume the same outer values, each in its own way
          const victims = ms.slice(0, rnd(3)); const x = freeze(env), y = freeze(env); let t1 = tick(), t2 = tick();
          for (const e of [x, y]) for (const v of victims) e.find(z => z.id === v.id).frozen = false;
          for (const v of victims) { t1 += consume(x, x.find(z => z.id === v.id)); t2 += consume(y, y.find(z => z.id === v.id)); }
          t1 += block(x, d - 1, 2, ctx); t2 += block(y, d - 1, 2, ctx);
          out += `if c(${S}) {\n${t1}} else {\n${t2}}\n`; for (const v of victims) gone(v);
        }
      }
      else if (k === 18 && d > 0) out += `{\n${scoped(d - 1, 3, ctx)}}\n`;
      else if (k === 19 && d > 0) { const i2 = nm('i'); const body = block(freeze(env), d - 1, 3, { ...ctx, loop: true }); out += rnd(2) ? `let mut ${i2}: u8 = 0u8;\nwhile ${i2} < ${1 + rnd(3)}u8 {\n${i2} = ${i2} + 1u8;\n${body}}\n` : `for ${i2} in 0..${1 + rnd(3)}usize {\n${body}}\n`; }
      else if (k === 20) { // fields of a pair or tuple
        const ps = env.filter(v => v.f && !v.frozen && v.whole); if (ps.length) { const p = pick(ps); const which = rnd(2) ? 'a' : 'b'; const fn = p.t === 'Pair' ? which : (which === 'a' ? '0' : '1');
          if (p.f[which] === 'live' && rnd(3) > 0) { const y = add(env, blk, 'M', 'y'); p.f[which] = 'moved'; out += `let ${y}: M = ${p.n}.${fn};\n`; }
          else { const m = mk(); if (m) { p.f[which] = 'live'; out += `${p.n}.${fn} = ${m};\n`; } } } }
      else if (k === 21) { const ps = live(env, 'Pair'); if (ps.length && rnd(2)) { const p = pick(ps); gone(p); out += `eatp(${p.n});\n`; } }
      else if (k === 22) { // take apart an option, box, vector, enum or Loud
        const os = live(env, 'Opt'), bs = live(env, 'Box'), vs = live(env, 'Vec'), es = live(env, 'E'), ds = live(env, 'Loud'); const kind = rnd(5); const tk = () => tick().trim();
        if (kind === 0 && os.length) { const o = pick(os); gone(o); out += `match ${o.n} { Some(m) => { ${tk()} ${pick(['eat(m);', '', 'drop(m);'])} ${tk()} } None => { ${tk()} } }\n`; }
        else if (kind === 1 && bs.length) { const bx = pick(bs); gone(bx); const y = add(env, blk, 'M', 'y'); out += `let ${y}: M = *${bx.n};\n`; }
        else if (kind === 2 && vs.length) { const v = pick(vs); out += `match ${v.n}.pop() { Some(m) => { ${tk()} ${pick(['eat(m);', ''])} ${tk()} } None => {} }\n`; }
        else if (kind === 3 && es.length) { const e = pick(es); gone(e); out += `match ${e.n} { E::One(m) => { ${tk()} } E::Two(m, n) => { ${pick(['eat(n);', 'eat(m);', ''])} ${tk()} } E::Zero => { ${tk()} } }\n`; }
        else if (ds.length) { const d2 = pick(ds); gone(d2); out += `drop(${d2.n});\n`; }
        else if (vs.length && rnd(2)) { const v = pick(vs); gone(v); out += `drop(${v.n});\n`; } }
      else if (k === 23) { const xs = env.filter(v => v.t === 'M' && v.mut && !v.frozen); const m = xs.length ? mk() : null; if (m) { const x = pick(xs); x.st = 'live'; x.whole = true; out += `${x.n} = ${m};\n`; } }
      else if (k === 24) { const xs = ms.filter(v => v.mut); const m = xs.length ? mk() : null; if (m) { const x = pick(xs); const y = add(env, blk, 'M', 'y'); out += `let ${y}: M = std::mem::replace(&mut ${x.n}, ${m});\n`; } }
      else if (k === 25 && !ctx.helper) { const r = nm('r'); out += `let ${r}: u8 = h0(${W}, ${S});\ntick(${W}, ${r});\n`; }
      else if (k === 26 && (ctx.loop || ctx.helper)) { // a branch that leaves: what it moves is still owned on the other path
        const x = clone(env); let t1 = tick(); const leaving = !(ctx.loop && (!ctx.helper || rnd(2)));
        const movable = x.filter(v => v.t === 'M' && v.st === 'live' && v.whole && (leaving || !v.frozen)); if (movable.length && rnd(2)) t1 += consume(x, pick(movable));
        let exit;
        if (!leaving) exit = pick(['break;', 'continue;']);
        else if (ctx.h1) { const ret = x.filter(v => v.t === 'M' && v.st === 'live' && v.whole); const m = mk(); exit = ret.length && rnd(2) ? `return ${pick(ret).n};` : m ? `return ${m};` : null; }
        else exit = `return ${rnd(9)}u8;`;
        if (exit) out += `if c(${S}) {\n${t1}${exit}\n}\n`; }
      else if (k === 27) { const xs = env.filter(v => v.t === 'M' && v.blk === blk); const m = xs.length ? mk() : null; if (m) { const x = pick(xs); env.splice(env.indexOf(x), 1); env.push({ id: uid++, n: x.n, t: 'M', st: 'live', mut: false, frozen: false, whole: true, blk }); out += `let ${x.n}: M = ${m};\n`; } }
      else out += tick();
    }
    return out;
  }
  let src = PRE;
  { const body = block([], 2, 5, { helper: true, h0: true }); src += `fn h0(world: &mut World, s: &mut u64) -> u8 {\n${body}${rnd(9)}u8\n}\n`; }
  { const env = [{ id: uid++, n: 'x', t: 'M', st: 'live', mut: false, frozen: false, whole: true, blk: -1 }]; const body = block(env, 2, 5, { helper: true, h1: true }); const x = env.find(v => v.n === 'x' && v.blk === -1 && v.st === 'live' && v.whole); src += `fn h1(world: &mut World, s: &mut u64, x: M) -> M {\n${body}${x ? 'x' : `mk(world, ${marks++}u8)`}\n}\n`; }
  { const body = block([], 3, 9 + rnd(8), {}); src += `pub fn main(mut world: World) {\nlet mut s: u64 = ${1 + rnd(60000)}u64;\n${body}}\n`; }
  return src;
}
const dir = path.resolve(outArg, `drops-${seedS}`); fs.mkdirSync(dir, { recursive: true });
const N = Number(countS); const srcs = []; const ranges = [];
const host = '#![allow(warnings)]\nmod __lkjscript_host {\n' + fs.readFileSync(path.join(repo, 'bridge/output.rs'), 'utf8') + '\n}\n';
for (let i = 0; i < N; i++) { let s = null; while (!s) s = gen(); srcs.push(s); fs.writeFileSync(`${dir}/c${i}.rs`, s); }
function oracle(skip) {
  let text = host; let line = text.split('\n').length; ranges.length = 0;
  for (let i = 0; i < N; i++) { if (skip.has(i)) { ranges.push([0, 0]); continue; } const body = `mod c${i} { use crate::__lkjscript_host::{World,Stdout};\n${srcs[i]}}\n`; const n = body.split('\n').length - 1; ranges.push([line, line + n - 1]); line += n; text += body; }
  text += 'fn main() { match std::env::var("LKJSCRIPT_ORACLE_CASE").unwrap().as_str() {\n' + srcs.map((_, i) => skip.has(i) ? '' : `"${i}" => __lkjscript_host::enter(c${i}::main),\n`).join('') + '_ => std::process::exit(2) } }\n';
  fs.writeFileSync(`${dir}/oracle.rs`, text);
  return spawnSync('rustc', ['--edition=2024', '--cfg', 'lkjscript_control', '--error-format=short', '-C', 'overflow-checks=yes', '-A', 'warnings', `${dir}/oracle.rs`, '-o', `${dir}/oracle`], { encoding: 'utf8', maxBuffer: 1 << 28 });
}
const referenceData=await oracleCache({generator:generatorHash,seed:seedS,count:N,oneSided:ONE_SIDED,host,source:srcs},async()=>{
const invalid = new Map(); let rc = oracle(invalid);
if (rc.status !== 0) { for (const l of rc.stderr.split('\n')) { const m = l.match(/oracle\.rs:(\d+):\d+: error(?:\[(E\d+)\])?/); if (!m) continue; const i = ranges.findIndex(r => Number(m[1]) >= r[0] && Number(m[1]) <= r[1]); if (i >= 0 && !invalid.has(i)) invalid.set(i, m[2] || 'error'); } if (!invalid.size) { console.log(rc.stderr.slice(0, 3000)); process.exit(1); } rc = oracle(invalid); if (rc.status !== 0) { console.log('oracle still fails:\n' + rc.stderr.slice(0, 3000)); process.exit(1); } }

const outputs=[];for(let i=0;i<N;i++)outputs.push(invalid.has(i)?null:packResult(await run(dir+'/oracle',[],{encoding:null,timeout:20000,captureControl:true,env:{...process.env,LKJSCRIPT_ORACLE_CASE:String(i)}})));return {invalid:[...invalid],outputs};
});
const invalid=new Map(referenceData.invalid);
const actualResults=await runRoutes(bin,srcs.map((_,i)=>({path:dir+'/c'+i+'.rs'})),{engines,output:dir,encoding:null,timeout:30000});
const tally = Object.fromEntries(engines.map(engine=>[engine,{}])); const show = []; const bump = (o, c) => { o[c] = (o[c] || 0) + 1; }; let generatorInvalid = 0;
const hex = b => Buffer.from(b).toString('hex').replace(/(..)/g, '$1 ').trim();
for (let i = 0; i < N; i++) {
  if (invalid.has(i)) { generatorInvalid++; show.push(`${dir}/c${i}.rs generator-invalid ${invalid.get(i)}`); continue; }
  const r = unpackResult(referenceData.outputs[i],null), outcome = worldOutcome(r);
  for (const engine of engines) {
    const l = actualResults[engine][i]; const err = (l.stderr || '').toString();
    const code = err.match(/\b(E_[A-Z_]+|M_[A-Z_]+|R\d{4})\b/)?.[1];
    let cls;
    if (outcome.kind !== 'normal' || outcome.status !== 0) cls = 'oracle-abnormal';
    else if (l.error || l.signal) cls = l.error?.code === 'ETIMEDOUT' ? 'timeout' : 'crash';
    else if (l.status === 0) cls = Buffer.compare(l.stdout, r.stdout) === 0 ? 'equal-output' : 'DIFFERENT-OUTPUT';
    else if (code && code.startsWith('E_')) cls = 'valid-but-rejected:' + code;
    else if (code && code.startsWith('M_')) cls = 'MACHINE-ERROR:' + code;
    else if (code) cls = 'TRAP:' + code; else cls = 'crash';
    bump(tally[engine], cls);
    if (cls !== 'equal-output' && !(engine === 'interpreter' && cls.startsWith('valid-but-rejected'))) show.push(`${dir}/c${i}.rs ${engine} ${cls}` + (cls === 'DIFFERENT-OUTPUT' ? `\n  rustc ${hex(r.stdout)}\n  lkjscript   ${hex(l.stdout)}` : ` ${err.split('\n')[0].replace(/^\S*\.rs:/, '').slice(0, 160)}`));
  }
}
const sh = (c, a) => (spawnSync(c, a, { encoding: 'utf8' }).stdout || '').trim();
const record={generator:'drops',sha256:generatorHash,compiler:sh('git',['-C',repo,'rev-parse','--short','HEAD']),binarySha256,rustc:sh('rustc',['--version']),nodeVersion:process.version,seed:seedS,programs:N,oneSided:ONE_SIDED,generatorInvalid,...tally};
console.log(JSON.stringify(record));fs.appendFileSync(path.join(outArg,'runs.jsonl'),JSON.stringify(record)+'\n');
if(generatorInvalid||Object.values(tally).some(classes=>Object.keys(classes).some(key=>key!=='equal-output')))process.exitCode=1;
fs.writeFileSync(`${dir}/report.txt`, show.join('\n') + '\n');
