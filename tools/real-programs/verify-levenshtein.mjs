#!/usr/bin/env node
// Independent inverse-projection verifier; host adapter retains O_NOFOLLOW/fstat checks.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const marker='\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.\n';
const header='extern crate levenshtein;\nuse levenshtein::levenshtein;\n';
const normalized='std::iter::range_from(1)',original='(1..)';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const occurrences=(text,needle)=>text.split(needle).length-1;
const fact=(ok,message)=>{if(!ok)throw new Error('provenance refusal: '+message);};
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
const scalar=v=>typeof v==='string'||typeof v==='number';
const eq=(a,b)=>scalar(a)&&String(a)===String(b);
const sorted=v=>Array.isArray(v)?v.map(sorted):object(v)?Object.fromEntries(Object.keys(v).sort().map(k=>[k,sorted(v[k])])):v;
function readRegular(file){let fd;try{fd=fs.openSync(file,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW);if(!fs.fstatSync(fd).isFile())throw new Error('not a regular file: '+file);return fs.readFileSync(fd);}finally{if(fd!==undefined)fs.closeSync(fd);}}
export function verify(manifest,projectionBytes,license){
 const projection=Buffer.isBuffer(projectionBytes)?projectionBytes.toString('latin1'):projectionBytes;
 fact(object(manifest),'manifest must be an object');fact(eq(manifest.schema,1),'unsupported manifest schema');fact(eq(manifest.name,'levenshtein'),'wrong package name');fact(eq(manifest.version,'1.0.5'),'wrong package version');fact(eq(manifest.origin,'https://github.com/wooorm/levenshtein-rs'),'wrong source origin');fact(eq(manifest.commit,'9c4730b1973d4e61e187f8fe6d5f299ad5c991fc'),'wrong pinned commit');fact(eq(manifest.license,'MIT'),'wrong license identity');fact(object(manifest.files),'missing file hashes');fact(object(manifest.adaptation),'missing adaptation contract');fact(eq(manifest.adaptation.range_expression,1),'range adaptation count must be one');fact(eq(manifest.adaptation.integration_header_lines,2),'integration header count must be two');fact(occurrences(projection,marker)===1,'projection must contain exactly one integration boundary');
 let[library,tests]=projection.split(marker);fact(tests!==undefined&&library.length&&tests.length,'empty projection section');fact(occurrences(library,normalized)===1,'library must contain exactly one normalized range expression');fact(occurrences(library,original)===0,'library contains an unexpected original range expression');library=library.replace(normalized,original);
 const reconstructed={'src/lib.rs':Buffer.from(library,'latin1'),'tests/lib.rs':Buffer.from(header+tests,'latin1'),license},verified={};
 for(const name of Object.keys(reconstructed).sort()){const expected=manifest.files[name]??'';fact(typeof expected==='string'&&/^[0-9a-f]{64}$/.test(expected),'invalid hash for '+name);const actual=sha(reconstructed[name]);fact(actual===expected,'upstream bytes differ: '+name);verified[name]={sha256:actual,bytes:reconstructed[name].length,lines:[...reconstructed[name]].filter(b=>b===10).length};}
 fact(scalar(manifest.source_lines)&&verified['src/lib.rs'].lines===Number(manifest.source_lines),'source line count differs');fact(scalar(manifest.test_lines)&&verified['tests/lib.rs'].lines===Number(manifest.test_lines),'test line count differs');
 const names=[...tests.matchAll(/^#\[test\]\nfn ([A-Za-z_][A-Za-z_0-9]*)\(\) \{/gm)].map(m=>m[1]),attributes=[...tests.matchAll(/^#\[test\]$/gm)].length,expected=manifest.test_names;
 fact(Array.isArray(expected)&&expected.length===8&&expected.every(name=>typeof name==='string'),'manifest must name eight tests');fact(names.length===8&&attributes===8,'projection must contain eight tests');fact(names.join('\0')===expected.join('\0'),'ordered test names differ');fact(new Set(names).size===names.length,'duplicate test name');const assertions=[...tests.matchAll(/^[ \t]*assert(?:_eq|_ne)?!\s*\(/gm)].length;fact(assertions>0,'no upstream assertions');
 return{schema:1,kind:'levenshtein-upstream-provenance',commit:manifest.commit,files:verified,projection_sha256:sha(Buffer.from(projection,'latin1')),test_names:names,test_count:names.length,assertion_count:assertions,inverse_range_replacements:1,restored_header_lines:2,exact_upstream_bytes:true};
}
export function readSpecimen(directory) {
    return {
        manifest: JSON.parse(new TextDecoder('utf-8', {fatal: true, ignoreBOM: true})
            .decode(readRegular(path.join(directory, 'upstream.json')))),
        projection: readRegular(path.join(directory, 'upstream.lkj')).toString('latin1'),
        license: readRegular(path.join(directory, 'LICENSE')),
    };
}

const render = report => JSON.stringify(sorted(report), null, 3) + '\n';

function run(args) {
    if (args.length !== 1 || args[0].startsWith('--')) {
        throw new Error('usage: node verify-levenshtein.mjs SPECIMEN_DIRECTORY');
    }
    const {manifest, projection, license} = readSpecimen(args[0]);
    process.stdout.write(render(verify(manifest, projection, license)));
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
    try {
        new TextDecoder('utf-8', {fatal: true, ignoreBOM: true}).decode(fs.readFileSync('/proc/self/cmdline'));
        run(process.argv.slice(2));
    } catch (error) {
        console.error(error.message);
        process.exitCode = 255;
    }
}
