const fs = require('fs');
const {L,I,C,F,eq,lt,iff,lets,fun,choose} = require('./source-author.cjs');
function parse(s) {
  const tokens = s.replace(/^declarations\.(begin|end)\s*$/gm, '').match(/"(?:\\.|[^"\\])*"|[()]|[^\s()]+/g) || [];
  let at = 0;
  function one() {
    const token = tokens[at++];
    if (token !== '(') return token;
    const values = [];
    while (tokens[at] !== ')') {
      if (at >= tokens.length) throw Error('Unclosed form');
      values.push(one());
    }
    at++;
    return values;
  }
  const forms = [];
  while (at < tokens.length) forms.push(one());
  return forms;
}
const print = value => Array.isArray(value) ? '(' + value.map(print).join(' ') + ')' : value;
const path = process.argv[2] || 'tests/fixtures/selfhost-source-probes.lkjc';
const forms = parse(fs.readFileSync('.artifacts/selfhost/source-probes-baseline.lkjc', 'utf8'));
const moduleForm = forms[0][1];
const out = (...values) => C('pack-fields', ...values);
const argumentsFor = count => ['data','at','end','a','b'].slice(0,count).map(L);
const scalar = (name,count) => lets([['value (type I64)',C('source-read::'+name,...argumentsFor(count))]],out(iff(lt(L('value'),I(0)),I(12),I(0)),L('value'),I(0),I(0)));
const record = (name,count,fields) => lets([['value (type (record '+fields.filter(Boolean).map(field=>'('+field+' I64)').join(' ')+'))',C('source-read::'+name,...argumentsFor(count))]],out(...fields.map(name => name === null ? I(0) : F(L('value'),name))));
const branches = [
  out(I(0),I(0),iff(C('source-read::range',L('data'),L('at'),L('a'),L('end')),I(1),I(0)),I(0)),
  scalar('typed',5),
  record('array',5,['code','next','start','length']),
  record('string',5,['code','next','start','length']),
  record('name',3,['code','next','start','length']),
  scalar('reference',4),
  scalar('single',4),
  scalar('optional',5),
  record('signed',3,['code','next','value',null]),
  record('unsigned',3,['code','next','value',null]),
];
const observed = moduleForm.find(value => Array.isArray(value) && value[0] === 'function' && value[2] === 'observe-fields');
const body = observed.findIndex(value => Array.isArray(value) && value[0] === 'body');
observed[body] = parse('(body '+choose(branches.map((value,index) => [eq(L('op'),I(index)),value]),C('ci::octets','(list I64)'))+')')[0];
const names = ['code','next','start','length'];
const packed = fun('pack-fields',names.map(name => [name,'I64']),'Bytes',lets([
  ['output-0 (type ByteBuffer)',C('ci::empty')],
  ...names.map((name,index) => ['output-'+(index+1)+' (type ByteBuffer)',C('wire::signed',L(name),L('output-'+index))]),
],C('ci::freeze',L('output-4')))).replace('(visibility public)','(visibility private)');
moduleForm.splice(moduleForm.indexOf(observed),0,parse(packed)[0]);
fs.writeFileSync(path,'declarations.begin\n'+forms.map(print).join('\n')+'\ndeclarations.end\n');
