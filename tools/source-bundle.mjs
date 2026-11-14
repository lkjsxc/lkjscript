#!/usr/bin/env node
// Pack only caller-declared files. The production source loader chooses modules.
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

const magic = Buffer.from('LKJSB1\n', 'ascii');
function hash(bytes) {
    return createHash('sha256').update(bytes).digest('hex');
}
function textBytes(value, label) {
    if (typeof value !== 'string') throw new Error(label + ' must be a string');
    const bytes = Buffer.from(value, 'utf8');
    if (bytes.toString('utf8') !== value) throw new Error(label + ' contains an unpaired surrogate');
    return bytes;
}
function length(value) {
    const bytes = Buffer.alloc(8);
    bytes.writeBigUInt64LE(BigInt(value));
    return bytes;
}
function appendText(chunks, value, label) {
    const bytes = textBytes(value, label);
    chunks.push(length(bytes.length), bytes);
}

/** Encode an explicit in-memory SourceBundle, preserving file order and bytes. */
export function encodeBundle(bundle) {
    if (!bundle || !Array.isArray(bundle.files)) throw new Error('files must be an array');
    const chunks = [magic];
    appendText(chunks, bundle.root, 'root');
    const library = bundle.library === undefined ? null : bundle.library;
    if (library === null) {
        chunks.push(Buffer.from([0]));
    } else {
        chunks.push(Buffer.from([1]));
        appendText(chunks, library, 'library');
    }
    chunks.push(length(bundle.files.length));
    for (const file of bundle.files) {
        appendText(chunks, file.path, 'logical path');
        appendText(chunks, file.text, 'source text');
    }
    return Buffer.concat(chunks);
}

/** Read exact declared files; neither discover modules nor add a library. */
export async function packFiles(specification) {
    if (!specification || !Array.isArray(specification.files)) {
        throw new Error('input specification needs a files array');
    }
    const files = [];
    const records = [];
    for (const entry of specification.files) {
        const input = typeof entry === 'string' ? {path: entry, file: entry} : entry;
        if (!input || typeof input.file !== 'string') throw new Error('each input needs a physical file');
        textBytes(input.path, 'logical path');
        const bytes = await fs.readFile(input.file);
        const text = bytes.toString('utf8');
        if (!Buffer.from(text, 'utf8').equals(bytes)) {
            throw new Error(input.file + ': source is not valid UTF-8');
        }
        files.push({path: input.path, text});
        records.push({
            path: input.path,
            file: input.file,
            bytes: bytes.length,
            sha256: hash(bytes),
        });
    }
    const library = specification.library === undefined ? null : specification.library;
    const bytes = encodeBundle({root: specification.root, library, files});
    return {
        bytes,
        manifest: {
            format: 'LKJSB1',
            root: specification.root,
            library,
            base: process.cwd(),
            files: records,
            bundle: {bytes: bytes.length, sha256: hash(bytes)},
        },
    };
}

const usage = [
    'usage: node tools/source-bundle.mjs --root ROOT [--library LIBRARY_ROOT]',
    '       --file FILE [--file FILE ...] --output BUNDLE [--manifest RECORD.json]',
    '   or: node tools/source-bundle.mjs --inputs SPEC.json --output BUNDLE',
    'SPEC.json: {root, library: null|string, files: [{path: LOGICAL, file: PHYSICAL}]}',
    'Without --inputs, every --file uses the same logical and physical path.',
    'The default record is BUNDLE.json; output and record must be new files.',
    'No module discovery or implicit library.',
].join('\n');

async function main(args) {
    let root;
    let library;
    let inputs;
    let output;
    let manifestPath;
    const files = [];
    for (let index = 0; index < args.length; index++) {
        const option = args[index];
        if (option === '--help') {
            process.stdout.write(usage + '\n');
            return;
        }
        if (!['--root', '--library', '--inputs', '--output', '--manifest', '--file'].includes(option)) {
            throw new Error('unknown option ' + option + '\n' + usage);
        }
        index += 1;
        if (index === args.length) throw new Error(option + ' needs a value');
        const value = args[index];
        if (option === '--root') root = value;
        else if (option === '--library') library = value;
        else if (option === '--inputs') inputs = value;
        else if (option === '--output') output = value;
        else if (option === '--manifest') manifestPath = value;
        else files.push(value);
    }
    if (!output) throw new Error('--output is required\n' + usage);
    let specification;
    if (inputs !== undefined) {
        if (root !== undefined || library !== undefined || files.length) {
            throw new Error('--inputs cannot be combined with --root, --library or --file');
        }
        specification = JSON.parse(await fs.readFile(inputs, 'utf8'));
    } else {
        if (root === undefined) throw new Error('--root or --inputs is required\n' + usage);
        specification = {root, library: library === undefined ? null : library, files};
    }
    manifestPath = manifestPath === undefined ? output + '.json' : manifestPath;
    const packed = await packFiles(specification);
    const protectedPaths = packed.manifest.files.map(entry => path.resolve(entry.file));
    if (inputs !== undefined) protectedPaths.push(path.resolve(inputs));
    if (path.resolve(output) === path.resolve(manifestPath)
        || protectedPaths.includes(path.resolve(output))
        || protectedPaths.includes(path.resolve(manifestPath))) {
        throw new Error('bundle and record must be distinct from every input');
    }
    packed.manifest.bundle.file = output;
    await fs.mkdir(path.dirname(path.resolve(output)), {recursive: true});
    await fs.mkdir(path.dirname(path.resolve(manifestPath)), {recursive: true});
    // Reserve both entries before writing either. Exclusive creation also rejects
    // symlinks and hardlinks to inputs that lexical path checks cannot identify.
    const created = [];
    try {
        const bundle = await fs.open(output, 'wx');
        created.push({path: output, handle: bundle});
        const manifest = await fs.open(manifestPath, 'wx');
        created.push({path: manifestPath, handle: manifest});
        await bundle.writeFile(packed.bytes);
        await manifest.writeFile(JSON.stringify(packed.manifest, null, 2) + '\n');
        for (const file of created) await file.handle.close();
    } catch (error) {
        // Only these successful exclusive creates belong to this invocation.
        for (const file of created) await file.handle.close().catch(() => {});
        for (const file of created) await fs.unlink(file.path).catch(() => {});
        throw error;
    }
    process.stdout.write(output + ': ' + packed.bytes.length + ' bytes; '
        + packed.manifest.files.length + ' declared files; sha256 '
        + packed.manifest.bundle.sha256 + '\n');
}

// Node resolves the module URL through symlinks, while argv keeps its entry spelling.
const entryPath = process.argv[1] ? await fs.realpath(process.argv[1]).catch(() => null) : null;
if (entryPath !== null && entryPath === await fs.realpath(fileURLToPath(import.meta.url))) {
    main(process.argv.slice(2)).catch(error => {
        process.stderr.write(String(error.message || error) + '\n');
        process.exitCode = 1;
    });
}
