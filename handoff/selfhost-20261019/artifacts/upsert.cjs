const fs=require('fs');
const kinds=new Set(['module','function','external','parameter','type-parameter','component','port','target']);
function parse(source) {
  const tokens=source.replace(/^declarations\.(begin|end)\s*$/gm,'').match(/"(?:\\.|[^"\\])*"|[()]|[^\s()]+/g)||[];
  let i=0;
  function one(){const t=tokens[i++];if(t!== '(')return t;let a=[];while(tokens[i]!==')'){if(i>=tokens.length)throw Error('Unclosed form');a.push(one());}i++;return a;}
  let forms=[];while(i<tokens.length)forms.push(one());return forms;
}
function rewrite(v){if(!Array.isArray(v))return v;if(kinds.has(v[0])&&v[1]==='create'){v=[`upsert-${v[0]}`,...v.slice(2)];}return v.map(rewrite);}
function print(v){return Array.isArray(v)?'('+v.map(print).join(' ')+')':v;}
let units=[];for(const p of process.argv.slice(2)){for(const u of parse(fs.readFileSync(p,'utf8'))){if(u[0]!=='units')throw Error('Expected units');units.push(...u.slice(1));}}
if(process.env.SELFHOST_ONLY){const names=new Set(process.env.SELFHOST_ONLY.split(','));function qualify(v,mod){if(!Array.isArray(v))return v;if(v[0]==='call'&&typeof v[1]==='string'&&!v[1].includes('::'))v=[v[0],mod+'::'+v[1],...v.slice(2)];return v.map(x=>qualify(x,mod));}units=units.map(u=>u[0]==='module'?[...u.slice(0,3),...u.slice(3).filter(v=>Array.isArray(v)&&names.has(v[2])).map(v=>qualify(v,u[2]))]:u);}
fs.writeFileSync('.artifacts/selfhost/current.lkjx',print(['tx',['base','@head'],...units.map(rewrite)])+'\n');
