// Unadmitted raw source input; no compiled units, instructions or flow certificates.
const d = require('./source-author.cjs');
const {L,I,C,F,add,sub,eq,lt,both,lets,fun,rec,save} = d;
const iff=d.iff, data=[['data','Bytes']];
const length=x=>C('ci::length',x), mul=(a,b)=>C('ci::mul',a,b);
const word=at=>C('ci::u32',L('data'),typeof at==='number'?I(at):at);
const slice=(a,b)=>C('ci::slice',L('data'),typeof a==='number'?I(a):a,typeof b==='number'?I(b):b);
const end=(base,n,width)=>add(base,mul(n,I(width)));
const gate=(predicate,value,code=12)=>iff(predicate,value,I(code));
const checked=(name,expression,value)=>lets([[name,expression]],iff(eq(L(name),I(0)),value,L(name)));
const forms=[];
const nextOwner=C('owner-bindings',L('data'),add(L('index'),I(1)));
const boundOwner=lets([
 ['start',word(add(L('object'),I(8)))],
 ['end',add(L('start'),word(add(L('object'),I(12))))],
],checked('code',C('source-object::owner',slice(L('start'),L('end')),slice(L('at'),add(L('at'),I(50)))),nextOwner));
const ownerStep=lets([
 ['at',end(word(72),L('index'),50)],
 ['object',C('artifact-input::object',L('data'),I(1),add(L('at'),I(18)),I(0),word(84))],
],gate(lt(I(-1),L('object')),boundOwner));
forms.push(fun('owner-bindings',[...data,['index','I64']],'I64',iff(eq(L('index'),word(76)),I(0),ownerStep)));
const nextShape=C('shapes',L('data'),add(L('index'),I(1)),L('owners'));
const shapeStep=lets([
 ['at',end(word(80),L('index'),48)],['domain',word(L('at'))],
 ['start',word(add(L('at'),I(8)))],['end',add(L('start'),word(add(L('at'),I(12))))],
],iff(eq(L('domain'),I(1)),C('shapes',L('data'),add(L('index'),I(1)),add(L('owners'),I(1))),
 gate(eq(L('domain'),I(2)),checked('code',C('source-shape::type',slice(L('start'),L('end'))),nextShape),13)));
forms.push(fun('shapes',[...data,['index','I64'],['owners','I64']],'I64',iff(eq(L('index'),word(84)),gate(eq(L('owners'),word(76)),I(0)),shapeStep)));
// Metadata: repo16/package16/state32/revision32/name-size4/core-size4/reserved8,
// then the canonical name and current raw RevisionCore.
const state=C('source-state::root',
 slice(add(L('meta'),I(16)),add(L('meta'),I(32))),slice(L('name-at'),L('core-at')),
 slice(word(72),end(word(72),word(76),50)),
 slice(word(96),end(word(96),word(100),48)),
 slice(word(104),end(word(104),word(108),49)));
const checkState=lets([['state',state]],iff(eq(F(L('state'),'diagnostic'),I(0)),
 gate(C('ci::same',F(L('state'),'image'),slice(add(L('meta'),I(32)),add(L('meta'),I(64)))),I(0)),F(L('state'),'diagnostic')));
const revisionBindings=both(
 C('ci::same',F(L('revision'),'image'),slice(add(L('meta'),I(64)),add(L('meta'),I(96)))),
 C('ci::same',C('ci::slice',L('core'),I(3),I(19)),slice(L('meta'),add(L('meta'),I(16)))),
 C('ci::same',C('ci::slice',L('core'),sub(L('core-size'),I(32)),L('core-size')),slice(add(L('meta'),I(32)),add(L('meta'),I(64)))));
const checkRevision=lets([['core',slice(L('core-at'),L('meta-end'))],['revision',C('source-history::revision',L('core'))]],
 gate(eq(F(L('revision'),'diagnostic'),I(0)),gate(revisionBindings,checkState)));
const metadataBounds=both(
 lt(I(0),L('name-size')),lt(L('name-size'),I(129)),lt(I(52),L('core-size')),lt(L('core-size'),I(120)),
 eq(add(L('core-at'),L('core-size')),L('meta-end')),
 C('artifact-check::zeros',L('data'),add(L('meta'),I(104)),add(L('meta'),I(112))));
