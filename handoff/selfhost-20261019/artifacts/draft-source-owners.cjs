const {L,I,C,F,add,sub,eq,lt,iff,yes,no,both,either,lets,fun,choose,data,at,span,number,typed,name,array,ref,single,range,get,optional,optref,zero,tag,chain,rec,spanShape,spanResult,save}=require('./source-author.cjs');
const fields=[];
fields.push(fun('tag',[...at,['maximum','I64']],'I64',iff(range(L('at'),1),iff(both(lt(I(-1),L('maximum')),lt(L('maximum'),I(251)),iff(lt(L('maximum'),get(L('at'))),no,yes)),add(L('at'),I(1)),I(-1)),I(-1))));
fields.push(fun('zero',at,'I64',lets([['value',C('source-read::unsigned',L('data'),L('at'),L('end'))]],iff(eq(F(L('value'),'code'),I(0)),iff(eq(F(L('value'),'value'),I(0)),F(L('value'),'next'),I(-2)),I(-1)))));
fields.push(fun('only-zero',[...at,['maximum','I64']],'I64',lets([['next',C('tag',L('data'),L('at'),L('end'),L('maximum'))]],iff(lt(I(-1),L('next')),iff(eq(get(L('at')),I(0)),L('next'),I(-2)),I(-1)))));
fields.push(fun('optional-reference',[...at,['tag','I64']],'I64',iff(range(L('at'),1),iff(eq(get(L('at')),I(0)),add(L('at'),I(1)),iff(eq(get(L('at')),I(1)),C('source-read::reference',L('data'),add(L('at'),I(1)),L('end'),L('tag')),I(-1))),I(-1))));
fields.push(fun('nonempty-array',[...at,['tag','I64'],['width','I64']],'I64',lets([['value',C('source-read::array',L('data'),L('at'),L('end'),L('tag'),L('width'))]],iff(both(eq(F(L('value'),'code'),I(0)),lt(I(0),F(L('value'),'length'))),F(L('value'),'next'),I(-1)))));
fields.push(fun('implementation-scan',[...at,['first','Bool']],'Bool',iff(range(L('at'),0),iff(eq(L('at'),L('end')),iff(L('first'),no,yes),lets([['byte',get(L('at'))]],iff(eq(L('byte'),I(46)),iff(L('first'),no,C('implementation-scan',L('data'),add(L('at'),I(1)),L('end'),yes)),iff(C('source-read::name-char',L('byte'),L('first')),C('implementation-scan',L('data'),add(L('at'),I(1)),L('end'),no),no)))),no)));
fields.push(fun('implementation',at,'I64',lets([['value',C('source-read::string',L('data'),L('at'),L('end'),I(1),I(128))]],iff(both(eq(F(L('value'),'code'),I(0)),C('implementation-scan',L('data'),F(L('value'),'start'),F(L('value'),'next'),yes)),F(L('value'),'next'),I(-1)))));
fields.push(fun('selector',at,'I64',iff(range(L('at'),1),iff(eq(get(L('at')),I(0)),ref(add(L('at'),I(1)),4),iff(eq(get(L('at')),I(1)),name(add(L('at'),I(1))),I(-1))),I(-1))));
fields.push(fun('record-field',[...at,['nominal','I64']],spanShape,iff(range(L('at'),1),iff(eq(get(L('at')),L('nominal')),iff(eq(L('nominal'),I(0)),lets([['next',ref(add(L('at'),I(1)),4)]],iff(lt(I(-1),L('next')),spanResult(I(0),L('next'),add(L('at'),I(1)),I(34)),spanResult(I(12),L('at'),I(0),I(0)))),C('source-read::name',L('data'),add(L('at'),I(1)),L('end'))),spanResult(I(12),L('at'),I(0),I(0))),spanResult(I(12),L('at'),I(0),I(0)))));
fields.push(fun('record-fields',[...at,['count','I64'],['nominal','I64'],['previous','I64'],['previous-length','I64']],'I64',iff(both(lt(I(-1),L('count')),lt(L('count'),I(100001)),range(L('at'),0)),iff(eq(L('count'),I(0)),L('at'),lets([['value',C('record-field',L('data'),L('at'),L('end'),L('nominal'))]],iff(both(eq(F(L('value'),'code'),I(0)),iff(eq(L('previous-length'),I(0)),yes,C('source-shape::name-less',L('data'),L('previous'),L('previous-length'),F(L('value'),'start'),F(L('value'),'length')))),lets([['next',typed(F(L('value'),'next'),9)]],iff(lt(I(-1),L('next')),C('record-fields',L('data'),L('next'),L('end'),sub(L('count'),I(1)),L('nominal'),F(L('value'),'start'),F(L('value'),'length')),I(-1))),I(-1)))),I(-1))));
fields.push(fun('record',at,'I64',iff(range(L('at'),1),lets([['nominal',get(L('at'))],['next',optref(L('at'),3)]],iff(lt(I(-1),L('next')),lets([['arguments',C('source-read::array',L('data'),L('next'),L('end'),I(2),I(33))]],iff(both(eq(F(L('arguments'),'code'),I(0)),iff(eq(L('nominal'),I(0)),eq(F(L('arguments'),'length'),I(0)),yes)),lets([['count',C('source-read::unsigned',L('data'),F(L('arguments'),'next'),L('end'))]],iff(both(eq(F(L('count'),'code'),I(0)),lt(I(0),F(L('count'),'value')),lt(F(L('count'),'value'),I(100001))),C('record-fields',L('data'),F(L('count'),'next'),L('end'),F(L('count'),'value'),sub(I(1),L('nominal')),I(0),I(0)),I(-1))),I(-1))),I(-1))),I(-1))));
save('source-fields',fields);
const only=(p,max)=>C('source-fields::only-zero',L('data'),p,L('end'),I(max));
const nonempty=(p,t,w=17)=>C('source-fields::nonempty-array',L('data'),p,L('end'),I(t),I(w));
const expressions=[];
expressions.push(fun('text',at,'I64',iff(range(L('at'),1),choose([
 [eq(get(L('at')),I(0)),span(C('source-read::string',L('data'),add(L('at'),I(1)),L('end'),I(0),I(65536)))],
 [eq(get(L('at')),I(1)),chain(add(L('at'),I(1)),[p=>typed(p,3,33),p=>lets([['size',C('source-read::integer',L('data'),p,L('end'))]],iff(both(eq(F(L('size'),'code'),I(0)),either(lt(I(0),F(L('size'),'low')),lt(I(0),F(L('size'),'high')))),F(L('size'),'next'),I(-1)))])]
],I(-1)),I(-1))));
expressions.push(fun('local',at,'I64',iff(range(L('at'),1),lets([['kind',get(L('at'))]],iff(lt(L('kind'),I(5)),typed(add(L('at'),I(1)),7),I(-1))),I(-1))));
// Function/operation parameters have tag7; lexical and payload bindings have tag8.
expressions[1]=fun('local',at,'I64',iff(range(L('at'),1),lets([['kind',get(L('at'))]],iff(lt(L('kind'),I(5)),C('source-read::typed',L('data'),add(L('at'),I(1)),L('end'),iff(lt(L('kind'),I(2)),I(7),I(8)),I(17)),I(-1))),I(-1)));
expressions.push(fun('body',[...at,['operation','I64']],'I64',choose([
 [eq(L('operation'),I(0)),L('at')],
 [eq(L('operation'),I(1)),tag(L('at'),1)],
 [eq(L('operation'),I(2)),number(C('source-read::signed',L('data'),L('at'),L('end')))],
 [either(eq(L('operation'),I(3)),eq(L('operation'),I(4))),C('text',L('data'),L('at'),L('end'))],
 [eq(L('operation'),I(5)),C('local',L('data'),L('at'),L('end'))],
 [eq(L('operation'),I(6)),ref(L('at'),3)],
 [eq(L('operation'),I(7)),chain(L('at'),[p=>typed(p,9),p=>typed(p,9),p=>typed(p,9)])],
 [eq(L('operation'),I(8)),chain(L('at'),[p=>nonempty(p,8),p=>typed(p,9)])],
 [eq(L('operation'),I(9)),nonempty(L('at'),9)],
 [eq(L('operation'),I(10)),chain(L('at'),[zero,zero,p=>ref(p,3),p=>array(p,2,33),zero,p=>array(p,9)])],
 [eq(L('operation'),I(11)),chain(L('at'),[zero,zero,p=>ref(p,3),p=>array(p,2,33),zero])],
 [eq(L('operation'),I(12)),chain(L('at'),[p=>typed(p,9),p=>array(p,9)])],
 [eq(L('operation'),I(13)),C('source-fields::record',L('data'),L('at'),L('end'))],
 [eq(L('operation'),I(14)),chain(L('at'),[p=>ref(p,5),p=>array(p,2,33),p=>optional(p,9)])],
 [eq(L('operation'),I(15)),chain(L('at'),[p=>typed(p,9),p=>C('source-fields::selector',L('data'),p,L('end'))])],
 [eq(L('operation'),I(16)),chain(L('at'),[p=>typed(p,2,33),p=>array(p,9)])]
],I(-2))));
save('source-expr',expressions);
const owners=[];
owners.push(fun('header',[...at,['key','I64'],['tag','I64'],['kind','I64']],'I64',chain(L('at'),[p=>single(p,1),p=>C('source-read::single',L('data'),p,L('end'),L('key')),p=>C('source-read::typed',L('data'),p,L('end'),L('tag'),I(17)),p=>C('source-read::single',L('data'),p,L('end'),L('kind'))])));
const implementation=p=>C('source-fields::implementation',L('data'),p,L('end'));
owners.push(fun('declaration-payload',[...at,['kind','I64']],'I64',iff(range(L('at'),1),lets([['payload',get(L('at'))],['start',add(L('at'),I(1))]],choose([
 [eq(L('payload'),I(0)),iff(eq(L('kind'),I(1)),chain(L('start'),[p=>array(p,17),p=>nonempty(p,4)]),I(-1))],
 [eq(L('payload'),I(1)),iff(eq(L('kind'),I(2)),chain(L('start'),[p=>array(p,17),p=>nonempty(p,5)]),I(-1))],
 [eq(L('payload'),I(3)),iff(eq(L('kind'),I(4)),chain(L('start'),[p=>array(p,17),p=>array(p,7),p=>typed(p,2,33),implementation]),I(-1))],
 [eq(L('payload'),I(4)),iff(eq(L('kind'),I(5)),chain(L('start'),[zero,zero,zero,p=>array(p,17),p=>array(p,7),p=>typed(p,2,33),p=>only(p,2),zero,zero,p=>only(p,1),p=>typed(p,9)]),iff(eq(L('kind'),I(6)),I(-2),I(-1)))],
 [eq(L('payload'),I(5)),iff(eq(L('kind'),I(7)),chain(L('start'),[p=>typed(p,2,33),p=>typed(p,9)]),I(-1))],
 [eq(L('payload'),I(6)),iff(eq(L('kind'),I(8)),chain(L('start'),[zero,p=>nonempty(p,11)]),I(-1))],
 [eq(L('payload'),I(7)),iff(eq(L('kind'),I(9)),chain(L('start'),[p=>typed(p,9),p=>typed(p,9),p=>single(p,0)]),I(-1))]
],iff(lt(L('payload'),I(12)),I(-2),I(-1)))),I(-1))));
owners.push(fun('declaration',[...at,['kind','I64']],'I64',chain(L('at'),[p=>typed(p,2),name,p=>tag(p,2),p=>C('declaration-payload',L('data'),p,L('end'),L('kind'))])));
owners.push(fun('parameter',at,'I64',chain(L('at'),[
 p=>iff(range(p,1),iff(eq(get(p),I(0)),typed(add(p,I(1)),3),iff(eq(get(p),I(1)),typed(add(p,I(1)),6),I(-1))),I(-1)),
 name,p=>typed(p,2,33),p=>tag(p,2),p=>optref(p,10)])));
