type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "[i64;5]" array 3 5
type "[i64]" slice shared 3
type "&[i64]" ref shared 8
type "&mut[i64]" ref mut 8
fn "middle" 1 9
return-leaf 0 bound 0
local "input" 9 let
local "start" 4 let
local "end" 4 let
local "result" 9 let
@1 int 1 1
@2 int 2 3
@3 view-range 3 shared exclusive 1 2 0 deref
@4 drop 0
@5 return 3
endfn
fn "probe" 0 3
local "a" 3 let
local "b" 3 let
local "c" 3 let
local "d" 3 let
local "e" 3 let
local "owner" 7 let
local "start" 4 let
local "end" 4 let
local "outer" 9 let
local "inner" 9 let
local "zero" 4 let
local "one" 4 let
local "left" 3 let
local "right" 3 let
local "result" 3 let
@1 int 0 0
@2 int 1 1
@3 int 2 19
@4 int 3 23
@5 int 4 0
@6 array 5 0 1 2 3 4
@7 int 6 1
@8 int 7 5
@9 view-range 8 shared exclusive 6 7 5
@10 call 9 0 8
@11 int 10 0
@12 int 11 1
@13 read 12 copy 9 deref index 10
@14 read 13 copy 9 deref index 11
@15 binary 14 "+" 12 13
@16 drop 10
@17 drop 11
@18 drop 9
@19 drop 5
@20 return 14
endfn
