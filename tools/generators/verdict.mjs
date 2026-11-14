// Ownership verdict fuzzer: type-correct programs whose moves/borrows are random, so many are invalid.
// rustc gives the reference verdict for this Rust-shaped subset. The report keeps apart: rejected by both for an
// ownership reason (with the pair of codes), rejected by lkjscript for another reason, a crash, a timeout,
// accepted by lkjscript although rustc rejects, valid but rejected, and equal or different results when both run.
// usage: node verdict.mjs <lkjscript-binary> <seed> <count>
import fs from 'node:fs';
import { run } from '../process.mjs';
import { runRoutes } from '../batch.mjs';
import { oracleCache } from '../oracle.mjs';
import { createHash } from 'node:crypto';
import path from 'node:path';
const [binary, seedS = '601', countS = '100', output = 'target/generators'] = process.argv.slice(2);
const bin = path.resolve(binary || 'target/debug/lkjscript');
const engines = (process.env.LKJSCRIPT_ENGINES || process.env.LKJSCRIPT_ENGINE || 'native').split(',');
const generatorHash = createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).update(fs.readFileSync(new URL('../process.mjs', import.meta.url))).digest('hex');
let state = (BigInt(seedS) * 0x9E3779B97F4A7C15n & 0xFFFFFFFFFFFFFFFFn) || 1n;
function rnd(n) { state ^= (state << 13n) & 0xFFFFFFFFFFFFFFFFn; state ^= state >> 7n; state ^= (state << 17n) & 0xFFFFFFFFFFFFFFFFn; return Number(state % BigInt(n)); }
const pick = a => a[rnd(a.length)];
const PRE = `struct Capture<T> { value: T }
fn capture<T>(value: T) -> T { value }
struct Hs { v: Vec<i64>, n: i64 }
struct View<'a> { r: &'a i64, k: i64 }
fn sumr(w: &Vec<i64>) -> i64 { let mut s: i64 = 0; let mut i = 0; while i < w.len() { s = s + w[i]; i = i + 1; } s }
fn sumv(w: Vec<i64>) -> i64 { sumr(&w) }
fn mv(h: Hs) -> Hs { h }
fn first(w: &Vec<i64>) -> &i64 { &w[0] }
fn slot(w: &mut Vec<i64>) -> &mut i64 { &mut w[0] }
fn field(h: &Hs) -> &i64 { &h.n }
fn inner(h: &mut Hs) -> &mut Vec<i64> { &mut h.v }
fn unbox(b: Box<i64>) -> i64 { *b }
fn look(v: View<'_>) -> i64 { *v.r + v.k }
`;
function gen() {
  let uid = 0;
  function stmts(env, d, n) {
    let out = ''; env = env.slice();
    const of = k => env.filter(v => v.k === k);
    for (let i = 0; i < n; i++) {
      const c = rnd(36); const N = uid++;
      const V = of('vec'), B = of('box'), H = of('hs'), R = of('ri'), RV = of('rv'), MV = of('mv'), MI = of('mi'), VW = of('view');
      if (c < 3) { out += `let mut v${N}: Vec<i64> = Vec::new(); v${N}.push(${rnd(9)}); v${N}.push(${rnd(9)});\n`; env.push({ k: 'vec', n: `v${N}` }); }
      else if (c < 4) { out += `let mut b${N}: Box<i64> = Box::new(${rnd(9)});\n`; env.push({ k: 'box', n: `b${N}` }); }
      else if (c < 6) { out += `let mut h${N}: Hs = Hs { v: Vec::new(), n: ${rnd(9)} }; h${N}.v.push(${rnd(9)});\n`; env.push({ k: 'hs', n: `h${N}` }); }
      else if (c < 8 && V.length) { out += `let r${N}: &i64 = ${pick([`&${pick(V).n}[0]`, `first(&${pick(V).n})`])};\n`; env.push({ k: 'ri', n: `r${N}` }); }
      else if (c < 9 && B.length) { out += `let r${N}: &i64 = &*${pick(B).n};\n`; env.push({ k: 'ri', n: `r${N}` }); }
      else if (c < 10 && H.length) { out += `let r${N}: &i64 = ${pick([`&${pick(H).n}.n`, `field(&${pick(H).n})`, `&${pick(H).n}.v[0]`])};\n`; env.push({ k: 'ri', n: `r${N}` }); }
      else if (c < 11 && V.length) { out += `let r${N}: &Vec<i64> = &${pick(V).n};\n`; env.push({ k: 'rv', n: `r${N}` }); }
      else if (c < 12 && H.length) { out += `let r${N}: &Vec<i64> = &${pick(H).n}.v;\n`; env.push({ k: 'rv', n: `r${N}` }); }
      else if (c < 13 && V.length) { out += `let m${N}: &mut Vec<i64> = &mut ${pick(V).n};\n`; env.push({ k: 'mv', n: `m${N}` }); }
      else if (c < 14 && H.length) { out += `let m${N}: &mut Vec<i64> = ${pick([`&mut ${pick(H).n}.v`, `inner(&mut ${pick(H).n})`])};\n`; env.push({ k: 'mv', n: `m${N}` }); }
      else if (c < 15 && H.length) { out += `let m${N}: &mut i64 = &mut ${pick(H).n}.n;\n`; env.push({ k: 'mi', n: `m${N}` }); }
      else if (c < 16 && (V.length || MV.length)) { out += `let m${N}: &mut i64 = ${V.length && rnd(2) ? `slot(&mut ${pick(V).n})` : MV.length ? `slot(${pick(MV).n})` : `&mut ${pick(V).n}[0]`};\n`; env.push({ k: 'mi', n: `m${N}` }); }
      else if (c < 17 && B.length) { out += `let m${N}: &mut i64 = &mut *${pick(B).n};\n`; env.push({ k: 'mi', n: `m${N}` }); }
      else if (c < 18 && R.length) { out += `let w${N}: View<'_> = View { r: ${pick(R).n}, k: ${rnd(5)} };\n`; env.push({ k: 'view', n: `w${N}` }); }
      else if (c < 20 && R.length) out += `acc = acc + *${pick(R).n};\n`;
      else if (c < 21 && RV.length) out += `acc = acc + ${pick([`sumr(${pick(RV).n})`, `${pick(RV).n}.len() as i64`, `${pick(RV).n}[0]`])};\n`;
      else if (c < 22 && MV.length) out += `${pick(MV).n}.push(${rnd(9)});\n`;
      else if (c < 23 && MI.length) out += `*${pick(MI).n} = *${pick(MI).n} + 1;\n`;
      else if (c < 24 && VW.length) out += `acc = acc + ${pick([`*${pick(VW).n}.r`, `look(${pick(VW).n})`])};\n`;
      else if (c < 25 && V.length) out += pick([`${pick(V).n}.push(${rnd(9)});\n`, `${pick(V).n}[0] = ${rnd(9)};\n`, `${pick(V).n} = Vec::new(); ${pick(V).n}.push(1);\n`]);
      else if (c < 26 && H.length) out += pick([`${pick(H).n}.n = ${rnd(9)};\n`, `${pick(H).n}.v.push(${rnd(9)});\n`, `${pick(H).n}.v = Vec::new();\n`]);
      else if (c < 27 && B.length) out += `*${pick(B).n} = ${rnd(9)};\n`;
      else if (c < 28 && V.length) { out += pick([`acc = acc + sumv(${pick(V).n});\n`, `let v${N}: Vec<i64> = ${pick(V).n};\n`]); }
      else if (c < 29 && H.length) { out += pick([`let h${N}: Hs = mv(${pick(H).n});\n`, `acc = acc + sumv(${pick(H).n}.v);\n`]); }
      else if (c < 30 && B.length) out += `acc = acc + unbox(${pick(B).n});\n`;
      else if (c < 31 && V.length) out += `acc = acc + ${pick([`sumr(&${pick(V).n})`, `${pick(V).n}.len() as i64`])};\n`;
      else if (c < 32 && d > 0) out += `if acc % 2 == ${rnd(2)} {\n${stmts(env, d - 1, 3)}}\n`;
      else if (c < 33 && d > 0) out += `if acc % 3 == ${rnd(3)} {\n${stmts(env, d - 1, 3)}} else {\n${stmts(env, d - 1, 3)}}\n`;
      else if (c === 34 && MI.length) {
        const reference = pick(MI).n;
        const initializer = pick([`Some(${reference})`, `Capture { value: ${reference} }`, `capture(${reference})`]);
        out += `let captured${N} = ${initializer}; drop(captured${N}); *${reference} = 1;\n`;
      }
      else if (c === 35 && MI.length) {
        const reference = pick(MI).n;
        out += `acc = acc + std::mem::replace(${reference}, ${rnd(9)}); *${reference} = *${reference} + 1;\n`;
      }
      else if (d > 0) { out += `let mut k${N}: i64 = 0;\nwhile k${N} < 2 {\nk${N} = k${N} + 1;\n${stmts(env, d - 1, 3)}}\n`; }
    }
    return out;
  }
  return PRE + `pub fn probe() -> i64 {\nlet mut acc: i64 = 0;\n${stmts([], 2, 9 + rnd(8))}acc\n}\n`;
}
const dir = path.resolve(output, `verdict-${seedS}`); fs.mkdirSync(dir, { recursive: true });
const N = Number(countS); const srcs = []; const ranges = [];
let crate = '#![allow(warnings)]\n'; let line = 2;
for (let i = 0; i < N; i++) { const s = gen(); srcs.push(s); fs.writeFileSync(`${dir}/c${i}.rs`, s); const body = `mod c${i} {\n${s}}\n`; const n = body.split('\n').length - 1; ranges.push([line, line + n - 1]); line += n; crate += body; }
fs.writeFileSync(`${dir}/all.rs`, crate + 'fn main() {}\n');
const OWN = new Set(['E0382', 'E0499', 'E0502', 'E0503', 'E0505', 'E0506', 'E0507', 'E0509', 'E0515', 'E0596', 'E0597', 'E0716', 'E0713', 'E0381', 'E0384', 'E0594']);
const rustKind = m => /with overflow/.test(m) ? 'R0001' : /divide by zero|divisor of zero/.test(m) ? 'R0002' : /index out of bounds/.test(m) ? 'R0003' : /assertion/.test(m) ? 'R0005' : 'R0004';
const references=await oracleCache({generator:generatorHash,seed:seedS,count:N,source:crate,flags:'2024 error-format=short overflow-checks=yes'},async()=>{
const first = await run('rustc', ['--edition=2024', '--error-format=short', '-A', 'warnings', '--emit=metadata', '-o', `${dir}/all.rmeta`, `${dir}/all.rs`], { encoding: 'utf8' });
const rejected = new Map();
for (const l of first.stderr.split('\n')) { const m = l.match(/all\.rs:(\d+):\d+: error\[(E\d+)\]/); if (!m) continue; const ln = Number(m[1]); const i = ranges.findIndex(r => ln >= r[0] && ln <= r[1]); if (i >= 0 && !rejected.has(i)) rejected.set(i, m[2]); }
if (first.status !== 0 && rejected.size === 0) { console.log('rustc failed without attributable errors:\n' + first.stderr.slice(0, 2000)); process.exit(1); }
let ok = '#![allow(warnings)]\n';
for (let i = 0; i < N; i++) if (!rejected.has(i)) ok += `mod c${i} {\n${srcs[i]}}\n`;
ok += 'fn main() { let n: usize = std::env::args().nth(1).unwrap().parse().unwrap(); let r = match n {\n' + srcs.map((_, i) => rejected.has(i) ? '' : `${i} => c${i}::probe(),\n`).join('') + '_ => 0 }; println!("{}", r); }\n';
fs.writeFileSync(`${dir}/ok.rs`, ok);
const second = await run('rustc', ['--edition=2024', '-C', 'overflow-checks=yes', '-A', 'warnings', `${dir}/ok.rs`, '-o', `${dir}/ok`], { encoding: 'utf8' });
if (second.status !== 0) { console.log('second rustc build failed:\n' + second.stderr.slice(0, 3000)); process.exit(1); }

const results=[];for(let i=0;i<N;i++){let rust;if(rejected.has(i))rust={k:'reject',code:rejected.get(i)};else{const r=await run(dir+'/ok',[String(i)],{encoding:'utf8'});const m=r.stderr.match(/panicked at [^:\n]+:\d+:\d+:\n(.*)/);rust=r.status===0?{k:'value',v:r.stdout.trim()}:r.status===101&&m?{k:'trap',kind:rustKind(m[1])}:{k:'abnormal'};}if(rust.k==='abnormal')throw new Error('abnormal Rust oracle');results.push(rust);}return results;
});
const actualRoutes=await runRoutes(bin,srcs.map((_,i)=>({path:dir+'/c'+i+'.rs',entry:'probe'})),{engines,output:dir});
for(const engine of engines) {
const actualResults=actualRoutes[engine];
const tally = {}; const pairs = {}; const show = []; const bump = (o, c) => { o[c] = (o[c] || 0) + 1; };
for (let i = 0; i < N; i++) {
  const rust = references[i];
  const l = actualResults[i];
  const text = (l.stdout + l.stderr).trim().split('\n')[0] || '';
  const code = text.match(/\b(E_[A-Z_]+|M_[A-Z_]+|R\d{4})\b/)?.[1];
  const lk = l.error || l.signal ? { k: l.error?.code === 'ETIMEDOUT' ? 'timeout' : 'crash' } : l.status === 0 ? { k: 'value', v: text } : code && code[0] === 'R' ? { k: 'trap', kind: code } : code && code[0] === 'M' ? { k: 'machine-error', code } : code ? { k: 'reject', code } : { k: 'crash' };
  const ownLk = lk.k === 'reject' && (lk.code === 'E_MOVE' || lk.code === 'E_BORROW');
  let cls;
  if (rust.k === 'abnormal' || (rust.k === 'reject' && !OWN.has(rust.code))) cls = 'generator-or-oracle-problem';
  else if (lk.k === 'crash' || lk.k === 'timeout') cls = lk.k;
  else if (lk.k === 'machine-error') cls = 'machine-error:' + lk.code;
  else if (rust.k === 'reject') { if (ownLk) { cls = 'both-reject-ownership'; bump(pairs, rust.code + '/' + lk.code); } else if (lk.k === 'reject') cls = 'rejected-for-another-reason:' + lk.code; else cls = 'ACCEPTS-RUST-REJECTED'; }
  else if (lk.k === 'reject') cls = (lk.code === 'E_UNSUPPORTED' || lk.code === 'E_PARSE') ? 'unsupported' : 'rejects-valid:' + lk.code;
  else if (rust.k === 'value' && lk.k === 'value') cls = rust.v === lk.v ? 'both-run-value-equal' : 'VALUE-DIFFERS';
  else if (rust.k === 'trap' && lk.k === 'trap') cls = rust.kind === lk.kind ? 'both-run-trap-equal' : 'TRAP-KIND-DIFFERS';
  else cls = 'VALUE-VS-TRAP';
  bump(tally, cls);
  if (!cls.startsWith('both-')) show.push(`${dir}/c${i} ${cls} rust=${rust.k}:${rust.code || rust.v || rust.kind || ''} lkjscript=${text.replace(/^\S*\/c\d+\.rs:/, '').slice(0, 140)}`);
}
const record = { generator: 'verdict', engine, rustFlags: { verdict: '--edition=2024 --error-format=short -A warnings --emit=metadata', execution: '--edition=2024 -C overflow-checks=yes -A warnings' }, seed: seedS, programs: N, ...tally, rejectCodePairs: pairs };
fs.mkdirSync(output, {recursive: true});
console.log(JSON.stringify(record)); fs.writeFileSync(`${dir}/report-${engine}.txt`, show.join('\n') + '\n');

const fatal = Object.keys(tally).some(key => /DIFFERS|VALUE-VS-TRAP|ACCEPTS-RUST-REJECTED|crash|timeout|oracle|generator|other-line|machine-error/.test(key));
if (fatal) process.exitCode = 1;
}
