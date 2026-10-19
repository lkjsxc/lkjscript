// Bind canonical object bytes to exact logical directory entries; no graph admission.
const fs=require('fs');
const {L,I,C,F,add,sub,eq,lt,iff,yes,no,both,lets,fun,rec,save}=require('./source-author.cjs');
const length=x=>C('ci::length',x),get=(d,n)=>C('ci::get',d,I(n)),slice=(d,a,b)=>C('ci::slice',d,I(a),I(b));
const data=L('data'),entry=L('entry'),forms=[];
const digest=(domain,at)=>C('ci::same',C('wire::digest',`(text "lkjscript.kernel.${domain}")`,data),slice(entry,at,at+32));
function bound(name,width,reader,predicate){return fun(name,[['data','Bytes'],['entry','Bytes']],'I64',iff(eq(length(entry),I(width)),lets([['code',C(reader,data)]],iff(eq(L('code'),I(0)),iff(predicate,I(0),I(12)),L('code'))),I(12)));}
forms.push(bound('type',32,'source-shape::type',digest('type-object.v10',0)));
const expression=eq(get(data,18),I(9));
forms.push(bound('owner',50,'source-owner::owner',both(
 eq(get(entry,0),iff(expression,I(9),add(get(data,20),I(1)))),
 C('ci::same',slice(entry,1,17),iff(expression,slice(data,21,37),slice(data,22,38))),
 eq(get(entry,17),iff(expression,I(17),add(get(data,38),I(1)))),digest('owner-object.v14',18)
)));
forms.push(bound('dependency',48,'source-history::dependency',both(C('ci::same',slice(entry,0,16),slice(data,20,36)),digest('dependency-object.v14',16))));
forms.push(bound('retirement',49,'source-history::retirement',both(eq(get(entry,0),add(get(data,19),I(1))),C('ci::same',slice(entry,1,17),slice(data,21,37)),digest('retirement-object.v14',17))));
save('source-object',forms);
const result='(record (diagnostic I64) (image Bytes))',empty=C('ci::text-bytes','(text "")');
const output=(c,image)=>rec([['diagnostic',c],['image',image]]);
const fail=output(I(12),empty),probes=[];
for(const [name,width] of [['type',32],['owner',50],['dependency',48],['retirement',49]])probes.push(fun('bind-'+name,[['data','Bytes']],result,iff(lt(length(data),I(width)),fail,output(C('source-object::'+name,C('ci::slice',data,I(width),length(data)),slice(data,0,width)),empty))));
fs.writeFileSync('.artifacts/selfhost/source-object-probe-draft.lkjc','declarations.begin\n(units (module create source-object-probe\n'+probes.join('\n')+'\n))\ndeclarations.end\n');
const failLoop=lets([['discarded',C('ci::freeze',L('output'))]],output(I(12),empty));
const widths=[32,50,48,49],names=['type','owner','dependency','retirement'];
const choose=(xs,fallback)=>xs.reduceRight((rest,[p,v])=>iff(p,v,rest),fallback);
const width=choose(widths.map((n,k)=>[eq(L('kind'),I(k)),I(n)]),I(0));
const boundCode=choose(names.map((n,k)=>[eq(L('kind'),I(k)),C('source-object::'+n,L('object'),L('entry'))]),I(12));
const step=lets([['kind',C('ci::get',data,L('at'))],['size',C('ci::u32',data,add(L('at'),I(1)))],['width',width],['start',add(L('at'),I(5))]],
 iff(both(lt(L('kind'),I(4)),C('source-read::range',data,L('start'),add(L('width'),L('size')),L('end'))),
 lets([['object-start',add(L('start'),L('width'))],['next',add(L('object-start'),L('size'))],['entry',C('ci::slice',data,L('start'),L('object-start'))],['object',C('ci::slice',data,L('object-start'),L('next'))],['code',boundCode],['continued (type ByteBuffer)',C('ci::push',L('code'),L('output'))]],
 C('bindings-loop',data,L('next'),L('end'),L('continued'))),failLoop));
let loop=fun('bindings-loop',[['data','Bytes'],['at','I64'],['end','I64'],['output','ByteBuffer']],result,
 iff(eq(L('at'),L('end')),output(I(0),C('ci::freeze',L('output'))),iff(C('source-read::range',data,L('at'),I(5),L('end')),step,failLoop)))
 .replace('(visibility public)','(visibility private)').replace('(parameter create output (type ByteBuffer))','(parameter create output (type ByteBuffer) (use consume))');
probes.push(loop);
probes.push(fun('read-bindings',[['data','Bytes']],result,lets([['initial (type ByteBuffer)',C('ci::empty')]],C('bindings-loop',data,I(0),length(data),L('initial')))));
fs.writeFileSync('.artifacts/selfhost/source-object-probe-draft.lkjc','declarations.begin\n(units (module create source-object-probe\n'+probes.join('\n')+'\n))\ndeclarations.end\n');
