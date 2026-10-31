const fs=require('fs');
const L=n=>`(local ${n})`, I=n=>`(i64 ${n})`, C=(n,...a)=>`(call ${n} ${a.join(' ')})`;
const field=(at,off)=>C('flow::field',L('data'),at,I(off));
const add=(a,b)=>C('ci::add',a,b), mul=(a,b)=>C('ci::mul',a,b);
const eq=(a,b)=>C('ci::eq',a,b), iff=(cond,a,b)=>`(if ${cond} ${a} ${b})`;
const letin=(bs,body)=>`(let ${bs.map(([n,x,t])=>`(binding ${n}${t?' (type '+t+')':''} ${x})`).join('\n')} (in ${body}))`;
const B=(n,x)=>[n,x,'ByteBuffer'];
const f=(n,ps,ret,body)=>`(function create ${n} (visibility public) (effect pure) ${ps.map(([p,t='I64'])=>`(parameter create ${p} (type ${t})${t==='ByteBuffer'?' (use consume)':''})`).join(' ')} (returns ${ret}) (body ${body}))`;
const range=(s,e,o)=>C('wire::range',L('data'),s,e,o);
const uint=(v,o)=>C('wire::unsigned',v,o);
const push=(v,o)=>C('ci::push',v,o);
const rawvec=(ptr,count,stride,out)=>range(ptr,add(ptr,mul(count,I(stride))),uint(count,out));
let a=[];
a.push(f('identities',[['data','Bytes'],['at'],['end'],['stride'],['tag'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',push(L('tag'),L('out'))),B('b',range(L('at'),add(L('at'),L('stride')),L('a')))
 ],C('identities',L('data'),add(L('at'),L('stride')),L('end'),L('stride'),L('tag'),L('b'))))));
a.push(f('references',[['data','Bytes'],['at'],['end'],['tag'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',push(I(18),L('out'))),B('b',range(L('at'),add(L('at'),I(16)),L('a'))),
 B('c',push(L('tag'),L('b'))),B('d',range(add(L('at'),I(16)),add(L('at'),I(32)),L('c')))
 ],C('references',L('data'),add(L('at'),I(32)),L('end'),L('tag'),L('d'))))));
a.push(f('integer',[['data','Bytes'],['at']],'I64',letin([
 ['lo',field(L('at'),0)],['hi',field(L('at'),4)]
],add(mul(iff(C('ci::lt',L('hi'),I(2147483648)),L('hi'),C('ci::sub',L('hi'),I(4294967296))),I(4294967296)),L('lo')))));
a.push(f('string',[['data','Bytes'],['at'],['out','ByteBuffer']],'ByteBuffer',letin([
 ['offset',field(L('at'),0)],['length',field(L('at'),4)],B('a',uint(L('length'),L('out')))
],range(L('offset'),add(L('offset'),L('length')),L('a')))));
a.push(f('numbers',[['data','Bytes'],['at'],['end'],['selector'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',iff(eq(L('selector'),I(0)),L('out'),push(I(1),L('out')))),
 B('b',uint(field(L('at'),0),L('a')))
 ],C('numbers',L('data'),add(L('at'),I(4)),L('end'),L('selector'),L('b'))))));
a.push(f('strings',[['data','Bytes'],['at'],['end'],['text'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',iff(eq(L('text'),I(0)),L('out'),push(I(0),L('out')))),
 B('b',C('string',L('data'),L('at'),L('a')))
 ],C('strings',L('data'),add(L('at'),I(8)),L('end'),L('text'),L('b'))))));
a.push(f('tables',[['data','Bytes'],['at'],['i'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('i'),I(10)),L('out'),letin([
 ['offset',field(L('at'),0)],['count',field(L('at'),4)],B('a',uint(L('count'),L('out'))),
 B('b',iff(C('ci::lt',L('i'),I(7)),
 iff(eq(L('i'),I(6)),C('identities',L('data'),L('offset'),add(L('offset'),mul(L('count'),I(32))),I(32),I(2),L('a')),
 C('references',L('data'),L('offset'),add(L('offset'),mul(L('count'),I(32))),
 iff(eq(L('i'),I(0)),I(3),iff(eq(L('i'),I(1)),I(4),iff(eq(L('i'),I(2)),I(5),iff(eq(L('i'),I(3)),I(10),iff(eq(L('i'),I(4)),I(6),I(11)))))),L('a'))),
 iff(eq(L('i'),I(8)),L('a'),C('strings',L('data'),L('offset'),add(L('offset'),mul(L('count'),I(8))),iff(eq(L('i'),I(9)),I(1),I(0)),L('a')))))
 ],C('tables',L('data'),add(L('at'),I(8)),add(L('i'),I(1)),L('b'))))));
