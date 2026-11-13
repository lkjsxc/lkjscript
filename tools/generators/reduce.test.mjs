import test from 'node:test';
import assert from 'node:assert/strict';
import { statementSpans, preservesFailure, reduceSource, classify } from './reduce.mjs';

test('deletion spans respect literals, comments, lifetimes, branches and macro token trees', () => {
  const source = [
    "fn probe<'a>(text: &'a str) -> i64 {",
    'let raw = r##"{ fake(); }"##;',
    'let quoted = "; \\" }"; /* outer /* fake(); */ comment */',
    "let character = '}';",
    'if true { let inner = 1; sink(inner); } else { sink(2); }',
    'vec![{ let macro_tokens = 3; macro_tokens }; 2];',
    '42',
    '}',
  ].join('\n');
  const pieces = statementSpans(source).map(span => source.slice(span.start, span.end));
  assert(pieces.includes('let raw = r##"{ fake(); }"##;'));
  assert(pieces.includes('let inner = 1;'));
  assert(pieces.includes('sink(inner);'));
  assert(pieces.includes('if true { let inner = 1; sink(inner); } else { sink(2); }'));
  assert(pieces.includes('vec![{ let macro_tokens = 3; macro_tokens }; 2];'));
  assert(!pieces.includes('fake();'));
  assert(!pieces.includes('let macro_tokens = 3;'));
  assert(!pieces.some(piece => piece.startsWith("'a")));
});

test('same category cannot replace Rust-valid evidence with an invalid candidate', () => {
  const baseline = {category: 'rejects-valid:E_BORROW', rustAccepted: true, lkCode: 'E_BORROW'};
  assert(preservesFailure(baseline, {...baseline}));
  assert(!preservesFailure(baseline, {...baseline, rustAccepted: false}));
  assert(!preservesFailure(baseline, {...baseline, rustAccepted: null}));
  assert(!preservesFailure(baseline, {...baseline, category: 'rejects-valid:E_MOVE'}));
  assert(!preservesFailure(baseline, {...baseline, lkCode: 'E_MOVE'}));
  const rejected = {category: 'ACCEPTS-RUST-REJECTED', rustAccepted: false, rustCode: 'E0382'};
  assert(!preservesFailure(rejected, {...rejected, rustCode: 'E0308'}));
});

test('fabricated evaluator keeps both validity and failure witnesses while deleting noise', async () => {
  const source = 'fn probe() { declare(); noise(); witness(); }';
  const evaluate = async candidate => ({
    category: candidate.includes('witness();') ? 'rejects-valid:E_BORROW' : 'rejects-valid:E_MOVE',
    rustAccepted: candidate.includes('declare();') && candidate.includes('fn probe()'),
    lkCode: candidate.includes('witness();') ? 'E_BORROW' : 'E_MOVE',
  });
  const reduced = await reduceSource(source, evaluate);
  assert(reduced.source.includes('declare();'));
  assert(reduced.source.includes('witness();'));
  assert(!reduced.source.includes('noise();'));
  assert(reduced.accepted > 0);
  assert(preservesFailure(reduced.baseline, reduced.observation));
  for (const span of statementSpans(reduced.source)) {
    const candidate = reduced.source.slice(0, span.start) + reduced.source.slice(span.end);
    assert(!preservesFailure(reduced.baseline, await evaluate(candidate)));
  }
});

test('explicit requested category must match and agreement is not an automatic failure', async () => {
  const observation = {category: 'value-equal', rustAccepted: true};
  await assert.rejects(reduceSource('fn probe() {}', async () => observation), /not a reducible/);
  await assert.rejects(reduceSource('fn probe() {}', async () => observation, {category: 'VALUE-DIFFERS'}), /expected category/);
});

test('matching output prefix cannot hide an unexpected crash or wrong trap', () => {
  const bytes = Buffer.from([0, 255, 65]);
  assert.equal(classify({kind: 'value', stdout: bytes}, {kind: 'crash', stdout: bytes}, 'output'), 'crash');
  assert.equal(classify({kind: 'trap', code: 'R0005', stdout: bytes}, {kind: 'trap', code: 'R0001', stdout: bytes}, 'output'), 'different-ending');
  assert.equal(classify({kind: 'trap', code: 'R0005', stdout: bytes}, {kind: 'trap', code: 'R0005', stdout: bytes}, 'output'), 'equal-output-trap');
  assert.equal(classify({kind: 'value', stdout: bytes}, {kind: 'value', stdout: Buffer.from([0])}, 'output'), 'different-output');
});

test('World outcome uses the private record even for a marker-shaped stderr and exit 101', async () => {
  const {worldOutcome} = await import('../oracle.mjs');
  const control = (code, status = 0, reserved = 0) => {
    const bytes = Buffer.alloc(24);
    [code, reserved, status].forEach((value, index) => bytes.writeBigUInt64LE(BigInt(value), index * 8));
    return bytes;
  };
  const result = {status: 101, signal: null, stderr: Buffer.from('LKJ_RUST_TRAP R0004\n'), control: control(0, 101)};
  assert.deepEqual(worldOutcome(result), {kind: 'normal', status: 101});
  assert.deepEqual(worldOutcome({...result, stderr: Buffer.alloc(0), control: control(4)}), {kind: 'trap', code: 'R0004'});
  for (const invalid of [undefined, Buffer.alloc(23), Buffer.concat([control(4), control(4)]), control(4, 1), control(4, 0, 1), control(0, 0), control(0, 256)]) {
    assert.deepEqual(worldOutcome({...result, control: invalid}), {kind: 'crash'});
  }
  assert.deepEqual(worldOutcome({...result, signal: 'SIGKILL'}), {kind: 'crash'});
  assert.deepEqual(worldOutcome({...result, error: new Error('incomplete capture')}), {kind: 'crash'});
});

test('oracle cache preserves binary stdout, stderr and control independently', async () => {
  const {packResult, unpackResult} = await import('../oracle.mjs');
  const result = {status: 101, stdout: Buffer.from([0, 255, 128]), stderr: Buffer.from([255, 0, 128]), control: Buffer.alloc(24)};
  result.control.writeBigUInt64LE(101n, 16);
  const restored = unpackResult(JSON.parse(JSON.stringify(packResult(result))), null);
  assert.equal(restored.status, result.status);
  for (const stream of ['stdout', 'stderr', 'control']) assert.deepEqual(restored[stream], result[stream]);
  assert.equal(unpackResult({status: 0, stdoutHex: '', stderr: 'old cache'}).stderr, 'old cache');
});

test('World reduction compares deliberate status and binary program stderr', () => {
  const reference = {kind: 'value', status: 101, stdout: Buffer.alloc(0), programStderr: Buffer.from([255, 0, 128])};
  assert.equal(classify(reference, {...reference}, 'output'), 'equal-output-exit');
  assert.equal(classify(reference, {...reference, status: 0}, 'output'), 'different-ending');
  assert.equal(classify(reference, {...reference, programStderr: Buffer.from([239, 191, 189, 0, 239, 191, 189])}, 'output'), 'different-stderr');
});
