type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "Choice" enum copy
variant "A" "value" 3
variant "B" "value" 3
endtype
type "Choices" array 7 2
fn "probe" 0 3
local "left" 3 let
local "right" 3 let
local "a" 7 let
local "b" 7 let
local "choices" 8 let
local "index" 4 mut
local "zero" 4 let
local "condition" 1 let
local "one" 4 let
local "result" 3 let
@1 int 0 20
@2 int 1 22
@3 variant 2 0 0
@4 variant 3 1 1
@5 array 4 2 3
@6 int 6 0
@7 assign 6 5
@8 tag-test-place 7 0 4 index 5
@9 read 8 move 5
@9 drop 8
@10 int 5 1
@11 if 7
@12 read 9 copy 4 index 5 variant 0 0
@13 drop 4
@14 drop 5
@14 return 9
@15 else
@16 trap "unreachable"
@17 end
endfn
