#!/usr/bin/env bash
set -eu
compiler=${1:-target/lkjscript}
mkdir -p target/gate
"$compiler" build src/structure_refactor_cases.lkj -o target/gate/structure-refactor-cases
target/gate/structure-refactor-cases
