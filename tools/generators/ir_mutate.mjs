#!/usr/bin/env node
// A direct attack on checked IR: no source frontend, source fixtures or rustc oracle.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { run } from '../process.mjs';
import { runCases } from '../batch.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const tokens = line => line.match(/"(?:\\.|[^"\\])*"|\S+/g) || [];
const control = new Set(['if', 'else', 'end', 'loop', 'break', 'continue', 'break-to', 'continue-to', 'return']);
const pureTypes = new Set(['unit', 'bool', 'int', 'f64', 'char', 'str', 'ref', 'array', 'slice', 'box', 'vec', 'struct', 'enum']);

export function seedProgram(text) {
    const lines = text.trimEnd().split('\n'), declarations = [], functions = [];
    let current;
    for (let line = 0; line < lines.length; line++) {
        const word = tokens(lines[line]);
        if (word[0] === 'type') {
            if (!pureTypes.has(word[2])) throw new Error('mutation seed has a non-pure type: ' + lines[line]);
            declarations.push({line, word});
        } else if (word[0] === 'fn') {
            current = {line, word, locals: [], statements: []};
            functions.push(current);
        } else if (word[0] === 'endfn') current = undefined;
        else if (word[0] === 'local' && current) current.locals.push({line, word});
        else if (word[0]?.startsWith('@') && current) {
            if (word[1] === 'intrinsic' && /^(?:raw_|world_|stdin_|stdout_|stderr_|directory_|file_)/.test(JSON.parse(word[3]))) {
                throw new Error('mutation seeds cannot contain host authority or raw intrinsics');
            }
            current.statements.push({line, word});
        }
    }
    if (!declarations.length || !functions.length) throw new Error('mutation seed needs IR types and functions');
    return {lines, declarations, functions};
}

// Edits retain IR's explicit source locations. A record describes the exact
// preimage/postimage, so a finding can be replayed without rerunning the PRNG.
export function mutationChoices(text) {
    const {lines, declarations, functions} = seedProgram(text);
    const groups = new Map();
    const add = (kind, edits) => {
        if (edits.every(edit => edit.after.length === 1 && edit.after[0] === lines[edit.line])) return;
        if (!groups.has(kind)) groups.set(kind, []);
        groups.get(kind).push({kind, edits: edits.map(edit => ({...edit, before: lines[edit.line]}))});
    };
    const replace = (kind, statement, position, value) => {
        if (statement.word[position] === String(value)) return;
        const word = [...statement.word]; word[position] = String(value);
        add(kind, [{line: statement.line, after: [word.join(' ')]}]);
    };
    for (const fn of functions) {
        const ordinary = fn.statements.filter(statement => !control.has(statement.word[1]));
        for (const statement of ordinary) {
            add('delete', [{line: statement.line, after: []}]);
            add('duplicate', [{line: statement.line, after: [lines[statement.line], lines[statement.line]]}]);
        }
        for (let index = 1; index < ordinary.length; index++) {
            const a = ordinary[index - 1], b = ordinary[index];
            add('swap', [{line: a.line, after: [lines[b.line]]}, {line: b.line, after: [lines[a.line]]}]);
        }
        for (const statement of fn.statements) {
            const op = statement.word[1];
            if (op === 'read') replace('copy-move', statement, 3, statement.word[3] === 'copy' ? 'move' : 'copy');
            if (op === 'borrow' || op === 'view-range') replace('borrow-kind', statement, 3, statement.word[3] === 'shared' ? 'mut' : 'shared');
            if (op === 'call') for (let callee = 0; callee < functions.length; callee++) replace('callee', statement, 3, callee);
            if (op === 'int') for (const value of ['-1', '0', '1', '42', '255', '9223372036854775807']) replace('constant', statement, 3, value);
            if (op === 'drop' || op === 'drop-if') for (const target of fn.statements) {
                if (target.line === statement.line) continue;
                add('move-drop', [{line: statement.line, after: []}, {line: target.line, after: [lines[statement.line], lines[target.line]]}]);
            }
            const operands = {
                read: [2, 4], borrow: [2, 4], assign: [2, 3], drop: [2], 'drop-if': [2],
                call: [2, ...statement.word.slice(4).map((_, index) => index + 4)],
                intrinsic: [2, ...statement.word.slice(4).map((_, index) => index + 4)],
                binary: [2, 4, 5], unary: [2, 4], cast: [2, 3],
                struct: statement.word.slice(2).map((_, index) => index + 2),
                array: statement.word.slice(2).map((_, index) => index + 2),
                variant: [2, ...statement.word.slice(4).map((_, index) => index + 4)],
                'tag-test': [2, 3], if: [2], return: [2],
            }[op] || [];
            for (const position of operands) if (/^\d+$/.test(statement.word[position] || '')) {
                for (let local = 0; local < fn.locals.length; local++) replace('operand', statement, position, local);
            }
        }
        for (const local of fn.locals) for (let ty = 0; ty < declarations.length; ty++) replace('local-type', local, 2, ty);
    }
    for (const declaration of declarations) {
        const kind = declaration.word[2];
        if (kind === 'ref' || kind === 'slice') replace('type-borrow-kind', declaration, 3, declaration.word[3] === 'shared' ? 'mut' : 'shared');
        if (kind === 'struct' || kind === 'enum') replace('type-copy-kind', declaration, 3, declaration.word[3] === 'copy' ? 'move' : 'copy');
        if (kind === 'array') {
            const length = BigInt(declaration.word[4]);
            for (const value of new Set([0n, 1n, 2n, length + 1n, length > 0n ? length - 1n : 0n])) replace('array-extent', declaration, 4, value);
        }
    }
    return [...groups.values()];
}

