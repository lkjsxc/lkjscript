#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
awk '
{
  rest=$0
  if (index(rest,"super::super::")) { print FILENAME ": interpreter escapes its module" > "/dev/stderr"; bad=1 }
  while (match(rest,/crate::[A-Za-z_][A-Za-z_0-9]*/)) {
    name=substr(rest,RSTART+7,RLENGTH-7)
    if (name != "ir" && name != "runtime_memory") { print FILENAME ": shared implementation import " name > "/dev/stderr"; bad=1 }
    rest=substr(rest,RSTART+RLENGTH)
  }
}
END { if (bad) exit 1 }
' src/interpreter.lkj src/interpreter/*.lkj src/runtime_memory.lkj
printf 'ok independent interpreter imports\n'
