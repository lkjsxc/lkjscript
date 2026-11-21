#!/usr/bin/env bash
# Exact trait-constant source owners, member uses and conservative checked edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/trait-constants.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        for file in "$fixture"/*.log "$fixture"/*.query; do
            [[ ! -f $file ]] || cat "$file" >&2
        done
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
mod left { pub trait Flag { const VALUE: u16; } }
mod right { pub trait Flag { const VALUE: u16; } }
struct Ready;
struct Other;
impl left::Flag for Ready { const VALUE: u16 = 40; }
impl right::Flag for Ready { const VALUE: u16 = 2; }
impl left::Flag for Other { const VALUE: u16 = 999; }
fn idle() -> u16 { <Ready as left::Flag>::VALUE }
fn main() -> i64 {
    (<Ready as left::Flag>::VALUE + <Ready as right::Flag>::VALUE) as i64
}
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
"$compiler" build "${common[@]}" --index -o "$fixture/program" > "$fixture/index.log"
snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index.log")
[[ ${#snapshot} == 64 ]]
query() {
    local kind=$1 identity=$2
    shift 2
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$kind" --id "$identity" --limit 128 --bytes 100000 "$@"
}
query outline crate --depth 8 > "$fixture/outline.query"
query type left::Flag::VALUE > "$fixture/required.query"
query type '<Ready as left::Flag>::VALUE' > "$fixture/provided.query"
query references value@main > "$fixture/references.query"
query uses '<Ready as left::Flag>::VALUE' > "$fixture/left.query"
query uses '<Ready as right::Flag>::VALUE' > "$fixture/right.query"
query uses '<Other as left::Flag>::VALUE' > "$fixture/other.query"
node --input-type=module - "$fixture" <<'CHECK'
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const dir=process.argv[2], read=name=>readFileSync(`${dir}/${name}.query`,'utf8');
for(const [file,id] of [['required','left::Flag::VALUE'],['provided','<Ready as left::Flag>::VALUE']]) {
  assert(read(file).includes(JSON.stringify(id)), `${file}: exact identity`);
  assert(read(file).includes('value="u16"'), `${file}: exact type`);
}
for(const id of ['left::Flag::VALUE','right::Flag::VALUE','<Ready as left::Flag>::VALUE','<Ready as right::Flag>::VALUE','<Other as left::Flag>::VALUE']) {
  assert(read('outline').split('\n').some(line=>line.startsWith(JSON.stringify(id)+'\t') && line.includes('kind=associated-constant')), `missing declaration ${id}`);
}
const source=Buffer.from(readFileSync(`${dir}/source/main.lkj`));
const refs=read('references').split('\n').filter(line=>line.includes('kind=associated-constant'));
assert.equal(refs.length,2,'main has exactly its two selected constant references');
for(const id of ['<Ready as left::Flag>::VALUE','<Ready as right::Flag>::VALUE']) {
  const row=refs.find(line=>line.startsWith(JSON.stringify(id)+'\t'));
  assert(row, `missing selected owner ${id}`);
  const span=row.match(/start=(\d+) end=(\d+)/); assert(span,'member span');
  assert.equal(source.subarray(Number(span[1]),Number(span[2])).toString(),'VALUE','only the authored member token is referenced');
}
for(const [name,count] of [['left',2],['right',1],['other',0]]) {
  assert.equal(read(name).split('\n').filter(line=>line.includes('fact=uses')&&line.includes('kind=associated-constant')).length,count,`${name}: exact source uses, including idle function`);
}
CHECK
for identity in 'left::Flag::VALUE' '<Ready as left::Flag>::VALUE'; do
    cp "$fixture/source/main.lkj" "$fixture/unchanged"
    node --input-type=module - "$identity" > "$fixture/request.json" <<'REQUEST'
console.log(JSON.stringify({version:1,operations:[{op:'rename',id:process.argv[2],name:'CHANGED'}]}));
REQUEST
    if "$compiler" edit "${common[@]}" --request "$fixture/request.json" > "$fixture/rename.log" 2>&1; then
        echo 'trait constant rename published an incomplete member cohort' >&2; exit 1
    fi
    grep -Fq E_EDIT_REFERENCES "$fixture/rename.log"
    cmp "$fixture/unchanged" "$fixture/source/main.lkj"
    test ! -e "$fixture/source/.lkjscript-edit.pending"
done
for engine in native interpreter; do
    [[ $("$compiler" run "${common[@]}" --engine "$engine") == 42 ]]
done
# Constant initializers are interface inputs, including a body-independent build.
mkdir "$fixture/header-store"
headers=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/header-store" --store-key "$fixture/key" --stats -o "$fixture/header-program")
"$compiler" build "${headers[@]}" > "$fixture/cold.log" 2>&1
"$compiler" build "${headers[@]}" > "$fixture/warm.log" 2>&1
grep -Eq '"build_reused":1' "$fixture/warm.log"
node --input-type=module - "$fixture/source/main.lkj" <<'CHANGE'
import {readFileSync,writeFileSync} from 'node:fs';
const file=process.argv[2], source=readFileSync(file,'utf8');
if(source.split('const VALUE: u16 = 40;').length!==2) throw new Error('expected unique initializer');
writeFileSync(file,source.replace('const VALUE: u16 = 40;','const VALUE: u16 = 41;'));
CHANGE
"$compiler" build "${headers[@]}" > "$fixture/changed.log" 2>&1
grep -Eq '"build_reused":0' "$fixture/changed.log"
grep -Eq '"front_reused":0' "$fixture/changed.log"
for engine in native interpreter; do
    [[ $("$compiler" run main.lkj --source-dir "$fixture/source" --no-library \
        --store "$fixture/header-store" --store-key "$fixture/key" --engine "$engine") == 43 ]]
done
printf 'ok trait constants: exact owners/types/member spans, idle uses, rename rollback and warm initializer invalidation\n'
# Equal spelling cannot merge primitive and nominal implementation identities.
mkdir "$fixture/collisions"
cp tests/conformance/pass/trait_constant_receiver_identity.lkj "$fixture/collisions/main.lkj"
"$compiler" build main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" --index \
    -o "$fixture/collision-program" > "$fixture/collision-index.log"
collision_snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/collision-index.log")
[[ "$collision_snapshot" =~ ^[0-9a-f]{64}$ ]]
"$compiler" query main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" \
    --snapshot "$collision_snapshot" --kind outline --id crate --depth 8 --limit 128 --bytes 100000 > "$fixture/collision-outline.query"
"$compiler" query main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" \
    --snapshot "$collision_snapshot" --kind uses --id '<crate::u8 as Flag>::VALUE' --limit 128 --bytes 100000 > "$fixture/collision-nominal.query"
"$compiler" query main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" \
    --snapshot "$collision_snapshot" --kind uses --id '<u8 as Flag>::VALUE' --limit 128 --bytes 100000 > "$fixture/collision-primitive.query"
node --input-type=module - "$fixture" <<'COLLISIONS'
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const dir=process.argv[2];
const outline=readFileSync(dir+'/collision-outline.query','utf8').split('\n');
const source=readFileSync(dir+'/collisions/main.lkj');
for(const [kind,receiver,spelling] of [
  ['nominal','crate::u8','<u8 as Flag>::VALUE'],
  ['primitive','u8','<u8 as super::Flag>::VALUE'],
]) {
  const id='<'+receiver+' as Flag>::VALUE';
  assert.equal(outline.filter(line=>line.startsWith(JSON.stringify(id)+'\t')&&line.includes('kind=associated-constant')).length,1,'one distinct member '+id);
  assert.equal(outline.filter(line=>line.startsWith('"impl:<'+receiver+' as Flag>#')).length,1,'one distinct implementation '+receiver);
  const rows=readFileSync(dir+'/collision-'+kind+'.query','utf8').split('\n').filter(line=>line.includes('fact=uses')&&line.includes('kind=associated-constant'));
  assert.equal(rows.length,1,'one exact use for '+receiver);
  const span=rows[0].match(/start=(\d+) end=(\d+)/);
  assert(span,'selected member span');
  const start=source.indexOf(spelling)+Buffer.byteLength(spelling)-5;
  assert.equal(Number(span[1]),start,'receiver-specific use offset');
  assert.equal(Number(span[2]),start+5,'member end');
  assert.equal(source.subarray(Number(span[1]),Number(span[2])).toString(),'VALUE');
}
COLLISIONS
printf 'ok trait constants: distinct nominal/primitive catalog identities and exact receiver uses\n'
