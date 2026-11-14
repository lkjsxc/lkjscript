const fs = require('fs');
const path = '.artifacts/lkjscript-dev/check/transient/workspace-tests/1-run.stdout';
const output = '.artifacts/selfhost/source-official-gc-failure.log';
const testcase = 'platform::storage::directory::gc::tests::every_compaction_durability_checkpoint_preserves_reachable_objects_and_retry';
let watcher, timer;
function inspect() {
  try {
    const text = fs.readFileSync(path, 'utf8');
    const at = text.indexOf('---- ' + testcase + ' stdout ----');
    if (at >= 0) {
      const next = text.indexOf('\n---- ', at + 5);
      const evidence = text.slice(at, next < 0 ? undefined : next);
      fs.writeFileSync(output, 'Source: 352f0997142d9fed279f3964a76102d61f7ac1e6\n' + evidence.slice(0, 60000));
      process.stdout.write(evidence.slice(0, 4000));
      watcher?.close(); clearInterval(timer);
    } else if (fs.existsSync('.artifacts/selfhost/source-full-finished.json')) {
      watcher?.close(); clearInterval(timer); process.exitCode = 1;
    }
  } catch(error) {
    if (error.code !== 'ENOENT') throw error;
    if (fs.existsSync('.artifacts/selfhost/source-full-finished.json')) { watcher?.close(); clearInterval(timer); process.exitCode = 1; }
  }
}
watcher = fs.watch(path, inspect);
timer = setInterval(inspect, 1000);
inspect();
