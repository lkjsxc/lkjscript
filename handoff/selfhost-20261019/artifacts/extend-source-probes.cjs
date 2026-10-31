const fs=require('fs');
const L=n=>`(local ${n})`,I=n=>`(i64 ${n})`,C=(n,...a)=>`(call ${n} ${a.join(' ')})`,F=(x,n)=>`(field ${x} (name ${n}))`;
const eq=(a,b)=>C('ci::eq',a,b),lt=(a,b)=>C('ci::lt',a,b),iff=(p,a,b)=>`(if ${p} ${a} ${b})`;
const lets=(xs,b)=>`(let ${xs.map(([n,v])=>`(binding ${n} ${v})`).join('\n')} (in ${b}))`;
const fun=(n,ps,r,b)=>`(function create ${n} (visibility public) (effect pure) ${ps.map(([n,t])=>`(parameter create ${n} (type ${t}))`).join(' ')} (returns ${r}) (body ${b}))`;
const empty=C('ci::octets','(list I64)');
const output=(code,image)=>`(record structural (field diagnostic ${code}) (field image ${image}))`;
const encode=vs=>{let binds='(binding observed-0 (type ByteBuffer) (call ci::empty))';for(let i=0;i<vs.length;i++)binds+='(binding observed-'+(i+1)+' (type ByteBuffer) '+C('wire::signed',vs[i],L('observed-'+i))+')';return '(let '+binds+' (in '+C('ci::freeze',L('observed-'+vs.length))+'))';};
const args=[L('data'),L('at'),L('end')];
const span=call=>lets([['value',call]],encode(['code','next','start','length'].map(n=>F(L('value'),n))));
const scalar=call=>lets([['value',call]],encode([...['code','next','value'].map(n=>F(L('value'),n)),I(0)]));
const cursor=call=>lets([['value',call]],encode([iff(lt(L('value'),I(0)),I(12),I(0)),L('value'),I(0),I(0)]));
const choices=[
 encode([I(0),I(0),iff(C('source-read::range',L('data'),L('at'),L('a'),L('end')),'(i64 1)','(i64 0)'),I(0)]),
 cursor(C('source-read::typed',...args,L('a'),L('b'))),
 span(C('source-read::array',...args,L('a'),L('b'))),
 span(C('source-read::string',...args,L('a'),L('b'))),
 span(C('source-read::name',...args)),
 cursor(C('source-read::reference',...args,L('a'))),
 cursor(C('source-read::single',...args,L('a'))),
 cursor(C('source-read::optional',...args,L('a'),L('b'))),
 scalar(C('source-read::signed',...args)),
 scalar(C('source-read::unsigned',...args))
];
let branch=empty;for(let i=choices.length-1;i>=0;i--)branch=iff(eq(L('op'),I(i)),choices[i],branch);
const rows=fun('observe-fields',[['data','Bytes'],...['op','at','end','a','b'].map(n=>[n,'I64'])],'Bytes',branch);
let packet=output(I(0),C('observe-fields',C('ci::slice',L('packet'),F(L('v4'),'next'),L('length')),...[0,1,2,3,4].map(i=>F(L('v'+i),'value'))));
for(let i=4;i>=0;i--)packet=lets([['v'+i,C('source-read::signed',L('packet'),i===0?I(0):F(L('v'+(i-1)),'next'),L('length'))]],iff(eq(F(L('v'+i),'code'),I(0)),packet,output(I(12),empty)));
const entry=fun('inspect-fields',[['packet','Bytes']],'(record (diagnostic I64) (image Bytes))',lets([['length',C('ci::length',L('packet'))]],packet));
const p='tests/fixtures/selfhost-source-probes.lkjc';let text=fs.readFileSync(p,'utf8');
if(text.includes('(function create inspect-fields'))throw Error('already extended');
text=text.replace('    (component create probes',rows+'\n'+entry+'\n    (component create probes');fs.writeFileSync(p,text);
