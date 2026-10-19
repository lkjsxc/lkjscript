// Additional structural readers, still an unadmitted proposal.
const either=(...ps)=>ps.reduceRight((b,a)=>iff(a,yes,b),no);
const spanShape='(record (code I64) (next I64) (start I64) (length I64))';
const spanResult=(c,n,s,l)=>rec([['code',c],['next',n],['start',s],['length',l]]);
const spanFail=spanResult(I(12),L('at'),I(0),I(0));
forms.push(fun('typed',[...at,['tag','I64'],['width','I64']],'I64',
 iff(both(either(eq(L('width'),I(17)),eq(L('width'),I(33))),C('range',L('data'),L('at'),L('width'),L('end'))),
 iff(both(eq(C('ci::get',L('data'),L('at')),L('tag')),iff(eq(L('width'),I(17)),iff(C('artifact-check::zeros',L('data'),add(L('at'),I(1)),add(L('at'),I(17))),no,yes),yes)),add(L('at'),L('width')),I(-1)),I(-1))));
forms.push(fun('array-items',[...at,['tag','I64'],['width','I64'],['count','I64']],'I64',
 iff(both(lt(I(-1),L('count')),lt(L('count'),I(100001)),C('range',L('data'),L('at'),I(0),L('end'))),
 iff(eq(L('count'),I(0)),L('at'),lets([['next',C('typed',L('data'),L('at'),L('end'),L('tag'),L('width'))]],
 iff(lt(I(-1),L('next')),C('array-items',L('data'),L('next'),L('end'),L('tag'),L('width'),sub(L('count'),I(1))),I(-1)))),I(-1))));
forms.push(fun('array',[...at,['tag','I64'],['width','I64']],spanShape,
 lets([['count',C('unsigned',L('data'),L('at'),L('end'))]],
 iff(both(eq(F(L('count'),'code'),I(0)),lt(F(L('count'),'value'),I(100001)),either(eq(L('width'),I(17)),eq(L('width'),I(33))),C('range',L('data'),F(L('count'),'next'),mul(F(L('count'),'value'),L('width')),L('end'))),
 lets([['next',C('array-items',L('data'),F(L('count'),'next'),L('end'),L('tag'),L('width'),F(L('count'),'value'))]],
 iff(lt(I(-1),L('next')),spanResult(I(0),L('next'),F(L('count'),'next'),F(L('count'),'value')),spanFail)),spanFail))));
forms.push(fun('string',[...at,['minimum','I64'],['maximum','I64']],spanShape,
 lets([['count',C('unsigned',L('data'),L('at'),L('end'))]],
 iff(both(eq(F(L('count'),'code'),I(0)),lt(I(-1),L('minimum')),iff(lt(L('maximum'),L('minimum')),no,yes),iff(lt(F(L('count'),'value'),L('minimum')),no,yes),iff(lt(L('maximum'),F(L('count'),'value')),no,yes),C('range',L('data'),F(L('count'),'next'),F(L('count'),'value'),L('end'))),
 lets([['next',add(F(L('count'),'next'),F(L('count'),'value'))]],
 iff(C('types::utf8',L('data'),F(L('count'),'next'),L('next')),spanResult(I(0),L('next'),F(L('count'),'next'),F(L('count'),'value')),spanFail)),spanFail))));
forms.push(fun('name-char',[['value','I64'],['first','Bool']],'Bool',
 either(both(lt(I(64),L('value')),lt(L('value'),I(91))),both(lt(I(96),L('value')),lt(L('value'),I(123))),eq(L('value'),I(95)),iff(L('first'),no,either(both(lt(I(47),L('value')),lt(L('value'),I(58))),eq(L('value'),I(45)))))));
forms.push(fun('name-scan',[...at,['first','Bool']],'Bool',
 iff(C('range',L('data'),L('at'),I(0),L('end')),iff(eq(L('at'),L('end')),yes,
 iff(C('name-char',C('ci::get',L('data'),L('at')),L('first')),C('name-scan',L('data'),add(L('at'),I(1)),L('end'),no),no)),no)));
forms.push(fun('name',at,spanShape,
 lets([['value',C('string',L('data'),L('at'),L('end'),I(1),I(128))]],
 iff(both(eq(F(L('value'),'code'),I(0)),C('name-scan',L('data'),F(L('value'),'start'),F(L('value'),'next'),yes)),L('value'),spanFail))));
forms.push(fun('reference',[...at,['tag','I64']],'I64',
 lets([['next',C('typed',L('data'),L('at'),L('end'),I(18),I(17))]],
 iff(lt(I(-1),L('next')),C('typed',L('data'),L('next'),L('end'),L('tag'),I(17)),I(-1)))));
forms.push(fun('single',[...at,['expected','I64']],'I64',
 iff(C('range',L('data'),L('at'),I(1),L('end')),iff(eq(C('ci::get',L('data'),L('at')),L('expected')),add(L('at'),I(1)),I(-1)),I(-1))));
forms.push(fun('optional',[...at,['tag','I64'],['width','I64']],'I64',
 iff(C('range',L('data'),L('at'),I(1),L('end')),lets([['present',C('ci::get',L('data'),L('at'))]],
 iff(eq(L('present'),I(0)),add(L('at'),I(1)),iff(eq(L('present'),I(1)),C('typed',L('data'),add(L('at'),I(1)),L('end'),L('tag'),L('width')),I(-1)))),I(-1))));