export function applyMutation(text, mutation) {
    const lines = text.trimEnd().split('\n');
    for (const edit of [...mutation.edits].sort((a, b) => b.line - a.line)) {
        if (lines[edit.line] !== edit.before) throw new Error('mutation preimage differs at line ' + (edit.line + 1));
        lines.splice(edit.line, 1, ...edit.after);
    }
    return lines.join('\n') + '\n';
}

export function generateMutations(seeds, seed, budget) {
    if (!Number.isSafeInteger(budget) || budget < 0 || !/^\d+$/.test(String(seed))) throw new Error('seed and budget must be nonnegative integers');
    const mask = (1n << 64n) - 1n;
    let state = (BigInt(seed) * 0x9e3779b97f4a7c15n & mask) || 1n;
    const pick = values => {
        state ^= (state << 13n) & mask; state ^= state >> 7n; state ^= (state << 17n) & mask;
        return values[Number(state % BigInt(values.length))];
    };
    const prepared = seeds.map(source => ({...source, groups: mutationChoices(source.text)}));
    if (!prepared.length || prepared.some(source => !source.groups.length)) throw new Error('every mutation seed needs editable IR');
    return Array.from({length: budget}, (_, index) => {
        const base = pick(prepared), mutation = pick(pick(base.groups));
        const text = applyMutation(base.text, mutation);
        return {index, base: base.path, entry: base.entry || 'probe', ...mutation, text};
    });
}

function diagnostic(text) {
    const found = text.match(/^(.*?):(\d+)(?::(\d+))?: (R\d+|[EM]_[A-Z_]+)\b([^\n]*)/m);
    const code = found?.[4] || text.match(/\b(?:[EM]_[A-Z_]+|R\d{4})\b/)?.[0];
    return {code, location: found ? {file: found[1], line: Number(found[2]), column: found[3] ? Number(found[3]) : null} : null};
}
export function classifyRoute(result) {
    const stdoutHex = Buffer.from(result.stdout || '').toString('hex');
    const stderr = Buffer.from(result.stderr || '').toString();
    const observed = {status: result.status, signal: result.signal || null, stdoutHex, stderr,
        error: result.error ? {code: result.error.code, message: result.error.message} : null};
    if (result.error?.code === 'ETIMEDOUT') return {...observed, kind: 'timeout'};
    if (result.error || result.signal) return {...observed, kind: 'host-error'};
    if (result.status === 0 && !stderr) return {...observed, kind: 'value'};
    const detail = diagnostic(stderr);
    if (detail.code?.startsWith('M_')) return {...observed, ...detail, kind: ['M_INVALID_IR', 'M_UNSUPPORTED'].includes(detail.code) ? 'model-error' : 'safety-error'};
    if (detail.code?.startsWith('R')) {
        const code = Number(detail.code.slice(1)) >= 16 ? 'R0004' : detail.code;
        return {...observed, ...detail, code, rawCode: detail.code, kind: ['R0006', 'R0008'].includes(code) ? 'resource' : 'trap'};
    }
    return {...observed, ...detail, kind: 'host-error'};
}

const sameLocation = (a, b) => a?.line === b?.line
    && (a?.column == null || b?.column == null || a.column === b.column);
export function compareRoutes(ir, native) {
    if ([ir, native].some(value => value.kind === 'safety-error')) return {category: 'safety-error', finding: true};
    if ([ir, native].some(value => value.kind === 'model-error')) return {category: 'model-error', finding: true};
    if ([ir, native].some(value => value.kind === 'host-error')) return {category: 'host-error', finding: true};
    if ([ir, native].some(value => value.kind === 'timeout')) return {category: 'timeout', finding: false};
    if (ir.kind === 'resource' || native.kind === 'resource') return {category: 'resource', finding: false};
    if (ir.kind !== native.kind) return {category: 'ending-differs', finding: true};
    if (ir.stdoutHex !== native.stdoutHex) return {category: 'value-differs', finding: true};
    if (ir.kind === 'value') return {category: 'value-equal', finding: false};
    const equal = ir.code === native.code && sameLocation(ir.location, native.location);
    return {category: equal ? 'trap-equal' : 'trap-differs', finding: !equal};
}

async function workers(count, jobs, action) {
    let cursor = 0;
    await Promise.all(Array.from({length: Math.min(jobs, count)}, async () => {
        while (cursor < count) { const index = cursor++; await action(index); }
    }));
}
const integer = (value, name) => {
    const number = Number(value);
    if (!Number.isSafeInteger(number) || number < 1) throw new Error(name + ' must be a positive integer');
    return number;
};
const save = (file, value) => fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n');

