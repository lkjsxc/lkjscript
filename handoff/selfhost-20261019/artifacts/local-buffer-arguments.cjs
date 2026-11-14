const fs=require('fs');
const files=process.argv.slice(2);
function parse(s){const ts=s.replace(/^declarations\.(begin|end)\s*$/gm,'').match(/"(?:\\.|[^"\\])*"|[()]|[^\s()]+/g)||[];let i=0;function one(){const t=ts[i++];if(t!=='(')return t;const a=[];while(ts[i]!==')'){if(i===ts.length)throw Error('Unclosed');a.push(one());}i++;return a;}const a=[];while(i<ts.length)a.push(one());return a;}
const all=fs.readdirSync('tools/selfhost').filter(n=>n.endsWith('.lkjc')).flatMap(n=>parse(fs.readFileSync('tools/selfhost/'+n,'utf8')));
const signatures=new Map;for(const unit of all)for(const mod of unit.slice(1))if(mod[0]==='module')for(const member of mod.slice(3))if(['function','external'].includes(member[0]))signatures.set(mod[2]+'::'+member[2],member.filter(v=>Array.isArray(v)&&v[0]==='parameter').map(p=>p.find(v=>Array.isArray(v)&&v[0]==='type')[1]));
let next=0,changed=0;
function rewrite(v,mod){if(!Array.isArray(v))return v;v=v.map(x=>rewrite(x,mod));if(v[0]!=='call')return v;const sig=signatures.get(v[1].includes('::')?v[1]:mod+'::'+v[1]);if(!sig)return v;const args=v.slice(2);if(args.some(a=>Array.isArray(a)&&a[0]==='types'))return v;
if(!args.some((a,i)=>sig[i]==='ByteBuffer'&&a[0]!=='local'))return v;
const bindings=[];const locals=args.map((arg,i)=>{if(Array.isArray(arg)&&arg[0]==='local')return arg;const name='wire-temp-'+next++;bindings.push(['binding',name,...(sig[i]==='ByteBuffer'?[['type','ByteBuffer']]:[]),arg]);return ['local',name];});changed++;return ['let',...bindings,['in',['call',v[1],...locals]]];}
function text(v){return Array.isArray(v)?'('+v.map(text).join(' ')+')':v;}
for(const file of files){const forms=parse(fs.readFileSync(file,'utf8'));for(const unit of forms)for(const mod of unit.slice(1))if(mod[0]==='module')for(let i=3;i<mod.length;i++)mod[i]=rewrite(mod[i],mod[2]);fs.writeFileSync(file,'declarations.begin\n'+forms.map(u=>'(units '+u.slice(1).map(m=>'(module create '+m[2]+'\n'+m.slice(3).map(text).join('\n')+'\n)').join('\n')+')').join('\n')+'\ndeclarations.end\n');}
process.stdout.write(JSON.stringify({buffer_calls_bound:changed})+'\n');
