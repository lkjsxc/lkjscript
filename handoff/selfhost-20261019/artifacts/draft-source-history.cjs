// Authoring proposals only. These readers do not grant source execution authority.
const fs=require('fs');
const d=require('./source-author.cjs');
const {L,I,C,F,add,sub,eq,lt,iff,yes,no,both,lets,fun,choose,data,at,typed,name,single,tag,chain,rec,save}=d;
const forms=[];
const length=x=>C('ci::length',x),get=p=>C('ci::get',L('data'),p);
const range=(p,n)=>C('source-read::range',L('data'),p,I(n),L('end'));
const keyTags=[2,3,17,4,5,6,7,8,9,10,11,12,15,16,18,19,20];
forms.push(fun('owner-key',at,'I64',iff(range(L('at'),1),lets([['kind',get(L('at'))]],choose(keyTags.map((t,k)=>[eq(L('kind'),I(k)),typed(add(L('at'),I(1)),t)]),I(-1))),I(-1))));
forms.push(fun('optional-name',at,'I64',iff(range(L('at'),1),iff(eq(get(L('at')),I(0)),add(L('at'),I(1)),iff(eq(get(L('at')),I(1)),name(add(L('at'),I(1))),I(-1))),I(-1))));
forms.push(fun('optional-owner',at,'I64',iff(range(L('at'),1),iff(eq(get(L('at')),I(0)),add(L('at'),I(1)),iff(eq(get(L('at')),I(1)),C('owner-key',L('data'),add(L('at'),I(1)),L('end')),I(-1))),I(-1))));
function record(n,magic,max,steps){return fun(n,data,'I64',lets([['end',C('source-read::envelope',L('data'),`(text "${magic}")`,`(text "lkjscript.kernel.${n}-envelope")`,I(max))]],iff(lt(I(-1),L('end')),lets([['next',chain(I(18),steps)]],iff(eq(L('next'),L('end')),I(0),I(12))),I(12))));}
forms.push(record('dependency','LKJDEP14',1048576,[p=>single(p,1),p=>typed(p,18),p=>typed(p,255,33),p=>typed(p,11,33)]));
forms.push(record('retirement','LKJRET14',65536,[p=>single(p,1),p=>C('owner-key',L('data'),p,L('end')),p=>tag(p,28),p=>C('optional-name',L('data'),p,L('end')),p=>C('optional-owner',L('data'),p,L('end')),p=>typed(p,255,33),p=>typed(p,9,33)]));
const failure=rec([['diagnostic',I(12)],['image',C('ci::text-bytes','(text "")')]]);
const digest=lets([
 ['a (type ByteBuffer)',C('ci::empty')],['b (type ByteBuffer)',C('ci::le',I(1),I(2),L('a'))],
 ['c (type ByteBuffer)',C('wire::be',L('end'),I(8),L('b'))],['e (type ByteBuffer)',C('ci::append',L('data'),I(0),L('c'))]
],rec([['diagnostic',I(0)],['image',C('ci::derive','(text "lkjscript.semantic-revision")',C('ci::freeze',L('e')))]]));
const parsed=chain(I(0),[p=>single(p,1),p=>single(p,1),p=>typed(p,1),p=>tag(p,2),p=>C('source-shape::span-cursor',C('source-read::array',L('data'),sub(p,I(1)),L('end'),I(255),I(33))),p=>typed(p,10,33)],p=>iff(both(eq(p,L('end')),iff(eq(get(I(19)),I(2)),lt(C('artifact-input::bytes-order',L('data'),I(21),I(54),I(32)),I(0)),yes)),I(0),I(-1)));
forms.push(fun('revision',data,'(record (diagnostic I64) (image Bytes))',lets([['end',length(L('data'))]],iff(both(lt(I(52),L('end')),lt(L('end'),I(120))),lets([['code',parsed]],iff(eq(L('code'),I(0)),digest,failure)),failure))));
save('source-history',forms);
const probes=['dependency','retirement'].map(n=>fun('read-'+n,data,'(record (diagnostic I64) (image Bytes))',rec([['diagnostic',C('source-history::'+n,L('data'))],['image',C('ci::text-bytes','(text "")')]])));
probes.push(fun('read-revision',data,'(record (diagnostic I64) (image Bytes))',C('source-history::revision',L('data'))));
fs.writeFileSync('.artifacts/selfhost/source-history-probe-draft.lkjc','declarations.begin\n(units (module create source-history-probe\n'+probes.join('\n')+'\n))\ndeclarations.end\n');
