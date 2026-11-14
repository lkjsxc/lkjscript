const fs=require('fs');
const L=n=>`(local ${n})`, I=n=>`(i64 ${n})`, C=(n,...args)=>`(call ${n} ${args.join(' ')})`;
const F=(at,n)=>C('flow::field',L('data'),at,I(n));
const add=(a,b)=>C('ci::add',a,b),sub=(a,b)=>C('ci::sub',a,b),mul=(a,b)=>C('ci::mul',a,b);
const eq=(a,b)=>C('ci::eq',a,b),lt=(a,b)=>C('ci::lt',a,b);
const iff=(p,a,b)=>{if([p,a,b].includes(undefined))throw Error('Missing if branch');return `(if ${p} ${a} ${b})`;};
const yes='(bool true)',no='(bool false)';
const both=(...ps)=>ps.reduceRight((b,a)=>iff(a,b,no),yes);
const either=(...ps)=>ps.reduceRight((b,a)=>iff(a,yes,b),no);
const guard=(p,body,fail=I(-1))=>iff(p,body,fail);
const lets=(xs,body)=>`(let ${xs.map(([n,v])=>`(binding ${n} ${v})`).join('\n')} (in ${body}))`;
const fun=(n,ps,ret,body)=>`(function create ${n} (visibility public) (effect pure) ${ps.map(p=>`(parameter create ${p} (type ${p==='data'?'Bytes':'I64'}))`).join(' ')} (returns ${ret}) (body ${body}))`;
const notzero=(a,b)=>iff(C('zeros',L('data'),a,b),no,yes);
const extent=(a,n,w)=>C('extent',L('data'),a,n,I(w));
const choose=(cases,base)=>cases.reduceRight((b,[p,a])=>iff(p,a,b),base);
const tailBoth=(...ps)=>ps.slice(0,-1).reduceRight((b,a)=>iff(a,b,no),ps.at(-1)||yes);
let forms=[];
const u=n=>C('ci::u32',L('data'),typeof n==='number'?I(n):n);
const z=(a,b)=>C('artifact-check::zeros',L('data'),a,b);
const nz=(a,b)=>iff(z(a,b),no,yes);
const ex=(a,n,w)=>C('artifact-check::extent',L('data'),a,n,I(w));
const neg=a=>lt(a,I(0));
const range=(v,lo,hi)=>both(lt(I(lo-1),v),lt(v,I(hi+1)));
const field=(a,n)=>u(add(a,I(n)));
forms.push(fun('bytes-order',['data','a','b','count'],'I64',iff(eq(L('count'),I(0)),I(0),lets([
 ['x',C('ci::get',L('data'),L('a'))],['y',C('ci::get',L('data'),L('b'))]],
 iff(eq(L('x'),L('y')),C('bytes-order',L('data'),add(L('a'),I(1)),add(L('b'),I(1)),sub(L('count'),I(1))),iff(lt(L('x'),L('y')),I(-1),I(1)))))));
forms.push(fun('rank',['tag'],'I64',choose([[eq(L('tag'),I(25)),I(14)],[eq(L('tag'),I(23)),I(15)]],L('tag'))));
forms.push(fun('object',['data','domain','digest','lo','hi'],'I64',iff(lt(L('lo'),L('hi')),lets([
 ['mid',C('ci::div',add(L('lo'),L('hi')),I(2))],['row',add(u(80),mul(L('mid'),I(48)))],
 ['a',C('rank',u(L('row')))],['b',C('rank',L('domain'))],['order',iff(eq(L('a'),L('b')),C('bytes-order',L('data'),add(L('row'),I(16)),L('digest'),I(32)),iff(lt(L('a'),L('b')),I(-1),I(1))) ]],
 choose([[eq(L('order'),I(0)),L('row')],[neg(L('order')),C('object',L('data'),L('domain'),L('digest'),add(L('mid'),I(1)),L('hi'))]],C('object',L('data'),L('domain'),L('digest'),L('lo'),L('mid')))),I(-1))));
