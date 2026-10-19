'use strict';
// Restore evidence only. This script never authors source, builds, or runs a candidate.
const fs = require('fs');
const path = require('path');
const cp = require('child_process');
const crypto = require('crypto');
const zlib = require('zlib');
const arguments_ = process.argv.slice(2);
if (arguments_.some(value => value !== '--verify-only') || arguments_.length > 1) {
  throw Error('Usage: node restore-artifacts.cjs [--verify-only]');
}
const verifyOnly = arguments_.includes('--verify-only');
const root = cp.execFileSync('git', ['rev-parse', '--show-toplevel'], {encoding: 'utf8'}).trim();
if (fs.realpathSync(process.cwd()) !== fs.realpathSync(root)) {
  throw Error('Run from the canonical repository root');
}
const manifest = JSON.parse(fs.readFileSync(path.join(__dirname, 'manifest.json'), 'utf8'));
if (manifest.schema !== 'lkjscript.selfhost.handoff.v1' || !Array.isArray(manifest.files)) {
  throw Error('Unknown handoff manifest');
}
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const target = path.join(root, '.artifacts', 'selfhost');
const missing = [];
const seen = new Set();
for (const file of manifest.files) {
  const relative = file.path;
  if (typeof relative !== 'string' || !relative || relative.includes('\\') ||
      path.isAbsolute(relative) || relative.split('/').some(part => !part || part === '.' || part === '..') ||
      seen.has(relative)) throw Error('Unsafe or repeated evidence path');
  seen.add(relative);
  if (file.archive_path !== relative && file.archive_path !== relative + '.gz') throw Error('Unsafe archive path');
  const source = path.join(__dirname, 'artifacts', file.archive_path);
  if (!fs.lstatSync(source).isFile()) throw Error('Evidence is not a regular file: ' + relative);
  const stored = fs.readFileSync(source);
  if (stored.length !== file.stored_bytes || digest(stored) !== file.stored_sha256) throw Error('Stored evidence digest mismatch: ' + relative);
  if (!['identity', 'gzip'].includes(file.encoding)) throw Error('Unknown evidence encoding');
  const bytes = file.encoding === 'gzip' ? zlib.gunzipSync(stored, {maxOutputLength: Math.max(file.bytes, 1)}) : stored;
  if (bytes.length !== file.bytes || digest(bytes) !== file.sha256) {
    throw Error('Evidence digest mismatch: ' + relative);
  }
  const destination = path.join(target, relative);
  if (fs.existsSync(destination)) {
    if (!fs.lstatSync(destination).isFile() || digest(fs.readFileSync(destination)) !== file.sha256) {
      throw Error('Existing local evidence differs; preserve it before restoring: ' + relative);
    }
  } else {
    missing.push({relative, destination, bytes});
  }
}
// Complete preflight before writing any file. Reject symlink parent directories.
for (const item of missing) {
  let directory = path.dirname(item.destination);
  while (directory !== root) {
    if (fs.existsSync(directory) && (!fs.lstatSync(directory).isDirectory() || fs.lstatSync(directory).isSymbolicLink())) {
      throw Error('Unsafe destination directory: ' + directory);
    }
    directory = path.dirname(directory);
  }
}
if (!verifyOnly) {
  for (const item of missing) {
    fs.mkdirSync(path.dirname(item.destination), {recursive: true});
    fs.writeFileSync(item.destination, item.bytes, {flag: 'wx'});
  }
}
process.stdout.write(JSON.stringify({status: 'passed', verified: manifest.files.length,
  restored: verifyOnly ? 0 : missing.length, absent: missing.length, verify_only: verifyOnly}) + '\n');
