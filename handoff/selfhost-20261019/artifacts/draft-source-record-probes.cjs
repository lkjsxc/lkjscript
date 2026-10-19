const {L,I,C,F,add,sub,eq,lt,iff,yes,no,both,either,lets,fun,choose,data,at,span,number,typed,name,array,ref,single,range,get,optional,optref,zero,tag,chain,rec,spanShape,spanResult,save}=require('./source-author.cjs');
const fs=require('fs'),result='(record (diagnostic I64) (image Bytes))';
const empty=C('ci::octets','(list I64)');
const output=(c,b)=>rec([['diagnostic',c],['image',b]]);
const rows=[];
rows.push(fun('read-owner',data,result,output(C('source-owner::owner',L('data')),empty)));
const failure=lets([['discarded',C('ci::freeze',L('output'))]],output(I(12),empty));
const step=lets([['kind',get(L('at'))],['length',C('ci::u32',L('data'),add(L('at'),I(1)))],['start',add(L('at'),I(5))]],
 iff(both(lt(L('kind'),I(2)),C('source-read::range',L('data'),L('start'),L('length'),L('end'))),
 lets([['next',add(L('start'),L('length'))],['object',C('ci::slice',L('data'),L('start'),L('next'))],['code',iff(eq(L('kind'),I(0)),C('source-shape::type',L('object')),C('source-owner::owner',L('object'))) ]],
 lets([['continued (type ByteBuffer)',C('ci::le',L('code'),I(1),L('output'))]],C('records-loop',L('data'),L('next'),L('end'),L('continued')))),failure));
let loop=fun('records-loop',[...at,['output','ByteBuffer']],result,iff(range(L('at'),0),iff(eq(L('at'),L('end')),output(I(0),C('ci::freeze',L('output'))),iff(range(L('at'),5),step,failure)),failure));
loop=loop.replace('(parameter create output (type ByteBuffer))','(parameter create output (type ByteBuffer) (use consume))');
rows.push(loop);
rows.push(fun('read-records',data,result,lets([['initial (type ByteBuffer)',C('ci::empty')]],C('records-loop',L('data'),I(0),C('ci::length',L('data')),L('initial')))));
rows.push('(component create probes (visibility public) (port create owner (type (function (Bytes) '+result+')) (function read-owner)) (port create records (type (function (Bytes) '+result+')) (function read-records)))');
fs.writeFileSync('.artifacts/selfhost/source-record-probes-draft.lkjc','declarations.begin\n(units (module create source-record-probe\n'+rows.join('\n')+'\n) (target create source-records (component source-record-probe::probes) (runner command) (port source-record-probe::probes::records)) (target create source-owner (component source-record-probe::probes) (runner command) (port source-record-probe::probes::owner)))\ndeclarations.end\n');