export async function main(argv = process.argv.slice(2)) {
    if (argv.length < 3 || argv.length > 4) throw new Error('usage: ir_mutate.mjs binary seed mutation-budget [output-directory]');
    const [binaryInput, seed, budgetText, outputInput] = argv;
    const budget = integer(budgetText, 'mutation budget');
    const jobs = integer(process.env.LKJSCRIPT_IR_MUTATION_JOBS || 1, 'LKJSCRIPT_IR_MUTATION_JOBS');
    const binarySource = path.resolve(binaryInput);
    const outputBase = path.resolve(outputInput || path.join(root, 'target/generated/ir-mutation'));
    fs.mkdirSync(outputBase, {recursive: true});
    const directory = fs.mkdtempSync(path.join(outputBase, 's' + seed + '-n' + budget + '-'));
    const binary = path.join(directory, 'compiler');
    fs.copyFileSync(binarySource, binary); fs.chmodSync(binary, 0o755);
    const config = path.join(root, 'tools/generators/ir-seeds.json');
    const seeds = JSON.parse(fs.readFileSync(config, 'utf8')).map(source => ({...source, text: fs.readFileSync(path.join(root, source.path), 'utf8')}));
    const mutants = generateMutations(seeds, seed, budget);
    const files = mutants.map(mutant => {
        const file = path.join(directory, mutant.index + '.lkjir'); fs.writeFileSync(file, mutant.text); return file;
    });
    const seedFiles = seeds.map((source, index) => {
        const file = path.join(directory, 'seed-' + index + '.lkjir'); fs.writeFileSync(file, source.text); return file;
    });
    const options = {cwd: root, encoding: null};
    const seedChecks = await runCases(binary, seeds.map((source, index) => ({path: seedFiles[index], mode: 'check', entry: source.entry})),
        {...options, output: directory, jobs});
    for (let index = 0; index < seeds.length; index++) if (seedChecks[index].error || seedChecks[index].checkStatus !== 0) {
        throw new Error('canonical seed failed: ' + seeds[index].path + ': ' + seedChecks[index].stderr.toString());
    }
    const checks = await runCases(binary, mutants.map((mutant, index) => ({path: files[index], mode: 'check', entry: mutant.entry})),
        {...options, output: directory, jobs});
    const results = new Array(budget), started = performance.now();
    await workers(budget, jobs, async index => {
        const mutant = mutants[index], check = checks[index];
        let result;
        if (check.error || check.signal || ![0, 1].includes(check.checkStatus)) {
            result = {category: 'checker-host-error', finding: true, checker: classifyRoute(check)};
        } else if (check.checkStatus === 1) {
            const detail = diagnostic(check.stderr.toString());
            result = {category: detail.code?.startsWith('E_') ? 'rejected' : 'checker-host-error', finding: !detail.code?.startsWith('E_'), checker: {...detail, stderr: check.stderr.toString()}};
        } else {
            const runRoute = async engine => {
                const time = performance.now();
                const processResult = await run(binary, ['run', files[index], '--entry', mutant.entry, '--engine', engine, ...(engine === 'interpreter' ? ['--unchecked-ir'] : [])], options);
                return {...classifyRoute(processResult), milliseconds: performance.now() - time};
            };
            const ir = await runRoute('interpreter');
            const native = ir.kind === 'timeout'
                ? {kind: 'skipped', reason: 'independent interpreter timed out'}
                : ['safety-error', 'model-error'].includes(ir.kind)
                    ? {kind: 'skipped', reason: 'independent model found a contract violation'}
                    : await runRoute('native');
            result = {...compareRoutes(ir, native), interpreter: ir, native};
        }
        results[index] = {index, base: mutant.base, kind: mutant.kind, ...result};
        if (result.finding || result.category === 'timeout') {
            save(path.join(directory, index + '.result.json'), {...results[index], edits: mutant.edits});
        }
    });
    const counts = {}, routeCounts = {interpreter: {}, native: {}}, operators = {};
    for (const result of results) {
        counts[result.category] = (counts[result.category] || 0) + 1;
        operators[result.kind] = (operators[result.kind] || 0) + 1;
        for (const engine of ['interpreter', 'native']) if (result[engine]) routeCounts[engine][result[engine].kind] = (routeCounts[engine][result[engine].kind] || 0) + 1;
    }
    const summary = {seed, budget, counts, routeCounts, operators, findings: results.filter(result => result.finding).length,
        executionSeconds: (performance.now() - started) / 1000, directory};
    process.stdout.write('IR mutation: ' + JSON.stringify(summary) + '\n');
    for (const result of results.filter(result => result.finding)) process.stderr.write('FINDING ' + result.index + ': ' + result.category + '\n');
    if (summary.findings) process.exitCode = 1;
    return summary;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    await main().catch(error => { process.stderr.write(error.stack + '\n'); process.exitCode = 1; });
}
