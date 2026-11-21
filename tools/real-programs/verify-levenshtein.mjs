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
function replaceOnce(probe,old,value){if(occurrences(probe.projection,old)!==1)throw new Error('invalid mutation fixture: expected one '+old);probe.projection=probe.projection.replace(old,()=>value);}
export function readSpecimen(directory) {
    return {
        manifest: JSON.parse(new TextDecoder('utf-8', {fatal: true, ignoreBOM: true})
            .decode(readRegular(path.join(directory, 'upstream.json')))),
        projection: readRegular(path.join(directory, 'upstream.lkj')).toString('latin1'),
        license: readRegular(path.join(directory, 'LICENSE')),
    };
}

export function mutationInputs({manifest, projection, license}) {
 const assertion='    assert_eq!(levenshtein("", "a"), 1);\n',mutants=[
 ['missing range normalization',p=>replaceOnce(p,normalized,original)],
 ['extra normalized expression',p=>{p.projection='// '+normalized+'\n'+p.projection;}],
 ['algorithm change',p=>replaceOnce(p,'cache[index_a] = result;','cache[index_a] = result + 1;')],
 ['deleted assertion',p=>replaceOnce(p,assertion,'')],
 ['changed expected value',p=>replaceOnce(p,assertion,'    assert_eq!(levenshtein("", "a"), 0);\n')],
 ['renamed test',p=>replaceOnce(p,'fn empty_left()','fn replaced_left()')],
 ['excluded test',p=>replaceOnce(p,'#[test]\nfn empty_left()','#[cfg(any())]\n#[test]\nfn empty_left()')],
 ['injected test',p=>{p.projection+='\n#[test]\nfn injected() {}\n';}],
 ['changed test literal bytes',p=>replaceOnce(p,'"DwAyNE"','"DWAYNE"')],
 ['changed boundary',p=>replaceOnce(p,marker,'\n// changed boundary\n')],
 ['duplicate boundary',p=>{p.projection+=marker;}],
 ['unexpected integration header',p=>replaceOnce(p,marker,marker+header)],
 ['trailing unaccounted source',p=>{p.projection+='// unrecorded source\n';}],
 ['changed license',p=>{p.license=Buffer.concat([p.license,Buffer.from('\n')]);}],
 ['reordered manifest tests',p=>{p.manifest.test_names.reverse();}],
 ['false source count',p=>{p.manifest.source_lines++;}],
 ['wrong pinned commit',p=>{p.manifest.commit='0'.repeat(40);}],
 ['false adaptation count',p=>{p.manifest.adaptation.range_expression=0;}]];
 const shapes=[['array schema',m=>{m.schema=[m.schema];}],['array pinned commit',m=>{m.commit=[m.commit];}],['array source count',m=>{m.source_lines=[m.source_lines];}],['nested test names',m=>{m.test_names=m.test_names.map(name=>[name]);}]];
 const copy = () => ({manifest: structuredClone(manifest), projection, license: Buffer.from(license)});
 return {
     provenance: mutants.map(([name, mutate]) => {
         const specimen = copy();
         mutate(specimen);
         return {name, specimen};
     }),
     shapes: shapes.map(([name, mutate]) => {
         const specimen = copy();
         mutate(specimen.manifest);
         return {name, specimen};
     }),
 };
}

function controls(inputs) {
    return inputs.map(({name, specimen}) => {
        let reason;
        try {
            verify(specimen.manifest, specimen.projection, specimen.license);
        } catch (error) {
            reason = error.message;
        }
        fact(reason?.startsWith('provenance refusal: '), 'expected a provenance refusal for ' + name);
        return {name, rejected: true, reason};
    });
}

const render = report => JSON.stringify(sorted(report), null, 3) + '\n';

function run(args) {
    const checkReport = args[0] === '--check-report';
    const self = args.includes('--self-test');
    args = args.filter(arg => arg !== '--self-test');
    const checkChecksums = checkReport && args.length === 5 && args[3] === '--checksums';
    if (args.length !== (checkChecksums ? 5 : checkReport ? 3 : 1) ||
        (!checkReport && args[0].startsWith('--'))) {
        throw new Error('usage: node verify-levenshtein.mjs [--self-test | --check-report REPORT] SPECIMEN_DIRECTORY [--checksums LIST]');
    }
    const {manifest, projection, license} = readSpecimen(args[checkReport ? 2 : 0]);
    const report = verify(manifest, projection, license);
    if (checkReport && !readRegular(args[1]).equals(Buffer.from(render(report)))) {
        throw new Error('provenance report bytes differ');
    }
    if (checkChecksums) {
        const expected = ['src/lib.rs', 'tests/lib.rs']
            .map(name => `${report.files[name].sha256}  ${name}\n`).join('');
        if (!readRegular(args[4]).equals(Buffer.from(expected))) {
            throw new Error('checksum list bytes differ');
        }
    }
    if (self || checkReport) {
        const inputs = mutationInputs({manifest, projection, license});
        const provenance = controls(inputs.provenance);
        const shapes = controls(inputs.shapes);
        if (checkReport) {
            if (report.test_count !== 8 || report.assertion_count !== 26 ||
                provenance.length !== 18 || shapes.length !== 4 || !report.exact_upstream_bytes) {
                throw new Error('unexpected upstream inventory');
            }
            return;
        }
        report.mutation_controls = provenance;
        report.mutants_rejected = provenance.length;
        report.temporary_files_created = 0;
        report.manifest_shape_controls = shapes;
        report.malformed_manifests_rejected = shapes.length;
    }
    process.stdout.write(render(report));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(path.resolve(process.argv[1])).href){try{new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(fs.readFileSync('/proc/self/cmdline'));run(process.argv.slice(2));}catch(error){console.error(error.message);process.exitCode=255;}}
