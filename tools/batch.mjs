import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';

let serial = 0;
// Compilation and lowering are shared. Execution state and results never are.
async function runShard(binary, cases, {
    engines = ['native', 'interpreter'],
    output = 'target/batch-input',
    encoding = 'utf8',
    timeout = Number(process.env.LKJSCRIPT_TEST_TIMEOUT_MS || 30000),
    ...options
} = {}) {
    if (!engines.length || new Set(engines).size !== engines.length
        || engines.some(engine => !['native', 'interpreter'].includes(engine))) {
        throw new Error('batch engines must be distinct names from native,interpreter');
    }
    fs.mkdirSync(output, { recursive: true });
    const manifest = path.resolve(output, process.pid + '-' + serial++ + '.tsv');
    const rows = cases.map(c => [c.mode || 'run', c.entry || '', path.resolve(c.path)].map(value => {
        if (/[\r\n\t]/.test(value)) throw new Error('batch field contains a line delimiter');
        return value;
    }).join('\t'));
    fs.writeFileSync(manifest, rows.length ? rows.join('\n') + '\n' : '');
    let result;
    try {
        result = await run(binary, ['batch', manifest, '--engines', engines.join(',')],
            { timeout: 0, idleTimeout: timeout, encoding: 'utf8', ...options });
    } finally {
        fs.rmSync(manifest, { force: true });
    }
    const routes = Object.fromEntries(engines.map(engine => [engine, new Array(cases.length)]));
    for (const line of result.stdout.split('\n').filter(Boolean)) {
        let record;
        try { record = JSON.parse(line); }
        catch { throw new Error('invalid batch output: ' + line); }
        const results = Object.hasOwn(routes, record.engine) ? routes[record.engine] : undefined;
        if (!results || !Number.isInteger(record.index) || record.index < 0
            || record.index >= cases.length || results[record.index]) {
            throw new Error('invalid or duplicate batch route/case index');
        }
        if (typeof record.stdoutHex !== 'string' || !/^(?:[0-9a-f]{2})*$/.test(record.stdoutHex) || typeof record.stderr !== 'string'
            || typeof record.stderrHex !== 'string' || !/^(?:[0-9a-f]{2})*$/.test(record.stderrHex)
            || ![0, 1].includes(record.checkStatus) || !Number.isInteger(record.status) || record.status < 0 || record.status > 255) {
            throw new Error('invalid batch result');
        }
        const stdout = Buffer.from(record.stdoutHex, 'hex'), stderr = Buffer.from(record.stderr);
        const programStderr = Buffer.from(record.stderrHex, 'hex');
        results[record.index] = {
            ...record,
            stdout: encoding ? stdout.toString(encoding) : stdout,
            stderr: encoding ? stderr.toString(encoding) : stderr,
            programStderr: encoding ? programStderr.toString(encoding) : programStderr,
            error: undefined, signal: null,
        };
    }
    for (const [engine, results] of Object.entries(routes)) {
        for (let i = 0; i < cases.length; i++) if (!results[i]) results[i] = {
            engine, checkStatus: null, status: result.status,
            stdout: encoding ? '' : Buffer.alloc(0),
            programStderr: encoding ? '' : Buffer.alloc(0),
            stderr: encoding ? result.stderr : Buffer.from(result.stderr),
            error: result.error || new Error('batch ended without ' + engine + ' case ' + i),
            signal: result.signal,
        };
        if (cases.length && (result.error || result.signal || result.status !== 0)
            && results.every(value => !value.error)) {
            results[results.length - 1].error = result.error
                || new Error('batch host failed after its final record: ' + result.stderr);
        }
    }
    return routes;
}
// A shard owns its process and local protocol indices. Merge only by the
// original input index, never by worker completion order.
export async function runRoutes(binary, cases, {
    jobs = 1,
    engines = ['native', 'interpreter'],
    encoding = 'utf8',
    ...options
} = {}) {
    if (!Number.isSafeInteger(jobs) || jobs < 1) throw new Error('batch jobs must be a positive integer');
    if (!engines.length || new Set(engines).size !== engines.length
        || engines.some(engine => !['native', 'interpreter'].includes(engine))) {
        throw new Error('batch engines must be distinct names from native,interpreter');
    }
    const routes = Object.fromEntries(engines.map(engine => [engine, new Array(cases.length)]));
    if (!cases.length) return routes;
    const shards = Array.from({length: Math.min(jobs, cases.length)}, () => []);
    for (let index = 0; index < cases.length; index++) shards[index % shards.length].push(index);
    const outcomes = await Promise.allSettled(shards.map(indices => runShard(binary,
        indices.map(index => cases[index]), {...options, engines, encoding})));
    for (let shard = 0; shard < shards.length; shard++) {
        const indices = shards[shard], outcome = outcomes[shard];
        for (const engine of engines) {
            for (let local = 0; local < indices.length; local++) {
                const index = indices[local];
                routes[engine][index] = outcome.status === 'fulfilled'
                    ? {...outcome.value[engine][local], index}
                    : {
                        index, engine, checkStatus: null, status: null,
                        stdout: encoding ? '' : Buffer.alloc(0),
                        programStderr: encoding ? '' : Buffer.alloc(0),
                        stderr: encoding ? '' : Buffer.alloc(0), signal: null,
                        error: outcome.reason instanceof Error ? outcome.reason : new Error(String(outcome.reason)),
                    };
            }
        }
    }
    return routes;
}

export async function runCases(binary, cases, { engine = 'interpreter', ...options } = {}) {
    const routes = await runRoutes(binary, cases, { ...options, engines: [engine] });
    return routes[engine];
}
