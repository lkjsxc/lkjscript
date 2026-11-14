const fs=require('fs');
const L=n=>`(local ${n})`,I=n=>`(i64 ${n})`,C=(n,...a)=>`(call ${n} ${a.join(' ')})`,F=(x,n)=>`(field ${x} (name ${n}))`;
const add=(a,b)=>C('ci::add',a,b),sub=(a,b)=>C('ci::sub',a,b),eq=(a,b)=>C('ci::eq',a,b),lt=(a,b)=>C('ci::lt',a,b);
const iff=(p,a,b)=>`(if ${p} ${a} ${b})`,yes='(bool true)',no='(bool false)';
const both=(...ps)=>ps.reduceRight((b,a)=>iff(a,b,no),yes),either=(...ps)=>ps.reduceRight((b,a)=>iff(a,yes,b),no);
const lets=(xs,b)=>`(let ${xs.map(([n,v])=>`(binding ${n} ${v})`).join('\n')} (in ${b}))`;
const fun=(n,ps,r,b)=>`(function create ${n} (visibility public) (effect pure) ${ps.map(([n,t])=>`(parameter create ${n} (type ${t}))`).join(' ')} (returns ${r}) (body ${b}))`;
const choose=(rows,otherwise)=>rows.reduceRight((rest,[p,v])=>iff(p,v,rest),otherwise);
const data=[['data','Bytes']],at=[...data,['at','I64'],['end','I64']];
const span=x=>C('source-shape::span-cursor',x),number=x=>C('source-shape::number-cursor',x);
const typed=(p,tag,width=17)=>C('source-read::typed',L('data'),p,L('end'),I(tag),I(width));
const name=p=>span(C('source-read::name',L('data'),p,L('end')));
const array=(p,tag,width=17)=>span(C('source-read::array',L('data'),p,L('end'),I(tag),I(width)));
const ref=(p,tag)=>C('source-read::reference',L('data'),p,L('end'),I(tag));
const single=(p,v)=>C('source-read::single',L('data'),p,L('end'),I(v));
const range=(p,n)=>C('source-read::range',L('data'),p,I(n),L('end'));
const get=p=>C('ci::get',L('data'),p);
const optional=(p,tag,width=17)=>C('source-read::optional',L('data'),p,L('end'),I(tag),I(width));
const optref=(p,tag)=>C('source-fields::optional-reference',L('data'),p,L('end'),I(tag));
const zero=p=>C('source-fields::zero',L('data'),p,L('end'));
const tag=(p,max)=>C('source-fields::tag',L('data'),p,L('end'),I(max));
function chain(start,steps,finish=x=>x){function walk(i,p){if(i===steps.length)return finish(p);const n='cursor-'+i;return lets([[n,steps[i](p)]],iff(lt(I(-1),L(n)),walk(i+1,L(n)),L(n)));}return walk(0,start);}
const rec=xs=>`(record structural ${xs.map(([n,v])=>`(field ${n} ${v})`).join(' ')})`;
const spanShape='(record (code I64) (next I64) (start I64) (length I64))';
const spanResult=(c,n,s,l)=>rec([['code',c],['next',n],['start',s],['length',l]]);
function save(mod,forms){const p='.artifacts/selfhost/'+mod+'-draft.lkjc';fs.writeFileSync(p,'declarations.begin\n(units (module create '+mod+'\n'+forms.join('\n')+'\n))\ndeclarations.end\n');console.log(JSON.stringify({module:mod,functions:forms.length,path:p,status:'Unadmitted proposal'}));}
module.exports={L,I,C,F,add,sub,eq,lt,iff,yes,no,both,either,lets,fun,choose,data,at,span,number,typed,name,array,ref,single,range,get,optional,optref,zero,tag,chain,rec,spanShape,spanResult,save};
