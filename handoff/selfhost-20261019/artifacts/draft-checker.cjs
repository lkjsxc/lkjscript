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
let forms=[];
forms.push(fun('indices',['data','at','end','limit'],'Bool',iff(eq(L('at'),L('end')),yes,
 guard(lt(F(L('at'),0),L('limit')),C('indices',L('data'),add(L('at'),I(4)),L('end'),L('limit')),no))));
forms.push(fun('parameters',['data','at','end','next','types'],'I64',iff(eq(L('at'),L('end')),L('next'),
 guard(both(notzero(L('at'),add(L('at'),I(16))),lt(I(0),F(L('at'),20)),lt(F(L('at'),24),L('types')),lt(F(L('at'),28),I(3))),
 lets([['after',C('string',L('data'),add(L('at'),I(16)),L('next'))]],
 guard(lt(I(-1),L('after')),C('parameters',L('data'),add(L('at'),I(32)),L('end'),L('after'),L('types'))))))));
forms.push(fun('signature',['data','at','types'],'I64',guard(lt(I(-1),extent(L('at'),I(1),20)),lets([
 ['tp',F(L('at'),0)],['tc',F(L('at'),4)],['pp',F(L('at'),8)],['pc',F(L('at'),12)]
],guard(both(eq(L('tp'),add(L('at'),I(20))),lt(L('tc'),I(129)),lt(L('pc'),I(129)),lt(F(L('at'),16),L('types'))),
 lets([['te',extent(L('tp'),L('tc'),16)]],guard(both(lt(I(-1),L('te')),eq(L('pp'),L('te'))),
 lets([['pe',extent(L('pp'),L('pc'),32)]],guard(lt(I(-1),L('pe')),C('parameters',L('data'),L('pp'),L('pe'),L('pe'),L('types')))))))))));
const plain=bounds=>guard(both(bounds,eq(L('b'),I(0)),eq(L('extra'),I(0)),eq(L('count'),I(0))),L('next'));
forms.push(fun('instruction',['data','at','index','total','locals','types','declarations','names','texts','next'],'I64',lets([
 ['tag',F(L('at'),0)],['a',F(L('at'),4)],['b',F(L('at'),8)],['extra',F(L('at'),24)],['count',F(L('at'),28)]
],guard(both(eq(F(L('at'),12),I(0)),iff(eq(L('tag'),I(4)),yes,C('ci::word-is',L('data'),add(L('at'),I(16)),I(0))),
 iff(eq(L('tag'),I(27)),eq(L('index'),sub(L('total'),I(1))),lt(L('index'),sub(L('total'),I(1))))),
 choose([
 [either(eq(L('tag'),I(2)),eq(L('tag'),I(4)),eq(L('tag'),I(9)),eq(L('tag'),I(27))),plain(eq(L('a'),I(0)))],
 [eq(L('tag'),I(3)),plain(lt(L('a'),I(2)))],
 [either(eq(L('tag'),I(5)),eq(L('tag'),I(6))),plain(lt(L('a'),L('texts')))],
 [eq(L('tag'),I(7)),guard(both(lt(L('a'),L('locals')),lt(L('b'),I(3)),eq(L('extra'),I(0)),eq(L('count'),I(0))),L('next'))],
 [eq(L('tag'),I(8)),plain(lt(L('a'),L('locals')))],
 [either(eq(L('tag'),I(10)),eq(L('tag'),I(11))),plain(both(lt(L('index'),L('a')),lt(L('a'),L('total'))))],
 [eq(L('tag'),I(17)),plain(lt(L('a'),L('names')))],
 [eq(L('tag'),I(18)),guard(both(lt(L('a'),L('types')),lt(L('b'),I(129)),eq(L('extra'),I(0)),eq(L('count'),I(0))),L('next'))],
 [either(eq(L('tag'),I(12)),eq(L('tag'),I(15))),
 guard(both(eq(L('extra'),L('next')),lt(L('count'),I(129)),iff(eq(L('tag'),I(12)),both(lt(L('a'),L('declarations')),lt(L('b'),I(129))),both(eq(L('a'),I(0)),eq(L('b'),I(0))))),
 lets([['end',extent(L('extra'),L('count'),4)]],guard(lt(I(-1),L('end')),
 guard(C('indices',L('data'),L('extra'),L('end'),iff(eq(L('tag'),I(12)),L('types'),L('names'))),L('end')))))],
 ],I(-1))))));
forms.push(fun('instructions',['data','at','end','index','total','locals','types','declarations','names','texts','next'],'I64',iff(eq(L('at'),L('end')),L('next'),
 lets([['after',C('instruction',...['data','at','index','total','locals','types','declarations','names','texts','next'].map(L))]],
 guard(lt(I(-1),L('after')),C('instructions',L('data'),add(L('at'),I(32)),L('end'),add(L('index'),I(1)),...['total','locals','types','declarations','names','texts','after'].map(L)))))));
forms.push(fun('code',['data','at','parameters','types','declarations','names','texts'],'I64',guard(lt(I(-1),extent(L('at'),I(1),16)),lets([
 ['locals',F(L('at'),4)],['nodes',F(L('at'),8)],['count',F(L('at'),12)]
],guard(both(eq(F(L('at'),0),L('parameters')),lt(L('locals'),I(129)),lt(sub(L('parameters'),I(1)),L('locals')),eq(L('nodes'),add(L('at'),I(16))),lt(I(0),L('count')),lt(L('count'),I(32769))),
 lets([['end',extent(L('nodes'),L('count'),32)]],guard(lt(I(-1),L('end')),
 C('instructions',L('data'),L('nodes'),L('end'),I(0),L('count'),...['locals','types','declarations','names','texts','end'].map(L)))))))));
