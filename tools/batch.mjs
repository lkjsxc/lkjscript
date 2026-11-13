import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';

let serial = 0;
// Compilation and lowering are shared. Execution state and results never are.
export async function runRoutes(binary, cases, {
    engines = ['native', 'k'],
    output = 'target/batch-input',
    encoding = 'utf8',
    timeout = Number(process.env.LKJ_TEST_TIMEOUT_MS || 30000),
    ...options
} = {}) {
    if (!engines.length || new Set(engines).size !== engines.length
        || engines.some(engine => !['native', 'k'].includes(engine))) {
        throw new Error('batch engines must be distinct names from native,k');
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
export async function runCases(binary, cases, { engine = 'k', ...options } = {}) {
    const routes = await runRoutes(binary, cases, { ...options, engines: [engine] });
    return routes[engine];
}
