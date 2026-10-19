// Next-slice proposal only; no source-graph admission or normalization claim.
const fs=require('fs');
const {L,I,C,add,sub,eq,lt,iff,lets,fun,both}=require('./source-author.cjs');
const mul=(a,b)=>C('ci::mul',a,b);
const position=index=>add(L('at'),mul(index,L('stride')));
const compare=fun('compare',[['data','Bytes'],['left','I64'],['other','Bytes'],['right','I64'],['width','I64'],['index','I64']],'I64',
  iff(eq(L('index'),L('width')),I(0),lets([
    ['a',C('ci::get',L('data'),add(L('left'),L('index')))],
    ['b',C('ci::get',L('other'),add(L('right'),L('index')))],
  ],iff(lt(L('a'),L('b')),I(-1),iff(lt(L('b'),L('a')),I(1),
    C('compare',L('data'),L('left'),L('other'),L('right'),L('width'),add(L('index'),I(1))))))));
const parameters=[['data','Bytes'],['at','I64'],['count','I64'],['width','I64'],['stride','I64']];
const bounds=fun('bounds',parameters,'Bool',both(
  lt(I(0),L('width')),lt(L('width'),I(33)),lt(sub(L('width'),I(1)),L('stride')),lt(L('stride'),I(129)),
  lt(I(-1),L('count')),lt(L('count'),I(100001)),lt(C('ci::length',L('data')),I(8388609)),
  C('source-read::range',L('data'),L('at'),mul(L('count'),L('stride')),C('ci::length',L('data')))));
const order=fun('ordered',[...parameters,['index','I64']],'Bool',
  iff(lt(L('index'),L('count')),
    iff(lt(C('compare',L('data'),position(sub(L('index'),I(1))),L('data'),position(L('index')),L('width'),I(0)),I(0)),
      C('ordered',...parameters.map(([name])=>L(name)),add(L('index'),I(1))),'(bool false)'),'(bool true)'));
const validate=fun('validate',parameters,'I64',
  iff(C('bounds',...parameters.map(([name])=>L(name))),
    iff(C('ordered',...parameters.map(([name])=>L(name)),I(1)),I(0),I(12)),I(12)));
const find=fun('find',[...parameters,['key','Bytes'],['low','I64'],['high','I64']],'I64',
  iff(lt(L('low'),L('high')),lets([
    ['middle',add(L('low'),C('ci::div',sub(L('high'),L('low')),I(2)))],
    ['ordering',C('compare',L('data'),position(L('middle')),L('key'),I(0),L('width'),I(0))],
  ],iff(eq(L('ordering'),I(0)),L('middle'),iff(lt(L('ordering'),I(0)),
    C('find',...parameters.map(([name])=>L(name)),L('key'),add(L('middle'),I(1)),L('high')),
    C('find',...parameters.map(([name])=>L(name)),L('key'),L('low'),L('middle'))))),I(-1)));
// Lookup is bounds-safe but assumes validate has established strict ordering.
// -1 means absent; -2 means malformed bounds/key. It grants no admission authority.
const lookup=fun('lookup',[...parameters,['key','Bytes']],'I64',
  iff(both(C('bounds',...parameters.map(([name])=>L(name))),eq(C('ci::length',L('key')),L('width'))),
    C('find',...parameters.map(([name])=>L(name)),L('key'),I(0),L('count')),I(-2)));
const helpers=[compare,bounds,order,find].map(value=>value.replace('(visibility public)','(visibility private)'));
fs.writeFileSync('.artifacts/selfhost/source-index-draft.lkjc','declarations.begin\n(units (module create source-index\n'+[...helpers,validate,lookup].join('\n')+'\n))\ndeclarations.end\n');
