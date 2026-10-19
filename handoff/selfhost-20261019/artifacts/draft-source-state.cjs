// Neutral logical-state commitment proposal; no graph or semantic admission claim.
const fs=require('fs');
const {L,I,C,F,add,sub,eq,lt,iff,both,lets,fun,rec,save}=require('./source-author.cjs');
const length=x=>C('ci::length',x),empty=C('ci::text-bytes','(text "")');
const failure=rec([['diagnostic',I(12)],['image',empty]]);
const forms=[];
forms.push(fun('section',[['tag','I64'],['count','I64'],['digest','Bytes'],['out','ByteBuffer']],'ByteBuffer',
 lets([['a (type ByteBuffer)',C('ci::push',L('tag'),L('out'))],['b (type ByteBuffer)',C('wire::be',L('count'),I(8),L('a'))]],C('ci::append',L('digest'),I(0),L('b'))))
 .replace('(visibility public)','(visibility private)').replace('(parameter create out (type ByteBuffer))','(parameter create out (type ByteBuffer) (use consume))'));
const build=lets([
 ['a (type ByteBuffer)',C('ci::empty')],
 ['b (type ByteBuffer)',C('wire::be',I(1),I(2),L('a'))],
 ['c (type ByteBuffer)',C('wire::be',I(1),I(2),L('b'))],
 ['d (type ByteBuffer)',C('ci::append',L('package'),I(0),L('c'))],
 ['e (type ByteBuffer)',C('wire::be',length(L('name')),I(8),L('d'))],
 ['f (type ByteBuffer)',C('ci::append',L('name'),I(0),L('e'))],
 ['g (type ByteBuffer)',C('section',I(1),C('ci::div',length(L('owners')),I(50)),F(L('owner-root'),'image'),L('f'))],
 ['h (type ByteBuffer)',C('section',I(2),C('ci::div',length(L('dependencies')),I(48)),F(L('dependency-root'),'image'),L('g'))],
 ['i (type ByteBuffer)',C('section',I(3),C('ci::div',length(L('retirements')),I(49)),F(L('retirement-root'),'image'),L('h'))],
 ['canonical',C('ci::freeze',L('i'))]
],rec([['diagnostic',I(0)],['image',C('ci::derive','(text "lkjscript.kernel.semantic-state.v1")',L('canonical'))]]));
const maps=lets([['owner-root',C('source-map::root',L('owners'),I(17),I(33))]],
 iff(eq(F(L('owner-root'),'diagnostic'),I(0)),
 lets([['dependency-root',C('source-map::root',L('dependencies'),I(16),I(32))]],
 iff(eq(F(L('dependency-root'),'diagnostic'),I(0)),
 lets([['retirement-root',C('source-map::root',L('retirements'),I(17),I(32))]],
 iff(eq(F(L('retirement-root'),'diagnostic'),I(0)),build,failure)),failure)),failure));
forms.push(fun('root',[['package','Bytes'],['name','Bytes'],['owners','Bytes'],['dependencies','Bytes'],['retirements','Bytes']], '(record (diagnostic I64) (image Bytes))',
 iff(both(eq(length(L('package')),I(16)),iff(C('artifact-check::zeros',L('package'),I(0),I(16)),'(bool false)','(bool true)'),lt(I(0),length(L('name'))),lt(length(L('name')),I(129)),C('source-read::name-scan',L('name'),I(0),length(L('name')),'(bool true)')),maps,failure)));
save('source-state',forms);
const data=L('data'),end=length(data),range=(at,n)=>C('source-read::range',data,at,n,end),slice=(a,b)=>C('ci::slice',data,a,b);
const names=['name','owners','dependencies','retirements'];
function readPart(index,at){if(index===names.length)return iff(eq(at,end),C('source-state::root',slice(I(0),I(16)),...names.map(L)),failure);
 const name=names[index],count=C('ci::u32',data,I(16+index*4));
 return lets([[name+'-end',add(at,count)]],iff(range(at,count),lets([[name,slice(at,L(name+'-end'))]],readPart(index+1,L(name+'-end'))),failure));}
const probe=fun('read-state',[['data','Bytes']], '(record (diagnostic I64) (image Bytes))',iff(lt(end,I(32)),failure,readPart(0,I(32))));
fs.writeFileSync('.artifacts/selfhost/source-state-probe-draft.lkjc','declarations.begin\n(units (module create source-state-probe\n'+probe+'\n))\ndeclarations.end\n');
