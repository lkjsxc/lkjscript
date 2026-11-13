import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { runRoutes } from './batch.mjs';

async function fixture(t, body) {
    const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'lkj-batch-test-'));
    t.after(() => fs.rmSync(directory, {recursive: true, force: true}));
    const binary = path.join(directory, 'worker.mjs');
    fs.writeFileSync(binary, '#!' + process.execPath + '\n' + body);
    fs.chmodSync(binary, 0o755);
    const cases = Array.from({length: 9}, (_, index) => ({path: path.join(directory, index + '.k')}));
    return {binary, cases, options: {output: directory, encoding: null, memoryBytes: 'unlimited', timeout: 2000}};
}

const worker = String.raw`
import fs from 'node:fs';
import path from 'node:path';
const rows = fs.readFileSync(process.argv[3], 'utf8').trim().split('\n');
const engines = process.argv[5].split(',');
for (let index = 0; index < rows.length; index++) {
    const id = Number(path.basename(rows[index].split('\t')[2], '.k'));
    if (process.env.FAIL_CASE === String(id)) process.exit(3);
    if (process.env.INVALID_CASE === String(id)) { console.log('invalid json'); process.exit(0); }
    if (process.env.STALL_CASE === String(id)) await new Promise(resolve => setTimeout(resolve, 10000));
    for (const engine of [...engines].reverse()) {
        const checkStatus = id === 4 ? 1 : 0;
        console.log(JSON.stringify({index, engine, checkStatus, status: checkStatus || (id === 7 ? 1 : 0),
            stdoutHex: Buffer.from([id, 0, 255]).toString('hex'), stderrHex: id === 2 ? 'ff0080' : '',
            stderr: checkStatus ? 'source:1: E_TYPE rejection' : id === 7 ? 'source:8: R0003 bounds' : ''}));
    }
}
`;

test('shards preserve original indices, engine order, binary streams and verdicts', async t => {
    const {binary, cases, options} = await fixture(t, worker);
    const serial = await runRoutes(binary, cases, {...options, jobs: 1});
    const sharded = await runRoutes(binary, cases, {...options, jobs: 4});
    assert.deepEqual(sharded, serial);
    for (const engine of ['native', 'k']) {
        assert.deepEqual(sharded[engine].map(result => result.index), cases.map((_, index) => index));
        assert.equal(sharded[engine][4].checkStatus, 1);
        assert.deepEqual(sharded[engine][2].programStderr, Buffer.from([255, 0, 128]));
    }
});

test('failed shard retains completed records and maps only missing cases to global indices', async t => {
    const {binary, cases, options} = await fixture(t, worker);
    const result = await runRoutes(binary, cases, {...options, jobs: 3, env: {...process.env, FAIL_CASE: '4'}});
    for (const engine of ['native', 'k']) {
        assert.deepEqual(result[engine].filter(record => record.error).map(record => record.index), [4, 7]);
        assert.equal(result[engine][1].status, 0);
        assert.equal(result[engine][8].status, 0);
        assert.equal(result[engine][4].status, 3);
    }
});

test('malformed worker output belongs to that shard, and an idle timeout preserves progress', async t => {
    const {binary, cases, options} = await fixture(t, worker);
    const malformed = await runRoutes(binary, cases, {...options, jobs: 3, env: {...process.env, INVALID_CASE: '1'}});
    assert.deepEqual(malformed.k.filter(record => record.error).map(record => record.index), [1, 4, 7]);
    assert.match(malformed.k[4].error.message, /invalid batch output/);
    const stalled = await runRoutes(binary, cases, {...options, jobs: 3, timeout: 500, env: {...process.env, STALL_CASE: '4'}});
    assert.deepEqual(stalled.k.filter(record => record.error).map(record => record.index), [4, 7]);
    assert.equal(stalled.k[4].error.code, 'ETIMEDOUT');
    assert.equal(stalled.k[1].status, 0);
});

test('empty input launches no worker and invalid worker counts fail before execution', async () => {
    assert.deepEqual(await runRoutes('/does/not/exist', [], {jobs: 4}), {native: [], k: []});
    for (const jobs of [0, -1, 1.5, NaN, Infinity, '2']) {
        await assert.rejects(runRoutes('/does/not/exist', [], {jobs}), /positive integer/);
    }
});