forms.push(fun('ports',['data','at','end','types','declarations','ports'],'Bool',iff(eq(L('at'),L('end')),yes,
 guard(both(lt(F(L('at'),0),L('ports')),lt(F(L('at'),4),L('types')),eq(F(L('at'),8),I(0)),lt(F(L('at'),12),L('declarations'))),
 C('ports',L('data'),add(L('at'),I(16)),L('end'),...['types','declarations','ports'].map(L)),no))));
forms.push(fun('component',['data','at','types','declarations','ports'],'I64',guard(lt(I(-1),extent(L('at'),I(1),8)),lets([
 ['offset',F(L('at'),0)],['count',F(L('at'),4)]
],guard(both(eq(L('offset'),add(L('at'),I(8))),lt(L('count'),I(129))),lets([
 ['end',extent(L('offset'),L('count'),16)]
],guard(lt(I(-1),L('end')),guard(C('ports',L('data'),L('offset'),L('end'),...['types','declarations','ports'].map(L)),L('end')))))))));
forms.push(fun('target',['data','at','declarations','ports'],'I64',guard(lt(I(-1),extent(L('at'),I(1),16)),
 guard(both(lt(F(L('at'),0),L('declarations')),lt(F(L('at'),4),I(2)),eq(F(L('at'),12),I(0)),
 iff(eq(F(L('at'),4),I(0)),eq(F(L('at'),8),I(0)),lt(F(L('at'),8),L('ports')))),add(L('at'),I(16))))));
const implementations=[...fs.readFileSync('tools/selfhost/input.lkjc','utf8').matchAll(/\(implementation ([^)]+)\)/g)].map(m=>m[1]).concat(['core.i64.less-equal','core.bool.not','core.bytes.copy','core.buffer.get','core.buffer.discard']);
forms.push(`(function create external-name (visibility public) (effect pure) (parameter create value (type Bytes)) (returns Bool) (body ${either(...implementations.map(name=>C('ci::same',L('value'),C('ci::text-bytes',JSON.stringify(name)).replace('"'+name+'"','(text "'+name+'")'))))}))`);
forms.push(fun('external',['data','at'],'I64',guard(lt(I(-1),extent(L('at'),I(1),8)),lets([
 ['end',C('string',L('data'),L('at'),add(L('at'),I(8)))]
],guard(lt(I(-1),L('end')),guard(C('external-name',C('ci::slice',L('data'),add(L('at'),I(8)),L('end'))),L('end')))))));
forms.push(fun('payload',['data','next'],'I64',lets([
 ['tag',F(I(0),248)],['at',F(I(0),252)],['signature',F(I(0),244)],
 ['types',F(I(0),308)],['declarations',F(I(0),260)],['names',F(I(0),316)],['texts',F(I(0),332)],['ports',F(I(0),300)]
],iff(either(eq(L('tag'),I(6)),eq(L('tag'),I(7))),guard(eq(L('signature'),L('next')),lets([
 ['after',C('signature',L('data'),L('signature'),L('types'))]
],guard(both(lt(I(-1),L('after')),eq(L('at'),L('after'))),iff(eq(L('tag'),I(6)),C('external',L('data'),L('at')),
 C('code',L('data'),L('at'),F(L('signature'),12),...['types','declarations','names','texts'].map(L)))))),
 guard(both(eq(L('signature'),I(0)),eq(L('at'),L('next'))),iff(eq(L('tag'),I(9)),C('component',L('data'),L('at'),...['types','declarations','ports'].map(L)),C('target',L('data'),L('at'),L('declarations'),L('ports'))))))));
forms.push(fun('unit',['data'],'Bool',guard(both(lt(I(335),C('ci::length',L('data'))),lt(C('ci::length',L('data')),I(8388609))),lets([
 ['owner',F(I(0),0)],['kind',F(I(0),4)],['tag',F(I(0),248)]
],guard(both(eq(F(I(0),8),add(L('kind'),I(1))),eq(F(I(0),12),I(0)),eq(F(I(0),240),I(256)),notzero(I(16),I(32)),notzero(I(32),I(48)),
 choose([[eq(L('tag'),I(6)),both(eq(L('owner'),I(1)),eq(L('kind'),I(4)))],[eq(L('tag'),I(7)),both(eq(L('owner'),I(1)),eq(L('kind'),I(5)))],[eq(L('tag'),I(9)),both(eq(L('owner'),I(1)),eq(L('kind'),I(8)))],[eq(L('tag'),I(11)),both(eq(L('owner'),I(11)),eq(L('kind'),I(19)))]],no)),
 lets([['next',C('tables',L('data'),I(0),I(336))]],guard(lt(I(-1),L('next')),eq(C('payload',L('data'),L('next')),C('ci::length',L('data'))),no)),no)),no)));
const base=fs.readFileSync('.artifacts/selfhost/artifact-check-draft.lkjc','utf8');
fs.writeFileSync('.artifacts/selfhost/artifact-check-full-draft.lkjc',base.replace(/\)\)\s*declarations.end\s*$/,forms.join('\n')+'\n))\ndeclarations.end\n'));
