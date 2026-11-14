const fs = require('fs'), crypto = require('crypto');
const source = process.argv[2];
if (!/^[0-9a-f]{40}$/.test(source || '')) throw Error('exact candidate source required');
const root = '.artifacts/lkjscript-dev/check/transient/workspace-tests';
const output = '.artifacts/selfhost/source-official-harnesses-v2.json';
const stop = '.artifacts/selfhost/source-full-finished.json';
const started = Date.now(), observed = new Map();
function scan() {
  try {
    if (fs.existsSync(root)) for (const name of fs.readdirSync(root).filter(n => n.endsWith('-run.stdout'))) {
      const bytes = fs.readFileSync(root + '/' + name), text = bytes.toString();
      const results = (text.match(/^test result: .*/gm) || []).filter(line => /; [01] filtered out;/.test(line));
      if (!results.length) continue;
      const lines = text.split('\n');
      const record = {file: name, terminal_result: results.at(-1), bytes: bytes.length,
        sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
        selfhost: lines.filter(l => /^test .*::selfhost_tests::.* \.\.\. (ok|FAILED)$/.test(l)),
        native: lines.filter(l => /^test .*::(native_elf_tests|native_filter_tests|native_derive_tests)::.* \.\.\. (ok|FAILED)$/.test(l) || /^test .*::stack_tests::native_elf_.* \.\.\. (ok|FAILED)$/.test(l)),
        ignored: lines.filter(l => /^test .* \.\.\. ignored/.test(l))};
      if (observed.get(name)?.sha256 === record.sha256) continue;
      observed.set(name, record);
      fs.writeFileSync(output, JSON.stringify({source, scope:'Observed terminal harness output; official full receipt owns overall success and producer identities', harnesses:[...observed.values()]}, null, 2) + '\n');
      process.stdout.write(JSON.stringify({file:name,result:record.terminal_result,selfhost:record.selfhost.length,native:record.native.length}) + '\n');
    }
    if (fs.existsSync(stop)) {
      clearInterval(timer);
      if (!observed.size) { process.exitCode = 1; process.stderr.write('No harness summaries captured\n'); }
    } else if (Date.now() - started > 7200000) {
      clearInterval(timer); process.exitCode = 1; process.stderr.write('Official receipt wait expired\n');
    }
  } catch (error) {
    if (error.code !== 'ENOENT') { clearInterval(timer); process.exitCode = 1; process.stderr.write(String(error) + '\n'); }
  }
}
const timer = setInterval(scan, 1000);
scan();
