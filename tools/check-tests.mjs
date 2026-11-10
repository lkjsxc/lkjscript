// Execute exactly the Cargo-selected workspace harnesses and summarize timings.
import { readFileSync, readdirSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { performance } from 'node:perf_hooks';

// These are observations of /proc, not enforced limits or exact resource accounting.
// HWM preserves earlier process peaks, but short-lived processes can exit between samples.
function observeRss(rootPid, onSample) {
  let harnessPeak;
  let processPeak;
  let treePeak;
  let samples = 0;
  const read = path => { try { return readFileSync(path, 'utf8'); } catch { return ''; } };
  const tasks = pid => { try { return readdirSync('/proc/' + pid + '/task'); } catch { return []; } };
  const sample = () => {
    if (!rootPid) return;
    const pending = [rootPid];
    const seen = new Set();
    let total = 0;
    let observed = false;
    while (pending.length) {
      const pid = pending.pop();
      if (seen.has(pid)) continue;
      seen.add(pid);
      const status = read('/proc/' + pid + '/status');
      const hwm = /^VmHWM:\s+(\d+)\s+kB$/m.exec(status);
      const rss = /^VmRSS:\s+(\d+)\s+kB$/m.exec(status);
      if (hwm) {
        const value = Number(hwm[1]);
        processPeak = Math.max(processPeak ?? 0, value);
        if (pid === rootPid) harnessPeak = Math.max(harnessPeak ?? 0, value);
        observed = true;
      }
      if (rss) total += Number(rss[1]);
      // Children belong to the spawning thread; walking every task includes test workers.
      for (const tid of tasks(pid)) {
        for (const child of read('/proc/' + pid + '/task/' + tid + '/children').trim().split(/\s+/)) {
          if (child) pending.push(Number(child));
        }
      }
    }
    if (observed) {
      samples += 1;
      treePeak = Math.max(treePeak ?? 0, total);
      onSample({ processPeak, currentTreeRss: total });
    }
  };
  sample();
  const timer = setInterval(sample, 500);
  timer.unref();
  return () => {
    clearInterval(timer);
    sample();
    return { harnessPeak, processPeak, treePeak, samples };
  };
}

const [mode, inventory, threads, ...filters] = process.argv.slice(2);
const artifacts = readFileSync(inventory, 'utf8').split('\n').filter(Boolean).map(JSON.parse);
for (const row of artifacts) {
  if (row.reason === 'compiler-message' && row.message?.rendered) {
    process.stderr.write(row.message.rendered);
  }
}
if (mode === 'diagnostics') process.exit(1);
if (!['routine', 'focused', 'slow'].includes(mode)
    || (mode === 'routine' ? filters.length !== 0
      : !filters[0] || filters[0].startsWith('-'))) {
  console.error('routine runs the complete suite; focused and slow require a test-name filter before harness options');
  process.exit(2);
}
const harnesses = [...new Set(artifacts.filter(row => row.reason === 'compiler-artifact'
  && row.profile?.test && row.executable).map(row => row.executable))];
if (!harnesses.length) throw new Error('Cargo returned no test executables');
const inheritedStack = process.env.RUST_MIN_STACK;
const physicalKiB = /^MemTotal:\s+(\d+)\s+kB$/m.exec(readFileSync('/proc/meminfo', 'utf8'));
if (!inheritedStack && !physicalKiB) throw new Error('cannot determine physical memory for test stacks');
const stack = inheritedStack || String(BigInt(physicalKiB[1]) * 2048n);
const env = { ...process.env, RUST_MIN_STACK: stack, RUSTC_BOOTSTRAP: '1' };
console.log(`test_configuration mode=${mode} threads=${threads} stack_reservation_bytes=${stack}`
  + ` filters=${JSON.stringify(filters)} rss_sampling_ms=500`);
const timings = [];
const totals = { passed: 0, failed: 0, ignored: 0, filtered_out: 0 };
let failed = false;
let sampledPeakRss;
let sampledPeakTreeRss;
const started = performance.now();
for (const harness of harnesses) {
  const args = ['-Z', 'unstable-options', '--format=json', '--report-time',
    `--test-threads=${threads}`, ...filters];
  if (mode === 'slow') args.push('--ignored');
  const harnessStarted = performance.now();
  const child = spawn(harness, args, { env, stdio: ['inherit', 'pipe', 'inherit'] });
  const active = new Map();
  let completed = 0;
  let reported = harnessStarted;
  const finishRss = observeRss(child.pid, rss => {
    const now = performance.now();
    if (now - reported < 30000) return;
    reported = now;
    const running = [...active].map(([name, since]) => ({
      name, seconds: Number(((now - since) / 1000).toFixed(1))
    }));
    console.log(`progress harness=${harness.split('/').at(-1)} completed=${completed}`
      + ` tree_rss_kib=${rss.currentTreeRss} sampled_peak_process_rss_kib=${rss.processPeak}`
      + ` active=${JSON.stringify(running)}`);
  });
  let summary;
  let invalidOutput = false;
  const lines = createInterface({ input: child.stdout });
  lines.on('line', line => {
    let event;
    try { event = JSON.parse(line); } catch {
      // A test subprocess can write directly to inherited stdout. Keep its output.
      console.log(line);
      return;
    }
    if (event.type === 'test' && event.event === 'started') active.set(event.name, performance.now());
    if (event.type === 'test' && ['ok', 'failed', 'ignored'].includes(event.event)) {
      active.delete(event.name);
      completed += 1;
    }
    if (event.type === 'test' && typeof event.exec_time === 'number') {
      timings.push({ name: event.name, seconds: event.exec_time, outcome: event.event });
    }
    if (event.type === 'test' && event.event === 'failed') {
      console.error(`FAILED ${event.name}\n${event.stdout ?? ''}`);
    }
    if (mode === 'slow' && event.type === 'test' && event.event === 'ok') {
      console.log(`PASSED ${event.name} seconds=${event.exec_time ?? 'unavailable'}`);
    }
    if (event.type === 'suite' && ['ok', 'failed'].includes(event.event)) summary = event;
  });
  child.on('error', error => { invalidOutput = true; console.error(error.message); });
  const { code, signal } = await new Promise(resolve => child.on('close',
    (code, signal) => resolve({ code, signal })));
  const rss = finishRss();
  if (rss.processPeak !== undefined) sampledPeakRss = Math.max(sampledPeakRss ?? 0, rss.processPeak);
  if (rss.treePeak !== undefined) sampledPeakTreeRss = Math.max(sampledPeakTreeRss ?? 0, rss.treePeak);
  console.log(`${harness.split('/').at(-1)}: wall_seconds=${((performance.now() - harnessStarted) / 1000).toFixed(2)}`
    + ` sampled_peak_harness_rss_kib=${rss.harnessPeak ?? 'unavailable'}`
    + ` sampled_peak_process_rss_kib=${rss.processPeak ?? 'unavailable'}`
    + ` sampled_peak_tree_rss_kib=${rss.treePeak ?? 'unavailable'} rss_samples=${rss.samples}`);
  if (!summary || invalidOutput || code !== 0 || signal) {
    failed = true;
    console.error(`harness failed: ${harness} exit=${code} signal=${signal ?? 'none'} active=${JSON.stringify([...active.keys()])}`);
  }
  if (summary) {
    for (const key of Object.keys(totals)) totals[key] += summary[key] ?? 0;
    console.log(`${harness.split('/').at(-1)}: passed=${summary.passed} failed=${summary.failed}`
      + ` ignored=${summary.ignored} filtered=${summary.filtered_out} seconds=${summary.exec_time}`);
  }
}
console.log(`tests mode=${mode} elapsed_seconds=${((performance.now() - started) / 1000).toFixed(2)}`
  + ` passed=${totals.passed} failed=${totals.failed} ignored=${totals.ignored} filtered=${totals.filtered_out}`
  + ` sampled_peak_process_rss_kib=${sampledPeakRss ?? 'unavailable'}`
  + ` sampled_peak_tree_rss_kib=${sampledPeakTreeRss ?? 'unavailable'}`);
if (totals.passed + totals.failed === 0) {
  failed = true;
  console.error('No selected tests ran; check the mode and test-name filters.');
}
console.log('Ten slowest executed tests (seconds):');
for (const row of timings.sort((a, b) => b.seconds - a.seconds).slice(0, 10)) {
  console.log(`${row.seconds.toFixed(3)} ${row.outcome} ${row.name}`);
}
process.exitCode = failed ? 1 : 0;