forms.push(fun('identity',data,'I64',lets([
 ['meta',word(88)],['meta-end',add(L('meta'),word(92))],
 ['name-size',word(add(L('meta'),I(96)))],['core-size',word(add(L('meta'),I(100)))],
 ['name-at',add(L('meta'),I(112))],['core-at',add(L('name-at'),L('name-size'))],
],gate(metadataBounds,checkRevision))));
const shapeAndBindings=gate(eq(L('dependencies'),I(0)),checked('shape-code',C('shapes',L('data'),I(0),I(0)),C('owner-bindings',L('data'),I(0))),13);
const objects=gate(C('artifact-input::objects',L('data'),I(0),L('after')),checked('identity-code',C('identity',L('data')),shapeAndBindings));
const tableOrder=both(
 eq(C('source-index::validate',L('data'),word(72),L('owners'),I(17),I(50)),I(0)),
 eq(C('source-index::validate',L('data'),word(96),L('dependencies'),I(16),I(48)),I(0)),
 eq(C('source-index::validate',L('data'),word(104),L('retirements'),I(17),I(49)),I(0)),
 eq(C('ci::get',L('data'),end(word(72),word(12),50)),I(2)));
const tableBounds=both(
 lt(I(0),L('owners')),lt(L('owners'),I(100001)),lt(I(0),L('objects')),lt(L('objects'),I(100001)),
 lt(L('dependencies'),I(100001)),lt(L('retirements'),I(100001)),eq(word(24),L('owners')),lt(word(12),L('owners')),
 eq(word(72),I(160)),eq(word(96),L('owner-end')),eq(word(104),L('dependency-end')),
 eq(word(80),L('retirement-end')),eq(word(88),L('object-end')),lt(I(111),L('meta-size')),
 C('source-read::range',L('data'),I(160),sub(L('after'),I(160)),length(L('data'))));
forms.push(fun('tables',data,'I64',lets([
 ['owners',word(76)],['objects',word(84)],['dependencies',word(100)],['retirements',word(108)],
 ['owner-end',end(I(160),L('owners'),50)],['dependency-end',end(L('owner-end'),L('dependencies'),48)],
 ['retirement-end',end(L('dependency-end'),L('retirements'),49)],['object-end',end(L('retirement-end'),L('objects'),48)],
 ['meta-size',word(92)],['after',add(L('object-end'),L('meta-size'))],
],gate(tableBounds,gate(tableOrder,objects)))));
const header=both(C('ci::word-is',L('data'),I(16),length(L('data'))),eq(word(28),I(0)),eq(word(36),I(0)),
 lt(word(32),I(2147483648)),lt(word(112),I(2)),C('artifact-check::zeros',L('data'),I(116),I(160)));
const counts=add(add(word(76),word(84)),add(word(100),word(108)));
const fuel=add(add(I(4096),mul(I(128),length(L('data')))),mul(I(64),counts));
const hash=gate(C('ci::same',slice(40,72),C('ci::digest',C('ci::concat',slice(0,40),slice(I(72),length(L('data')))))),C('tables',L('data')),4);
const budget=iff(lt(word(32),fuel),I(6),hash);
const checkedHeader=gate(header,budget,3);
const profile=gate(eq(word(8),I(5)),checkedHeader,2);
const magic=gate(C('ci::same',slice(0,8),C('ci::text-bytes','(text "LKJCI001")')),profile,1);
forms.push(fun('validate',data,'I64',gate(both(lt(I(159),length(L('data'))),lt(length(L('data')),I(8388609))),magic,3)));
const result=image=>rec([['diagnostic',L('code')],['image',image]]);
forms.push(fun('inspect',data,'(record (diagnostic I64) (image Bytes))',lets([['code',C('validate',L('data'))]],
 iff(eq(L('code'),I(0)),result(slice(add(word(88),I(32)),add(word(88),I(64)))),result(C('ci::text-bytes','(text "")'))))));
save('source-input',forms.map((form,i)=>i<forms.length-2?form.replace('(visibility public)','(visibility private)'):form));