forms.push(fun('has',['data','domain','digest'],'Bool',lt(I(-1),C('object',L('data'),L('domain'),L('digest'),I(0),u(84)))));
forms.push(fun('owner-order',['data','a','a-id','b','b-id'],'I64',lets([['x',u(L('a'))],['y',u(L('b'))]],iff(eq(L('x'),L('y')),C('bytes-order',L('data'),L('a-id'),L('b-id'),I(16)),iff(lt(L('x'),L('y')),I(-1),I(1))))));
forms.push(fun('owner',['data','unit','lo','hi'],'I64',iff(lt(L('lo'),L('hi')),lets([
 ['mid',C('ci::div',add(L('lo'),L('hi')),I(2))],['row',add(add(u(88),I(312)),mul(L('mid'),I(56)))],['order',C('owner-order',L('data'),L('row'),add(L('row'),I(8)),L('unit'),add(L('unit'),I(32)))]],
 choose([[eq(L('order'),I(0)),L('row')],[neg(L('order')),C('owner',L('data'),L('unit'),add(L('mid'),I(1)),L('hi'))]],C('owner',L('data'),L('unit'),L('lo'),L('mid')))),I(-1))));
forms.push(fun('units',['data','i','next','previous'],'I64',iff(eq(L('i'),u(76)),L('next'),lets([
 ['row',add(I(160),mul(L('i'),I(8)))],['offset',u(L('row'))],['size',field(L('row'),4)],['end',ex(L('offset'),L('size'),1)]],
 guard(both(eq(L('offset'),L('next')),lt(I(335),L('size')),lt(I(-1),L('end'))),
 guard(both(eq(C('bytes-order',L('data'),add(L('offset'),I(16)),add(u(88),I(16)),I(16)),I(0)),
 iff(eq(L('i'),I(0)),yes,eq(C('owner-order',L('data'),L('previous'),add(L('previous'),I(32)),L('offset'),add(L('offset'),I(32))),I(-1))),
 C('artifact-check::unit',C('ci::slice',L('data'),L('offset'),L('end')))),
 lets([['owner',C('owner',L('data'),L('offset'),I(0),field(u(88),304))]],guard(iff(neg(L('owner')),yes,eq(field(L('owner'),4),field(L('offset'),4))),C('units',L('data'),add(L('i'),I(1)),L('end'),L('offset'))))))))));
const contexts=[[1,'lkjscript.kernel.owner-object.v14'],[2,'lkjscript.kernel.type-object.v10'],[3,'lkjscript.kernel.blob-object.v5'],[5,'lkjscript.persistent-map-page.v2'],[25,'lkjscript.kernel.package-revision.v1'],[23,'lkjscript.package-interface-owner.v7']];
forms.push(fun('context',['domain'],'Text',choose(contexts.map(([n,s])=>[eq(L('domain'),I(n)),`(text "${s}")`]),'(text "")')));
forms.push(fun('objects',['data','i','next'],'Bool',iff(eq(L('i'),u(84)),eq(L('next'),C('ci::length',L('data'))),lets([
 ['row',add(u(80),mul(L('i'),I(48)))],['domain',u(L('row'))],['offset',field(L('row'),8)],['size',field(L('row'),12)],['end',ex(L('offset'),L('size'),1)]],
 tailBoth(either(...contexts.map(([n])=>eq(L('domain'),I(n)))),eq(field(L('row'),4),I(0)),eq(L('offset'),L('next')),lt(I(-1),L('end')),
 iff(eq(L('i'),I(0)),yes,eq(C('artifact-order::compare',L('data'),sub(L('row'),I(48)),L('data'),L('row')),I(-1))),
 C('ci::same',C('ci::slice',L('data'),add(L('row'),I(16)),add(L('row'),I(48))),C('wire::digest',C('context',L('domain')),C('ci::slice',L('data'),L('offset'),L('end')))),
 C('objects',L('data'),add(L('i'),I(1)),L('end')))))));
const ownerKinds=[[1,[4,5,8]],[2,[10]],[6,[14]],[10,[18]],[11,[19]]];
forms.push(fun('owners',['data','at','end'],'Bool',iff(eq(L('at'),L('end')),yes,lets([
 ['owner',u(L('at'))],['kind',field(L('at'),4)]],tailBoth(
 either(...ownerKinds.map(([o,ks])=>both(eq(L('owner'),I(o)),either(...ks.map(k=>eq(L('kind'),I(k))))))),nz(add(L('at'),I(8)),add(L('at'),I(24))),
 iff(eq(L('at'),add(u(88),I(312))),yes,eq(C('owner-order',L('data'),sub(L('at'),I(56)),sub(L('at'),I(48)),L('at'),add(L('at'),I(8))),I(-1))),
 C('has',L('data'),I(1),add(L('at'),I(24))),C('owners',L('data'),add(L('at'),I(56)),L('end')))))));
