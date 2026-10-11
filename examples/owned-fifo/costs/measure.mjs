// Public-product orchestration and an independent ordinary-data oracle only.
// All native program meaning enters through the retained literal requests.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const names = ['library', 'steps', 'trace', 'elements', 'application'];
const lanes = ['baseline-structural', 'candidate-structural', 'candidate-nominal'];
const orders = [[0, 1, 2], [2, 1, 0], [1, 2, 0], [0, 2, 1], [2, 0, 1], [1, 0, 2]];
const sha256 = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const json = (file, value) => fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
const action = (kind, value = 0) => ({ kind, value });

function balanced(source) {
  let depth = 0;
  let comment = false;
  let quoted = false;
  let escaped = false;
  for (const character of source) {
    if (comment) {
      if (character === '\n') comment = false;
    } else if (quoted) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') quoted = false;
    } else if (character === ';') comment = true;
    else if (character === '"') quoted = true;
    else if (character === '(') depth++;
    else if (character === ')') assert(--depth >= 0, 'unexpected closing parenthesis');
  }
  assert.equal(depth, 0, 'unclosed parenthesis');
  assert.equal(quoted, false, 'unclosed string');
}

function records(output) {
  return output.trim().split('\n').filter(Boolean).map(line => {
    const tokens = line.match(/(?:[^\s"]+|"(?:\\.|[^"\\])*")+/g);
    assert(tokens?.length, `invalid compact line: ${line}`);
    const [name, ...fields] = tokens;
    const values = Object.create(null);
    for (const field of fields) {
      const separator = field.indexOf('=');
      assert(separator > 0, `invalid compact field: ${field}`);
      const key = field.slice(0, separator);
      const value = field.slice(separator + 1);
      assert(!Object.hasOwn(values, key), `duplicate compact field: ${key}`);
      values[key] = value.startsWith('"') ? JSON.parse(value) : value;
    }
    return { name, fields: values };
  });
}

function record(all, name) {
  const selected = all.filter(item => item.name === name);
  assert.equal(selected.length, 1, `expected one ${name} record`);
  return selected[0].fields;
}

function burst(length) {
  const result = Array.from({ length }, (_, i) => action(0, i % 256));
  for (let i = 0; i < length; i++) {
    result.push(action(2), action(1));
    if (i % 3 === 0) result.push(action(0, 255 - i % 256));
  }
  return result;
}

function mixed() {
  let state = 0xc39c6592;
  return Array.from({ length: 512 }, () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    const decision = state % 16;
    return action(decision === 0 ? 3 : decision <= 5 ? 1 : decision <= 8 ? 2 : 0,
      (state >>> 8) & 255);
  });
}

function expected(actions, mode) {
  assert(mode >= 0 && mode < 3);
  const encode = value => mode === 0 ? value : value * 255 + (mode === 1 ? 255 : 286);
  // Ordinary host FIFO independent of the native two-stack algorithm.
  let queue = [];
  const events = actions.map(input => {
    assert(input.kind >= 0 && input.kind <= 3);
    let observed;
    if (input.kind === 0) {
      observed = encode(input.value);
      queue.push(observed);
    } else if (input.kind === 1) observed = queue.shift();
    else if (input.kind === 2) observed = queue[0];
    else queue = [];
    return { kind: input.kind, present: observed !== undefined, value: observed ?? 0, length: queue.length };
  });
  return { events, remaining: queue, 'empty-length': 0, reused: [encode(37)] };
}

function workloads() {
  const traces = [0, 31, 33, 256, 1024].map(length => ({ name: `burst-${length}`, actions: burst(length) }));
  traces.push({ name: 'mixed-512', actions: mixed() });
  return ['cells', 'buffers', 'packets'].flatMap((carrier, mode) => traces.map(trace => ({
    name: `${carrier}-${trace.name}`, target: `fifo-${carrier}`,
    arguments: [trace.actions], expected: expected(trace.actions, mode),
  })));
}

function verifyLiterals(inputs) {
  const structural = fs.readFileSync(path.join(here, 'structural.lkjc'), 'utf8');
  const nominal = fs.readFileSync(path.join(here, 'nominal.lkjc'), 'utf8');
  balanced(structural);
  balanced(nominal);
  assert.equal([...structural.matchAll(/\(function create fifo-/g)].length, 7);
  assert.equal([...nominal.matchAll(/\(function create fifo-/g)].length, 7);
  const shape = '(owned-product (field incoming (owned-sequence T)) (field outgoing (owned-sequence T)))';
  const declaration = '    (owned-record create Fifo (visibility public)\n' +
    '      (type-parameter create T (constraint owned))\n' +
    '      (field create incoming (type (owned-sequence T)))\n' +
    '      (field create outgoing (type (owned-sequence T))))\n';
  const normalize = source => source.replace(/\s+/g, ' ').trim();
  assert.equal(normalize(nominal.replace(declaration, '').replaceAll('(Fifo T)', shape)), normalize(structural),
    'representation proposals must retain identical algorithms');
  if (inputs) {
    const fifoShape = /\(owned-product\s*\(field incoming \(owned-sequence ([^()\s]+)\)\)\s*\(field outgoing \(owned-sequence \1\)\)\)/g;
    for (const name of names) {
      const old = fs.readFileSync(path.join(inputs, 'structural', `${name}.lkjc`), 'utf8');
      const next = fs.readFileSync(path.join(inputs, 'nominal', `${name}.lkjc`), 'utf8');
      balanced(old);
      balanced(next);
      assert.equal(normalize(old.replace(fifoShape, '($std-fifo $1)')),
        normalize(next.replaceAll('(std::Fifo ', '($std-fifo ')), `${name}: consumer behavior changed`);
    }
  }
}

function selfCheck(inputs) {
  verifyLiterals(inputs);
  assert.equal(workloads().length, 18);
  assert.deepEqual(expected([action(0, 3), action(0, 5), action(2), action(1)], 0), {
    events: [{ kind: 0, present: true, value: 3, length: 1 }, { kind: 0, present: true, value: 5, length: 2 },
      { kind: 2, present: true, value: 3, length: 2 }, { kind: 1, present: true, value: 3, length: 1 }],
    remaining: [5], 'empty-length': 0, reused: [37],
  });
  assert.deepEqual(expected([action(0, 255), action(3), action(2), action(1)], 2), {
    events: [{ kind: 0, present: true, value: 65311, length: 1 }, { kind: 3, present: false, value: 0, length: 0 },
      { kind: 2, present: false, value: 0, length: 0 }, { kind: 1, present: false, value: 0, length: 0 }],
    remaining: [], 'empty-length': 0, reused: [9721],
  });
  const nested = JSON.stringify({ live_handles_after: 0, quote: 'a "quoted" value' });
  const compact = `execution production-observation=${JSON.stringify(nested)} invocation-nanoseconds=123\n`;
  assert.equal(record(records(compact), 'execution')['production-observation'], nested);
  const next = record(records('next order=1 kind=status operation=status name=selected\n'), 'next');
  assert.equal(next.operation, 'status');
  assert.equal(next.kind, 'status');
  assert.equal(next.name, 'selected');
  assert.throws(() => records('next operation=status operation=check\n'), /duplicate compact field: operation/);
}

function command(lane, args, label) {
  const started = process.hrtime.bigint();
  const child = spawnSync(lane.executable, args, {
    cwd: lane.directory, env: { PATH: '' }, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024,
  });
  const wallNanoseconds = Number(process.hrtime.bigint() - started);
  const serial = lane.serial++;
  const prefix = path.join(lane.directory, 'commands', `${String(serial).padStart(4, '0')}-${label}`);
  fs.writeFileSync(`${prefix}.stdout`, child.stdout ?? '', { flag: 'wx' });
  fs.writeFileSync(`${prefix}.stderr`, child.stderr ?? '', { flag: 'wx' });
  json(`${prefix}.json`, { args, status: child.status, signal: child.signal, wallNanoseconds, error: child.error?.message });
  assert.equal(child.status, 0, `${lane.name}/${label}: ${child.stderr}\n${child.stdout}`);
  return { records: records(child.stdout), wallNanoseconds, evidence: prefix };
}

function projectCommand(lane, project, args, label) {
  return command(lane, ['--project', project, ...args], label);
}

function accept(lane, project, label, source) {
  const status = projectCommand(lane, project, ['status'], `${label}-status`);
  const base = record(status.records, 'revision').id;
  const input = path.join(lane.directory, 'requests', `${label}.lkjc`);
  fs.writeFileSync(input, `request base=${base}\n${source}`, { flag: 'wx' });
  const planned = projectCommand(lane, project, ['change', 'plan', '--input-file', input], `${label}-plan`);
  const plan = record(planned.records, 'plan').token;
  projectCommand(lane, project, ['change', 'apply', '--input-file', input, '--plan', plan], `${label}-apply`);
}

function check(lane, project, label) {
  const checked = projectCommand(lane, project, ['check'], label);
  const tests = record(checked.records, 'tests');
  assert.equal(tests.failed, '0');
  assert.equal(tests.differential, 'equal');
}

function preflight(root, name, executable, expectedRevision) {
  const lane = { name, directory: path.join(root, 'preflight', name), serial: 0 };
  fs.mkdirSync(lane.directory, { recursive: true });
  fs.mkdirSync(path.join(lane.directory, 'commands'));
  lane.executable = path.join(lane.directory, 'lkjscript');
  fs.copyFileSync(executable, lane.executable, fs.constants.COPYFILE_EXCL);
  fs.chmodSync(lane.executable, 0o755);
  const inspected = command(lane, ['package', 'builtin', 'inspect'], 'builtin-inspect');
  const standard = record(inspected.records, 'package');
  json(path.join(lane.directory, 'selected.json'), { executable_sha256: sha256(lane.executable),
    expected_standard_revision: expectedRevision ?? null, standard });
  if (expectedRevision) assert.equal(standard.revision, expectedRevision,
    `${name}: embedded standard revision differs from the selected accepted supplier; use the refreshed executable`);
  return { executable: lane.executable, standard, expectedRevision: expectedRevision ?? null };
}

function prepareLane(output, name, executable, inputs) {
  const lane = { name, directory: path.join(output, name), serial: 0 };
  fs.mkdirSync(lane.directory);
  for (const directory of ['commands', 'requests', 'inputs']) fs.mkdirSync(path.join(lane.directory, directory));
  lane.executable = path.join(lane.directory, 'lkjscript');
  fs.copyFileSync(executable, lane.executable, fs.constants.COPYFILE_EXCL);
  fs.chmodSync(lane.executable, 0o755);
  const representation = name.endsWith('-nominal') ? 'nominal' : 'structural';
  const supplier = path.join(lane.directory, 'supplier-project');
  const consumer = path.join(lane.directory, 'consumer-project');
  command(lane, ['new', supplier, '--template', 'command', '--name', 'fifo-cost-supplier'], 'supplier-new');
  const standard = path.join(lane.directory, 'standard.lkjp');
  const standardExport = command(lane, ['package', 'builtin', 'export', '--kind', 'transport', '--output', standard], 'standard-export');
  const standardIdentity = record(standardExport.records, 'package');
  accept(lane, supplier, 'representation', fs.readFileSync(path.join(here, `${representation}.lkjc`), 'utf8'));
  const source = name => fs.readFileSync(path.join(inputs, representation, `${name}.lkjc`), 'utf8')
    .replaceAll('std::fifo-', 'fifo-cost::fifo-').replaceAll('std::Fifo', 'fifo-cost::Fifo');
  for (const label of ['library', 'steps', 'trace']) accept(lane, supplier, label, source(label));
  check(lane, supplier, 'supplier-check');
  for (const module of ['fifo-cost', 'fifo-support', 'fifo-steps', 'fifo-trace']) {
    const draft = path.join(lane.directory, 'requests', `${module}-draft.lkjc`);
    projectCommand(lane, supplier, ['change', 'draft', '--module', module, '--output', draft], `${module}-draft`);
    const planned = projectCommand(lane, supplier, ['change', 'plan', '--input-file', draft], `${module}-draft-plan`);
    assert.equal(record(planned.records, 'result').outcome, 'unchanged');
  }
  const transport = path.join(lane.directory, 'supplier.lkjp');
  const exported = projectCommand(lane, supplier,
    ['package', 'current', 'export', '--kind', 'transport', '--output', transport], 'supplier-export');
  const identity = record(exported.records, 'package');
  command(lane, ['new', consumer, '--template', 'command', '--name', 'fifo-cost-consumer'], 'consumer-new');
  projectCommand(lane, consumer, ['package', 'dependency', 'stage', '--transport', identity.transport, '--input-file', transport], 'supplier-stage');
  const imports = `declarations.begin\n(units ${['fifo-cost', 'fifo-support', 'fifo-trace'].map(module =>
    `(use ${module} ${identity.id} ${identity['package-revision']})`).join(' ')})\ndeclarations.end\n`;
  const dependency = `add.dependency package=${identity.id} semantic-revision=${identity.revision} package-revision=${identity['package-revision']}\n`;
  accept(lane, consumer, 'elements', dependency + imports + source('elements'));
  accept(lane, consumer, 'application', imports + source('application'));
  check(lane, consumer, 'consumer-check');
  lane.artifact = path.join(lane.directory, 'fifo.lkja');
  projectCommand(lane, consumer, ['build', '--output', lane.artifact], 'consumer-build');
  lane.descriptors = {};
  for (const target of ['fifo-cells', 'fifo-buffers', 'fifo-packets']) {
    const descriptor = path.join(lane.directory, `${target}.deployment.json`);
    json(descriptor, { artifact: 'fifo.lkja', target, listen: null, http: null, session: null, worker: null,
      streams: { maximum_chunk_bytes: 65536, maximum_buffered_chunks: 8, maximum_total_bytes: 1048576, maximum_live_streams: 1024 },
      grants: [], secrets: [], configuration: {} });
    lane.descriptors[target] = descriptor;
  }
  lane.identity = { executable_sha256: sha256(lane.executable), artifact_sha256: sha256(lane.artifact),
    standard: standardIdentity, supplier: identity, standard_transport_sha256: sha256(standard), supplier_transport_sha256: sha256(transport) };
  // Only resources created under this new evidence root are removed.
  for (const project of [supplier, consumer]) fs.rmSync(project, { recursive: true });
  for (const file of [standard, transport]) fs.unlinkSync(file);
  lane.detached = { projects_absent: !fs.existsSync(supplier) && !fs.existsSync(consumer),
    transports_absent: !fs.existsSync(standard) && !fs.existsSync(transport) };
  json(path.join(lane.directory, 'identity.json'), lane.identity);
  return lane;
}

function joined(execution) {
  const observation = JSON.parse(execution['production-observation']);
  for (const field of ['live_call_frames_after', 'live_handles_after', 'live_locals_after',
    'live_operands_after', 'live_transactions_after', 'live_type_bindings_after']) assert.equal(observation[field], 0, field);
  assert.equal(observation.capability_calls, 0);
  const cleanup = JSON.parse(execution.cleanup);
  assert.equal(cleanup.admission_stopped, true);
  assert.equal(cleanup.remaining_tasks, 0);
  assert.deepEqual(cleanup.cleanup_failures, []);
  const executor = JSON.parse(execution['executor-observation']);
  assert.equal(executor.dispatch_open, false);
  assert.equal(executor.active_dispatches, 0);
  assert.equal(executor.remaining_workers, 0);
  return observation;
}

function invoke(lane, workload, sample, root) {
  const label = `${workload.name}-${sample}`;
  const input = path.join(lane.directory, 'inputs', `${label}.json`);
  const result = path.join(lane.directory, 'inputs', `${label}-result.json`);
  json(input, workload.arguments);
  const run = command(lane, ['run', '--deployment', lane.descriptors[workload.target],
    '--arguments-file', input, '--result-file', result], label);
  assert.deepEqual(JSON.parse(fs.readFileSync(result, 'utf8')), workload.expected, `${lane.name}/${label}`);
  const execution = record(run.records, 'execution');
  assert.equal(execution['execution-mode'], 'production');
  assert.equal(execution.verification, 'not-performed');
  const observation = joined(execution);
  const evidence = { lane: lane.name, workload: workload.name, sample, commands: path.relative(root, run.evidence),
    wall_nanoseconds: run.wallNanoseconds, preparation_nanoseconds: Number(execution['preparation-nanoseconds']),
    invocation_nanoseconds: Number(execution['invocation-nanoseconds']),
    result_encoding_nanoseconds: Number(execution['result-encoding-nanoseconds']), observation };
  for (const field of ['wall_nanoseconds', 'preparation_nanoseconds', 'invocation_nanoseconds', 'result_encoding_nanoseconds'])
    assert(Number.isSafeInteger(evidence[field]) && evidence[field] >= 0, `invalid ${field}`);
  return evidence;
}

const median = values => {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
};

function measure(root, state, samples) {
  const directory = path.join(root, `measurement-${Date.now()}`);
  fs.mkdirSync(directory);
  const dataset = workloads();
  for (const lane of state.lanes) lane.serial = fs.readdirSync(path.join(lane.directory, 'commands')).filter(name => name.endsWith('.json')).length;
  const observations = [];
  for (let index = 0; index < dataset.length; index++) {
    const workload = dataset[index];
    for (let sample = 0; sample <= samples; sample++) {
      const order = orders[(index + sample) % orders.length];
      for (const selected of order) {
        const observation = invoke(state.lanes[selected], workload, `${path.basename(directory)}-${sample === 0 ? 'warmup' : sample}`, root);
        observation.warmup = sample === 0;
        observations.push(observation);
        fs.appendFileSync(path.join(directory, 'samples.jsonl'), `${JSON.stringify(observation)}\n`);
      }
    }
    console.log(`checked ${workload.name}: 3 warmups and ${samples * 3} measured executions`);
  }
  const timings = ['wall_nanoseconds', 'preparation_nanoseconds', 'invocation_nanoseconds', 'result_encoding_nanoseconds'];
  const summary = dataset.map(workload => {
    const byLane = Object.fromEntries(state.lanes.map(lane => {
      const rows = observations.filter(row => !row.warmup && row.workload === workload.name && row.lane === lane.name);
      return [lane.name, { medians: Object.fromEntries(timings.map(field => [field, median(rows.map(row => row[field]))])),
        observations: rows.map(row => row.observation) }];
    }));
    const ratio = (left, right) => Object.fromEntries(timings.map(field =>
      [field, byLane[left].medians[field] / byLane[right].medians[field]]));
    return { workload: workload.name, lanes: byLane,
      candidate_nominal_over_structural: ratio('candidate-nominal', 'candidate-structural'),
      candidate_structural_over_baseline: ratio('candidate-structural', 'baseline-structural') };
  });
  for (const lane of state.lanes) {
    assert.equal(sha256(lane.executable), lane.identity.executable_sha256);
    assert.equal(sha256(lane.artifact), lane.identity.artifact_sha256);
  }
  json(path.join(directory, 'summary.json'), { workloads: dataset.length, samples_per_lane: samples,
    total_executions: observations.length, complete_results_equal: true, joined_cleanup: true, hashes_unchanged: true,
    interpretation: 'Modeled allocation is not allocator bytes or RSS; successful cleanup is not a count of custody envelopes.', summary });
  return directory;
}

function options(argv) {
  const result = {};
  for (let index = 0; index < argv.length; index++) {
    const key = argv[index];
    assert(key.startsWith('--'), `unexpected argument ${key}`);
    if (['--self-check', '--prepare-only'].includes(key)) result[key] = true;
    else {
      assert(index + 1 < argv.length, `missing value for ${key}`);
      assert(!result[key], `duplicate option ${key}`);
      result[key] = argv[++index];
    }
  }
  for (const key of Object.keys(result)) assert(['--self-check', '--prepare-only', '--measure-only',
    '--baseline-executable', '--candidate-executable', '--inputs', '--output', '--samples',
    '--expected-baseline-standard-revision', '--expected-candidate-standard-revision'].includes(key), `unknown option ${key}`);
  return result;
}

const selected = options(process.argv.slice(2));
const inputs = selected['--inputs'] && path.resolve(selected['--inputs']);
selfCheck(inputs);
if (selected['--self-check']) console.log('Literal algorithm/consumer equality, oracle results and compact decoding checked; no executable invoked.');
else {
  const samples = Number(selected['--samples'] ?? 7);
  assert(Number.isInteger(samples) && samples > 0);
  if (selected['--measure-only']) {
    const root = path.resolve(selected['--measure-only']);
    const state = JSON.parse(fs.readFileSync(path.join(root, 'prepared.json'), 'utf8'));
    console.log(measure(root, state, samples));
  } else {
    assert(inputs && selected['--baseline-executable'] && selected['--candidate-executable'] && selected['--output'],
      'require --inputs, --baseline-executable, --candidate-executable and an absent --output directory');
    const root = path.resolve(selected['--output']);
    assert(!fs.existsSync(root), 'preserve existing runs; choose an absent evidence directory');
    fs.mkdirSync(root, { recursive: true });
    const baseline = path.resolve(selected['--baseline-executable']);
    const candidate = path.resolve(selected['--candidate-executable']);
    json(path.join(root, 'environment.json'), { utc: new Date().toISOString(), platform: process.platform,
      arch: process.arch, node: process.version, release: os.release(), cpus: os.cpus().map(cpu => cpu.model),
      available_parallelism: os.availableParallelism(), total_memory_bytes: os.totalmem(),
      runner_sha256: sha256(fileURLToPath(import.meta.url)),
      representation_sha256: Object.fromEntries(['structural', 'nominal'].map(kind =>
        [kind, sha256(path.join(here, `${kind}.lkjc`))])),
      consumer_inputs_sha256: Object.fromEntries(['structural', 'nominal'].map(kind => [kind,
        Object.fromEntries(names.map(name => [name, sha256(path.join(inputs, kind, `${name}.lkjc`))]))])),
      expected_baseline_standard_revision: selected['--expected-baseline-standard-revision'] ?? null,
      expected_candidate_standard_revision: selected['--expected-candidate-standard-revision'] ?? null,
      parent_program: process.execPath, input_origin: fs.existsSync(path.join(inputs, 'origin.json'))
        ? JSON.parse(fs.readFileSync(path.join(inputs, 'origin.json'), 'utf8')) : null });
    // Inspect both immutable executable copies before creating any graph or artifact.
    const preflights = {
      baseline: preflight(root, 'baseline', baseline, selected['--expected-baseline-standard-revision']),
      candidate: preflight(root, 'candidate', candidate, selected['--expected-candidate-standard-revision']),
    };
    const state = { preflights, lanes: lanes.map(name => prepareLane(root, name,
      name.startsWith('baseline-') ? preflights.baseline.executable : preflights.candidate.executable, inputs)) };
    assert.equal(state.lanes[1].identity.executable_sha256, state.lanes[2].identity.executable_sha256);
    json(path.join(root, 'prepared.json'), state);
    json(path.join(root, 'workloads.json'), workloads());
    console.log(`prepared detached FIFO comparison: ${root}`);
    if (!selected['--prepare-only']) console.log(measure(root, state, samples));
  }
}
