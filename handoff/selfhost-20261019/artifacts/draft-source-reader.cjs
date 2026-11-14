const fs=require('fs');
const L=n=>`(local ${n})`,I=n=>`(i64 ${n})`,C=(n,...a)=>`(call ${n} ${a.join(' ')})`,F=(x,n)=>`(field ${x} (name ${n}))`;
const add=(a,b)=>C('ci::add',a,b),sub=(a,b)=>C('ci::sub',a,b),mul=(a,b)=>C('ci::mul',a,b),div=(a,b)=>C('ci::div',a,b),eq=(a,b)=>C('ci::eq',a,b),lt=(a,b)=>C('ci::lt',a,b);
const iff=(p,a,b)=>`(if ${p} ${a} ${b})`,yes='(bool true)',no='(bool false)';
const both=(...ps)=>ps.reduceRight((b,a)=>iff(a,b,no),yes);
const lets=(xs,b)=>`(let ${xs.map(([n,v])=>`(binding ${n} ${v})`).join('\n')} (in ${b}))`;
const rec=xs=>`(record structural ${xs.map(([n,v])=>`(field ${n} ${v})`).join(' ')})`;
const shape='(record (code I64) (next I64) (low I64) (high I64))';
const result=(code,next,lo,hi=I(0))=>rec([['code',code],['next',next],['low',lo],['high',hi]]);
const fail=result(I(12),L('at'),I(0));
const fun=(n,ps,r,b)=>`(function create ${n} (visibility public) (effect pure) ${ps.map(([n,t])=>`(parameter create ${n} (type ${t}))`).join(' ')} (returns ${r}) (body ${b}))`;
const data=[['data','Bytes']],at=[...data,['at','I64'],['end','I64']];
const forms=[];
forms.push(fun('range',[...data,['at','I64'],['count','I64'],['end','I64']],'Bool',
 iff(lt(L('at'),I(0)),no,iff(lt(L('count'),I(0)),no,iff(lt(L('end'),I(0)),no,
 iff(lt(C('ci::length',L('data')),L('end')),no,iff(lt(L('end'),L('at')),no,
 iff(lt(sub(L('end'),L('at')),L('count')),no,yes))))))));
const range=n=>C('range',L('data'),L('at'),I(n),L('end'));
const payload=add(L('at'),I(1));
const read16=add(C('ci::get',L('data'),payload),mul(C('ci::get',L('data'),add(L('at'),I(2))),I(256)));
const wide=iff(range(9),lets([
 ['low',C('ci::u32',L('data'),payload)],['high',C('ci::u32',L('data'),add(L('at'),I(5))) ]],
 iff(eq(L('high'),I(0)),fail,result(I(0),add(L('at'),I(9)),L('low'),L('high')))),fail);
const word=iff(range(5),lets([['value',C('ci::u32',L('data'),payload)]],iff(lt(L('value'),I(65536)),fail,result(I(0),add(L('at'),I(5)),L('value')))),fail);
const short=iff(range(3),lets([['value',read16]],iff(lt(L('value'),I(251)),fail,result(I(0),add(L('at'),I(3)),L('value')))),fail);
const tags=iff(lt(L('tag'),I(251)),result(I(0),add(L('at'),I(1)),L('tag')),iff(eq(L('tag'),I(251)),short,iff(eq(L('tag'),I(252)),word,iff(eq(L('tag'),I(253)),wide,fail))));
forms.push(fun('integer',at,shape,iff(range(1),lets([['tag',C('ci::get',L('data'),L('at'))]],tags),fail)));
const scalar='(record (code I64) (next I64) (value I64))';
const scalarResult=(c,n,v)=>rec([['code',c],['next',n],['value',v]]);
const scalarFail=scalarResult(I(12),L('at'),I(0));
forms.push(fun('unsigned',at,scalar,lets([['decoded',C('integer',L('data'),L('at'),L('end'))]],
 iff(both(eq(F(L('decoded'),'code'),I(0)),lt(F(L('decoded'),'high'),I(2147483648))),
 scalarResult(I(0),F(L('decoded'),'next'),add(mul(F(L('decoded'),'high'),I(4294967296)),F(L('decoded'),'low'))),scalarFail))));