forms.push(`(function create interface (visibility public) (effect pure)
(parameter create data (type Bytes)) (returns Bool)
(body (let (binding at (call ci::u32 (local data) (i64 88)))
 (binding a (type ByteBuffer) (call ci::empty))
 (binding b (type ByteBuffer) (call ci::push (i64 1) (local a)))
 (binding c (type ByteBuffer) (call ci::push (i64 1) (local b)))
 (binding d (type ByteBuffer) (call ci::push (i64 18) (local c)))
 (binding e (type ByteBuffer) (call wire::range (local data) (call ci::add (local at) (i64 16)) (call ci::add (local at) (i64 32)) (local d)))
 (binding f (type ByteBuffer) (call ci::push (i64 1) (local e)))
 (binding g (type ByteBuffer) (call wire::range (local data) (call ci::add (local at) (i64 200)) (call ci::add (local at) (i64 232)) (local f)))
 (binding h (type ByteBuffer) (call wire::unsigned (call flow::field (local data) (local at) (i64 192)) (local g)))
 (binding encoded (call wire::envelope (text "LKJPIFI1") (text "lkjscript.package-interface-identity.v1") (call ci::freeze (local h))))
 (in (call ci::same
  (call ci::slice (local data) (call ci::add (local at) (i64 128)) (call ci::add (local at) (i64 160)))
  (call wire::digest (text "lkjscript.kernel.package-interface.v1") (local encoded)))))))`);
forms.push(fun('metadata',['data'],'Bool',lets([['at',u(88)],['end',add(L('at'),u(92))]],both(
 nz(L('at'),add(L('at'),I(16))),nz(add(L('at'),I(16)),add(L('at'),I(32))),
 z(add(L('at'),I(196)),add(L('at'),I(200))),z(add(L('at'),I(268)),add(L('at'),I(272))),z(add(L('at'),I(308)),add(L('at'),I(312))),
 lt(field(L('at'),192),I(100001)),lt(field(L('at'),264),I(100001)),
 C('has',L('data'),I(25),add(L('at'),I(32))),C('interface',L('data')),
 C('has',L('data'),I(5),add(L('at'),I(160))),C('has',L('data'),I(5),add(L('at'),I(232))),
 C('owners',L('data'),add(L('at'),I(312)),L('end'))))));
forms.push(fun('layout',['data'],'I64',lets([
 ['units',u(76)],['objects',u(84)],['rows',add(I(160),mul(L('units'),I(8)))],['meta',add(L('rows'),mul(L('objects'),I(48)))],['end',ex(L('meta'),u(92),1)]],
 guard(both(eq(u(72),I(160)),eq(L('units'),u(24)),range(L('objects'),1,100000),eq(u(80),L('rows')),eq(u(88),L('meta')),lt(I(311),u(92)),lt(I(-1),L('end')),z(I(96),I(112)),eq(u(112),I(1)),z(I(116),I(160))),
 guard(both(range(field(L('meta'),304),0,100000),eq(u(92),add(I(312),mul(field(L('meta'),304),I(56))))),
 lets([['fuel',add(I(4096),add(mul(C('ci::length',L('data')),I(128)),add(mul(mul(L('units'),L('units')),I(64)),mul(L('objects'),I(64))))) ]],
 guard(both(eq(u(36),I(0)),lt(u(32),I(2147483648)),lt(sub(L('fuel'),I(1)),u(32))),
 lets([['after',C('units',L('data'),I(0),L('end'),I(0))]],
 guard(lt(I(-1),L('after')),guard(C('objects',L('data'),I(0),L('after')),iff(C('metadata',L('data')),I(0),I(10)),I(11)),I(10))),I(6))),I(10)),I(10)))));
forms.push(fun('validate',['data'],'I64',lets([['length',C('ci::length',L('data'))]],
 guard(range(L('length'),160,8388608),guard(C('ci::same',C('ci::slice',L('data'),I(0),I(8)),C('ci::text-bytes','(text "LKJCI001")')),
 guard(eq(u(8),I(4)),guard(both(eq(u(12),I(0)),C('ci::word-is',L('data'),I(16),L('length')),eq(u(28),I(0)),range(u(24),1,700)),
 guard(C('ci::same',C('ci::slice',L('data'),I(40),I(72)),C('ci::digest',C('ci::concat',C('ci::slice',L('data'),I(0),I(40)),C('ci::slice',L('data'),I(72),L('length'))))),C('layout',L('data')),I(4)),I(3)),I(2)),I(1)),I(3)))));
fs.writeFileSync('.artifacts/selfhost/artifact-input-draft.lkjc','declarations.begin\n(units (module create artifact-input\n'+forms.join('\n')+'\n))\ndeclarations.end\n');
