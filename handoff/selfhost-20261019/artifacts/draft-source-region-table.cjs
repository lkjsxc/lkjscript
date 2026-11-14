// Proposed workload only. Not authored into S and not a native support claim.
const fs=require('fs');
const {L,I,C,add,sub,eq,lt,iff,lets,fun,rec}=require('./source-author.cjs');
const pointer='(rel-ptr R I64)', cap='(region-cap R unique)';
const allocate=value=>`(region-alloc (type ${pointer}) arena ${value})`;
const resolve=offset=>`(region-resolve (type ${pointer}) arena ${offset})`;
const read=value=>`(region-read (type ${pointer}) arena ${value})`;
function generic(name,parameters,result,body){
  return fun(name,[...parameters,['arena',cap]],result,body)
    .replace('(effect pure)','(effect pure) (type-parameter create R (constraint region))')
    .replace(`(parameter create arena (type ${cap}))`,`(parameter create arena (type ${cap}) (use consume))`);
}
const fill=generic('fill',[['index','I64'],['count','I64']],cap,
  iff(lt(L('index'),L('count')),
    lets([['pointer',allocate(L('index'))]],C('fill','(types R)',add(L('index'),I(1)),L('count'),L('arena'))),L('arena')));
const sum=generic('sum',[['base','I64'],['stride','I64'],['index','I64'],['total','I64']],'I64',
  iff(eq(L('index'),I(0)),L('total'),lets([
    ['index-before',sub(L('index'),I(1))],
    ['pointer',resolve(add(L('base'),C('ci::mul',L('stride'),L('index-before'))))],
    ['value',read(L('pointer'))],
  ],C('sum','(types R)',L('base'),L('stride'),L('index-before'),add(L('total'),L('value')),L('arena')))));
const table=generic('table',[['count','I64']],'I64',lets([
  ['first',allocate(I(0))],['second',allocate(I(1))],
  ['base','(region-offset (local first))'],
  ['stride',sub('(region-offset (local second))',L('base'))],
  [`filled (type ${cap})`,C('fill','(types R)',I(2),L('count'),L('arena'))],
],C('sum','(types R)',L('base'),L('stride'),L('count'),I(0),L('filled'))));
const brand='(region-brand observe $table)', own=`(region-cap ${brand} unique)`;
const observe=fun('observe',[['count','I64']],'I64',
  iff(lt(L('count'),I(0)),I(-1),iff(lt(I(100000),L('count')),I(-1),
    `(region-with (binding arena (as $table) (type ${own})) (i64 8388608) (in ${C('table',`(types ${brand})`,L('count'),L('arena'))}))`)));
fs.writeFileSync('.artifacts/selfhost/source-region-table-draft.lkjc','declarations.begin\n(units (module create source-region-table\n'+[fill,sum,table,observe].join('\n')+'\n))\ndeclarations.end\n');
