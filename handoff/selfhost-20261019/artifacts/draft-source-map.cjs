// Authoring proposal only. This does not write accepted graph objects.
const fs=require('fs');
const d=require('./source-author.cjs');
const {L,I,C,F,add,sub,eq,lt,iff,both,lets,fun,rec,save}=d;
function parse(text){const ts=text.replace(/^declarations\.(begin|end)\s*$/gm,'').match(/"(?:\\.|[^"\\])*"|[()]|[^\s()]+/g)||[];let at=0;function one(){const t=ts[at++];if(t!=='(')return t;const xs=[];while(ts[at]!==')'){if(at===ts.length)throw Error('unclosed');xs.push(one());}at++;return xs;}const out=[];while(at<ts.length)out.push(one());return out;}
function print(v){return Array.isArray(v)?'('+v.map(print).join(' ')+')':v;}
const mapModule=parse(fs.readFileSync('tools/selfhost/unit-map.lkjc','utf8'))[0][1];
const helpers=new Set(['prefix','group-end','children','content']);
function transform(v){if(!Array.isArray(v))return v;if(v[0]==='i64'){
 if(v[1]==='17')return ['local','key-bytes'];
 if(v[1]==='65')return ['local','value-bytes'];
 if(v[1]==='82')return ['call','ci::add',['local','key-bytes'],['local','value-bytes']];
}
 let out=v.map(transform);
 if(out[0]==='call'&&helpers.has(out[1]))out.splice(out[1]==='children'?out.length-1:out.length,0,['local','key-bytes'],['local','value-bytes']);
 return out;
}
const forms=mapModule.slice(3).filter(f=>f[0]==='function'&&helpers.has(f[2])).map(f=>{
 const result=transform(f),index=result.findIndex(f=>Array.isArray(f)&&(f[0]==='returns'||f[0]==='parameter'&&f.some(x=>Array.isArray(x)&&x[0]==='use')));
 result.splice(index,0,['parameter','create','key-bytes',['type','I64']],['parameter','create','value-bytes',['type','I64']]);
 const vis=result.find(f=>Array.isArray(f)&&f[0]==='visibility');vis[1]='private';return print(result);
});
const len=C('ci::length',L('data'));
forms.push(fun('ordered',[['data','Bytes'],['at','I64'],['end','I64'],['stride','I64'],['key-bytes','I64']],'Bool',
 iff(eq(L('at'),L('end')),'(bool true)',iff(lt(C('artifact-input::bytes-order',L('data'),sub(L('at'),L('stride')),L('at'),L('key-bytes')),I(0)),C('ordered',L('data'),add(L('at'),L('stride')),L('end'),L('stride'),L('key-bytes')),'(bool false)'))).replace('(visibility public)','(visibility private)'));
const failure=rec([['diagnostic',I(12)],['image',C('ci::text-bytes','(text "")')]]);
forms.push(fun('root',[['data','Bytes'],['key-bytes','I64'],['value-bytes','I64']],'(record (diagnostic I64) (image Bytes))',
 iff(both(lt(I(0),L('key-bytes')),lt(L('key-bytes'),I(33)),lt(I(0),L('value-bytes')),lt(L('value-bytes'),I(129)),lt(len,I(8388609))),
 lets([['stride',add(L('key-bytes'),L('value-bytes'))],['count',C('ci::div',len,L('stride'))]],
 iff(both(lt(L('count'),I(100001)),eq(C('ci::mul',L('count'),L('stride')),len)),
 iff(iff(eq(L('count'),I(0)),'(bool true)',C('ordered',L('data'),L('stride'),len,L('stride'),L('key-bytes'))),
 rec([['diagnostic',I(0)],['image',C('content',L('data'),I(0),len,L('key-bytes'),L('value-bytes'))]]),failure),failure)),failure)));
save('source-map',forms);
const probe=fun('read-map',[['data','Bytes']],'(record (diagnostic I64) (image Bytes))',iff(lt(len,I(8)),failure,C('source-map::root',C('ci::slice',L('data'),I(8),len),C('ci::u32',L('data'),I(0)),C('ci::u32',L('data'),I(4)))));
fs.writeFileSync('.artifacts/selfhost/source-map-probe-draft.lkjc','declarations.begin\n(units (module create source-map-probe\n'+probe+'\n(component create probes (visibility public) (port create run (type (function (Bytes) (record (diagnostic I64) (image Bytes)))) (function read-map)))) (target create source-map-preview (component source-map-probe::probes) (port source-map-probe::probes::run) (runner command)))\ndeclarations.end\n');