a.push(f('parameters',[['data','Bytes'],['at'],['end'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',range(L('at'),add(L('at'),I(16)),push(I(7),L('out')))),
 B('b',C('string',L('data'),add(L('at'),I(16)),L('a'))),
 B('c',uint(field(L('at'),24),L('b'))),B('d',uint(field(L('at'),28),L('c'))),B('e',push(I(0),L('d')))
 ],C('parameters',L('data'),add(L('at'),I(32)),L('end'),L('e'))))));
a.push(f('signature',[['data','Bytes'],['at'],['out','ByteBuffer']],'ByteBuffer',letin([
 ['types',field(L('at'),0)],['type-count',field(L('at'),4)],['params',field(L('at'),8)],['param-count',field(L('at'),12)],
 B('a',C('ci::le',I(0),I(4),L('out'))),
 B('b',C('identities',L('data'),L('types'),add(L('types'),mul(L('type-count'),I(16))),I(16),I(17),uint(L('type-count'),L('a')))),
 B('c',uint(L('type-count'),L('b'))),
 B('d',C('ci::pad',add(C('ci::size',L('c')),L('type-count')),I(0),L('c'))),
 B('e',uint(L('param-count'),L('d'))),
 B('g',C('parameters',L('data'),L('params'),add(L('params'),mul(L('param-count'),I(32))),L('e'))),
 B('h',uint(field(L('at'),16),L('g')))
],C('ci::le',I(0),I(4),L('h')))));
a.push(f('source',[['data','Bytes'],['at']],'Bytes',letin([
 B('a',range(add(L('at'),I(16)),add(L('at'),I(32)),push(I(18),C('ci::empty')))),
 B('b',uint(field(L('at'),0),L('a'))),
 B('c',range(add(L('at'),I(32)),add(L('at'),I(48)),push(iff(eq(field(L('at'),0),I(1)),I(3),I(12)),L('b')))),
 B('d',uint(field(L('at'),4),L('c'))),
 B('e',C('identities',L('data'),add(L('at'),I(48)),add(L('at'),I(208)),I(32),I(5),L('d'))),
 B('g',push(I(0),L('e'))),
 B('h',range(add(L('at'),I(208)),add(L('at'),I(240)),push(I(5),L('g'))))
],C('ci::freeze',L('h')))));
const extravec=(sel,o)=>C('numbers',L('data'),L('extra'),add(L('extra'),mul(L('count'),I(4))),I(sel),uint(L('count'),o));
a.push(f('instruction',[['data','Bytes'],['at'],['out','ByteBuffer']],'ByteBuffer',letin([
 ['tag',field(L('at'),0)],['x',field(L('at'),4)],['y',field(L('at'),8)],['extra',field(L('at'),24)],['count',field(L('at'),28)],B('a',uint(L('tag'),L('out')))
],iff(eq(L('tag'),I(4)),C('wire::signed',C('integer',L('data'),add(L('at'),I(16))),L('a')),
iff(eq(L('tag'),I(7)),uint(L('y'),uint(L('x'),L('a'))),
iff(eq(L('tag'),I(12)),letin([
 B('b',C('ci::le',I(0),I(2),L('a'))),B('c',uint(L('x'),L('b'))),B('d',extravec(0,L('c'))),B('e',uint(L('y'),L('d')))
],push(I(0),L('e'))),
iff(eq(L('tag'),I(15)),extravec(1,C('ci::le',I(0),I(2),L('a'))),
iff(eq(L('tag'),I(17)),uint(L('x'),push(I(1),L('a'))),
iff(eq(L('tag'),I(18)),uint(L('y'),uint(L('x'),L('a'))),
iff(C('ci::either',eq(L('tag'),I(2)),C('ci::either',eq(L('tag'),I(9)),eq(L('tag'),I(27)))),L('a'),uint(L('x'),L('a'))))))))))));
a.push(f('instructions',[['data','Bytes'],['at'],['end'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([B('next',C('instruction',L('data'),L('at'),L('out')))],
 C('instructions',L('data'),add(L('at'),I(32)),L('end'),L('next'))))));
a.push(f('code',[['data','Bytes'],['at'],['out','ByteBuffer']],'ByteBuffer',letin([
 ['offset',field(L('at'),8)],['count',field(L('at'),12)],B('a',uint(field(L('at'),0),L('out'))),B('b',uint(field(L('at'),4),L('a'))),B('c',uint(L('count'),L('b')))
],C('instructions',L('data'),L('offset'),add(L('offset'),mul(L('count'),I(32))),L('c')))));
a.push(f('ports',[['data','Bytes'],['at'],['end'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',uint(field(L('at'),0),L('out'))),B('b',uint(field(L('at'),4),L('a'))),B('c',push(I(0),L('b'))),B('d',uint(field(L('at'),12),L('c')))
 ],C('ports',L('data'),add(L('at'),I(16)),L('end'),L('d'))))));
