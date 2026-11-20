#!/usr/bin/env node
// Reconstruct authenticated original source for the native/interpreter and Rust witnesses.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';

try {
    const [source, output, mode, ...extra] = process.argv.slice(2);
    if (!source || !output || extra.length || (mode && mode !== '--rust')) {
        throw new Error('usage: restore-levenshtein.mjs SOURCE OUTPUT [--rust]');
    }
    const manifest = JSON.parse(fs.readFileSync(path.join(source, 'upstream.json'), 'utf8'));
    // Latin-1 is a reversible byte mapping; only ASCII syntax is replaced.
    const projection = fs.readFileSync(path.join(source, 'upstream.lkj')).toString('latin1');
    const marker = '\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.\n';
    const parts = projection.split(marker);
    if (parts.length !== 2) throw new Error('invalid integration boundary');
    const [normalized, tests] = parts;
    const originalRange = 'std::iter::range_from(1)';
    const ranges = normalized.split(originalRange);
    if (ranges.length !== 2) throw new Error('invalid inverse range count');
    const library = ranges.join('(1..)');
    const originalTests = 'extern crate levenshtein;\nuse levenshtein::levenshtein;\n' + tests;
    const hash = text => createHash('sha256').update(Buffer.from(text, 'latin1')).digest('hex');
    if (hash(library) !== manifest.files['src/lib.rs']) throw new Error('original library bytes differ');
    if (hash(originalTests) !== manifest.files['tests/lib.rs']) throw new Error('original test bytes differ');
    const lines = library.split('\n').length - 1;
    if (lines !== 65 || lines !== manifest.source_lines) throw new Error('original library line count differs');
    // Validate both complete inputs before publishing any projection.
    if (mode === '--rust') {
        fs.writeFileSync(path.join(output, 'upstream.rs'), Buffer.from(library, 'latin1'));
        fs.writeFileSync(path.join(output, 'upstream_tests.rs'), Buffer.from(originalTests, 'latin1'));
    } else {
        fs.writeFileSync(path.join(output, 'upstream.lkj'), Buffer.from(library + marker + tests, 'latin1'));
    }
} catch (error) {
    process.stderr.write('restore-levenshtein: ' + error.message + '\n');
    process.exitCode = 1;
}