forms.push(fun('signed',at,scalar,lets([['decoded',C('integer',L('data'),L('at'),L('end'))]],
 iff(eq(F(L('decoded'),'code'),I(0)),lets([
 ['low',F(L('decoded'),'low')],['half',div(L('low'),I(2))],
 ['magnitude',add(mul(F(L('decoded'),'high'),I(2147483648)),L('half'))]],
 scalarResult(I(0),F(L('decoded'),'next'),iff(eq(L('low'),mul(L('half'),I(2))),L('magnitude'),sub(I(-1),L('magnitude'))))),scalarFail))));
forms.push(fun('envelope',[...data,['magic','Text'],['context','Text'],['maximum','I64']],'I64',
 lets([['length',C('ci::length',L('data'))]],
 iff(both(lt(I(49),L('length')),lt(L('length'),I(8388609))),
 lets([['end',sub(L('length'),I(32))]],
 iff(both(
 C('ci::same',C('ci::slice',L('data'),I(0),I(8)),C('ci::text-bytes',L('magic'))),
 eq(C('ci::get',L('data'),I(8)),I(1)),eq(C('ci::get',L('data'),I(9)),I(0)),
 eq(C('ci::u32',L('data'),I(14)),I(0)),
 eq(C('ci::u32',L('data'),I(10)),sub(L('length'),I(50))),
 lt(I(-1),L('maximum')),iff(lt(L('maximum'),sub(L('length'),I(50))),no,yes),
 C('ci::same',C('ci::slice',L('data'),L('end'),L('length')),C('wire::digest',L('context'),C('ci::slice',L('data'),I(0),L('end'))))),L('end'),I(-1))),I(-1)))));
// Additional structural readers, still an unadmitted proposal.
const either=(...ps)=>ps.reduceRight((b,a)=>iff(a,yes,b),no);
const spanShape='(record (code I64) (next I64) (start I64) (length I64))';
const spanResult=(c,n,s,l)=>rec([['code',c],['next',n],['start',s],['length',l]]);
const spanFail=spanResult(I(12),L('at'),I(0),I(0));
forms.push(fun('typed',[...at,['tag','I64'],['width','I64']],'I64',
 iff(both(either(eq(L('width'),I(17)),eq(L('width'),I(33))),C('range',L('data'),L('at'),L('width'),L('end'))),
 iff(both(eq(C('ci::get',L('data'),L('at')),L('tag')),iff(eq(L('width'),I(17)),iff(C('artifact-check::zeros',L('data'),add(L('at'),I(1)),add(L('at'),I(17))),no,yes),yes)),add(L('at'),L('width')),I(-1)),I(-1))));
forms.push(fun('array-items',[...at,['tag','I64'],['width','I64'],['count','I64']],'I64',
 iff(both(lt(I(-1),L('count')),lt(L('count'),I(100001)),C('range',L('data'),L('at'),I(0),L('end'))),
 iff(eq(L('count'),I(0)),L('at'),lets([['next',C('typed',L('data'),L('at'),L('end'),L('tag'),L('width'))]],
 iff(lt(I(-1),L('next')),C('array-items',L('data'),L('next'),L('end'),L('tag'),L('width'),sub(L('count'),I(1))),I(-1)))),I(-1))));