a.push(f('payload',[['data','Bytes'],['at'],['out','ByteBuffer']],'ByteBuffer',letin([
 ['tag',field(L('at'),248)],['payload',field(L('at'),252)],B('a',uint(L('tag'),L('out')))
],iff(C('ci::either',eq(L('tag'),I(6)),eq(L('tag'),I(7))),letin([
 B('b',C('signature',L('data'),field(L('at'),244),L('a')))
],iff(eq(L('tag'),I(6)),C('string',L('data'),L('payload'),L('b')),C('code',L('data'),L('payload'),L('b')))),
iff(eq(L('tag'),I(9)),letin([
 ['offset',field(L('payload'),0)],['count',field(L('payload'),4)],B('b',uint(L('count'),push(I(0),L('a'))))
],C('ports',L('data'),L('offset'),add(L('offset'),mul(L('count'),I(16))),L('b'))),
letin([
 B('b',uint(field(L('payload'),0),L('a'))),B('c',push(field(L('payload'),4),L('b'))),
 B('d',iff(eq(field(L('payload'),4),I(0)),L('c'),uint(field(L('payload'),8),L('c')))),
 B('e',push(I(0),L('d')))
],uint(field(L('payload'),12),L('e'))))))));
a.push(f('encode-unit',[['data','Bytes'],['at']],'(record (digest Bytes) (image Bytes) (key Bytes))',letin([
 ['source',C('source',L('data'),L('at'))],
 B('a',C('ci::le',I(65793),I(3),C('ci::empty'))),B('b',C('ci::append',L('source'),I(0),L('a'))),
 ['key-core',C('ci::freeze',push(I(0),L('b')))],
 B('c',C('ci::le',I(1),I(2),C('ci::empty'))),B('d',C('wire::be',C('ci::length',L('key-core')),I(8),L('c'))),
 ['key',C('ci::derive','(text "lkjscript.compiler-unit-key")',C('ci::concat',C('ci::freeze',L('d')),L('key-core')))],
 B('e',C('ci::le',I(65793),I(3),C('ci::empty'))),B('g',C('ci::append',L('key'),I(0),L('e'))),B('h',C('ci::append',L('source'),I(0),L('g'))),B('i',push(I(0),L('h'))),
 B('j',C('tables',L('data'),field(L('at'),240),I(0),L('i'))),B('k',C('payload',L('data'),L('at'),L('j'))),
 ['image',C('wire::envelope','(text "LKJCUNIT")','(text "lkjscript.compiler-unit-envelope")',C('ci::freeze',L('k')))]
],`(record structural (field digest ${C('wire::digest','(text "lkjscript.compiler-unit.v1")',L('image'))}) (field image ${L('image')}) (field key ${L('key')}))`)));
fs.writeFileSync('.artifacts/selfhost/unit-draft.lkjc','declarations.begin\n(units (module create unit-wire\n'+a.join('\n')+'\n))\ndeclarations.end\n');
