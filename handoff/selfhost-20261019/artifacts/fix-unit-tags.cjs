const fs=require('fs');const p='.artifacts/selfhost/draft-unit.cjs';let s=fs.readFileSync(p,'utf8');
s=s.replace('let a=[];',`let a=[];
a.push(f('identities',[['data','Bytes'],['at'],['end'],['stride'],['tag'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',push(L('tag'),L('out'))),B('b',range(L('at'),add(L('at'),L('stride')),L('a')))
 ],C('identities',L('data'),add(L('at'),L('stride')),L('end'),L('stride'),L('tag'),L('b'))))));
a.push(f('references',[['data','Bytes'],['at'],['end'],['tag'],['out','ByteBuffer']],'ByteBuffer',
 iff(eq(L('at'),L('end')),L('out'),letin([
 B('a',push(I(18),L('out'))),B('b',range(L('at'),add(L('at'),I(16)),L('a'))),
 B('c',push(L('tag'),L('b'))),B('d',range(add(L('at'),I(16)),add(L('at'),I(32)),L('c')))
 ],C('references',L('data'),add(L('at'),I(32)),L('end'),L('tag'),L('d'))))));`);
s=s.replace("B('b',iff(C('ci::lt',L('i'),I(7)),range(L('offset'),add(L('offset'),mul(L('count'),I(32))),L('a')),",`B('b',iff(C('ci::lt',L('i'),I(7)),
 iff(eq(L('i'),I(6)),C('identities',L('data'),L('offset'),add(L('offset'),mul(L('count'),I(32))),I(32),I(2),L('a')),
 C('references',L('data'),L('offset'),add(L('offset'),mul(L('count'),I(32))),
 iff(eq(L('i'),I(0)),I(3),iff(eq(L('i'),I(1)),I(4),iff(eq(L('i'),I(2)),I(5),iff(eq(L('i'),I(3)),I(10),iff(eq(L('i'),I(4)),I(6),I(11)))))),L('a'))),`);
s=s.replace("B('a',range(L('at'),add(L('at'),I(16)),L('out'))),","B('a',range(L('at'),add(L('at'),I(16)),push(I(7),L('out')))),");
s=s.replace("B('b',rawvec(L('types'),L('type-count'),16,L('a'))),","B('b',C('identities',L('data'),L('types'),add(L('types'),mul(L('type-count'),I(16))),I(16),I(17),uint(L('type-count'),L('a')))),");
s=s.replace("B('a',range(add(L('at'),I(16)),add(L('at'),I(32)),C('ci::empty'))),","B('a',range(add(L('at'),I(16)),add(L('at'),I(32)),push(I(18),C('ci::empty')))),");
s=s.replace("B('c',range(add(L('at'),I(32)),add(L('at'),I(48)),L('b'))),","B('c',range(add(L('at'),I(32)),add(L('at'),I(48)),push(iff(eq(field(L('at'),0),I(1)),I(3),I(12)),L('b')))),");
s=s.replace("B('e',range(add(L('at'),I(48)),add(L('at'),I(208)),L('d'))),","B('e',C('identities',L('data'),add(L('at'),I(48)),add(L('at'),I(208)),I(32),I(5),L('d'))),");
s=s.replace("B('h',range(add(L('at'),I(208)),add(L('at'),I(240)),L('g')))","B('h',range(add(L('at'),I(208)),add(L('at'),I(240)),push(I(5),L('g'))))");
fs.writeFileSync(p,s);
