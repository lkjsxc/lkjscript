type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "World" world
type "Either" enum move
variant "Data" "0" 3
variant "Capability" "0" 7
endtype
fn "probe" 0 7
local "value" 3 let
local "either" 8 let
local "world" 7 let
@1 int 0 42
@2 variant 1 0 0
@3 read 2 move 1 variant 1 0
@4 return 2
endfn
