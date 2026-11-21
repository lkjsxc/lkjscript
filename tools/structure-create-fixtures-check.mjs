#!/usr/bin/env node
// Foreign byte oracle; it does not invoke the fixture producer or compiler.
import {readFileSync, readdirSync} from 'node:fs';
import {join} from 'node:path';

function canonical(value) {
    if (Array.isArray(value)) return value.map(canonical);
    if (value !== null && typeof value === 'object') {
        return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]));
    }
    return value;
}
function verify(root) {
    const files = new Map();
    const operations = [];
    const modules = [];
    for (let index = 0; index < 16; ++index) {
        modules.push(`mod old${index};\n`);
        files.set(`old/old${index}.lkj`, `// ${'x'.repeat(16384)}\npub fn value() -> i64 { ${index}i64 }\n`);
        operations.push({op:'replace-body', id:`old${index}::value`, source:`{ ${1000 + index}i64 }`});
        operations.push({op:'add-item', id:'crate', source:`mod fresh${index};`, projection:{
            path:`fresh${index}.lkj`, source:`// ${'y'.repeat(16384)}\npub fn value() -> i64 { ${2000 + index}i64 }\n`,
        }});
    }
    files.set('old/main.lkj', modules.join('') + 'pub fn main() -> i64 { old0::value() }\n');
    operations.push({op:'replace-body', id:'main', source:'{ old0::value() + fresh0::value() }'});
    files.set('request.json', JSON.stringify(canonical({version:1, operations})) + '\n');
    for (const [name, expected] of files) {
        if (!readFileSync(join(root, name)).equals(Buffer.from(expected))) throw new Error(`${name} differs`);
    }
    const actualNames = readdirSync(join(root, 'old')).sort();
    const expectedNames = [...files.keys()].filter(name => name.startsWith('old/')).map(name => name.slice(4)).sort();
    if (JSON.stringify(actualNames) !== JSON.stringify(expectedNames)) throw new Error('source inventory differs');
}
try {
    if (process.argv.length !== 3) throw new Error('expected fixture root');
    verify(process.argv[2]);
} catch (error) {
    process.stderr.write(`create-kill fixture: ${error.message}\n`);
    process.exitCode = 1;
}
