const fs=require('fs');const {L,I,C,F,add,eq,lt,iff,lets,fun,rec}=require('./source-author.cjs');
function parse(s){const ts=s.replace(/^declarations\.(begin|end)\s*$/gm,'').match(/"(?:\\.|[^"\\])*"|[()]|[^\s()]+/g)||[];let i=0;function one(){const t=ts[i++];if(t!=='(')return t;const xs=[];while(ts[i]!==')'){if(i>=ts.length)throw Error('unclosed');xs.push(one());}i++;return xs;}let out=[];while(i<ts.length)out.push(one());return out;}
const print=v=>Array.isArray(v)?'('+v.map(print).join(' ')+')':v;
const result='(record (diagnostic I64) (image Bytes))',empty=C('ci::octets','(list I64)');
const fail=rec([['diagnostic',I(12)],['image',empty]]);
const names=['op','start','end','a','b'];
const finish=rec([['diagnostic',I(0)],['image',C('observe-fields',C('ci::slice',L('packet'),L('at'),C('ci::length',L('packet'))),...names.map(L))]]);
const next=lets([['decoded (type (record (code I64) (next I64) (value I64)))',C('source-read::signed',L('packet'),L('at'),C('ci::length',L('packet')))]],iff(eq(F(L('decoded'),'code'),I(0)),C('read-fields',L('packet'),F(L('decoded'),'next'),add(L('index'),I(1)),...names.map((name,index)=>iff(eq(L('index'),I(index)),F(L('decoded'),'value'),L(name)))),fail));
const loop=fun('read-fields',[['packet','Bytes'],['at','I64'],['index','I64'],...names.map(n=>[n,'I64'])],result,iff(eq(L('index'),I(5)),finish,iff(lt(L('index'),I(5)),next,fail))).replace('(visibility public)','(visibility private)');
const entry=fun('inspect-fields',[['packet','Bytes']],result,C('read-fields',L('packet'),I(0),I(0),I(0),I(0),I(0),I(0),I(0)));
const path=process.argv[2]||'tests/fixtures/selfhost-source-probes.lkjc',forms=parse(fs.readFileSync(path,'utf8'));const mod=forms[0][1];const at=mod.findIndex(v=>Array.isArray(v)&&v[0]==='function'&&v[2]==='inspect-fields');if(at<0)throw Error('entry missing');mod.splice(at,1,parse(loop)[0],parse(entry)[0]);fs.writeFileSync(path,'declarations.begin\n'+forms.map(print).join('\n')+'\ndeclarations.end\n');
