import assert from 'node:assert/strict';
import { test } from 'node:test';
import { startWorker } from '../bridge/worker.mjs';

test('resource failures reduce only pre-start reservation and preserve arguments', () => {
  const attempts=[];const data={entry:'main'};const instance={started:true};
  const result=startWorker('guest',{workerData:data},1024*1024,(url,options)=>{
    assert.equal(url,'guest');assert.equal(options.workerData,data);
    const bytes=options.resourceLimits.stackSizeMb*1024*1024;attempts.push(bytes);
    if(bytes>256*1024)throw Object.assign(new Error('EAGAIN'),{code:'ERR_WORKER_INIT_FAILED'});
    return instance;
  });
  assert.equal(result.worker,instance);assert.equal(result.stackBytes,256*1024);
  assert.deepEqual(attempts,[1024*1024,512*1024,256*1024]);
});

test('non-resource initialization errors are not retried', () => {
  let attempts=0;const failure=Object.assign(new Error('bad script'),{code:'ERR_WORKER_INIT_FAILED'});
  assert.throws(()=>startWorker('guest',{},1024,()=>{attempts++;throw failure;}),e=>e===failure);
  assert.equal(attempts,1);
});

test('successful construction returns immediately without interpreting outcomes', () => {
  const worker={outcome:'failure after effect'};let attempts=0;
  assert.equal(startWorker('guest',{},1024,()=>{attempts++;return worker;}).worker,worker);
  assert.equal(attempts,1);
});