owners.push(fun('port',at,'I64',chain(L('at'),[p=>typed(p,3),name,p=>typed(p,2,33),p=>iff(range(p,1),iff(eq(get(p),I(0)),typed(add(p,I(1)),9),iff(eq(get(p),I(1)),ref(add(p,I(1)),3),I(-1))),I(-1))])));
const header=(key,tag,kind)=>C('header',L('data'),I(19),L('end'),I(key),I(tag),I(kind));
const checked=(key,tag,kind,steps)=>chain(header(key,tag,kind),steps);
// Check a header before using its cursor; chain ordinarily checks each step's result.
const afterHeader=(key,tag,kind,steps)=>lets([['start',header(key,tag,kind)]],iff(lt(I(-1),L('start')),chain(L('start'),steps),L('start')));
owners.push(fun('body',[...data,['end','I64'],['record','I64']],'I64',choose([
 [eq(L('record'),I(0)),afterHeader(0,2,0,[name])],
 [eq(L('record'),I(1)),iff(range(I(19),20),lets([['kind',get(I(38))],['start',C('header',L('data'),I(19),L('end'),I(1),I(3),L('kind'))]],iff(lt(I(-1),L('start')),C('declaration',L('data'),L('start'),L('end'),L('kind')),I(-1))),I(-1))],
 [eq(L('record'),I(2)),afterHeader(2,17,10,[p=>typed(p,3),name,p=>tag(p,8)])],
 [eq(L('record'),I(4)),afterHeader(3,4,11,[p=>typed(p,3),name,p=>typed(p,2,33)])],
 [eq(L('record'),I(5)),afterHeader(4,5,12,[p=>typed(p,3),name,p=>optional(p,2,33)])],
 [eq(L('record'),I(7)),afterHeader(6,7,14,[p=>C('parameter',L('data'),p,L('end'))])],
 [eq(L('record'),I(8)),afterHeader(7,8,15,[name,p=>only(p,8),p=>single(p,1),p=>typed(p,9),p=>optional(p,2,33)])],
 [eq(L('record'),I(9)),chain(I(19),[p=>single(p,1),p=>typed(p,9),p=>iff(range(p,1),lets([['op',get(p)]],iff(lt(L('op'),I(52)),C('source-expr::body',L('data'),add(p,I(1)),L('end'),L('op')),I(-1))),I(-1))])],
 [eq(L('record'),I(11)),afterHeader(10,11,18,[p=>C('port',L('data'),p,L('end'))])],
 [eq(L('record'),I(12)),afterHeader(11,12,19,[name,p=>ref(p,3),p=>optref(p,11),p=>only(p,5)])]
],I(-2))));
owners.push(fun('owner',data,'I64',lets([['end',C('source-read::envelope',L('data'),'(text "LKJOWNER")','(text "lkjscript.kernel.owner-envelope")',I(4194304))]],iff(lt(I(-1),L('end')),iff(range(I(18),1),lets([['record',get(I(18))]],iff(lt(L('record'),I(17)),lets([['next',C('body',L('data'),L('end'),L('record'))]],iff(eq(L('next'),L('end')),I(0),iff(eq(L('next'),I(-2)),I(13),I(12)))),I(12))),I(12)),I(12)))));
save('source-owner',owners);
