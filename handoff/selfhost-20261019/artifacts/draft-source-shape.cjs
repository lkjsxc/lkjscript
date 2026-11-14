const fs=require('fs');
const L=n=>`(local ${n})`,I=n=>`(i64 ${n})`,C=(n,...a)=>`(call ${n} ${a.join(' ')})`,F=(x,n)=>`(field ${x} (name ${n}))`;
const add=(a,b)=>C('ci::add',a,b),sub=(a,b)=>C('ci::sub',a,b),eq=(a,b)=>C('ci::eq',a,b),lt=(a,b)=>C('ci::lt',a,b);
const iff=(p,a,b)=>`(if ${p} ${a} ${b})`,yes='(bool true)',no='(bool false)';
const both=(...ps)=>ps.reduceRight((b,a)=>iff(a,b,no),yes),either=(...ps)=>ps.reduceRight((b,a)=>iff(a,yes,b),no);
const lets=(xs,b)=>`(let ${xs.map(([n,v])=>`(binding ${n} ${v})`).join('\n')} (in ${b}))`;
const fun=(n,ps,r,b)=>`(function create ${n} (visibility public) (effect pure) ${ps.map(([n,t])=>`(parameter create ${n} (type ${t}))`).join(' ')} (returns ${r}) (body ${b}))`;
const choose=(rows,otherwise)=>rows.reduceRight((rest,[p,v])=>iff(p,v,rest),otherwise);
const data=[['data','Bytes']],at=[...data,['at','I64'],['end','I64']];
const forms=[];
const spanShape='(record (code I64) (next I64) (start I64) (length I64))';
const scalarShape='(record (code I64) (next I64) (value I64))';
forms.push(fun('span-cursor',[['value',spanShape]],'I64',iff(eq(F(L('value'),'code'),I(0)),F(L('value'),'next'),I(-1))));
forms.push(fun('number-cursor',[['value',scalarShape]],'I64',iff(eq(F(L('value'),'code'),I(0)),F(L('value'),'next'),I(-1))));
const span=x=>C('span-cursor',x),number=x=>C('number-cursor',x);
const typed=(p,tag,width=17)=>C('source-read::typed',L('data'),p,L('end'),I(tag),I(width));
const name=p=>span(C('source-read::name',L('data'),p,L('end')));
const array=(p,tag,width=17)=>span(C('source-read::array',L('data'),p,L('end'),I(tag),I(width)));
const ref=(p,tag)=>C('source-read::reference',L('data'),p,L('end'),I(tag));
const single=(p,v)=>C('source-read::single',L('data'),p,L('end'),I(v));
function chain(start,steps,finish=x=>x){
 function walk(i,p){if(i===steps.length)return finish(p);const n='cursor-'+i;return lets([[n,steps[i](p)]],iff(lt(I(-1),L(n)),walk(i+1,L(n)),L(n)));}
 return walk(0,start);
}
forms.push(fun('name-less',[...data,['a','I64'],['an','I64'],['b','I64'],['bn','I64']],'Bool',
 lets([['count',iff(lt(L('an'),L('bn')),L('an'),L('bn'))],['order',C('artifact-input::bytes-order',L('data'),L('a'),L('b'),L('count'))]],
 iff(eq(L('order'),I(0)),lt(L('an'),L('bn')),lt(L('order'),I(0))))));
forms.push(fun('type-fields',[...at,['count','I64'],['previous','I64'],['previous-length','I64']],'I64',
 iff(both(lt(I(-1),L('count')),lt(L('count'),I(100001))),
 iff(eq(L('count'),I(0)),L('at'),lets([['field',C('source-read::name',L('data'),L('at'),L('end'))]],
 iff(eq(F(L('field'),'code'),I(0)),
 iff(iff(eq(L('previous-length'),I(0)),yes,C('name-less',L('data'),L('previous'),L('previous-length'),F(L('field'),'start'),F(L('field'),'length'))),
 lets([['next',typed(F(L('field'),'next'),2,33)]],iff(lt(I(-1),L('next')),C('type-fields',L('data'),L('next'),L('end'),sub(L('count'),I(1)),F(L('field'),'start'),F(L('field'),'length')),I(-1))),I(-1)),I(-1)))),I(-1))));
const fields=lets([['count',C('source-read::unsigned',L('data'),L('at'),L('end'))]],
 iff(both(eq(F(L('count'),'code'),I(0)),lt(I(0),F(L('count'),'value')),lt(F(L('count'),'value'),I(100001))),C('type-fields',L('data'),F(L('count'),'next'),L('end'),F(L('count'),'value'),I(0),I(0)),I(-1)));
forms.push(fun('type-body',[...at,['form','I64']],'I64',choose([
 [either(...[0,1,2,3,4,5,20].map(n=>eq(L('form'),I(n)))),L('at')],
 [eq(L('form'),I(7)),typed(L('at'),17)],
 [eq(L('form'),I(8)),ref(L('at'),3)],
 [eq(L('form'),I(10)),fields],
 [eq(L('form'),I(11)),typed(L('at'),2,33)],
 [eq(L('form'),I(16)),chain(L('at'),[p=>array(p,2,33),p=>typed(p,2,33)])],
 [eq(L('form'),I(17)),chain(L('at'),[p=>ref(p,3),p=>lets([['arguments',C('source-read::array',L('data'),p,L('end'),I(2),I(33))]],iff(both(eq(F(L('arguments'),'code'),I(0)),lt(I(0),F(L('arguments'),'length'))),F(L('arguments'),'next'),I(-1)))])]
],I(-2))));
forms.push(fun('type',data,'I64',lets([['end',C('source-read::envelope',L('data'),'(text "LKJTYPES")','(text "lkjscript.kernel.type-envelope")',I(1048576))]],
 iff(lt(I(-1),L('end')),iff(C('source-read::range',L('data'),I(18),I(2),L('end')),
 iff(eq(C('ci::get',L('data'),I(18)),I(1)),lets([['form',C('ci::get',L('data'),I(19))]],
 iff(lt(L('form'),I(35)),lets([['next',C('type-body',L('data'),I(20),L('end'),L('form'))]],iff(eq(L('next'),L('end')),I(0),iff(eq(L('next'),I(-2)),I(13),I(12)))),I(12))),I(12)),I(12)),I(12)))));
const output='.artifacts/selfhost/source-shape-draft.lkjc';
fs.writeFileSync(output,'declarations.begin\n(units (module create source-shape\n'+forms.join('\n')+'\n))\ndeclarations.end\n');
console.log(JSON.stringify({output,functions:forms.length,status:'Unadmitted structural prototype. Type references/cycles and semantic validity not checked.'}));
