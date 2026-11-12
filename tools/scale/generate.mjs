import fs from 'node:fs';
import { pathToFileURL } from 'node:url';
export function generate(kind, n) {
if (!Number.isSafeInteger(n) || n < 1) throw new Error('size must be a positive safe integer');
let s = '';
if (kind === 'funcs') { for (let i = 0; i < n; i++) s += `fn f${i}(x: i64) -> i64 { ${i ? `f${i-1}(x) + 1` : 'x'} }\n`; s += `pub fn probe() -> i64 { f${n-1}(0) }\n`; }
// Many method owners share one nominal type; no call-site search is involved.
if (kind === 'methods') { s += 'struct Owner { value: i64 }\n'; for (let i = 0; i < n; i++) s += `impl Owner { fn method${i}() -> i64 { ${i} } }\n`; s += 'pub fn probe() -> i64 { 42 }\n'; }
if (kind === 'ifs') { s += 'pub fn probe() -> i64 { let mut s: i64 = 0;\n'; for (let i = 0; i < n; i++) s += `let a${i}: i64 = ${i}; if a${i} % 2 == 0 { s = s + 1; }\n`; s += 's }\n'; }
if (kind === 'structs') { for (let i = 0; i < n; i++) s += `struct S${i} { a: i64, b: ${i ? `S${i-1}` : 'i64'} }\n`; s += 'pub fn probe() -> i64 { 1 }\n'; }
if (kind === 'boxes') { s += 'pub fn probe() -> i64 { let mut s: i64 = 0;\n'; for (let i = 0; i < n; i++) s += `let b${i}: Box<i64> = Box::new(${i}); let r${i}: &i64 = &*b${i}; s = s + *r${i};\n`; s += 's }\n'; }
if (kind === 'expr') { s += 'pub fn probe() -> i64 { ' + '('.repeat(n) + '1' + ' + 1)'.repeat(n) + ' }\n'; }
if (kind === 'match') { s += 'pub fn probe() -> i64 { let x: i64 = 7; match x {\n'; for (let i = 0; i < n; i++) s += `${i} => ${i + 1},\n`; s += '_ => 0 } }\n'; }
if (kind === 'vecs') { s += 'pub fn probe() -> i64 { let mut s: i64 = 0;\n'; for (let i = 0; i < n; i++) s += `let mut v${i}: Vec<i64> = Vec::new(); v${i}.push(${i}); s = s + v${i}[0];\n`; s += 's }\n'; }
if (kind === 'allocvec') s = `pub fn probe() -> i64 { let mut v: Vec<i64> = Vec::new(); let mut i: usize = 0; while i < ${n}usize { v.push(i as i64); i = i + 1; } v[v.len() - 1] }\n`;
if (kind === 'alloclist') s = `enum List { Nil, Cons(i64, Box<List>) } pub fn probe() -> i64 { let mut v: List = List::Nil; let mut i: i64 = 0; while i < ${n} { v = List::Cons(i, Box::new(v)); i = i + 1; } match v { List::Cons(x, rest) => x, List::Nil => 0 } }\n`;
if (!s) throw new Error('unknown scale family: ' + kind);
return s;
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
 const [kind, size, output = `${kind}${size}.rs`] = process.argv.slice(2);
 fs.writeFileSync(output, generate(kind, Number(size)));
}
