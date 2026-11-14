import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { seedProgram, mutationChoices, applyMutation, generateMutations, classifyRoute, compareRoutes } from './ir_mutate.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const seed = {path: 'calls.lkjir', entry: 'probe', text: fs.readFileSync(path.join(directory, 'ir-seeds/calls.lkjir'), 'utf8')};
const observed = (stderr = '', stdout = '', extra = {}) => classifyRoute({status: stderr ? 1 : 0, signal: null, stdout: Buffer.from(stdout), stderr: Buffer.from(stderr), ...extra});

test('seeded mutations replay by saved edit independently of generation and scheduling', () => {
    const first = generateMutations([seed], '1701', 64), again = generateMutations([seed], '1701', 64);
    assert.deepEqual(first, again);
    assert.deepEqual(generateMutations([seed], '1701', 20), first.slice(0, 20));
    assert.notDeepEqual(generateMutations([seed], '1702', 64), first);
    for (const mutation of first) {
        assert.notEqual(mutation.text, seed.text);
        assert.equal(applyMutation(seed.text, mutation), mutation.text);
        assert.equal(applyMutation(seed.text, JSON.parse(JSON.stringify(mutation))), mutation.text);
    }
    const edit = first[0].edits[0];
    assert.throws(() => applyMutation(seed.text.replace(edit.before, 'changed preimage'), first[0]), /preimage differs/);
    assert(new Set(first.map(mutation => mutation.kind)).size > 4);
});

test('pure seed validation excludes capability types and sealed raw operations', () => {
    seedProgram(seed.text);
    assert.throws(() => seedProgram(seed.text.replace('type "i64" int 64 signed', 'type "World" world')), /non-pure type/);
    assert.throws(() => seedProgram(seed.text.replace('@1 int 0 42', '@1 intrinsic 0 "raw_alloc"')), /raw intrinsics/);
    assert.throws(() => seedProgram(seed.text.replace('@1 int 0 42', '@1 intrinsic 0 "file_write"')), /host authority/);
    const candidates = mutationChoices(seed.text).flat();
    const recursive = candidates.find(mutation => mutation.kind === 'callee' && mutation.edits[0].after[0] === '@4 call 1 1 0');
    assert(recursive, 'the direct call seed must admit self-recursion without a signature change');
});

test('value bytes and trap kind/location remain exact, including equal bounds traps', () => {
    const value = observed('', '42\n');
    assert.deepEqual(compareRoutes(value, observed('', '42\n')), {category: 'value-equal', finding: false});
    assert.equal(compareRoutes(value, observed('', '43\n')).category, 'value-differs');
    const trap = observed('mutant.lkjir:19: R0003 index out of bounds\n');
    assert.deepEqual(compareRoutes(trap, observed('mutant.lkjir:19: R0003 out of bounds\n')), {category: 'trap-equal', finding: false});
    assert.equal(compareRoutes(trap, observed('mutant.lkjir:20: R0003 out of bounds\n')).category, 'trap-differs');
    assert.equal(compareRoutes(trap, observed('mutant.lkjir:19: R0002 division by zero\n')).category, 'trap-differs');
    assert.equal(compareRoutes(value, trap).category, 'ending-differs');
    const panic = observed('mutant.lkjir:8:3: R0016 panic payload\n');
    assert.equal(panic.code, 'R0004');
    assert.equal(panic.rawCode, 'R0016');
    assert.equal(panic.location.line, 8);
    assert.equal(panic.location.column, 3);
});

test('resource exhaustion remains a separate outcome without route comparison', () => {
    const stack = observed('mutant.lkjir:114: R0008 stack exhausted\n');
    const otherStack = observed('mutant.lkjir:62: R0008 stack exhausted\n');
    const heap = observed('mutant.lkjir:114: R0006 allocation failed\n');
    assert.deepEqual(compareRoutes(stack, otherStack), {category: 'resource', finding: false});
    assert.equal(stack.location.line, 114);
    assert.equal(otherStack.location.line, 62);
    assert.deepEqual(compareRoutes(stack, heap), {category: 'resource', finding: false});
    assert.deepEqual(compareRoutes(stack, observed('', '42\n')), {category: 'resource', finding: false});
});

test('timeouts, safety/model errors and process failures cannot masquerade as traps or values', () => {
    const timeout = observed('', '42\n', {error: Object.assign(new Error('deadline'), {code: 'ETIMEDOUT'}), signal: 'SIGKILL'});
    const safety = observed('mutant.lkjir:8: M_USE_AFTER_MOVE storage moved\n');
    const unsupported = observed('mutant.lkjir:8: M_UNSUPPORTED missing operation\n');
    const crash = observed('', '42\n', {signal: 'SIGSEGV', status: null});
    assert.equal(timeout.kind, 'did-not-terminate');
    assert.equal(compareRoutes(timeout, observed('', '42\n')).category, 'did-not-terminate');
    assert.deepEqual(compareRoutes(timeout, {kind: 'skipped', reason: 'independent interpreter did not terminate within its allowance'}), {category: 'did-not-terminate', finding: false});
    assert.equal(compareRoutes(safety, observed('', '42\n')).category, 'safety-error');
    assert.equal(compareRoutes(unsupported, observed('', '42\n')).category, 'model-error');
    assert.equal(compareRoutes(crash, observed('', '42\n')).category, 'host-error');
    assert.equal(compareRoutes(timeout, safety).finding, true);
});