forms.push(fun('array',[...at,['tag','I64'],['width','I64']],spanShape,
 lets([['count',C('unsigned',L('data'),L('at'),L('end'))]],
 iff(both(eq(F(L('count'),'code'),I(0)),lt(F(L('count'),'value'),I(100001)),either(eq(L('width'),I(17)),eq(L('width'),I(33))),C('range',L('data'),F(L('count'),'next'),mul(F(L('count'),'value'),L('width')),L('end'))),
 lets([['next',C('array-items',L('data'),F(L('count'),'next'),L('end'),L('tag'),L('width'),F(L('count'),'value'))]],
 iff(lt(I(-1),L('next')),spanResult(I(0),L('next'),F(L('count'),'next'),F(L('count'),'value')),spanFail)),spanFail))));
forms.push(fun('string',[...at,['minimum','I64'],['maximum','I64']],spanShape,
 lets([['count',C('unsigned',L('data'),L('at'),L('end'))]],
 iff(both(eq(F(L('count'),'code'),I(0)),lt(I(-1),L('minimum')),iff(lt(L('maximum'),L('minimum')),no,yes),iff(lt(F(L('count'),'value'),L('minimum')),no,yes),iff(lt(L('maximum'),F(L('count'),'value')),no,yes),C('range',L('data'),F(L('count'),'next'),F(L('count'),'value'),L('end'))),
 lets([['next',add(F(L('count'),'next'),F(L('count'),'value'))]],
 iff(C('types::utf8',L('data'),F(L('count'),'next'),L('next')),spanResult(I(0),L('next'),F(L('count'),'next'),F(L('count'),'value')),spanFail)),spanFail))));
forms.push(fun('name-char',[['value','I64'],['first','Bool']],'Bool',
 either(both(lt(I(64),L('value')),lt(L('value'),I(91))),both(lt(I(96),L('value')),lt(L('value'),I(123))),eq(L('value'),I(95)),iff(L('first'),no,either(both(lt(I(47),L('value')),lt(L('value'),I(58))),eq(L('value'),I(45)))))));
forms.push(fun('name-scan',[...at,['first','Bool']],'Bool',
 iff(C('range',L('data'),L('at'),I(0),L('end')),iff(eq(L('at'),L('end')),yes,
 iff(C('name-char',C('ci::get',L('data'),L('at')),L('first')),C('name-scan',L('data'),add(L('at'),I(1)),L('end'),no),no)),no)));
forms.push(fun('name',at,spanShape,
 lets([['value',C('string',L('data'),L('at'),L('end'),I(1),I(128))]],
 iff(both(eq(F(L('value'),'code'),I(0)),C('name-scan',L('data'),F(L('value'),'start'),F(L('value'),'next'),yes)),L('value'),spanFail))));
forms.push(fun('reference',[...at,['tag','I64']],'I64',
 lets([['next',C('typed',L('data'),L('at'),L('end'),I(18),I(17))]],
 iff(lt(I(-1),L('next')),C('typed',L('data'),L('next'),L('end'),L('tag'),I(17)),I(-1)))));
forms.push(fun('single',[...at,['expected','I64']],'I64',
 iff(C('range',L('data'),L('at'),I(1),L('end')),iff(eq(C('ci::get',L('data'),L('at')),L('expected')),add(L('at'),I(1)),I(-1)),I(-1))));
forms.push(fun('optional',[...at,['tag','I64'],['width','I64']],'I64',
 iff(C('range',L('data'),L('at'),I(1),L('end')),lets([['present',C('ci::get',L('data'),L('at'))]],
 iff(eq(L('present'),I(0)),add(L('at'),I(1)),iff(eq(L('present'),I(1)),C('typed',L('data'),add(L('at'),I(1)),L('end'),L('tag'),L('width')),I(-1)))),I(-1))));

const output='.artifacts/selfhost/source-reader-draft.lkjc';
fs.writeFileSync(output,'declarations.begin\n(units (module create source-read\n'+forms.join('\n')+'\n))\ndeclarations.end\n');
console.log(JSON.stringify({output,functions:forms.length,status:'Unadmitted proposal draft; must review bounds and compare independent canonical fixtures before publication.'}));
