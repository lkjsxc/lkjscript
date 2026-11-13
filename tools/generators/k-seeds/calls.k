type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
fn "probe" 0 3
local "seed" 3 let
local "answer" 3 let
@1 int 0 42
@2 call 1 1 0
@3 return 1
endfn
fn "forward" 1 3
local "input" 3 let
local "answer" 3 let
@4 call 1 2 0
@5 return 1
endfn
fn "identity" 1 3
local "input" 3 let
@6 return 0
endfn
