// Executable source and parser contracts migrated from the Rust semantic tests.
use super::{source_case, source_case_at, derived_source_case, source_file, parse_case, record};

pub fn run(shard:usize, workers:usize, errors: &mut Vec<String>) -> usize {
    let mut count=0usize;
    if 0usize % workers == shard {
        record(parse_case("grouping_and_postfix_values_still_require_explicit_statement_terminators_0000", "fn bad(){({}) {}}", "E_PARSE"), errors);
        count += 1;
    }
    if 1usize % workers == shard {
        record(parse_case("grouping_and_postfix_values_still_require_explicit_statement_terminators_0001", "fn bad(){if true {}.clone() {}}", "E_PARSE"), errors);
        count += 1;
    }
    if 2usize % workers == shard {
        record(parse_case("grouping_and_postfix_values_still_require_explicit_statement_terminators_0002", "fn bad()->i64{match 0i64{0=>({42i64}) 1=>1i64,_=>2i64}}", "E_PARSE"), errors);
        count += 1;
    }
    if 3usize % workers == shard {
        record(parse_case("grouping_and_postfix_values_still_require_explicit_statement_terminators_0003", "fn bad()->i64{match 0i64{0=>{42i64}.clone() 1=>1i64,_=>2i64}}", "E_PARSE"), errors);
        count += 1;
    }
    if 4usize % workers == shard {
        record(parse_case("grouping_and_postfix_values_still_require_explicit_statement_terminators_0004", "fn bad()->i64{if true{42i64}else{0i64} as i64}", "E_PARSE"), errors);
        count += 1;
    }
    if 5usize % workers == shard {
        record(parse_case("grouping_and_postfix_values_still_require_explicit_statement_terminators_0005", "fn bad()->i64{if true{} +42i64}", "E_PARSE"), errors);
        count += 1;
    }
    if 6usize % workers == shard {
        record(parse_case("language__syntax__tests__diagnostics_name_rejected_constructs_0000", "fn f() { let b = b\"é\"; }", "E_LEX"), errors);
        count += 1;
    }
    if 7usize % workers == shard {
        record(parse_case("language__syntax__tests__diagnostics_name_rejected_constructs_0001", "fn f() { println!(\"ambient\"); }", "E_CAPABILITY"), errors);
        count += 1;
    }
    if 8usize % workers == shard {
        record(parse_case("language__syntax__tests__diagnostics_name_rejected_constructs_0002", "fn f() { unsafe {} }", "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 9usize % workers == shard {
        record(parse_case("language__syntax__tests__diagnostics_name_rejected_constructs_0003", "fn f() { mystery!(); }", "E_UNSUPPORTED_MACRO"), errors);
        count += 1;
    }
    if 10usize % workers == shard {
        record(parse_case("language__syntax__tests__diagnostics_name_rejected_constructs_0004", "fn f() { /* missing }", "E_LEX"), errors);
        count += 1;
    }
    if 11usize % workers == shard {
        record(source_case("a_call_cannot_hide_a_reference_installed_from_a_shorter_lifetime_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        struct View<'a> { value: &'a i64 }\n        fn install<'a>(slot: &mut View<'a>, replacement: &'a i64) {\n            let old = std::mem::replace(slot, View { value: replacement });\n        }\n        fn probe() -> i64 {\n            let external: i64 = 42;\n            let mut view = View { value: &external };\n            {\n                let local: i64 = 7;\n                install(&mut view, &local);\n            };\n            *view.value\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 12usize % workers == shard {
        record(source_case("a_concrete_implementation_is_not_an_arbitrary_parameter_witness_0002", "case.rs", None, vec![source_file("case.rs", Some("trait Score{fn score(&self)->i64;} impl Score for i64{fn score(&self)->i64{*self}} fn need<T:Score>(x:T){} fn bad<U>(x:U){need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 13usize % workers == shard {
        record(source_case("a_later_instantiation_cannot_hide_an_invalid_derive_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; #[derive(Clone)] struct Item<T> {value:T,bad:NoClone} fn probe()->i64{let x=Item{value:42i64,bad:NoClone};x.value}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 14usize % workers == shard {
        record(source_case("a_parameter_root_does_not_justify_borrowing_its_owned_cache_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        struct View<'a> { value: &'a i64 }\n        struct Data<'a> { view: View<'a>, cache: i64 }\n        fn install<'a>(data: &mut Data<'a>) {\n            let old = std::mem::replace(&mut data.view, View { value: &data.cache });\n        }\n        fn probe() -> i64 {\n            let external: i64 = 42;\n            let mut data = Data { view: View { value: &external }, cache: 7 };\n            install(&mut data);\n            *data.view.value\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 15usize % workers == shard {
        record(source_case("a_single_gap_is_not_reported_as_exhaustive_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{0..128=>0,129..=255=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 16usize % workers == shard {
        record(source_case("a_single_gap_is_not_reported_as_exhaustive_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i8)->i64{match value{-128..0=>0,1..=127=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 17usize % workers == shard {
        record(source_case("a_single_gap_is_not_reported_as_exhaustive_0002", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u64)->i64{match value{0..18446744073709551615u64=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 18usize % workers == shard {
        record(source_case("a_successful_specialization_cannot_hide_a_bad_definition_0002", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T>(x:T)->i64{x} fn probe()->i64{bad(42i64)}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 19usize % workers == shard {
        record(source_case("a_type_parameter_cannot_be_applied_as_a_type_constructor_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("fn invalid<T>(value:T<i64>){} pub fn probe()->i64{0}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 20usize % workers == shard {
        record(source_case("a_user_trait_named_copy_does_not_grant_builtin_copy_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Copy{} fn bad<T:Copy>(x:T)->(T,T){(x,x)}"))], "E_MOVE"), errors);
        count += 1;
    }
    if 21usize % workers == shard {
        record(source_case("a_user_utf8_function_is_selected_by_its_resolved_declaration_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("\n        mod std {\n            pub mod str {\n                pub fn from_utf8(value: i64) -> i64 { value + 1 }\n            }\n        }\n        pub fn probe() -> i64 { std::str::from_utf8(41) }\n    ")),
], ""), errors);
        count += 1;
    }
    if 22usize % workers == shard {
        record(source_case("a_user_utf8_function_is_selected_by_its_resolved_declaration_0002", "$runtime/runtime.rs", None, vec![
source_file("$runtime/runtime.rs", Some("//! Source allocator over sealed raw addresses. Ordinary application stubs trap.\n//! Allocator metadata is private to this runtime; programs receive payload pointers.\n\npub fn raw_load8(_address: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_load64(_address: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store8(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store64(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_memory_copy(_destination: u64, _source: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_map(_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_unmap(_address: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_remap(_address: u64, _old_bytes: u64, _new_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_allocator_state() -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_fail() -> u64 { panic!(\"unsealed runtime primitive\") }\n\nfn add_extent(left: u64, right: u64) -> u64 {\n    if left > 18446744073709551615 - right {\n        return raw_fail();\n    }\n    left + right\n}\n\nfn payload_class(bytes: u64) -> u64 {\n    let mut size: u64 = 16;\n    let mut class: u64 = 0;\n    while size < bytes {\n        if size > 9223372036854775807 {\n            return raw_fail();\n        }\n        size = size * 2;\n        class = class + 1;\n    }\n    class\n}\n\n// Free-list heads occupy the first 64 words of allocator state. The next three\n// words are the current region's cursor, remaining bytes and next region size.\n// Regions grow geometrically; a refused request is retried down to the pages\n// needed by this allocation. Retained regions supply slabs and medium blocks.\nfn take_region(bytes: u64) -> u64 {\n    let state = raw_allocator_state();\n    let mut cursor = raw_load64(state + 512);\n    let mut remaining = raw_load64(state + 520);\n    if remaining < bytes {\n        let minimum = add_extent(bytes, 4095) & !4095u64;\n        let mut size = raw_load64(state + 528);\n        if size == 0 {\n            size = 65536;\n        }\n        if size < minimum {\n            size = minimum;\n        }\n        cursor = raw_map(size);\n        while cursor == 0 && size > minimum {\n            size = (size / 2) & !4095u64;\n            if size < minimum {\n                size = minimum;\n            }\n            cursor = raw_map(size);\n        }\n        if cursor == 0 {\n            return raw_fail();\n        }\n        remaining = size;\n        let mut next = size;\n        if next <= 9223372036854775807 {\n            next = next * 2;\n        }\n        raw_store64(state + 528, next);\n    }\n    raw_store64(state + 512, cursor + bytes);\n    raw_store64(state + 520, remaining - bytes);\n    cursor\n}\n\n// Each block has {class, next_free} followed by a power-of-two payload.\n// Payloads through 1 MiB are reused by class. Small blocks are carved from\n// 4096-byte slabs inside regions; medium blocks are carved directly. Larger\n// blocks have independent mappings so growth can remap without copying.\npub fn allocate(bytes: u64) -> u64 {\n    let class = payload_class(bytes);\n    let size = 16u64 << class;\n    let block_size = add_extent(size, 16);\n    if size > 1048576 {\n        let block = raw_map(block_size);\n        if block == 0 {\n            return raw_fail();\n        }\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let head = raw_allocator_state() + class * 8;\n    let old = raw_load64(head);\n    if old != 0 {\n        raw_store64(head, raw_load64(old + 8));\n        raw_store64(old + 8, 0);\n        return old + 16;\n    }\n    if block_size > 4096 {\n        let block = take_region(block_size);\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let slab = take_region(4096);\n    let count = 4096 / block_size;\n    let mut index: u64 = count;\n    let mut next: u64 = 0;\n    while index != 0 {\n        index = index - 1;\n        let block = slab + index * block_size;\n        raw_store64(block, class);\n        raw_store64(block + 8, next);\n        next = block;\n    }\n    raw_store64(head, raw_load64(slab + 8));\n    raw_store64(slab + 8, 0);\n    slab + 16\n}\n\npub fn release(pointer: u64) -> u64 {\n    if pointer == 0 {\n        return 0;\n    }\n    let block = pointer - 16;\n    let class = raw_load64(block);\n    let size = 16u64 << class;\n    if size > 1048576 {\n        if raw_unmap(block, add_extent(size, 16)) != 0 {\n            return raw_fail();\n        }\n        return 0;\n    }\n    let head = raw_allocator_state() + class * 8;\n    raw_store64(block + 8, raw_load64(head));\n    raw_store64(head, block);\n    0\n}\n\n// No caller retains a pointer into a block whose ownership is transferred here.\n// Shrinking keeps capacity. Dedicated mappings grow with mremap; only pooled\n// blocks copy their payload when moving into a larger size class.\npub fn reallocate(pointer: u64, bytes: u64) -> u64 {\n    if pointer == 0 {\n        return allocate(bytes);\n    }\n    let block = pointer - 16;\n    let old_size = 16u64 << raw_load64(block);\n    if bytes <= old_size {\n        return pointer;\n    }\n    if old_size > 1048576 {\n        let class = payload_class(bytes);\n        let size = 16u64 << class;\n        let replacement = raw_remap(block, add_extent(old_size, 16), add_extent(size, 16));\n        if replacement == 0 {\n            return raw_fail();\n        }\n        raw_store64(replacement, class);\n        return replacement + 16;\n    }\n    let replacement = allocate(bytes);\n    raw_memory_copy(replacement, pointer, old_size);\n    release(pointer);\n    replacement\n}\n\npub fn valid_utf8_bytes(data: u64, length: u64) -> u64 {\n    let mut index: u64 = 0;\n    let mut remaining: u64 = 0;\n    let mut low: u64 = 128;\n    let mut high: u64 = 191;\n    while index < length {\n        let byte = raw_load8(data + index);\n        if remaining != 0 {\n            if byte < low || byte > high {\n                return 0;\n            }\n            remaining = remaining - 1;\n            low = 128;\n            high = 191;\n        } else if byte >= 128 {\n            low = 128;\n            high = 191;\n            if byte >= 194 && byte <= 223 {\n                remaining = 1;\n            } else if byte >= 224 && byte <= 239 {\n                remaining = 2;\n                if byte == 224 {\n                    low = 160;\n                }\n                if byte == 237 {\n                    high = 159;\n                }\n            } else if byte >= 240 && byte <= 244 {\n                remaining = 3;\n                if byte == 240 {\n                    low = 144;\n                }\n                if byte == 244 {\n                    high = 143;\n                }\n            } else {\n                return 0;\n            }\n        }\n        index = index + 1;\n    }\n    if remaining == 0 { 1 } else { 0 }\n}\n\npub fn utf8_validate(data: u64, length: u64) -> u64 {\n    valid_utf8_bytes(data, length)\n}\n")),
], ""), errors);
        count += 1;
    }
    if 23usize % workers == shard {
        record(source_case("a_wildcard_does_not_hide_invalid_later_bounds_or_bodies_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{_=>0,0..=256=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 24usize % workers == shard {
        record(source_case("a_wildcard_does_not_hide_invalid_later_bounds_or_bodies_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{_=>0,4..4=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 25usize % workers == shard {
        record(source_case("a_wildcard_does_not_hide_invalid_later_bounds_or_bodies_0002", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad<T>(value:T)->i64{match 0i64{_=>1,1=>true}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 26usize % workers == shard {
        record(source_case("absent_enum_leaves_survive_calls_clone_and_assignment_0000", "provenance.rs", None, vec![
source_file("provenance.rs", Some("\n        #[derive(Copy, Clone)]\n        enum Maybe<'a> { None, Some(&'a i64) }\n        fn inspect(value: &mut Maybe<'_>) {}\n        fn empty<'a>(external: &'a i64) -> Maybe<'a> {\n            let mut value: Maybe<'a> = Maybe::None;\n            inspect(&mut value);\n            let copied = value.clone();\n            value = copied;\n            value\n        }\n        fn probe() -> i64 {\n            let external: i64 = 42;\n            match empty(&external) {\n                Maybe::None => 42,\n                Maybe::Some(found) => *found\n            }\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 27usize % workers == shard {
        record(source_case("absent_input_reference_leaves_can_still_produce_static_text_0000", "source_bounds.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("source_bounds.rs", Some("\nfn selected<'a>(value:&Option<&'a str>)->&'a str {\n    match value {Option::Some(text)=>{return *text;},Option::None=>{}}\n    \"static\"\n}\npub fn probe()->usize {\n    let value:Option<&str>=Option::None;\n    let returned=selected(&value);\n    drop(value);\n    returned.len()\n}\n")),
], ""), errors);
        count += 1;
    }
    if 28usize % workers == shard {
        record(source_case("abstract_verification_cannot_leak_into_an_executable_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Score{fn score(&self)->i64;} struct Ticket{value:i64} impl Score for Ticket{fn score(&self)->i64{self.value}} fn read<T:Score>(x:&T)->i64{x.score()} fn probe()->i64{read(&Ticket{value:42})}"))], ""), errors);
        count += 1;
    }
    if 29usize % workers == shard {
        record(source_case("alias_arguments_and_field_inference_match_rust_0000", "main.rs", None, vec![
source_file("main.rs", Some("\nstruct Thing {value:i64}\ntype Alias=Thing;\ntype Identity<T> =T;\nstruct Pair<A,B>{first:A,second:B}\ntype Swapped<T,U> =Pair<U,T>;\ntype Fixed<T> =Swapped<T,u8>;\ntype Nested<T> =Pair<Box<T>,u8>;\ntype Chain<X> =Nested<X>;\ntype TupleAlias<T> =Pair<(T,u8),u8>;\ntype ArrayAlias<T> =Pair<[T;2],u8>;\nstruct Holder<T>{value:T}\ntype Held<T> =Holder<T>;\ntype Borrowed<'a,T> =Holder<&'a T>;\nstruct Empty;\ntype EmptyAlias=Empty;\nimpl<T> Holder<T>{fn make(value:T)->Self{Self{value}}}\nfn generic<T>(unused:T)->i64{let result=Held{value:42i64};result.value}\nfn preserve<T>(value:T)->Holder<T>{Held{value}}\nfn take_alias<T>(value:Held<T>)->T{value.value}\npub fn alias_argument_inference()->i64{take_alias(Held{value:42i64})}\npub fn concrete()->i64{Alias{value:42}.value}\npub fn identity_alias()->i64{Identity::<Thing>{value:42}.value}\npub fn explicit()->i64{let v=Swapped::<i64,u8>{second:40,first:2};v.second+v.first as i64}\npub fn inferred()->i64{let v=Swapped{first:2u8,second:40i64};v.second+v.first as i64}\npub fn partial_arguments()->i64{let v=Swapped::<_,u8>{second:40i64,first:2};v.second+v.first as i64}\npub fn fixed_argument()->i64{let v=Fixed{second:40i64,first:2};v.second+v.first as i64}\npub fn expected()->i64{let v:Fixed<i64> =Fixed{second:40,first:2};v.second+v.first as i64}\npub fn nested_chain()->i64{let v=Chain{first:Box::new(40i64),second:2};*v.first+v.second as i64}\npub fn tuple_argument()->i64{let v=TupleAlias{first:(40i64,1u8),second:1};let (a,b)=v.first;a+b as i64+v.second as i64}\npub fn array_argument()->i64{let v=ArrayAlias{first:[20i64,20i64],second:2};v.first[0]+v.first[1]+v.second as i64}\npub fn caller_parameter_is_not_a_constructor_parameter()->i64{generic(true)}\npub fn parameter_transport()->i64{preserve(42i64).value}\npub fn stored_reference()->i64{let number:i64=42;let v=Borrowed{value:&number};*v.value}\npub fn self_constructor()->i64{Holder::<i64>::make(42).value}\npub fn empty_alias()->i64{let value=EmptyAlias{};drop(value);42}\n")),
], ""), errors);
        count += 1;
    }
    if 30usize % workers == shard {
        record(source_case("aliases_cross_files_without_changing_nominal_identity_0000", "main.rs", None, vec![
source_file("main.rs", Some("\nmod a;mod b;\nuse crate::a::Alias as Imported;\nuse crate::b::Thing as Other;\ntype Chain=Imported;\npub fn probe()->i64{\n    let left=Chain{value:40};\n    let right=Other{value:2};\n    a::read(left)+b::read(right)\n}\n")),
source_file("a.rs", Some("pub struct Thing{pub value:i64} pub type Alias=Thing;pub fn read(value:Thing)->i64{value.value}")),
source_file("b.rs", Some("pub struct Thing{pub value:i64} pub fn read(value:Thing)->i64{value.value}")),
], ""), errors);
        count += 1;
    }
    if 31usize % workers == shard {
        record(source_case("aliases_preserve_private_fields_and_private_underlying_types_0000", "main.rs", None, vec![
source_file("main.rs", Some("mod secret{pub struct Token{value:i64}pub type Alias=Token;}fn probe(){let value=secret::Alias{value:9};}")),
], "E_PRIVACY"), errors);
        count += 1;
    }
    if 32usize % workers == shard {
        record(source_case("aliases_preserve_private_fields_and_private_underlying_types_0001", "main.rs", None, vec![
source_file("main.rs", Some("mod secret{pub struct Token{value:i64}pub type Alias=Token;pub fn make()->Alias{Token{value:9}}}fn probe()->i64{secret::make().value}")),
], "E_PRIVACY"), errors);
        count += 1;
    }
    if 33usize % workers == shard {
        record(source_case("aliases_preserve_private_fields_and_private_underlying_types_0002", "main.rs", None, vec![
source_file("main.rs", Some("mod secret{struct Token{pub value:i64}pub type Alias=Token;}fn probe(){let value=secret::Alias{value:9};}")),
], "E_PRIVACY"), errors);
        count += 1;
    }
    if 34usize % workers == shard {
        record(source_case("array_type_is_the_only_repeat_extent_0000", "repeat.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("repeat.rs", Some("pub fn probe()->i64 { let a=[7u8;4]; a.len() as i64 }")),
], ""), errors);
        count += 1;
    }
    if 35usize % workers == shard {
        record(source_case("assignment_requires_exclusive_writable_storage_0000", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("fn bad(value:&Box<i64>){*value=Box::new(42);}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 36usize % workers == shard {
        record(source_case("assignment_requires_exclusive_writable_storage_0001", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("fn bad(value:&Vec<Box<i64>>){value[0]=Box::new(42);}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 37usize % workers == shard {
        record(source_case("assignment_requires_exclusive_writable_storage_0002", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("struct Holder{value:Box<i64>} fn bad(){let holder=Holder{value:Box::new(1)};holder.value=Box::new(42);}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 38usize % workers == shard {
        record(source_case("associated_results_retain_their_declared_identity_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Source{type Item;fn get(&self)->Self::Item;} fn good<T:Source>(x:&T)->T::Item{x.get()}"))], ""), errors);
        count += 1;
    }
    if 39usize % workers == shard {
        record(source_case("associated_results_retain_their_declared_identity_0003", "case.rs", None, vec![source_file("case.rs", Some("trait Source{type Item;fn get(&self)->Self::Item;} fn bad<T:Source>(x:&T)->i64{x.get()}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 40usize % workers == shard {
        record(source_case("builtin_and_declared_bounds_with_the_same_method_are_ambiguous_0002", "case.rs", None, vec![source_file("case.rs", Some("trait Other{fn clone(&self)->Self;} fn ambiguous<T:Clone+Other>(value:&T)->T{value.clone()}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 41usize % workers == shard {
        record(source_case("builtin_and_declared_bounds_with_the_same_method_are_ambiguous_0004", "case.rs", None, vec![source_file("case.rs", Some("trait Other{fn clone(&self)->Self;} fn ambiguous<T:Copy+Other>(value:&T)->T{value.clone()}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 42usize % workers == shard {
        record(source_case("builtin_and_declared_bounds_with_the_same_method_are_ambiguous_0006", "case.rs", None, vec![source_file("case.rs", Some("trait Other{fn clone_from(&mut self,other:&Self);} fn ambiguous<T:Clone+Other>(left:&mut T,right:&T){left.clone_from(right);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 43usize % workers == shard {
        record(source_case("builtin_and_declared_bounds_with_the_same_method_are_ambiguous_0008", "case.rs", None, vec![source_file("case.rs", Some("trait Other{fn eq(&self,other:&Self)->bool;} fn ambiguous<T:PartialEq+Other>(left:&T,right:&T)->bool{left.eq(right)}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 44usize % workers == shard {
        record(source_case("builtin_and_declared_bounds_with_the_same_method_are_ambiguous_0009", "case.rs", None, vec![source_file("case.rs", Some("trait Other{fn ne(&self,other:&Self)->bool;} fn ambiguous<T:Eq+Other>(left:&T,right:&T)->bool{left.ne(right)}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 45usize % workers == shard {
        record(source_case("builtin_clone_cannot_strengthen_a_same_named_declared_bound_0000", "trait_identity.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("trait_identity.rs", Some("trait Clone{} trait Contract{fn take<T:crate::Clone>(&self,value:T)->T;}\n         mod nested{pub struct Counter;impl crate::Contract for Counter{fn take<T:Clone>(&self,value:T)->T{value}}}\n         pub fn probe()->i64{0}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 46usize % workers == shard {
        record(source_case("builtin_comparison_methods_must_have_the_exact_contract_0000", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{number:i64} impl PartialEq for Value{fn eq(self,other:Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 47usize % workers == shard {
        record(source_case("builtin_comparison_methods_must_have_the_exact_contract_0001", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{number:i64} impl PartialEq for Value{fn eq(&mut self,other:&Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 48usize % workers == shard {
        record(source_case("builtin_comparison_methods_must_have_the_exact_contract_0002", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{number:i64} impl PartialEq for Value{fn eq(&self,other:Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 49usize % workers == shard {
        record(source_case("builtin_comparison_methods_must_have_the_exact_contract_0003", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{number:i64} impl PartialEq for Value{fn eq(&self,other:&Self)->i64{1}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 50usize % workers == shard {
        record(source_case("builtin_comparison_methods_must_have_the_exact_contract_0004", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{number:i64} impl PartialEq for Value{fn eq<T>(&self,other:&Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 51usize % workers == shard {
        record(source_case("builtin_comparison_methods_must_have_the_exact_contract_0005", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{} impl PartialEq for Value{fn eq(&self,other:&Self)->bool{true} fn ne(&self,other:&Self)->i64{1}} fn bad(left:Value,right:Value)->bool{left!=right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 52usize % workers == shard {
        record(source_case("builtin_comparison_methods_must_have_the_exact_contract_0006", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{} impl PartialEq for Value{fn ne(&self,other:&Self)->bool{false}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 53usize % workers == shard {
        record(source_case("builtin_copy_does_not_imply_a_same_named_user_clone_0000", "trait_identity.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("trait_identity.rs", Some("trait Clone{} trait Contract{fn take<T:Copy>(&self,value:T)->T;}\n         mod nested{pub struct Counter;impl crate::Contract for Counter{fn take<T:crate::Clone>(&self,value:T)->T{value}}}\n         pub fn probe()->i64{0}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 54usize % workers == shard {
        record(source_case("builtin_default_operations_have_checked_executable_bodies_0001", "case.rs", None, vec![source_file("case.rs", Some("\nstruct Item(i64);\nimpl Clone for Item{fn clone(&self)->Self{Item(self.0)}}\nimpl PartialEq for Item{fn eq(&self,other:&Self)->bool{self.0==other.0}}\nfn update<T:Clone>(left:&mut T,right:&T){left.clone_from(right);}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\nfn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\npub fn updated()->i64{let mut left=Item(1);let right=Item(42);update(&mut left,&right);left.0}\npub fn defaults()->i64{if different(&Item(1),&Item(2)) && same(&42i64,&42i64){42}else{0}}\n"))], ""), errors);
        count += 1;
    }
    if 55usize % workers == shard {
        record(source_case("builtin_method_evidence_requires_a_self_receiver_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Clone for Item{fn clone(value:&Self)->Self{Item{value:value.value}}} #[derive(Clone)] struct Holder(Item);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 56usize % workers == shard {
        record(source_case("builtin_method_evidence_requires_a_self_receiver_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl PartialEq for Item{fn eq(left:&Self,right:&Self)->bool{left.value==right.value}} #[derive(PartialEq)] struct Holder(Item);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 57usize % workers == shard {
        record(source_case("builtin_method_is_not_a_same_spelled_generic_free_call_0001", "case.rs", None, vec![source_file("case.rs", Some("fn len<T>(value:T)->usize{let values=vec![value];values.len()} fn probe()->usize{len(42i64)}"))], ""), errors);
        count += 1;
    }
    if 58usize % workers == shard {
        record(source_case("builtin_text_cannot_be_inferred_as_a_parameter_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("fn text_only(value:&str)->i64{7} pub fn probe()->i64{let value=1i64;text_only(&value)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 59usize % workers == shard {
        record(source_case("byte_readers_match_rust_0000", "probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("probe.rs", Some("use std::bytes::*; use std::collections::BTreeMap;\n\n        fn bytes_probe() -> i64 {\n            let mut bytes: Vec<u8> = vec![65, 66];\n            let tail: Vec<u8> = vec![67, 240, 159, 152, 128];\n            append_bytes(&mut bytes, &tail);\n            let prefix: Vec<u8> = vec![65, 66, 67];\n            assert!(starts_with(&bytes, &prefix));\n            assert!(valid_utf8(&bytes));\n            let invalid: Vec<u8> = vec![237, 160, 128];\n            assert!(!valid_utf8(&invalid));\n            find_byte(&bytes, 67).unwrap() as i64 + bytes.len() as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 60usize % workers == shard {
        record(source_case("byte_vector_keys_use_source_lexicographic_order_0000", "probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("probe.rs", Some("use std::bytes::*; use std::collections::BTreeMap;\n\n        fn byte_keys_probe() -> i64 {\n            let mut map: BTreeMap<Vec<u8>, usize> = BTreeMap::new();\n            map.insert(vec![3, 2], 32);\n            map.insert(vec![1], 10);\n            map.insert(vec![3, 1], 31);\n            map.insert(vec![3, 2, 0], 320);\n            let key: Vec<u8> = vec![3, 2];\n            let value = { let found = map.get(&key).unwrap(); *found };\n            assert_eq!(key.len(), 2);\n            let before: Vec<u8> = vec![3, 1];\n            let removed = map.remove(&before).unwrap();\n            value as i64 + removed as i64 + map.len() as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 61usize % workers == shard {
        record(source_case("calls_require_the_declared_generic_bounds_0001", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Copy>(x:T){} fn bad<U>(x:U){need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 62usize % workers == shard {
        record(source_case("calls_require_the_declared_generic_bounds_0003", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Copy>(x:T){} fn good<U:Copy>(x:U){need(x);}"))], ""), errors);
        count += 1;
    }
    if 63usize % workers == shard {
        record(source_case("changing_the_production_join_changes_generated_behavior_0000", "text_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_component.rs", Some("mod names; use names::qualified as join;\npub fn compare_0()->usize {\nlet value=join(\"\",\"leaf\");\nlet actual=value.as_bytes();\nlet expected=\"leaf\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\npub fn compare_1()->usize {\nlet value=join(\"module\",\"leaf\");\nlet actual=value.as_bytes();\nlet expected=\"module::leaf\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\n")),
source_file("names.rs", Some("//! Canonical declaration names used by resolution, generic validation, and elaboration.\n/// Join one module path and declaration spelling without changing either input.\npub fn qualified(module: &str, name: &str) -> String {\n    let mut result = module.to_string();\n    if !module.is_empty() {\n        result.push_str(\"::\");\n    }\n    result.push_str(name);\n    result\n}\n")),
], ""), errors);
        count += 1;
    }
    if 64usize % workers == shard {
        record(source_case("changing_the_production_join_changes_generated_behavior_0001", "text_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_component.rs", Some("mod names; use names::qualified as join;\npub fn compare_0()->usize {\nlet value=join(\"\",\"leaf\");\nlet actual=value.as_bytes();\nlet expected=\"leaf\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\npub fn compare_1()->usize {\nlet value=join(\"module\",\"leaf\");\nlet actual=value.as_bytes();\nlet expected=\"module::leaf\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\n")),
source_file("names.rs", Some("//! Canonical declaration names used by resolution, generic validation, and elaboration.\n/// Join one module path and declaration spelling without changing either input.\npub fn qualified(module: &str, name: &str) -> String {\n    let mut result = module.to_string();\n    if !module.is_empty() {\n        result.push_str(\":\");\n    }\n    result.push_str(name);\n    result\n}\n")),
], ""), errors);
        count += 1;
    }
    if 65usize % workers == shard {
        record(source_case("character_construction_checks_its_signature_on_both_ir_boundaries_0000", "unicode.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("unicode.rs", Some("pub fn probe()->i64 { match char::from_u32(97u32) { Some(c) => c as u32 as i64, None => 0 } }")),
], ""), errors);
        count += 1;
    }
    if 66usize % workers == shard {
        record(source_case("checked_associated_contracts_lower_with_concrete_witnesses_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Source{type Item;fn get(&self)->Self::Item;} struct Integer{value:i64} impl Source for Integer{type Item=i64;fn get(&self)->Self::Item{self.value}} fn read<T:Source>(x:&T)->T::Item{x.get()} fn probe()->i64{read(&Integer{value:42})}"))], ""), errors);
        count += 1;
    }
    if 67usize % workers == shard {
        record(source_case("checked_builtin_and_declared_trait_calls_keep_their_selection_0001", "case.rs", None, vec![source_file("case.rs", Some("\nstruct Item { value:i64 }\nimpl Clone for Item {fn clone(&self)->Self{Item{value:1}}}\ntrait Score {fn score(&self)->i64;}\nimpl Score for Item {fn score(&self)->i64{3}}\nimpl Item {\n    fn clone(&self)->Self{Item{value:2}}\n    fn score(&self)->i64{4}\n}\nfn copied<T:Clone>(value:&T)->T{value.clone()}\nfn scored<T:Score>(value:&T)->i64{value.score()}\npub fn selected_clone()->i64{copied(&Item{value:0}).value}\npub fn direct_clone()->i64{Item{value:0}.clone().value}\npub fn selected_score()->i64{scored(&Item{value:0})}\npub fn direct_score()->i64{Item{value:0}.score()}\n"))], ""), errors);
        count += 1;
    }
    if 68usize % workers == shard {
        record(source_case("checked_entry_arguments_cannot_construct_addresses_or_invalid_scalars_0000", "entry.rs", None, vec![
source_file("entry.rs", Some("pub fn probe(value:&i64)->i64{*value}")),
], ""), errors);
        count += 1;
    }
    if 69usize % workers == shard {
        record(source_case("checked_entry_arguments_cannot_construct_addresses_or_invalid_scalars_0001", "entry.rs", None, vec![
source_file("entry.rs", Some("pub fn probe(value:bool)->bool{value}")),
], ""), errors);
        count += 1;
    }
    if 70usize % workers == shard {
        record(source_case("checked_receiver_dereferences_do_not_change_when_a_parameter_becomes_a_reference_0001", "case.rs", None, vec![source_file("case.rs", Some("\nfn copied<T:Clone>(value:&T)->T{value.clone()}\ntrait Score {fn score(&self)->i64;}\nimpl Score for &i64 {fn score(&self)->i64{**self+1}}\nfn scored<T:Score>(value:&T)->i64{value.score()}\npub fn cloned_reference()->i64 {\n    let value:i64=42;let reference=&value;\n    let copied_reference=copied::<&i64>(&reference);\n    *copied_reference\n}\npub fn scored_reference()->i64 {\n    let value:i64=42;let reference=&value;\n    scored::<&i64>(&reference)\n}\n"))], ""), errors);
        count += 1;
    }
    if 71usize % workers == shard {
        record(source_case("clone_cannot_return_a_view_of_a_dead_local_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        #[derive(Copy, Clone)]\n        struct View<'a> { value: &'a i64 }\n        fn leak<'a>(external: &'a i64) -> View<'a> {\n            let local: i64 = 7;\n            let view = View { value: &local };\n            view.clone()\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 72usize % workers == shard {
        record(source_case("clone_from_rejects_overlapping_borrows_and_views_cannot_outlive_clones_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn probe(){let mut value=String::from_str(\"owner\"); value.clone_from(&value);}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 73usize % workers == shard {
        record(source_case("clone_from_rejects_overlapping_borrows_and_views_cannot_outlive_clones_0001", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn probe()->u8{let mut value=String::from_str(\"owner\");let view=value.as_bytes();let other=String::from_str(\"other\");value.clone_from(&other);view[0]}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 74usize % workers == shard {
        record(source_case("clone_from_rejects_overlapping_borrows_and_views_cannot_outlive_clones_0002", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn bad<'a>(value:&'a String)->&'a [u8]{let copied=value.clone();copied.as_bytes()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 75usize % workers == shard {
        record(source_case("clone_preserves_valid_shared_reference_leaves_0000", "provenance.rs", None, vec![
source_file("provenance.rs", Some("\n        #[derive(Copy, Clone)]\n        struct View<'a> { value: &'a i64 }\n        fn copied<'a>(external: &'a i64) -> View<'a> {\n            let view = View { value: external };\n            view.clone()\n        }\n        fn probe() -> i64 {\n            let value: i64 = 42;\n            *copied(&value).value\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 76usize % workers == shard {
        record(source_case("cloned_shared_payloads_cannot_escape_their_actual_owners_0000", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn bad<'a>(external:&'a i64)->Option<&'a i64>{let local:i64=7;let carrier=Option::Some(&local);carrier.clone()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 77usize % workers == shard {
        record(source_case("cloned_shared_payloads_cannot_escape_their_actual_owners_0001", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn bad<'a>(external:&'a i64)->Result<i64,&'a i64>{let local:i64=7;let carrier:Result<i64,&i64>=Result::Err(&local);carrier.clone()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 78usize % workers == shard {
        record(source_case("cloned_shared_payloads_cannot_escape_their_actual_owners_0002", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn require<T:Clone>(value:&T){}fn bad(){let mut value:i64=7;let carrier=Option::Some(&mut value);require(&carrier);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 79usize % workers == shard {
        record(source_case("cloned_string_vectors_preserve_every_byte_after_original_cleanup_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("use std::{__lkjscript_vec_clone,__lkjscript_vec_equal};\n\nfn same_bytes(actual: &String, expected: &str) -> bool {\n    let left=actual.as_bytes();let right=expected.as_bytes();\n    if left.len()!=right.len(){return false;}\n    let mut index:usize=0;\n    while index<left.len(){\n        if left[index]!=right[index]{return false;}\n        index=index+1;\n    }\n    true\n}\nfn cloned<T:Clone>(value:&T)->T{value.clone()}\nfn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\nfn same_operator<T:PartialEq>(left:&T,right:&T)->bool{*left==*right}\n\nstruct Token { phase:i64 }\nimpl Drop for Token {\n    fn drop(&mut self) {\n        assert!(self.phase==2);\n        self.phase=3;\n    }\n}\nstruct Item { value:i64, generation:i64, token:Box<Token> }\nimpl Item {\n    fn clone(&self)->Self {\n        Item{value:self.value,generation:99,token:Box::new(Token{phase:1})}\n    }\n}\nimpl Clone for Item {\n    fn clone(&self)->Self {\n        assert!(self.token.phase==1);\n        Item{value:self.value,generation:self.generation+1,token:Box::new(Token{phase:1})}\n    }\n}\nimpl PartialEq for Item {\n    fn eq(&self,other:&Self)->bool { self.value==other.value }\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool { !self.eq(other) }\n}\nimpl Eq for Item {}\nimpl Drop for Item {\n    fn drop(&mut self) {\n        assert!(self.token.phase==1);\n        self.token.phase=2;\n    }\n}\nfn item(value:i64)->Item {\n    Item{value,generation:0,token:Box::new(Token{phase:1})}\n}\n\npub fn affine_clones_have_independent_storage_and_ordered_cleanup()->i64 {\n    let mut original:Vec<Item>=Vec::new();\n    original.push(item(4));original.push(item(2));\n    let direct=original[0].clone();\n    if direct.generation!=99 {return 8;}\n    drop(direct);\n    let first=original.clone();\n    let mut second=cloned(&first);\n    if first[0].generation!=1 || first[1].generation!=1 {return 1;}\n    if second[0].generation!=2 || second[1].generation!=2 {return 2;}\n    if second[0].value!=4 || second[1].value!=2 {return 3;}\n    drop(first);\n    if original[0].token.phase!=1 || second[0].token.phase!=1 {return 4;}\n    original[0].value=7;\n    if second[0].value!=4 {return 5;}\n    drop(original);\n    let last=second.pop().unwrap();\n    if last.value!=2 || last.generation!=2 {return 6;}\n    drop(last);\n    if second[0].token.phase!=1 {return 7;}\n    0\n}\n\npub fn equality_uses_element_eq_and_preserves_owners()->i64 {\n    let mut left:Vec<Item>=Vec::new();left.push(item(4));left.push(item(2));\n    let mut right=cloned(&left);\n    if !(left==right) || left!=right {return 1;}\n    if !left.eq(&right) || left.ne(&right) {return 2;}\n    if !same(&left,&right) || different(&left,&right) {return 3;}\n    if !same_operator(&left,&right) {return 4;}\n    right[1].value=9;\n    if left==right || !(left!=right) {return 5;}\n    if same(&left,&right) || !different(&left,&right) {return 6;}\n    if !left.ne(&right) {return 7;}\n    left.push(item(8));right.pop();\n    if left.len()!=3 || right.len()!=1 {return 8;}\n    if left==right || right==left || !left.ne(&right) {return 10;}\n    if left[0].token.phase!=1 || right[0].token.phase!=1 {return 9;}\n    0\n}\n\npub fn nested_vectors_and_strings_clone_as_owned_values()->i64 {\n    let empty:Vec<String>=Vec::new();\n    if empty.clone().len()!=0 {return 5;}\n    let mut inner:Vec<String>=Vec::new();\n    inner.push(String::from_str(\"\"));inner.push(String::from_str(\"a\\0水😀\"));\n    let mut outer:Vec<Vec<String>>=Vec::new();outer.push(inner);\n    let copied=cloned(&outer);\n    if !same(&outer,&copied) || outer!=copied {return 1;}\n    outer[0][1].clear();outer[0][1].push_str(\"changed\");\n    if !same_bytes(&copied[0][1],\"a\\0水😀\") {return 2;}\n    if outer==copied || !different(&outer,&copied) {return 3;}\n    drop(outer);\n    if !same_bytes(&copied[0][0],\"\") || !same_bytes(&copied[0][1],\"a\\0水😀\") {return 4;}\n    0\n}\n\npub fn floating_vector_equality_needs_partial_eq_only()->i64 {\n    let mut left:Vec<f64>=vec![1.5,-0.0,4.25];\n    let mut right:Vec<f64>=vec![1.5,0.0,4.25];\n    if left!=right || !same(&left,&right) {return 1;}\n    let not_a_number:f64=0.0/0.0;\n    let values:Vec<f64>=vec![not_a_number];\n    if values==values || same(&values,&values) {return 2;}\n    if !values.ne(&values) || !different(&values,&values) {return 3;}\n    let empty:Vec<f64>=Vec::new();\n    if empty!=empty || !(empty==empty) {return 4;}\n    left.push(8.0);right.pop();\n    if left.len()!=4 || right.len()!=2 {return 5;}\n    0\n}\n\npub fn exact_source_helpers_match_the_ordinary_vector_api()->i64 {\n    let mut values:Vec<Item>=Vec::new();values.push(item(1));values.push(item(3));\n    let explicit=__lkjscript_vec_clone(&values);\n    let ordinary=values.clone();\n    if !__lkjscript_vec_equal(&explicit,&ordinary) {return 1;}\n    if explicit[0].generation!=ordinary[0].generation {return 2;}\n    if explicit[1].value!=3 || ordinary[0].value!=1 {return 3;}\n    drop(values);drop(ordinary);\n    if explicit[0].token.phase!=1 {return 4;}\n    0\n}\npub fn all_bytes()->i64{\nlet mut values:Vec<String>=Vec::new();\nvalues.push(String::from_str(\"\"));\nvalues.push(String::from_str(\"\\0\"));\nvalues.push(String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"));\nvalues.push(String::from_str(\"α水😀\\0\\u{10ffff}\"));\nvalues.push(String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"));\nlet copied=values.clone();let copied_twice=cloned(&copied);drop(values);\nif !same_bytes(&copied[0],\"\"){return 1;}\nif !same_bytes(&copied[1],\"\\0\"){return 2;}\nif !same_bytes(&copied[2],\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"){return 3;}\nif !same_bytes(&copied[3],\"α水😀\\0\\u{10ffff}\"){return 4;}\nif !same_bytes(&copied[4],\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"){return 5;}\ndrop(copied);\nif !same_bytes(&copied_twice[0],\"\"){return 6;}\nif !same_bytes(&copied_twice[1],\"\\0\"){return 7;}\nif !same_bytes(&copied_twice[2],\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"){return 8;}\nif !same_bytes(&copied_twice[3],\"α水😀\\0\\u{10ffff}\"){return 9;}\nif !same_bytes(&copied_twice[4],\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"){return 10;}\n0}\n")),
], ""), errors);
        count += 1;
    }
    if 80usize % workers == shard {
        record(source_case("clones_and_clone_from_match_rust_over_every_byte_and_keep_independent_owners_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\nfn same_bytes(value: &String, expected: &str) -> bool {\n    let actual = value.as_bytes();\n    let expected_bytes = expected.as_bytes();\n    if actual.len() != expected_bytes.len() { return false; }\n    let mut index: usize = 0;\n    while index < actual.len() {\n        if actual[index] != expected_bytes[index] { return false; }\n        index = index + 1;\n    }\n    true\n}\npub fn lifecycle_0() -> usize {\nlet mut original = String::from_str(\"\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"\") { return 1; }\nif !same_bytes(&cloned, \"\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\npub fn lifecycle_1() -> usize {\nlet mut original = String::from_str(\"a\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"a\") { return 1; }\nif !same_bytes(&cloned, \"a\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"a\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"a\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"a\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"a\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"a\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\npub fn lifecycle_2() -> usize {\nlet mut original = String::from_str(\"ab\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"ab\") { return 1; }\nif !same_bytes(&cloned, \"ab\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"ab\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"ab\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"ab\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"ab\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"ab\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\npub fn lifecycle_3() -> usize {\nlet mut original = String::from_str(\"b\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"b\") { return 1; }\nif !same_bytes(&cloned, \"b\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"b\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"b\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"b\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"b\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"b\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\npub fn lifecycle_4() -> usize {\nlet mut original = String::from_str(\"\\0\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"\\0\") { return 1; }\nif !same_bytes(&cloned, \"\\0\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"\\0\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"\\0\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"\\0\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"\\0\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"\\0\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\npub fn lifecycle_5() -> usize {\nlet mut original = String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\") { return 1; }\nif !same_bytes(&cloned, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\npub fn lifecycle_6() -> usize {\nlet mut original = String::from_str(\"α水😀\\0\\u{10ffff}\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"α水😀\\0\\u{10ffff}\") { return 1; }\nif !same_bytes(&cloned, \"α水😀\\0\\u{10ffff}\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"α水😀\\0\\u{10ffff}\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"α水😀\\0\\u{10ffff}\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"α水😀\\0\\u{10ffff}\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"α水😀\\0\\u{10ffff}\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"α水😀\\0\\u{10ffff}\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\npub fn lifecycle_7() -> usize {\nlet mut original = String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nlet mut cloned = original.clone();\nif !same_bytes(&original, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\") { return 1; }\nif !same_bytes(&cloned, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\") { return 2; }\noriginal.clear(); original.push_str(\"original\\0😀\");\nif !same_bytes(&original, \"original\\0😀\") { return 3; }\nif !same_bytes(&cloned, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\") { return 4; }\ncloned.clear(); cloned.push_str(\"clone\\u{7f}水\");\nif !same_bytes(&original, \"original\\0😀\") { return 5; }\nif !same_bytes(&cloned, \"clone\\u{7f}水\") { return 6; }\nlet survivor = original.clone(); drop(original);\nif !same_bytes(&survivor, \"original\\0😀\") { return 7; }\ndrop(cloned); let discarded = survivor.clone(); drop(discarded);\nif !same_bytes(&survivor, \"original\\0😀\") { return 8; }\nlet mut target = String::from_str(\"preexisting allocation\");\n{ let temporary = String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); target.clone_from(&temporary); }\nif !same_bytes(&target, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\") { return 9; }\ntarget.clone_from(&survivor);\nif !same_bytes(&target, \"original\\0😀\") { return 10; }\nlet empty = String::new(); target.clone_from(&empty);\nif !same_bytes(&target, \"\") { return 11; }\nlet long = String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let short = String::from_str(\"s\");\nlet mut round:usize=0; while round<16 {\ntarget.clone_from(&long); if !same_bytes(&target,\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\") {return 12;}\ntarget.clone_from(&short); if !same_bytes(&target,\"s\") {return 13;}\ntarget.clone_from(&empty); if !same_bytes(&target,\"\") {return 14;}\nround=round+1; }\ntarget.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\nif !same_bytes(&target, \"original\\0😀!\") { return 14; }\n0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 81usize % workers == shard {
        record(source_case("comparisons_use_borrowed_declared_contracts_0001", "case.rs", None, vec![source_file("case.rs", Some("fn less<T:Ord>(left:&T,right:&T)->bool{*left < *right}"))], ""), errors);
        count += 1;
    }
    if 82usize % workers == shard {
        record(source_case("comparisons_use_borrowed_declared_contracts_0003", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T:Copy>(left:T,right:T)->bool{left < right}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 83usize % workers == shard {
        record(source_case("concrete_specialization_executes_a_derived_clone_0001", "case.rs", None, vec![source_file("case.rs", Some("#[derive(Clone)] struct Item{value:Box<i64>} fn copied<T:Clone>(value:&T)->T{value.clone()} pub fn probe()->i64{let value=Item{value:Box::new(42)};let copy=copied(&value);drop(value);*copy.value}"))], ""), errors);
        count += 1;
    }
    if 84usize % workers == shard {
        record(source_case("conditional_traits_require_every_payload_even_when_that_variant_is_absent_0000", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("struct Owned{value:Box<i64>}fn require<T:Copy>(value:T){}fn bad(){let value:Option<Owned>=Option::None;require(value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 85usize % workers == shard {
        record(source_case("conditional_traits_require_every_payload_even_when_that_variant_is_absent_0001", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("struct Owned{value:Box<i64>}fn require<T:Copy>(value:T){}fn bad(){let value:Result<i64,Owned>=Result::Ok(1);require(value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 86usize % workers == shard {
        record(source_case("conditional_traits_require_every_payload_even_when_that_variant_is_absent_0002", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("struct NoClone{}fn require<T:Clone>(value:&T){}fn bad(){let value:Option<NoClone>=Option::None;require(&value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 87usize % workers == shard {
        record(source_case("conditional_traits_require_every_payload_even_when_that_variant_is_absent_0003", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("struct NoClone{}fn require<T:Clone>(value:&T){}fn bad(){let value:Result<i64,NoClone>=Result::Ok(1);require(&value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 88usize % workers == shard {
        record(source_case("conditional_traits_require_every_payload_even_when_that_variant_is_absent_0004", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn require<T:Eq>(value:&T){}fn bad(){let value:Option<f64>=Option::None;require(&value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 89usize % workers == shard {
        record(source_case("conditional_traits_require_every_payload_even_when_that_variant_is_absent_0005", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn require<T:Eq>(value:&T){}fn bad(){let value:Result<i64,f64>=Result::Ok(1);require(&value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 90usize % workers == shard {
        record(source_case("conditional_traits_require_every_payload_even_when_that_variant_is_absent_0006", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("struct NoEq{}impl NoEq{fn eq(&self,other:&Self)->bool{true}}fn require<T:PartialEq>(value:&T){}fn bad(){let value:Option<NoEq>=Option::None;require(&value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 91usize % workers == shard {
        record(source_case("conditional_view_joins_preserve_present_and_absent_reference_leaves_0000", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("\nstruct Numbers { left:i64, right:i64 }\nenum Maybe<'a> { Empty, Full{left:&'a i64,right:&'a i64} }\nfn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {\n    let carrier=if present {\n        Maybe::Full{left:&values.left,right:&values.right}\n    } else {Maybe::Empty};\n    carrier\n}\npub fn conditional_views()->i64 {\n    let numbers=Numbers{left:20,right:22};\n    let first=optional(&numbers,true);\n    let second=optional(&numbers,false);\n    let a=match first {Maybe::Full{left,right}=>*left+*right,Maybe::Empty=>0};\n    let b=match second {Maybe::Full{left,right}=>*left+*right,Maybe::Empty=>0};\n    a+b-42\n}\n")),
], ""), errors);
        count += 1;
    }
    if 92usize % workers == shard {
        record(source_case("container_clone_from_remains_an_explicit_element_update_boundary_0000", "derived_operations.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_operations.rs", Some("fn probe(){let mut target=Box::new(1i64);target.clone_from(&Box::new(2i64));}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 93usize % workers == shard {
        record(source_case("container_clone_from_remains_an_explicit_element_update_boundary_0001", "derived_operations.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_operations.rs", Some("fn probe(){let mut target=[1i64];target.clone_from(&[2i64]);}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 94usize % workers == shard {
        record(source_case("container_clone_from_remains_an_explicit_element_update_boundary_0002", "derived_operations.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_operations.rs", Some("fn probe(){let mut target=vec![1i64];target.clone_from(&vec![2i64]);}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 95usize % workers == shard {
        record(source_case("copy_derive_shortcut_matches_declaration_genericity_0000", "derived_operations.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_operations.rs", Some("\n#[derive(Copy,PartialEq,Eq)] struct Leaf{value:i64}\nimpl Clone for Leaf{fn clone(&self)->Self{Leaf{value:42}}}\nimpl Leaf{fn clone(&self)->Self{Leaf{value:99}}}\n#[derive(Clone,Copy)] struct Plain{value:Leaf}\n#[derive(Clone,Copy)] struct Generic<T>{value:T}\n#[derive(Clone,Copy)] struct Lifetime<'a>{value:Leaf,reference:&'a i64}\n#[derive(Clone,Copy)] enum PlainEnum{Value(Leaf)}\n#[derive(Clone,Copy)] enum GenericEnum<T>{Value(T)}\n#[derive(Clone)] struct Affine{value:Leaf}\nfn cloned<T:Clone>(value:&T)->T{value.clone()}\nfn wrapped<T:Clone>(value:&Generic<T>)->Generic<T>{value.clone()}\npub fn copies()->i64{\n let number:i64=7;\n let plain=Plain{value:Leaf{value:1}}.clone();\n let generic=wrapped(&Generic{value:Leaf{value:1}});\n let lifetime=Lifetime{value:Leaf{value:1},reference:&number}.clone();\n let affine=Affine{value:Leaf{value:1}}.clone();\n let array=[Leaf{value:1},Leaf{value:2}].clone();\n let tuple=(Leaf{value:1},).clone();\n let selected=cloned(&Leaf{value:1});\n if plain.value.value!=1 || generic.value.value!=42 || lifetime.value.value!=1 {return 1;}\n if affine.value.value!=42 || array[0].value!=42 || array[1].value!=42 || tuple.0.value!=42 || selected.value!=42 {return 2;}\n if (Leaf{value:1}).clone().value!=99 {return 3;}\n let one=PlainEnum::Value(Leaf{value:1}).clone();\n let two=GenericEnum::Value(Leaf{value:1}).clone();\n match one {PlainEnum::Value(value)=>{if value.value!=1{return 4;}}}\n match two {GenericEnum::Value(value)=>{if value.value!=42{return 5;}}}\n 0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 96usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0001", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:World){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 97usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0003", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:World){need(value);}"))], ""), errors);
        count += 1;
    }
    if 98usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0006", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:&World){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 99usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0008", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:&World){need(value);}"))], ""), errors);
        count += 1;
    }
    if 100usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0010", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Box<World>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 101usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0012", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Box<World>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 102usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0014", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Vec<World>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 103usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0016", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Vec<World>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 104usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0018", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:[World;1]){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 105usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0020", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:[World;1]){need(value);}"))], ""), errors);
        count += 1;
    }
    if 106usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0022", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:(World,)){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 107usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0024", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:(World,)){need(value);}"))], ""), errors);
        count += 1;
    }
    if 108usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0026", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Holder<World>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 109usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0028", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Holder<World>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 110usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0030", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Maybe<World>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 111usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0032", "case.rs", None, vec![source_file("case.rs", Some("struct World{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Maybe<World>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 112usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0034", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Stdout){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 113usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0036", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Stdout){need(value);}"))], ""), errors);
        count += 1;
    }
    if 114usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0039", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:&Stdout){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 115usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0041", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:&Stdout){need(value);}"))], ""), errors);
        count += 1;
    }
    if 116usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0043", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Box<Stdout>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 117usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0045", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Box<Stdout>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 118usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0047", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Vec<Stdout>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 119usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0049", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Vec<Stdout>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 120usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0051", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:[Stdout;1]){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 121usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0053", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:[Stdout;1]){need(value);}"))], ""), errors);
        count += 1;
    }
    if 122usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0055", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:(Stdout,)){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 123usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0057", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:(Stdout,)){need(value);}"))], ""), errors);
        count += 1;
    }
    if 124usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0059", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Holder<Stdout>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 125usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0061", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Holder<Stdout>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 126usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0063", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn unused<U>(value:Maybe<Stdout>){need(value);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 127usize % workers == shard {
        record(source_case("data_excludes_typed_capabilities_through_structural_children_0065", "case.rs", None, vec![source_file("case.rs", Some("struct Stdout{value:i64} struct Holder<T>{value:T} enum Maybe<T>{None,Some(T)} fn need<T:Data>(value:T){} fn ordinary<U>(value:Maybe<Stdout>){need(value);}"))], ""), errors);
        count += 1;
    }
    if 128usize % workers == shard {
        record(source_case("declared_library_bytes_determine_the_emitted_behavior_0000", "main.rs", Some("library/lib.rs"), vec![
source_file("main.rs", Some("mod child; pub fn main(mut world:World) { let mut out=world.stdout(); out.write(&[child::answer() as u8]); }")),
source_file("child.rs", Some("pub fn answer()->i64 { std::answer() }")),
source_file("library/lib.rs", Some("pub fn answer()->i64 { 41 }")),
], ""), errors);
        count += 1;
    }
    if 129usize % workers == shard {
        record(source_case("declared_library_bytes_determine_the_emitted_behavior_0001", "main.rs", Some("library/lib.rs"), vec![
source_file("main.rs", Some("mod child; pub fn main(mut world:World) { let mut out=world.stdout(); out.write(&[child::answer() as u8]); }")),
source_file("child.rs", Some("pub fn answer()->i64 { std::answer() }")),
source_file("library/lib.rs", Some("pub fn answer()->i64 { 42 }")),
], ""), errors);
        count += 1;
    }
    if 130usize % workers == shard {
        record(source_case("declared_parameters_shadow_unrelated_nominal_names_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("struct T; fn identity<T>(value:T)->T{value} pub fn probe()->i64{identity(7i64)}")),
], ""), errors);
        count += 1;
    }
    if 131usize % workers == shard {
        record(source_case("declared_parameters_shadow_unrelated_nominal_names_0003", "scope.rs", None, vec![
source_file("scope.rs", Some("enum E{Nothing} struct Holder<E>{value:E} impl<E> Holder<E>{fn take(self)->E{self.value}} pub fn probe()->i64{let value:Holder<i64>=Holder{value:9};value.take()}")),
], ""), errors);
        count += 1;
    }
    if 132usize % workers == shard {
        record(source_case("declared_parameters_shadow_unrelated_nominal_names_0005", "scope.rs", None, vec![
source_file("scope.rs", Some("struct E; struct Counter(i64); impl Counter{fn echo<E>(&self,value:E)->E{value}} pub fn probe()->i64{let counter=Counter(0);counter.echo(11i64)}")),
], ""), errors);
        count += 1;
    }
    if 133usize % workers == shard {
        record(source_case("declared_parameters_shadow_unrelated_nominal_names_0007", "scope.rs", None, vec![
source_file("scope.rs", Some("fn identity<str>(value:str)->str{value} pub fn probe()->i64{identity(13i64)}")),
], ""), errors);
        count += 1;
    }
    if 134usize % workers == shard {
        record(source_case("declared_supertraits_and_aliases_retain_their_identity_0000", "trait_identity.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("trait_identity.rs", Some("trait Clone{} trait Copy:Clone{} trait Contract{fn take<T:crate::Copy>(&self,value:T)->T;}\n         mod nested{use crate::Clone as RootClone;pub struct Counter;impl crate::Contract for Counter{fn take<T:RootClone>(&self,value:T)->T{value}}}\n         pub fn probe()->i64{0}")),
], ""), errors);
        count += 1;
    }
    if 135usize % workers == shard {
        record(source_case("declared_trait_methods_use_their_signatures_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Score{fn score(&self)->i64;} fn good<T:Score>(x:&T)->i64{x.score()}"))], ""), errors);
        count += 1;
    }
    if 136usize % workers == shard {
        record(source_case("declared_trait_methods_use_their_signatures_0003", "case.rs", None, vec![source_file("case.rs", Some("trait Score{fn score(&self)->i64;} fn bad<T:Score>(x:&T)->bool{x.score()}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 137usize % workers == shard {
        record(source_case("derived_and_explicit_implementations_cannot_overlap_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] struct Item {value:i64} impl Clone for Item {fn clone(&self)->Self{Item{value:self.value}}}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 138usize % workers == shard {
        record(source_case("derived_and_explicit_implementations_cannot_overlap_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] struct Item<T> {value:T} impl Clone for Item<i64> {fn clone(&self)->Self{Item{value:self.value}}}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 139usize % workers == shard {
        record(source_case("derived_and_explicit_implementations_cannot_overlap_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] struct Item {value:i64} type Alias=Item; impl Clone for Alias {fn clone(&self)->Self{Item{value:self.value}}}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 140usize % workers == shard {
        record(source_case("derived_and_explicit_implementations_cannot_overlap_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("trait Clone {fn clone(&self)->i64;} #[derive(Clone)] struct Item {value:i64} impl Clone for Item {fn clone(&self)->i64{self.value}}")),
], ""), errors);
        count += 1;
    }
    if 141usize % workers == shard {
        record(source_case("derived_comparisons_execute_and_keep_owners_usable_0000", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("#[derive(PartialEq)] struct Value{number:i64} pub fn probe()->i64{let left=Value{number:21};let right=Value{number:21};if !(left==right)||left!=right{return 0;}left.number+right.number}")),
], ""), errors);
        count += 1;
    }
    if 142usize % workers == shard {
        record(source_case("derived_copy_on_generic_layouts_is_conditional_0001", "case.rs", None, vec![source_file("case.rs", Some("#[derive(Copy,Clone)] struct Holder<T>{value:T} fn wrap<T>(x:T)->Holder<T>{Holder{value:x}}"))], ""), errors);
        count += 1;
    }
    if 143usize % workers == shard {
        record(source_case("derived_copy_on_generic_layouts_is_conditional_0003", "case.rs", None, vec![source_file("case.rs", Some("#[derive(Copy,Clone)] enum Maybe<T>{None,Some(T)} fn wrap<T>(x:T)->Maybe<T>{Maybe::Some(x)}"))], ""), errors);
        count += 1;
    }
    if 144usize % workers == shard {
        record(source_case("derived_fields_use_trait_equality_and_short_circuit_0000", "derived_operations.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_operations.rs", Some("\n#[derive(Clone)] struct Leaf{value:i64}\nimpl PartialEq for Leaf {\n fn eq(&self,other:&Self)->bool{assert!(self.value<100);self.value==other.value}\n fn ne(&self,other:&Self)->bool{false}\n}\nimpl Eq for Leaf{}\nimpl Leaf{fn eq(&self,other:&Self)->bool{false}}\n#[derive(Clone,PartialEq,Eq)] struct Pair{first:Leaf,second:Leaf}\n#[derive(Clone,PartialEq,Eq)] enum Choice{Empty,Named{first:Leaf,second:Leaf},Tuple(Leaf)}\nfn equal<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\npub fn fields()->i64{\n let left=Pair{first:Leaf{value:1},second:Leaf{value:100}};\n let right=Pair{first:Leaf{value:2},second:Leaf{value:100}};\n if left==right || !left.ne(&right) || equal(&left,&right){return 1;}\n let left=Choice::Named{first:Leaf{value:1},second:Leaf{value:100}};\n let right=Choice::Named{first:Leaf{value:2},second:Leaf{value:100}};\n if left==right || left==Choice::Empty || left==Choice::Tuple(Leaf{value:1}) {return 2;}\n let first=Pair{first:Leaf{value:1},second:Leaf{value:2}};\n let second=first.clone();\n if first!=second || !first.eq(&second) || first.ne(&second){return 3;}\n let tuples=(Leaf{value:1},Leaf{value:2});\n if tuples!=tuples.clone(){return 4;}\n let array=[Leaf{value:1},Leaf{value:2}];\n if array!=array.clone(){return 5;}\n if ()!=(){return 6;}\n 0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 145usize % workers == shard {
        record(source_case("derived_holders_clone_their_vector_storage_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("\n#[derive(Clone)] struct Holder {values:Vec<i64>}\npub fn derived_holder()->i64 {\n    let empty=Holder{values:Vec::new()};\n    let copied_empty=empty.clone();\n    drop(empty);\n    if copied_empty.values.len()!=0 {return 1;}\n    let mut original=Holder{values:vec![4,2]};\n    let copied=original.clone();\n    original.values[0]=9;\n    drop(original);\n    if copied.values.len()!=2 || copied.values[0]!=4 || copied.values[1]!=2 {return 2;}\n    0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 146usize % workers == shard {
        record(source_case("derived_supertraits_require_actual_evidence_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Copy)] struct Marker;")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 147usize % workers == shard {
        record(source_case("derived_supertraits_require_actual_evidence_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Eq)] struct Key(i64);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 148usize % workers == shard {
        record(source_case("derived_supertraits_require_actual_evidence_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(PartialOrd)] struct Key(i64);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 149usize % workers == shard {
        record(source_case("derived_supertraits_require_actual_evidence_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Ord)] struct Key(i64);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 150usize % workers == shard {
        record(source_case("derived_supertraits_require_actual_evidence_0004", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Copy,Clone)] struct Marker;")),
], ""), errors);
        count += 1;
    }
    if 151usize % workers == shard {
        record(source_case("derived_supertraits_require_actual_evidence_0005", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(PartialEq,Eq,PartialOrd,Ord)] enum Key {Empty,Number(i64)}")),
], ""), errors);
        count += 1;
    }
    if 152usize % workers == shard {
        record(source_case("derived_supertraits_require_actual_evidence_0006", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Copy)] struct Marker {value:i64} impl Clone for Marker {fn clone(&self)->Self {Marker{value:self.value}}}")),
], ""), errors);
        count += 1;
    }
    if 153usize % workers == shard {
        record(source_case("destructor_registration_uses_the_builtin_trait_identity_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Drop{fn value(&self)->i64;} struct Thing{} impl Drop for Thing{fn value(&self)->i64{42}} fn probe()->i64{Thing{}.value()}"))], ""), errors);
        count += 1;
    }
    if 154usize % workers == shard {
        record(source_case("destructor_registration_uses_the_builtin_trait_identity_0004", "case.rs", None, vec![source_file("case.rs", Some("trait Drop{fn drop(&self)->i64;} struct Thing{} impl Drop for Thing{fn drop(&self)->i64{42}} fn probe()->i64{Thing{}.drop()}"))], ""), errors);
        count += 1;
    }
    if 155usize % workers == shard {
        record(source_case("destructor_registration_uses_the_builtin_trait_identity_0007", "case.rs", None, vec![source_file("case.rs", Some("struct Thing{} impl Thing{fn drop(&self)->i64{42}} fn probe()->i64{Thing{}.drop()}"))], ""), errors);
        count += 1;
    }
    if 156usize % workers == shard {
        record(source_case("destructor_registration_uses_the_builtin_trait_identity_0010", "case.rs", None, vec![source_file("case.rs", Some("struct Thing{} impl Drop for Thing{fn drop(&mut self){}} impl Thing{fn drop(&self)->i64{42}} fn probe()->i64{Thing{}.drop()}"))], ""), errors);
        count += 1;
    }
    if 157usize % workers == shard {
        record(source_case("destructor_registration_uses_the_builtin_trait_identity_0013", "case.rs", None, vec![source_file("case.rs", Some("struct Thing{} impl Drop for Thing{fn drop(&mut self){}} fn probe(){let mut value=Thing{};value.drop();}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 158usize % workers == shard {
        record(source_case("diagnostics_name_the_original_child_file_0001", "main.rs", Some("library/lib.rs"), vec![
source_file("main.rs", Some("mod child; pub fn main(mut world:World) { let mut out=world.stdout(); out.write(&[child::answer() as u8]); }")),
source_file("child.rs", Some("pub fn answer()->i64 { true }")),
source_file("library/lib.rs", Some("pub fn answer()->i64 { 42 }")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 159usize % workers == shard {
        record(source_case("direct_reference_replacement_cannot_erase_an_escape_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        fn leak<'a>(external: &'a i64) -> &'a i64 {\n            let local: i64 = 7;\n            let mut pointer = &local;\n            std::mem::replace(&mut pointer, external)\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 160usize % workers == shard {
        record(source_case("direct_reference_replacement_preserves_exclusive_owners_0000", "provenance.rs", None, vec![
source_file("provenance.rs", Some("\n        struct Data { a: i64, b: i64 }\n        fn probe() -> i64 {\n            let mut data = Data { a: 1, b: 2 };\n            let mut pointer = &mut data.a;\n            let old = std::mem::replace(&mut pointer, &mut data.b);\n            *old = 19;\n            *pointer = 23;\n            *old + *pointer\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 161usize % workers == shard {
        record(source_case("duplicate_and_unsupported_derives_are_not_ignored_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone,Clone)] struct Item;")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 162usize % workers == shard {
        record(source_case("duplicate_and_unsupported_derives_are_not_ignored_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] #[derive(Clone)] struct Item;")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 163usize % workers == shard {
        record(source_case("duplicate_and_unsupported_derives_are_not_ignored_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Debug)] struct Item;")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 164usize % workers == shard {
        record(source_case("duplicate_and_unsupported_derives_are_not_ignored_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Hash)] struct Item;")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 165usize % workers == shard {
        record(source_case("duplicate_and_unsupported_derives_are_not_ignored_0004", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Unknown)] struct Item;")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 166usize % workers == shard {
        record(source_case("element_constraints_and_existing_view_storage_rules_are_enforced_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("fn bad<T>(value:&Vec<T>)->Vec<T>{value.clone()}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 167usize % workers == shard {
        record(source_case("element_constraints_and_existing_view_storage_rules_are_enforced_0001", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("fn bad<T>(left:&Vec<T>,right:&Vec<T>)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 168usize % workers == shard {
        record(source_case("element_constraints_and_existing_view_storage_rules_are_enforced_0002", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("struct Item;fn probe(){let values:Vec<Item>=Vec::new();values.clone();}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 169usize % workers == shard {
        record(source_case("element_constraints_and_existing_view_storage_rules_are_enforced_0003", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("struct Item;fn probe()->bool{let values:Vec<Item>=Vec::new();values==values}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 170usize % workers == shard {
        record(source_case("element_constraints_and_existing_view_storage_rules_are_enforced_0004", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("fn probe(){let values:Vec<i64>=Vec::new();values.clone::<bool>();}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 171usize % workers == shard {
        record(source_case("element_constraints_and_existing_view_storage_rules_are_enforced_0005", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("fn probe()->bool{let values:Vec<i64>=Vec::new();let wrong:i64=1;values.eq(wrong)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 172usize % workers == shard {
        record(source_case("element_constraints_and_existing_view_storage_rules_are_enforced_0006", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("fn probe(){let value:i64=1;let values=vec![&value];values.clone();}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 173usize % workers == shard {
        record(source_case("empty_and_reversed_ranges_are_rejected_before_coverage_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{4..4=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 174usize % workers == shard {
        record(source_case("empty_and_reversed_ranges_are_rejected_before_coverage_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{..0=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 175usize % workers == shard {
        record(source_case("empty_and_reversed_ranges_are_rejected_before_coverage_0002", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i8)->i64{match value{..-128i8=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 176usize % workers == shard {
        record(source_case("empty_and_reversed_ranges_are_rejected_before_coverage_0003", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u64)->i64{match value{18446744073709551615u64..0u64=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 177usize % workers == shard {
        record(source_case("empty_and_reversed_ranges_are_rejected_before_coverage_0004", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{5..=4=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 178usize % workers == shard {
        record(source_case("empty_and_reversed_ranges_are_rejected_before_coverage_0005", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i8)->i64{match value{-2..=-3=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 179usize % workers == shard {
        record(source_case("empty_and_reversed_ranges_are_rejected_before_coverage_0006", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i8)->i64{match value{-0..0=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 180usize % workers == shard {
        record(source_case("endpoint_names_are_explicitly_unsupported_instead_of_runtime_values_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8,limit:u8)->i64{match value{0..limit=>1,_=>0}}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 181usize % workers == shard {
        record(source_case("endpoint_names_are_explicitly_unsupported_instead_of_runtime_values_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("const LIMIT:u8=10;fn bad(value:u8)->i64{match value{0..=LIMIT=>1,_=>0}}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 182usize % workers == shard {
        record(source_case("entire_production_ir_module_executes_cloning_equality_and_representation_access_0000", "src/ir_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("src/ir_component.rs", Some("//! Execution probes for the unchanged production IR representation module.\nmod ir;\n\nfn flag(value: &bool) -> i64 {\n    if *value { 1 } else { 0 }\n}\n\nfn place(value: usize) -> ir::Place {\n    let mut result = ir::Place::local(value);\n    result.projections.push(ir::Projection::Deref);\n    result.projections.push(ir::Projection::Field(value + 1));\n    result\n}\n\nfn operation(which: usize, changed: bool) -> ir::Op {\n    let n: usize = if changed { 43 } else { 42 };\n    if changed && which >= 19 && which <= 23 {\n        return ir::Op::Trap {\n            message: \"different tag\".to_string(),\n        };\n    }\n    match which {\n        0 => ir::Op::Text {\n            dst: n,\n            bytes: vec![0u8, 65u8, 255u8],\n        },\n        1 => ir::Op::Int { dst: n, value: -17 },\n        2 => ir::Op::Float {\n            dst: n,\n            bits: 4607182418800017408u64,\n        },\n        3 => ir::Op::Unit { dst: n },\n        4 => ir::Op::Read {\n            dst: n,\n            place: place(7),\n            moving: true,\n        },\n        5 => ir::Op::Borrow {\n            dst: n,\n            place: place(8),\n            mutable: false,\n        },\n        6 => ir::Op::SliceBorrow {\n            dst: n,\n            place: place(9),\n            mutable: true,\n        },\n        7 => ir::Op::Array {\n            dst: n,\n            elements: vec![3usize, 5usize],\n        },\n        8 => ir::Op::Assign {\n            place: place(n),\n            src: 11,\n        },\n        9 => ir::Op::Drop { place: place(n) },\n        10 => ir::Op::Unary {\n            dst: n,\n            op: ir::UnaryOp::Not,\n            src: 12,\n        },\n        11 => ir::Op::Binary {\n            dst: n,\n            op: ir::BinaryOp::Add,\n            left: 13,\n            right: 14,\n        },\n        12 => ir::Op::Call {\n            dst: n,\n            function: 4,\n            args: vec![15usize, 16usize],\n        },\n        13 => ir::Op::Struct {\n            dst: n,\n            fields: vec![17usize, 18usize],\n        },\n        14 => ir::Op::Variant {\n            dst: n,\n            variant: 2,\n            fields: vec![19usize, 20usize],\n        },\n        15 => ir::Op::Discriminant { dst: n, src: 21 },\n        16 => ir::Op::TagTest {\n            dst: n,\n            place: ir::Place::local(22),\n            variant: 3,\n        },\n        17 => ir::Op::Intrinsic {\n            dst: n,\n            name: ir::IntrinsicOp::VecNew,\n            args: vec![23usize, 24usize],\n        },\n        18 => ir::Op::If { condition: n },\n        19 => ir::Op::Else,\n        20 => ir::Op::End,\n        21 => ir::Op::Loop,\n        22 => ir::Op::Break,\n        23 => ir::Op::Continue,\n        24 => ir::Op::Return { src: n },\n        26 => ir::Op::DropIf { place: place(n) },\n        27 => ir::Op::BreakTo { depth: n },\n        28 => ir::Op::ContinueTo { depth: n },\n        29 => ir::Op::ViewRange {\n            dst: n,\n            place: place(6),\n            start: 7,\n            end: 8,\n            inclusive: true,\n            mutable: false,\n        },\n        _ => ir::Op::Trap {\n            message: if changed {\n                \"changed\".to_string()\n            } else {\n                \"trap\\0é水😀\".to_string()\n            },\n        },\n    }\n}\n\nfn operation_value(value: &ir::Op) -> i64 {\n    match value {\n        ir::Op::Text { dst, bytes } => *dst as i64 + bytes.len() as i64 + bytes[2] as i64,\n        ir::Op::Int { dst, value } => *dst as i64 + *value,\n        ir::Op::Float { dst, bits } => *dst as i64 + (*bits / 65536) as i64,\n        ir::Op::Unit { dst } => *dst as i64,\n        ir::Op::Read { dst, place, moving } => {\n            *dst as i64 + place.local as i64 + place.projections.len() as i64 + flag(moving)\n        }\n        ir::Op::Borrow {\n            dst,\n            place,\n            mutable,\n        } => *dst as i64 + place.local as i64 + flag(mutable),\n        ir::Op::SliceBorrow {\n            dst,\n            place,\n            mutable,\n        } => *dst as i64 + place.local as i64 + flag(mutable),\n        ir::Op::ViewRange {\n            dst,\n            place,\n            start,\n            end,\n            inclusive,\n            mutable,\n        } => {\n            *dst as i64\n                + place.local as i64\n                + *start as i64\n                + *end as i64\n                + flag(inclusive)\n                + flag(mutable)\n        }\n        ir::Op::Array { dst, elements } => *dst as i64 + elements[0] as i64 + elements[1] as i64,\n        ir::Op::Assign { place, src } => place.local as i64 + *src as i64,\n        ir::Op::Drop { place } => place.local as i64,\n        ir::Op::DropIf { place } => place.local as i64 + 1,\n        ir::Op::Unary { dst, op, src } => {\n            *dst as i64 + op.as_str().as_bytes()[0] as i64 + *src as i64\n        }\n        ir::Op::Binary {\n            dst,\n            op,\n            left,\n            right,\n        } => *dst as i64 + op.as_str().as_bytes()[0] as i64 + *left as i64 + *right as i64,\n        ir::Op::Call {\n            dst,\n            function,\n            args,\n        } => *dst as i64 + *function as i64 + args[0] as i64 + args[1] as i64,\n        ir::Op::Struct { dst, fields } => *dst as i64 + fields[0] as i64 + fields[1] as i64,\n        ir::Op::Variant {\n            dst,\n            variant,\n            fields,\n        } => *dst as i64 + *variant as i64 + fields[0] as i64 + fields[1] as i64,\n        ir::Op::Discriminant { dst, src } => *dst as i64 + *src as i64,\n        ir::Op::TagTest {\n            dst,\n            place,\n            variant,\n        } => *dst as i64 + place.local as i64 + *variant as i64,\n        ir::Op::Intrinsic { dst, name, args } => {\n            *dst as i64 + name.as_str().len() as i64 + args[0] as i64 + args[1] as i64\n        }\n        ir::Op::If { condition } => *condition as i64,\n        ir::Op::Else => 1001,\n        ir::Op::End => 1002,\n        ir::Op::Loop => 1003,\n        ir::Op::Break => 1004,\n        ir::Op::Continue => 1005,\n        ir::Op::BreakTo { depth } => 1006 + *depth as i64,\n        ir::Op::ContinueTo { depth } => 1007 + *depth as i64,\n        ir::Op::Return { src } => *src as i64,\n        ir::Op::Trap { message } => message.len() as i64,\n    }\n}\n\nfn kind_value(kind: &ir::TypeKind) -> i64 {\n    match kind {\n        ir::TypeKind::Unit => 1,\n        ir::TypeKind::Bool => 2,\n        ir::TypeKind::Char => 5,\n        ir::TypeKind::Int { bits, signed } => *bits as i64 * 2 + flag(signed),\n        ir::TypeKind::F64 => 3,\n        ir::TypeKind::Struct { fields, copy } => {\n            fields.len() as i64 + fields[0].name.len() as i64 + fields[0].ty as i64 + flag(copy)\n        }\n        ir::TypeKind::Enum { variants, copy } => {\n            variants.len() as i64\n                + variants[0].name.len() as i64\n                + variants[1].fields[0].ty as i64\n                + flag(copy)\n        }\n        ir::TypeKind::Ref { inner, mutable } => *inner as i64 + flag(mutable),\n        ir::TypeKind::Slice { inner, mutable } => *inner as i64 + flag(mutable),\n        ir::TypeKind::Array { inner, len } => *inner as i64 + *len as i64,\n        ir::TypeKind::Vec { inner } => *inner as i64,\n        ir::TypeKind::Box { inner } => *inner as i64,\n        ir::TypeKind::Str => 4,\n        ir::TypeKind::World => 5,\n        ir::TypeKind::Stdout => 6,\n        ir::TypeKind::Stdin => 7,\n        ir::TypeKind::Stderr => 8,\n        ir::TypeKind::Directory => 9,\n        ir::TypeKind::File => 10,\n        ir::TypeKind::Process => 11,\n        ir::TypeKind::Clock => 12,\n        ir::TypeKind::Child => 13,\n    }\n}\n\nfn model() -> ir::Program {\n    let mut types = ir::primitives();\n    types.push(ir::Type {\n        name: \"record\".to_string(),\n        kind: ir::TypeKind::Struct {\n            fields: vec![ir::Field {\n                name: \"field\\0水\".to_string(),\n                ty: ir::I64,\n            }],\n            copy: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"choice\".to_string(),\n        kind: ir::TypeKind::Enum {\n            variants: vec![\n                ir::Variant {\n                    name: \"Empty\".to_string(),\n                    fields: Vec::new(),\n                },\n                ir::Variant {\n                    name: \"Full\".to_string(),\n                    fields: vec![ir::Field {\n                        name: \"value\".to_string(),\n                        ty: ir::I64,\n                    }],\n                },\n            ],\n            copy: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"reference\".to_string(),\n        kind: ir::TypeKind::Ref {\n            inner: ir::I64,\n            mutable: true,\n        },\n    });\n    types.push(ir::Type {\n        name: \"slice\".to_string(),\n        kind: ir::TypeKind::Slice {\n            inner: ir::U8,\n            mutable: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"array\".to_string(),\n        kind: ir::TypeKind::Array {\n            inner: ir::BOOL,\n            len: 17,\n        },\n    });\n    types.push(ir::Type {\n        name: \"vector\".to_string(),\n        kind: ir::TypeKind::Vec { inner: ir::I32 },\n    });\n    types.push(ir::Type {\n        name: \"box\".to_string(),\n        kind: ir::TypeKind::Box { inner: ir::U64 },\n    });\n    types.push(ir::Type {\n        name: \"str\".to_string(),\n        kind: ir::TypeKind::Str,\n    });\n    types.push(ir::Type {\n        name: \"world\".to_string(),\n        kind: ir::TypeKind::World,\n    });\n    types.push(ir::Type {\n        name: \"stdout\".to_string(),\n        kind: ir::TypeKind::Stdout,\n    });\n    types.push(ir::Type {\n        name: \"stdin\".to_string(),\n        kind: ir::TypeKind::Stdin,\n    });\n    types.push(ir::Type {\n        name: \"stderr\".to_string(),\n        kind: ir::TypeKind::Stderr,\n    });\n    types.push(ir::Type {\n        name: \"directory\".to_string(),\n        kind: ir::TypeKind::Directory,\n    });\n    types.push(ir::Type {\n        name: \"file\".to_string(),\n        kind: ir::TypeKind::File,\n    });\n    let mut body: Vec<ir::Instruction> = Vec::new();\n    let mut index: usize = 0;\n    while index < 30 {\n        body.push(ir::Instruction {\n            column: 1,\n            op: operation(index, false),\n            line: 100 + index,\n        });\n        index = index + 1;\n    }\n    let function = ir::Function {\n        file: 2,\n        name: \"function\\0é水😀\".to_string(),\n        params: 1,\n        result: ir::I64,\n        return_source: Option::Some(ir::ReturnSource::Parameter {\n            index: 0,\n            projections: vec![\n                ir::Projection::Deref,\n                ir::Projection::Field(3),\n                ir::Projection::VariantField(2, 4),\n                ir::Projection::Index(9),\n            ],\n        }),\n        locals: vec![ir::Local {\n            name: \"parameter\".to_string(),\n            ty: ir::I64,\n            mutable: true,\n        }],\n        body,\n        test: true,\n    };\n    ir::Program {\n        types,\n        functions: vec![function],\n        destructors: vec![(7usize, 0usize)],\n    }\n}\n\npub fn primitive_snapshot(index: usize, field: usize) -> i64 {\n    let original = ir::primitives();\n    let copied = original.clone();\n    drop(original);\n    if field == 0 {\n        return kind_value(&copied[index].kind);\n    }\n    if field == 1 {\n        return copied[index].name.len() as i64;\n    }\n    let bytes = copied[index].name.as_bytes();\n    if field - 2 < bytes.len() {\n        bytes[field - 2] as i64\n    } else {\n        -1\n    }\n}\n\npub fn capability_snapshot(which: usize) -> i64 {\n    let original = match which {\n        0 => ir::TypeKind::Process,\n        1 => ir::TypeKind::Clock,\n        _ => ir::TypeKind::Child,\n    };\n    let copied = original.clone();\n    assert!(copied == original);\n    drop(original);\n    let different = if which == 0 {\n        ir::TypeKind::Clock\n    } else {\n        ir::TypeKind::Process\n    };\n    assert!(copied != different);\n    kind_value(&copied)\n}\n\npub fn operation_snapshot(which: usize) -> i64 {\n    let original = operation(which, false);\n    let copied = original.clone();\n    assert!(original == copied);\n    drop(original);\n    operation_value(&copied)\n}\n\npub fn operation_equality(which: usize, changed: bool) -> i64 {\n    let original = operation(which, false);\n    let candidate = operation(which, changed);\n    let result = original == candidate;\n    assert!((original != candidate) == !result);\n    if result { 1 } else { 0 }\n}\n\npub fn program_snapshot(field: usize) -> i64 {\n    let mut original = model();\n    let copied = original.clone();\n    assert!(copied == original);\n    original.functions[0].name.push_str(\" changed\");\n    original.functions[0].body[0].op = operation(0, true);\n    original.destructors[0].0 = 99;\n    assert!(copied != original);\n    drop(original);\n    let checked = ir::CheckedProgram { program: copied };\n    let retained: ir::Program = checked.program().clone();\n    drop(checked);\n    assert!(retained == model());\n    if field == 0 {\n        let mut total: i64 = 0;\n        let mut index: usize = 0;\n        while index < retained.types.len() {\n            total = total + kind_value(&retained.types[index].kind);\n            index = index + 1;\n        }\n        return total;\n    }\n    if field == 1 {\n        let mut total: i64 = 0;\n        let mut index: usize = 0;\n        while index < retained.functions[0].body.len() {\n            total = total + operation_value(&retained.functions[0].body[index].op);\n            total = total + retained.functions[0].body[index].line as i64;\n            index = index + 1;\n        }\n        return total;\n    }\n    if field == 2 {\n        return retained.functions[0].name.len() as i64;\n    }\n    if field == 3 {\n        return retained.destructors[0].0 as i64;\n    }\n    if field == 4 {\n        return retained.functions[0].locals[0].name.len() as i64;\n    }\n    match &retained.functions[0].return_source {\n        Option::None => -1,\n        Option::Some(source) => match source {\n            ir::ReturnSource::Static => -2,\n            ir::ReturnSource::Leaves(leaves) => leaves.len() as i64,\n            ir::ReturnSource::Parameter { index, projections } => {\n                *index as i64 + projections.len() as i64\n            }\n        },\n    }\n}\n\nfn projections_value(projections: &Vec<ir::Projection>) -> i64 {\n    let mut result: i64 = 0;\n    let mut cursor: usize = 0;\n    while cursor < projections.len() {\n        result = result\n            + match &projections[cursor] {\n                ir::Projection::Deref => 1,\n                ir::Projection::Field(value) => *value as i64,\n                ir::Projection::VariantField(tag, value) => *tag as i64 + *value as i64,\n                ir::Projection::Index(value) => *value as i64,\n            };\n        cursor = cursor + 1;\n    }\n    result\n}\n\npub fn return_source_snapshot(which: usize) -> i64 {\n    let original: Option<ir::ReturnSource> = if which == 0 {\n        Option::None\n    } else if which == 1 {\n        Option::Some(ir::ReturnSource::Static)\n    } else if which == 2 {\n        Option::Some(ir::ReturnSource::Parameter {\n            index: 17,\n            projections: vec![\n                ir::Projection::Deref,\n                ir::Projection::Field(2),\n                ir::Projection::VariantField(3, 4),\n                ir::Projection::Index(5),\n            ],\n        })\n    } else {\n        let origin = if which == 5 {\n            ir::LeafOrigin::Static\n        } else {\n            ir::LeafOrigin::Inputs {\n                inputs: vec![\n                    ir::InputLeaf {\n                        index: 11,\n                        projections: vec![ir::Projection::Deref, ir::Projection::Field(7)],\n                    },\n                    ir::InputLeaf {\n                        index: 13,\n                        projections: vec![ir::Projection::Field(2)],\n                    },\n                ],\n                exact: which == 3,\n            }\n        };\n        Option::Some(ir::ReturnSource::Leaves(vec![ir::ReturnLeaf {\n            projections: vec![ir::Projection::VariantField(2, 9)],\n            origin,\n        }]))\n    };\n    let copied = original.clone();\n    assert!(copied == original);\n    drop(original);\n    match copied {\n        Option::None => 100,\n        Option::Some(source) => match source {\n            ir::ReturnSource::Static => 101,\n            ir::ReturnSource::Parameter { index, projections } => {\n                index as i64 + projections_value(&projections)\n            }\n            ir::ReturnSource::Leaves(leaves) => {\n                let mut result: i64 = 0;\n                let mut cursor: usize = 0;\n                while cursor < leaves.len() {\n                    result = result + projections_value(&leaves[cursor].projections);\n                    result = result\n                        + match &leaves[cursor].origin {\n                            ir::LeafOrigin::Static => 103,\n                            ir::LeafOrigin::Inputs { inputs, exact } => {\n                                let mut value = flag(exact);\n                                let mut index: usize = 0;\n                                while index < inputs.len() {\n                                    value = value\n                                        + inputs[index].index as i64\n                                        + projections_value(&inputs[index].projections);\n                                    index = index + 1;\n                                }\n                                value\n                            }\n                        };\n                    cursor = cursor + 1;\n                }\n                result\n            }\n        },\n    }\n}\n\npub fn production_place_local() -> usize {\n    ir::Place::local(42).local\n}\n")),
source_file("src/ir.rs", Some("//! Explicit, monomorphic meaning. Only CheckedProgram reaches a backend.\n#[derive(Clone, PartialEq)]\npub enum TypeKind {\n    Unit,\n    Bool,\n    /// A Unicode scalar value: 0..=0x10ffff excluding 0xd800..=0xdfff.\n    Char,\n    Int {\n        bits: u8,\n        signed: bool,\n    },\n    F64,\n    Struct {\n        fields: Vec<Field>,\n        copy: bool,\n    },\n    Enum {\n        variants: Vec<Variant>,\n        copy: bool,\n    },\n    Ref {\n        inner: usize,\n        mutable: bool,\n    },\n    Slice {\n        inner: usize,\n        mutable: bool,\n    },\n    Array {\n        inner: usize,\n        len: usize,\n    },\n    Vec {\n        inner: usize,\n    },\n    Box {\n        inner: usize,\n    },\n    /// Unsized immutable UTF-8 text; stored values use a shared Ref.\n    Str,\n    World,\n    /// Affine authority to write standard output, derived from World.\n    Stdout,\n    Stdin,\n    Stderr,\n    /// Affine directory authority; descendants remain beneath this root.\n    Directory,\n    /// Affine open file; structural destruction closes its handle.\n    File,\n    Process,\n    Clock,\n    Child,\n}\n\n#[derive(Clone, PartialEq)]\npub struct Field {\n    pub name: String,\n    pub ty: usize,\n}\n#[derive(Clone, PartialEq)]\npub struct Variant {\n    pub name: String,\n    pub fields: Vec<Field>,\n}\n#[derive(Clone, PartialEq)]\npub struct Type {\n    pub name: String,\n    pub kind: TypeKind,\n}\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub enum Projection {\n    Deref,\n    Field(usize),\n    VariantField(usize, usize),\n    Index(usize),\n}\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub struct Place {\n    pub local: usize,\n    pub projections: Vec<Projection>,\n}\nimpl Place {\n    pub fn local(local: usize) -> Place {\n        Place {\n            local,\n            projections: Vec::new(),\n        }\n    }\n}\n#[derive(Clone, PartialEq)]\npub struct Local {\n    pub name: String,\n    pub ty: usize,\n    pub mutable: bool,\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum UnaryOp {\n    Neg,\n    Not,\n}\nimpl UnaryOp {\n    pub fn parse(text: &str) -> Option<UnaryOp> {\n        match text {\n            \"-\" => Some(UnaryOp::Neg),\n            \"!\" => Some(UnaryOp::Not),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            UnaryOp::Neg => \"-\",\n            UnaryOp::Not => \"!\",\n        }\n    }\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum BinaryOp {\n    Add,\n    Sub,\n    Mul,\n    Div,\n    Rem,\n    BitAnd,\n    BitOr,\n    BitXor,\n    Shl,\n    Shr,\n    Eq,\n    Ne,\n    Lt,\n    Le,\n    Gt,\n    Ge,\n    And,\n    Or,\n    WrappingAdd,\n    WrappingSub,\n    WrappingMul,\n}\nimpl BinaryOp {\n    pub fn parse(text: &str) -> Option<BinaryOp> {\n        match text {\n            \"+\" => Some(BinaryOp::Add),\n            \"-\" => Some(BinaryOp::Sub),\n            \"*\" => Some(BinaryOp::Mul),\n            \"/\" => Some(BinaryOp::Div),\n            \"%\" => Some(BinaryOp::Rem),\n            \"&\" => Some(BinaryOp::BitAnd),\n            \"|\" => Some(BinaryOp::BitOr),\n            \"^\" => Some(BinaryOp::BitXor),\n            \"<<\" => Some(BinaryOp::Shl),\n            \">>\" => Some(BinaryOp::Shr),\n            \"==\" => Some(BinaryOp::Eq),\n            \"!=\" => Some(BinaryOp::Ne),\n            \"<\" => Some(BinaryOp::Lt),\n            \"<=\" => Some(BinaryOp::Le),\n            \">\" => Some(BinaryOp::Gt),\n            \">=\" => Some(BinaryOp::Ge),\n            \"&&\" => Some(BinaryOp::And),\n            \"||\" => Some(BinaryOp::Or),\n            \"wrapping_add\" => Some(BinaryOp::WrappingAdd),\n            \"wrapping_sub\" => Some(BinaryOp::WrappingSub),\n            \"wrapping_mul\" => Some(BinaryOp::WrappingMul),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            BinaryOp::Add => \"+\",\n            BinaryOp::Sub => \"-\",\n            BinaryOp::Mul => \"*\",\n            BinaryOp::Div => \"/\",\n            BinaryOp::Rem => \"%\",\n            BinaryOp::BitAnd => \"&\",\n            BinaryOp::BitOr => \"|\",\n            BinaryOp::BitXor => \"^\",\n            BinaryOp::Shl => \"<<\",\n            BinaryOp::Shr => \">>\",\n            BinaryOp::Eq => \"==\",\n            BinaryOp::Ne => \"!=\",\n            BinaryOp::Lt => \"<\",\n            BinaryOp::Le => \"<=\",\n            BinaryOp::Gt => \">\",\n            BinaryOp::Ge => \">=\",\n            BinaryOp::And => \"&&\",\n            BinaryOp::Or => \"||\",\n            BinaryOp::WrappingAdd => \"wrapping_add\",\n            BinaryOp::WrappingSub => \"wrapping_sub\",\n            BinaryOp::WrappingMul => \"wrapping_mul\",\n        }\n    }\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum IntrinsicOp {\n    CharFromU32,\n    F64FromBits,\n    F64ToBits,\n    ArrayRepeat,\n    StrLen,\n    StrAsBytes,\n    StrFromUtf8,\n    Assert,\n    VecNew,\n    VecLen,\n    VecPush,\n    VecPop,\n    Replace,\n    BoxNew,\n    Cast,\n    Clone,\n    WorldStdout,\n    WorldStdin,\n    WorldStderr,\n    WorldArgCount,\n    WorldArgLen,\n    WorldArgCopy,\n    WorldExitStatus,\n    WorldDirectory,\n    DirectoryOpen,\n    StdinRead,\n    FileRead,\n    FileWrite,\n    StdoutWrite,\n    StderrWrite,\n    WorldProcess,\n    WorldClock,\n    WorldFile,\n    ClockRead,\n    ProcessSpawn,\n    ChildWait,\n    ChildTerminate,\n    DirectoryOpenDirectory,\n    DirectoryRead,\n    DirectoryRewind,\n    DirectoryCreate,\n    DirectoryRemove,\n    DirectoryRename,\n    FileRewind,\n    FileExecutable,\n    FileDuplicate,\n    FileReadOnly,\n}\nimpl IntrinsicOp {\n    pub fn parse(text: &str) -> Option<IntrinsicOp> {\n        match text {\n            \"char_from_u32\" => Some(IntrinsicOp::CharFromU32),\n            \"f64_from_bits\" => Some(IntrinsicOp::F64FromBits),\n            \"f64_to_bits\" => Some(IntrinsicOp::F64ToBits),\n            \"array_repeat\" => Some(IntrinsicOp::ArrayRepeat),\n            \"str_len\" => Some(IntrinsicOp::StrLen),\n            \"str_as_bytes\" => Some(IntrinsicOp::StrAsBytes),\n            \"str_from_utf8\" => Some(IntrinsicOp::StrFromUtf8),\n            \"assert\" => Some(IntrinsicOp::Assert),\n            \"vec_new\" => Some(IntrinsicOp::VecNew),\n            \"vec_len\" => Some(IntrinsicOp::VecLen),\n            \"vec_push\" => Some(IntrinsicOp::VecPush),\n            \"vec_pop\" => Some(IntrinsicOp::VecPop),\n            \"replace\" => Some(IntrinsicOp::Replace),\n            \"box_new\" => Some(IntrinsicOp::BoxNew),\n            \"cast\" => Some(IntrinsicOp::Cast),\n            \"clone\" => Some(IntrinsicOp::Clone),\n            \"world_stdout\" => Some(IntrinsicOp::WorldStdout),\n            \"world_stdin\" => Some(IntrinsicOp::WorldStdin),\n            \"world_stderr\" => Some(IntrinsicOp::WorldStderr),\n            \"world_arg_count\" => Some(IntrinsicOp::WorldArgCount),\n            \"world_arg_len\" => Some(IntrinsicOp::WorldArgLen),\n            \"world_arg_copy\" => Some(IntrinsicOp::WorldArgCopy),\n            \"world_exit_status\" => Some(IntrinsicOp::WorldExitStatus),\n            \"world_directory\" => Some(IntrinsicOp::WorldDirectory),\n            \"directory_open\" => Some(IntrinsicOp::DirectoryOpen),\n            \"stdin_read\" => Some(IntrinsicOp::StdinRead),\n            \"file_read\" => Some(IntrinsicOp::FileRead),\n            \"file_write\" => Some(IntrinsicOp::FileWrite),\n            \"stdout_write\" => Some(IntrinsicOp::StdoutWrite),\n            \"stderr_write\" => Some(IntrinsicOp::StderrWrite),\n            \"world_process\" => Some(IntrinsicOp::WorldProcess),\n            \"world_clock\" => Some(IntrinsicOp::WorldClock),\n            \"world_file\" => Some(IntrinsicOp::WorldFile),\n            \"clock_read\" => Some(IntrinsicOp::ClockRead),\n            \"process_spawn\" => Some(IntrinsicOp::ProcessSpawn),\n            \"child_wait\" => Some(IntrinsicOp::ChildWait),\n            \"child_terminate\" => Some(IntrinsicOp::ChildTerminate),\n            \"directory_open_directory\" => Some(IntrinsicOp::DirectoryOpenDirectory),\n            \"directory_read\" => Some(IntrinsicOp::DirectoryRead),\n            \"directory_rewind\" => Some(IntrinsicOp::DirectoryRewind),\n            \"directory_create\" => Some(IntrinsicOp::DirectoryCreate),\n            \"directory_remove\" => Some(IntrinsicOp::DirectoryRemove),\n            \"directory_rename\" => Some(IntrinsicOp::DirectoryRename),\n            \"file_rewind\" => Some(IntrinsicOp::FileRewind),\n            \"file_executable\" => Some(IntrinsicOp::FileExecutable),\n            \"file_duplicate\" => Some(IntrinsicOp::FileDuplicate),\n            \"file_readonly\" => Some(IntrinsicOp::FileReadOnly),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            IntrinsicOp::CharFromU32 => \"char_from_u32\",\n            IntrinsicOp::F64FromBits => \"f64_from_bits\",\n            IntrinsicOp::F64ToBits => \"f64_to_bits\",\n            IntrinsicOp::ArrayRepeat => \"array_repeat\",\n            IntrinsicOp::StrLen => \"str_len\",\n            IntrinsicOp::StrAsBytes => \"str_as_bytes\",\n            IntrinsicOp::StrFromUtf8 => \"str_from_utf8\",\n            IntrinsicOp::Assert => \"assert\",\n            IntrinsicOp::VecNew => \"vec_new\",\n            IntrinsicOp::VecLen => \"vec_len\",\n            IntrinsicOp::VecPush => \"vec_push\",\n            IntrinsicOp::VecPop => \"vec_pop\",\n            IntrinsicOp::Replace => \"replace\",\n            IntrinsicOp::BoxNew => \"box_new\",\n            IntrinsicOp::Cast => \"cast\",\n            IntrinsicOp::Clone => \"clone\",\n            IntrinsicOp::WorldStdout => \"world_stdout\",\n            IntrinsicOp::WorldStdin => \"world_stdin\",\n            IntrinsicOp::WorldStderr => \"world_stderr\",\n            IntrinsicOp::WorldArgCount => \"world_arg_count\",\n            IntrinsicOp::WorldArgLen => \"world_arg_len\",\n            IntrinsicOp::WorldArgCopy => \"world_arg_copy\",\n            IntrinsicOp::WorldExitStatus => \"world_exit_status\",\n            IntrinsicOp::WorldDirectory => \"world_directory\",\n            IntrinsicOp::DirectoryOpen => \"directory_open\",\n            IntrinsicOp::StdinRead => \"stdin_read\",\n            IntrinsicOp::FileRead => \"file_read\",\n            IntrinsicOp::FileWrite => \"file_write\",\n            IntrinsicOp::StdoutWrite => \"stdout_write\",\n            IntrinsicOp::StderrWrite => \"stderr_write\",\n            IntrinsicOp::WorldProcess => \"world_process\",\n            IntrinsicOp::WorldClock => \"world_clock\",\n            IntrinsicOp::WorldFile => \"world_file\",\n            IntrinsicOp::ClockRead => \"clock_read\",\n            IntrinsicOp::ProcessSpawn => \"process_spawn\",\n            IntrinsicOp::ChildWait => \"child_wait\",\n            IntrinsicOp::ChildTerminate => \"child_terminate\",\n            IntrinsicOp::DirectoryOpenDirectory => \"directory_open_directory\",\n            IntrinsicOp::DirectoryRead => \"directory_read\",\n            IntrinsicOp::DirectoryRewind => \"directory_rewind\",\n            IntrinsicOp::DirectoryCreate => \"directory_create\",\n            IntrinsicOp::DirectoryRemove => \"directory_remove\",\n            IntrinsicOp::DirectoryRename => \"directory_rename\",\n            IntrinsicOp::FileRewind => \"file_rewind\",\n            IntrinsicOp::FileExecutable => \"file_executable\",\n            IntrinsicOp::FileDuplicate => \"file_duplicate\",\n            IntrinsicOp::FileReadOnly => \"file_readonly\",\n        }\n    }\n}\n#[derive(Clone, PartialEq)]\npub enum Op {\n    /// Immutable module-long bytes: a UTF-8 str or an exact-length u8 array.\n    Text {\n        dst: usize,\n        bytes: Vec<u8>,\n    },\n    Int {\n        dst: usize,\n        value: i64,\n    },\n    Float {\n        dst: usize,\n        bits: u64,\n    },\n    Unit {\n        dst: usize,\n    },\n    Read {\n        dst: usize,\n        place: Place,\n        moving: bool,\n    },\n    Borrow {\n        dst: usize,\n        place: Place,\n        mutable: bool,\n    },\n    SliceBorrow {\n        dst: usize,\n        place: Place,\n        mutable: bool,\n    },\n    /// Borrow a checked subrange without narrowing its static alias footprint.\n    /// Bounds are consumed usize values; failure traps with the bounds code.\n    ViewRange {\n        dst: usize,\n        place: Place,\n        start: usize,\n        end: usize,\n        inclusive: bool,\n        mutable: bool,\n    },\n    Array {\n        dst: usize,\n        elements: Vec<usize>,\n    },\n    Assign {\n        place: Place,\n        src: usize,\n    },\n    Drop {\n        place: Place,\n    },\n    /// Destroy this place only when its own storage is initialized. Every\n    /// projected ancestor must still exist; this does not authorize a hole\n    /// through a reference or an indexed container.\n    DropIf {\n        place: Place,\n    },\n    Unary {\n        dst: usize,\n        op: UnaryOp,\n        src: usize,\n    },\n    Binary {\n        dst: usize,\n        op: BinaryOp,\n        left: usize,\n        right: usize,\n    },\n    Call {\n        dst: usize,\n        function: usize,\n        args: Vec<usize>,\n    },\n    Struct {\n        dst: usize,\n        fields: Vec<usize>,\n    },\n    Variant {\n        dst: usize,\n        variant: usize,\n        fields: Vec<usize>,\n    },\n    Discriminant {\n        dst: usize,\n        src: usize,\n    },\n    TagTest {\n        dst: usize,\n        place: Place,\n        variant: usize,\n    },\n    Intrinsic {\n        dst: usize,\n        name: IntrinsicOp,\n        args: Vec<usize>,\n    },\n    If {\n        condition: usize,\n    },\n    Else,\n    End,\n    Loop,\n    Break,\n    Continue,\n    /// Select an enclosing lexical loop, counting only loops (zero is nearest).\n    BreakTo {\n        depth: usize,\n    },\n    ContinueTo {\n        depth: usize,\n    },\n    Return {\n        src: usize,\n    },\n    Trap {\n        message: String,\n    },\n}\n#[derive(Clone, PartialEq)]\npub struct Instruction {\n    pub op: Op,\n    pub line: usize,\n    pub column: usize,\n}\n/// An input selector names one stored reference leaf in a parameter value.\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub struct InputLeaf {\n    pub index: usize,\n    pub projections: Vec<Projection>,\n}\n/// Every result reference leaf selects a declared input alternative or static storage.\n#[derive(Clone, PartialEq, Eq)]\npub enum LeafOrigin {\n    Inputs {\n        inputs: Vec<InputLeaf>,\n        /// Exact preserves an input referent; bounded permits a descendant.\n        exact: bool,\n    },\n    Static,\n}\n#[derive(Clone, PartialEq, Eq)]\npub struct ReturnLeaf {\n    pub projections: Vec<Projection>,\n    pub origin: LeafOrigin,\n}\n#[derive(Clone, PartialEq, Eq)]\npub enum ReturnSource {\n    /// A complete, unique contract for every reference leaf in the result.\n    Leaves(Vec<ReturnLeaf>),\n    /// Reference leaves in a parameter value, after the declared dereferences.\n    /// Projected sources currently require shared leaves; exclusive reborrows\n    /// retain the lifetime of their outer reference.\n    Parameter {\n        index: usize,\n        projections: Vec<Projection>,\n    },\n    Static,\n}\n#[derive(Clone, PartialEq)]\npub struct Function {\n    pub file: usize,\n    pub name: String,\n    pub params: usize,\n    pub result: usize,\n    pub return_source: Option<ReturnSource>,\n    pub locals: Vec<Local>,\n    pub body: Vec<Instruction>,\n    pub test: bool,\n}\n#[derive(Clone, PartialEq)]\npub struct Program {\n    pub types: Vec<Type>,\n    pub functions: Vec<Function>,\n    pub destructors: Vec<(usize, usize)>,\n}\npub struct CheckedProgram {\n    pub(crate) program: Program,\n}\nimpl CheckedProgram {\n    pub fn program(&self) -> &Program {\n        &self.program\n    }\n}\npub const UNIT: usize = 0;\npub const BOOL: usize = 1;\npub const I32: usize = 2;\npub const I64: usize = 3;\npub const U64: usize = 4;\npub const U8: usize = 5;\npub const F64: usize = 6;\npub fn primitives() -> Vec<Type> {\n    vec![\n        Type {\n            name: \"()\".to_string(),\n            kind: TypeKind::Unit,\n        },\n        Type {\n            name: \"bool\".to_string(),\n            kind: TypeKind::Bool,\n        },\n        Type {\n            name: \"i32\".to_string(),\n            kind: TypeKind::Int {\n                bits: 32,\n                signed: true,\n            },\n        },\n        Type {\n            name: \"i64\".to_string(),\n            kind: TypeKind::Int {\n                bits: 64,\n                signed: true,\n            },\n        },\n        Type {\n            name: \"usize\".to_string(),\n            kind: TypeKind::Int {\n                bits: 64,\n                signed: false,\n            },\n        },\n        Type {\n            name: \"u8\".to_string(),\n            kind: TypeKind::Int {\n                bits: 8,\n                signed: false,\n            },\n        },\n        Type {\n            name: \"f64\".to_string(),\n            kind: TypeKind::F64,\n        },\n    ]\n}\n")),
], ""), errors);
        count += 1;
    }
    if 183usize % workers == shard {
        record(source_case("enum_aliases_use_the_same_constructor_and_pattern_identity_0000", "main.rs", None, vec![
source_file("main.rs", Some("\nenum Packet<T,U>{Data{payload:T,tag:U},Tuple(T,U),End}\ntype Alias<T> =Packet<Box<T>,u8>;\ntype Chain<X> =Alias<X>;\nfn read(value:Packet<Box<i64>,u8>)->i64{\n    match value{\n        Chain::Data{payload,tag}=>*payload+tag as i64,\n        Chain::Tuple(payload,tag)=>*payload+tag as i64,\n        Chain::End=>42\n    }\n}\npub fn inferred()->i64{read(Chain::Data{tag:2,payload:Box::new(40i64)})}\npub fn explicit()->i64{read(Alias::<i64>::Data{payload:Box::new(40),tag:2})}\npub fn expected()->i64{let value:Alias<i64> =Alias::Data{tag:2,payload:Box::new(40)};read(value)}\npub fn tuple()->i64{read(Alias::Tuple(Box::new(40i64),2))}\npub fn unit()->i64{let value:Alias<i64> =Alias::End;read(value)}\n")),
], ""), errors);
        count += 1;
    }
    if 184usize % workers == shard {
        record(source_case("equality_and_ordering_check_float_and_nested_field_requirements_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(PartialEq,PartialOrd)] struct Reading {value:f64}")),
], ""), errors);
        count += 1;
    }
    if 185usize % workers == shard {
        record(source_case("equality_and_ordering_check_float_and_nested_field_requirements_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(PartialEq,Eq)] struct Reading {value:f64}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 186usize % workers == shard {
        record(source_case("equality_and_ordering_check_float_and_nested_field_requirements_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(PartialEq,PartialOrd,Eq,Ord)] enum Reading {Values(Vec<f64>)}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 187usize % workers == shard {
        record(source_case("equality_matches_the_same_rust_source_0000", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("\nmod misleading {\n    pub trait PartialEq { fn eq(&self, other: &Self) -> bool; }\n}\nstruct Explicit { value: Box<i64> }\nimpl Explicit {\n    fn eq(&self, _other: &Self) -> bool { true }\n}\nimpl misleading::PartialEq for Explicit {\n    fn eq(&self, _other: &Self) -> bool { true }\n}\nimpl PartialEq for Explicit {\n    fn eq(&self, other: &Self) -> bool { *self.value == *other.value }\n    fn ne(&self, _other: &Self) -> bool { false }\n}\npub fn selected_methods() -> i64 {\n    let left = Explicit { value: Box::new(1) };\n    let right = Explicit { value: Box::new(2) };\n    if left == right { return 1; };\n    if left != right { return 2; };\n    if !left.eq(&right) { return 3; };\n    *left.value + *right.value + 39\n}\nstruct Defaulted { value: Box<i64> }\nimpl Defaulted {\n    fn ne(&self, _other: &Self) -> bool { false }\n}\nimpl PartialEq for Defaulted {\n    fn eq(&self, other: &Self) -> bool { *self.value == *other.value }\n}\nimpl Clone for Defaulted {\n    fn clone(&self) -> Self { Defaulted { value: Box::new(*self.value) } }\n    fn clone_from(&mut self, other: &Self) { *self.value = *other.value; }\n}\nfn compare_then_clone<T: PartialEq + Clone>(left: &mut T, right: &T) -> bool {\n    *left == *right && { left.clone_from(right); true }\n}\npub fn generic_comparison_releases_its_loans() -> i64 {\n    let mut left = Defaulted { value: Box::new(42) };\n    let right = Defaulted { value: Box::new(42) };\n    if compare_then_clone(&mut left, &right) { *left.value } else { 0 }\n}\npub fn default_ne() -> i64 {\n    let left = Defaulted { value: Box::new(20) };\n    let right = Defaulted { value: Box::new(22) };\n    if !(left != right) { return 1; };\n    if left.ne(&right) { return 2; };\n    *left.value + *right.value\n}\nfn make(order: &mut i64, value: i64) -> Defaulted {\n    *order = *order * 10 + value;\n    Defaulted { value: Box::new(value) }\n}\npub fn operand_calls_once_in_order() -> i64 {\n    let mut order: i64 = 0;\n    if make(&mut order, 1) == make(&mut order, 2) { return 0; };\n    order\n}\npub fn scalar_left_read_precedes_rhs_effects() -> i64 {\n    let mut value: i64 = 1;\n    let equal = value == { value = 2; 2 };\n    if equal { 0 } else { value }\n}\npub fn loans_end_before_next_boolean_operand() -> i64 {\n    let mut left = Defaulted { value: Box::new(1) };\n    let right = Defaulted { value: Box::new(1) };\n    if left == right && { *left.value = 20; true } {\n        if &left != &right && { *left.value = 42; true } { return *left.value; };\n    }\n    0\n}\nstruct Pair { left: Defaulted, right: Defaulted }\nfn borrowed(left: &Defaulted, right: &Defaulted) -> bool { left == right }\nfn nested_borrowed(left: &&Defaulted, right: &&Defaulted) -> bool { left == right }\nfn exclusive(left: &mut Defaulted, right: &mut Defaulted) -> bool {\n    let equal = left == right;\n    *left.value = 20;\n    *right.value = 22;\n    equal\n}\npub fn projections_dereferences_and_references() -> i64 {\n    let mut pair = Pair {\n        left: Defaulted { value: Box::new(1) },\n        right: Defaulted { value: Box::new(1) }\n    };\n    if !(pair.left == pair.right) { return 1; };\n    if !borrowed(&pair.left, &pair.right) { return 2; };\n    if !nested_borrowed(&&pair.left, &&pair.right) { return 3; };\n    if !exclusive(&mut pair.left, &mut pair.right) { return 4; };\n    *pair.left.value + *pair.right.value\n}\nstruct Tracked<'a> { drops: &'a mut i64, value: i64 }\nimpl<'a> Drop for Tracked<'a> { fn drop(&mut self) { *self.drops = *self.drops + 1; } }\nimpl<'a> PartialEq for Tracked<'a> {\n    fn eq(&self, other: &Self) -> bool { self.value == other.value }\n}\npub fn owners_drop_exactly_once() -> i64 {\n    let mut left_drops: i64 = 0;\n    let mut right_drops: i64 = 0;\n    {\n        let left = Tracked { drops: &mut left_drops, value: 7 };\n        let right = Tracked { drops: &mut right_drops, value: 7 };\n        if !(left == right) { return 0; };\n        if left != right { return 0; };\n    }\n    left_drops * 10 + right_drops\n}\npub fn temporary_owners_drop_exactly_once() -> i64 {\n    let mut left_drops: i64 = 0;\n    let mut right_drops: i64 = 0;\n    let equal = Tracked { drops: &mut left_drops, value: 7 }\n        == Tracked { drops: &mut right_drops, value: 7 };\n    if equal { left_drops * 10 + right_drops } else { 0 }\n}\nstruct Wrapper<T> { value: T }\nimpl<T: PartialEq> PartialEq for Wrapper<T> {\n    fn eq(&self, other: &Self) -> bool { self.value == other.value }\n}\nfn identity<T>(value: T) -> T { value }\nfn equal_results<T: PartialEq>(left: T, right: T) -> bool {\n    identity(left) == identity(right)\n}\npub fn generic_implementation_and_unknown_hints() -> i64 {\n    let left = Wrapper { value: Defaulted { value: Box::new(1) } };\n    let right = Wrapper { value: Defaulted { value: Box::new(1) } };\n    if !(left == right) { return 1; };\n    if equal_results(left, right) { 42 } else { 2 }\n}\nstruct Factory {}\nimpl Factory { fn len(&self) -> Defaulted { Defaulted { value: Box::new(7) } } }\npub fn method_name_does_not_set_the_operand_type() -> i64 {\n    if (Factory {}.len()) == (Factory {}.len()) { 42 } else { 0 }\n}\nenum Packet { Data { value: Box<i64> } }\nimpl PartialEq for Packet {\n    fn eq(&self, other: &Self) -> bool {\n        match self {\n            Packet::Data { value: left } => match other {\n                Packet::Data { value: right } => **left == **right\n            }\n        }\n    }\n}\npub fn enum_equality() -> i64 {\n    let left = Packet::Data { value: Box::new(7) };\n    let right = Packet::Data { value: Box::new(7) };\n    if left != right { return 0; };\n    match left { Packet::Data { value } => *value + 35 }\n}\nmod nominal {\n    pub struct Value { pub number: i64 }\n    impl PartialEq for Value {\n        fn eq(&self, other: &Self) -> bool { self.number == other.number }\n    }\n}\nuse nominal::Value as Imported;\ntype Alias = nominal::Value;\npub fn imported_alias_identity() -> i64 {\n    let left = Imported { number: 7 };\n    let right: Alias = Imported { number: 7 };\n    if left == right { 42 } else { 0 }\n}\npub fn scalar_inference_and_references() -> i64 {\n    let left: i64 = -1;\n    let right: i64 = -1;\n    if 1 == 1u64 && -1i64 == -1 && &left == &right && true != false { 42 } else { 0 }\n}\nfn vector(order: &mut i64, value: i64) -> Vec<i64> {\n    *order = *order * 10 + value;\n    let mut result: Vec<i64> = Vec::new();\n    result.push(value);\n    result\n}\npub fn vector_temporary_operands_are_evaluated_once() -> i64 {\n    let mut order: i64 = 0;\n    if vector(&mut order, 1) == vector(&mut order, 2) { return 0; };\n    order\n}\n")),
], ""), errors);
        count += 1;
    }
    if 188usize % workers == shard {
        record(source_case("equality_methods_match_rust_and_borrow_both_owners_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\nfn same_bytes(value: &String, expected: &str) -> bool {\n    let actual = value.as_bytes();\n    let expected_bytes = expected.as_bytes();\n    if actual.len() != expected_bytes.len() { return false; }\n    let mut index: usize = 0;\n    while index < actual.len() {\n        if actual[index] != expected_bytes[index] { return false; }\n        index = index + 1;\n    }\n    true\n}\npub fn equality_0_0()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_0_1()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_0_2()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_0_3()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_0_4()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_0_5()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_0_6()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_0_7()->usize {\nlet mut a=String::from_str(\"\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_0()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_1()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_2()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_3()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_4()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_5()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_6()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_1_7()->usize {\nlet mut a=String::from_str(\"a\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"a!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_0()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_1()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_2()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_3()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_4()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_5()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_6()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_2_7()->usize {\nlet mut a=String::from_str(\"ab\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"ab!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_0()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_1()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_2()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_3()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_4()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_5()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_6()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_3_7()->usize {\nlet mut a=String::from_str(\"b\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"b!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_0()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_1()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_2()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_3()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_4()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_5()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_6()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_4_7()->usize {\nlet mut a=String::from_str(\"\\0\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_0()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_1()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_2()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_3()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_4()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_5()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_6()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_5_7()->usize {\nlet mut a=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_0()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_1()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_2()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_3()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_4()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_5()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_6()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_6_7()->usize {\nlet mut a=String::from_str(\"α水😀\\0\\u{10ffff}\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"α水😀\\0\\u{10ffff}!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_0()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_1()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"a\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_2()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"ab\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_3()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"b\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_4()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"\\0\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_5()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_6()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"α水😀\\0\\u{10ffff}\");\nif a.eq(&b)!=false {return 1;}\nif a.ne(&b)!=true {return 2;}\nif (a==b)!=false {return 6;}\nif (a!=b)!=true {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\npub fn equality_7_7()->usize {\nlet mut a=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\"); let mut b=String::from_str(\"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀\");\nif a.eq(&b)!=true {return 1;}\nif a.ne(&b)!=false {return 2;}\nif (a==b)!=true {return 6;}\nif (a!=b)!=false {return 7;}\nif !a.eq(&a) {return 3;}\na.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\nif !same_bytes(&a, \"long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀long\\0α水😀!\") { return 4; }\nif !same_bytes(&b, \"reused\\0水\") { return 5; }\n0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 189usize % workers == shard {
        record(source_case("exhaustive_integer_domains_and_boundaries_match_the_same_rust_source_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("\npub fn u8_direct(value: u8) -> i64 {\n    match value { ..32 => 1, 32..=126 => 2, 127.. => 3 }\n}\nfn u8_reference(value: &u8) -> i64 {\n    match value { ..=31 => 1, 32..127 => 2, 127.. => 3 }\n}\npub fn u8_borrowed(value: u8) -> i64 { u8_reference(&value) }\npub fn u8_radices(value: u8) -> i64 {\n    match value { 0x00u8..0o40u8 => 1, 0b0010_0000u8..=0x7eu8 => 2, 0o177u8.. => 3 }\n}\npub fn u8_bytes(value: u8) -> i64 {\n    match value { b'\\0'..=b' ' => 1, b'!'..=b'~' => 2, 0x7f..=0xff => 3 }\n}\npub fn u8_overlap(value: u8) -> i64 {\n    match value { 0..=100 => 1, 50..=200 => 2, 201.. => 3 }\n}\npub fn u8_unordered(value: u8) -> i64 {\n    match value { 200..=255 => 3, ..64 => 1, 64..200 => 2 }\n}\npub fn i8_direct(value: i8) -> i64 {\n    match value { ..=-65i8 => 1, -64i8..0 => 2, 0 => 3, 1..=63 => 4, 64.. => 5 }\n}\nfn i64_reference(value: &i64) -> i64 {\n    match value {\n        -9223372036854775808i64..=-1i64 => 11,\n        0i64..9223372036854775807i64 => 12,\n        9223372036854775807i64 => 13\n    }\n}\npub fn i64_borrowed(value: i64) -> i64 { i64_reference(&value) }\nfn u64_reference(value: &u64) -> i64 {\n    match value {\n        0u64..9223372036854775808u64 => 21,\n        9223372036854775808u64..18446744073709551615u64 => 22,\n        18446744073709551615u64 => 23\n    }\n}\npub fn u64_borrowed(value: u64) -> i64 { u64_reference(&value) }\npub fn signed_literals(value: i64) -> i64 {\n    match value { -9223372036854775808i64 => 31, -1i64 => 32, _ => 33 }\n}\npub fn max_suffix(value: u64) -> i64 {\n    match value { 0xffff_ffff_ffff_ffffu64.. => 41, ..0xffff_ffff_ffff_ffffu64 => 42 }\n}\npub fn negative_singleton(value: i8) -> i64 {\n    match value { -128i8..=-128i8 => 51, -127i8.. => 52 }\n}\npub fn wildcard_first(value: i64) -> i64 { match value { _ => 1, -1 => 2 } }\nstruct Counted<'a> { count: &'a mut i64, payload: Box<i64> }\nimpl<'a> Drop for Counted<'a> {\n    fn drop(&mut self) { *self.count = *self.count + 1; }\n}\nenum Choice<'a> { Empty, Value(Counted<'a>) }\nfn select_owned(value: Choice<'_>) -> i64 {\n    match value {\n        bound => { drop(bound); 42 },\n        Choice::Value(other) => { drop(other); 1000 },\n        Choice::Empty => 0\n    }\n}\npub fn default_owned(value: i64) -> i64 {\n    let mut count: i64 = 0;\n    let selected = select_owned(Choice::Value(Counted { count: &mut count, payload: Box::new(value) }));\n    assert_eq!(count, 1);\n    selected + value\n}\npub fn inferred(value: i64) -> i64 {\n    let first = match 255 { 0u8..255 => 1i64, 255 => 2 };\n    let second = match -128 { -128i8..=-1 => 3i64, 0.. => 4 };\n    first + second + value\n}\npub fn inferred_borrowed(value: i64) -> i64 {\n    let first = match &7 { 0u8..=9u8 => 1i64, _ => 0 };\n    first + value\n}\nfn mutable_reference(value: &mut u8) -> i64 {\n    let selected = match value { ..=31 => 1i64, 32.. => 2 };\n    selected + *value as i64\n}\npub fn mutable_byte(value: u8) -> i64 {\n    let mut local = value;\n    mutable_reference(&mut local)\n}\n")),
], ""), errors);
        count += 1;
    }
    if 190usize % workers == shard {
        record(source_case("explicit_arguments_do_not_substitute_a_qualified_nominal_type_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("struct T(i64); fn accept<T>(named:crate::T,value:T)->i64{5} pub fn probe()->i64{accept::<i64>(1i64,2i64)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 191usize % workers == shard {
        record(source_case("explicit_builtin_clone_evidence_has_its_actual_method_contract_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Clone for Item{} #[derive(Clone)] struct Holder(Item);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 192usize % workers == shard {
        record(source_case("explicit_builtin_clone_evidence_has_its_actual_method_contract_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Clone for Item{fn clone(&self)->i64{0}} #[derive(Clone)] struct Holder(Item);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 193usize % workers == shard {
        record(source_case("explicit_builtin_clone_evidence_has_its_actual_method_contract_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Clone for Item{fn clone(&mut self)->Self{Item{value:0}}} #[derive(Clone)] struct Holder(Item);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 194usize % workers == shard {
        record(source_case("explicit_builtin_clone_evidence_has_its_actual_method_contract_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Clone for Item{fn clone<T>(&self)->Self{Item{value:0}}} #[derive(Clone)] struct Holder(Item);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 195usize % workers == shard {
        record(source_case("explicit_builtin_clone_evidence_has_its_actual_method_contract_0004", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Clone for Item{fn other(&self)->Self{Item{value:0}}} #[derive(Clone)] struct Holder(Item);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 196usize % workers == shard {
        record(source_case("explicit_builtin_clone_evidence_has_its_actual_method_contract_0005", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Clone for Item{fn clone(&self)->Self{Item{value:self.value}} fn clone_from(&mut self,source:&Self){self.value=source.value;}} #[derive(Clone)] struct Holder(Item);")),
], ""), errors);
        count += 1;
    }
    if 197usize % workers == shard {
        record(source_case("explicit_builtin_equality_evidence_has_its_actual_method_contract_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Eq for Item{}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 198usize % workers == shard {
        record(source_case("explicit_builtin_equality_evidence_has_its_actual_method_contract_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl PartialEq for Item{fn eq(&self,other:&Self)->i64{0}}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 199usize % workers == shard {
        record(source_case("explicit_builtin_equality_evidence_has_its_actual_method_contract_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl PartialEq for Item{fn eq(&self,other:Self)->bool{true}}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 200usize % workers == shard {
        record(source_case("explicit_builtin_equality_evidence_has_its_actual_method_contract_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl PartialEq for Item{fn eq(&self,other:&Self)->bool{self.value==other.value}} impl Eq for Item{} #[derive(PartialEq,Eq)] struct Holder(Item);")),
], ""), errors);
        count += 1;
    }
    if 201usize % workers == shard {
        record(source_case("explicit_builtin_implementations_do_not_override_structural_proofs_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("impl Clone for i64{fn clone(&self)->Self{*self}}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 202usize % workers == shard {
        record(source_case("explicit_builtin_implementations_do_not_override_structural_proofs_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("impl PartialEq for Vec<i64>{fn eq(&self,other:&Self)->bool{true}}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 203usize % workers == shard {
        record(source_case("explicit_builtin_implementations_do_not_override_structural_proofs_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("type Values=Vec<i64>; impl Eq for Values{}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 204usize % workers == shard {
        record(source_case("explicit_builtin_implementations_do_not_override_structural_proofs_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} type Alias=Item; impl Clone for Alias{fn clone(&self)->Self{Item{value:self.value}}}")),
], ""), errors);
        count += 1;
    }
    if 205usize % workers == shard {
        record(source_case("explicit_clone_methods_are_selected_by_declaration_0001", "case.rs", None, vec![source_file("case.rs", Some("struct Thing(i64); impl Thing{fn clone(&self)->i64{42}} fn probe()->i64{Thing(1).clone()}"))], ""), errors);
        count += 1;
    }
    if 206usize % workers == shard {
        record(source_case("explicit_clone_methods_are_selected_by_declaration_0004", "case.rs", None, vec![source_file("case.rs", Some("#[derive(Copy,Clone)] struct Thing(i64); impl Thing{fn clone(&self)->Thing{Thing(42)}} fn probe()->i64{Thing(1).clone().0}"))], ""), errors);
        count += 1;
    }
    if 207usize % workers == shard {
        record(source_case("explicit_clone_methods_are_selected_by_declaration_0007", "case.rs", None, vec![source_file("case.rs", Some("mod custom{pub trait Clone{fn clone(&self)->i64;}} fn read<T:custom::Clone>(x:&T)->i64{x.clone()}"))], ""), errors);
        count += 1;
    }
    if 208usize % workers == shard {
        record(source_case("explicit_generic_method_arguments_are_honored_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Mapper{fn take<U>(&self,x:U)->U;} fn bad<T:Mapper>(x:&T)->i64{x.take::<bool>(1i64)}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 209usize % workers == shard {
        record(source_case("explicit_generic_method_arguments_are_honored_0004", "case.rs", None, vec![source_file("case.rs", Some("trait Mapper{fn take<U>(&self,x:U)->U;} struct M{} impl Mapper for M{fn take<U>(&self,x:U)->U{x}} fn probe()->i64{M{}.take::<bool>(1i64)}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 210usize % workers == shard {
        record(source_case("explicit_generic_method_arguments_are_honored_0005", "case.rs", None, vec![source_file("case.rs", Some("trait Mapper{fn take<U>(&self,x:U)->U;} struct M{} impl Mapper for M{fn take<U>(&self,x:U)->U{x}} fn probe()->i64{M{}.take::<i64>(42)}"))], ""), errors);
        count += 1;
    }
    if 211usize % workers == shard {
        record(source_case("explicit_mutable_field_bindings_preserve_original_place_mutability_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{value:i64}} fn bad(){let value=E::Named{value:1};match value{E::Named{ref mut value}=>{*value=2;}}}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 212usize % workers == shard {
        record(source_case("field_evaluation_order_and_method_candidate_trials_are_preserved_0000", "main.rs", None, vec![
source_file("main.rs", Some("\nstruct Pair<A,B>{first:A,second:B}\ntype Alias=Pair<i64,i64>;\nfn next(counter:&mut i64)->i64{*counter=*counter+1;*counter}\nimpl<T> Pair<T,T>{fn score(&self)->i64{1}}\nimpl Pair<i64,bool>{fn score(&self)->i64{42}}\npub fn order()->i64{let mut counter:i64=0;let value=Alias{second:next(&mut counter),first:next(&mut counter)};value.first*10+value.second+21}\npub fn disjoint_candidate()->i64{Pair{first:0i64,second:false}.score()}\nstruct Tuple(i64);\nuse Tuple as Imported;\npub fn value_import()->i64{Imported(42).0}\n")),
], ""), errors);
        count += 1;
    }
    if 213usize % workers == shard {
        record(source_case("file_readonly_reopens_with_independent_offset_on_native_and_witness_0000", "file-readonly", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("file-readonly", Some("// Expected output: readonly ok\\n\npub fn main(mut world: World) {\n    let mut original = world.temporary_file().unwrap();\n    original.write_all(b\"abc\").unwrap();\n    original.rewind().unwrap();\n    let mut byte = [0u8; 1];\n    assert_eq!(original.read(&mut byte).unwrap(), 1);\n    assert_eq!(byte[0], b'a');\n    let mut readonly = original.readonly().unwrap();\n    assert_eq!(readonly.read(&mut byte).unwrap(), 1);\n    assert_eq!(byte[0], b'a');\n    assert_eq!(original.read(&mut byte).unwrap(), 1);\n    assert_eq!(byte[0], b'b');\n    assert_eq!(readonly.__lkjscript_write(b\"changed\"), -9);\n    let mut second = readonly.__lkjscript_readonly().unwrap();\n    assert_eq!(second.read(&mut byte).unwrap(), 1);\n    assert_eq!(byte[0], b'a');\n    drop(original);\n    drop(readonly);\n    assert_eq!(second.read(&mut byte).unwrap(), 1);\n    assert_eq!(byte[0], b'b');\n    world.stdout().write(b\"readonly ok\\n\");\n}\n")),
], ""), errors);
        count += 1;
    }
    if 214usize % workers == shard {
        record(source_case("generated_clones_preserve_referents_without_retaining_local_carriers_0000", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("\nfn text(value:&str)->String {\n    let mut result=String::new();\n    result.push_str(value);\n    result\n}\nfn cloned<T:Clone>(value:&T)->T { value.clone() }\n#[derive(Clone,PartialEq,Eq)]\nstruct View<'a> { value:&'a i64, text:String }\n#[derive(Clone,PartialEq,Eq)]\nstruct Pair<'a> { left:&'a i64, right:&'a i64, text:String }\nstruct Numbers { left:i64, right:i64 }\n#[derive(Clone,PartialEq,Eq)]\nenum Maybe<'a> {\n    Empty,\n    Full { left:&'a i64, right:&'a i64, text:String },\n}\nfn direct<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"original\")};\n    carrier.clone()\n}\nfn generic<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"generic\")};\n    cloned(&carrier)\n}\nfn paired<'a>(values:&'a Numbers)->Pair<'a> {\n    let carrier=Pair{left:&values.left,right:&values.right,text:text(\"pair\")};\n    cloned(&carrier)\n}\nfn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {\n    if present {\n        let carrier=Maybe::Full{left:&values.left,right:&values.right,text:text(\"enum\")};\n        return cloned(&carrier);\n    }\n    let carrier=Maybe::Empty;\n    cloned(&carrier)\n}\npub fn cloned_views_outlive_carriers_and_keep_owned_siblings()->i64 {\n    let number:i64=42;\n    let mut first=direct(&number);\n    let second=generic(&number);\n    if *first.value!=42 || *second.value!=42 {return 1;}\n    let independent=first.clone();\n    let equal_number:i64=42;\n    let equal_view=direct(&equal_number);\n    let other_number:i64=43;\n    let other_view=direct(&other_number);\n    if first!=independent || first!=equal_view || first==other_view {return 4;}\n    if !first.eq(&equal_view) || first.ne(&independent) {return 5;}\n    first.text.clear();\n    first.text.push_str(\"changed\");\n    drop(first);\n    if independent.text.len()!=8 || *independent.value!=42 {return 2;}\n    drop(independent);\n    if second.text.len()!=7 || *second.value!=42 {return 3;}\n    0\n}\npub fn cloned_multiple_leaves_keep_the_common_source()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let value=paired(&numbers);\n    if *value.left*10+*value.right!=42 || value.text.len()!=4 {return 1;}\n    let copied=value.clone();\n    drop(value);\n    if *copied.left!=4 || *copied.right!=2 {return 2;}\n    0\n}\npub fn cloned_enum_variants_keep_present_and_absent_leaves()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let empty=optional(&numbers,false);\n    let copied_empty=empty.clone();\n    drop(empty);\n    match copied_empty {Maybe::Empty=>{},Maybe::Full{..}=>return 1}\n    let full=optional(&numbers,true);\n    let copied_full=cloned(&full);\n    drop(full);\n    match copied_full {\n        Maybe::Full{left,right,text}=>{\n            if *left*10+*right!=42 || text.len()!=4 {return 2;}\n        },\n        Maybe::Empty=>return 3,\n    }\n    0\n}\npub fn clone_retains_exclusive_ancestors_until_the_last_view_dies()->i64 {\n    let mut number:i64=42;\n    let exclusive=&mut number;\n    let carrier=View{value:&*exclusive,text:text(\"borrowed\")};\n    let copied=cloned(&carrier);\n    drop(carrier);\n    if *copied.value!=42 {return 1;}\n    drop(copied);\n    *exclusive=43;\n    *exclusive-43\n}\n")),
], ""), errors);
        count += 1;
    }
    if 215usize % workers == shard {
        record(derived_source_case("generated_cloning_cannot_hide_local_reference_escape_0000", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("\nfn text(value:&str)->String {\n    let mut result=String::new();\n    result.push_str(value);\n    result\n}\nfn cloned<T:Clone>(value:&T)->T { value.clone() }\n#[derive(Clone,PartialEq,Eq)]\nstruct View<'a> { value:&'a i64, text:String }\n#[derive(Clone,PartialEq,Eq)]\nstruct Pair<'a> { left:&'a i64, right:&'a i64, text:String }\nstruct Numbers { left:i64, right:i64 }\n#[derive(Clone,PartialEq,Eq)]\nenum Maybe<'a> {\n    Empty,\n    Full { left:&'a i64, right:&'a i64, text:String },\n}\nfn direct<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"original\")};\n    carrier.clone()\n}\nfn generic<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"generic\")};\n    cloned(&carrier)\n}\nfn paired<'a>(values:&'a Numbers)->Pair<'a> {\n    let carrier=Pair{left:&values.left,right:&values.right,text:text(\"pair\")};\n    cloned(&carrier)\n}\nfn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {\n    if present {\n        let carrier=Maybe::Full{left:&values.left,right:&values.right,text:text(\"enum\")};\n        return cloned(&carrier);\n    }\n    let carrier=Maybe::Empty;\n    cloned(&carrier)\n}\npub fn cloned_views_outlive_carriers_and_keep_owned_siblings()->i64 {\n    let number:i64=42;\n    let mut first=direct(&number);\n    let second=generic(&number);\n    if *first.value!=42 || *second.value!=42 {return 1;}\n    let independent=first.clone();\n    let equal_number:i64=42;\n    let equal_view=direct(&equal_number);\n    let other_number:i64=43;\n    let other_view=direct(&other_number);\n    if first!=independent || first!=equal_view || first==other_view {return 4;}\n    if !first.eq(&equal_view) || first.ne(&independent) {return 5;}\n    first.text.clear();\n    first.text.push_str(\"changed\");\n    drop(first);\n    if independent.text.len()!=8 || *independent.value!=42 {return 2;}\n    drop(independent);\n    if second.text.len()!=7 || *second.value!=42 {return 3;}\n    0\n}\npub fn cloned_multiple_leaves_keep_the_common_source()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let value=paired(&numbers);\n    if *value.left*10+*value.right!=42 || value.text.len()!=4 {return 1;}\n    let copied=value.clone();\n    drop(value);\n    if *copied.left!=4 || *copied.right!=2 {return 2;}\n    0\n}\npub fn cloned_enum_variants_keep_present_and_absent_leaves()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let empty=optional(&numbers,false);\n    let copied_empty=empty.clone();\n    drop(empty);\n    match copied_empty {Maybe::Empty=>{},Maybe::Full{..}=>return 1}\n    let full=optional(&numbers,true);\n    let copied_full=cloned(&full);\n    drop(full);\n    match copied_full {\n        Maybe::Full{left,right,text}=>{\n            if *left*10+*right!=42 || text.len()!=4 {return 2;}\n        },\n        Maybe::Empty=>return 3,\n    }\n    0\n}\npub fn clone_retains_exclusive_ancestors_until_the_last_view_dies()->i64 {\n    let mut number:i64=42;\n    let exclusive=&mut number;\n    let carrier=View{value:&*exclusive,text:text(\"borrowed\")};\n    let copied=cloned(&carrier);\n    drop(carrier);\n    if *copied.value!=42 {return 1;}\n    drop(copied);\n    *exclusive=43;\n    *exclusive-43\n}\n\nfn leak<'a>(external:&'a i64)->View<'a>{let local:i64=7;let carrier=View{value:&local,text:text(\"local\")};carrier.clone()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 216usize % workers == shard {
        record(derived_source_case("generated_cloning_cannot_hide_local_reference_escape_0001", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("\nfn text(value:&str)->String {\n    let mut result=String::new();\n    result.push_str(value);\n    result\n}\nfn cloned<T:Clone>(value:&T)->T { value.clone() }\n#[derive(Clone,PartialEq,Eq)]\nstruct View<'a> { value:&'a i64, text:String }\n#[derive(Clone,PartialEq,Eq)]\nstruct Pair<'a> { left:&'a i64, right:&'a i64, text:String }\nstruct Numbers { left:i64, right:i64 }\n#[derive(Clone,PartialEq,Eq)]\nenum Maybe<'a> {\n    Empty,\n    Full { left:&'a i64, right:&'a i64, text:String },\n}\nfn direct<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"original\")};\n    carrier.clone()\n}\nfn generic<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"generic\")};\n    cloned(&carrier)\n}\nfn paired<'a>(values:&'a Numbers)->Pair<'a> {\n    let carrier=Pair{left:&values.left,right:&values.right,text:text(\"pair\")};\n    cloned(&carrier)\n}\nfn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {\n    if present {\n        let carrier=Maybe::Full{left:&values.left,right:&values.right,text:text(\"enum\")};\n        return cloned(&carrier);\n    }\n    let carrier=Maybe::Empty;\n    cloned(&carrier)\n}\npub fn cloned_views_outlive_carriers_and_keep_owned_siblings()->i64 {\n    let number:i64=42;\n    let mut first=direct(&number);\n    let second=generic(&number);\n    if *first.value!=42 || *second.value!=42 {return 1;}\n    let independent=first.clone();\n    let equal_number:i64=42;\n    let equal_view=direct(&equal_number);\n    let other_number:i64=43;\n    let other_view=direct(&other_number);\n    if first!=independent || first!=equal_view || first==other_view {return 4;}\n    if !first.eq(&equal_view) || first.ne(&independent) {return 5;}\n    first.text.clear();\n    first.text.push_str(\"changed\");\n    drop(first);\n    if independent.text.len()!=8 || *independent.value!=42 {return 2;}\n    drop(independent);\n    if second.text.len()!=7 || *second.value!=42 {return 3;}\n    0\n}\npub fn cloned_multiple_leaves_keep_the_common_source()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let value=paired(&numbers);\n    if *value.left*10+*value.right!=42 || value.text.len()!=4 {return 1;}\n    let copied=value.clone();\n    drop(value);\n    if *copied.left!=4 || *copied.right!=2 {return 2;}\n    0\n}\npub fn cloned_enum_variants_keep_present_and_absent_leaves()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let empty=optional(&numbers,false);\n    let copied_empty=empty.clone();\n    drop(empty);\n    match copied_empty {Maybe::Empty=>{},Maybe::Full{..}=>return 1}\n    let full=optional(&numbers,true);\n    let copied_full=cloned(&full);\n    drop(full);\n    match copied_full {\n        Maybe::Full{left,right,text}=>{\n            if *left*10+*right!=42 || text.len()!=4 {return 2;}\n        },\n        Maybe::Empty=>return 3,\n    }\n    0\n}\npub fn clone_retains_exclusive_ancestors_until_the_last_view_dies()->i64 {\n    let mut number:i64=42;\n    let exclusive=&mut number;\n    let carrier=View{value:&*exclusive,text:text(\"borrowed\")};\n    let copied=cloned(&carrier);\n    drop(carrier);\n    if *copied.value!=42 {return 1;}\n    drop(copied);\n    *exclusive=43;\n    *exclusive-43\n}\n\nfn leak<'a>(external:&'a i64)->Maybe<'a>{let local:i64=7;let carrier=Maybe::Full{left:&local,right:&local,text:text(\"local\")};cloned(&carrier)}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 217usize % workers == shard {
        record(derived_source_case("generated_cloning_cannot_hide_local_reference_escape_0002", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("\nfn text(value:&str)->String {\n    let mut result=String::new();\n    result.push_str(value);\n    result\n}\nfn cloned<T:Clone>(value:&T)->T { value.clone() }\n#[derive(Clone,PartialEq,Eq)]\nstruct View<'a> { value:&'a i64, text:String }\n#[derive(Clone,PartialEq,Eq)]\nstruct Pair<'a> { left:&'a i64, right:&'a i64, text:String }\nstruct Numbers { left:i64, right:i64 }\n#[derive(Clone,PartialEq,Eq)]\nenum Maybe<'a> {\n    Empty,\n    Full { left:&'a i64, right:&'a i64, text:String },\n}\nfn direct<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"original\")};\n    carrier.clone()\n}\nfn generic<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"generic\")};\n    cloned(&carrier)\n}\nfn paired<'a>(values:&'a Numbers)->Pair<'a> {\n    let carrier=Pair{left:&values.left,right:&values.right,text:text(\"pair\")};\n    cloned(&carrier)\n}\nfn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {\n    if present {\n        let carrier=Maybe::Full{left:&values.left,right:&values.right,text:text(\"enum\")};\n        return cloned(&carrier);\n    }\n    let carrier=Maybe::Empty;\n    cloned(&carrier)\n}\npub fn cloned_views_outlive_carriers_and_keep_owned_siblings()->i64 {\n    let number:i64=42;\n    let mut first=direct(&number);\n    let second=generic(&number);\n    if *first.value!=42 || *second.value!=42 {return 1;}\n    let independent=first.clone();\n    let equal_number:i64=42;\n    let equal_view=direct(&equal_number);\n    let other_number:i64=43;\n    let other_view=direct(&other_number);\n    if first!=independent || first!=equal_view || first==other_view {return 4;}\n    if !first.eq(&equal_view) || first.ne(&independent) {return 5;}\n    first.text.clear();\n    first.text.push_str(\"changed\");\n    drop(first);\n    if independent.text.len()!=8 || *independent.value!=42 {return 2;}\n    drop(independent);\n    if second.text.len()!=7 || *second.value!=42 {return 3;}\n    0\n}\npub fn cloned_multiple_leaves_keep_the_common_source()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let value=paired(&numbers);\n    if *value.left*10+*value.right!=42 || value.text.len()!=4 {return 1;}\n    let copied=value.clone();\n    drop(value);\n    if *copied.left!=4 || *copied.right!=2 {return 2;}\n    0\n}\npub fn cloned_enum_variants_keep_present_and_absent_leaves()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let empty=optional(&numbers,false);\n    let copied_empty=empty.clone();\n    drop(empty);\n    match copied_empty {Maybe::Empty=>{},Maybe::Full{..}=>return 1}\n    let full=optional(&numbers,true);\n    let copied_full=cloned(&full);\n    drop(full);\n    match copied_full {\n        Maybe::Full{left,right,text}=>{\n            if *left*10+*right!=42 || text.len()!=4 {return 2;}\n        },\n        Maybe::Empty=>return 3,\n    }\n    0\n}\npub fn clone_retains_exclusive_ancestors_until_the_last_view_dies()->i64 {\n    let mut number:i64=42;\n    let exclusive=&mut number;\n    let carrier=View{value:&*exclusive,text:text(\"borrowed\")};\n    let copied=cloned(&carrier);\n    drop(carrier);\n    if *copied.value!=42 {return 1;}\n    drop(copied);\n    *exclusive=43;\n    *exclusive-43\n}\n\n#[derive(Clone)] struct TextView<'a>{value:&'a str} fn leak<'a>(external:&'a str)->TextView<'a>{let local=text(\"local\");let carrier=TextView{value:local.as_str()};carrier.clone()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 218usize % workers == shard {
        record(derived_source_case("generated_cloning_preserves_exclusive_parent_and_trait_requirements_0000", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("\nfn text(value:&str)->String {\n    let mut result=String::new();\n    result.push_str(value);\n    result\n}\nfn cloned<T:Clone>(value:&T)->T { value.clone() }\n#[derive(Clone,PartialEq,Eq)]\nstruct View<'a> { value:&'a i64, text:String }\n#[derive(Clone,PartialEq,Eq)]\nstruct Pair<'a> { left:&'a i64, right:&'a i64, text:String }\nstruct Numbers { left:i64, right:i64 }\n#[derive(Clone,PartialEq,Eq)]\nenum Maybe<'a> {\n    Empty,\n    Full { left:&'a i64, right:&'a i64, text:String },\n}\nfn direct<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"original\")};\n    carrier.clone()\n}\nfn generic<'a>(external:&'a i64)->View<'a> {\n    let carrier=View{value:external,text:text(\"generic\")};\n    cloned(&carrier)\n}\nfn paired<'a>(values:&'a Numbers)->Pair<'a> {\n    let carrier=Pair{left:&values.left,right:&values.right,text:text(\"pair\")};\n    cloned(&carrier)\n}\nfn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {\n    if present {\n        let carrier=Maybe::Full{left:&values.left,right:&values.right,text:text(\"enum\")};\n        return cloned(&carrier);\n    }\n    let carrier=Maybe::Empty;\n    cloned(&carrier)\n}\npub fn cloned_views_outlive_carriers_and_keep_owned_siblings()->i64 {\n    let number:i64=42;\n    let mut first=direct(&number);\n    let second=generic(&number);\n    if *first.value!=42 || *second.value!=42 {return 1;}\n    let independent=first.clone();\n    let equal_number:i64=42;\n    let equal_view=direct(&equal_number);\n    let other_number:i64=43;\n    let other_view=direct(&other_number);\n    if first!=independent || first!=equal_view || first==other_view {return 4;}\n    if !first.eq(&equal_view) || first.ne(&independent) {return 5;}\n    first.text.clear();\n    first.text.push_str(\"changed\");\n    drop(first);\n    if independent.text.len()!=8 || *independent.value!=42 {return 2;}\n    drop(independent);\n    if second.text.len()!=7 || *second.value!=42 {return 3;}\n    0\n}\npub fn cloned_multiple_leaves_keep_the_common_source()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let value=paired(&numbers);\n    if *value.left*10+*value.right!=42 || value.text.len()!=4 {return 1;}\n    let copied=value.clone();\n    drop(value);\n    if *copied.left!=4 || *copied.right!=2 {return 2;}\n    0\n}\npub fn cloned_enum_variants_keep_present_and_absent_leaves()->i64 {\n    let numbers=Numbers{left:4,right:2};\n    let empty=optional(&numbers,false);\n    let copied_empty=empty.clone();\n    drop(empty);\n    match copied_empty {Maybe::Empty=>{},Maybe::Full{..}=>return 1}\n    let full=optional(&numbers,true);\n    let copied_full=cloned(&full);\n    drop(full);\n    match copied_full {\n        Maybe::Full{left,right,text}=>{\n            if *left*10+*right!=42 || text.len()!=4 {return 2;}\n        },\n        Maybe::Empty=>return 3,\n    }\n    0\n}\npub fn clone_retains_exclusive_ancestors_until_the_last_view_dies()->i64 {\n    let mut number:i64=42;\n    let exclusive=&mut number;\n    let carrier=View{value:&*exclusive,text:text(\"borrowed\")};\n    let copied=cloned(&carrier);\n    drop(carrier);\n    if *copied.value!=42 {return 1;}\n    drop(copied);\n    *exclusive=43;\n    *exclusive-43\n}\n\nfn invalid()->i64{let mut number:i64=7;let exclusive=&mut number;let carrier=View{value:&*exclusive,text:text(\"borrowed\")};let copied=cloned(&carrier);drop(carrier);*exclusive=9;*copied.value}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 219usize % workers == shard {
        record(derived_source_case("generated_cloning_preserves_exclusive_parent_and_trait_requirements_0001", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("#[derive(Clone)] struct Exclusive<'a>{value:&'a mut i64}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 220usize % workers == shard {
        record(derived_source_case("generated_cloning_preserves_exclusive_parent_and_trait_requirements_0002", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("struct Wrong<'a>{external:&'a i64,own:i64} #[derive(Clone)] struct Wrapped<'a>{value:Wrong<'a>} impl<'a> Clone for Wrong<'a>{fn clone(&self)->Self{Wrong{external:&self.own,own:self.own}}}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 221usize % workers == shard {
        record(source_case("generated_equality_preserves_owner_lifetimes_and_cleanup_0000", "derived_provenance.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_provenance.rs", Some("\nstruct Token { phase:i64 }\nimpl Drop for Token {\n    fn drop(&mut self) {\n        assert!(self.phase==2);\n        self.phase=3;\n    }\n}\nstruct Owner { value:i64, token:Box<Token> }\nimpl Clone for Owner {\n    fn clone(&self)->Self {\n        assert!(self.token.phase==1);\n        Owner{value:self.value,token:Box::new(Token{phase:1})}\n    }\n}\nimpl PartialEq for Owner {\n    fn eq(&self,other:&Self)->bool {\n        assert!(self.token.phase==1);\n        assert!(other.token.phase==1);\n        self.value==other.value\n    }\n}\nimpl Eq for Owner {}\nimpl Drop for Owner {\n    fn drop(&mut self) {\n        assert!(self.token.phase==1);\n        self.token.phase=2;\n    }\n}\nfn owner(value:i64)->Owner {Owner{value,token:Box::new(Token{phase:1})}}\n#[derive(Clone,PartialEq,Eq)]\nstruct Record { first:Owner, second:Owner }\n#[derive(Clone,PartialEq,Eq)]\nenum Packet { Empty, Record{value:Record}, Number(i64) }\nfn same<T:PartialEq>(left:&T,right:&T)->bool {left.eq(right)}\npub fn equality_borrows_owned_fields_and_preserves_drop_order()->i64 {\n    let mut left=Record{first:owner(4),second:owner(2)};\n    let mut right=left.clone();\n    if !(left==right) || left!=right {return 1;}\n    if !left.eq(&right) || left.ne(&right) || !same(&left,&right) {return 2;}\n    right.second.value=3;\n    if left==right || !(left!=right) {return 3;}\n    left.first.value=8;\n    if left.first.token.phase!=1 || right.second.token.phase!=1 {return 4;}\n    drop(left);\n    if right.first.value!=4 || right.first.token.phase!=1 {return 5;}\n    drop(right);\n    0\n}\npub fn enum_equality_preserves_payload_owners_across_mismatched_tags()->i64 {\n    let first=Packet::Record{value:Record{first:owner(4),second:owner(2)}};\n    let copied=first.clone();\n    let empty=Packet::Empty;\n    let scalar=Packet::Number(42);\n    if first!=copied || !same(&first,&copied) {return 1;}\n    if first==empty || first==scalar || !first.ne(&empty) {return 2;}\n    drop(first);\n    match copied {\n        Packet::Record{value}=>{\n            if value.first.token.phase!=1 || value.second.token.phase!=1 {return 3;}\n            if value.first.value*10+value.second.value!=42 {return 4;}\n            drop(value);\n        },\n        _=>return 5,\n    }\n    0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 222usize % workers == shard {
        record(source_case("generic_clone_and_equality_use_declared_builtin_witnesses_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\nfn same_bytes(value: &String, expected: &str) -> bool {\n    let actual = value.as_bytes();\n    let expected_bytes = expected.as_bytes();\n    if actual.len() != expected_bytes.len() { return false; }\n    let mut index: usize = 0;\n    while index < actual.len() {\n        if actual[index] != expected_bytes[index] { return false; }\n        index = index + 1;\n    }\n    true\n}\n\n\nfn copied<T: Clone>(value: &T) -> T { value.clone() }\nfn copied_from<T: Clone>(target: &mut T, source: &T) { target.clone_from(source); }\nfn same<T: PartialEq>(left: &T, right: &T) -> bool { left.eq(right) }\nfn different<T: PartialEq>(left: &T, right: &T) -> bool { left.ne(right) }\nfn reflexive<T: Eq>(value: &T) -> bool { value.eq(value) }\nfn copied_scalar<T: Copy>(value: &T) -> T { value.clone() }\nfn same_operator<T: PartialEq>(left: &T, right: &T) -> bool { *left == *right }\nfn different_operator<T: PartialEq>(left: &T, right: &T) -> bool { *left != *right }\npub fn generic_probe() -> usize {\n    let original=String::from_str(\"generic\\0水😀\");\n    let mut copy=copied(&original);\n    if !same_bytes(&copy,\"generic\\0水😀\") {return 1;}\n    if !same(&copy,&original) || different(&copy,&original) {return 2;}\n    if !reflexive(&copy) {return 3;}\n    if !same_operator(&copy,&original) || different_operator(&copy,&original) {return 5;}\n    if copied_scalar(&42i64)!=42 {return 6;}\n    copy.clear();\n    copied_from(&mut copy,&original);\n    drop(original);\n    if !same_bytes(&copy,\"generic\\0水😀\") {return 4;}\n    0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 223usize % workers == shard {
        record(source_case("generic_derive_validation_uses_constraints_instead_of_sampling_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] struct Item<T> {value:Vec<T>}")),
], ""), errors);
        count += 1;
    }
    if 224usize % workers == shard {
        record(source_case("generic_derive_validation_uses_constraints_instead_of_sampling_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] enum Chain<T> {End, Next(T,Box<Chain<T>>)}")),
], ""), errors);
        count += 1;
    }
    if 225usize % workers == shard {
        record(source_case("generic_derive_validation_uses_constraints_instead_of_sampling_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Copy,Clone)] struct Pair<T> {first:T,second:T}")),
], ""), errors);
        count += 1;
    }
    if 226usize % workers == shard {
        record(source_case("generic_derive_validation_uses_constraints_instead_of_sampling_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(PartialEq,Eq,PartialOrd,Ord)] struct Key<T> {value:T}")),
], ""), errors);
        count += 1;
    }
    if 227usize % workers == shard {
        record(source_case("generic_derive_validation_uses_constraints_instead_of_sampling_0004", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Copy,Clone)] struct Impossible<T> {value:Vec<T>}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 228usize % workers == shard {
        record(source_case("generic_derive_validation_uses_constraints_instead_of_sampling_0005", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] struct Impossible<'a,T> {value:&'a mut T}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 229usize % workers == shard {
        record(source_case("generic_impl_methods_are_checked_even_when_unused_0000", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T>{value:T} impl<T> Holder<T>{fn bad(&self)->i64{true}}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 230usize % workers == shard {
        record(source_case("generic_local_borrows_cannot_escape_0001", "case.rs", None, vec![source_file("case.rs", Some("fn bad<'a,T>(x:T)->&'a T {&x}"))], "E_BORROW"), errors);
        count += 1;
    }
    if 231usize % workers == shard {
        record(source_case("heap_pressure_releases_reservation_and_stack_regrows_with_live_references_0000", "trim-regrow", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("trim-regrow", Some("\n        fn grow(n:i64,parent:&i64,a:i64,b:i64,c:i64,d:i64,e:i64,f:i64)->i64 {\n            let page = [n;1024];\n            if n == 0 { return *parent+a+b+c+d+e+f; }\n            let result = grow(n-1,&page[17],a,b,c,d,e,f);\n            assert!(page[17] == n);\n            result\n        }\n        pub fn probe()->i64 {\n            let mut values = Vec::new();\n            let mut index = 0usize;\n            while index < 2097152 {\n                values.push(7i64);\n                index += 1;\n            }\n            let marker = 41i64;\n            let result = grow(96,&marker,1,2,3,4,5,6);\n            result + values[0] + values[2097151]\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 232usize % workers == shard {
        record(source_case("implicit_block_statements_require_unit_even_in_unused_generic_bodies_0000", "block_statements.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("block_statements.rs", Some("fn bad()->i64{{1i64} -1i64;0}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 233usize % workers == shard {
        record(source_case("implicit_block_statements_require_unit_even_in_unused_generic_bodies_0001", "block_statements.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("block_statements.rs", Some("fn bad()->i64{if true {1i64}else{2i64} 0}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 234usize % workers == shard {
        record(source_case("implicit_block_statements_require_unit_even_in_unused_generic_bodies_0002", "block_statements.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("block_statements.rs", Some("fn bad()->i64{match 0i64{_=>1i64} 0}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 235usize % workers == shard {
        record(source_case("implicit_block_statements_require_unit_even_in_unused_generic_bodies_0003", "block_statements.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("block_statements.rs", Some("fn bad<T>(value:T)->i64{{1i64} 0} pub fn probe()->i64{42}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 236usize % workers == shard {
        record(source_case("implicit_block_statements_require_unit_even_in_unused_generic_bodies_0004", "block_statements.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("block_statements.rs", Some("fn bad<T>(value:T)->i64{return 0;{1i64} 0} pub fn probe()->i64{42}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 237usize % workers == shard {
        record(source_case("implicit_block_statements_require_unit_even_in_unused_generic_bodies_0005", "block_statements.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("block_statements.rs", Some("fn bad()->i64{return 0;{1i64} 0}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 238usize % workers == shard {
        record(source_case("incomplete_inclusive_ranges_have_a_named_syntax_error_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{0..= =>1,_=>0}}")),
], "E_PARSE"), errors);
        count += 1;
    }
    if 239usize % workers == shard {
        record(source_case("incomplete_inclusive_ranges_have_a_named_syntax_error_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{..= =>1,_=>0}}")),
], "E_PARSE"), errors);
        count += 1;
    }
    if 240usize % workers == shard {
        record(source_case("inline_views_keep_lengths_and_utf8_boundaries_0000", "views", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("views", Some("pub fn probe()->i64{let values=[10i64,20i64,22i64,99i64];let middle=&values[1..3];let text=\"aéz\";let part=&text[1..3];middle[0]+middle[1]+part.len() as i64-2}")),
], ""), errors);
        count += 1;
    }
    if 241usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0000", "main.rs", None, vec![
source_file("main.rs", Some("struct Thing{value:i64} type Alias=Thing;fn probe(){let v=Alias{value:true};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 242usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0001", "main.rs", None, vec![
source_file("main.rs", Some("struct Pair<A,B>{first:A,second:B} type Same<T> =Pair<T,T>;fn probe(){let v=Same{first:1u8,second:true};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 243usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0002", "main.rs", None, vec![
source_file("main.rs", Some("mod a{pub struct Thing{pub value:i64}pub type Alias=Thing;}mod b{pub struct Thing{pub value:i64}}fn probe(){let v:b::Thing=a::Alias{value:1};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 244usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0003", "main.rs", None, vec![
source_file("main.rs", Some("struct Thing{value:i64}type Alias=Thing;fn probe(){let v=Alias::<i64>{value:1};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 245usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0004", "main.rs", None, vec![
source_file("main.rs", Some("struct Thing{left:i64,right:i64}type Alias=Thing;fn probe(){let v=Alias{left:1};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 246usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0005", "main.rs", None, vec![
source_file("main.rs", Some("struct Thing{value:i64}type Alias=Thing;fn probe(){let v=Alias{value:1,value:2};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 247usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0006", "main.rs", None, vec![
source_file("main.rs", Some("struct Thing{value:i64}type Alias=Thing;fn probe(){let v=Alias{other:1};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 248usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0007", "main.rs", None, vec![
source_file("main.rs", Some("enum Packet<T>{Data{value:T}}type Alias<T> =Packet<T>;fn probe(){let v=Alias::Data{other:1i64};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 249usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0008", "main.rs", None, vec![
source_file("main.rs", Some("type Alias=i64;fn probe(){let v=Alias{};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 250usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0009", "main.rs", None, vec![
source_file("main.rs", Some("struct Holder<T>{value:T}type Alias<T> =Holder<T>;fn unused<U>(v:U){let bad=Alias::<i64>{value:true};}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 251usize % workers == shard {
        record(source_case("invalid_alias_constructors_fail_for_the_intended_type_reason_0010", "main.rs", None, vec![
source_file("main.rs", Some("struct Tuple(i64);type Alias=Tuple;fn probe(){let v=Alias(42);}")),
], "E_NAME"), errors);
        count += 1;
    }
    if 252usize % workers == shard {
        record(source_case_at("invalid_production_clone_mutation_reports_the_library_file_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", Some("//! UTF-8 strings own bytes. Validation happens when unchecked bytes become a String.\nuse crate::Result;\nuse crate::bytes::{append_bytes, starts_with, valid_utf8};\n\n#[derive(PartialOrd, Ord)]\npub struct String {\n    bytes: Vec<u8>,\n}\n\npub struct FromUtf8Error {\n    bytes: Vec<u8>,\n}\n\nimpl String {\n    pub fn new() -> String {\n        String { bytes: Vec::new() }\n    }\n\n    pub fn from_utf8(bytes: Vec<u8>) -> Result<String, FromUtf8Error> {\n        if valid_utf8(&bytes) {\n            Result::Ok(String { bytes })\n        } else {\n            Result::Err(FromUtf8Error { bytes })\n        }\n    }\n\n    pub fn from(value: &str) -> String {\n        String::from_str(value)\n    }\n\n    pub fn from_str(value: &str) -> String {\n        let mut result = String::new();\n        result.push_str(value);\n        result\n    }\n\n    pub fn push_str(&mut self, value: &str) {\n        append_bytes(&mut self.bytes, value.as_bytes());\n    }\n\n    pub fn push(&mut self, value: char) {\n        let code = value as u32;\n        if code < 0x80u32 {\n            self.bytes.push(code as u8);\n        } else if code < 0x800u32 {\n            self.bytes.push(((code >> 6) | 0xc0u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else if code < 0x10000u32 {\n            self.bytes.push(((code >> 12) | 0xe0u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else {\n            self.bytes.push(((code >> 18) | 0xf0u32) as u8);\n            self.bytes.push((((code >> 12) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        }\n    }\n\n    pub fn starts_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_starts_with(self.as_str(), pattern)\n    }\n    pub fn ends_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_ends_with(self.as_str(), pattern)\n    }\n    pub fn contains<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_contains(self.as_str(), pattern)\n    }\n    pub fn find<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_find(self.as_str(), pattern)\n    }\n    pub fn rfind<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_rfind(self.as_str(), pattern)\n    }\n    pub fn is_ascii(&self) -> bool {\n        crate::str::__lkjscript_is_ascii(self.as_str())\n    }\n    pub fn to_ascii_lowercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_lowercase(self.as_str())\n    }\n    pub fn to_ascii_uppercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_uppercase(self.as_str())\n    }\n    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {\n        crate::str::__lkjscript_eq_ignore_ascii_case(self.as_str(), other)\n    }\n    pub fn repeat(&self, count: usize) -> String {\n        crate::str::__lkjscript_repeat(self.as_str(), count)\n    }\n    pub fn replace<P: crate::str::Pattern>(&self, pattern: P, replacement: &str) -> String {\n        crate::str::__lkjscript_replace(self.as_str(), pattern, replacement)\n    }\n\n    pub fn to_string(&self) -> String {\n        self.clone()\n    }\n\n    pub fn as_str(&self) -> &str {\n        std::str::from_utf8(self.bytes.as_slice()).unwrap()\n    }\n\n    pub fn len(&self) -> usize {\n        self.bytes.len()\n    }\n\n    pub fn is_empty(&self) -> bool {\n        self.bytes.len() == 0\n    }\n\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n\n    pub fn clear(&mut self) {\n        while self.bytes.len() > 0 {\n            self.bytes.pop();\n        }\n    }\n}\n\n// Only valid UTF-8 can enter the private byte storage, so exact copies preserve it.\nimpl Clone for String {\n    fn clone(&self) -> Self {\n        let mut bytes: Vec<u8> = Vec::new();\n        append_bytes(&mut bytes, true);\n        String { bytes }\n    }\n\n    fn clone_from(&mut self, source: &Self) {\n        self.clear();\n        append_bytes(&mut self.bytes, source.bytes.as_slice());\n    }\n}\n\nimpl PartialEq for String {\n    fn eq(&self, other: &Self) -> bool {\n        self.bytes.len() == other.bytes.len()\n            && starts_with(self.bytes.as_slice(), other.bytes.as_slice())\n    }\n\n    // lkjscript currently requires an executable method for each builtin operation.\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self, other: &Self) -> bool {\n        !self.eq(other)\n    }\n}\n\nimpl Eq for String {}\n\nimpl FromUtf8Error {\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n}\n\n// Resolved compiler helpers retain the same source String implementation.\npub fn __lkjscript_new() -> String {\n    String::new()\n}\n\n// Range syntax resolves this declaration, preserving the source String invariant.\npub fn __lkjscript_as_str(value: &String) -> &str {\n    value.as_str()\n}\n\n/// Canonical lexical String ordering uses its UTF-8 bytes, as Rust does.\npub fn __lkjscript_compare(left: &String, right: &String) -> i32 {\n    let left = left.as_bytes();\n    let right = right.as_bytes();\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1;\n        }\n        if left[index] > right[index] {\n            return 1;\n        }\n        index = index + 1;\n    }\n    if left.len() < right.len() {\n        -1\n    } else if left.len() > right.len() {\n        1\n    } else {\n        0\n    }\n}\n")),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn probe()->usize{String::from_str(\"owned\").clone().len()}")),
], "E_TYPE", "$std/string.rs", 0, 0), errors);
        count += 1;
    }
    if 253usize % workers == shard {
        record(source_case_at("invalid_production_ir_body_mutation_reports_its_original_file_0000", "src/ir_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("src/ir_component.rs", Some("//! Execution probes for the unchanged production IR representation module.\nmod ir;\n\nfn flag(value: &bool) -> i64 {\n    if *value { 1 } else { 0 }\n}\n\nfn place(value: usize) -> ir::Place {\n    let mut result = ir::Place::local(value);\n    result.projections.push(ir::Projection::Deref);\n    result.projections.push(ir::Projection::Field(value + 1));\n    result\n}\n\nfn operation(which: usize, changed: bool) -> ir::Op {\n    let n: usize = if changed { 43 } else { 42 };\n    if changed && which >= 19 && which <= 23 {\n        return ir::Op::Trap {\n            message: \"different tag\".to_string(),\n        };\n    }\n    match which {\n        0 => ir::Op::Text {\n            dst: n,\n            bytes: vec![0u8, 65u8, 255u8],\n        },\n        1 => ir::Op::Int { dst: n, value: -17 },\n        2 => ir::Op::Float {\n            dst: n,\n            bits: 4607182418800017408u64,\n        },\n        3 => ir::Op::Unit { dst: n },\n        4 => ir::Op::Read {\n            dst: n,\n            place: place(7),\n            moving: true,\n        },\n        5 => ir::Op::Borrow {\n            dst: n,\n            place: place(8),\n            mutable: false,\n        },\n        6 => ir::Op::SliceBorrow {\n            dst: n,\n            place: place(9),\n            mutable: true,\n        },\n        7 => ir::Op::Array {\n            dst: n,\n            elements: vec![3usize, 5usize],\n        },\n        8 => ir::Op::Assign {\n            place: place(n),\n            src: 11,\n        },\n        9 => ir::Op::Drop { place: place(n) },\n        10 => ir::Op::Unary {\n            dst: n,\n            op: ir::UnaryOp::Not,\n            src: 12,\n        },\n        11 => ir::Op::Binary {\n            dst: n,\n            op: ir::BinaryOp::Add,\n            left: 13,\n            right: 14,\n        },\n        12 => ir::Op::Call {\n            dst: n,\n            function: 4,\n            args: vec![15usize, 16usize],\n        },\n        13 => ir::Op::Struct {\n            dst: n,\n            fields: vec![17usize, 18usize],\n        },\n        14 => ir::Op::Variant {\n            dst: n,\n            variant: 2,\n            fields: vec![19usize, 20usize],\n        },\n        15 => ir::Op::Discriminant { dst: n, src: 21 },\n        16 => ir::Op::TagTest {\n            dst: n,\n            place: ir::Place::local(22),\n            variant: 3,\n        },\n        17 => ir::Op::Intrinsic {\n            dst: n,\n            name: ir::IntrinsicOp::VecNew,\n            args: vec![23usize, 24usize],\n        },\n        18 => ir::Op::If { condition: n },\n        19 => ir::Op::Else,\n        20 => ir::Op::End,\n        21 => ir::Op::Loop,\n        22 => ir::Op::Break,\n        23 => ir::Op::Continue,\n        24 => ir::Op::Return { src: n },\n        26 => ir::Op::DropIf { place: place(n) },\n        27 => ir::Op::BreakTo { depth: n },\n        28 => ir::Op::ContinueTo { depth: n },\n        29 => ir::Op::ViewRange {\n            dst: n,\n            place: place(6),\n            start: 7,\n            end: 8,\n            inclusive: true,\n            mutable: false,\n        },\n        _ => ir::Op::Trap {\n            message: if changed {\n                \"changed\".to_string()\n            } else {\n                \"trap\\0é水😀\".to_string()\n            },\n        },\n    }\n}\n\nfn operation_value(value: &ir::Op) -> i64 {\n    match value {\n        ir::Op::Text { dst, bytes } => *dst as i64 + bytes.len() as i64 + bytes[2] as i64,\n        ir::Op::Int { dst, value } => *dst as i64 + *value,\n        ir::Op::Float { dst, bits } => *dst as i64 + (*bits / 65536) as i64,\n        ir::Op::Unit { dst } => *dst as i64,\n        ir::Op::Read { dst, place, moving } => {\n            *dst as i64 + place.local as i64 + place.projections.len() as i64 + flag(moving)\n        }\n        ir::Op::Borrow {\n            dst,\n            place,\n            mutable,\n        } => *dst as i64 + place.local as i64 + flag(mutable),\n        ir::Op::SliceBorrow {\n            dst,\n            place,\n            mutable,\n        } => *dst as i64 + place.local as i64 + flag(mutable),\n        ir::Op::ViewRange {\n            dst,\n            place,\n            start,\n            end,\n            inclusive,\n            mutable,\n        } => {\n            *dst as i64\n                + place.local as i64\n                + *start as i64\n                + *end as i64\n                + flag(inclusive)\n                + flag(mutable)\n        }\n        ir::Op::Array { dst, elements } => *dst as i64 + elements[0] as i64 + elements[1] as i64,\n        ir::Op::Assign { place, src } => place.local as i64 + *src as i64,\n        ir::Op::Drop { place } => place.local as i64,\n        ir::Op::DropIf { place } => place.local as i64 + 1,\n        ir::Op::Unary { dst, op, src } => {\n            *dst as i64 + op.as_str().as_bytes()[0] as i64 + *src as i64\n        }\n        ir::Op::Binary {\n            dst,\n            op,\n            left,\n            right,\n        } => *dst as i64 + op.as_str().as_bytes()[0] as i64 + *left as i64 + *right as i64,\n        ir::Op::Call {\n            dst,\n            function,\n            args,\n        } => *dst as i64 + *function as i64 + args[0] as i64 + args[1] as i64,\n        ir::Op::Struct { dst, fields } => *dst as i64 + fields[0] as i64 + fields[1] as i64,\n        ir::Op::Variant {\n            dst,\n            variant,\n            fields,\n        } => *dst as i64 + *variant as i64 + fields[0] as i64 + fields[1] as i64,\n        ir::Op::Discriminant { dst, src } => *dst as i64 + *src as i64,\n        ir::Op::TagTest {\n            dst,\n            place,\n            variant,\n        } => *dst as i64 + place.local as i64 + *variant as i64,\n        ir::Op::Intrinsic { dst, name, args } => {\n            *dst as i64 + name.as_str().len() as i64 + args[0] as i64 + args[1] as i64\n        }\n        ir::Op::If { condition } => *condition as i64,\n        ir::Op::Else => 1001,\n        ir::Op::End => 1002,\n        ir::Op::Loop => 1003,\n        ir::Op::Break => 1004,\n        ir::Op::Continue => 1005,\n        ir::Op::BreakTo { depth } => 1006 + *depth as i64,\n        ir::Op::ContinueTo { depth } => 1007 + *depth as i64,\n        ir::Op::Return { src } => *src as i64,\n        ir::Op::Trap { message } => message.len() as i64,\n    }\n}\n\nfn kind_value(kind: &ir::TypeKind) -> i64 {\n    match kind {\n        ir::TypeKind::Unit => 1,\n        ir::TypeKind::Bool => 2,\n        ir::TypeKind::Char => 5,\n        ir::TypeKind::Int { bits, signed } => *bits as i64 * 2 + flag(signed),\n        ir::TypeKind::F64 => 3,\n        ir::TypeKind::Struct { fields, copy } => {\n            fields.len() as i64 + fields[0].name.len() as i64 + fields[0].ty as i64 + flag(copy)\n        }\n        ir::TypeKind::Enum { variants, copy } => {\n            variants.len() as i64\n                + variants[0].name.len() as i64\n                + variants[1].fields[0].ty as i64\n                + flag(copy)\n        }\n        ir::TypeKind::Ref { inner, mutable } => *inner as i64 + flag(mutable),\n        ir::TypeKind::Slice { inner, mutable } => *inner as i64 + flag(mutable),\n        ir::TypeKind::Array { inner, len } => *inner as i64 + *len as i64,\n        ir::TypeKind::Vec { inner } => *inner as i64,\n        ir::TypeKind::Box { inner } => *inner as i64,\n        ir::TypeKind::Str => 4,\n        ir::TypeKind::World => 5,\n        ir::TypeKind::Stdout => 6,\n        ir::TypeKind::Stdin => 7,\n        ir::TypeKind::Stderr => 8,\n        ir::TypeKind::Directory => 9,\n        ir::TypeKind::File => 10,\n        ir::TypeKind::Process => 11,\n        ir::TypeKind::Clock => 12,\n        ir::TypeKind::Child => 13,\n    }\n}\n\nfn model() -> ir::Program {\n    let mut types = ir::primitives();\n    types.push(ir::Type {\n        name: \"record\".to_string(),\n        kind: ir::TypeKind::Struct {\n            fields: vec![ir::Field {\n                name: \"field\\0水\".to_string(),\n                ty: ir::I64,\n            }],\n            copy: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"choice\".to_string(),\n        kind: ir::TypeKind::Enum {\n            variants: vec![\n                ir::Variant {\n                    name: \"Empty\".to_string(),\n                    fields: Vec::new(),\n                },\n                ir::Variant {\n                    name: \"Full\".to_string(),\n                    fields: vec![ir::Field {\n                        name: \"value\".to_string(),\n                        ty: ir::I64,\n                    }],\n                },\n            ],\n            copy: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"reference\".to_string(),\n        kind: ir::TypeKind::Ref {\n            inner: ir::I64,\n            mutable: true,\n        },\n    });\n    types.push(ir::Type {\n        name: \"slice\".to_string(),\n        kind: ir::TypeKind::Slice {\n            inner: ir::U8,\n            mutable: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"array\".to_string(),\n        kind: ir::TypeKind::Array {\n            inner: ir::BOOL,\n            len: 17,\n        },\n    });\n    types.push(ir::Type {\n        name: \"vector\".to_string(),\n        kind: ir::TypeKind::Vec { inner: ir::I32 },\n    });\n    types.push(ir::Type {\n        name: \"box\".to_string(),\n        kind: ir::TypeKind::Box { inner: ir::U64 },\n    });\n    types.push(ir::Type {\n        name: \"str\".to_string(),\n        kind: ir::TypeKind::Str,\n    });\n    types.push(ir::Type {\n        name: \"world\".to_string(),\n        kind: ir::TypeKind::World,\n    });\n    types.push(ir::Type {\n        name: \"stdout\".to_string(),\n        kind: ir::TypeKind::Stdout,\n    });\n    types.push(ir::Type {\n        name: \"stdin\".to_string(),\n        kind: ir::TypeKind::Stdin,\n    });\n    types.push(ir::Type {\n        name: \"stderr\".to_string(),\n        kind: ir::TypeKind::Stderr,\n    });\n    types.push(ir::Type {\n        name: \"directory\".to_string(),\n        kind: ir::TypeKind::Directory,\n    });\n    types.push(ir::Type {\n        name: \"file\".to_string(),\n        kind: ir::TypeKind::File,\n    });\n    let mut body: Vec<ir::Instruction> = Vec::new();\n    let mut index: usize = 0;\n    while index < 30 {\n        body.push(ir::Instruction {\n            column: 1,\n            op: operation(index, false),\n            line: 100 + index,\n        });\n        index = index + 1;\n    }\n    let function = ir::Function {\n        file: 2,\n        name: \"function\\0é水😀\".to_string(),\n        params: 1,\n        result: ir::I64,\n        return_source: Option::Some(ir::ReturnSource::Parameter {\n            index: 0,\n            projections: vec![\n                ir::Projection::Deref,\n                ir::Projection::Field(3),\n                ir::Projection::VariantField(2, 4),\n                ir::Projection::Index(9),\n            ],\n        }),\n        locals: vec![ir::Local {\n            name: \"parameter\".to_string(),\n            ty: ir::I64,\n            mutable: true,\n        }],\n        body,\n        test: true,\n    };\n    ir::Program {\n        types,\n        functions: vec![function],\n        destructors: vec![(7usize, 0usize)],\n    }\n}\n\npub fn primitive_snapshot(index: usize, field: usize) -> i64 {\n    let original = ir::primitives();\n    let copied = original.clone();\n    drop(original);\n    if field == 0 {\n        return kind_value(&copied[index].kind);\n    }\n    if field == 1 {\n        return copied[index].name.len() as i64;\n    }\n    let bytes = copied[index].name.as_bytes();\n    if field - 2 < bytes.len() {\n        bytes[field - 2] as i64\n    } else {\n        -1\n    }\n}\n\npub fn capability_snapshot(which: usize) -> i64 {\n    let original = match which {\n        0 => ir::TypeKind::Process,\n        1 => ir::TypeKind::Clock,\n        _ => ir::TypeKind::Child,\n    };\n    let copied = original.clone();\n    assert!(copied == original);\n    drop(original);\n    let different = if which == 0 {\n        ir::TypeKind::Clock\n    } else {\n        ir::TypeKind::Process\n    };\n    assert!(copied != different);\n    kind_value(&copied)\n}\n\npub fn operation_snapshot(which: usize) -> i64 {\n    let original = operation(which, false);\n    let copied = original.clone();\n    assert!(original == copied);\n    drop(original);\n    operation_value(&copied)\n}\n\npub fn operation_equality(which: usize, changed: bool) -> i64 {\n    let original = operation(which, false);\n    let candidate = operation(which, changed);\n    let result = original == candidate;\n    assert!((original != candidate) == !result);\n    if result { 1 } else { 0 }\n}\n\npub fn program_snapshot(field: usize) -> i64 {\n    let mut original = model();\n    let copied = original.clone();\n    assert!(copied == original);\n    original.functions[0].name.push_str(\" changed\");\n    original.functions[0].body[0].op = operation(0, true);\n    original.destructors[0].0 = 99;\n    assert!(copied != original);\n    drop(original);\n    let checked = ir::CheckedProgram { program: copied };\n    let retained: ir::Program = checked.program().clone();\n    drop(checked);\n    assert!(retained == model());\n    if field == 0 {\n        let mut total: i64 = 0;\n        let mut index: usize = 0;\n        while index < retained.types.len() {\n            total = total + kind_value(&retained.types[index].kind);\n            index = index + 1;\n        }\n        return total;\n    }\n    if field == 1 {\n        let mut total: i64 = 0;\n        let mut index: usize = 0;\n        while index < retained.functions[0].body.len() {\n            total = total + operation_value(&retained.functions[0].body[index].op);\n            total = total + retained.functions[0].body[index].line as i64;\n            index = index + 1;\n        }\n        return total;\n    }\n    if field == 2 {\n        return retained.functions[0].name.len() as i64;\n    }\n    if field == 3 {\n        return retained.destructors[0].0 as i64;\n    }\n    if field == 4 {\n        return retained.functions[0].locals[0].name.len() as i64;\n    }\n    match &retained.functions[0].return_source {\n        Option::None => -1,\n        Option::Some(source) => match source {\n            ir::ReturnSource::Static => -2,\n            ir::ReturnSource::Leaves(leaves) => leaves.len() as i64,\n            ir::ReturnSource::Parameter { index, projections } => {\n                *index as i64 + projections.len() as i64\n            }\n        },\n    }\n}\n\nfn projections_value(projections: &Vec<ir::Projection>) -> i64 {\n    let mut result: i64 = 0;\n    let mut cursor: usize = 0;\n    while cursor < projections.len() {\n        result = result\n            + match &projections[cursor] {\n                ir::Projection::Deref => 1,\n                ir::Projection::Field(value) => *value as i64,\n                ir::Projection::VariantField(tag, value) => *tag as i64 + *value as i64,\n                ir::Projection::Index(value) => *value as i64,\n            };\n        cursor = cursor + 1;\n    }\n    result\n}\n\npub fn return_source_snapshot(which: usize) -> i64 {\n    let original: Option<ir::ReturnSource> = if which == 0 {\n        Option::None\n    } else if which == 1 {\n        Option::Some(ir::ReturnSource::Static)\n    } else if which == 2 {\n        Option::Some(ir::ReturnSource::Parameter {\n            index: 17,\n            projections: vec![\n                ir::Projection::Deref,\n                ir::Projection::Field(2),\n                ir::Projection::VariantField(3, 4),\n                ir::Projection::Index(5),\n            ],\n        })\n    } else {\n        let origin = if which == 5 {\n            ir::LeafOrigin::Static\n        } else {\n            ir::LeafOrigin::Inputs {\n                inputs: vec![\n                    ir::InputLeaf {\n                        index: 11,\n                        projections: vec![ir::Projection::Deref, ir::Projection::Field(7)],\n                    },\n                    ir::InputLeaf {\n                        index: 13,\n                        projections: vec![ir::Projection::Field(2)],\n                    },\n                ],\n                exact: which == 3,\n            }\n        };\n        Option::Some(ir::ReturnSource::Leaves(vec![ir::ReturnLeaf {\n            projections: vec![ir::Projection::VariantField(2, 9)],\n            origin,\n        }]))\n    };\n    let copied = original.clone();\n    assert!(copied == original);\n    drop(original);\n    match copied {\n        Option::None => 100,\n        Option::Some(source) => match source {\n            ir::ReturnSource::Static => 101,\n            ir::ReturnSource::Parameter { index, projections } => {\n                index as i64 + projections_value(&projections)\n            }\n            ir::ReturnSource::Leaves(leaves) => {\n                let mut result: i64 = 0;\n                let mut cursor: usize = 0;\n                while cursor < leaves.len() {\n                    result = result + projections_value(&leaves[cursor].projections);\n                    result = result\n                        + match &leaves[cursor].origin {\n                            ir::LeafOrigin::Static => 103,\n                            ir::LeafOrigin::Inputs { inputs, exact } => {\n                                let mut value = flag(exact);\n                                let mut index: usize = 0;\n                                while index < inputs.len() {\n                                    value = value\n                                        + inputs[index].index as i64\n                                        + projections_value(&inputs[index].projections);\n                                    index = index + 1;\n                                }\n                                value\n                            }\n                        };\n                    cursor = cursor + 1;\n                }\n                result\n            }\n        },\n    }\n}\n\npub fn production_place_local() -> usize {\n    ir::Place::local(42).local\n}\n")),
source_file("src/ir.rs", Some("//! Explicit, monomorphic meaning. Only CheckedProgram reaches a backend.\n#[derive(Clone, PartialEq)]\npub enum TypeKind {\n    Unit,\n    Bool,\n    /// A Unicode scalar value: 0..=0x10ffff excluding 0xd800..=0xdfff.\n    Char,\n    Int {\n        bits: u8,\n        signed: bool,\n    },\n    F64,\n    Struct {\n        fields: Vec<Field>,\n        copy: bool,\n    },\n    Enum {\n        variants: Vec<Variant>,\n        copy: bool,\n    },\n    Ref {\n        inner: usize,\n        mutable: bool,\n    },\n    Slice {\n        inner: usize,\n        mutable: bool,\n    },\n    Array {\n        inner: usize,\n        len: usize,\n    },\n    Vec {\n        inner: usize,\n    },\n    Box {\n        inner: usize,\n    },\n    /// Unsized immutable UTF-8 text; stored values use a shared Ref.\n    Str,\n    World,\n    /// Affine authority to write standard output, derived from World.\n    Stdout,\n    Stdin,\n    Stderr,\n    /// Affine directory authority; descendants remain beneath this root.\n    Directory,\n    /// Affine open file; structural destruction closes its handle.\n    File,\n    Process,\n    Clock,\n    Child,\n}\n\n#[derive(Clone, PartialEq)]\npub struct Field {\n    pub name: String,\n    pub ty: usize,\n}\n#[derive(Clone, PartialEq)]\npub struct Variant {\n    pub name: String,\n    pub fields: Vec<Field>,\n}\n#[derive(Clone, PartialEq)]\npub struct Type {\n    pub name: String,\n    pub kind: TypeKind,\n}\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub enum Projection {\n    Deref,\n    Field(usize),\n    VariantField(usize, usize),\n    Index(usize),\n}\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub struct Place {\n    pub local: usize,\n    pub projections: Vec<Projection>,\n}\nimpl Place {\n    pub fn local(local: usize) -> Place {\n        Place {\n            local,\n            projections: Vec::new(),\n        }\n    }\n}\n#[derive(Clone, PartialEq)]\npub struct Local {\n    pub name: String,\n    pub ty: usize,\n    pub mutable: bool,\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum UnaryOp {\n    Neg,\n    Not,\n}\nimpl UnaryOp {\n    pub fn parse(text: &str) -> Option<UnaryOp> {\n        match text {\n            \"-\" => Some(UnaryOp::Neg),\n            \"!\" => Some(UnaryOp::Not),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            UnaryOp::Neg => \"-\",\n            UnaryOp::Not => \"!\",\n        }\n    }\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum BinaryOp {\n    Add,\n    Sub,\n    Mul,\n    Div,\n    Rem,\n    BitAnd,\n    BitOr,\n    BitXor,\n    Shl,\n    Shr,\n    Eq,\n    Ne,\n    Lt,\n    Le,\n    Gt,\n    Ge,\n    And,\n    Or,\n    WrappingAdd,\n    WrappingSub,\n    WrappingMul,\n}\nimpl BinaryOp {\n    pub fn parse(text: &str) -> Option<BinaryOp> {\n        match text {\n            \"+\" => Some(BinaryOp::Add),\n            \"-\" => Some(BinaryOp::Sub),\n            \"*\" => Some(BinaryOp::Mul),\n            \"/\" => Some(BinaryOp::Div),\n            \"%\" => Some(BinaryOp::Rem),\n            \"&\" => Some(BinaryOp::BitAnd),\n            \"|\" => Some(BinaryOp::BitOr),\n            \"^\" => Some(BinaryOp::BitXor),\n            \"<<\" => Some(BinaryOp::Shl),\n            \">>\" => Some(BinaryOp::Shr),\n            \"==\" => Some(BinaryOp::Eq),\n            \"!=\" => Some(BinaryOp::Ne),\n            \"<\" => Some(BinaryOp::Lt),\n            \"<=\" => Some(BinaryOp::Le),\n            \">\" => Some(BinaryOp::Gt),\n            \">=\" => Some(BinaryOp::Ge),\n            \"&&\" => Some(BinaryOp::And),\n            \"||\" => Some(BinaryOp::Or),\n            \"wrapping_add\" => Some(BinaryOp::WrappingAdd),\n            \"wrapping_sub\" => Some(BinaryOp::WrappingSub),\n            \"wrapping_mul\" => Some(BinaryOp::WrappingMul),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            BinaryOp::Add => \"+\",\n            BinaryOp::Sub => \"-\",\n            BinaryOp::Mul => \"*\",\n            BinaryOp::Div => \"/\",\n            BinaryOp::Rem => \"%\",\n            BinaryOp::BitAnd => \"&\",\n            BinaryOp::BitOr => \"|\",\n            BinaryOp::BitXor => \"^\",\n            BinaryOp::Shl => \"<<\",\n            BinaryOp::Shr => \">>\",\n            BinaryOp::Eq => \"==\",\n            BinaryOp::Ne => \"!=\",\n            BinaryOp::Lt => \"<\",\n            BinaryOp::Le => \"<=\",\n            BinaryOp::Gt => \">\",\n            BinaryOp::Ge => \">=\",\n            BinaryOp::And => \"&&\",\n            BinaryOp::Or => \"||\",\n            BinaryOp::WrappingAdd => \"wrapping_add\",\n            BinaryOp::WrappingSub => \"wrapping_sub\",\n            BinaryOp::WrappingMul => \"wrapping_mul\",\n        }\n    }\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum IntrinsicOp {\n    CharFromU32,\n    F64FromBits,\n    F64ToBits,\n    ArrayRepeat,\n    StrLen,\n    StrAsBytes,\n    StrFromUtf8,\n    Assert,\n    VecNew,\n    VecLen,\n    VecPush,\n    VecPop,\n    Replace,\n    BoxNew,\n    Cast,\n    Clone,\n    WorldStdout,\n    WorldStdin,\n    WorldStderr,\n    WorldArgCount,\n    WorldArgLen,\n    WorldArgCopy,\n    WorldExitStatus,\n    WorldDirectory,\n    DirectoryOpen,\n    StdinRead,\n    FileRead,\n    FileWrite,\n    StdoutWrite,\n    StderrWrite,\n    WorldProcess,\n    WorldClock,\n    WorldFile,\n    ClockRead,\n    ProcessSpawn,\n    ChildWait,\n    ChildTerminate,\n    DirectoryOpenDirectory,\n    DirectoryRead,\n    DirectoryRewind,\n    DirectoryCreate,\n    DirectoryRemove,\n    DirectoryRename,\n    FileRewind,\n    FileExecutable,\n    FileDuplicate,\n    FileReadOnly,\n}\nimpl IntrinsicOp {\n    pub fn parse(text: &str) -> Option<IntrinsicOp> {\n        match text {\n            \"char_from_u32\" => Some(IntrinsicOp::CharFromU32),\n            \"f64_from_bits\" => Some(IntrinsicOp::F64FromBits),\n            \"f64_to_bits\" => Some(IntrinsicOp::F64ToBits),\n            \"array_repeat\" => Some(IntrinsicOp::ArrayRepeat),\n            \"str_len\" => Some(IntrinsicOp::StrLen),\n            \"str_as_bytes\" => Some(IntrinsicOp::StrAsBytes),\n            \"str_from_utf8\" => Some(IntrinsicOp::StrFromUtf8),\n            \"assert\" => Some(IntrinsicOp::Assert),\n            \"vec_new\" => Some(IntrinsicOp::VecNew),\n            \"vec_len\" => Some(IntrinsicOp::VecLen),\n            \"vec_push\" => Some(IntrinsicOp::VecPush),\n            \"vec_pop\" => Some(IntrinsicOp::VecPop),\n            \"replace\" => Some(IntrinsicOp::Replace),\n            \"box_new\" => Some(IntrinsicOp::BoxNew),\n            \"cast\" => Some(IntrinsicOp::Cast),\n            \"clone\" => Some(IntrinsicOp::Clone),\n            \"world_stdout\" => Some(IntrinsicOp::WorldStdout),\n            \"world_stdin\" => Some(IntrinsicOp::WorldStdin),\n            \"world_stderr\" => Some(IntrinsicOp::WorldStderr),\n            \"world_arg_count\" => Some(IntrinsicOp::WorldArgCount),\n            \"world_arg_len\" => Some(IntrinsicOp::WorldArgLen),\n            \"world_arg_copy\" => Some(IntrinsicOp::WorldArgCopy),\n            \"world_exit_status\" => Some(IntrinsicOp::WorldExitStatus),\n            \"world_directory\" => Some(IntrinsicOp::WorldDirectory),\n            \"directory_open\" => Some(IntrinsicOp::DirectoryOpen),\n            \"stdin_read\" => Some(IntrinsicOp::StdinRead),\n            \"file_read\" => Some(IntrinsicOp::FileRead),\n            \"file_write\" => Some(IntrinsicOp::FileWrite),\n            \"stdout_write\" => Some(IntrinsicOp::StdoutWrite),\n            \"stderr_write\" => Some(IntrinsicOp::StderrWrite),\n            \"world_process\" => Some(IntrinsicOp::WorldProcess),\n            \"world_clock\" => Some(IntrinsicOp::WorldClock),\n            \"world_file\" => Some(IntrinsicOp::WorldFile),\n            \"clock_read\" => Some(IntrinsicOp::ClockRead),\n            \"process_spawn\" => Some(IntrinsicOp::ProcessSpawn),\n            \"child_wait\" => Some(IntrinsicOp::ChildWait),\n            \"child_terminate\" => Some(IntrinsicOp::ChildTerminate),\n            \"directory_open_directory\" => Some(IntrinsicOp::DirectoryOpenDirectory),\n            \"directory_read\" => Some(IntrinsicOp::DirectoryRead),\n            \"directory_rewind\" => Some(IntrinsicOp::DirectoryRewind),\n            \"directory_create\" => Some(IntrinsicOp::DirectoryCreate),\n            \"directory_remove\" => Some(IntrinsicOp::DirectoryRemove),\n            \"directory_rename\" => Some(IntrinsicOp::DirectoryRename),\n            \"file_rewind\" => Some(IntrinsicOp::FileRewind),\n            \"file_executable\" => Some(IntrinsicOp::FileExecutable),\n            \"file_duplicate\" => Some(IntrinsicOp::FileDuplicate),\n            \"file_readonly\" => Some(IntrinsicOp::FileReadOnly),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            IntrinsicOp::CharFromU32 => \"char_from_u32\",\n            IntrinsicOp::F64FromBits => \"f64_from_bits\",\n            IntrinsicOp::F64ToBits => \"f64_to_bits\",\n            IntrinsicOp::ArrayRepeat => \"array_repeat\",\n            IntrinsicOp::StrLen => \"str_len\",\n            IntrinsicOp::StrAsBytes => \"str_as_bytes\",\n            IntrinsicOp::StrFromUtf8 => \"str_from_utf8\",\n            IntrinsicOp::Assert => \"assert\",\n            IntrinsicOp::VecNew => \"vec_new\",\n            IntrinsicOp::VecLen => \"vec_len\",\n            IntrinsicOp::VecPush => \"vec_push\",\n            IntrinsicOp::VecPop => \"vec_pop\",\n            IntrinsicOp::Replace => \"replace\",\n            IntrinsicOp::BoxNew => \"box_new\",\n            IntrinsicOp::Cast => \"cast\",\n            IntrinsicOp::Clone => \"clone\",\n            IntrinsicOp::WorldStdout => \"world_stdout\",\n            IntrinsicOp::WorldStdin => \"world_stdin\",\n            IntrinsicOp::WorldStderr => \"world_stderr\",\n            IntrinsicOp::WorldArgCount => \"world_arg_count\",\n            IntrinsicOp::WorldArgLen => \"world_arg_len\",\n            IntrinsicOp::WorldArgCopy => \"world_arg_copy\",\n            IntrinsicOp::WorldExitStatus => \"world_exit_status\",\n            IntrinsicOp::WorldDirectory => \"world_directory\",\n            IntrinsicOp::DirectoryOpen => \"directory_open\",\n            IntrinsicOp::StdinRead => \"stdin_read\",\n            IntrinsicOp::FileRead => \"file_read\",\n            IntrinsicOp::FileWrite => \"file_write\",\n            IntrinsicOp::StdoutWrite => \"stdout_write\",\n            IntrinsicOp::StderrWrite => \"stderr_write\",\n            IntrinsicOp::WorldProcess => \"world_process\",\n            IntrinsicOp::WorldClock => \"world_clock\",\n            IntrinsicOp::WorldFile => \"world_file\",\n            IntrinsicOp::ClockRead => \"clock_read\",\n            IntrinsicOp::ProcessSpawn => \"process_spawn\",\n            IntrinsicOp::ChildWait => \"child_wait\",\n            IntrinsicOp::ChildTerminate => \"child_terminate\",\n            IntrinsicOp::DirectoryOpenDirectory => \"directory_open_directory\",\n            IntrinsicOp::DirectoryRead => \"directory_read\",\n            IntrinsicOp::DirectoryRewind => \"directory_rewind\",\n            IntrinsicOp::DirectoryCreate => \"directory_create\",\n            IntrinsicOp::DirectoryRemove => \"directory_remove\",\n            IntrinsicOp::DirectoryRename => \"directory_rename\",\n            IntrinsicOp::FileRewind => \"file_rewind\",\n            IntrinsicOp::FileExecutable => \"file_executable\",\n            IntrinsicOp::FileDuplicate => \"file_duplicate\",\n            IntrinsicOp::FileReadOnly => \"file_readonly\",\n        }\n    }\n}\n#[derive(Clone, PartialEq)]\npub enum Op {\n    /// Immutable module-long bytes: a UTF-8 str or an exact-length u8 array.\n    Text {\n        dst: usize,\n        bytes: Vec<u8>,\n    },\n    Int {\n        dst: usize,\n        value: i64,\n    },\n    Float {\n        dst: usize,\n        bits: u64,\n    },\n    Unit {\n        dst: usize,\n    },\n    Read {\n        dst: usize,\n        place: Place,\n        moving: bool,\n    },\n    Borrow {\n        dst: usize,\n        place: Place,\n        mutable: bool,\n    },\n    SliceBorrow {\n        dst: usize,\n        place: Place,\n        mutable: bool,\n    },\n    /// Borrow a checked subrange without narrowing its static alias footprint.\n    /// Bounds are consumed usize values; failure traps with the bounds code.\n    ViewRange {\n        dst: usize,\n        place: Place,\n        start: usize,\n        end: usize,\n        inclusive: bool,\n        mutable: bool,\n    },\n    Array {\n        dst: usize,\n        elements: Vec<usize>,\n    },\n    Assign {\n        place: Place,\n        src: usize,\n    },\n    Drop {\n        place: Place,\n    },\n    /// Destroy this place only when its own storage is initialized. Every\n    /// projected ancestor must still exist; this does not authorize a hole\n    /// through a reference or an indexed container.\n    DropIf {\n        place: Place,\n    },\n    Unary {\n        dst: usize,\n        op: UnaryOp,\n        src: usize,\n    },\n    Binary {\n        dst: usize,\n        op: BinaryOp,\n        left: usize,\n        right: usize,\n    },\n    Call {\n        dst: usize,\n        function: usize,\n        args: Vec<usize>,\n    },\n    Struct {\n        dst: usize,\n        fields: Vec<usize>,\n    },\n    Variant {\n        dst: usize,\n        variant: usize,\n        fields: Vec<usize>,\n    },\n    Discriminant {\n        dst: usize,\n        src: usize,\n    },\n    TagTest {\n        dst: usize,\n        place: Place,\n        variant: usize,\n    },\n    Intrinsic {\n        dst: usize,\n        name: IntrinsicOp,\n        args: Vec<usize>,\n    },\n    If {\n        condition: usize,\n    },\n    Else,\n    End,\n    Loop,\n    Break,\n    Continue,\n    /// Select an enclosing lexical loop, counting only loops (zero is nearest).\n    BreakTo {\n        depth: usize,\n    },\n    ContinueTo {\n        depth: usize,\n    },\n    Return {\n        src: usize,\n    },\n    Trap {\n        message: String,\n    },\n}\n#[derive(Clone, PartialEq)]\npub struct Instruction {\n    pub op: Op,\n    pub line: usize,\n    pub column: usize,\n}\n/// An input selector names one stored reference leaf in a parameter value.\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub struct InputLeaf {\n    pub index: usize,\n    pub projections: Vec<Projection>,\n}\n/// Every result reference leaf selects a declared input alternative or static storage.\n#[derive(Clone, PartialEq, Eq)]\npub enum LeafOrigin {\n    Inputs {\n        inputs: Vec<InputLeaf>,\n        /// Exact preserves an input referent; bounded permits a descendant.\n        exact: bool,\n    },\n    Static,\n}\n#[derive(Clone, PartialEq, Eq)]\npub struct ReturnLeaf {\n    pub projections: Vec<Projection>,\n    pub origin: LeafOrigin,\n}\n#[derive(Clone, PartialEq, Eq)]\npub enum ReturnSource {\n    /// A complete, unique contract for every reference leaf in the result.\n    Leaves(Vec<ReturnLeaf>),\n    /// Reference leaves in a parameter value, after the declared dereferences.\n    /// Projected sources currently require shared leaves; exclusive reborrows\n    /// retain the lifetime of their outer reference.\n    Parameter {\n        index: usize,\n        projections: Vec<Projection>,\n    },\n    Static,\n}\n#[derive(Clone, PartialEq)]\npub struct Function {\n    pub file: usize,\n    pub name: String,\n    pub params: usize,\n    pub result: usize,\n    pub return_source: Option<ReturnSource>,\n    pub locals: Vec<Local>,\n    pub body: Vec<Instruction>,\n    pub test: bool,\n}\n#[derive(Clone, PartialEq)]\npub struct Program {\n    pub types: Vec<Type>,\n    pub functions: Vec<Function>,\n    pub destructors: Vec<(usize, usize)>,\n}\npub struct CheckedProgram {\n    pub(crate) program: Program,\n}\nimpl CheckedProgram {\n    pub fn program(&self) -> &Program {\n        &self.program\n    }\n}\npub const UNIT: usize = 0;\npub const BOOL: usize = 1;\npub const I32: usize = 2;\npub const I64: usize = 3;\npub const U64: usize = 4;\npub const U8: usize = 5;\npub const F64: usize = 6;\npub fn primitives() -> Vec<Type> {\n    vec![\n        Type {\n            name: \"()\".to_string(),\n            kind: TypeKind::Unit,\n        },\n        Type {\n            name: \"bool\".to_string(),\n            kind: TypeKind::Bool,\n        },\n        Type {\n            name: \"i32\".to_string(),\n            kind: TypeKind::Int {\n                bits: true,\n                signed: true,\n            },\n        },\n        Type {\n            name: \"i64\".to_string(),\n            kind: TypeKind::Int {\n                bits: 64,\n                signed: true,\n            },\n        },\n        Type {\n            name: \"usize\".to_string(),\n            kind: TypeKind::Int {\n                bits: 64,\n                signed: false,\n            },\n        },\n        Type {\n            name: \"u8\".to_string(),\n            kind: TypeKind::Int {\n                bits: 8,\n                signed: false,\n            },\n        },\n        Type {\n            name: \"f64\".to_string(),\n            kind: TypeKind::F64,\n        },\n    ]\n}\n")),
], "E_TYPE", "src/ir.rs", 0, 0), errors);
        count += 1;
    }
    if 254usize % workers == shard {
        record(source_case_at("invalid_production_join_mutation_is_checked_0000", "text_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_component.rs", Some("mod names; use names::qualified as join;\npub fn compare_0()->usize {\nlet value=join(\"module\",\"leaf\");\nlet actual=value.as_bytes();\nlet expected=\"module::leaf\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\n")),
source_file("names.rs", Some("//! Canonical declaration names used by resolution, generic validation, and elaboration.\n/// Join one module path and declaration spelling without changing either input.\npub fn qualified(module: &str, name: &str) -> String {\n    let mut result = module.to_string();\n    if !module.is_empty() {\n        result.push_str(\"::\");\n    }\n    result.push_str(1i64);\n    result\n}\n")),
], "E_TYPE", "names.rs", 0, 0), errors);
        count += 1;
    }
    if 255usize % workers == shard {
        record(source_case_at("invalid_production_range_mutation_reports_the_original_file_0001", "json_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("json_component.rs", Some("mod json;\npub fn release(bytes:Vec<u8>) { drop(bytes); }\npub fn encode_0()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"text\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn main(mut world:World) { let mut out=world.stdout();\nlet value=encode_0();out.write(&value);\n}\n")),
source_file("json.rs", Some("//! JSON string bytes used by the emitter's trap metadata.\n//! Input is validated UTF-8; non-control bytes retain their original encoding.\n\nfn hex_digit(value: u8) -> u8 {\n    match value {\n        0..=9 => b'0' + value,\n        10..=15 => b'a' + (value - 10),\n        _ => panic!(\"invalid hexadecimal digit\"),\n    }\n}\n\n/// Append one quoted JSON string, preserving the emitter's lowercase escapes.\npub fn write_string(out: &mut Vec<u8>, text: &str) {\n    out.push(b'\"');\n    let bytes = text.as_bytes();\n    let mut index: usize = 0;\n    while index < bytes.len() {\n        let byte = bytes[index];\n        match byte {\n            b'\"' => {\n                out.push(b'\\\\');\n                out.push(b'\"');\n            }\n            b'\\\\' => {\n                out.push(b'\\\\');\n                out.push(b'\\\\');\n            }\n            32..=31 => {\n                out.push(b'\\\\');\n                out.push(b'u');\n                out.push(b'0');\n                out.push(b'0');\n                out.push(hex_digit(byte >> 4));\n                out.push(hex_digit(byte & 15));\n            }\n            _ => out.push(byte),\n        }\n        index = index + 1;\n    }\n    out.push(b'\"');\n}\n")),
], "E_TYPE", "json.rs", 0, 0), errors);
        count += 1;
    }
    if 256usize % workers == shard {
        record(source_case_at("invalid_sequence_source_mutation_keeps_the_library_diagnostic_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", Some("//! Vec operations are ordinary source over checked element access.\nuse crate::prelude::Option;\npub fn __lkjscript_vec_clone<T: Clone>(source: &Vec<T>) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    let mut index: usize = 0;\n    while index < source.len() {\n        result.push(true);\n        index = index + 1;\n    }\n    result\n}\n\npub fn __lkjscript_vec_equal<T: PartialEq>(left: &Vec<T>, right: &Vec<T>) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index: usize = 0;\n    while index < left.len() {\n        // Equality uses the element's eq contract; ne may be independently overridden.\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index = index + 1;\n    }\n    true\n}\n\n// Clone the first count-1 elements and move the original into the last slot.\n// The element expression is evaluated even when count is zero.\npub fn __lkjscript_vec_repeat<T: Clone>(value: T, count: usize) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    if count == 0 {\n        return result;\n    }\n    let mut remaining = count - 1;\n    while remaining > 0 {\n        result.push(value.clone());\n        remaining = remaining - 1;\n    }\n    result.push(value);\n    result\n}\n\n/// Shared inspection does not require an element equality or copying contract.\npub trait VecAccess<T> {\n    fn is_empty(&self) -> bool;\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T>;\n    fn first<'a>(&'a self) -> Option<&'a T>;\n    fn last<'a>(&'a self) -> Option<&'a T>;\n}\n\nimpl<T> VecAccess<T> for Vec<T> {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\n\n/// Equality search borrows its elements and stops at the first match.\npub trait VecContains<T: PartialEq> {\n    fn contains(&self, value: &T) -> bool;\n}\n\nimpl<T: PartialEq> VecContains<T> for Vec<T> {\n    fn contains(&self, value: &T) -> bool {\n        let mut index: usize = 0;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index = index + 1;\n        }\n        false\n    }\n}\n\n/// Append independent clones in source order; the shared source remains usable.\npub trait VecExtend<T: Clone> {\n    fn extend_from_slice(&mut self, values: &[T]);\n}\n\nimpl<T: Clone> VecExtend<T> for Vec<T> {\n    fn extend_from_slice(&mut self, values: &[T]) {\n        let mut index: usize = 0;\n        while index < values.len() {\n            self.push(values[index].clone());\n            index = index + 1;\n        }\n    }\n}\n\n/// Mutation visits values in their original order and keeps affine ownership.\npub trait VecMutate<T> {\n    fn retain<F: FnMut(&T) -> bool>(&mut self, predicate: F);\n}\nimpl<T> VecMutate<T> for Vec<T> {\n    // Explicit replacement uses the declared primitive; mem::take is not provided.\n    #[allow(clippy::mem_replace_with_default)]\n    fn retain<F: FnMut(&T) -> bool>(&mut self, mut predicate: F) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            if predicate(&value) {\n                self.push(value);\n            }\n        }\n    }\n}\n\n/// Stable merge sorting requires only checked indexing and owned Option slots.\n/// Each pass is linear; the number of passes grows logarithmically with length.\npub trait VecSort<T: Ord> {\n    fn sort(&mut self);\n}\nimpl<T: Ord> VecSort<T> for Vec<T> {\n    #[allow(clippy::mem_replace_with_default)]\n    fn sort(&mut self) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        let mut values: Vec<Option<T>> = Vec::new();\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            values.push(Option::Some(value));\n        }\n        let count = values.len();\n        let mut width = 1usize;\n        while width < count {\n            let mut merged: Vec<Option<T>> = Vec::new();\n            let mut start = 0usize;\n            while start < count {\n                let left_count = if width < count - start {\n                    width\n                } else {\n                    count - start\n                };\n                let middle = start + left_count;\n                let right_count = if width < count - middle {\n                    width\n                } else {\n                    count - middle\n                };\n                let end = middle + right_count;\n                let mut left = start;\n                let mut right = middle;\n                while left < middle || right < end {\n                    let use_left = if left == middle {\n                        false\n                    } else if right == end {\n                        true\n                    } else {\n                        *values[left].as_ref().unwrap() <= *values[right].as_ref().unwrap()\n                    };\n                    if use_left {\n                        merged.push(values[left].take());\n                        left = left + 1;\n                    } else {\n                        merged.push(values[right].take());\n                        right = right + 1;\n                    }\n                }\n                start = end;\n            }\n            values = merged;\n            width = if width > count / 2 { count } else { width * 2 };\n        }\n        let mut index = 0usize;\n        while index < count {\n            self.push(values[index].take().unwrap());\n            index = index + 1;\n        }\n    }\n}\n\n/// Slice inspection has the same borrowing contract as vector inspection.\nimpl<T> VecAccess<T> for [T] {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\nimpl<T: PartialEq> VecContains<T> for [T] {\n    fn contains(&self, value: &T) -> bool {\n        let mut index = 0usize;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index += 1;\n        }\n        false\n    }\n}\n\npub trait VecAccessMut<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T>;\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T>;\n}\nimpl<T> VecAccessMut<T> for Vec<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\nimpl<T> VecAccessMut<T> for [T] {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\n\npub trait SliceClone<T: Clone> {\n    fn to_vec(&self) -> Vec<T>;\n}\nimpl<T: Clone> SliceClone<T> for [T] {\n    fn to_vec(&self) -> Vec<T> {\n        let mut result: Vec<T> = Vec::new();\n        let mut index = 0usize;\n        while index < self.len() {\n            result.push(self[index].clone());\n            index += 1;\n        }\n        result\n    }\n}\n\npub trait SequencePrefix<T: PartialEq> {\n    fn starts_with(&self, prefix: &[T]) -> bool;\n}\nimpl<T: PartialEq> SequencePrefix<T> for [T] {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\nimpl<T: PartialEq> SequencePrefix<T> for Vec<T> {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\n\npub trait SliceCopy<T: Copy> {\n    fn copy_from_slice(&mut self, source: &[T]);\n}\nimpl<T: Copy> SliceCopy<T> for [T] {\n    fn copy_from_slice(&mut self, source: &[T]) {\n        assert!(self.len() == source.len());\n        let mut index = 0usize;\n        while index < self.len() {\n            self[index] = source[index];\n            index += 1;\n        }\n    }\n}\n\npub trait VecClear<T> {\n    fn clear(&mut self);\n    fn truncate(&mut self, length: usize);\n}\nimpl<T> VecClear<T> for Vec<T> {\n    fn clear(&mut self) {\n        self.retain(|_| false);\n    }\n    fn truncate(&mut self, length: usize) {\n        if length >= self.len() {\n            return;\n        }\n        let mut index = 0usize;\n        self.retain(|_| {\n            let keep = index < length;\n            index += 1;\n            keep\n        });\n    }\n}\n\n/// A four-way lexicographic result: less, equal, greater, or unordered.\n/// Relational operators are used directly so custom equality does not affect order.\n#[allow(clippy::neg_cmp_op_on_partial_ord)]\npub fn __lkjscript_slice_order<T: PartialOrd>(left: &[T], right: &[T]) -> i32 {\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1i32;\n        }\n        if left[index] > right[index] {\n            return 1i32;\n        }\n        if !(left[index] <= right[index]) {\n            return 2i32;\n        }\n        index += 1;\n    }\n    if left.len() < right.len() {\n        -1i32\n    } else if left.len() > right.len() {\n        1i32\n    } else {\n        0i32\n    }\n}\n\npub fn __lkjscript_vec_order<T: PartialOrd>(left: &Vec<T>, right: &Vec<T>) -> i32 {\n    __lkjscript_slice_order(&left[..], &right[..])\n}\n\npub fn __lkjscript_slice_equal<T: PartialEq>(left: &[T], right: &[T]) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index = 0usize;\n    while index < left.len() {\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index += 1;\n    }\n    true\n}\n\n/// Split borrows the original elements and evaluates the predicate once per element.\npub struct SliceSplit<'a, T, F> {\n    items: &'a [T],\n    predicate: F,\n    start: usize,\n    done: bool,\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::Iterator for SliceSplit<'a, T, F> {\n    type Item = &'a [T];\n    fn next(&mut self) -> Option<&'a [T]> {\n        if self.done {\n            return Option::None;\n        }\n        let items = self.items;\n        let start = self.start;\n        let mut end = start;\n        while end < items.len() {\n            if (self.predicate)(&items[end]) {\n                self.start = end + 1;\n                return Option::Some(&items[start..end]);\n            }\n            end += 1;\n        }\n        self.done = true;\n        Option::Some(&items[start..])\n    }\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::IntoIterator for SliceSplit<'a, T, F> {\n    type IntoIter = Self;\n    fn into_iter(self) -> Self {\n        self\n    }\n}\npub trait SliceSplitSource<T> {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F>;\n}\nimpl<T> SliceSplitSource<T> for [T] {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F> {\n        SliceSplit {\n            items: self,\n            predicate,\n            start: 0,\n            done: false,\n        }\n    }\n}\n")),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("fn probe(){let values:Vec<i64>=Vec::new();values.clone();}")),
], "E_TYPE", "$std/sequence.rs", 0, 0), errors);
        count += 1;
    }
    if 257usize % workers == shard {
        record(source_case("kernel_refusal_and_unrepresentable_allocation_report_heap_failure_0000", "runtime.rs", None, vec![
source_file("runtime.rs", Some("//! Source allocator over sealed raw addresses. Ordinary application stubs trap.\n//! Allocator metadata is private to this runtime; programs receive payload pointers.\n\npub fn raw_load8(_address: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_load64(_address: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store8(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store64(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_memory_copy(_destination: u64, _source: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_map(_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_unmap(_address: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_remap(_address: u64, _old_bytes: u64, _new_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_allocator_state() -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_fail() -> u64 { panic!(\"unsealed runtime primitive\") }\n\nfn add_extent(left: u64, right: u64) -> u64 {\n    if left > 18446744073709551615 - right {\n        return raw_fail();\n    }\n    left + right\n}\n\nfn payload_class(bytes: u64) -> u64 {\n    let mut size: u64 = 16;\n    let mut class: u64 = 0;\n    while size < bytes {\n        if size > 9223372036854775807 {\n            return raw_fail();\n        }\n        size = size * 2;\n        class = class + 1;\n    }\n    class\n}\n\n// Free-list heads occupy the first 64 words of allocator state. The next three\n// words are the current region's cursor, remaining bytes and next region size.\n// Regions grow geometrically; a refused request is retried down to the pages\n// needed by this allocation. Retained regions supply slabs and medium blocks.\nfn take_region(bytes: u64) -> u64 {\n    let state = raw_allocator_state();\n    let mut cursor = raw_load64(state + 512);\n    let mut remaining = raw_load64(state + 520);\n    if remaining < bytes {\n        let minimum = add_extent(bytes, 4095) & !4095u64;\n        let mut size = raw_load64(state + 528);\n        if size == 0 {\n            size = 65536;\n        }\n        if size < minimum {\n            size = minimum;\n        }\n        cursor = raw_map(size);\n        while cursor == 0 && size > minimum {\n            size = (size / 2) & !4095u64;\n            if size < minimum {\n                size = minimum;\n            }\n            cursor = raw_map(size);\n        }\n        if cursor == 0 {\n            return raw_fail();\n        }\n        remaining = size;\n        let mut next = size;\n        if next <= 9223372036854775807 {\n            next = next * 2;\n        }\n        raw_store64(state + 528, next);\n    }\n    raw_store64(state + 512, cursor + bytes);\n    raw_store64(state + 520, remaining - bytes);\n    cursor\n}\n\n// Each block has {class, next_free} followed by a power-of-two payload.\n// Payloads through 1 MiB are reused by class. Small blocks are carved from\n// 4096-byte slabs inside regions; medium blocks are carved directly. Larger\n// blocks have independent mappings so growth can remap without copying.\npub fn allocate(bytes: u64) -> u64 {\n    let class = payload_class(bytes);\n    let size = 16u64 << class;\n    let block_size = add_extent(size, 16);\n    if size > 1048576 {\n        let block = raw_map(block_size);\n        if block == 0 {\n            return raw_fail();\n        }\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let head = raw_allocator_state() + class * 8;\n    let old = raw_load64(head);\n    if old != 0 {\n        raw_store64(head, raw_load64(old + 8));\n        raw_store64(old + 8, 0);\n        return old + 16;\n    }\n    if block_size > 4096 {\n        let block = take_region(block_size);\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let slab = take_region(4096);\n    let count = 4096 / block_size;\n    let mut index: u64 = count;\n    let mut next: u64 = 0;\n    while index != 0 {\n        index = index - 1;\n        let block = slab + index * block_size;\n        raw_store64(block, class);\n        raw_store64(block + 8, next);\n        next = block;\n    }\n    raw_store64(head, raw_load64(slab + 8));\n    raw_store64(slab + 8, 0);\n    slab + 16\n}\n\npub fn release(pointer: u64) -> u64 {\n    if pointer == 0 {\n        return 0;\n    }\n    let block = pointer - 16;\n    let class = raw_load64(block);\n    let size = 16u64 << class;\n    if size > 1048576 {\n        if raw_unmap(block, add_extent(size, 16)) != 0 {\n            return raw_fail();\n        }\n        return 0;\n    }\n    let head = raw_allocator_state() + class * 8;\n    raw_store64(block + 8, raw_load64(head));\n    raw_store64(head, block);\n    0\n}\n\n// No caller retains a pointer into a block whose ownership is transferred here.\n// Shrinking keeps capacity. Dedicated mappings grow with mremap; only pooled\n// blocks copy their payload when moving into a larger size class.\npub fn reallocate(pointer: u64, bytes: u64) -> u64 {\n    if pointer == 0 {\n        return allocate(bytes);\n    }\n    let block = pointer - 16;\n    let old_size = 16u64 << raw_load64(block);\n    if bytes <= old_size {\n        return pointer;\n    }\n    if old_size > 1048576 {\n        let class = payload_class(bytes);\n        let size = 16u64 << class;\n        let replacement = raw_remap(block, add_extent(old_size, 16), add_extent(size, 16));\n        if replacement == 0 {\n            return raw_fail();\n        }\n        raw_store64(replacement, class);\n        return replacement + 16;\n    }\n    let replacement = allocate(bytes);\n    raw_memory_copy(replacement, pointer, old_size);\n    release(pointer);\n    replacement\n}\n\npub fn valid_utf8_bytes(data: u64, length: u64) -> u64 {\n    let mut index: u64 = 0;\n    let mut remaining: u64 = 0;\n    let mut low: u64 = 128;\n    let mut high: u64 = 191;\n    while index < length {\n        let byte = raw_load8(data + index);\n        if remaining != 0 {\n            if byte < low || byte > high {\n                return 0;\n            }\n            remaining = remaining - 1;\n            low = 128;\n            high = 191;\n        } else if byte >= 128 {\n            low = 128;\n            high = 191;\n            if byte >= 194 && byte <= 223 {\n                remaining = 1;\n            } else if byte >= 224 && byte <= 239 {\n                remaining = 2;\n                if byte == 224 {\n                    low = 160;\n                }\n                if byte == 237 {\n                    high = 159;\n                }\n            } else if byte >= 240 && byte <= 244 {\n                remaining = 3;\n                if byte == 240 {\n                    low = 144;\n                }\n                if byte == 244 {\n                    high = 143;\n                }\n            } else {\n                return 0;\n            }\n        }\n        index = index + 1;\n    }\n    if remaining == 0 { 1 } else { 0 }\n}\n\npub fn utf8_validate(data: u64, length: u64) -> u64 {\n    valid_utf8_bytes(data, length)\n}\n\n// Runtime probe: expected u64 result 42 when appended to the sealed runtime source.\npub fn probe_allocator() -> u64 {\n    if payload_class(0) != 0 || payload_class(16) != 0 || payload_class(17) != 1 || payload_class(9223372036854775808) != 59 { return 1; }\n    let a = allocate(8);\n    let b = allocate(32);\n    raw_store64(a, 11);\n    raw_store64(b, 22);\n    release(a);\n    let reused = allocate(8);\n    if reused != a || raw_load64(b) != 22 { return 2; }\n    release(reused);\n    release(b);\n\n    // Multiple simultaneously live slabs must remain disjoint.\n    let slots = allocate(2400);\n    let mut index: u64 = 0;\n    while index < 300 {\n        let block = allocate(16);\n        raw_store64(slots + index * 8, block);\n        raw_store64(block, index);\n        raw_store64(block + 8, index + 1000);\n        index = index + 1;\n    }\n    index = 0;\n    while index < 300 {\n        let block = raw_load64(slots + index * 8);\n        if raw_load64(block) != index || raw_load64(block + 8) != index + 1000 { return 3; }\n        release(block);\n        index = index + 1;\n    }\n    release(slots);\n\n    let mut size: u64 = 1;\n    while size <= 1048576 {\n        let block = allocate(size);\n        raw_store8(block + size - 1, 173);\n        if raw_load8(block + size - 1) != 173 { return 4; }\n        release(block);\n        size = size * 2;\n    }\n\n    let block = allocate(32);\n    index = 0;\n    while index < 16 {\n        raw_store8(block + index, index);\n        index = index + 1;\n    }\n    raw_memory_copy(block + 4, block, 12);\n    if raw_load8(block + 4) != 0 || raw_load8(block + 15) != 11 { return 5; }\n    release(block);\n\n    let left = allocate(32);\n    let guard = allocate(64);\n    raw_store64(left, 6789);\n    raw_store64(left + 24, 9876);\n    raw_store64(guard, 54321);\n    let grown = reallocate(left, 129);\n    if grown == left || raw_load64(grown - 16) != 4 { return 6; }\n    if raw_load64(grown) != 6789 || raw_load64(grown + 24) != 9876 || raw_load64(guard) != 54321 { return 7; }\n    if reallocate(grown, 17) != grown { return 8; }\n    if allocate(32) != left { return 9; }\n    release(left);\n    release(guard);\n    release(grown);\n\n    let large = allocate(2097152);\n    raw_store64(large, 12345);\n    raw_store8(large + 2097151, 201);\n    let larger = reallocate(large, 4194304);\n    if raw_load64(larger) != 12345 || raw_load8(larger + 2097151) != 201 { return 10; }\n    release(larger);\n\n    let empty = reallocate(0, 7);\n    raw_store64(empty, 42);\n    let answer = raw_load64(empty);\n    release(empty);\n    release(0);\n    answer\n}\npub fn probe_overflow() -> u64 { allocate(18446744073709551615) }\n\npub fn probe_medium_regions() -> u64 {\n    let slots = allocate(520);\n    let mut index: u64 = 0;\n    while index < 65 {\n        let block = allocate(32768);\n        raw_store64(slots + index * 8, block);\n        raw_store64(block, index);\n        raw_store64(block + 32760, index + 1000);\n        index = index + 1;\n    }\n    index = 0;\n    while index < 65 {\n        let block = raw_load64(slots + index * 8);\n        if raw_load64(block) != index || raw_load64(block + 32760) != index + 1000 {\n            return 1;\n        }\n        release(block);\n        index = index + 1;\n    }\n    release(slots);\n    42\n}\n")),
], ""), errors);
        count += 1;
    }
    if 258usize % workers == shard {
        record(source_case("known_receiver_keeps_the_source_method_selected_under_generic_parameters_0001", "case.rs", None, vec![source_file("case.rs", Some("\nstruct Wrapper<T>{value:T}\ntrait Score {fn score(&self)->i64;}\nimpl<T> Score for Wrapper<T> {fn score(&self)->i64{1}}\nimpl Wrapper<i64> {fn score(&self)->i64{2}}\nfn scored<T>(value:&Wrapper<T>)->i64{value.score()}\npub fn selected()->i64{scored(&Wrapper{value:0i64})}\npub fn direct()->i64{Wrapper{value:0i64}.score()}\n"))], ""), errors);
        count += 1;
    }
    if 259usize % workers == shard {
        record(source_case("language__ir_text__tests__arrays_slices_views_and_destructors_round_trip_0001", "case.rs", None, vec![source_file("case.rs", Some("struct Guard<'a>{ count:&'a mut i64 } impl<'a> Drop for Guard<'a>{ fn drop(&mut self){*self.count=*self.count+1;} } fn first(values:&[u8])->&u8 { &values[0] } pub fn probe()->i64 { let mut count:i64=0; {let _guard=Guard{count:&mut count};} let bytes:[u8;2]=[20,22]; count+*first(&bytes) as i64 }"))], ""), errors);
        count += 1;
    }
    if 260usize % workers == shard {
        record(source_case("language__ir_text__tests__source_round_trip_preserves_meaning_and_locations_0000", "case.rs", None, vec![source_file("case.rs", Some("struct Pair { left:i64, right:i64 } pub fn probe()->i64 { let p=Pair {left:20,right:22}; p.left+p.right }"))], ""), errors);
        count += 1;
    }
    if 261usize % workers == shard {
        record(source_case("lhs_borrow_is_live_while_rhs_is_evaluated_0000", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{number:i64} impl PartialEq for Value{fn eq(&self,other:&Self)->bool{self.number==other.number}} fn change(value:&mut Value)->Value{value.number=2;Value{number:2}} fn bad()->bool{let mut value=Value{number:1};value==change(&mut value)}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 262usize % workers == shard {
        record(source_case("live_element_borrows_prevent_owner_replacement_0000", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("fn bad()->i64{let mut values:Vec<Box<i64>>=Vec::new();values.push(Box::new(1i64));let view=&values[0];values[0]=Box::new(42);**view}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 263usize % workers == shard {
        record(source_case("live_element_borrows_prevent_owner_replacement_0001", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("fn bad()->i64{let mut value=Box::new(1i64);let view=&*value;*(&mut value)=Box::new(42);*view}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 264usize % workers == shard {
        record(source_case("loop_conditions_clean_up_projected_indexes_and_owned_receivers_0000", "derived_operations.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_operations.rs", Some("\nstruct Row{values:Vec<i64>}\nfn at(calls:&mut i64)->usize{*calls=*calls+1;0}\npub fn projected_loop()->i64{\n let mut rows=vec![Row{values:vec![1i64,2,3]}];\n let mut index:usize=0;let mut calls:i64=0;\n while index<rows[at(&mut calls)].values.len(){\n  rows[0].values[index]=rows[0].values[index]+1;\n  index=index+1;\n }\n if calls!=4 || rows[0].values[2]!=4{return 1;}\n let mut count:usize=0;\n while rows[0].values.clone().len()>count {\n  count=count+1;\n  if count==2 {continue;}\n }\n if count!=3 || rows[0].values.len()!=3{return 2;}\n let mut count:usize=0;\n while rows[0].values.clone().len()>count {\n  count=count+1;\n  if count==2 {break;}\n }\n if count!=2{return 3;}\n 0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 265usize % workers == shard {
        record(source_case("method_type_parameters_remain_rigid_at_use_sites_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Mapper{fn take<U>(&self,x:U)->U;} fn good<T:Mapper,U>(x:&T,y:U)->U{x.take(y)}"))], ""), errors);
        count += 1;
    }
    if 266usize % workers == shard {
        record(source_case("missing_library_input_is_not_replaced_with_embedded_source_0000", "main.rs", Some("library/lib.rs"), vec![
source_file("main.rs", Some("mod child; pub fn main(mut world:World) { let mut out=world.stdout(); out.write(&[child::answer() as u8]); }")),
source_file("child.rs", Some("pub fn answer()->i64 { std::answer() }")),
], "E_MODULE_MISSING"), errors);
        count += 1;
    }
    if 267usize % workers == shard {
        record(source_case("mixed_affine_moves_and_explicit_references_preserve_disjoint_places_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:Box<i64>,right:Box<i64>}} fn read(value:E)->i64{match value{E::Named{left,ref right}=>*left+**right}}")),
], ""), errors);
        count += 1;
    }
    if 268usize % workers == shard {
        record(source_case("mixed_affine_moves_and_explicit_references_preserve_disjoint_places_0001", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:Box<i64>,right:Box<i64>}} struct Holder{value:E} fn read(holder:Holder)->i64{match holder.value{E::Named{left,ref right}=>*left+**right}}")),
], ""), errors);
        count += 1;
    }
    if 269usize % workers == shard {
        record(source_case("model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:i8)->i8{-x}")),
], ""), errors);
        count += 1;
    }
    if 270usize % workers == shard {
        record(source_case("model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths_0001", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:i16)->i16{x / -1i16}")),
], ""), errors);
        count += 1;
    }
    if 271usize % workers == shard {
        record(source_case("model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths_0002", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:i32)->i32{x % -1i32}")),
], ""), errors);
        count += 1;
    }
    if 272usize % workers == shard {
        record(source_case("model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths_0003", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:i64)->i64{x / -1i64}")),
], ""), errors);
        count += 1;
    }
    if 273usize % workers == shard {
        record(source_case("model_float_negation_preserves_payload_bits_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("pub fn probe() -> i64 {\n    let values = vec![\n        0u64,\n        9223372036854775808u64,\n        9221120237041090561u64,\n        18444492273895866369u64,\n        9218868437227405313u64,\n    ];\n    for bits in values {\n        let value = f64::from_bits(bits);\n        assert_eq!(value.to_bits(), bits);\n        assert_eq!((-value).to_bits(), bits ^ (1u64 << 63));\n    }\n    42\n}\n")),
], ""), errors);
        count += 1;
    }
    if 274usize % workers == shard {
        record(source_case("model_host_failure_keeps_previous_output_and_has_its_own_trap_kind_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("pub fn main(mut world:World){let mut out=world.stdout();out.write(\"a\".as_bytes());out.write(\"b\".as_bytes());}")),
], ""), errors);
        count += 1;
    }
    if 275usize % workers == shard {
        record(source_case("model_output_preserves_bytes_drop_order_and_trap_prefix_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn write(out: &mut Stdout, bytes: &[u8]) { out.write(bytes); }\npub fn main(mut world: World) {\n    let writer: Stdout = world.stdout();\n    let mut out: Stdout = writer;\n    out.write(\"Hello, 世界!\\n\".as_bytes());\n    out.write(\"\".as_bytes());\n    let bytes: [u8; 4] = [0u8, 255u8, 13u8, 10u8];\n    write(&mut out, bytes.as_slice());\n    let mut values: Vec<u8> = Vec::new();\n    let mut i: usize = 0;\n    while i < 256 { values.push(i as u8); i = i + 1; }\n    out.write(values.as_slice());\n}\n")),
], ""), errors);
        count += 1;
    }
    if 276usize % workers == shard {
        record(source_case("model_output_preserves_bytes_drop_order_and_trap_prefix_0001", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("struct Mark<'a> { out: &'a mut Stdout, byte: u8 }\nimpl<'a> Drop for Mark<'a> {\n    fn drop(&mut self) {\n        let bytes: [u8; 1] = [self.byte];\n        self.out.write(bytes.as_slice());\n    }\n}\npub fn main(mut world: World) {\n    let mut out: Stdout = world.stdout();\n    out.write(\"a\".as_bytes());\n    { let mark: Mark<'_> = Mark { out: &mut out, byte: 98u8 }; }\n    out.write(\"c\\n\".as_bytes());\n}\n")),
], ""), errors);
        count += 1;
    }
    if 277usize % workers == shard {
        record(source_case("model_output_preserves_bytes_drop_order_and_trap_prefix_0002", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn writer(mut world: World) -> Stdout { world.stdout() }\npub fn main(world: World) {\n    let mut out: Stdout = writer(world);\n    out.write(\"derived\\n\".as_bytes());\n}\n")),
], ""), errors);
        count += 1;
    }
    if 278usize % workers == shard {
        record(source_case("model_output_preserves_bytes_drop_order_and_trap_prefix_0003", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("pub fn main(mut world: World) {\n    let mut out: Stdout = world.stdout();\n    out.write(\"before trap\\n\".as_bytes());\n    assert!(false);\n}\n")),
], ""), errors);
        count += 1;
    }
    if 279usize % workers == shard {
        record(source_case("model_reports_program_traps_separately_from_machine_errors_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:i8)->i8{x+1i8}")),
], ""), errors);
        count += 1;
    }
    if 280usize % workers == shard {
        record(source_case("model_reports_program_traps_separately_from_machine_errors_0001", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:i64)->i64{42/x}")),
], ""), errors);
        count += 1;
    }
    if 281usize % workers == shard {
        record(source_case("model_reports_program_traps_separately_from_machine_errors_0002", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:usize)->i64{let a:[i64;1]=[1];a[x]}")),
], ""), errors);
        count += 1;
    }
    if 282usize % workers == shard {
        record(source_case("model_reports_program_traps_separately_from_machine_errors_0003", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn probe(x:i64)->i64{assert!(x==0);x}")),
], ""), errors);
        count += 1;
    }
    if 283usize % workers == shard {
        record(source_case("model_runs_custom_drop_in_source_order_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("\n        struct Counter<'a> { count: &'a mut i64 }\n        impl<'a> Drop for Counter<'a> {\n            fn drop(&mut self) { *self.count = *self.count * 10 + 1; }\n        }\n        struct Parent<'a> { child: Counter<'a> }\n        fn returning(count: &mut i64) -> i64 { let counter = Counter { count }; return 7; }\n        fn breaking(count: &mut i64) { loop { let counter = Counter { count }; break; } }\n        fn probe() -> i64 {\n            let mut count: i64 = 0;\n            let value = returning(&mut count);\n            breaking(&mut count);\n            { let parent = Parent { child: Counter { count: &mut count } }; }\n            count + value\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 284usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("pub fn probe() -> i64 { (7i64 * 8 - 14) / 1 }\n")),
], ""), errors);
        count += 1;
    }
    if 285usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0001", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn fib(n: i64) -> i64 {\n    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }\n}\npub fn probe() -> i64 { fib(10) }\n")),
], ""), errors);
        count += 1;
    }
    if 286usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0002", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("pub fn probe() -> i64 {\n    let mut i: i64 = 0;\n    let mut sum: i64 = 0;\n    loop {\n        i = i + 1;\n        if i > 10 { break; }\n        if i % 2 == 0 { continue; }\n        sum = sum + i;\n    }\n    sum\n}\n")),
], ""), errors);
        count += 1;
    }
    if 287usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0003", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("struct Pair { left: i64, right: i64 }\npub fn probe() -> i64 {\n    let mut pair = Pair { left: 0, right: 0 };\n    let left = &mut pair.left;\n    let right = &mut pair.right;\n    *left = 20;\n    *right = 22;\n    pair.left + pair.right\n}\n")),
], ""), errors);
        count += 1;
    }
    if 288usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0004", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("struct Pair { left: Box<i64>, right: Box<i64> }\npub fn probe() -> i64 {\n    let mut pair = Pair { left: Box::new(10i64), right: Box::new(20i64) };\n    let first = pair.left;\n    pair.left = Box::new(12i64);\n    *first + *pair.left + *pair.right\n}\n")),
], ""), errors);
        count += 1;
    }
    if 289usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0005", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("enum List { End, Next(i64, Box<List>) }\nfn sum(list: &List) -> i64 {\n    match list { List::End => 0, List::Next(value, next) => *value + sum(next) }\n}\npub fn probe() -> i64 {\n    let list = List::Next(20, Box::new(List::Next(22, Box::new(List::End))));\n    sum(&list)\n}\n")),
], ""), errors);
        count += 1;
    }
    if 290usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0006", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("pub fn probe() -> i64 {let mut v:Vec<i64>=vec![10,20];v.push(12);let out=v.pop().unwrap();v[0]+v[1]+out}")),
], ""), errors);
        count += 1;
    }
    if 291usize % workers == shard {
        record(source_case("model_runs_values_moves_borrows_collections_and_source_library_0007", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("pub fn probe() -> i64 {let mut s=String::from_str(\"ab\");s.push_str(\"c\");let t=s.clone();t.len() as i64}")),
], ""), errors);
        count += 1;
    }
    if 292usize % workers == shard {
        record(source_case("model_uses_an_explicit_stack_for_non_tail_calls_and_deep_cleanup_0000", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("fn recurse(n:i64)->i64 { if n==0 {0} else { let value=recurse(n-1);value+1 } } pub fn probe()->i64 {recurse(20000)}")),
], ""), errors);
        count += 1;
    }
    if 293usize % workers == shard {
        record(source_case("model_uses_an_explicit_stack_for_non_tail_calls_and_deep_cleanup_0001", "interpreter.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("interpreter.rs", Some("enum List {End,Next(Box<List>)} pub fn probe()->i64 {let mut v=List::End;let mut i:i64=0;while i<20000 {v=List::Next(Box::new(v));i=i+1;}i}")),
], ""), errors);
        count += 1;
    }
    if 294usize % workers == shard {
        record(source_case("named_construction_matching_and_cleanup_match_the_same_rust_sources_0000", "main.rs", None, vec![
source_file("main.rs", Some("\nmod a;\nmod b;\nuse crate::a::Packet as Imported;\ntype PacketAlias = a::Packet;\n\nenum Basic { Empty, Pair { left: i64, right: i64 }, NamedEmpty {}, TupleEmpty() }\nfn record(order: &mut i64, value: i64) -> i64 {\n    *order = *order * 10 + value;\n    value\n}\npub fn evaluation_order() -> i64 {\n    let mut order: i64 = 0;\n    let value = Basic::Pair { right: record(&mut order, 2), left: record(&mut order, 1) };\n    match value {\n        Basic::Pair { right, left: renamed } => order * 100 + renamed * 10 + right,\n        _ => 0\n    }\n}\npub fn rest_and_mutable_binding() -> i64 {\n    let value = Basic::Pair { left: 1, right: 40 };\n    match value { Basic::Pair { mut right, .. } => { right = right + 2; right }, _ => 0 }\n}\npub fn named_empty() -> i64 {\n    let value = Basic::NamedEmpty {};\n    match value { Basic::NamedEmpty {} => 42, _ => 0 }\n}\npub fn empty_brace_forms() -> i64 {\n    let unit = Basic::Empty {};\n    let tuple = Basic::TupleEmpty {};\n    let first = match unit { Basic::Empty {} => 20i64, _ => 0 };\n    let second = match tuple { Basic::TupleEmpty {} => 22i64, _ => 0 };\n    first + second\n}\nenum Outcome<T> { Ready { value: T, count: i64 }, Waiting }\nfn wrap<T>(value: T) -> Outcome<T> { Outcome::Ready { count: 2, value } }\npub fn inferred_generic() -> i64 {\n    let inferred = Outcome::Ready { count: 1, value: Box::new(40i64) };\n    let extra = wrap(Box::new(2i64));\n    let left = match inferred { Outcome::Ready { value, .. } => *value, Outcome::Waiting => 0 };\n    let right = match extra { Outcome::Ready { value, count: _ } => *value, Outcome::Waiting => 0 };\n    left + right\n}\npub fn scoped_aliases() -> i64 {\n    let first: Imported = Imported::Data { payload: Box::new(5) };\n    let second = b::Packet::Data { payload: Box::new(35i64) };\n    let third = PacketAlias::Data { payload: Box::new(0) };\n    let extra = match third { PacketAlias::Data { payload } => *payload + 1 };\n    a::take(first) + b::take(second) + extra\n}\nenum Repeated<T> { Named { first: T, second: T } }\npub fn repeated_generic_field_inference() -> i64 {\n    let value = Repeated::Named { first: 1, second: 41i64 };\n    match value { Repeated::Named { first, second } => first + second }\n}\nenum Store { Empty, Data { left: Box<i64>, right: Box<i64> } }\nfn read_store(value: &Store) -> i64 {\n    match value { Store::Data { right, left } => **left + **right, Store::Empty => 0 }\n}\nfn change_store(value: &mut Store) {\n    match value {\n        Store::Data { right, left } => { **left = 7; **right = 35; },\n        Store::Empty => {}\n    }\n}\npub fn shared_and_mutable_borrows() -> i64 {\n    let mut value = Store::Data { right: Box::new(1), left: Box::new(2) };\n    change_store(&mut value);\n    read_store(&value)\n}\nstruct Container { value: Store }\npub fn projected_mutable_field_match() -> i64 {\n    let mut holder = Container { value: Store::Data { left: Box::new(1), right: Box::new(2) } };\n    match holder.value {\n        Store::Data { ref mut left, ref mut right } => { **left = 7; **right = 35; },\n        Store::Empty => {}\n    }\n    read_store(&holder.value)\n}\nstruct Measured {}\nimpl Measured { fn len(&self) -> i64 { 42 } }\npub fn inferred_method_result() -> i64 {\n    let value = Outcome::Ready { value: Measured {}.len(), count: 0 };\n    match value { Outcome::Ready { value, .. } => value, Outcome::Waiting => 0 }\n}\nstruct Counter<'a> { count: &'a mut i64, payload: Box<i64> }\nimpl<'a> Drop for Counter<'a> {\n    fn drop(&mut self) { *self.count = *self.count + 1; }\n}\nenum Owned<'a> { Empty, Named { kept: Box<i64>, discarded: Counter<'a> } }\nfn extract(value: Owned<'_>) -> Box<i64> {\n    match value { Owned::Named { kept, .. } => kept, Owned::Empty => Box::new(0) }\n}\npub fn partial_move_cleanup() -> i64 {\n    let mut count: i64 = 0;\n    let kept = extract(Owned::Named {\n        discarded: Counter { count: &mut count, payload: Box::new(0) },\n        kept: Box::new(40)\n    });\n    *kept + count * 2\n}\nenum Observed<'a> { Named { value: i64, observed: &'a mut i64 } }\nimpl<'a> Drop for Observed<'a> {\n    fn drop(&mut self) {\n        match self { Observed::Named { value, observed } => { **observed = *value; } }\n    }\n}\npub fn explicit_mutable_field_binding() -> i64 {\n    let mut observed: i64 = 0;\n    {\n        let mut value = Observed::Named { observed: &mut observed, value: 1 };\n        match value { Observed::Named { ref mut value, .. } => { *value = 42; } }\n    }\n    observed\n}\nenum Guarded { Named { payload: Box<i64> } }\nimpl Drop for Guarded { fn drop(&mut self) {} }\npub fn explicit_shared_affine_field_binding() -> i64 {\n    let value = Guarded::Named { payload: Box::new(21) };\n    let first = match value { Guarded::Named { ref payload } => **payload };\n    let second = match value { Guarded::Named { ref payload } => **payload };\n    first + second\n}\nenum View<'a> { Empty, Named { label: i64, value: &'a i64 } }\nfn view<'a>(value: &'a i64) -> View<'a> { View::Named { value, label: 2 } }\npub fn borrowed_view_transport() -> i64 {\n    let number: i64 = 40;\n    let wrapped = view(&number);\n    match wrapped { View::Named { value, label } => *value + label, View::Empty => 0 }\n}\n")),
source_file("a.rs", Some("\npub enum Packet { Data { payload: Box<i64> } }\npub fn take(value: Packet) -> i64 {\n    match value { Packet::Data { payload } => *payload + 1 }\n}\n")),
source_file("b.rs", Some("\npub enum Packet { Data { payload: Box<i64> } }\npub fn take(value: Packet) -> i64 {\n    match value { Packet::Data { payload } => *payload }\n}\n")),
], ""), errors);
        count += 1;
    }
    if 295usize % workers == shard {
        record(source_case("named_constructor_field_sets_are_checked_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:i64,right:i64}} fn bad()->E{E::Named{left:1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 296usize % workers == shard {
        record(source_case("named_constructor_field_sets_are_checked_0001", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:i64,right:i64}} fn bad()->E{E::Named{left:1,right:2,extra:3}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 297usize % workers == shard {
        record(source_case("named_constructor_field_sets_are_checked_0002", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:i64,right:i64}} fn bad()->E{E::Named{left:1,left:2,right:3}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 298usize % workers == shard {
        record(source_case("named_pattern_field_sets_are_checked_even_after_a_default_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:i64,right:i64}} fn bad(value:E)->i64{match value{_=>0,E::Named{left}=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 299usize % workers == shard {
        record(source_case("named_pattern_field_sets_are_checked_even_after_a_default_0001", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:i64,right:i64}} fn bad(value:E)->i64{match value{_=>0,E::Named{left,right,extra}=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 300usize % workers == shard {
        record(source_case("named_pattern_field_sets_are_checked_even_after_a_default_0002", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{left:i64,right:i64}} fn bad(value:E)->i64{match value{_=>0,E::Named{left:a,left:b,right}=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 301usize % workers == shard {
        record(source_case("named_patterns_cannot_move_fields_through_borrowed_or_drop_owners_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{payload:Box<i64>}} fn bad(value:&E)->Box<i64>{match value{E::Named{payload}=>*payload}}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 302usize % workers == shard {
        record(source_case("named_patterns_cannot_move_fields_through_borrowed_or_drop_owners_0001", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{payload:Box<i64>}} impl Drop for E{fn drop(&mut self){}} fn bad(value:E)->Box<i64>{match value{E::Named{payload}=>payload}}")),
], "E_MOVE"), errors);
        count += 1;
    }
    if 303usize % workers == shard {
        record(source_case("named_payload_reference_provenance_cannot_escape_a_local_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum View<'a>{Named{label:i64,value:&'a i64}} fn bad<'a>(external:&'a i64)->View<'a>{let local:i64=7;View::Named{value:&local,label:42}}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 304usize % workers == shard {
        record(source_case("narrow_results_and_borrowed_aliases_keep_storage_semantics_0000", "narrow-alias", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("narrow-alias", Some("\n        fn change(value:&mut i8) { *value = value.wrapping_add(17i8); }\n        pub fn probe()->i64 {\n            let a = 250u8;\n            let b = a.wrapping_add(14u8);\n            let mut c = b as i8 - 12i8;\n            change(&mut c);\n            let d = -c;\n            let e = !d;\n            let f = 65530u16.wrapping_add(20u16);\n            let g = -300i16;\n            let h = (g >> 2) as i8;\n            c as i64 + d as i64 + e as i64 + f as i64 + h as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 305usize % workers == shard {
        record(source_case("nested_collections_and_recursive_nominals_reuse_checked_helpers_0000", "derived_operations.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("derived_operations.rs", Some("\n#[derive(Clone,PartialEq,Eq)] enum List{End,Next(i64,Box<List>)}\n#[derive(Clone,PartialEq,Eq)] struct Graph{lists:Vec<Vec<List>>,label:String}\nfn cloned<T:Clone>(value:&T)->T{value.clone()}\npub fn recursive()->i64{\n let chain=List::Next(1,Box::new(List::Next(2,Box::new(List::End))));\n let mut lists:Vec<Vec<List>>=Vec::new(); lists.push(vec![chain]);\n let mut label=String::new();label.push_str(\"graph\");\n let original=Graph{lists,label};\n let copied=cloned(&original);\n if original!=copied{return 1;}\n drop(original);\n let mut changed=Graph{lists:Vec::new(),label:String::new()};\n changed.clone_from(&copied);\n if changed!=copied || changed.label.len()!=5{return 2;}\n 0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 306usize % workers == shard {
        record(source_case("nested_effectful_targets_reuse_their_checked_type_context_0000", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("fn slot<'a>(value:&'a mut u8)->&'a mut u8{value}pub fn nested_2()->i64{let mut value:u8=0;*{*{*slot(&mut value)=1;slot(&mut value)}=1;slot(&mut value)}=42;value as i64}pub fn nested_4()->i64{let mut value:u8=0;*{*{*{*{*slot(&mut value)=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=42;value as i64}pub fn nested_8()->i64{let mut value:u8=0;*{*{*{*{*{*{*{*{*slot(&mut value)=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=1;slot(&mut value)}=42;value as i64}")),
], ""), errors);
        count += 1;
    }
    if 307usize % workers == shard {
        record(source_case("nested_loops_keep_carried_locals_through_breaks_and_branches_0000", "loop-carried", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("loop-carried", Some("\n        pub fn probe() -> i64 {\n            let mut sum = 0i64;\n            let mut outer = 0i64;\n            while outer < 11 {\n                let mut inner = 0i64;\n                let carried = outer * 7;\n                while inner < 9 {\n                    inner += 1;\n                    if inner % 3 == 0 { continue; }\n                    sum += carried + inner;\n                    if sum > 400 { break; }\n                }\n                outer += 1;\n            }\n            sum\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 308usize % workers == shard {
        record(source_case("nested_partial_alias_expectation_propagates_before_literal_defaults_0000", "main.rs", None, vec![
source_file("main.rs", Some("\nstruct Pair<A,B>{first:A,second:B}\nstruct Holder<T>{value:T}\ntype Alias<T> = Holder<Pair<T,u8>>;\npub fn probe()->i64 {\n    let value=Alias{value:Pair{first:40i64,second:2}};\n    value.value.first+value.value.second as i64\n}\n")),
], ""), errors);
        count += 1;
    }
    if 309usize % workers == shard {
        record(source_case("nested_partial_alias_expectation_propagates_before_literal_defaults_0002", "main.rs", None, vec![
source_file("main.rs", Some("\nstruct Pair<A,B>{first:A,second:B}\nstruct Holder<T>{value:T}\ntype Alias<T> = Holder<Pair<T,u8>>;\npub fn probe()->i64 {\n    let value=Alias::<i64>{value:Pair{first:40i64,second:2}};\n    value.value.first+value.value.second as i64\n}\n")),
], ""), errors);
        count += 1;
    }
    if 310usize % workers == shard {
        record(source_case("nested_partial_alias_expectation_propagates_before_literal_defaults_0004", "main.rs", None, vec![
source_file("main.rs", Some("\nstruct Pair<A,B>{first:A,second:B}\nstruct Holder<T>{value:T}\ntype Alias<T> = Holder<Pair<T,u8>>;\npub fn probe()->i64 {\n    let value=Alias{value:Pair{first:40i64,second:2u8}};\n    value.value.first+value.value.second as i64\n}\n")),
], ""), errors);
        count += 1;
    }
    if 311usize % workers == shard {
        record(source_case("nominal_and_alias_type_arguments_require_their_declared_bounds_0000", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T:Copy>{value:T} fn bad<U>(x:U)->Holder<U>{Holder{value:x}}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 312usize % workers == shard {
        record(source_case("nominal_and_alias_type_arguments_require_their_declared_bounds_0001", "case.rs", None, vec![source_file("case.rs", Some("type Holder<T:Copy>=(T,); fn bad<U>(x:U)->Holder<U>{(x,)}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 313usize % workers == shard {
        record(source_case("nominal_and_alias_type_arguments_require_their_declared_bounds_0003", "case.rs", None, vec![source_file("case.rs", Some("struct Holder<T:Copy>{value:T} fn good<U:Copy>(x:U)->Holder<U>{Holder{value:x}}"))], ""), errors);
        count += 1;
    }
    if 314usize % workers == shard {
        record(source_case("noninteger_range_patterns_are_rejected_by_type_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:bool)->i64{match value{false..=true=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 315usize % workers == shard {
        record(source_case("noninteger_range_patterns_are_rejected_by_type_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:f64)->i64{match value{0.0..=1.0=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 316usize % workers == shard {
        record(source_case("opaque_types_do_not_gain_automatic_traits_0001", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Data>(x:T){} fn bad<U>(x:U){need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 317usize % workers == shard {
        record(source_case("opaque_types_do_not_gain_automatic_traits_0003", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Data>(x:T){} fn good<U:Data>(x:U){need(x);}"))], ""), errors);
        count += 1;
    }
    if 318usize % workers == shard {
        record(source_case("opaque_types_do_not_gain_automatic_traits_0004", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Send>(x:T){} fn bad<U>(x:U){need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 319usize % workers == shard {
        record(source_case("opaque_types_do_not_gain_automatic_traits_0005", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Send>(x:T){} fn good<U:Send>(x:U){need(x);}"))], ""), errors);
        count += 1;
    }
    if 320usize % workers == shard {
        record(source_case("opaque_types_do_not_gain_automatic_traits_0006", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Sync>(x:T){} fn bad<U>(x:U){need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 321usize % workers == shard {
        record(source_case("opaque_types_do_not_gain_automatic_traits_0007", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Sync>(x:T){} fn good<U:Sync>(x:U){need(x);}"))], ""), errors);
        count += 1;
    }
    if 322usize % workers == shard {
        record(source_case("opaque_types_do_not_gain_automatic_traits_0009", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Data>(x:T){} fn bad<U:Copy>(x:U){need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 323usize % workers == shard {
        record(source_case("option_and_result_own_affine_payloads_0000", "probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("probe.rs", Some("use std::bytes::*; use std::collections::BTreeMap;\n\n        fn prelude_probe() -> i64 {\n            let mut present: Option<Box<i64>> = Option::Some(Box::new(17));\n            assert!(present.is_some());\n            let held = present.take();\n            assert!(present.is_none());\n            let value = held.unwrap();\n            let absent: Option<i64> = Option::None;\n            let fallback = absent.unwrap_or(5);\n            let result: Result<i64, i64> = Result::Ok(*value + fallback);\n            assert!(result.is_ok());\n            result.unwrap()\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 324usize % workers == shard {
        record(source_case("ordered_map_inserts_replaces_and_removes_affine_values_0000", "probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("probe.rs", Some("use std::bytes::*; use std::collections::BTreeMap;\n\n        fn map_probe() -> i64 {\n            let mut map: BTreeMap<i64, Box<i64>> = BTreeMap::new();\n            assert!(map.insert(4, Box::new(40)).is_none());\n            assert!(map.insert(1, Box::new(10)).is_none());\n            assert!(map.insert(3, Box::new(30)).is_none());\n            let previous = map.insert(3, Box::new(33)).unwrap();\n            assert!(map.contains_key(&1));\n            assert!(!map.contains_key(&2));\n            let observed = { let found = map.get(&3).unwrap(); **found };\n            { let found = map.get_mut(&4).unwrap(); **found = 44; }\n            let changed = { let found = map.get(&4).unwrap(); **found };\n            let removed = map.remove(&1).unwrap();\n            *previous + *removed + map.len() as i64 + observed + changed\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 325usize % workers == shard {
        record(source_case("ordinary_vector_methods_include_builtin_traits_in_scope_resolution_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("trait Other{fn clone(&self)->Vec<i64>;} impl Other for Vec<i64>{fn clone(&self)->Vec<i64>{Vec::new()}} fn probe(){let values:Vec<i64>=Vec::new();values.clone();}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 326usize % workers == shard {
        record(source_case("ordinary_vector_methods_include_builtin_traits_in_scope_resolution_0001", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("trait Clone{fn clone(&self)->Vec<i64>;} impl Clone for Vec<i64>{fn clone(&self)->Vec<i64>{Vec::new()}} fn probe(){let values:Vec<i64>=Vec::new();values.clone();}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 327usize % workers == shard {
        record(source_case("ordinary_vector_methods_include_builtin_traits_in_scope_resolution_0002", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("trait Other{fn eq(&self,other:&Self)->bool;} impl Other for Vec<i64>{fn eq(&self,other:&Self)->bool{false}} fn probe(){let values:Vec<i64>=Vec::new();values.eq(&values);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 328usize % workers == shard {
        record(source_case("ordinary_vector_methods_include_builtin_traits_in_scope_resolution_0003", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("trait PartialEq{fn eq(&self,other:&Self)->bool;} impl PartialEq for Vec<i64>{fn eq(&self,other:&Self)->bool{false}} fn probe(){let values:Vec<i64>=Vec::new();values.eq(&values);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 329usize % workers == shard {
        record(source_case("ordinary_vector_methods_include_builtin_traits_in_scope_resolution_0004", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("trait Other{fn ne(&self,other:&Self)->bool;} impl Other for Vec<i64>{fn ne(&self,other:&Self)->bool{false}} fn probe(){let values:Vec<i64>=Vec::new();values.ne(&values);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 330usize % workers == shard {
        record(source_case("ordinary_vector_methods_include_builtin_traits_in_scope_resolution_0005", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("\nmod extra {\n    pub trait Other {\n        fn clone(&self)->Vec<i64>;\n        fn eq(&self,other:&Self)->bool;\n        fn ne(&self,other:&Self)->bool;\n    }\n    impl Other for Vec<i64> {\n        fn clone(&self)->Vec<i64>{vec![9]}\n        fn eq(&self,_other:&Self)->bool{false}\n        fn ne(&self,_other:&Self)->bool{false}\n    }\n}\nfn selected<T:extra::Other>(value:&T)->i64 {\n    let copied=value.clone();\n    if value.eq(value) || value.ne(value) {return 0;}\n    copied[0]\n}\npub fn scope_probe()->i64 {\n    let values:Vec<i64>=vec![4,2];let copied=values.clone();\n    if copied!=values || !values.eq(&values) || values.ne(&values) {return 0;}\n    copied[0]*10+copied[1]+selected(&values)\n}\n")),
], ""), errors);
        count += 1;
    }
    if 331usize % workers == shard {
        record(source_case("ordinary_vector_methods_include_builtin_traits_in_scope_resolution_0007", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("\nstruct NoClone;\ntrait Other {fn clone(&self)->Vec<i64>;}\nimpl Other for Vec<NoClone>{fn clone(&self)->Vec<i64>{vec![9]}}\npub fn only_source_trait()->i64 {\n    let values:Vec<NoClone>=Vec::new();values.clone()[0]\n}\n")),
], ""), errors);
        count += 1;
    }
    if 332usize % workers == shard {
        record(source_case("owned_collections_and_partial_moves_0000", "partial", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial", Some("struct Pair { left: Box<i64>, right: Box<i64> }\npub fn probe() -> i64 {\n    let mut pair = Pair { left: Box::new(10i64), right: Box::new(20i64) };\n    let first = pair.left;\n    pair.left = Box::new(12i64);\n    *first + *pair.left + *pair.right\n}\n")),
], ""), errors);
        count += 1;
    }
    if 333usize % workers == shard {
        record(source_case("owned_collections_and_partial_moves_0001", "recursive", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("recursive", Some("enum List { End, Next(i64, Box<List>) }\nfn sum(list: &List) -> i64 {\n    match list { List::End => 0, List::Next(value, next) => *value + sum(next) }\n}\npub fn probe() -> i64 {\n    let list = List::Next(20, Box::new(List::Next(22, Box::new(List::End))));\n    sum(&list)\n}\n")),
], ""), errors);
        count += 1;
    }
    if 334usize % workers == shard {
        record(source_case("owned_collections_and_partial_moves_0002", "vector", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector", Some("struct Pair{a:Box<i64>,b:i64} pub fn probe()->i64{let mut values=Vec::new();let mut i=0i64;while i<100 {values.push(Pair{a:Box::new(i),b:i+1});i=i+1;}let last=values.pop().unwrap();*values[20].a+last.b-78}")),
], ""), errors);
        count += 1;
    }
    if 335usize % workers == shard {
        record(source_case("owned_vectors_and_exact_helpers_match_rust_with_affine_elements_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("use std::{__lkjscript_vec_clone,__lkjscript_vec_equal};\n\nfn same_bytes(actual: &String, expected: &str) -> bool {\n    let left=actual.as_bytes();let right=expected.as_bytes();\n    if left.len()!=right.len(){return false;}\n    let mut index:usize=0;\n    while index<left.len(){\n        if left[index]!=right[index]{return false;}\n        index=index+1;\n    }\n    true\n}\nfn cloned<T:Clone>(value:&T)->T{value.clone()}\nfn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\nfn same_operator<T:PartialEq>(left:&T,right:&T)->bool{*left==*right}\n\nstruct Token { phase:i64 }\nimpl Drop for Token {\n    fn drop(&mut self) {\n        assert!(self.phase==2);\n        self.phase=3;\n    }\n}\nstruct Item { value:i64, generation:i64, token:Box<Token> }\nimpl Item {\n    fn clone(&self)->Self {\n        Item{value:self.value,generation:99,token:Box::new(Token{phase:1})}\n    }\n}\nimpl Clone for Item {\n    fn clone(&self)->Self {\n        assert!(self.token.phase==1);\n        Item{value:self.value,generation:self.generation+1,token:Box::new(Token{phase:1})}\n    }\n}\nimpl PartialEq for Item {\n    fn eq(&self,other:&Self)->bool { self.value==other.value }\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool { !self.eq(other) }\n}\nimpl Eq for Item {}\nimpl Drop for Item {\n    fn drop(&mut self) {\n        assert!(self.token.phase==1);\n        self.token.phase=2;\n    }\n}\nfn item(value:i64)->Item {\n    Item{value,generation:0,token:Box::new(Token{phase:1})}\n}\n\npub fn affine_clones_have_independent_storage_and_ordered_cleanup()->i64 {\n    let mut original:Vec<Item>=Vec::new();\n    original.push(item(4));original.push(item(2));\n    let direct=original[0].clone();\n    if direct.generation!=99 {return 8;}\n    drop(direct);\n    let first=original.clone();\n    let mut second=cloned(&first);\n    if first[0].generation!=1 || first[1].generation!=1 {return 1;}\n    if second[0].generation!=2 || second[1].generation!=2 {return 2;}\n    if second[0].value!=4 || second[1].value!=2 {return 3;}\n    drop(first);\n    if original[0].token.phase!=1 || second[0].token.phase!=1 {return 4;}\n    original[0].value=7;\n    if second[0].value!=4 {return 5;}\n    drop(original);\n    let last=second.pop().unwrap();\n    if last.value!=2 || last.generation!=2 {return 6;}\n    drop(last);\n    if second[0].token.phase!=1 {return 7;}\n    0\n}\n\npub fn equality_uses_element_eq_and_preserves_owners()->i64 {\n    let mut left:Vec<Item>=Vec::new();left.push(item(4));left.push(item(2));\n    let mut right=cloned(&left);\n    if !(left==right) || left!=right {return 1;}\n    if !left.eq(&right) || left.ne(&right) {return 2;}\n    if !same(&left,&right) || different(&left,&right) {return 3;}\n    if !same_operator(&left,&right) {return 4;}\n    right[1].value=9;\n    if left==right || !(left!=right) {return 5;}\n    if same(&left,&right) || !different(&left,&right) {return 6;}\n    if !left.ne(&right) {return 7;}\n    left.push(item(8));right.pop();\n    if left.len()!=3 || right.len()!=1 {return 8;}\n    if left==right || right==left || !left.ne(&right) {return 10;}\n    if left[0].token.phase!=1 || right[0].token.phase!=1 {return 9;}\n    0\n}\n\npub fn nested_vectors_and_strings_clone_as_owned_values()->i64 {\n    let empty:Vec<String>=Vec::new();\n    if empty.clone().len()!=0 {return 5;}\n    let mut inner:Vec<String>=Vec::new();\n    inner.push(String::from_str(\"\"));inner.push(String::from_str(\"a\\0水😀\"));\n    let mut outer:Vec<Vec<String>>=Vec::new();outer.push(inner);\n    let copied=cloned(&outer);\n    if !same(&outer,&copied) || outer!=copied {return 1;}\n    outer[0][1].clear();outer[0][1].push_str(\"changed\");\n    if !same_bytes(&copied[0][1],\"a\\0水😀\") {return 2;}\n    if outer==copied || !different(&outer,&copied) {return 3;}\n    drop(outer);\n    if !same_bytes(&copied[0][0],\"\") || !same_bytes(&copied[0][1],\"a\\0水😀\") {return 4;}\n    0\n}\n\npub fn floating_vector_equality_needs_partial_eq_only()->i64 {\n    let mut left:Vec<f64>=vec![1.5,-0.0,4.25];\n    let mut right:Vec<f64>=vec![1.5,0.0,4.25];\n    if left!=right || !same(&left,&right) {return 1;}\n    let not_a_number:f64=0.0/0.0;\n    let values:Vec<f64>=vec![not_a_number];\n    if values==values || same(&values,&values) {return 2;}\n    if !values.ne(&values) || !different(&values,&values) {return 3;}\n    let empty:Vec<f64>=Vec::new();\n    if empty!=empty || !(empty==empty) {return 4;}\n    left.push(8.0);right.pop();\n    if left.len()!=4 || right.len()!=2 {return 5;}\n    0\n}\n\npub fn exact_source_helpers_match_the_ordinary_vector_api()->i64 {\n    let mut values:Vec<Item>=Vec::new();values.push(item(1));values.push(item(3));\n    let explicit=__lkjscript_vec_clone(&values);\n    let ordinary=values.clone();\n    if !__lkjscript_vec_equal(&explicit,&ordinary) {return 1;}\n    if explicit[0].generation!=ordinary[0].generation {return 2;}\n    if explicit[1].value!=3 || ordinary[0].value!=1 {return 3;}\n    drop(values);drop(ordinary);\n    if explicit[0].token.phase!=1 {return 4;}\n    0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 336usize % workers == shard {
        record(source_case("parameter_name_does_not_grant_a_trait_to_another_declaration_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("struct T(i64); trait Marker{} impl<T> Marker for (crate::T,T){} fn require<X:Marker>(value:X)->i64{7} pub fn probe()->i64{require((T(1),2i64))}")),
], ""), errors);
        count += 1;
    }
    if 337usize % workers == shard {
        record(source_case("parameter_name_does_not_grant_a_trait_to_another_declaration_0002", "scope.rs", None, vec![
source_file("scope.rs", Some("struct T(i64); trait Marker{} impl<T> Marker for (crate::T,T){} fn require<X:Marker>(value:X)->i64{7} pub fn probe()->i64{require((1i64,2i64))}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 338usize % workers == shard {
        record(source_case("parameter_scope_uses_the_original_file_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("struct T(i64); mod generic; pub fn probe()->i64{generic::identity(17i64)}")),
source_file("generic.rs", Some("pub fn identity<T>(value:T)->T{value}")),
], ""), errors);
        count += 1;
    }
    if 339usize % workers == shard {
        record(source_case("parameters_are_live_across_a_loop_at_function_entry_0000", "entry-loop", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("entry-loop", Some("\n        fn touch(value:i64)->i64 { value + 1 }\n        fn scan(mut left:i64, remembered:i64)->i64 {\n            loop {\n                if left == 0 { return remembered; }\n                let before = left + 11;\n                let after = touch(before);\n                left -= 1;\n                if after < 0 { return after; }\n            }\n        }\n        pub fn probe()->i64 { scan(8,42) }\n    ")),
], ""), errors);
        count += 1;
    }
    if 340usize % workers == shard {
        record(source_case("partial_box_cleanup_and_conditional_custom_drop_0000", "box_partial", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("box_partial", Some("struct Pair{a:Box<i64>,b:Box<i64>} pub fn probe()->i64{let value=Box::new(Pair{a:Box::new(20i64),b:Box::new(22i64)});let a=(*value).a;*a+*value.b}")),
], ""), errors);
        count += 1;
    }
    if 341usize % workers == shard {
        record(source_case("partial_box_cleanup_and_conditional_custom_drop_0001", "drop_order", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("drop_order", Some("struct Mark {\n    out: Stdout,\n    tag: u8,\n}\nimpl Drop for Mark {\n    fn drop(&mut self) {\n        self.out.write([self.tag].as_slice());\n    }\n}\nfn mark(world: &mut World) -> Mark {\n    let mut out = world.stdout();\n    out.write([65u8].as_slice());\n    Mark { out, tag: 97u8 }\n}\nfn peek(value: &Mark) {}\nstruct Capture<'a> {\n    owned: Mark,\n    counter: &'a mut i64,\n}\nfn invoke(value: &mut Capture<'_>) {\n    peek(&value.owned);\n    *value.counter = *value.counter + 1;\n}\npub fn main(mut world: World) {\n    let mut count: i64 = 0;\n    let owned = mark(&mut world);\n    let counter = &mut count;\n    let mut update = Capture { owned, counter };\n    invoke(&mut update);\n    drop(update.counter);\n    let mut out = world.stdout();\n    out.write([(count as u8) + 48u8].as_slice());\n}\n")),
], ""), errors);
        count += 1;
    }
    if 342usize % workers == shard {
        record(source_case("poisoning_reports_blocked_semantics_instead_of_invented_name_errors_0001", "case/main.rs", None, vec![
source_file("case/main.rs", Some("fn broken() { let x = ; x.absent(); } fn other() -> i64 { unknown }")),
], "E_PARSE"), errors);
        count += 1;
    }
    if 343usize % workers == shard {
        record(source_case("prelude_traits_execute_with_the_same_production_source_under_rust_0000", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("\nfn cloned<T:Clone>(value:&T)->T{value.clone()}\nfn updated<T:Clone>(target:&mut T,source:&T){target.clone_from(source);}\nfn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\nfn eq_contract<T:Eq>(value:&T){}\nfn copied_twice<T:Copy>(value:T)->T{let other=value;drop(other);value}\n\nmod misleading {\n    pub trait Clone { fn clone(&self)->Self; }\n    pub trait PartialEq { fn eq(&self,other:&Self)->bool; }\n}\nstruct Token { phase:i64 }\nimpl Drop for Token {\n    fn drop(&mut self){assert!(self.phase==2);self.phase=3;}\n}\nstruct Item { value:Box<i64>, generation:i64, token:Box<Token> }\nimpl Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:99,token:Box::new(Token{phase:1})}}\n    fn eq(&self,other:&Self)->bool{false}\n}\nimpl misleading::Clone for Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:77,token:Box::new(Token{phase:1})}}\n}\nimpl misleading::PartialEq for Item {fn eq(&self,other:&Self)->bool{false}}\nimpl Clone for Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:self.generation+1,token:Box::new(Token{phase:1})}}\n}\nimpl PartialEq for Item {\n    fn eq(&self,other:&Self)->bool{*self.value==*other.value}\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool{!(*self.value==*other.value)}\n}\nimpl Eq for Item {}\nimpl Drop for Item {fn drop(&mut self){assert!(self.token.phase==1);self.token.phase=2;}}\nfn item(value:i64)->Item{Item{value:Box::new(value),generation:0,token:Box::new(Token{phase:1})}}\n\npub fn affine_option_clone_keeps_independent_payloads()->i64{\n    let mut original=Option::Some(item(42));\n    let first=original.clone();\n    let second=cloned(&first);\n    match &mut original{Option::Some(value)=>{*value.value=7;},Option::None=>{}}\n    let second_value=second.unwrap();\n    if *second_value.value!=42 || second_value.generation!=2{return 1;}\n    drop(second_value);\n    let first_value=first.unwrap();\n    if *first_value.value!=42 || first_value.generation!=1{return 2;}\n    let direct=first_value.clone();\n    if direct.generation!=99{return 3;}\n    drop(direct);drop(first_value);\n    let original_value=original.unwrap();\n    if *original_value.value!=7 || original_value.generation!=0{return 4;}\n    0\n}\npub fn affine_result_clones_only_the_active_variant()->i64{\n    let original_ok:Result<Item,Item>=Result::Ok(item(40));\n    let original_err:Result<Item,Item>=Result::Err(item(2));\n    let copied_ok=cloned(&original_ok);\n    let copied_err=original_err.clone();\n    drop(original_ok);drop(original_err);\n    let ok=copied_ok.unwrap();let error=copied_err.unwrap_err();\n    if ok.generation!=1 || error.generation!=1{return 1;}\n    *ok.value+*error.value\n}\npub fn option_clone_from_covers_all_variant_transitions()->i64{\n    let mut target:Option<Item>=Option::None;\n    let some=Option::Some(item(42));\n    let none:Option<Item>=Option::None;\n    updated(&mut target,&some);\n    match &target{Option::Some(value)=>{if value.generation!=1{return 1;}},Option::None=>return 2}\n    target.clone_from(&some);\n    match &mut target{Option::Some(value)=>{*value.value=7;},Option::None=>return 3}\n    match &some{Option::Some(value)=>{if *value.value!=42{return 4;}},Option::None=>return 5}\n    target.clone_from(&none);\n    if target.is_some(){return 6;}\n    updated(&mut target,&none);\n    if !target.is_none(){return 7;}\n    0\n}\npub fn result_clone_from_covers_both_payload_types_and_transitions()->i64{\n    let mut target:Result<Item,Item>=Result::Ok(item(1));\n    let ok:Result<Item,Item>=Result::Ok(item(40));\n    let error:Result<Item,Item>=Result::Err(item(2));\n    target.clone_from(&error);\n    match &target{Result::Err(value)=>{if value.generation!=1 || *value.value!=2{return 1;}},Result::Ok(_)=>return 2}\n    updated(&mut target,&error);\n    updated(&mut target,&ok);\n    match &mut target{Result::Ok(value)=>{if value.generation!=1{return 3;}*value.value=42;},Result::Err(_)=>return 4}\n    match &ok{Result::Ok(value)=>{if *value.value!=40{return 5;}},Result::Err(_)=>return 6}\n    target.clone_from(&ok);\n    let final_value=target.unwrap();\n    if *final_value.value!=40 || final_value.generation!=1{return 7;}\n    0\n}\npub fn option_equality_checks_tags_and_builtin_payload_evidence()->i64{\n    let mut left=Option::Some(item(42));let right=Option::Some(item(42));\n    let other=Option::Some(item(7));let none:Option<Item>=Option::None;\n    eq_contract(&left);\n    if !(left==right) || left!=right || !same(&left,&right) || different(&left,&right){return 1;}\n    if left==other || !(left!=other){return 2;}\n    if left==none || none==right || !(left!=none){return 3;}\n    if !(none==none) || none!=none{return 4;}\n    match &mut left{Option::Some(value)=>{*value.value=7;},Option::None=>return 5}\n    if !(left==other) || left==right{return 6;}\n    0\n}\npub fn result_equality_checks_tags_and_both_payloads()->i64{\n    let ok:Result<Item,Item>=Result::Ok(item(42));let ok_same:Result<Item,Item>=Result::Ok(item(42));\n    let error:Result<Item,Item>=Result::Err(item(42));let error_same:Result<Item,Item>=Result::Err(item(42));\n    let error_other:Result<Item,Item>=Result::Err(item(7));\n    eq_contract(&ok);eq_contract(&error);\n    if !same(&ok,&ok_same) || ok!=ok_same{return 1;}\n    if !same(&error,&error_same) || error!=error_same{return 2;}\n    if ok==error || !(ok!=error){return 3;}\n    if error==error_other || !different(&error,&error_other){return 4;}\n    0\n}\n#[derive(Clone,Copy)] struct NeOverride { value:i64 }\nimpl PartialEq for NeOverride {\n    fn eq(&self,other:&Self)->bool{self.value==other.value}\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool{false}\n}\npub fn derived_ne_negates_aggregate_eq_not_payload_ne()->i64{\n    let left=Option::Some(NeOverride{value:1});let right=Option::Some(NeOverride{value:2});\n    if left==right || !(left!=right) || !left.ne(&right){return 1;}\n    let first:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:1});\n    let second:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:2});\n    if first==second || !(first!=second) || !different(&first,&second){return 2;}\n    0\n}\n#[derive(Copy)] struct CopyPayload { value:i64 }\nimpl Clone for CopyPayload {fn clone(&self)->Self{CopyPayload{value:self.value+1}}}\npub fn generic_enum_clone_calls_clone_even_for_a_copy_payload()->i64{\n    let original=Option::Some(CopyPayload{value:41});\n    let copy=original.clone();\n    if original.unwrap().value!=41 || copy.unwrap().value!=42{return 1;}\n    let ok:Result<CopyPayload,CopyPayload>=Result::Ok(CopyPayload{value:41});\n    let error:Result<CopyPayload,CopyPayload>=Result::Err(CopyPayload{value:41});\n    if cloned(&ok).unwrap().value!=42 || ok.unwrap().value!=41{return 2;}\n    if error.clone().unwrap_err().value!=42 || error.unwrap_err().value!=41{return 3;}\n    0\n}\npub fn copy_is_conditional_and_keeps_original_values_usable()->i64{\n    let option=Option::Some(21i64);let first=copied_twice(option);\n    if first.unwrap()+option.unwrap()!=42{return 1;}\n    let result:Result<i64,i64>=Result::Err(21);let second=copied_twice(result);\n    if second.unwrap_err()+result.unwrap_err()!=42{return 2;}\n    0\n}\nfn option_view<'a>(external:&'a i64)->Option<&'a i64>{let carrier=Option::Some(external);cloned(&carrier)}\nfn result_ok_view<'a>(external:&'a i64)->Result<&'a i64,i64>{let carrier:Result<&i64,i64>=Result::Ok(external);carrier.clone()}\nfn result_err_view<'a>(external:&'a i64)->Result<i64,&'a i64>{let carrier:Result<i64,&i64>=Result::Err(external);cloned(&carrier)}\nfn nested_view<'a>(external:&'a i64)->Result<Option<&'a i64>,i64>{let carrier:Result<Option<&i64>,i64>=Result::Ok(Option::Some(external));carrier.clone()}\nfn absent_view<'a>()->Option<&'a i64>{let carrier:Option<&i64>=Option::None;carrier.clone()}\npub fn shared_payload_clones_outlive_their_enum_carriers()->i64{\n    let value:i64=42;\n    if *option_view(&value).unwrap()!=42{return 1;}\n    if *result_ok_view(&value).unwrap()!=42{return 2;}\n    if *result_err_view(&value).unwrap_err()!=42{return 3;}\n    if *nested_view(&value).unwrap().unwrap()!=42{return 4;}\n    let absent=absent_view();if !absent.is_none(){return 5;}\n    0\n}\npub fn exclusive_payloads_can_be_compared_without_being_copied()->i64{\n    let mut first:i64=42;let mut second:i64=42;\n    {\n        let left=Option::Some(&mut first);let right=Option::Some(&mut second);\n        if !(left==right) || left!=right{return 1;}\n    }\n    first=40;second=2;\n    first+second\n}\npub fn mutation_clone_probe()->i64{Option::Some(item(42)).clone().unwrap().generation}\npub fn mutation_equality_probe()->i64{if Option::Some(item(1))==Option::Some(item(2)){1}else{0}}\npub fn mutation_prelude_probe()->i64{if Option::Some(42i64).is_some(){1}else{0}}\n")),
], ""), errors);
        count += 1;
    }
    if 344usize % workers == shard {
        record(source_case("primitive_text_methods_reject_spurious_generic_type_arguments_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn bad(){let bytes:Vec<u8>=Vec::new();std::str::from_utf8::<i64>(bytes.as_slice());}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 345usize % workers == shard {
        record(source_case("primitive_text_methods_reject_spurious_generic_type_arguments_0001", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn bad(){let value=\"text\".len::<bool>();}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 346usize % workers == shard {
        record(source_case("primitive_text_methods_reject_spurious_generic_type_arguments_0002", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn bad(){let value=\"text\".is_empty::<bool>();}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 347usize % workers == shard {
        record(source_case("primitive_text_methods_reject_spurious_generic_type_arguments_0003", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn bad(){let value=\"text\".as_bytes::<bool>();}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 348usize % workers == shard {
        record(source_case("primitive_text_methods_reject_spurious_generic_type_arguments_0004", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn bad(){let value=\"text\".to_string::<bool>();}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 349usize % workers == shard {
        record(source_case("production_clone_and_equality_mutations_change_artifacts_and_results_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\nfn same_bytes(value: &String, expected: &str) -> bool {\n    let actual = value.as_bytes();\n    let expected_bytes = expected.as_bytes();\n    if actual.len() != expected_bytes.len() { return false; }\n    let mut index: usize = 0;\n    while index < actual.len() {\n        if actual[index] != expected_bytes[index] { return false; }\n        index = index + 1;\n    }\n    true\n}\n\n\npub fn original_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"source\") {0} else {1}\n}\npub fn changed_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"changed\") {0} else {1}\n}\npub fn equal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.eq(&right) {1} else {0}\n}\npub fn unequal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.ne(&right) {1} else {0}\n}\npub fn equal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left==right {1} else {0}\n}\npub fn unequal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left!=right {1} else {0}\n}\npub fn differing_method()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left.ne(&right) {1} else {0}\n}\npub fn differing_operator()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left!=right {1} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 350usize % workers == shard {
        record(source_case("production_clone_and_equality_mutations_change_artifacts_and_results_0009", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", Some("//! UTF-8 strings own bytes. Validation happens when unchecked bytes become a String.\nuse crate::Result;\nuse crate::bytes::{append_bytes, starts_with, valid_utf8};\n\n#[derive(PartialOrd, Ord)]\npub struct String {\n    bytes: Vec<u8>,\n}\n\npub struct FromUtf8Error {\n    bytes: Vec<u8>,\n}\n\nimpl String {\n    pub fn new() -> String {\n        String { bytes: Vec::new() }\n    }\n\n    pub fn from_utf8(bytes: Vec<u8>) -> Result<String, FromUtf8Error> {\n        if valid_utf8(&bytes) {\n            Result::Ok(String { bytes })\n        } else {\n            Result::Err(FromUtf8Error { bytes })\n        }\n    }\n\n    pub fn from(value: &str) -> String {\n        String::from_str(value)\n    }\n\n    pub fn from_str(value: &str) -> String {\n        let mut result = String::new();\n        result.push_str(value);\n        result\n    }\n\n    pub fn push_str(&mut self, value: &str) {\n        append_bytes(&mut self.bytes, value.as_bytes());\n    }\n\n    pub fn push(&mut self, value: char) {\n        let code = value as u32;\n        if code < 0x80u32 {\n            self.bytes.push(code as u8);\n        } else if code < 0x800u32 {\n            self.bytes.push(((code >> 6) | 0xc0u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else if code < 0x10000u32 {\n            self.bytes.push(((code >> 12) | 0xe0u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else {\n            self.bytes.push(((code >> 18) | 0xf0u32) as u8);\n            self.bytes.push((((code >> 12) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        }\n    }\n\n    pub fn starts_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_starts_with(self.as_str(), pattern)\n    }\n    pub fn ends_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_ends_with(self.as_str(), pattern)\n    }\n    pub fn contains<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_contains(self.as_str(), pattern)\n    }\n    pub fn find<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_find(self.as_str(), pattern)\n    }\n    pub fn rfind<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_rfind(self.as_str(), pattern)\n    }\n    pub fn is_ascii(&self) -> bool {\n        crate::str::__lkjscript_is_ascii(self.as_str())\n    }\n    pub fn to_ascii_lowercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_lowercase(self.as_str())\n    }\n    pub fn to_ascii_uppercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_uppercase(self.as_str())\n    }\n    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {\n        crate::str::__lkjscript_eq_ignore_ascii_case(self.as_str(), other)\n    }\n    pub fn repeat(&self, count: usize) -> String {\n        crate::str::__lkjscript_repeat(self.as_str(), count)\n    }\n    pub fn replace<P: crate::str::Pattern>(&self, pattern: P, replacement: &str) -> String {\n        crate::str::__lkjscript_replace(self.as_str(), pattern, replacement)\n    }\n\n    pub fn to_string(&self) -> String {\n        self.clone()\n    }\n\n    pub fn as_str(&self) -> &str {\n        std::str::from_utf8(self.bytes.as_slice()).unwrap()\n    }\n\n    pub fn len(&self) -> usize {\n        self.bytes.len()\n    }\n\n    pub fn is_empty(&self) -> bool {\n        self.bytes.len() == 0\n    }\n\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n\n    pub fn clear(&mut self) {\n        while self.bytes.len() > 0 {\n            self.bytes.pop();\n        }\n    }\n}\n\n// Only valid UTF-8 can enter the private byte storage, so exact copies preserve it.\nimpl Clone for String {\n    fn clone(&self) -> Self {\n        let mut bytes: Vec<u8> = Vec::new();\n        append_bytes(&mut bytes, \"changed\".as_bytes());\n        String { bytes }\n    }\n\n    fn clone_from(&mut self, source: &Self) {\n        self.clear();\n        append_bytes(&mut self.bytes, source.bytes.as_slice());\n    }\n}\n\nimpl PartialEq for String {\n    fn eq(&self, other: &Self) -> bool {\n        self.bytes.len() == other.bytes.len()\n            && starts_with(self.bytes.as_slice(), other.bytes.as_slice())\n    }\n\n    // lkjscript currently requires an executable method for each builtin operation.\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self, other: &Self) -> bool {\n        !self.eq(other)\n    }\n}\n\nimpl Eq for String {}\n\nimpl FromUtf8Error {\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n}\n\n// Resolved compiler helpers retain the same source String implementation.\npub fn __lkjscript_new() -> String {\n    String::new()\n}\n\n// Range syntax resolves this declaration, preserving the source String invariant.\npub fn __lkjscript_as_str(value: &String) -> &str {\n    value.as_str()\n}\n\n/// Canonical lexical String ordering uses its UTF-8 bytes, as Rust does.\npub fn __lkjscript_compare(left: &String, right: &String) -> i32 {\n    let left = left.as_bytes();\n    let right = right.as_bytes();\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1;\n        }\n        if left[index] > right[index] {\n            return 1;\n        }\n        index = index + 1;\n    }\n    if left.len() < right.len() {\n        -1\n    } else if left.len() > right.len() {\n        1\n    } else {\n        0\n    }\n}\n")),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\nfn same_bytes(value: &String, expected: &str) -> bool {\n    let actual = value.as_bytes();\n    let expected_bytes = expected.as_bytes();\n    if actual.len() != expected_bytes.len() { return false; }\n    let mut index: usize = 0;\n    while index < actual.len() {\n        if actual[index] != expected_bytes[index] { return false; }\n        index = index + 1;\n    }\n    true\n}\n\n\npub fn original_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"source\") {0} else {1}\n}\npub fn changed_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"changed\") {0} else {1}\n}\npub fn equal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.eq(&right) {1} else {0}\n}\npub fn unequal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.ne(&right) {1} else {0}\n}\npub fn equal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left==right {1} else {0}\n}\npub fn unequal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left!=right {1} else {0}\n}\npub fn differing_method()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left.ne(&right) {1} else {0}\n}\npub fn differing_operator()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left!=right {1} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 351usize % workers == shard {
        record(source_case("production_clone_and_equality_mutations_change_artifacts_and_results_0018", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", Some("//! UTF-8 strings own bytes. Validation happens when unchecked bytes become a String.\nuse crate::Result;\nuse crate::bytes::{append_bytes, starts_with, valid_utf8};\n\n#[derive(PartialOrd, Ord)]\npub struct String {\n    bytes: Vec<u8>,\n}\n\npub struct FromUtf8Error {\n    bytes: Vec<u8>,\n}\n\nimpl String {\n    pub fn new() -> String {\n        String { bytes: Vec::new() }\n    }\n\n    pub fn from_utf8(bytes: Vec<u8>) -> Result<String, FromUtf8Error> {\n        if valid_utf8(&bytes) {\n            Result::Ok(String { bytes })\n        } else {\n            Result::Err(FromUtf8Error { bytes })\n        }\n    }\n\n    pub fn from(value: &str) -> String {\n        String::from_str(value)\n    }\n\n    pub fn from_str(value: &str) -> String {\n        let mut result = String::new();\n        result.push_str(value);\n        result\n    }\n\n    pub fn push_str(&mut self, value: &str) {\n        append_bytes(&mut self.bytes, value.as_bytes());\n    }\n\n    pub fn push(&mut self, value: char) {\n        let code = value as u32;\n        if code < 0x80u32 {\n            self.bytes.push(code as u8);\n        } else if code < 0x800u32 {\n            self.bytes.push(((code >> 6) | 0xc0u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else if code < 0x10000u32 {\n            self.bytes.push(((code >> 12) | 0xe0u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else {\n            self.bytes.push(((code >> 18) | 0xf0u32) as u8);\n            self.bytes.push((((code >> 12) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        }\n    }\n\n    pub fn starts_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_starts_with(self.as_str(), pattern)\n    }\n    pub fn ends_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_ends_with(self.as_str(), pattern)\n    }\n    pub fn contains<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_contains(self.as_str(), pattern)\n    }\n    pub fn find<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_find(self.as_str(), pattern)\n    }\n    pub fn rfind<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_rfind(self.as_str(), pattern)\n    }\n    pub fn is_ascii(&self) -> bool {\n        crate::str::__lkjscript_is_ascii(self.as_str())\n    }\n    pub fn to_ascii_lowercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_lowercase(self.as_str())\n    }\n    pub fn to_ascii_uppercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_uppercase(self.as_str())\n    }\n    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {\n        crate::str::__lkjscript_eq_ignore_ascii_case(self.as_str(), other)\n    }\n    pub fn repeat(&self, count: usize) -> String {\n        crate::str::__lkjscript_repeat(self.as_str(), count)\n    }\n    pub fn replace<P: crate::str::Pattern>(&self, pattern: P, replacement: &str) -> String {\n        crate::str::__lkjscript_replace(self.as_str(), pattern, replacement)\n    }\n\n    pub fn to_string(&self) -> String {\n        self.clone()\n    }\n\n    pub fn as_str(&self) -> &str {\n        std::str::from_utf8(self.bytes.as_slice()).unwrap()\n    }\n\n    pub fn len(&self) -> usize {\n        self.bytes.len()\n    }\n\n    pub fn is_empty(&self) -> bool {\n        self.bytes.len() == 0\n    }\n\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n\n    pub fn clear(&mut self) {\n        while self.bytes.len() > 0 {\n            self.bytes.pop();\n        }\n    }\n}\n\n// Only valid UTF-8 can enter the private byte storage, so exact copies preserve it.\nimpl Clone for String {\n    fn clone(&self) -> Self {\n        let mut bytes: Vec<u8> = Vec::new();\n        append_bytes(&mut bytes, self.bytes.as_slice());\n        String { bytes }\n    }\n\n    fn clone_from(&mut self, source: &Self) {\n        self.clear();\n        append_bytes(&mut self.bytes, source.bytes.as_slice());\n    }\n}\n\nimpl PartialEq for String {\n    fn eq(&self, other: &Self) -> bool {\n        false\n    }\n\n    // lkjscript currently requires an executable method for each builtin operation.\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self, other: &Self) -> bool {\n        !self.eq(other)\n    }\n}\n\nimpl Eq for String {}\n\nimpl FromUtf8Error {\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n}\n\n// Resolved compiler helpers retain the same source String implementation.\npub fn __lkjscript_new() -> String {\n    String::new()\n}\n\n// Range syntax resolves this declaration, preserving the source String invariant.\npub fn __lkjscript_as_str(value: &String) -> &str {\n    value.as_str()\n}\n\n/// Canonical lexical String ordering uses its UTF-8 bytes, as Rust does.\npub fn __lkjscript_compare(left: &String, right: &String) -> i32 {\n    let left = left.as_bytes();\n    let right = right.as_bytes();\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1;\n        }\n        if left[index] > right[index] {\n            return 1;\n        }\n        index = index + 1;\n    }\n    if left.len() < right.len() {\n        -1\n    } else if left.len() > right.len() {\n        1\n    } else {\n        0\n    }\n}\n")),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\nfn same_bytes(value: &String, expected: &str) -> bool {\n    let actual = value.as_bytes();\n    let expected_bytes = expected.as_bytes();\n    if actual.len() != expected_bytes.len() { return false; }\n    let mut index: usize = 0;\n    while index < actual.len() {\n        if actual[index] != expected_bytes[index] { return false; }\n        index = index + 1;\n    }\n    true\n}\n\n\npub fn original_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"source\") {0} else {1}\n}\npub fn changed_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"changed\") {0} else {1}\n}\npub fn equal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.eq(&right) {1} else {0}\n}\npub fn unequal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.ne(&right) {1} else {0}\n}\npub fn equal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left==right {1} else {0}\n}\npub fn unequal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left!=right {1} else {0}\n}\npub fn differing_method()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left.ne(&right) {1} else {0}\n}\npub fn differing_operator()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left!=right {1} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 352usize % workers == shard {
        record(source_case("production_clone_and_equality_mutations_change_artifacts_and_results_0027", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", Some("//! UTF-8 strings own bytes. Validation happens when unchecked bytes become a String.\nuse crate::Result;\nuse crate::bytes::{append_bytes, starts_with, valid_utf8};\n\n#[derive(PartialOrd, Ord)]\npub struct String {\n    bytes: Vec<u8>,\n}\n\npub struct FromUtf8Error {\n    bytes: Vec<u8>,\n}\n\nimpl String {\n    pub fn new() -> String {\n        String { bytes: Vec::new() }\n    }\n\n    pub fn from_utf8(bytes: Vec<u8>) -> Result<String, FromUtf8Error> {\n        if valid_utf8(&bytes) {\n            Result::Ok(String { bytes })\n        } else {\n            Result::Err(FromUtf8Error { bytes })\n        }\n    }\n\n    pub fn from(value: &str) -> String {\n        String::from_str(value)\n    }\n\n    pub fn from_str(value: &str) -> String {\n        let mut result = String::new();\n        result.push_str(value);\n        result\n    }\n\n    pub fn push_str(&mut self, value: &str) {\n        append_bytes(&mut self.bytes, value.as_bytes());\n    }\n\n    pub fn push(&mut self, value: char) {\n        let code = value as u32;\n        if code < 0x80u32 {\n            self.bytes.push(code as u8);\n        } else if code < 0x800u32 {\n            self.bytes.push(((code >> 6) | 0xc0u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else if code < 0x10000u32 {\n            self.bytes.push(((code >> 12) | 0xe0u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        } else {\n            self.bytes.push(((code >> 18) | 0xf0u32) as u8);\n            self.bytes.push((((code >> 12) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push((((code >> 6) & 0x3fu32) | 0x80u32) as u8);\n            self.bytes.push(((code & 0x3fu32) | 0x80u32) as u8);\n        }\n    }\n\n    pub fn starts_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_starts_with(self.as_str(), pattern)\n    }\n    pub fn ends_with<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_ends_with(self.as_str(), pattern)\n    }\n    pub fn contains<P: crate::str::Pattern>(&self, pattern: P) -> bool {\n        crate::str::__lkjscript_contains(self.as_str(), pattern)\n    }\n    pub fn find<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_find(self.as_str(), pattern)\n    }\n    pub fn rfind<P: crate::str::Pattern>(&self, pattern: P) -> crate::Option<usize> {\n        crate::str::__lkjscript_rfind(self.as_str(), pattern)\n    }\n    pub fn is_ascii(&self) -> bool {\n        crate::str::__lkjscript_is_ascii(self.as_str())\n    }\n    pub fn to_ascii_lowercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_lowercase(self.as_str())\n    }\n    pub fn to_ascii_uppercase(&self) -> String {\n        crate::str::__lkjscript_to_ascii_uppercase(self.as_str())\n    }\n    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {\n        crate::str::__lkjscript_eq_ignore_ascii_case(self.as_str(), other)\n    }\n    pub fn repeat(&self, count: usize) -> String {\n        crate::str::__lkjscript_repeat(self.as_str(), count)\n    }\n    pub fn replace<P: crate::str::Pattern>(&self, pattern: P, replacement: &str) -> String {\n        crate::str::__lkjscript_replace(self.as_str(), pattern, replacement)\n    }\n\n    pub fn to_string(&self) -> String {\n        self.clone()\n    }\n\n    pub fn as_str(&self) -> &str {\n        std::str::from_utf8(self.bytes.as_slice()).unwrap()\n    }\n\n    pub fn len(&self) -> usize {\n        self.bytes.len()\n    }\n\n    pub fn is_empty(&self) -> bool {\n        self.bytes.len() == 0\n    }\n\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n\n    pub fn clear(&mut self) {\n        while self.bytes.len() > 0 {\n            self.bytes.pop();\n        }\n    }\n}\n\n// Only valid UTF-8 can enter the private byte storage, so exact copies preserve it.\nimpl Clone for String {\n    fn clone(&self) -> Self {\n        let mut bytes: Vec<u8> = Vec::new();\n        append_bytes(&mut bytes, self.bytes.as_slice());\n        String { bytes }\n    }\n\n    fn clone_from(&mut self, source: &Self) {\n        self.clear();\n        append_bytes(&mut self.bytes, source.bytes.as_slice());\n    }\n}\n\nimpl PartialEq for String {\n    fn eq(&self, other: &Self) -> bool {\n        self.bytes.len() == other.bytes.len()\n            && starts_with(self.bytes.as_slice(), other.bytes.as_slice())\n    }\n\n    // lkjscript currently requires an executable method for each builtin operation.\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self, other: &Self) -> bool {\n        false\n    }\n}\n\nimpl Eq for String {}\n\nimpl FromUtf8Error {\n    pub fn as_bytes(&self) -> &[u8] {\n        self.bytes.as_slice()\n    }\n\n    pub fn into_bytes(self) -> Vec<u8> {\n        self.bytes\n    }\n}\n\n// Resolved compiler helpers retain the same source String implementation.\npub fn __lkjscript_new() -> String {\n    String::new()\n}\n\n// Range syntax resolves this declaration, preserving the source String invariant.\npub fn __lkjscript_as_str(value: &String) -> &str {\n    value.as_str()\n}\n\n/// Canonical lexical String ordering uses its UTF-8 bytes, as Rust does.\npub fn __lkjscript_compare(left: &String, right: &String) -> i32 {\n    let left = left.as_bytes();\n    let right = right.as_bytes();\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1;\n        }\n        if left[index] > right[index] {\n            return 1;\n        }\n        index = index + 1;\n    }\n    if left.len() < right.len() {\n        -1\n    } else if left.len() > right.len() {\n        1\n    } else {\n        0\n    }\n}\n")),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\nfn same_bytes(value: &String, expected: &str) -> bool {\n    let actual = value.as_bytes();\n    let expected_bytes = expected.as_bytes();\n    if actual.len() != expected_bytes.len() { return false; }\n    let mut index: usize = 0;\n    while index < actual.len() {\n        if actual[index] != expected_bytes[index] { return false; }\n        index = index + 1;\n    }\n    true\n}\n\n\npub fn original_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"source\") {0} else {1}\n}\npub fn changed_clone()->usize {\n    let value=String::from_str(\"source\").clone();\n    if same_bytes(&value,\"changed\") {0} else {1}\n}\npub fn equal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.eq(&right) {1} else {0}\n}\npub fn unequal_probe()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left.ne(&right) {1} else {0}\n}\npub fn equal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left==right {1} else {0}\n}\npub fn unequal_operator()->usize {\n    let left=String::from_str(\"same\");let right=String::from_str(\"same\");\n    if left!=right {1} else {0}\n}\npub fn differing_method()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left.ne(&right) {1} else {0}\n}\npub fn differing_operator()->usize {\n    let left=String::from_str(\"left\");let right=String::from_str(\"right\");\n    if left!=right {1} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 353usize % workers == shard {
        record(source_case("production_decimal_conversion_executes_on_both_routes_0000", "float_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("float_component.rs", Some("mod floats; use floats::{decimal_bits, Parsed};\npub fn compare_0()->i64 { match decimal_bits(\"0\") { Parsed::Finite(bits) => if bits==0u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_1()->i64 { match decimal_bits(\"-0\") { Parsed::Finite(bits) => if bits==9223372036854775808u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_2()->i64 { match decimal_bits(\".5\") { Parsed::Finite(bits) => if bits==4602678819172646912u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_3()->i64 { match decimal_bits(\"1.e2\") { Parsed::Finite(bits) => if bits==4636737291354636288u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_4()->i64 { match decimal_bits(\"1.00000000000000011102230246251565404236316680908203125\") { Parsed::Finite(bits) => if bits==4607182418800017408u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_5()->i64 { match decimal_bits(\"1.00000000000000033306690738754696212708950042724609375\") { Parsed::Finite(bits) => if bits==4607182418800017410u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_6()->i64 { match decimal_bits(\"2.4703282292062327e-324\") { Parsed::Finite(bits) => if bits==0u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_7()->i64 { match decimal_bits(\"2.4703282292062328e-324\") { Parsed::Finite(bits) => if bits==1u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_8()->i64 { match decimal_bits(\"2.2250738585072012e-308\") { Parsed::Finite(bits) => if bits==4503599627370496u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_9()->i64 { match decimal_bits(\"1.7976931348623157e308\") { Parsed::Finite(bits) => if bits==9218868437227405311u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_10()->i64 { match decimal_bits(\"1.7976931348623159e308\") { Parsed::Overflow=>0i64, _=>3i64 } }\npub fn compare_11()->i64 { match decimal_bits(\"1e999999999999999999999999999999\") { Parsed::Overflow=>0i64, _=>3i64 } }\npub fn compare_12()->i64 { match decimal_bits(\"-1e-999999999999999999999999999999\") { Parsed::Finite(bits) => if bits==9223372036854775808u64 {0i64} else {1i64}, _=>2i64 } }\npub fn compare_13()->i64 { match decimal_bits(\"0e999x\") { Parsed::Invalid=>0i64, _=>4i64 } }\n")),
source_file("floats.rs", Some("//! Decimal literal conversion uses one exact rational rounding step. This is\n//! ordinary producer code; no host float parser or new IR operation is required.\n//! Decimal magnitude shortcuts classify values outside binary64's range. They\n//! never limit the number of source digits or reject a representable literal.\n\npub(super) enum Parsed {\n    Finite(u64),\n    Overflow,\n    Invalid,\n}\n\n#[derive(Clone)]\nstruct Natural {\n    // Little-endian base 2^32 limbs, with no high zero limbs. Zero is empty.\n    limbs: Vec<u32>,\n}\nimpl Natural {\n    fn zero() -> Self {\n        Self { limbs: Vec::new() }\n    }\n    fn one() -> Self {\n        Self { limbs: vec![1u32] }\n    }\n    fn multiply_add(&mut self, factor: u32, add: u32) {\n        let mut carry = add as u64;\n        let mut index = 0;\n        while index < self.limbs.len() {\n            let value = self.limbs[index] as u64 * factor as u64 + carry;\n            self.limbs[index] = value as u32;\n            carry = value >> 32;\n            index += 1;\n        }\n        if carry != 0 {\n            self.limbs.push(carry as u32);\n        }\n    }\n    fn bits(&self) -> usize {\n        if self.limbs.is_empty() {\n            return 0;\n        }\n        let mut high = self.limbs[self.limbs.len() - 1];\n        let mut bits = (self.limbs.len() - 1) * 32;\n        while high != 0 {\n            bits += 1;\n            high >>= 1;\n        }\n        bits\n    }\n    fn compare(&self, other: &Self) -> i32 {\n        if self.limbs.len() < other.limbs.len() {\n            return -1;\n        }\n        if self.limbs.len() > other.limbs.len() {\n            return 1;\n        }\n        let mut index = self.limbs.len();\n        while index > 0 {\n            index -= 1;\n            if self.limbs[index] < other.limbs[index] {\n                return -1;\n            }\n            if self.limbs[index] > other.limbs[index] {\n                return 1;\n            }\n        }\n        0\n    }\n    fn shifted(&self, count: usize) -> Self {\n        if self.limbs.is_empty() {\n            return Self::zero();\n        }\n        let mut limbs = Vec::new();\n        let mut index = 0;\n        while index < count / 32 {\n            limbs.push(0u32);\n            index += 1;\n        }\n        let shift = count % 32;\n        let mut carry = 0u64;\n        for limb in &self.limbs {\n            let value = ((*limb as u64) << shift) | carry;\n            limbs.push(value as u32);\n            carry = value >> 32;\n        }\n        if carry != 0 {\n            limbs.push(carry as u32);\n        }\n        Self { limbs }\n    }\n    // The caller compares first, so subtraction is nonnegative.\n    fn subtract(&mut self, other: &Self) {\n        let mut borrow = 0u64;\n        let mut index = 0;\n        while index < self.limbs.len() {\n            let left = self.limbs[index] as u64;\n            let right = if index < other.limbs.len() {\n                other.limbs[index] as u64\n            } else {\n                0\n            } + borrow;\n            if left >= right {\n                self.limbs[index] = (left - right) as u32;\n                borrow = 0;\n            } else {\n                self.limbs[index] = ((1u64 << 32) + left - right) as u32;\n                borrow = 1;\n            }\n            index += 1;\n        }\n        while !self.limbs.is_empty() {\n            let last = self.limbs.len() - 1;\n            if self.limbs[last] != 0 {\n                break;\n            }\n            self.limbs.pop();\n        }\n    }\n}\n\n// A magnitude beyond the whole input length plus 400 cannot be cancelled back\n// into binary64's decimal range by the mantissa's decimal-point position.\nfn exponent_limit(length: usize) -> usize {\n    length.checked_add(400).unwrap_or(!0usize)\n}\nfn signed_sum(\n    left_negative: bool,\n    left: usize,\n    right_negative: bool,\n    right: usize,\n) -> (bool, usize) {\n    if left_negative == right_negative {\n        (left_negative, left.checked_add(right).unwrap_or(!0usize))\n    } else if left >= right {\n        (left_negative, left - right)\n    } else {\n        (right_negative, right - left)\n    }\n}\n\n// The scaled rational has a quotient of at most 53 bits. The remainder is kept\n// exactly so midpoint ties are resolved from the low significand bit once.\nfn rounded_quotient(mut numerator: Natural, denominator: &Natural) -> u64 {\n    let numerator_bits = numerator.bits();\n    let denominator_bits = denominator.bits();\n    let mut shift = if numerator_bits >= denominator_bits {\n        numerator_bits - denominator_bits\n    } else {\n        0\n    };\n    let mut quotient = 0u64;\n    loop {\n        let candidate = denominator.shifted(shift);\n        if numerator.compare(&candidate) >= 0 {\n            numerator.subtract(&candidate);\n            quotient |= 1u64 << shift;\n        }\n        if shift == 0 {\n            break;\n        }\n        shift -= 1;\n    }\n    let doubled = numerator.shifted(1);\n    let comparison = doubled.compare(denominator);\n    if comparison > 0 || (comparison == 0 && quotient & 1 != 0) {\n        quotient += 1;\n    }\n    quotient\n}\n\npub(super) fn decimal_bits(text: &str) -> Parsed {\n    let bytes = text.as_bytes();\n    let mut at = 0;\n    let mut negative = false;\n    if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {\n        negative = bytes[at] == b'-';\n        at += 1;\n    }\n    let sign = if negative { 1u64 << 63 } else { 0 };\n    let mut digits: Vec<u8> = Vec::new();\n    let mut fractional = 0usize;\n    let mut decimal_point = false;\n    let mut saw_digit = false;\n    while at < bytes.len() {\n        let byte = bytes[at];\n        if byte.is_ascii_digit() {\n            saw_digit = true;\n            if decimal_point {\n                fractional += 1;\n            }\n            if !digits.is_empty() || byte != b'0' {\n                digits.push(byte - b'0');\n            }\n        } else if byte == b'.' && !decimal_point {\n            decimal_point = true;\n        } else {\n            break;\n        }\n        at += 1;\n    }\n    if !saw_digit {\n        return Parsed::Invalid;\n    }\n    let mut exponent = 0usize;\n    let mut exponent_negative = false;\n    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {\n        at += 1;\n        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {\n            exponent_negative = bytes[at] == b'-';\n            at += 1;\n        }\n        let start = at;\n        let limit = exponent_limit(text.len());\n        while at < bytes.len() && bytes[at].is_ascii_digit() {\n            let digit = (bytes[at] - b'0') as usize;\n            if exponent > limit / 10 || (exponent == limit / 10 && digit > limit % 10) {\n                exponent = limit;\n            } else {\n                exponent = exponent * 10 + digit;\n            }\n            at += 1;\n        }\n        if at == start {\n            return Parsed::Invalid;\n        }\n    }\n    if at != bytes.len() {\n        return Parsed::Invalid;\n    }\n    if digits.is_empty() {\n        return Parsed::Finite(sign);\n    }\n\n    let integer_position = digits.len() - 1;\n    let (base_negative, base) = if integer_position >= fractional {\n        (false, integer_position - fractional)\n    } else {\n        (true, fractional - integer_position)\n    };\n    let (order_negative, order) = signed_sum(exponent_negative, exponent, base_negative, base);\n    if !order_negative && order > 308 {\n        return Parsed::Overflow;\n    }\n    if order_negative && order > 324 {\n        return Parsed::Finite(sign);\n    }\n\n    // Trailing zeros affect the power of ten, not the integer significand.\n    loop {\n        let last = digits.len() - 1;\n        if digits[last] != 0 {\n            break;\n        }\n        digits.pop();\n    }\n    let significant_position = digits.len() - 1;\n    let (power_negative, power) = if order_negative {\n        (true, significant_position + order)\n    } else if significant_position >= order {\n        (true, significant_position - order)\n    } else {\n        (false, order - significant_position)\n    };\n    let mut numerator = Natural::zero();\n    for digit in digits {\n        numerator.multiply_add(10, digit as u32);\n    }\n    let mut denominator = Natural::one();\n    let mut index = 0;\n    while index < power {\n        if power_negative {\n            denominator.multiply_add(10, 0);\n        } else {\n            numerator.multiply_add(10, 0);\n        }\n        index += 1;\n    }\n\n    let numerator_bits = numerator.bits();\n    let denominator_bits = denominator.bits();\n    let mut binary_exponent: i32 = if numerator_bits >= denominator_bits {\n        (numerator_bits - denominator_bits) as i32\n    } else {\n        -((denominator_bits - numerator_bits) as i32)\n    };\n    let comparison = if binary_exponent >= 0 {\n        let shifted = denominator.shifted(binary_exponent as usize);\n        numerator.compare(&shifted)\n    } else {\n        let shifted = numerator.shifted((-binary_exponent) as usize);\n        shifted.compare(&denominator)\n    };\n    if comparison < 0 {\n        binary_exponent -= 1;\n    }\n    if binary_exponent > 1023 {\n        return Parsed::Overflow;\n    }\n    let normal = binary_exponent >= -1022;\n    let shift = if normal { 52 - binary_exponent } else { 1074 };\n    let mut significand = if shift >= 0 {\n        rounded_quotient(numerator.shifted(shift as usize), &denominator)\n    } else {\n        let shifted = denominator.shifted((-shift) as usize);\n        rounded_quotient(numerator, &shifted)\n    };\n    if !normal {\n        // 2^52 after rounding is exactly the smallest normal encoding.\n        return Parsed::Finite(sign | significand);\n    }\n    if significand == 1u64 << 53 {\n        significand >>= 1;\n        binary_exponent += 1;\n    }\n    if binary_exponent > 1023 {\n        return Parsed::Overflow;\n    }\n    let biased = (binary_exponent + 1023) as u64;\n    Parsed::Finite(sign | (biased << 52) | (significand - (1u64 << 52)))\n}\n")),
], ""), errors);
        count += 1;
    }
    if 354usize % workers == shard {
        record(source_case("production_ir_body_mutations_change_artifacts_and_results_0000", "src/ir_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("src/ir_component.rs", Some("//! Execution probes for the unchanged production IR representation module.\nmod ir;\n\nfn flag(value: &bool) -> i64 {\n    if *value { 1 } else { 0 }\n}\n\nfn place(value: usize) -> ir::Place {\n    let mut result = ir::Place::local(value);\n    result.projections.push(ir::Projection::Deref);\n    result.projections.push(ir::Projection::Field(value + 1));\n    result\n}\n\nfn operation(which: usize, changed: bool) -> ir::Op {\n    let n: usize = if changed { 43 } else { 42 };\n    if changed && which >= 19 && which <= 23 {\n        return ir::Op::Trap {\n            message: \"different tag\".to_string(),\n        };\n    }\n    match which {\n        0 => ir::Op::Text {\n            dst: n,\n            bytes: vec![0u8, 65u8, 255u8],\n        },\n        1 => ir::Op::Int { dst: n, value: -17 },\n        2 => ir::Op::Float {\n            dst: n,\n            bits: 4607182418800017408u64,\n        },\n        3 => ir::Op::Unit { dst: n },\n        4 => ir::Op::Read {\n            dst: n,\n            place: place(7),\n            moving: true,\n        },\n        5 => ir::Op::Borrow {\n            dst: n,\n            place: place(8),\n            mutable: false,\n        },\n        6 => ir::Op::SliceBorrow {\n            dst: n,\n            place: place(9),\n            mutable: true,\n        },\n        7 => ir::Op::Array {\n            dst: n,\n            elements: vec![3usize, 5usize],\n        },\n        8 => ir::Op::Assign {\n            place: place(n),\n            src: 11,\n        },\n        9 => ir::Op::Drop { place: place(n) },\n        10 => ir::Op::Unary {\n            dst: n,\n            op: ir::UnaryOp::Not,\n            src: 12,\n        },\n        11 => ir::Op::Binary {\n            dst: n,\n            op: ir::BinaryOp::Add,\n            left: 13,\n            right: 14,\n        },\n        12 => ir::Op::Call {\n            dst: n,\n            function: 4,\n            args: vec![15usize, 16usize],\n        },\n        13 => ir::Op::Struct {\n            dst: n,\n            fields: vec![17usize, 18usize],\n        },\n        14 => ir::Op::Variant {\n            dst: n,\n            variant: 2,\n            fields: vec![19usize, 20usize],\n        },\n        15 => ir::Op::Discriminant { dst: n, src: 21 },\n        16 => ir::Op::TagTest {\n            dst: n,\n            place: ir::Place::local(22),\n            variant: 3,\n        },\n        17 => ir::Op::Intrinsic {\n            dst: n,\n            name: ir::IntrinsicOp::VecNew,\n            args: vec![23usize, 24usize],\n        },\n        18 => ir::Op::If { condition: n },\n        19 => ir::Op::Else,\n        20 => ir::Op::End,\n        21 => ir::Op::Loop,\n        22 => ir::Op::Break,\n        23 => ir::Op::Continue,\n        24 => ir::Op::Return { src: n },\n        26 => ir::Op::DropIf { place: place(n) },\n        27 => ir::Op::BreakTo { depth: n },\n        28 => ir::Op::ContinueTo { depth: n },\n        29 => ir::Op::ViewRange {\n            dst: n,\n            place: place(6),\n            start: 7,\n            end: 8,\n            inclusive: true,\n            mutable: false,\n        },\n        _ => ir::Op::Trap {\n            message: if changed {\n                \"changed\".to_string()\n            } else {\n                \"trap\\0é水😀\".to_string()\n            },\n        },\n    }\n}\n\nfn operation_value(value: &ir::Op) -> i64 {\n    match value {\n        ir::Op::Text { dst, bytes } => *dst as i64 + bytes.len() as i64 + bytes[2] as i64,\n        ir::Op::Int { dst, value } => *dst as i64 + *value,\n        ir::Op::Float { dst, bits } => *dst as i64 + (*bits / 65536) as i64,\n        ir::Op::Unit { dst } => *dst as i64,\n        ir::Op::Read { dst, place, moving } => {\n            *dst as i64 + place.local as i64 + place.projections.len() as i64 + flag(moving)\n        }\n        ir::Op::Borrow {\n            dst,\n            place,\n            mutable,\n        } => *dst as i64 + place.local as i64 + flag(mutable),\n        ir::Op::SliceBorrow {\n            dst,\n            place,\n            mutable,\n        } => *dst as i64 + place.local as i64 + flag(mutable),\n        ir::Op::ViewRange {\n            dst,\n            place,\n            start,\n            end,\n            inclusive,\n            mutable,\n        } => {\n            *dst as i64\n                + place.local as i64\n                + *start as i64\n                + *end as i64\n                + flag(inclusive)\n                + flag(mutable)\n        }\n        ir::Op::Array { dst, elements } => *dst as i64 + elements[0] as i64 + elements[1] as i64,\n        ir::Op::Assign { place, src } => place.local as i64 + *src as i64,\n        ir::Op::Drop { place } => place.local as i64,\n        ir::Op::DropIf { place } => place.local as i64 + 1,\n        ir::Op::Unary { dst, op, src } => {\n            *dst as i64 + op.as_str().as_bytes()[0] as i64 + *src as i64\n        }\n        ir::Op::Binary {\n            dst,\n            op,\n            left,\n            right,\n        } => *dst as i64 + op.as_str().as_bytes()[0] as i64 + *left as i64 + *right as i64,\n        ir::Op::Call {\n            dst,\n            function,\n            args,\n        } => *dst as i64 + *function as i64 + args[0] as i64 + args[1] as i64,\n        ir::Op::Struct { dst, fields } => *dst as i64 + fields[0] as i64 + fields[1] as i64,\n        ir::Op::Variant {\n            dst,\n            variant,\n            fields,\n        } => *dst as i64 + *variant as i64 + fields[0] as i64 + fields[1] as i64,\n        ir::Op::Discriminant { dst, src } => *dst as i64 + *src as i64,\n        ir::Op::TagTest {\n            dst,\n            place,\n            variant,\n        } => *dst as i64 + place.local as i64 + *variant as i64,\n        ir::Op::Intrinsic { dst, name, args } => {\n            *dst as i64 + name.as_str().len() as i64 + args[0] as i64 + args[1] as i64\n        }\n        ir::Op::If { condition } => *condition as i64,\n        ir::Op::Else => 1001,\n        ir::Op::End => 1002,\n        ir::Op::Loop => 1003,\n        ir::Op::Break => 1004,\n        ir::Op::Continue => 1005,\n        ir::Op::BreakTo { depth } => 1006 + *depth as i64,\n        ir::Op::ContinueTo { depth } => 1007 + *depth as i64,\n        ir::Op::Return { src } => *src as i64,\n        ir::Op::Trap { message } => message.len() as i64,\n    }\n}\n\nfn kind_value(kind: &ir::TypeKind) -> i64 {\n    match kind {\n        ir::TypeKind::Unit => 1,\n        ir::TypeKind::Bool => 2,\n        ir::TypeKind::Char => 5,\n        ir::TypeKind::Int { bits, signed } => *bits as i64 * 2 + flag(signed),\n        ir::TypeKind::F64 => 3,\n        ir::TypeKind::Struct { fields, copy } => {\n            fields.len() as i64 + fields[0].name.len() as i64 + fields[0].ty as i64 + flag(copy)\n        }\n        ir::TypeKind::Enum { variants, copy } => {\n            variants.len() as i64\n                + variants[0].name.len() as i64\n                + variants[1].fields[0].ty as i64\n                + flag(copy)\n        }\n        ir::TypeKind::Ref { inner, mutable } => *inner as i64 + flag(mutable),\n        ir::TypeKind::Slice { inner, mutable } => *inner as i64 + flag(mutable),\n        ir::TypeKind::Array { inner, len } => *inner as i64 + *len as i64,\n        ir::TypeKind::Vec { inner } => *inner as i64,\n        ir::TypeKind::Box { inner } => *inner as i64,\n        ir::TypeKind::Str => 4,\n        ir::TypeKind::World => 5,\n        ir::TypeKind::Stdout => 6,\n        ir::TypeKind::Stdin => 7,\n        ir::TypeKind::Stderr => 8,\n        ir::TypeKind::Directory => 9,\n        ir::TypeKind::File => 10,\n        ir::TypeKind::Process => 11,\n        ir::TypeKind::Clock => 12,\n        ir::TypeKind::Child => 13,\n    }\n}\n\nfn model() -> ir::Program {\n    let mut types = ir::primitives();\n    types.push(ir::Type {\n        name: \"record\".to_string(),\n        kind: ir::TypeKind::Struct {\n            fields: vec![ir::Field {\n                name: \"field\\0水\".to_string(),\n                ty: ir::I64,\n            }],\n            copy: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"choice\".to_string(),\n        kind: ir::TypeKind::Enum {\n            variants: vec![\n                ir::Variant {\n                    name: \"Empty\".to_string(),\n                    fields: Vec::new(),\n                },\n                ir::Variant {\n                    name: \"Full\".to_string(),\n                    fields: vec![ir::Field {\n                        name: \"value\".to_string(),\n                        ty: ir::I64,\n                    }],\n                },\n            ],\n            copy: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"reference\".to_string(),\n        kind: ir::TypeKind::Ref {\n            inner: ir::I64,\n            mutable: true,\n        },\n    });\n    types.push(ir::Type {\n        name: \"slice\".to_string(),\n        kind: ir::TypeKind::Slice {\n            inner: ir::U8,\n            mutable: false,\n        },\n    });\n    types.push(ir::Type {\n        name: \"array\".to_string(),\n        kind: ir::TypeKind::Array {\n            inner: ir::BOOL,\n            len: 17,\n        },\n    });\n    types.push(ir::Type {\n        name: \"vector\".to_string(),\n        kind: ir::TypeKind::Vec { inner: ir::I32 },\n    });\n    types.push(ir::Type {\n        name: \"box\".to_string(),\n        kind: ir::TypeKind::Box { inner: ir::U64 },\n    });\n    types.push(ir::Type {\n        name: \"str\".to_string(),\n        kind: ir::TypeKind::Str,\n    });\n    types.push(ir::Type {\n        name: \"world\".to_string(),\n        kind: ir::TypeKind::World,\n    });\n    types.push(ir::Type {\n        name: \"stdout\".to_string(),\n        kind: ir::TypeKind::Stdout,\n    });\n    types.push(ir::Type {\n        name: \"stdin\".to_string(),\n        kind: ir::TypeKind::Stdin,\n    });\n    types.push(ir::Type {\n        name: \"stderr\".to_string(),\n        kind: ir::TypeKind::Stderr,\n    });\n    types.push(ir::Type {\n        name: \"directory\".to_string(),\n        kind: ir::TypeKind::Directory,\n    });\n    types.push(ir::Type {\n        name: \"file\".to_string(),\n        kind: ir::TypeKind::File,\n    });\n    let mut body: Vec<ir::Instruction> = Vec::new();\n    let mut index: usize = 0;\n    while index < 30 {\n        body.push(ir::Instruction {\n            column: 1,\n            op: operation(index, false),\n            line: 100 + index,\n        });\n        index = index + 1;\n    }\n    let function = ir::Function {\n        file: 2,\n        name: \"function\\0é水😀\".to_string(),\n        params: 1,\n        result: ir::I64,\n        return_source: Option::Some(ir::ReturnSource::Parameter {\n            index: 0,\n            projections: vec![\n                ir::Projection::Deref,\n                ir::Projection::Field(3),\n                ir::Projection::VariantField(2, 4),\n                ir::Projection::Index(9),\n            ],\n        }),\n        locals: vec![ir::Local {\n            name: \"parameter\".to_string(),\n            ty: ir::I64,\n            mutable: true,\n        }],\n        body,\n        test: true,\n    };\n    ir::Program {\n        types,\n        functions: vec![function],\n        destructors: vec![(7usize, 0usize)],\n    }\n}\n\npub fn primitive_snapshot(index: usize, field: usize) -> i64 {\n    let original = ir::primitives();\n    let copied = original.clone();\n    drop(original);\n    if field == 0 {\n        return kind_value(&copied[index].kind);\n    }\n    if field == 1 {\n        return copied[index].name.len() as i64;\n    }\n    let bytes = copied[index].name.as_bytes();\n    if field - 2 < bytes.len() {\n        bytes[field - 2] as i64\n    } else {\n        -1\n    }\n}\n\npub fn capability_snapshot(which: usize) -> i64 {\n    let original = match which {\n        0 => ir::TypeKind::Process,\n        1 => ir::TypeKind::Clock,\n        _ => ir::TypeKind::Child,\n    };\n    let copied = original.clone();\n    assert!(copied == original);\n    drop(original);\n    let different = if which == 0 {\n        ir::TypeKind::Clock\n    } else {\n        ir::TypeKind::Process\n    };\n    assert!(copied != different);\n    kind_value(&copied)\n}\n\npub fn operation_snapshot(which: usize) -> i64 {\n    let original = operation(which, false);\n    let copied = original.clone();\n    assert!(original == copied);\n    drop(original);\n    operation_value(&copied)\n}\n\npub fn operation_equality(which: usize, changed: bool) -> i64 {\n    let original = operation(which, false);\n    let candidate = operation(which, changed);\n    let result = original == candidate;\n    assert!((original != candidate) == !result);\n    if result { 1 } else { 0 }\n}\n\npub fn program_snapshot(field: usize) -> i64 {\n    let mut original = model();\n    let copied = original.clone();\n    assert!(copied == original);\n    original.functions[0].name.push_str(\" changed\");\n    original.functions[0].body[0].op = operation(0, true);\n    original.destructors[0].0 = 99;\n    assert!(copied != original);\n    drop(original);\n    let checked = ir::CheckedProgram { program: copied };\n    let retained: ir::Program = checked.program().clone();\n    drop(checked);\n    assert!(retained == model());\n    if field == 0 {\n        let mut total: i64 = 0;\n        let mut index: usize = 0;\n        while index < retained.types.len() {\n            total = total + kind_value(&retained.types[index].kind);\n            index = index + 1;\n        }\n        return total;\n    }\n    if field == 1 {\n        let mut total: i64 = 0;\n        let mut index: usize = 0;\n        while index < retained.functions[0].body.len() {\n            total = total + operation_value(&retained.functions[0].body[index].op);\n            total = total + retained.functions[0].body[index].line as i64;\n            index = index + 1;\n        }\n        return total;\n    }\n    if field == 2 {\n        return retained.functions[0].name.len() as i64;\n    }\n    if field == 3 {\n        return retained.destructors[0].0 as i64;\n    }\n    if field == 4 {\n        return retained.functions[0].locals[0].name.len() as i64;\n    }\n    match &retained.functions[0].return_source {\n        Option::None => -1,\n        Option::Some(source) => match source {\n            ir::ReturnSource::Static => -2,\n            ir::ReturnSource::Leaves(leaves) => leaves.len() as i64,\n            ir::ReturnSource::Parameter { index, projections } => {\n                *index as i64 + projections.len() as i64\n            }\n        },\n    }\n}\n\nfn projections_value(projections: &Vec<ir::Projection>) -> i64 {\n    let mut result: i64 = 0;\n    let mut cursor: usize = 0;\n    while cursor < projections.len() {\n        result = result\n            + match &projections[cursor] {\n                ir::Projection::Deref => 1,\n                ir::Projection::Field(value) => *value as i64,\n                ir::Projection::VariantField(tag, value) => *tag as i64 + *value as i64,\n                ir::Projection::Index(value) => *value as i64,\n            };\n        cursor = cursor + 1;\n    }\n    result\n}\n\npub fn return_source_snapshot(which: usize) -> i64 {\n    let original: Option<ir::ReturnSource> = if which == 0 {\n        Option::None\n    } else if which == 1 {\n        Option::Some(ir::ReturnSource::Static)\n    } else if which == 2 {\n        Option::Some(ir::ReturnSource::Parameter {\n            index: 17,\n            projections: vec![\n                ir::Projection::Deref,\n                ir::Projection::Field(2),\n                ir::Projection::VariantField(3, 4),\n                ir::Projection::Index(5),\n            ],\n        })\n    } else {\n        let origin = if which == 5 {\n            ir::LeafOrigin::Static\n        } else {\n            ir::LeafOrigin::Inputs {\n                inputs: vec![\n                    ir::InputLeaf {\n                        index: 11,\n                        projections: vec![ir::Projection::Deref, ir::Projection::Field(7)],\n                    },\n                    ir::InputLeaf {\n                        index: 13,\n                        projections: vec![ir::Projection::Field(2)],\n                    },\n                ],\n                exact: which == 3,\n            }\n        };\n        Option::Some(ir::ReturnSource::Leaves(vec![ir::ReturnLeaf {\n            projections: vec![ir::Projection::VariantField(2, 9)],\n            origin,\n        }]))\n    };\n    let copied = original.clone();\n    assert!(copied == original);\n    drop(original);\n    match copied {\n        Option::None => 100,\n        Option::Some(source) => match source {\n            ir::ReturnSource::Static => 101,\n            ir::ReturnSource::Parameter { index, projections } => {\n                index as i64 + projections_value(&projections)\n            }\n            ir::ReturnSource::Leaves(leaves) => {\n                let mut result: i64 = 0;\n                let mut cursor: usize = 0;\n                while cursor < leaves.len() {\n                    result = result + projections_value(&leaves[cursor].projections);\n                    result = result\n                        + match &leaves[cursor].origin {\n                            ir::LeafOrigin::Static => 103,\n                            ir::LeafOrigin::Inputs { inputs, exact } => {\n                                let mut value = flag(exact);\n                                let mut index: usize = 0;\n                                while index < inputs.len() {\n                                    value = value\n                                        + inputs[index].index as i64\n                                        + projections_value(&inputs[index].projections);\n                                    index = index + 1;\n                                }\n                                value\n                            }\n                        };\n                    cursor = cursor + 1;\n                }\n                result\n            }\n        },\n    }\n}\n\npub fn production_place_local() -> usize {\n    ir::Place::local(42).local\n}\n")),
source_file("src/ir.rs", Some("//! Explicit, monomorphic meaning. Only CheckedProgram reaches a backend.\n#[derive(Clone, PartialEq)]\npub enum TypeKind {\n    Unit,\n    Bool,\n    /// A Unicode scalar value: 0..=0x10ffff excluding 0xd800..=0xdfff.\n    Char,\n    Int {\n        bits: u8,\n        signed: bool,\n    },\n    F64,\n    Struct {\n        fields: Vec<Field>,\n        copy: bool,\n    },\n    Enum {\n        variants: Vec<Variant>,\n        copy: bool,\n    },\n    Ref {\n        inner: usize,\n        mutable: bool,\n    },\n    Slice {\n        inner: usize,\n        mutable: bool,\n    },\n    Array {\n        inner: usize,\n        len: usize,\n    },\n    Vec {\n        inner: usize,\n    },\n    Box {\n        inner: usize,\n    },\n    /// Unsized immutable UTF-8 text; stored values use a shared Ref.\n    Str,\n    World,\n    /// Affine authority to write standard output, derived from World.\n    Stdout,\n    Stdin,\n    Stderr,\n    /// Affine directory authority; descendants remain beneath this root.\n    Directory,\n    /// Affine open file; structural destruction closes its handle.\n    File,\n    Process,\n    Clock,\n    Child,\n}\n\n#[derive(Clone, PartialEq)]\npub struct Field {\n    pub name: String,\n    pub ty: usize,\n}\n#[derive(Clone, PartialEq)]\npub struct Variant {\n    pub name: String,\n    pub fields: Vec<Field>,\n}\n#[derive(Clone, PartialEq)]\npub struct Type {\n    pub name: String,\n    pub kind: TypeKind,\n}\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub enum Projection {\n    Deref,\n    Field(usize),\n    VariantField(usize, usize),\n    Index(usize),\n}\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub struct Place {\n    pub local: usize,\n    pub projections: Vec<Projection>,\n}\nimpl Place {\n    pub fn local(local: usize) -> Place {\n        Place {\n            local: local + 1,\n            projections: Vec::new(),\n        }\n    }\n}\n#[derive(Clone, PartialEq)]\npub struct Local {\n    pub name: String,\n    pub ty: usize,\n    pub mutable: bool,\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum UnaryOp {\n    Neg,\n    Not,\n}\nimpl UnaryOp {\n    pub fn parse(text: &str) -> Option<UnaryOp> {\n        match text {\n            \"-\" => Some(UnaryOp::Neg),\n            \"!\" => Some(UnaryOp::Not),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            UnaryOp::Neg => \"-\",\n            UnaryOp::Not => \"!\",\n        }\n    }\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum BinaryOp {\n    Add,\n    Sub,\n    Mul,\n    Div,\n    Rem,\n    BitAnd,\n    BitOr,\n    BitXor,\n    Shl,\n    Shr,\n    Eq,\n    Ne,\n    Lt,\n    Le,\n    Gt,\n    Ge,\n    And,\n    Or,\n    WrappingAdd,\n    WrappingSub,\n    WrappingMul,\n}\nimpl BinaryOp {\n    pub fn parse(text: &str) -> Option<BinaryOp> {\n        match text {\n            \"+\" => Some(BinaryOp::Add),\n            \"-\" => Some(BinaryOp::Sub),\n            \"*\" => Some(BinaryOp::Mul),\n            \"/\" => Some(BinaryOp::Div),\n            \"%\" => Some(BinaryOp::Rem),\n            \"&\" => Some(BinaryOp::BitAnd),\n            \"|\" => Some(BinaryOp::BitOr),\n            \"^\" => Some(BinaryOp::BitXor),\n            \"<<\" => Some(BinaryOp::Shl),\n            \">>\" => Some(BinaryOp::Shr),\n            \"==\" => Some(BinaryOp::Eq),\n            \"!=\" => Some(BinaryOp::Ne),\n            \"<\" => Some(BinaryOp::Lt),\n            \"<=\" => Some(BinaryOp::Le),\n            \">\" => Some(BinaryOp::Gt),\n            \">=\" => Some(BinaryOp::Ge),\n            \"&&\" => Some(BinaryOp::And),\n            \"||\" => Some(BinaryOp::Or),\n            \"wrapping_add\" => Some(BinaryOp::WrappingAdd),\n            \"wrapping_sub\" => Some(BinaryOp::WrappingSub),\n            \"wrapping_mul\" => Some(BinaryOp::WrappingMul),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            BinaryOp::Add => \"+\",\n            BinaryOp::Sub => \"-\",\n            BinaryOp::Mul => \"*\",\n            BinaryOp::Div => \"/\",\n            BinaryOp::Rem => \"%\",\n            BinaryOp::BitAnd => \"&\",\n            BinaryOp::BitOr => \"|\",\n            BinaryOp::BitXor => \"^\",\n            BinaryOp::Shl => \"<<\",\n            BinaryOp::Shr => \">>\",\n            BinaryOp::Eq => \"==\",\n            BinaryOp::Ne => \"!=\",\n            BinaryOp::Lt => \"<\",\n            BinaryOp::Le => \"<=\",\n            BinaryOp::Gt => \">\",\n            BinaryOp::Ge => \">=\",\n            BinaryOp::And => \"&&\",\n            BinaryOp::Or => \"||\",\n            BinaryOp::WrappingAdd => \"wrapping_add\",\n            BinaryOp::WrappingSub => \"wrapping_sub\",\n            BinaryOp::WrappingMul => \"wrapping_mul\",\n        }\n    }\n}\n/// Closed operation vocabulary; textual spellings are only an interchange format.\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum IntrinsicOp {\n    CharFromU32,\n    F64FromBits,\n    F64ToBits,\n    ArrayRepeat,\n    StrLen,\n    StrAsBytes,\n    StrFromUtf8,\n    Assert,\n    VecNew,\n    VecLen,\n    VecPush,\n    VecPop,\n    Replace,\n    BoxNew,\n    Cast,\n    Clone,\n    WorldStdout,\n    WorldStdin,\n    WorldStderr,\n    WorldArgCount,\n    WorldArgLen,\n    WorldArgCopy,\n    WorldExitStatus,\n    WorldDirectory,\n    DirectoryOpen,\n    StdinRead,\n    FileRead,\n    FileWrite,\n    StdoutWrite,\n    StderrWrite,\n    WorldProcess,\n    WorldClock,\n    WorldFile,\n    ClockRead,\n    ProcessSpawn,\n    ChildWait,\n    ChildTerminate,\n    DirectoryOpenDirectory,\n    DirectoryRead,\n    DirectoryRewind,\n    DirectoryCreate,\n    DirectoryRemove,\n    DirectoryRename,\n    FileRewind,\n    FileExecutable,\n    FileDuplicate,\n    FileReadOnly,\n}\nimpl IntrinsicOp {\n    pub fn parse(text: &str) -> Option<IntrinsicOp> {\n        match text {\n            \"char_from_u32\" => Some(IntrinsicOp::CharFromU32),\n            \"f64_from_bits\" => Some(IntrinsicOp::F64FromBits),\n            \"f64_to_bits\" => Some(IntrinsicOp::F64ToBits),\n            \"array_repeat\" => Some(IntrinsicOp::ArrayRepeat),\n            \"str_len\" => Some(IntrinsicOp::StrLen),\n            \"str_as_bytes\" => Some(IntrinsicOp::StrAsBytes),\n            \"str_from_utf8\" => Some(IntrinsicOp::StrFromUtf8),\n            \"assert\" => Some(IntrinsicOp::Assert),\n            \"vec_new\" => Some(IntrinsicOp::VecNew),\n            \"vec_len\" => Some(IntrinsicOp::VecLen),\n            \"vec_push\" => Some(IntrinsicOp::VecPush),\n            \"vec_pop\" => Some(IntrinsicOp::VecPop),\n            \"replace\" => Some(IntrinsicOp::Replace),\n            \"box_new\" => Some(IntrinsicOp::BoxNew),\n            \"cast\" => Some(IntrinsicOp::Cast),\n            \"clone\" => Some(IntrinsicOp::Clone),\n            \"world_stdout\" => Some(IntrinsicOp::WorldStdout),\n            \"world_stdin\" => Some(IntrinsicOp::WorldStdin),\n            \"world_stderr\" => Some(IntrinsicOp::WorldStderr),\n            \"world_arg_count\" => Some(IntrinsicOp::WorldArgCount),\n            \"world_arg_len\" => Some(IntrinsicOp::WorldArgLen),\n            \"world_arg_copy\" => Some(IntrinsicOp::WorldArgCopy),\n            \"world_exit_status\" => Some(IntrinsicOp::WorldExitStatus),\n            \"world_directory\" => Some(IntrinsicOp::WorldDirectory),\n            \"directory_open\" => Some(IntrinsicOp::DirectoryOpen),\n            \"stdin_read\" => Some(IntrinsicOp::StdinRead),\n            \"file_read\" => Some(IntrinsicOp::FileRead),\n            \"file_write\" => Some(IntrinsicOp::FileWrite),\n            \"stdout_write\" => Some(IntrinsicOp::StdoutWrite),\n            \"stderr_write\" => Some(IntrinsicOp::StderrWrite),\n            \"world_process\" => Some(IntrinsicOp::WorldProcess),\n            \"world_clock\" => Some(IntrinsicOp::WorldClock),\n            \"world_file\" => Some(IntrinsicOp::WorldFile),\n            \"clock_read\" => Some(IntrinsicOp::ClockRead),\n            \"process_spawn\" => Some(IntrinsicOp::ProcessSpawn),\n            \"child_wait\" => Some(IntrinsicOp::ChildWait),\n            \"child_terminate\" => Some(IntrinsicOp::ChildTerminate),\n            \"directory_open_directory\" => Some(IntrinsicOp::DirectoryOpenDirectory),\n            \"directory_read\" => Some(IntrinsicOp::DirectoryRead),\n            \"directory_rewind\" => Some(IntrinsicOp::DirectoryRewind),\n            \"directory_create\" => Some(IntrinsicOp::DirectoryCreate),\n            \"directory_remove\" => Some(IntrinsicOp::DirectoryRemove),\n            \"directory_rename\" => Some(IntrinsicOp::DirectoryRename),\n            \"file_rewind\" => Some(IntrinsicOp::FileRewind),\n            \"file_executable\" => Some(IntrinsicOp::FileExecutable),\n            \"file_duplicate\" => Some(IntrinsicOp::FileDuplicate),\n            \"file_readonly\" => Some(IntrinsicOp::FileReadOnly),\n            _ => None,\n        }\n    }\n    pub fn as_str(self) -> &'static str {\n        match self {\n            IntrinsicOp::CharFromU32 => \"char_from_u32\",\n            IntrinsicOp::F64FromBits => \"f64_from_bits\",\n            IntrinsicOp::F64ToBits => \"f64_to_bits\",\n            IntrinsicOp::ArrayRepeat => \"array_repeat\",\n            IntrinsicOp::StrLen => \"str_len\",\n            IntrinsicOp::StrAsBytes => \"str_as_bytes\",\n            IntrinsicOp::StrFromUtf8 => \"str_from_utf8\",\n            IntrinsicOp::Assert => \"assert\",\n            IntrinsicOp::VecNew => \"vec_new\",\n            IntrinsicOp::VecLen => \"vec_len\",\n            IntrinsicOp::VecPush => \"vec_push\",\n            IntrinsicOp::VecPop => \"vec_pop\",\n            IntrinsicOp::Replace => \"replace\",\n            IntrinsicOp::BoxNew => \"box_new\",\n            IntrinsicOp::Cast => \"cast\",\n            IntrinsicOp::Clone => \"clone\",\n            IntrinsicOp::WorldStdout => \"world_stdout\",\n            IntrinsicOp::WorldStdin => \"world_stdin\",\n            IntrinsicOp::WorldStderr => \"world_stderr\",\n            IntrinsicOp::WorldArgCount => \"world_arg_count\",\n            IntrinsicOp::WorldArgLen => \"world_arg_len\",\n            IntrinsicOp::WorldArgCopy => \"world_arg_copy\",\n            IntrinsicOp::WorldExitStatus => \"world_exit_status\",\n            IntrinsicOp::WorldDirectory => \"world_directory\",\n            IntrinsicOp::DirectoryOpen => \"directory_open\",\n            IntrinsicOp::StdinRead => \"stdin_read\",\n            IntrinsicOp::FileRead => \"file_read\",\n            IntrinsicOp::FileWrite => \"file_write\",\n            IntrinsicOp::StdoutWrite => \"stdout_write\",\n            IntrinsicOp::StderrWrite => \"stderr_write\",\n            IntrinsicOp::WorldProcess => \"world_process\",\n            IntrinsicOp::WorldClock => \"world_clock\",\n            IntrinsicOp::WorldFile => \"world_file\",\n            IntrinsicOp::ClockRead => \"clock_read\",\n            IntrinsicOp::ProcessSpawn => \"process_spawn\",\n            IntrinsicOp::ChildWait => \"child_wait\",\n            IntrinsicOp::ChildTerminate => \"child_terminate\",\n            IntrinsicOp::DirectoryOpenDirectory => \"directory_open_directory\",\n            IntrinsicOp::DirectoryRead => \"directory_read\",\n            IntrinsicOp::DirectoryRewind => \"directory_rewind\",\n            IntrinsicOp::DirectoryCreate => \"directory_create\",\n            IntrinsicOp::DirectoryRemove => \"directory_remove\",\n            IntrinsicOp::DirectoryRename => \"directory_rename\",\n            IntrinsicOp::FileRewind => \"file_rewind\",\n            IntrinsicOp::FileExecutable => \"file_executable\",\n            IntrinsicOp::FileDuplicate => \"file_duplicate\",\n            IntrinsicOp::FileReadOnly => \"file_readonly\",\n        }\n    }\n}\n#[derive(Clone, PartialEq)]\npub enum Op {\n    /// Immutable module-long bytes: a UTF-8 str or an exact-length u8 array.\n    Text {\n        dst: usize,\n        bytes: Vec<u8>,\n    },\n    Int {\n        dst: usize,\n        value: i64,\n    },\n    Float {\n        dst: usize,\n        bits: u64,\n    },\n    Unit {\n        dst: usize,\n    },\n    Read {\n        dst: usize,\n        place: Place,\n        moving: bool,\n    },\n    Borrow {\n        dst: usize,\n        place: Place,\n        mutable: bool,\n    },\n    SliceBorrow {\n        dst: usize,\n        place: Place,\n        mutable: bool,\n    },\n    /// Borrow a checked subrange without narrowing its static alias footprint.\n    /// Bounds are consumed usize values; failure traps with the bounds code.\n    ViewRange {\n        dst: usize,\n        place: Place,\n        start: usize,\n        end: usize,\n        inclusive: bool,\n        mutable: bool,\n    },\n    Array {\n        dst: usize,\n        elements: Vec<usize>,\n    },\n    Assign {\n        place: Place,\n        src: usize,\n    },\n    Drop {\n        place: Place,\n    },\n    /// Destroy this place only when its own storage is initialized. Every\n    /// projected ancestor must still exist; this does not authorize a hole\n    /// through a reference or an indexed container.\n    DropIf {\n        place: Place,\n    },\n    Unary {\n        dst: usize,\n        op: UnaryOp,\n        src: usize,\n    },\n    Binary {\n        dst: usize,\n        op: BinaryOp,\n        left: usize,\n        right: usize,\n    },\n    Call {\n        dst: usize,\n        function: usize,\n        args: Vec<usize>,\n    },\n    Struct {\n        dst: usize,\n        fields: Vec<usize>,\n    },\n    Variant {\n        dst: usize,\n        variant: usize,\n        fields: Vec<usize>,\n    },\n    Discriminant {\n        dst: usize,\n        src: usize,\n    },\n    TagTest {\n        dst: usize,\n        place: Place,\n        variant: usize,\n    },\n    Intrinsic {\n        dst: usize,\n        name: IntrinsicOp,\n        args: Vec<usize>,\n    },\n    If {\n        condition: usize,\n    },\n    Else,\n    End,\n    Loop,\n    Break,\n    Continue,\n    /// Select an enclosing lexical loop, counting only loops (zero is nearest).\n    BreakTo {\n        depth: usize,\n    },\n    ContinueTo {\n        depth: usize,\n    },\n    Return {\n        src: usize,\n    },\n    Trap {\n        message: String,\n    },\n}\n#[derive(Clone, PartialEq)]\npub struct Instruction {\n    pub op: Op,\n    pub line: usize,\n    pub column: usize,\n}\n/// An input selector names one stored reference leaf in a parameter value.\n#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\npub struct InputLeaf {\n    pub index: usize,\n    pub projections: Vec<Projection>,\n}\n/// Every result reference leaf selects a declared input alternative or static storage.\n#[derive(Clone, PartialEq, Eq)]\npub enum LeafOrigin {\n    Inputs {\n        inputs: Vec<InputLeaf>,\n        /// Exact preserves an input referent; bounded permits a descendant.\n        exact: bool,\n    },\n    Static,\n}\n#[derive(Clone, PartialEq, Eq)]\npub struct ReturnLeaf {\n    pub projections: Vec<Projection>,\n    pub origin: LeafOrigin,\n}\n#[derive(Clone, PartialEq, Eq)]\npub enum ReturnSource {\n    /// A complete, unique contract for every reference leaf in the result.\n    Leaves(Vec<ReturnLeaf>),\n    /// Reference leaves in a parameter value, after the declared dereferences.\n    /// Projected sources currently require shared leaves; exclusive reborrows\n    /// retain the lifetime of their outer reference.\n    Parameter {\n        index: usize,\n        projections: Vec<Projection>,\n    },\n    Static,\n}\n#[derive(Clone, PartialEq)]\npub struct Function {\n    pub file: usize,\n    pub name: String,\n    pub params: usize,\n    pub result: usize,\n    pub return_source: Option<ReturnSource>,\n    pub locals: Vec<Local>,\n    pub body: Vec<Instruction>,\n    pub test: bool,\n}\n#[derive(Clone, PartialEq)]\npub struct Program {\n    pub types: Vec<Type>,\n    pub functions: Vec<Function>,\n    pub destructors: Vec<(usize, usize)>,\n}\npub struct CheckedProgram {\n    pub(crate) program: Program,\n}\nimpl CheckedProgram {\n    pub fn program(&self) -> &Program {\n        &self.program\n    }\n}\npub const UNIT: usize = 0;\npub const BOOL: usize = 1;\npub const I32: usize = 2;\npub const I64: usize = 3;\npub const U64: usize = 4;\npub const U8: usize = 5;\npub const F64: usize = 6;\npub fn primitives() -> Vec<Type> {\n    vec![\n        Type {\n            name: \"()\".to_string(),\n            kind: TypeKind::Unit,\n        },\n        Type {\n            name: \"bool\".to_string(),\n            kind: TypeKind::Bool,\n        },\n        Type {\n            name: \"i32\".to_string(),\n            kind: TypeKind::Int {\n                bits: 31,\n                signed: true,\n            },\n        },\n        Type {\n            name: \"i64\".to_string(),\n            kind: TypeKind::Int {\n                bits: 64,\n                signed: true,\n            },\n        },\n        Type {\n            name: \"usize\".to_string(),\n            kind: TypeKind::Int {\n                bits: 64,\n                signed: false,\n            },\n        },\n        Type {\n            name: \"u8\".to_string(),\n            kind: TypeKind::Int {\n                bits: 8,\n                signed: false,\n            },\n        },\n        Type {\n            name: \"f64\".to_string(),\n            kind: TypeKind::F64,\n        },\n    ]\n}\n")),
], ""), errors);
        count += 1;
    }
    if 355usize % workers == shard {
        record(source_case("production_json_encoder_executes_and_round_trips_every_output_byte_0000", "json_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("json_component.rs", Some("mod json;\npub fn release(bytes:Vec<u8>) { drop(bytes); }\npub fn encode_0()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_1()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"plain ASCII\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_2()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"\\\"quoted\\\" \\\\ slash /\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_3()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f}\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_4()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\" !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\\u{80}\\u{81}\\u{82}\\u{83}\\u{84}\\u{85}\\u{86}\\u{87}\\u{88}\\u{89}\\u{8a}\\u{8b}\\u{8c}\\u{8d}\\u{8e}\\u{8f}\\u{90}\\u{91}\\u{92}\\u{93}\\u{94}\\u{95}\\u{96}\\u{97}\\u{98}\\u{99}\\u{9a}\\u{9b}\\u{9c}\\u{9d}\\u{9e}\\u{9f}\\u{a0}¡¢£¤¥¦§¨©ª«¬\\u{ad}®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_5()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"é 水 😀 \\u{2028}\\u{2029}\\u{10ffff}\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_6()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水é\\\"\\\\\\n水\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn main(mut world:World) { let mut out=world.stdout();\nlet value=encode_0();out.write(&value);\nlet value=encode_1();out.write(&value);\nlet value=encode_2();out.write(&value);\nlet value=encode_3();out.write(&value);\nlet value=encode_4();out.write(&value);\nlet value=encode_5();out.write(&value);\nlet value=encode_6();out.write(&value);\n}\n")),
source_file("json.rs", Some("//! JSON string bytes used by the emitter's trap metadata.\n//! Input is validated UTF-8; non-control bytes retain their original encoding.\n\nfn hex_digit(value: u8) -> u8 {\n    match value {\n        0..=9 => b'0' + value,\n        10..=15 => b'a' + (value - 10),\n        _ => panic!(\"invalid hexadecimal digit\"),\n    }\n}\n\n/// Append one quoted JSON string, preserving the emitter's lowercase escapes.\npub fn write_string(out: &mut Vec<u8>, text: &str) {\n    out.push(b'\"');\n    let bytes = text.as_bytes();\n    let mut index: usize = 0;\n    while index < bytes.len() {\n        let byte = bytes[index];\n        match byte {\n            b'\"' => {\n                out.push(b'\\\\');\n                out.push(b'\"');\n            }\n            b'\\\\' => {\n                out.push(b'\\\\');\n                out.push(b'\\\\');\n            }\n            0..=31 => {\n                out.push(b'\\\\');\n                out.push(b'u');\n                out.push(b'0');\n                out.push(b'0');\n                out.push(hex_digit(byte >> 4));\n                out.push(hex_digit(byte & 15));\n            }\n            _ => out.push(byte),\n        }\n        index = index + 1;\n    }\n    out.push(b'\"');\n}\n")),
], ""), errors);
        count += 1;
    }
    if 356usize % workers == shard {
        record(source_case("production_qualified_names_match_rust_over_every_output_byte_0000", "text_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_component.rs", Some("mod names; use names::qualified as join;\npub fn compare_0()->usize {\nlet value=join(\"\",\"\");\nlet actual=value.as_bytes();\nlet expected=\"\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\npub fn compare_1()->usize {\nlet value=join(\"\",\"root\");\nlet actual=value.as_bytes();\nlet expected=\"root\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\npub fn compare_2()->usize {\nlet value=join(\"outer::inner\",\"leaf\");\nlet actual=value.as_bytes();\nlet expected=\"outer::inner::leaf\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\npub fn compare_3()->usize {\nlet value=join(\"α::水\",\"😀\\0leaf\\u{10ffff}\");\nlet actual=value.as_bytes();\nlet expected=\"α::水::😀\\0leaf\\u{10ffff}\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\npub fn compare_4()->usize {\nlet value=join(\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\",\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\");\nlet actual=value.as_bytes();\nlet expected=\"\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}::\\0\\u{1}\\u{2}\\u{3}\\u{4}\\u{5}\\u{6}\\u{7}\\u{8}\\t\\n\\u{b}\\u{c}\\r\\u{e}\\u{f}\\u{10}\\u{11}\\u{12}\\u{13}\\u{14}\\u{15}\\u{16}\\u{17}\\u{18}\\u{19}\\u{1a}\\u{1b}\\u{1c}\\u{1d}\\u{1e}\\u{1f} !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\u{7f}\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\npub fn compare_5()->usize {\nlet value=join(\"module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::\",\"水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀\");\nlet actual=value.as_bytes();\nlet expected=\"module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::module_segment::::水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀水😀\".as_bytes();\nif actual.len()!=expected.len(){return 1;}\nlet mut index:usize=0;\nwhile index<actual.len(){\nif actual[index]!=expected[index]{return index+2;}\nindex=index+1;\n}\n0\n}\n")),
source_file("names.rs", Some("//! Canonical declaration names used by resolution, generic validation, and elaboration.\n/// Join one module path and declaration spelling without changing either input.\npub fn qualified(module: &str, name: &str) -> String {\n    let mut result = module.to_string();\n    if !module.is_empty() {\n        result.push_str(\"::\");\n    }\n    result.push_str(name);\n    result\n}\n")),
], ""), errors);
        count += 1;
    }
    if 357usize % workers == shard {
        record(source_case("production_range_mutation_changes_artifact_and_json_behavior_0000", "json_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("json_component.rs", Some("mod json;\npub fn release(bytes:Vec<u8>) { drop(bytes); }\npub fn encode_0()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"\\u{1f}\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_1()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"ordinary\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn main(mut world:World) { let mut out=world.stdout();\nlet value=encode_0();out.write(&value);\nlet value=encode_1();out.write(&value);\n}\n")),
source_file("json.rs", Some("//! JSON string bytes used by the emitter's trap metadata.\n//! Input is validated UTF-8; non-control bytes retain their original encoding.\n\nfn hex_digit(value: u8) -> u8 {\n    match value {\n        0..=9 => b'0' + value,\n        10..=15 => b'a' + (value - 10),\n        _ => panic!(\"invalid hexadecimal digit\"),\n    }\n}\n\n/// Append one quoted JSON string, preserving the emitter's lowercase escapes.\npub fn write_string(out: &mut Vec<u8>, text: &str) {\n    out.push(b'\"');\n    let bytes = text.as_bytes();\n    let mut index: usize = 0;\n    while index < bytes.len() {\n        let byte = bytes[index];\n        match byte {\n            b'\"' => {\n                out.push(b'\\\\');\n                out.push(b'\"');\n            }\n            b'\\\\' => {\n                out.push(b'\\\\');\n                out.push(b'\\\\');\n            }\n            0..=31 => {\n                out.push(b'\\\\');\n                out.push(b'u');\n                out.push(b'0');\n                out.push(b'0');\n                out.push(hex_digit(byte >> 4));\n                out.push(hex_digit(byte & 15));\n            }\n            _ => out.push(byte),\n        }\n        index = index + 1;\n    }\n    out.push(b'\"');\n}\n")),
], ""), errors);
        count += 1;
    }
    if 358usize % workers == shard {
        record(source_case("production_range_mutation_changes_artifact_and_json_behavior_0001", "json_component.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("json_component.rs", Some("mod json;\npub fn release(bytes:Vec<u8>) { drop(bytes); }\npub fn encode_0()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"\\u{1f}\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn encode_1()->Vec<u8> {\nlet mut bytes:Vec<u8>=Vec::new();\nbytes.push(b'[');\njson::write_string(&mut bytes,\"ordinary\");\nbytes.push(b',');\njson::write_string(&mut bytes,\"tail\");\nbytes.push(b']');\nbytes\n}\npub fn main(mut world:World) { let mut out=world.stdout();\nlet value=encode_0();out.write(&value);\nlet value=encode_1();out.write(&value);\n}\n")),
source_file("json.rs", Some("//! JSON string bytes used by the emitter's trap metadata.\n//! Input is validated UTF-8; non-control bytes retain their original encoding.\n\nfn hex_digit(value: u8) -> u8 {\n    match value {\n        0..=9 => b'0' + value,\n        10..=15 => b'a' + (value - 10),\n        _ => panic!(\"invalid hexadecimal digit\"),\n    }\n}\n\n/// Append one quoted JSON string, preserving the emitter's lowercase escapes.\npub fn write_string(out: &mut Vec<u8>, text: &str) {\n    out.push(b'\"');\n    let bytes = text.as_bytes();\n    let mut index: usize = 0;\n    while index < bytes.len() {\n        let byte = bytes[index];\n        match byte {\n            b'\"' => {\n                out.push(b'\\\\');\n                out.push(b'\"');\n            }\n            b'\\\\' => {\n                out.push(b'\\\\');\n                out.push(b'\\\\');\n            }\n            0..=30 => {\n                out.push(b'\\\\');\n                out.push(b'u');\n                out.push(b'0');\n                out.push(b'0');\n                out.push(hex_digit(byte >> 4));\n                out.push(hex_digit(byte & 15));\n            }\n            _ => out.push(byte),\n        }\n        index = index + 1;\n    }\n    out.push(b'\"');\n}\n")),
], ""), errors);
        count += 1;
    }
    if 359usize % workers == shard {
        record(source_case("production_sequence_mutations_change_vector_artifacts_and_behavior_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("\npub fn clone_probe()->i64 {\n    let values:Vec<i64>=vec![4,2];let copied=values.clone();\n    copied[0]*10+copied[1]\n}\npub fn eq_probe()->i64 {\n    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];\n    if left==right {1} else {0}\n}\npub fn ne_probe()->i64 {\n    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];\n    if left.ne(&right) {1} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 360usize % workers == shard {
        record(source_case("production_sequence_mutations_change_vector_artifacts_and_behavior_0004", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", Some("//! Vec operations are ordinary source over checked element access.\nuse crate::prelude::Option;\npub fn __lkjscript_vec_clone<T: Clone>(source: &Vec<T>) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    let mut index: usize = 0;\n    while index < source.len() {\n        result.push(source[0].clone());\n        index = index + 1;\n    }\n    result\n}\n\npub fn __lkjscript_vec_equal<T: PartialEq>(left: &Vec<T>, right: &Vec<T>) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index: usize = 0;\n    while index < left.len() {\n        // Equality uses the element's eq contract; ne may be independently overridden.\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index = index + 1;\n    }\n    true\n}\n\n// Clone the first count-1 elements and move the original into the last slot.\n// The element expression is evaluated even when count is zero.\npub fn __lkjscript_vec_repeat<T: Clone>(value: T, count: usize) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    if count == 0 {\n        return result;\n    }\n    let mut remaining = count - 1;\n    while remaining > 0 {\n        result.push(value.clone());\n        remaining = remaining - 1;\n    }\n    result.push(value);\n    result\n}\n\n/// Shared inspection does not require an element equality or copying contract.\npub trait VecAccess<T> {\n    fn is_empty(&self) -> bool;\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T>;\n    fn first<'a>(&'a self) -> Option<&'a T>;\n    fn last<'a>(&'a self) -> Option<&'a T>;\n}\n\nimpl<T> VecAccess<T> for Vec<T> {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\n\n/// Equality search borrows its elements and stops at the first match.\npub trait VecContains<T: PartialEq> {\n    fn contains(&self, value: &T) -> bool;\n}\n\nimpl<T: PartialEq> VecContains<T> for Vec<T> {\n    fn contains(&self, value: &T) -> bool {\n        let mut index: usize = 0;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index = index + 1;\n        }\n        false\n    }\n}\n\n/// Append independent clones in source order; the shared source remains usable.\npub trait VecExtend<T: Clone> {\n    fn extend_from_slice(&mut self, values: &[T]);\n}\n\nimpl<T: Clone> VecExtend<T> for Vec<T> {\n    fn extend_from_slice(&mut self, values: &[T]) {\n        let mut index: usize = 0;\n        while index < values.len() {\n            self.push(values[index].clone());\n            index = index + 1;\n        }\n    }\n}\n\n/// Mutation visits values in their original order and keeps affine ownership.\npub trait VecMutate<T> {\n    fn retain<F: FnMut(&T) -> bool>(&mut self, predicate: F);\n}\nimpl<T> VecMutate<T> for Vec<T> {\n    // Explicit replacement uses the declared primitive; mem::take is not provided.\n    #[allow(clippy::mem_replace_with_default)]\n    fn retain<F: FnMut(&T) -> bool>(&mut self, mut predicate: F) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            if predicate(&value) {\n                self.push(value);\n            }\n        }\n    }\n}\n\n/// Stable merge sorting requires only checked indexing and owned Option slots.\n/// Each pass is linear; the number of passes grows logarithmically with length.\npub trait VecSort<T: Ord> {\n    fn sort(&mut self);\n}\nimpl<T: Ord> VecSort<T> for Vec<T> {\n    #[allow(clippy::mem_replace_with_default)]\n    fn sort(&mut self) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        let mut values: Vec<Option<T>> = Vec::new();\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            values.push(Option::Some(value));\n        }\n        let count = values.len();\n        let mut width = 1usize;\n        while width < count {\n            let mut merged: Vec<Option<T>> = Vec::new();\n            let mut start = 0usize;\n            while start < count {\n                let left_count = if width < count - start {\n                    width\n                } else {\n                    count - start\n                };\n                let middle = start + left_count;\n                let right_count = if width < count - middle {\n                    width\n                } else {\n                    count - middle\n                };\n                let end = middle + right_count;\n                let mut left = start;\n                let mut right = middle;\n                while left < middle || right < end {\n                    let use_left = if left == middle {\n                        false\n                    } else if right == end {\n                        true\n                    } else {\n                        *values[left].as_ref().unwrap() <= *values[right].as_ref().unwrap()\n                    };\n                    if use_left {\n                        merged.push(values[left].take());\n                        left = left + 1;\n                    } else {\n                        merged.push(values[right].take());\n                        right = right + 1;\n                    }\n                }\n                start = end;\n            }\n            values = merged;\n            width = if width > count / 2 { count } else { width * 2 };\n        }\n        let mut index = 0usize;\n        while index < count {\n            self.push(values[index].take().unwrap());\n            index = index + 1;\n        }\n    }\n}\n\n/// Slice inspection has the same borrowing contract as vector inspection.\nimpl<T> VecAccess<T> for [T] {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\nimpl<T: PartialEq> VecContains<T> for [T] {\n    fn contains(&self, value: &T) -> bool {\n        let mut index = 0usize;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index += 1;\n        }\n        false\n    }\n}\n\npub trait VecAccessMut<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T>;\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T>;\n}\nimpl<T> VecAccessMut<T> for Vec<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\nimpl<T> VecAccessMut<T> for [T] {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\n\npub trait SliceClone<T: Clone> {\n    fn to_vec(&self) -> Vec<T>;\n}\nimpl<T: Clone> SliceClone<T> for [T] {\n    fn to_vec(&self) -> Vec<T> {\n        let mut result: Vec<T> = Vec::new();\n        let mut index = 0usize;\n        while index < self.len() {\n            result.push(self[index].clone());\n            index += 1;\n        }\n        result\n    }\n}\n\npub trait SequencePrefix<T: PartialEq> {\n    fn starts_with(&self, prefix: &[T]) -> bool;\n}\nimpl<T: PartialEq> SequencePrefix<T> for [T] {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\nimpl<T: PartialEq> SequencePrefix<T> for Vec<T> {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\n\npub trait SliceCopy<T: Copy> {\n    fn copy_from_slice(&mut self, source: &[T]);\n}\nimpl<T: Copy> SliceCopy<T> for [T] {\n    fn copy_from_slice(&mut self, source: &[T]) {\n        assert!(self.len() == source.len());\n        let mut index = 0usize;\n        while index < self.len() {\n            self[index] = source[index];\n            index += 1;\n        }\n    }\n}\n\npub trait VecClear<T> {\n    fn clear(&mut self);\n    fn truncate(&mut self, length: usize);\n}\nimpl<T> VecClear<T> for Vec<T> {\n    fn clear(&mut self) {\n        self.retain(|_| false);\n    }\n    fn truncate(&mut self, length: usize) {\n        if length >= self.len() {\n            return;\n        }\n        let mut index = 0usize;\n        self.retain(|_| {\n            let keep = index < length;\n            index += 1;\n            keep\n        });\n    }\n}\n\n/// A four-way lexicographic result: less, equal, greater, or unordered.\n/// Relational operators are used directly so custom equality does not affect order.\n#[allow(clippy::neg_cmp_op_on_partial_ord)]\npub fn __lkjscript_slice_order<T: PartialOrd>(left: &[T], right: &[T]) -> i32 {\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1i32;\n        }\n        if left[index] > right[index] {\n            return 1i32;\n        }\n        if !(left[index] <= right[index]) {\n            return 2i32;\n        }\n        index += 1;\n    }\n    if left.len() < right.len() {\n        -1i32\n    } else if left.len() > right.len() {\n        1i32\n    } else {\n        0i32\n    }\n}\n\npub fn __lkjscript_vec_order<T: PartialOrd>(left: &Vec<T>, right: &Vec<T>) -> i32 {\n    __lkjscript_slice_order(&left[..], &right[..])\n}\n\npub fn __lkjscript_slice_equal<T: PartialEq>(left: &[T], right: &[T]) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index = 0usize;\n    while index < left.len() {\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index += 1;\n    }\n    true\n}\n\n/// Split borrows the original elements and evaluates the predicate once per element.\npub struct SliceSplit<'a, T, F> {\n    items: &'a [T],\n    predicate: F,\n    start: usize,\n    done: bool,\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::Iterator for SliceSplit<'a, T, F> {\n    type Item = &'a [T];\n    fn next(&mut self) -> Option<&'a [T]> {\n        if self.done {\n            return Option::None;\n        }\n        let items = self.items;\n        let start = self.start;\n        let mut end = start;\n        while end < items.len() {\n            if (self.predicate)(&items[end]) {\n                self.start = end + 1;\n                return Option::Some(&items[start..end]);\n            }\n            end += 1;\n        }\n        self.done = true;\n        Option::Some(&items[start..])\n    }\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::IntoIterator for SliceSplit<'a, T, F> {\n    type IntoIter = Self;\n    fn into_iter(self) -> Self {\n        self\n    }\n}\npub trait SliceSplitSource<T> {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F>;\n}\nimpl<T> SliceSplitSource<T> for [T] {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F> {\n        SliceSplit {\n            items: self,\n            predicate,\n            start: 0,\n            done: false,\n        }\n    }\n}\n")),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("\npub fn clone_probe()->i64 {\n    let values:Vec<i64>=vec![4,2];let copied=values.clone();\n    copied[0]*10+copied[1]\n}\npub fn eq_probe()->i64 {\n    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];\n    if left==right {1} else {0}\n}\npub fn ne_probe()->i64 {\n    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];\n    if left.ne(&right) {1} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 361usize % workers == shard {
        record(source_case("production_sequence_mutations_change_vector_artifacts_and_behavior_0006", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", Some("//! Vec operations are ordinary source over checked element access.\nuse crate::prelude::Option;\npub fn __lkjscript_vec_clone<T: Clone>(source: &Vec<T>) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    let mut index: usize = 0;\n    while index < source.len() {\n        result.push(source[index].clone());\n        index = index + 1;\n    }\n    result\n}\n\npub fn __lkjscript_vec_equal<T: PartialEq>(left: &Vec<T>, right: &Vec<T>) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index: usize = 0;\n    while index < left.len() {\n        // Equality uses the element's eq contract; ne may be independently overridden.\n        if false {\n            return false;\n        }\n        index = index + 1;\n    }\n    true\n}\n\n// Clone the first count-1 elements and move the original into the last slot.\n// The element expression is evaluated even when count is zero.\npub fn __lkjscript_vec_repeat<T: Clone>(value: T, count: usize) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    if count == 0 {\n        return result;\n    }\n    let mut remaining = count - 1;\n    while remaining > 0 {\n        result.push(value.clone());\n        remaining = remaining - 1;\n    }\n    result.push(value);\n    result\n}\n\n/// Shared inspection does not require an element equality or copying contract.\npub trait VecAccess<T> {\n    fn is_empty(&self) -> bool;\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T>;\n    fn first<'a>(&'a self) -> Option<&'a T>;\n    fn last<'a>(&'a self) -> Option<&'a T>;\n}\n\nimpl<T> VecAccess<T> for Vec<T> {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\n\n/// Equality search borrows its elements and stops at the first match.\npub trait VecContains<T: PartialEq> {\n    fn contains(&self, value: &T) -> bool;\n}\n\nimpl<T: PartialEq> VecContains<T> for Vec<T> {\n    fn contains(&self, value: &T) -> bool {\n        let mut index: usize = 0;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index = index + 1;\n        }\n        false\n    }\n}\n\n/// Append independent clones in source order; the shared source remains usable.\npub trait VecExtend<T: Clone> {\n    fn extend_from_slice(&mut self, values: &[T]);\n}\n\nimpl<T: Clone> VecExtend<T> for Vec<T> {\n    fn extend_from_slice(&mut self, values: &[T]) {\n        let mut index: usize = 0;\n        while index < values.len() {\n            self.push(values[index].clone());\n            index = index + 1;\n        }\n    }\n}\n\n/// Mutation visits values in their original order and keeps affine ownership.\npub trait VecMutate<T> {\n    fn retain<F: FnMut(&T) -> bool>(&mut self, predicate: F);\n}\nimpl<T> VecMutate<T> for Vec<T> {\n    // Explicit replacement uses the declared primitive; mem::take is not provided.\n    #[allow(clippy::mem_replace_with_default)]\n    fn retain<F: FnMut(&T) -> bool>(&mut self, mut predicate: F) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            if predicate(&value) {\n                self.push(value);\n            }\n        }\n    }\n}\n\n/// Stable merge sorting requires only checked indexing and owned Option slots.\n/// Each pass is linear; the number of passes grows logarithmically with length.\npub trait VecSort<T: Ord> {\n    fn sort(&mut self);\n}\nimpl<T: Ord> VecSort<T> for Vec<T> {\n    #[allow(clippy::mem_replace_with_default)]\n    fn sort(&mut self) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        let mut values: Vec<Option<T>> = Vec::new();\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            values.push(Option::Some(value));\n        }\n        let count = values.len();\n        let mut width = 1usize;\n        while width < count {\n            let mut merged: Vec<Option<T>> = Vec::new();\n            let mut start = 0usize;\n            while start < count {\n                let left_count = if width < count - start {\n                    width\n                } else {\n                    count - start\n                };\n                let middle = start + left_count;\n                let right_count = if width < count - middle {\n                    width\n                } else {\n                    count - middle\n                };\n                let end = middle + right_count;\n                let mut left = start;\n                let mut right = middle;\n                while left < middle || right < end {\n                    let use_left = if left == middle {\n                        false\n                    } else if right == end {\n                        true\n                    } else {\n                        *values[left].as_ref().unwrap() <= *values[right].as_ref().unwrap()\n                    };\n                    if use_left {\n                        merged.push(values[left].take());\n                        left = left + 1;\n                    } else {\n                        merged.push(values[right].take());\n                        right = right + 1;\n                    }\n                }\n                start = end;\n            }\n            values = merged;\n            width = if width > count / 2 { count } else { width * 2 };\n        }\n        let mut index = 0usize;\n        while index < count {\n            self.push(values[index].take().unwrap());\n            index = index + 1;\n        }\n    }\n}\n\n/// Slice inspection has the same borrowing contract as vector inspection.\nimpl<T> VecAccess<T> for [T] {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\nimpl<T: PartialEq> VecContains<T> for [T] {\n    fn contains(&self, value: &T) -> bool {\n        let mut index = 0usize;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index += 1;\n        }\n        false\n    }\n}\n\npub trait VecAccessMut<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T>;\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T>;\n}\nimpl<T> VecAccessMut<T> for Vec<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\nimpl<T> VecAccessMut<T> for [T] {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\n\npub trait SliceClone<T: Clone> {\n    fn to_vec(&self) -> Vec<T>;\n}\nimpl<T: Clone> SliceClone<T> for [T] {\n    fn to_vec(&self) -> Vec<T> {\n        let mut result: Vec<T> = Vec::new();\n        let mut index = 0usize;\n        while index < self.len() {\n            result.push(self[index].clone());\n            index += 1;\n        }\n        result\n    }\n}\n\npub trait SequencePrefix<T: PartialEq> {\n    fn starts_with(&self, prefix: &[T]) -> bool;\n}\nimpl<T: PartialEq> SequencePrefix<T> for [T] {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\nimpl<T: PartialEq> SequencePrefix<T> for Vec<T> {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\n\npub trait SliceCopy<T: Copy> {\n    fn copy_from_slice(&mut self, source: &[T]);\n}\nimpl<T: Copy> SliceCopy<T> for [T] {\n    fn copy_from_slice(&mut self, source: &[T]) {\n        assert!(self.len() == source.len());\n        let mut index = 0usize;\n        while index < self.len() {\n            self[index] = source[index];\n            index += 1;\n        }\n    }\n}\n\npub trait VecClear<T> {\n    fn clear(&mut self);\n    fn truncate(&mut self, length: usize);\n}\nimpl<T> VecClear<T> for Vec<T> {\n    fn clear(&mut self) {\n        self.retain(|_| false);\n    }\n    fn truncate(&mut self, length: usize) {\n        if length >= self.len() {\n            return;\n        }\n        let mut index = 0usize;\n        self.retain(|_| {\n            let keep = index < length;\n            index += 1;\n            keep\n        });\n    }\n}\n\n/// A four-way lexicographic result: less, equal, greater, or unordered.\n/// Relational operators are used directly so custom equality does not affect order.\n#[allow(clippy::neg_cmp_op_on_partial_ord)]\npub fn __lkjscript_slice_order<T: PartialOrd>(left: &[T], right: &[T]) -> i32 {\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1i32;\n        }\n        if left[index] > right[index] {\n            return 1i32;\n        }\n        if !(left[index] <= right[index]) {\n            return 2i32;\n        }\n        index += 1;\n    }\n    if left.len() < right.len() {\n        -1i32\n    } else if left.len() > right.len() {\n        1i32\n    } else {\n        0i32\n    }\n}\n\npub fn __lkjscript_vec_order<T: PartialOrd>(left: &Vec<T>, right: &Vec<T>) -> i32 {\n    __lkjscript_slice_order(&left[..], &right[..])\n}\n\npub fn __lkjscript_slice_equal<T: PartialEq>(left: &[T], right: &[T]) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index = 0usize;\n    while index < left.len() {\n        if false {\n            return false;\n        }\n        index += 1;\n    }\n    true\n}\n\n/// Split borrows the original elements and evaluates the predicate once per element.\npub struct SliceSplit<'a, T, F> {\n    items: &'a [T],\n    predicate: F,\n    start: usize,\n    done: bool,\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::Iterator for SliceSplit<'a, T, F> {\n    type Item = &'a [T];\n    fn next(&mut self) -> Option<&'a [T]> {\n        if self.done {\n            return Option::None;\n        }\n        let items = self.items;\n        let start = self.start;\n        let mut end = start;\n        while end < items.len() {\n            if (self.predicate)(&items[end]) {\n                self.start = end + 1;\n                return Option::Some(&items[start..end]);\n            }\n            end += 1;\n        }\n        self.done = true;\n        Option::Some(&items[start..])\n    }\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::IntoIterator for SliceSplit<'a, T, F> {\n    type IntoIter = Self;\n    fn into_iter(self) -> Self {\n        self\n    }\n}\npub trait SliceSplitSource<T> {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F>;\n}\nimpl<T> SliceSplitSource<T> for [T] {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F> {\n        SliceSplit {\n            items: self,\n            predicate,\n            start: 0,\n            done: false,\n        }\n    }\n}\n")),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("\npub fn clone_probe()->i64 {\n    let values:Vec<i64>=vec![4,2];let copied=values.clone();\n    copied[0]*10+copied[1]\n}\npub fn eq_probe()->i64 {\n    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];\n    if left==right {1} else {0}\n}\npub fn ne_probe()->i64 {\n    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];\n    if left.ne(&right) {1} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 362usize % workers == shard {
        record(source_case("projected_assignment_matches_rust_ownership_and_evaluation_order_0000", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("\nstruct Token { phase:i64 }\nimpl Drop for Token {fn drop(&mut self){assert!(self.phase==2);self.phase=3;}}\nstruct Owner {value:Box<i64>,token:Box<Token>}\nimpl Drop for Owner {fn drop(&mut self){assert!(self.token.phase==1);self.token.phase=2;}}\nfn owner(value:i64)->Owner{Owner{value:Box::new(value),token:Box::new(Token{phase:1})}}\nfn identity_target<'a,T>(target:&'a mut T)->&'a mut T{target}\nfn replace_generic<T>(target:&mut T,incoming:T){*identity_target(target)=incoming;}\nfn replace_owner(target:&mut Owner){*target=owner(42);}\nfn replace_box(target:&mut Box<Owner>){*target=Box::new(owner(42));}\nfn replace_box_payload(target:&mut Box<Owner>){**target=owner(42);}\npub fn assignments_through_exclusive_owned_targets()->i64{\n    let mut first=owner(1);replace_owner(&mut first);replace_generic(&mut first,owner(42));\n    let mut second=Box::new(owner(2));replace_box(&mut second);\n    let mut third=Box::new(owner(3));replace_box_payload(&mut third);\n    if *first.value!=42 || *second.value!=42 || *third.value!=42{return 1;}\n    *first.value=40;*second.value=1;*third.value=1;\n    *first.value+*second.value+*third.value\n}\npub fn indexed_vector_and_array_elements_keep_their_containers()->i64{\n    let mut values:Vec<Owner>=Vec::new();values.push(owner(1));values.push(owner(2));\n    values[0]=owner(40);values[1]=owner(2);\n    let mut array=[Box::new(owner(1)),Box::new(owner(2))];\n    array[0]=Box::new(owner(40));*array[1]=owner(2);\n    if values.len()!=2 || *values[0].value+*values[1].value!=42{return 1;}\n    if *array[0].value+*array[1].value!=42{return 2;}\n    let mut boxed=Box::new([owner(1),owner(2)]);\n    (*boxed)[0]=owner(40);(*boxed)[1]=owner(2);\n    *(*boxed)[0].value+*(*boxed)[1].value\n}\nstruct Row {values:Vec<Owner>}\nstruct Container {row:Box<Row>}\npub fn nested_vector_box_and_struct_places()->i64{\n    let mut row:Vec<Owner>=Vec::new();row.push(owner(1));row.push(owner(2));\n    let mut rows:Vec<Row>=Vec::new();rows.push(Row{values:row});\n    rows[0].values[0]=owner(40);rows[0].values[1]=owner(2);\n    let mut other:Vec<Owner>=Vec::new();other.push(owner(1));other.push(owner(2));\n    let mut container=Container{row:Box::new(Row{values:other})};\n    container.row.values[0]=owner(20);container.row.values[1]=owner(22);\n    if *rows[0].values[0].value+*rows[0].values[1].value!=42{return 1;}\n    *container.row.values[0].value+*container.row.values[1].value\n}\nfn replace_slice(values:&mut [Owner]){values[0]=owner(40);values[1]=owner(2);}\npub fn slices_replace_elements_without_moving_the_slice_owner()->i64{\n    let mut values=[owner(1),owner(2)];\n    replace_slice(&mut values);\n    *values[0].value+*values[1].value\n}\nstruct Counted<'a> {drops:&'a mut i64,payload:Box<i64>}\nimpl<'a> Drop for Counted<'a>{fn drop(&mut self){*self.drops=*self.drops+1;}}\npub fn replacement_drops_old_and_new_values_once()->i64{\n    let mut old_drops:i64=0;let mut new_drops:i64=0;\n    {\n        let mut value=Counted{drops:&mut old_drops,payload:Box::new(1)};\n        *(&mut value)=Counted{drops:&mut new_drops,payload:Box::new(42)};\n        if *value.payload!=42{return 1;}\n    }\n    old_drops*10+new_drops\n}\nstruct PlainView<'a>{value:&'a i64}\nfn assigned_view<'a>(external:&'a i64)->PlainView<'a>{\n    let mut view=PlainView{value:external};\n    *(&mut view)=PlainView{value:external};\n    view\n}\npub fn replacement_preserves_the_installed_shared_view_origin()->i64{\n    let first:i64=7;let second:i64=42;\n    let mut view=PlainView{value:&first};\n    *(&mut view)=PlainView{value:&second};\n    if *view.value!=42{return 1;}\n    *assigned_view(&second).value\n}\nfn scalar_target<'a>(trace:&mut i64,target:&'a mut i64)->&'a mut i64{*trace=*trace*10+2;target}\nfn scalar_rhs(trace:&mut i64)->i64{*trace=*trace*10+1;42}\npub fn simple_assignment_evaluates_rhs_then_place_once()->i64{\n    let mut trace:i64=0;let mut value:i64=0;\n    *scalar_target(&mut trace,&mut value)=scalar_rhs(&mut trace);\n    trace*100+value\n}\npub fn primitive_compound_assignment_evaluates_rhs_before_reading_place()->i64{\n    let mut trace:i64=0;let mut value:i64=0;\n    *scalar_target(&mut trace,&mut value)+=scalar_rhs(&mut trace);\n    value+={value=2;40};\n    trace*100+value\n}\nfn owner_target<'a>(trace:&mut i64,target:&'a mut Owner)->&'a mut Owner{*trace=*trace*10+2;target}\nfn owner_rhs(trace:&mut i64)->Owner{*trace=*trace*10+1;owner(42)}\npub fn affine_assignment_evaluates_rhs_then_borrowed_place_once()->i64{\n    let mut trace:i64=0;let mut value=owner(1);\n    *owner_target(&mut trace,&mut value)=owner_rhs(&mut trace);\n    trace*100+*value.value\n}\nfn index(trace:&mut i64)->usize{*trace=*trace*10+2;0}\npub fn indexed_assignment_evaluates_rhs_before_the_index()->i64{\n    let mut trace:i64=0;let mut values:Vec<Owner>=Vec::new();values.push(owner(1));\n    values[index(&mut trace)]=owner_rhs(&mut trace);\n    trace*100+*values[0].value\n}\npub fn rhs_mutation_selects_the_new_index_and_preserves_container_growth()->i64{\n    let mut values:Vec<Owner>=Vec::new();values.push(owner(1));\n    let mut selected:usize=0;\n    values[selected]={selected=1;values.push(owner(2));owner(42)};\n    if values.len()!=2 || *values[0].value!=1{return 1;}\n    *values[1].value\n}\nfn small_target<'a>(target:&'a mut u8)->&'a mut u8{target}\nstruct Factory{}\nimpl Factory{fn len<'a>(&self,target:&'a mut u8)->&'a mut u8{target}}\npub fn target_type_controls_unsuffixed_rhs_without_name_shortcuts()->i64{\n    let mut value:u8=1;\n    *small_target(&mut value)=40;\n    *Factory{}.len(&mut value)+=2;\n    value as i64\n}\n")),
], ""), errors);
        count += 1;
    }
    if 363usize % workers == shard {
        record(source_case("projected_shared_results_do_not_thaw_an_exclusive_ancestor_0000", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn inner<'a>(r:&mut &'a i64)->&'a i64{*r} fn bad()->i64{let mut x:i64=7;let y:i64=9;let exclusive=&mut x;let mut r=&*exclusive;let out=inner(&mut r);r=&y;*exclusive=9;*out+*r}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 364usize % workers == shard {
        record(source_case("projected_shared_results_do_not_thaw_an_exclusive_ancestor_0001", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn returned<'a>(r:&&'a &mut i64)->&'a i64{&***r} fn bad()->i64{let mut value:i64=7;let exclusive=&mut value;let inner=&exclusive;let carrier=&inner;let out=returned(carrier);*exclusive=9;*out}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 365usize % workers == shard {
        record(source_case("qualified_nominal_types_keep_their_declaration_inside_generic_scopes_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("struct T(i64); fn combine<T>(named:crate::T,value:T)->(crate::T,T){(named,value)} pub fn probe()->i64{let (named,value)=combine::<i64>(T(4),6i64);named.0+value}")),
], ""), errors);
        count += 1;
    }
    if 366usize % workers == shard {
        record(source_case("range_endpoint_suffixes_do_not_silently_change_the_matched_type_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{0u16..=1u16=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 367usize % workers == shard {
        record(source_case("range_endpoint_suffixes_do_not_silently_change_the_matched_type_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i8)->i64{match value{-1i16..=1i16=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 368usize % workers == shard {
        record(source_case("range_endpoint_suffixes_do_not_silently_change_the_matched_type_0002", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u64)->i64{match value{b'A'..=b'Z'=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 369usize % workers == shard {
        record(source_case("range_endpoints_obey_the_scrutinees_integer_representation_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{0..=256=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 370usize % workers == shard {
        record(source_case("range_endpoints_obey_the_scrutinees_integer_representation_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i8)->i64{match value{-129..=0=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 371usize % workers == shard {
        record(source_case("range_endpoints_obey_the_scrutinees_integer_representation_0002", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i8)->i64{match value{0..128=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 372usize % workers == shard {
        record(source_case("range_endpoints_obey_the_scrutinees_integer_representation_0003", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u64)->i64{match value{0..=18446744073709551616u64=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 373usize % workers == shard {
        record(source_case("range_endpoints_obey_the_scrutinees_integer_representation_0004", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:i64)->i64{match value{-9223372036854775809i64..0=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 374usize % workers == shard {
        record(source_case("range_endpoints_obey_the_scrutinees_integer_representation_0005", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{-1..0=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 375usize % workers == shard {
        record(source_case("raw_names_require_an_explicit_checked_runtime_seal_0000", "runtime.rs", None, vec![
source_file("runtime.rs", Some("//! Source allocator over sealed raw addresses. Ordinary application stubs trap.\n//! Allocator metadata is private to this runtime; programs receive payload pointers.\n\npub fn raw_load8(_address: u64) -> u64 { 0 }\npub fn raw_load64(_address: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store8(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store64(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_memory_copy(_destination: u64, _source: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_map(_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_unmap(_address: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_remap(_address: u64, _old_bytes: u64, _new_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_allocator_state() -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_fail() -> u64 { panic!(\"unsealed runtime primitive\") }\n\nfn add_extent(left: u64, right: u64) -> u64 {\n    if left > 18446744073709551615 - right {\n        return raw_fail();\n    }\n    left + right\n}\n\nfn payload_class(bytes: u64) -> u64 {\n    let mut size: u64 = 16;\n    let mut class: u64 = 0;\n    while size < bytes {\n        if size > 9223372036854775807 {\n            return raw_fail();\n        }\n        size = size * 2;\n        class = class + 1;\n    }\n    class\n}\n\n// Free-list heads occupy the first 64 words of allocator state. The next three\n// words are the current region's cursor, remaining bytes and next region size.\n// Regions grow geometrically; a refused request is retried down to the pages\n// needed by this allocation. Retained regions supply slabs and medium blocks.\nfn take_region(bytes: u64) -> u64 {\n    let state = raw_allocator_state();\n    let mut cursor = raw_load64(state + 512);\n    let mut remaining = raw_load64(state + 520);\n    if remaining < bytes {\n        let minimum = add_extent(bytes, 4095) & !4095u64;\n        let mut size = raw_load64(state + 528);\n        if size == 0 {\n            size = 65536;\n        }\n        if size < minimum {\n            size = minimum;\n        }\n        cursor = raw_map(size);\n        while cursor == 0 && size > minimum {\n            size = (size / 2) & !4095u64;\n            if size < minimum {\n                size = minimum;\n            }\n            cursor = raw_map(size);\n        }\n        if cursor == 0 {\n            return raw_fail();\n        }\n        remaining = size;\n        let mut next = size;\n        if next <= 9223372036854775807 {\n            next = next * 2;\n        }\n        raw_store64(state + 528, next);\n    }\n    raw_store64(state + 512, cursor + bytes);\n    raw_store64(state + 520, remaining - bytes);\n    cursor\n}\n\n// Each block has {class, next_free} followed by a power-of-two payload.\n// Payloads through 1 MiB are reused by class. Small blocks are carved from\n// 4096-byte slabs inside regions; medium blocks are carved directly. Larger\n// blocks have independent mappings so growth can remap without copying.\npub fn allocate(bytes: u64) -> u64 {\n    let class = payload_class(bytes);\n    let size = 16u64 << class;\n    let block_size = add_extent(size, 16);\n    if size > 1048576 {\n        let block = raw_map(block_size);\n        if block == 0 {\n            return raw_fail();\n        }\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let head = raw_allocator_state() + class * 8;\n    let old = raw_load64(head);\n    if old != 0 {\n        raw_store64(head, raw_load64(old + 8));\n        raw_store64(old + 8, 0);\n        return old + 16;\n    }\n    if block_size > 4096 {\n        let block = take_region(block_size);\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let slab = take_region(4096);\n    let count = 4096 / block_size;\n    let mut index: u64 = count;\n    let mut next: u64 = 0;\n    while index != 0 {\n        index = index - 1;\n        let block = slab + index * block_size;\n        raw_store64(block, class);\n        raw_store64(block + 8, next);\n        next = block;\n    }\n    raw_store64(head, raw_load64(slab + 8));\n    raw_store64(slab + 8, 0);\n    slab + 16\n}\n\npub fn release(pointer: u64) -> u64 {\n    if pointer == 0 {\n        return 0;\n    }\n    let block = pointer - 16;\n    let class = raw_load64(block);\n    let size = 16u64 << class;\n    if size > 1048576 {\n        if raw_unmap(block, add_extent(size, 16)) != 0 {\n            return raw_fail();\n        }\n        return 0;\n    }\n    let head = raw_allocator_state() + class * 8;\n    raw_store64(block + 8, raw_load64(head));\n    raw_store64(head, block);\n    0\n}\n\n// No caller retains a pointer into a block whose ownership is transferred here.\n// Shrinking keeps capacity. Dedicated mappings grow with mremap; only pooled\n// blocks copy their payload when moving into a larger size class.\npub fn reallocate(pointer: u64, bytes: u64) -> u64 {\n    if pointer == 0 {\n        return allocate(bytes);\n    }\n    let block = pointer - 16;\n    let old_size = 16u64 << raw_load64(block);\n    if bytes <= old_size {\n        return pointer;\n    }\n    if old_size > 1048576 {\n        let class = payload_class(bytes);\n        let size = 16u64 << class;\n        let replacement = raw_remap(block, add_extent(old_size, 16), add_extent(size, 16));\n        if replacement == 0 {\n            return raw_fail();\n        }\n        raw_store64(replacement, class);\n        return replacement + 16;\n    }\n    let replacement = allocate(bytes);\n    raw_memory_copy(replacement, pointer, old_size);\n    release(pointer);\n    replacement\n}\n\npub fn valid_utf8_bytes(data: u64, length: u64) -> u64 {\n    let mut index: u64 = 0;\n    let mut remaining: u64 = 0;\n    let mut low: u64 = 128;\n    let mut high: u64 = 191;\n    while index < length {\n        let byte = raw_load8(data + index);\n        if remaining != 0 {\n            if byte < low || byte > high {\n                return 0;\n            }\n            remaining = remaining - 1;\n            low = 128;\n            high = 191;\n        } else if byte >= 128 {\n            low = 128;\n            high = 191;\n            if byte >= 194 && byte <= 223 {\n                remaining = 1;\n            } else if byte >= 224 && byte <= 239 {\n                remaining = 2;\n                if byte == 224 {\n                    low = 160;\n                }\n                if byte == 237 {\n                    high = 159;\n                }\n            } else if byte >= 240 && byte <= 244 {\n                remaining = 3;\n                if byte == 240 {\n                    low = 144;\n                }\n                if byte == 244 {\n                    high = 143;\n                }\n            } else {\n                return 0;\n            }\n        }\n        index = index + 1;\n    }\n    if remaining == 0 { 1 } else { 0 }\n}\n\npub fn utf8_validate(data: u64, length: u64) -> u64 {\n    valid_utf8_bytes(data, length)\n}\n\n// Runtime probe: expected u64 result 42 when appended to the sealed runtime source.\npub fn probe_allocator() -> u64 {\n    if payload_class(0) != 0 || payload_class(16) != 0 || payload_class(17) != 1 || payload_class(9223372036854775808) != 59 { return 1; }\n    let a = allocate(8);\n    let b = allocate(32);\n    raw_store64(a, 11);\n    raw_store64(b, 22);\n    release(a);\n    let reused = allocate(8);\n    if reused != a || raw_load64(b) != 22 { return 2; }\n    release(reused);\n    release(b);\n\n    // Multiple simultaneously live slabs must remain disjoint.\n    let slots = allocate(2400);\n    let mut index: u64 = 0;\n    while index < 300 {\n        let block = allocate(16);\n        raw_store64(slots + index * 8, block);\n        raw_store64(block, index);\n        raw_store64(block + 8, index + 1000);\n        index = index + 1;\n    }\n    index = 0;\n    while index < 300 {\n        let block = raw_load64(slots + index * 8);\n        if raw_load64(block) != index || raw_load64(block + 8) != index + 1000 { return 3; }\n        release(block);\n        index = index + 1;\n    }\n    release(slots);\n\n    let mut size: u64 = 1;\n    while size <= 1048576 {\n        let block = allocate(size);\n        raw_store8(block + size - 1, 173);\n        if raw_load8(block + size - 1) != 173 { return 4; }\n        release(block);\n        size = size * 2;\n    }\n\n    let block = allocate(32);\n    index = 0;\n    while index < 16 {\n        raw_store8(block + index, index);\n        index = index + 1;\n    }\n    raw_memory_copy(block + 4, block, 12);\n    if raw_load8(block + 4) != 0 || raw_load8(block + 15) != 11 { return 5; }\n    release(block);\n\n    let left = allocate(32);\n    let guard = allocate(64);\n    raw_store64(left, 6789);\n    raw_store64(left + 24, 9876);\n    raw_store64(guard, 54321);\n    let grown = reallocate(left, 129);\n    if grown == left || raw_load64(grown - 16) != 4 { return 6; }\n    if raw_load64(grown) != 6789 || raw_load64(grown + 24) != 9876 || raw_load64(guard) != 54321 { return 7; }\n    if reallocate(grown, 17) != grown { return 8; }\n    if allocate(32) != left { return 9; }\n    release(left);\n    release(guard);\n    release(grown);\n\n    let large = allocate(2097152);\n    raw_store64(large, 12345);\n    raw_store8(large + 2097151, 201);\n    let larger = reallocate(large, 4194304);\n    if raw_load64(larger) != 12345 || raw_load8(larger + 2097151) != 201 { return 10; }\n    release(larger);\n\n    let empty = reallocate(0, 7);\n    raw_store64(empty, 42);\n    let answer = raw_load64(empty);\n    release(empty);\n    release(0);\n    answer\n}\npub fn probe_overflow() -> u64 { allocate(18446744073709551615) }\n\npub fn probe_medium_regions() -> u64 {\n    let slots = allocate(520);\n    let mut index: u64 = 0;\n    while index < 65 {\n        let block = allocate(32768);\n        raw_store64(slots + index * 8, block);\n        raw_store64(block, index);\n        raw_store64(block + 32760, index + 1000);\n        index = index + 1;\n    }\n    index = 0;\n    while index < 65 {\n        let block = raw_load64(slots + index * 8);\n        if raw_load64(block) != index || raw_load64(block + 32760) != index + 1000 {\n            return 1;\n        }\n        release(block);\n        index = index + 1;\n    }\n    release(slots);\n    42\n}\n")),
], ""), errors);
        count += 1;
    }
    if 376usize % workers == shard {
        record(source_case("raw_names_require_an_explicit_checked_runtime_seal_0001", "runtime.rs", None, vec![
source_file("runtime.rs", Some("//! Source allocator over sealed raw addresses. Ordinary application stubs trap.\n//! Allocator metadata is private to this runtime; programs receive payload pointers.\n\npub fn raw_load8(_address: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_load64(_address: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store8(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_store64(_address: u64, _value: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_memory_copy(_destination: u64, _source: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_map(_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_unmap(_address: u64, _bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_remap(_address: u64, _old_bytes: u64, _new_bytes: u64) -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_allocator_state() -> u64 { panic!(\"unsealed runtime primitive\") }\npub fn raw_fail() -> u64 { panic!(\"unsealed runtime primitive\") }\n\nfn add_extent(left: u64, right: u64) -> u64 {\n    if left > 18446744073709551615 - right {\n        return raw_fail();\n    }\n    left + right\n}\n\nfn payload_class(bytes: u64) -> u64 {\n    let mut size: u64 = 16;\n    let mut class: u64 = 0;\n    while size < bytes {\n        if size > 9223372036854775807 {\n            return raw_fail();\n        }\n        size = size * 2;\n        class = class + 1;\n    }\n    class\n}\n\n// Free-list heads occupy the first 64 words of allocator state. The next three\n// words are the current region's cursor, remaining bytes and next region size.\n// Regions grow geometrically; a refused request is retried down to the pages\n// needed by this allocation. Retained regions supply slabs and medium blocks.\nfn take_region(bytes: u64) -> u64 {\n    let state = raw_allocator_state();\n    let mut cursor = raw_load64(state + 512);\n    let mut remaining = raw_load64(state + 520);\n    if remaining < bytes {\n        let minimum = add_extent(bytes, 4095) & !4095u64;\n        let mut size = raw_load64(state + 528);\n        if size == 0 {\n            size = 65536;\n        }\n        if size < minimum {\n            size = minimum;\n        }\n        cursor = raw_map(size);\n        while cursor == 0 && size > minimum {\n            size = (size / 2) & !4095u64;\n            if size < minimum {\n                size = minimum;\n            }\n            cursor = raw_map(size);\n        }\n        if cursor == 0 {\n            return raw_fail();\n        }\n        remaining = size;\n        let mut next = size;\n        if next <= 9223372036854775807 {\n            next = next * 2;\n        }\n        raw_store64(state + 528, next);\n    }\n    raw_store64(state + 512, cursor + bytes);\n    raw_store64(state + 520, remaining - bytes);\n    cursor\n}\n\n// Each block has {class, next_free} followed by a power-of-two payload.\n// Payloads through 1 MiB are reused by class. Small blocks are carved from\n// 4096-byte slabs inside regions; medium blocks are carved directly. Larger\n// blocks have independent mappings so growth can remap without copying.\npub fn allocate(bytes: u64) -> u64 {\n    let class = payload_class(bytes);\n    let size = 16u64 << class;\n    let block_size = add_extent(size, 16);\n    if size > 1048576 {\n        let block = raw_map(block_size);\n        if block == 0 {\n            return raw_fail();\n        }\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let head = raw_allocator_state() + class * 8;\n    let old = raw_load64(head);\n    if old != 0 {\n        raw_store64(head, raw_load64(old + 8));\n        raw_store64(old + 8, 0);\n        return old + 16;\n    }\n    if block_size > 4096 {\n        let block = take_region(block_size);\n        raw_store64(block, class);\n        raw_store64(block + 8, 0);\n        return block + 16;\n    }\n    let slab = take_region(4096);\n    let count = 4096 / block_size;\n    let mut index: u64 = count;\n    let mut next: u64 = 0;\n    while index != 0 {\n        index = index - 1;\n        let block = slab + index * block_size;\n        raw_store64(block, class);\n        raw_store64(block + 8, next);\n        next = block;\n    }\n    raw_store64(head, raw_load64(slab + 8));\n    raw_store64(slab + 8, 0);\n    slab + 16\n}\n\npub fn release(pointer: u64) -> u64 {\n    if pointer == 0 {\n        return 0;\n    }\n    let block = pointer - 16;\n    let class = raw_load64(block);\n    let size = 16u64 << class;\n    if size > 1048576 {\n        if raw_unmap(block, add_extent(size, 16)) != 0 {\n            return raw_fail();\n        }\n        return 0;\n    }\n    let head = raw_allocator_state() + class * 8;\n    raw_store64(block + 8, raw_load64(head));\n    raw_store64(head, block);\n    0\n}\n\n// No caller retains a pointer into a block whose ownership is transferred here.\n// Shrinking keeps capacity. Dedicated mappings grow with mremap; only pooled\n// blocks copy their payload when moving into a larger size class.\npub fn reallocate(pointer: u64, bytes: u64) -> u64 {\n    if pointer == 0 {\n        return allocate(bytes);\n    }\n    let block = pointer - 16;\n    let old_size = 16u64 << raw_load64(block);\n    if bytes <= old_size {\n        return pointer;\n    }\n    if old_size > 1048576 {\n        let class = payload_class(bytes);\n        let size = 16u64 << class;\n        let replacement = raw_remap(block, add_extent(old_size, 16), add_extent(size, 16));\n        if replacement == 0 {\n            return raw_fail();\n        }\n        raw_store64(replacement, class);\n        return replacement + 16;\n    }\n    let replacement = allocate(bytes);\n    raw_memory_copy(replacement, pointer, old_size);\n    release(pointer);\n    replacement\n}\n\npub fn valid_utf8_bytes(data: u64, length: u64) -> u64 {\n    let mut index: u64 = 0;\n    let mut remaining: u64 = 0;\n    let mut low: u64 = 128;\n    let mut high: u64 = 191;\n    while index < length {\n        let byte = raw_load8(data + index);\n        if remaining != 0 {\n            if byte < low || byte > high {\n                return 0;\n            }\n            remaining = remaining - 1;\n            low = 128;\n            high = 191;\n        } else if byte >= 128 {\n            low = 128;\n            high = 191;\n            if byte >= 194 && byte <= 223 {\n                remaining = 1;\n            } else if byte >= 224 && byte <= 239 {\n                remaining = 2;\n                if byte == 224 {\n                    low = 160;\n                }\n                if byte == 237 {\n                    high = 159;\n                }\n            } else if byte >= 240 && byte <= 244 {\n                remaining = 3;\n                if byte == 240 {\n                    low = 144;\n                }\n                if byte == 244 {\n                    high = 143;\n                }\n            } else {\n                return 0;\n            }\n        }\n        index = index + 1;\n    }\n    if remaining == 0 { 1 } else { 0 }\n}\n\npub fn utf8_validate(data: u64) -> u64 { valid_utf8_bytes(data, 0) }\n\n// Runtime probe: expected u64 result 42 when appended to the sealed runtime source.\npub fn probe_allocator() -> u64 {\n    if payload_class(0) != 0 || payload_class(16) != 0 || payload_class(17) != 1 || payload_class(9223372036854775808) != 59 { return 1; }\n    let a = allocate(8);\n    let b = allocate(32);\n    raw_store64(a, 11);\n    raw_store64(b, 22);\n    release(a);\n    let reused = allocate(8);\n    if reused != a || raw_load64(b) != 22 { return 2; }\n    release(reused);\n    release(b);\n\n    // Multiple simultaneously live slabs must remain disjoint.\n    let slots = allocate(2400);\n    let mut index: u64 = 0;\n    while index < 300 {\n        let block = allocate(16);\n        raw_store64(slots + index * 8, block);\n        raw_store64(block, index);\n        raw_store64(block + 8, index + 1000);\n        index = index + 1;\n    }\n    index = 0;\n    while index < 300 {\n        let block = raw_load64(slots + index * 8);\n        if raw_load64(block) != index || raw_load64(block + 8) != index + 1000 { return 3; }\n        release(block);\n        index = index + 1;\n    }\n    release(slots);\n\n    let mut size: u64 = 1;\n    while size <= 1048576 {\n        let block = allocate(size);\n        raw_store8(block + size - 1, 173);\n        if raw_load8(block + size - 1) != 173 { return 4; }\n        release(block);\n        size = size * 2;\n    }\n\n    let block = allocate(32);\n    index = 0;\n    while index < 16 {\n        raw_store8(block + index, index);\n        index = index + 1;\n    }\n    raw_memory_copy(block + 4, block, 12);\n    if raw_load8(block + 4) != 0 || raw_load8(block + 15) != 11 { return 5; }\n    release(block);\n\n    let left = allocate(32);\n    let guard = allocate(64);\n    raw_store64(left, 6789);\n    raw_store64(left + 24, 9876);\n    raw_store64(guard, 54321);\n    let grown = reallocate(left, 129);\n    if grown == left || raw_load64(grown - 16) != 4 { return 6; }\n    if raw_load64(grown) != 6789 || raw_load64(grown + 24) != 9876 || raw_load64(guard) != 54321 { return 7; }\n    if reallocate(grown, 17) != grown { return 8; }\n    if allocate(32) != left { return 9; }\n    release(left);\n    release(guard);\n    release(grown);\n\n    let large = allocate(2097152);\n    raw_store64(large, 12345);\n    raw_store8(large + 2097151, 201);\n    let larger = reallocate(large, 4194304);\n    if raw_load64(larger) != 12345 || raw_load8(larger + 2097151) != 201 { return 10; }\n    release(larger);\n\n    let empty = reallocate(0, 7);\n    raw_store64(empty, 42);\n    let answer = raw_load64(empty);\n    release(empty);\n    release(0);\n    answer\n}\npub fn probe_overflow() -> u64 { allocate(18446744073709551615) }\n\npub fn probe_medium_regions() -> u64 {\n    let slots = allocate(520);\n    let mut index: u64 = 0;\n    while index < 65 {\n        let block = allocate(32768);\n        raw_store64(slots + index * 8, block);\n        raw_store64(block, index);\n        raw_store64(block + 32760, index + 1000);\n        index = index + 1;\n    }\n    index = 0;\n    while index < 65 {\n        let block = raw_load64(slots + index * 8);\n        if raw_load64(block) != index || raw_load64(block + 32760) != index + 1000 {\n            return 1;\n        }\n        release(block);\n        index = index + 1;\n    }\n    release(slots);\n    42\n}\n")),
], ""), errors);
        count += 1;
    }
    if 377usize % workers == shard {
        record(source_case_at("reborrowing_a_call_result_does_not_narrow_its_owner_footprint_0000", "source_bounds.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("source_bounds.rs", Some("struct Cell{value:i64}\nstruct Pair{left:Cell,right:Cell}\nfn choose<'a>(owner:&'a Pair)->&'a Cell{&owner.right}\nfn probe()->i64{\n    let mut owner=Pair{left:Cell{value:1},right:Cell{value:2}};\n    let selected=choose(&owner);\n    let child=&selected.value;\n    drop(selected);\n    owner.right.value=9;\n    *child\n}\n")),
], "E_BORROW", "source_bounds.rs", 9, 0), errors);
        count += 1;
    }
    if 378usize % workers == shard {
        record(source_case("recovery_reaches_independent_statements_items_and_nested_methods_0000", "case/main.rs", None, vec![
source_file("case/main.rs", Some("\n        fn broken() { let = 1; mystery!(); let Some(x) = unknown else {}; }\n        fn good() -> i64 { 42 }\n        mod nested { fn broken() { other!(); } fn good() -> i64 { 7 } }\n        struct Item;\n        impl Item {\n            const BAD: i64 = 0;\n            fn broken() { third!(); }\n            fn good() -> i64 { 9 }\n        }\n    ")),
], "E_PARSE"), errors);
        count += 1;
    }
    if 379usize % workers == shard {
        record(source_case("recursive_associated_bounds_are_introduced_lazily_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Chain{type Next:Chain;fn next(self)->Self::Next;} fn next<T:Chain>(x:T)->T::Next{x.next()}"))], ""), errors);
        count += 1;
    }
    if 380usize % workers == shard {
        record(source_case("recursive_calls_and_stack_arguments_preserve_live_registers_0000", "call-preservation", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("call-preservation", Some("\n        fn mix(a:i64,b:i64,c:i64,d:i64,e:i64,f:i64,g:i64,h:i64)->i64 {\n            let x = a + b; let y = c * d; let z = e ^ f; x + y + z + g + h\n        }\n        fn recurse(n:i64)->i64 {\n            if n == 0 { return 3; }\n            let kept = n * 17;\n            kept + recurse(n - 1) + kept\n        }\n        pub fn probe()->i64 {\n            let kept = recurse(18);\n            let other = mix(11,13,17,19,23,29,31,37);\n            kept + other + mix(other,2,3,4,5,6,7,kept)\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 381usize % workers == shard {
        record(source_case("reference_field_assignment_cannot_bypass_external_write_contracts_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        struct View<'a> { value: &'a i64 }\n        fn install<'a>(slot: &mut View<'a>, replacement: &'a i64) {\n            slot.value = replacement;\n        }\n        fn probe() -> i64 {\n            let external: i64 = 42;\n            let mut view = View { value: &external };\n            {\n                let local: i64 = 7;\n                install(&mut view, &local);\n            };\n            *view.value\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 382usize % workers == shard {
        record(source_case("reinitializing_an_absent_enum_leaf_restores_its_loan_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        enum Maybe<'a> { None, Some(&'a i64) }\n        fn inspect(value: &mut Maybe<'_>) {}\n        fn leak<'a>(external: &'a i64) -> Maybe<'a> {\n            let mut value: Maybe<'_> = Maybe::None;\n            inspect(&mut value);\n            let local: i64 = 7;\n            value = Maybe::Some(&local);\n            value\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 383usize % workers == shard {
        record(source_case("removing_source_derives_removes_their_contracts_0000", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", Some("//! The source prelude. These enums own their payloads and work with affine values.\npub use crate::integer::{ByteAscii, CheckedArithmetic, ScalarOrder};\npub use crate::iter::{IntoIterator, IterSource, Iterator, IteratorExt};\npub use crate::sequence::{\n    SequencePrefix, SliceClone, SliceCopy, SliceSplitSource, VecAccess, VecAccessMut, VecClear,\n    VecContains, VecExtend, VecMutate, VecSort,\n};\npub use crate::str::{ParseInteger, TextView};\n#[derive(PartialEq, Eq, PartialOrd, Ord)]\npub enum Option<T> {\n    None,\n    Some(T),\n}\n\nimpl<T> Option<T> {\n    pub fn as_ref(&self) -> Option<&T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn as_mut(&mut self) -> Option<&mut T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn expect(self, _message: &str) -> T {\n        self.unwrap()\n    }\n    pub fn and<U>(self, other: Option<U>) -> Option<U> {\n        match self {\n            Option::Some(_) => other,\n            Option::None => Option::None,\n        }\n    }\n    pub fn or(self, other: Option<T>) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => other,\n        }\n    }\n    pub fn xor(self, other: Option<T>) -> Option<T> {\n        match (self, other) {\n            (Option::Some(value), Option::None) | (Option::None, Option::Some(value)) => {\n                Option::Some(value)\n            }\n            _ => Option::None,\n        }\n    }\n    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {\n        match (self, other) {\n            (Option::Some(left), Option::Some(right)) => Option::Some((left, right)),\n            _ => Option::None,\n        }\n    }\n    pub fn replace(&mut self, value: T) -> Option<T> {\n        std::mem::replace(self, Option::Some(value))\n    }\n    pub fn insert(&mut self, value: T) -> &mut T {\n        *self = Option::Some(value);\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n    pub fn get_or_insert(&mut self, value: T) -> &mut T {\n        if self.is_none() {\n            *self = Option::Some(value);\n        }\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n\n    pub fn is_some(&self) -> bool {\n        match self {\n            Option::None => false,\n            Option::Some(_) => true,\n        }\n    }\n\n    pub fn is_none(&self) -> bool {\n        !self.is_some()\n    }\n\n    pub fn is_some_and<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => false,\n        }\n    }\n\n    pub fn is_none_or<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => true,\n        }\n    }\n\n    pub fn ok_or<E>(self, error: E) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error),\n        }\n    }\n\n    pub fn ok_or_else<E, F: FnOnce() -> E>(self, error: F) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error()),\n        }\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => panic!(\"called Option::unwrap on None\"),\n        }\n    }\n\n    pub fn take(&mut self) -> Option<T> {\n        std::mem::replace(self, Option::None)\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => return default,\n        }\n    }\n}\n\n#[derive(PartialEq, Eq)]\npub enum Result<T, E> {\n    Ok(T),\n    Err(E),\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn as_ref(&self) -> Result<&T, &E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> {\n        match self {\n            Result::Ok(_) => other,\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or<F>(self, other: Result<T, F>) -> Result<T, F> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(_) => other,\n        }\n    }\n\n    pub fn is_ok(&self) -> bool {\n        match self {\n            Result::Ok(_) => true,\n            Result::Err(_) => false,\n        }\n    }\n\n    pub fn is_err(&self) -> bool {\n        !self.is_ok()\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => panic!(\"called Result::unwrap on Err\"),\n        }\n    }\n\n    pub fn unwrap_err(self) -> E {\n        match self {\n            Result::Err(value) => return value,\n            Result::Ok(_) => panic!(\"called Result::unwrap_err on Ok\"),\n        }\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => return default,\n        }\n    }\n\n    pub fn ok(self) -> Option<T> {\n        match self {\n            Result::Ok(value) => return Option::Some(value),\n            Result::Err(_) => return Option::None,\n        }\n    }\n\n    pub fn err(self) -> Option<E> {\n        match self {\n            Result::Err(value) => return Option::Some(value),\n            Result::Ok(_) => return Option::None,\n        }\n    }\n}\n\nimpl<T> Option<Option<T>> {\n    pub fn flatten(self) -> Option<T> {\n        match self {\n            Option::Some(value) => value,\n            Option::None => Option::None,\n        }\n    }\n}\nimpl<T, E> Option<Result<T, E>> {\n    pub fn transpose(self) -> Result<Option<T>, E> {\n        match self {\n            Option::Some(Result::Ok(value)) => Result::Ok(Option::Some(value)),\n            Option::Some(Result::Err(error)) => Result::Err(error),\n            Option::None => Result::Ok(Option::None),\n        }\n    }\n}\nimpl<T, E> Result<Option<T>, E> {\n    pub fn transpose(self) -> Option<Result<T, E>> {\n        match self {\n            Result::Ok(Option::Some(value)) => Option::Some(Result::Ok(value)),\n            Result::Ok(Option::None) => Option::None,\n            Result::Err(error) => Option::Some(Result::Err(error)),\n        }\n    }\n}\nimpl<T, U> Option<(T, U)> {\n    pub fn unzip(self) -> (Option<T>, Option<U>) {\n        match self {\n            Option::Some((left, right)) => (Option::Some(left), Option::Some(right)),\n            Option::None => (Option::None, Option::None),\n        }\n    }\n}\nimpl<T> Option<T> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => Option::Some(function(value)),\n            Option::None => Option::None,\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default(),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn or_else<F: FnOnce() -> Option<T>>(self, function: F) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => function(),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce() -> T>(self, function: F) -> T {\n        match self {\n            Option::Some(value) => value,\n            Option::None => function(),\n        }\n    }\n    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {\n        match self {\n            Option::Some(value) => {\n                if predicate(&value) {\n                    Option::Some(value)\n                } else {\n                    Option::None\n                }\n            }\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => Result::Ok(function(value)),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn map_err<U, F: FnOnce(E) -> U>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(function(error)),\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(_) => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(\n        self,\n        default: D,\n        function: F,\n    ) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => default(error),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or_else<U, F: FnOnce(E) -> Result<T, U>>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => function(error),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, function: F) -> T {\n        match self {\n            Result::Ok(value) => value,\n            Result::Err(error) => function(error),\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a mut T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a mut T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\n/// An ordinary source contract for a canonical initial value.\npub trait Default {\n    fn default() -> Self;\n}\n\nimpl Default for bool {\n    fn default() -> bool {\n        false\n    }\n}\n\nimpl Default for char {\n    fn default() -> char {\n        '\\0'\n    }\n}\n\nimpl Default for u8 {\n    fn default() -> u8 {\n        0u8\n    }\n}\n\nimpl Default for u16 {\n    fn default() -> u16 {\n        0u16\n    }\n}\n\nimpl Default for u32 {\n    fn default() -> u32 {\n        0u32\n    }\n}\n\nimpl Default for u64 {\n    fn default() -> u64 {\n        0u64\n    }\n}\n\nimpl Default for usize {\n    fn default() -> usize {\n        0usize\n    }\n}\n\nimpl Default for i8 {\n    fn default() -> i8 {\n        0i8\n    }\n}\n\nimpl Default for i16 {\n    fn default() -> i16 {\n        0i16\n    }\n}\n\nimpl Default for i32 {\n    fn default() -> i32 {\n        0i32\n    }\n}\n\nimpl Default for i64 {\n    fn default() -> i64 {\n        0i64\n    }\n}\n\nimpl Default for isize {\n    fn default() -> isize {\n        0isize\n    }\n}\n\nimpl Default for f64 {\n    fn default() -> f64 {\n        0.0\n    }\n}\n\nimpl<T> Default for Vec<T> {\n    fn default() -> Vec<T> {\n        Vec::new()\n    }\n}\nimpl<T> Default for Option<T> {\n    fn default() -> Option<T> {\n        Option::None\n    }\n}\nimpl Default for crate::string::String {\n    fn default() -> crate::string::String {\n        crate::string::String::new()\n    }\n}\n\n// Derive expansion resolves this declaration by standard-library identity.\npub fn __lkjscript_default<T: Default>() -> T {\n    T::default()\n}\n")),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn require<T:Clone>(value:&T){}fn bad(){require(&Option::Some(42i64));}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 384usize % workers == shard {
        record(source_case("removing_source_derives_removes_their_contracts_0001", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", Some("//! The source prelude. These enums own their payloads and work with affine values.\npub use crate::integer::{ByteAscii, CheckedArithmetic, ScalarOrder};\npub use crate::iter::{IntoIterator, IterSource, Iterator, IteratorExt};\npub use crate::sequence::{\n    SequencePrefix, SliceClone, SliceCopy, SliceSplitSource, VecAccess, VecAccessMut, VecClear,\n    VecContains, VecExtend, VecMutate, VecSort,\n};\npub use crate::str::{ParseInteger, TextView};\n#[derive(PartialEq, Eq, PartialOrd, Ord)]\npub enum Option<T> {\n    None,\n    Some(T),\n}\n\nimpl<T> Option<T> {\n    pub fn as_ref(&self) -> Option<&T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn as_mut(&mut self) -> Option<&mut T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn expect(self, _message: &str) -> T {\n        self.unwrap()\n    }\n    pub fn and<U>(self, other: Option<U>) -> Option<U> {\n        match self {\n            Option::Some(_) => other,\n            Option::None => Option::None,\n        }\n    }\n    pub fn or(self, other: Option<T>) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => other,\n        }\n    }\n    pub fn xor(self, other: Option<T>) -> Option<T> {\n        match (self, other) {\n            (Option::Some(value), Option::None) | (Option::None, Option::Some(value)) => {\n                Option::Some(value)\n            }\n            _ => Option::None,\n        }\n    }\n    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {\n        match (self, other) {\n            (Option::Some(left), Option::Some(right)) => Option::Some((left, right)),\n            _ => Option::None,\n        }\n    }\n    pub fn replace(&mut self, value: T) -> Option<T> {\n        std::mem::replace(self, Option::Some(value))\n    }\n    pub fn insert(&mut self, value: T) -> &mut T {\n        *self = Option::Some(value);\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n    pub fn get_or_insert(&mut self, value: T) -> &mut T {\n        if self.is_none() {\n            *self = Option::Some(value);\n        }\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n\n    pub fn is_some(&self) -> bool {\n        match self {\n            Option::None => false,\n            Option::Some(_) => true,\n        }\n    }\n\n    pub fn is_none(&self) -> bool {\n        !self.is_some()\n    }\n\n    pub fn is_some_and<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => false,\n        }\n    }\n\n    pub fn is_none_or<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => true,\n        }\n    }\n\n    pub fn ok_or<E>(self, error: E) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error),\n        }\n    }\n\n    pub fn ok_or_else<E, F: FnOnce() -> E>(self, error: F) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error()),\n        }\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => panic!(\"called Option::unwrap on None\"),\n        }\n    }\n\n    pub fn take(&mut self) -> Option<T> {\n        std::mem::replace(self, Option::None)\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => return default,\n        }\n    }\n}\n\n#[derive(PartialEq, Eq)]\npub enum Result<T, E> {\n    Ok(T),\n    Err(E),\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn as_ref(&self) -> Result<&T, &E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> {\n        match self {\n            Result::Ok(_) => other,\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or<F>(self, other: Result<T, F>) -> Result<T, F> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(_) => other,\n        }\n    }\n\n    pub fn is_ok(&self) -> bool {\n        match self {\n            Result::Ok(_) => true,\n            Result::Err(_) => false,\n        }\n    }\n\n    pub fn is_err(&self) -> bool {\n        !self.is_ok()\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => panic!(\"called Result::unwrap on Err\"),\n        }\n    }\n\n    pub fn unwrap_err(self) -> E {\n        match self {\n            Result::Err(value) => return value,\n            Result::Ok(_) => panic!(\"called Result::unwrap_err on Ok\"),\n        }\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => return default,\n        }\n    }\n\n    pub fn ok(self) -> Option<T> {\n        match self {\n            Result::Ok(value) => return Option::Some(value),\n            Result::Err(_) => return Option::None,\n        }\n    }\n\n    pub fn err(self) -> Option<E> {\n        match self {\n            Result::Err(value) => return Option::Some(value),\n            Result::Ok(_) => return Option::None,\n        }\n    }\n}\n\nimpl<T> Option<Option<T>> {\n    pub fn flatten(self) -> Option<T> {\n        match self {\n            Option::Some(value) => value,\n            Option::None => Option::None,\n        }\n    }\n}\nimpl<T, E> Option<Result<T, E>> {\n    pub fn transpose(self) -> Result<Option<T>, E> {\n        match self {\n            Option::Some(Result::Ok(value)) => Result::Ok(Option::Some(value)),\n            Option::Some(Result::Err(error)) => Result::Err(error),\n            Option::None => Result::Ok(Option::None),\n        }\n    }\n}\nimpl<T, E> Result<Option<T>, E> {\n    pub fn transpose(self) -> Option<Result<T, E>> {\n        match self {\n            Result::Ok(Option::Some(value)) => Option::Some(Result::Ok(value)),\n            Result::Ok(Option::None) => Option::None,\n            Result::Err(error) => Option::Some(Result::Err(error)),\n        }\n    }\n}\nimpl<T, U> Option<(T, U)> {\n    pub fn unzip(self) -> (Option<T>, Option<U>) {\n        match self {\n            Option::Some((left, right)) => (Option::Some(left), Option::Some(right)),\n            Option::None => (Option::None, Option::None),\n        }\n    }\n}\nimpl<T> Option<T> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => Option::Some(function(value)),\n            Option::None => Option::None,\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default(),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn or_else<F: FnOnce() -> Option<T>>(self, function: F) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => function(),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce() -> T>(self, function: F) -> T {\n        match self {\n            Option::Some(value) => value,\n            Option::None => function(),\n        }\n    }\n    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {\n        match self {\n            Option::Some(value) => {\n                if predicate(&value) {\n                    Option::Some(value)\n                } else {\n                    Option::None\n                }\n            }\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => Result::Ok(function(value)),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn map_err<U, F: FnOnce(E) -> U>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(function(error)),\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(_) => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(\n        self,\n        default: D,\n        function: F,\n    ) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => default(error),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or_else<U, F: FnOnce(E) -> Result<T, U>>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => function(error),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, function: F) -> T {\n        match self {\n            Result::Ok(value) => value,\n            Result::Err(error) => function(error),\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a mut T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a mut T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\n/// An ordinary source contract for a canonical initial value.\npub trait Default {\n    fn default() -> Self;\n}\n\nimpl Default for bool {\n    fn default() -> bool {\n        false\n    }\n}\n\nimpl Default for char {\n    fn default() -> char {\n        '\\0'\n    }\n}\n\nimpl Default for u8 {\n    fn default() -> u8 {\n        0u8\n    }\n}\n\nimpl Default for u16 {\n    fn default() -> u16 {\n        0u16\n    }\n}\n\nimpl Default for u32 {\n    fn default() -> u32 {\n        0u32\n    }\n}\n\nimpl Default for u64 {\n    fn default() -> u64 {\n        0u64\n    }\n}\n\nimpl Default for usize {\n    fn default() -> usize {\n        0usize\n    }\n}\n\nimpl Default for i8 {\n    fn default() -> i8 {\n        0i8\n    }\n}\n\nimpl Default for i16 {\n    fn default() -> i16 {\n        0i16\n    }\n}\n\nimpl Default for i32 {\n    fn default() -> i32 {\n        0i32\n    }\n}\n\nimpl Default for i64 {\n    fn default() -> i64 {\n        0i64\n    }\n}\n\nimpl Default for isize {\n    fn default() -> isize {\n        0isize\n    }\n}\n\nimpl Default for f64 {\n    fn default() -> f64 {\n        0.0\n    }\n}\n\nimpl<T> Default for Vec<T> {\n    fn default() -> Vec<T> {\n        Vec::new()\n    }\n}\nimpl<T> Default for Option<T> {\n    fn default() -> Option<T> {\n        Option::None\n    }\n}\nimpl Default for crate::string::String {\n    fn default() -> crate::string::String {\n        crate::string::String::new()\n    }\n}\n\n// Derive expansion resolves this declaration by standard-library identity.\npub fn __lkjscript_default<T: Default>() -> T {\n    T::default()\n}\n")),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn require<T:Clone>(value:&T){}fn bad(){let value:Result<i64,i64>=Result::Ok(42);require(&value);}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 385usize % workers == shard {
        record(source_case("removing_source_derives_removes_their_contracts_0002", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", Some("//! The source prelude. These enums own their payloads and work with affine values.\npub use crate::integer::{ByteAscii, CheckedArithmetic, ScalarOrder};\npub use crate::iter::{IntoIterator, IterSource, Iterator, IteratorExt};\npub use crate::sequence::{\n    SequencePrefix, SliceClone, SliceCopy, SliceSplitSource, VecAccess, VecAccessMut, VecClear,\n    VecContains, VecExtend, VecMutate, VecSort,\n};\npub use crate::str::{ParseInteger, TextView};\n#[derive(Clone, Copy, PartialEq)]\npub enum Option<T> {\n    None,\n    Some(T),\n}\n\nimpl<T> Option<T> {\n    pub fn as_ref(&self) -> Option<&T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn as_mut(&mut self) -> Option<&mut T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn expect(self, _message: &str) -> T {\n        self.unwrap()\n    }\n    pub fn and<U>(self, other: Option<U>) -> Option<U> {\n        match self {\n            Option::Some(_) => other,\n            Option::None => Option::None,\n        }\n    }\n    pub fn or(self, other: Option<T>) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => other,\n        }\n    }\n    pub fn xor(self, other: Option<T>) -> Option<T> {\n        match (self, other) {\n            (Option::Some(value), Option::None) | (Option::None, Option::Some(value)) => {\n                Option::Some(value)\n            }\n            _ => Option::None,\n        }\n    }\n    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {\n        match (self, other) {\n            (Option::Some(left), Option::Some(right)) => Option::Some((left, right)),\n            _ => Option::None,\n        }\n    }\n    pub fn replace(&mut self, value: T) -> Option<T> {\n        std::mem::replace(self, Option::Some(value))\n    }\n    pub fn insert(&mut self, value: T) -> &mut T {\n        *self = Option::Some(value);\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n    pub fn get_or_insert(&mut self, value: T) -> &mut T {\n        if self.is_none() {\n            *self = Option::Some(value);\n        }\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n\n    pub fn is_some(&self) -> bool {\n        match self {\n            Option::None => false,\n            Option::Some(_) => true,\n        }\n    }\n\n    pub fn is_none(&self) -> bool {\n        !self.is_some()\n    }\n\n    pub fn is_some_and<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => false,\n        }\n    }\n\n    pub fn is_none_or<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => true,\n        }\n    }\n\n    pub fn ok_or<E>(self, error: E) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error),\n        }\n    }\n\n    pub fn ok_or_else<E, F: FnOnce() -> E>(self, error: F) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error()),\n        }\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => panic!(\"called Option::unwrap on None\"),\n        }\n    }\n\n    pub fn take(&mut self) -> Option<T> {\n        std::mem::replace(self, Option::None)\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => return default,\n        }\n    }\n}\n\n#[derive(Clone, Copy, PartialEq)]\npub enum Result<T, E> {\n    Ok(T),\n    Err(E),\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn as_ref(&self) -> Result<&T, &E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> {\n        match self {\n            Result::Ok(_) => other,\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or<F>(self, other: Result<T, F>) -> Result<T, F> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(_) => other,\n        }\n    }\n\n    pub fn is_ok(&self) -> bool {\n        match self {\n            Result::Ok(_) => true,\n            Result::Err(_) => false,\n        }\n    }\n\n    pub fn is_err(&self) -> bool {\n        !self.is_ok()\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => panic!(\"called Result::unwrap on Err\"),\n        }\n    }\n\n    pub fn unwrap_err(self) -> E {\n        match self {\n            Result::Err(value) => return value,\n            Result::Ok(_) => panic!(\"called Result::unwrap_err on Ok\"),\n        }\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => return default,\n        }\n    }\n\n    pub fn ok(self) -> Option<T> {\n        match self {\n            Result::Ok(value) => return Option::Some(value),\n            Result::Err(_) => return Option::None,\n        }\n    }\n\n    pub fn err(self) -> Option<E> {\n        match self {\n            Result::Err(value) => return Option::Some(value),\n            Result::Ok(_) => return Option::None,\n        }\n    }\n}\n\nimpl<T> Option<Option<T>> {\n    pub fn flatten(self) -> Option<T> {\n        match self {\n            Option::Some(value) => value,\n            Option::None => Option::None,\n        }\n    }\n}\nimpl<T, E> Option<Result<T, E>> {\n    pub fn transpose(self) -> Result<Option<T>, E> {\n        match self {\n            Option::Some(Result::Ok(value)) => Result::Ok(Option::Some(value)),\n            Option::Some(Result::Err(error)) => Result::Err(error),\n            Option::None => Result::Ok(Option::None),\n        }\n    }\n}\nimpl<T, E> Result<Option<T>, E> {\n    pub fn transpose(self) -> Option<Result<T, E>> {\n        match self {\n            Result::Ok(Option::Some(value)) => Option::Some(Result::Ok(value)),\n            Result::Ok(Option::None) => Option::None,\n            Result::Err(error) => Option::Some(Result::Err(error)),\n        }\n    }\n}\nimpl<T, U> Option<(T, U)> {\n    pub fn unzip(self) -> (Option<T>, Option<U>) {\n        match self {\n            Option::Some((left, right)) => (Option::Some(left), Option::Some(right)),\n            Option::None => (Option::None, Option::None),\n        }\n    }\n}\nimpl<T> Option<T> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => Option::Some(function(value)),\n            Option::None => Option::None,\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default(),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn or_else<F: FnOnce() -> Option<T>>(self, function: F) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => function(),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce() -> T>(self, function: F) -> T {\n        match self {\n            Option::Some(value) => value,\n            Option::None => function(),\n        }\n    }\n    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {\n        match self {\n            Option::Some(value) => {\n                if predicate(&value) {\n                    Option::Some(value)\n                } else {\n                    Option::None\n                }\n            }\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => Result::Ok(function(value)),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn map_err<U, F: FnOnce(E) -> U>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(function(error)),\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(_) => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(\n        self,\n        default: D,\n        function: F,\n    ) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => default(error),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or_else<U, F: FnOnce(E) -> Result<T, U>>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => function(error),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, function: F) -> T {\n        match self {\n            Result::Ok(value) => value,\n            Result::Err(error) => function(error),\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a mut T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a mut T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\n/// An ordinary source contract for a canonical initial value.\npub trait Default {\n    fn default() -> Self;\n}\n\nimpl Default for bool {\n    fn default() -> bool {\n        false\n    }\n}\n\nimpl Default for char {\n    fn default() -> char {\n        '\\0'\n    }\n}\n\nimpl Default for u8 {\n    fn default() -> u8 {\n        0u8\n    }\n}\n\nimpl Default for u16 {\n    fn default() -> u16 {\n        0u16\n    }\n}\n\nimpl Default for u32 {\n    fn default() -> u32 {\n        0u32\n    }\n}\n\nimpl Default for u64 {\n    fn default() -> u64 {\n        0u64\n    }\n}\n\nimpl Default for usize {\n    fn default() -> usize {\n        0usize\n    }\n}\n\nimpl Default for i8 {\n    fn default() -> i8 {\n        0i8\n    }\n}\n\nimpl Default for i16 {\n    fn default() -> i16 {\n        0i16\n    }\n}\n\nimpl Default for i32 {\n    fn default() -> i32 {\n        0i32\n    }\n}\n\nimpl Default for i64 {\n    fn default() -> i64 {\n        0i64\n    }\n}\n\nimpl Default for isize {\n    fn default() -> isize {\n        0isize\n    }\n}\n\nimpl Default for f64 {\n    fn default() -> f64 {\n        0.0\n    }\n}\n\nimpl<T> Default for Vec<T> {\n    fn default() -> Vec<T> {\n        Vec::new()\n    }\n}\nimpl<T> Default for Option<T> {\n    fn default() -> Option<T> {\n        Option::None\n    }\n}\nimpl Default for crate::string::String {\n    fn default() -> crate::string::String {\n        crate::string::String::new()\n    }\n}\n\n// Derive expansion resolves this declaration by standard-library identity.\npub fn __lkjscript_default<T: Default>() -> T {\n    T::default()\n}\n")),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn require<T:Eq>(value:&T){}fn bad(){require(&Option::Some(42i64));}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 386usize % workers == shard {
        record(source_case("repeat_extent_does_not_expand_ir_locals_or_instructions_0000", "repeat.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("repeat.rs", Some("pub fn probe()->i64 { let a=[7u8;16]; a.len() as i64 }")),
], ""), errors);
        count += 1;
    }
    if 387usize % workers == shard {
        record(source_case("repeat_extent_does_not_expand_ir_locals_or_instructions_0001", "repeat.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("repeat.rs", Some("pub fn probe()->i64 { let a=[7u8;8192]; a.len() as i64 }")),
], ""), errors);
        count += 1;
    }
    if 388usize % workers == shard {
        record(source_case("repeat_rejects_malformed_ir_signatures_and_uninitialized_seeds_0000", "repeat.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("repeat.rs", Some("pub fn probe()->i64 {let a=[7u8;4];a.len() as i64}")),
], ""), errors);
        count += 1;
    }
    if 389usize % workers == shard {
        record(source_case("repeat_rejects_malformed_ir_signatures_and_uninitialized_seeds_0009", "repeat.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("repeat.rs", Some("#[derive(Clone,Copy)]struct Item {value:i64} pub fn probe()->i64 {let a=[Item {value:7};4];a.len() as i64}")),
], ""), errors);
        count += 1;
    }
    if 390usize % workers == shard {
        record(source_case("repeated_copy_aggregates_have_independent_storage_on_both_routes_0000", "repeat.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("repeat.rs", Some("#[derive(Clone, Copy)]\nstruct Leaf {\n    bytes: [u8; 3],\n    value: i64,\n}\n#[derive(Clone, Copy)]\nenum Choice {\n    Empty,\n    Leaf(Leaf),\n}\n#[derive(Copy)]\nstruct NoClone {\n    value: i64,\n}\nimpl Clone for NoClone {\n    fn clone(&self) -> Self {\n        panic!(\"array repeat must not call Clone\");\n    }\n}\nfn seed(calls: &mut i64) -> Leaf {\n    *calls += 1;\n    Leaf {\n        bytes: [1u8, 2u8, 3u8],\n        value: 7,\n    }\n}\npub fn probe() -> i64 {\n    let mut calls = 0i64;\n    let mut leaves = [seed(&mut calls); 5];\n    assert!(calls == 1);\n    leaves[0].bytes[1] = 99u8;\n    leaves[0].value = 88;\n    assert!(leaves[1].bytes[1] == 2u8 && leaves[4].value == 7);\n    let mut matrix = [[leaves[1]; 2]; 3];\n    matrix[0][0].bytes[0] = 55u8;\n    assert!(matrix[0][1].bytes[0] == 1u8);\n    assert!(matrix[1][0].bytes[0] == 1u8);\n    assert!(leaves[1].bytes[0] == 1u8);\n    let mut choices = [Choice::Leaf(leaves[1]); 3];\n    match &mut choices[0] {\n        Choice::Leaf(value) => value.bytes[2] = 77u8,\n        Choice::Empty => panic!(),\n    }\n    match &choices[2] {\n        Choice::Leaf(value) => assert!(value.bytes[2] == 3u8),\n        Choice::Empty => panic!(),\n    }\n    let absent = [Choice::Empty; 2];\n    assert!(matches!(absent[1], Choice::Empty));\n    let bytes = [7u8; 8192];\n    assert!(bytes.len() == 8192 && bytes[0] == 7u8 && bytes[8191] == 7u8);\n    let chars = ['水'; 3];\n    assert!(chars[2] == '水');\n    let flags = [true; 3];\n    assert!(flags[2]);\n    let numbers = [1.5f64; 3];\n    assert!(numbers[2] == 1.5f64);\n    let copies = [NoClone { value: 42 }; 3];\n    copies[2].value\n}\n")),
], ""), errors);
        count += 1;
    }
    if 391usize % workers == shard {
        record(source_case("replacement_cannot_hide_local_or_conflicting_view_provenance_0000", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("struct View<'a>{value:&'a i64} fn leak<'a>(external:&'a i64)->View<'a>{let local:i64=7;let mut view=View{value:external};*(&mut view)=View{value:&local};view}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 392usize % workers == shard {
        record(source_case("replacement_cannot_hide_local_or_conflicting_view_provenance_0001", "projected_assignment.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_assignment.rs", Some("struct View<'a>{value:&'a i64} fn bad()->i64{let first:i64=1;let mut second:i64=2;let mut view=View{value:&first};*(&mut view)=View{value:&second};second=42;*view.value}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 393usize % workers == shard {
        record(source_case("replacement_cannot_install_a_local_view_in_a_parameter_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        struct View<'a> { value: &'a i64 }\n        fn overwrite<'a>(slot: &mut View<'a>) {\n            let local: i64 = 7;\n            let old = std::mem::replace(slot, View { value: &local });\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 394usize % workers == shard {
        record(source_case("replacement_cannot_return_a_view_of_a_dead_local_0000", "case.rs", None, vec![source_file("case.rs", Some("\n        struct View<'a> { value: &'a i64 }\n        fn leak<'a>(external: &'a i64) -> View<'a> {\n            let local: i64 = 7;\n            let mut view = View { value: &local };\n            std::mem::replace(&mut view, View { value: external })\n        }\n    "))], "E_BORROW"), errors);
        count += 1;
    }
    if 395usize % workers == shard {
        record(source_case("replacement_keeps_old_and_installed_views_usable_0000", "provenance.rs", None, vec![
source_file("provenance.rs", Some("\n        struct View<'a> { value: &'a i64 }\n        fn probe() -> i64 {\n            let left: i64 = 7;\n            let right: i64 = 42;\n            let mut view = View { value: &left };\n            let old = std::mem::replace(&mut view, View { value: &right });\n            *old.value + *view.value\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 396usize % workers == shard {
        record(source_case("replacement_of_a_field_preserves_disjoint_borrows_0000", "provenance.rs", None, vec![
source_file("provenance.rs", Some("\n        struct View<'a> { value: &'a i64 }\n        struct Data { a: i64, b: i64 }\n        struct Pair<'a> { left: View<'a>, right: View<'a> }\n        fn probe() -> i64 {\n            let data = Data { a: 7, b: 35 };\n            let mut pair = Pair {\n                left: View { value: &data.a },\n                right: View { value: &data.b }\n            };\n            let right = &pair.right;\n            let old = std::mem::replace(&mut pair.left, View { value: &data.b });\n            *old.value + *right.value + *pair.left.value\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 397usize % workers == shard {
        record(source_case("replacement_preserves_reference_free_enum_variants_0000", "provenance.rs", None, vec![
source_file("provenance.rs", Some("\n        enum Maybe<'a> { None, Some(&'a i64) }\n        struct Wrapped<'a> { choice: Maybe<'a> }\n        fn empty<'a>(external: &'a i64) -> Wrapped<'a> {\n            let mut view = Wrapped { choice: Maybe::Some(external) };\n            let old = std::mem::replace(&mut view, Wrapped { choice: Maybe::None });\n            view\n        }\n        fn probe() -> i64 {\n            let value: i64 = 42;\n            let view = empty(&value);\n            match view.choice { Maybe::None => 42, Maybe::Some(found) => *found }\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 398usize % workers == shard {
        record(source_case("replacement_transfers_exclusive_reference_fields_0000", "provenance.rs", None, vec![
source_file("provenance.rs", Some("\n        struct View<'a> { value: &'a mut i64 }\n        struct Data { a: i64, b: i64 }\n        fn probe() -> i64 {\n            let mut data = Data { a: 7, b: 35 };\n            let mut view = View { value: &mut data.a };\n            let old = std::mem::replace(&mut view, View { value: &mut data.b });\n            *old.value = 11;\n            *view.value = 31;\n            *old.value + *view.value\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 399usize % workers == shard {
        record(source_case("replacing_view_payloads_keeps_its_named_mutation_contract_boundary_0000", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn update<'a>(target:&mut Option<&'a i64>,source:&Option<&'a i64>){target.clone_from(source);}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 400usize % workers == shard {
        record(source_case("replacing_view_payloads_keeps_its_named_mutation_contract_boundary_0001", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("fn update<'a>(target:&mut Result<&'a i64,&'a i64>,source:&Result<&'a i64,&'a i64>){target.clone_from(source);}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 401usize % workers == shard {
        record(source_case("resolved_nominal_paths_keep_generic_implementations_disjoint_0000", "scope.rs", None, vec![
source_file("scope.rs", Some("pub struct T(pub i64); trait Marker<U>{} impl<T> Marker<T> for crate::T{} impl<U> Marker<U> for i64{} pub fn probe()->i64{0}")),
], ""), errors);
        count += 1;
    }
    if 402usize % workers == shard {
        record(source_case("resolved_nominal_paths_keep_generic_implementations_disjoint_0001", "scope.rs", None, vec![
source_file("scope.rs", Some("pub struct T(pub i64); mod names{pub use crate::T as Target;} trait Marker<U>{} impl<T> Marker<T> for names::Target{} impl<U> Marker<U> for i64{} pub fn probe()->i64{0}")),
], ""), errors);
        count += 1;
    }
    if 403usize % workers == shard {
        record(source_case("resolved_nominal_paths_keep_generic_implementations_disjoint_0002", "scope.rs", None, vec![
source_file("scope.rs", Some("pub struct T(pub i64); mod sibling{pub trait Marker<U>{}} impl<T> sibling::Marker<T> for crate::T{} impl<U> sibling::Marker<U> for i64{} pub fn probe()->i64{0}")),
], ""), errors);
        count += 1;
    }
    if 404usize % workers == shard {
        record(source_case("rigid_parameters_do_not_assume_concrete_structure_0000", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T>(x:T)->i64{x.value}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 405usize % workers == shard {
        record(source_case("rigid_parameters_do_not_assume_concrete_structure_0002", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T>(x:T)->i64{x as i64}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 406usize % workers == shard {
        record(source_case("same_spelled_nominal_operands_remain_different_types_0000", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("mod a{pub struct Value{pub number:i64} impl PartialEq for Value{fn eq(&self,other:&Self)->bool{self.number==other.number}}} mod b{pub struct Value{pub number:i64}} fn bad(left:a::Value,right:b::Value)->bool{left==right}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 407usize % workers == shard {
        record(source_case("same_spelled_source_traits_keep_their_own_method_contracts_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("\ntrait Clone {fn clone(&self)->i64;}\ntrait PartialEq {fn eq(&self,other:&Self)->i64;fn ne(&self,other:&Self)->i64;}\nstruct Item(i64);\nimpl Clone for Item {fn clone(&self)->i64{self.0+30}}\nimpl PartialEq for Item {\n    fn eq(&self,other:&Self)->i64{self.0+other.0}\n    fn ne(&self,other:&Self)->i64{self.0+other.0+100}\n}\nfn copy_value<T:Clone>(value:&T)->i64{value.clone()}\nfn equal_value<T:PartialEq>(left:&T,right:&T)->i64{left.eq(right)}\nfn unequal_value<T:PartialEq>(left:&T,right:&T)->i64{left.ne(right)}\npub fn custom_probe()->i64 {\n    let left=Item(4);let right=Item(5);\n    copy_value(&left)+equal_value(&left,&right)+unequal_value(&left,&right)\n}\n")),
], ""), errors);
        count += 1;
    }
    if 408usize % workers == shard {
        record(source_case("same_spelled_source_traits_keep_their_own_method_contracts_0002", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("trait Clone{fn clone(&self)->i64;} fn wrong<T:Clone>(value:&T)->T{value.clone()}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 409usize % workers == shard {
        record(source_case("same_spelled_source_traits_keep_their_own_method_contracts_0003", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("trait PartialEq{fn eq(&self,other:&Self)->i64;} fn wrong<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 410usize % workers == shard {
        record(source_case("same_spelled_user_traits_are_not_builtin_derive_evidence_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("trait Clone{} struct Value; impl Clone for Value{} #[derive(Clone)] struct Holder(Value);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 411usize % workers == shard {
        record(source_case("same_spelled_user_traits_are_not_builtin_derive_evidence_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("mod a {#[derive(Clone)] pub struct Value;} mod b {pub struct Value;} #[derive(Clone)] struct Holder(a::Value);")),
], ""), errors);
        count += 1;
    }
    if 412usize % workers == shard {
        record(source_case("same_spelled_user_traits_are_not_builtin_derive_evidence_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("mod a {#[derive(Clone)] pub struct Value;} mod b {pub struct Value;} #[derive(Clone)] struct Holder(b::Value);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 413usize % workers == shard {
        record(source_case("scalar_calls_loops_and_inline_values_0000", "scalar", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("scalar", Some("fn sum(a:i64,b:i64,c:i64,d:i64,e:i64,f:i64,g:i64,h:i64)->i64{a+b+c+d+e+f+g+h} pub fn probe()->i64{let mut x=0i64;let mut i=0i64;while i<6 {x=x+i;i=i+1;}sum(x,1,2,3,4,5,6,6)}")),
], ""), errors);
        count += 1;
    }
    if 414usize % workers == shard {
        record(source_case("scalar_calls_loops_and_inline_values_0001", "inline", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("inline", Some("#[derive(Clone,Copy)] struct Pair{a:i64,b:i64} fn swap(p:Pair)->Pair{Pair{a:p.b,b:p.a}} pub fn probe()->i64{let mut p=Pair{a:20,b:22};p=swap(p);let values=[p,p];values[0].a+values[1].b}")),
], ""), errors);
        count += 1;
    }
    if 415usize % workers == shard {
        record(source_case("scalar_owners_retain_addressable_cleanup_storage_0000", "owner-cleanup", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("owner-cleanup", Some("pub fn probe()->i64{let unused=Box::new(7i64);drop(Box::new(9i64));42}")),
], ""), errors);
        count += 1;
    }
    if 416usize % workers == shard {
        record(source_case("scalar_rest_is_not_silently_treated_as_a_wildcard_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad(value:u8)->i64{match value{.. =>1}}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 417usize % workers == shard {
        record(source_case("selected_source_clone_and_equality_bodies_change_derived_behavior_0002", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("\nfn cloned<T:Clone>(value:&T)->T{value.clone()}\nfn updated<T:Clone>(target:&mut T,source:&T){target.clone_from(source);}\nfn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\nfn eq_contract<T:Eq>(value:&T){}\nfn copied_twice<T:Copy>(value:T)->T{let other=value;drop(other);value}\n\nmod misleading {\n    pub trait Clone { fn clone(&self)->Self; }\n    pub trait PartialEq { fn eq(&self,other:&Self)->bool; }\n}\nstruct Token { phase:i64 }\nimpl Drop for Token {\n    fn drop(&mut self){assert!(self.phase==2);self.phase=3;}\n}\nstruct Item { value:Box<i64>, generation:i64, token:Box<Token> }\nimpl Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:99,token:Box::new(Token{phase:1})}}\n    fn eq(&self,other:&Self)->bool{false}\n}\nimpl misleading::Clone for Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:77,token:Box::new(Token{phase:1})}}\n}\nimpl misleading::PartialEq for Item {fn eq(&self,other:&Self)->bool{false}}\nimpl Clone for Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:self.generation+7,token:Box::new(Token{phase:1})}}\n}\nimpl PartialEq for Item {\n    fn eq(&self,other:&Self)->bool{*self.value==*other.value}\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool{!(*self.value==*other.value)}\n}\nimpl Eq for Item {}\nimpl Drop for Item {fn drop(&mut self){assert!(self.token.phase==1);self.token.phase=2;}}\nfn item(value:i64)->Item{Item{value:Box::new(value),generation:0,token:Box::new(Token{phase:1})}}\n\npub fn affine_option_clone_keeps_independent_payloads()->i64{\n    let mut original=Option::Some(item(42));\n    let first=original.clone();\n    let second=cloned(&first);\n    match &mut original{Option::Some(value)=>{*value.value=7;},Option::None=>{}}\n    let second_value=second.unwrap();\n    if *second_value.value!=42 || second_value.generation!=2{return 1;}\n    drop(second_value);\n    let first_value=first.unwrap();\n    if *first_value.value!=42 || first_value.generation!=1{return 2;}\n    let direct=first_value.clone();\n    if direct.generation!=99{return 3;}\n    drop(direct);drop(first_value);\n    let original_value=original.unwrap();\n    if *original_value.value!=7 || original_value.generation!=0{return 4;}\n    0\n}\npub fn affine_result_clones_only_the_active_variant()->i64{\n    let original_ok:Result<Item,Item>=Result::Ok(item(40));\n    let original_err:Result<Item,Item>=Result::Err(item(2));\n    let copied_ok=cloned(&original_ok);\n    let copied_err=original_err.clone();\n    drop(original_ok);drop(original_err);\n    let ok=copied_ok.unwrap();let error=copied_err.unwrap_err();\n    if ok.generation!=1 || error.generation!=1{return 1;}\n    *ok.value+*error.value\n}\npub fn option_clone_from_covers_all_variant_transitions()->i64{\n    let mut target:Option<Item>=Option::None;\n    let some=Option::Some(item(42));\n    let none:Option<Item>=Option::None;\n    updated(&mut target,&some);\n    match &target{Option::Some(value)=>{if value.generation!=1{return 1;}},Option::None=>return 2}\n    target.clone_from(&some);\n    match &mut target{Option::Some(value)=>{*value.value=7;},Option::None=>return 3}\n    match &some{Option::Some(value)=>{if *value.value!=42{return 4;}},Option::None=>return 5}\n    target.clone_from(&none);\n    if target.is_some(){return 6;}\n    updated(&mut target,&none);\n    if !target.is_none(){return 7;}\n    0\n}\npub fn result_clone_from_covers_both_payload_types_and_transitions()->i64{\n    let mut target:Result<Item,Item>=Result::Ok(item(1));\n    let ok:Result<Item,Item>=Result::Ok(item(40));\n    let error:Result<Item,Item>=Result::Err(item(2));\n    target.clone_from(&error);\n    match &target{Result::Err(value)=>{if value.generation!=1 || *value.value!=2{return 1;}},Result::Ok(_)=>return 2}\n    updated(&mut target,&error);\n    updated(&mut target,&ok);\n    match &mut target{Result::Ok(value)=>{if value.generation!=1{return 3;}*value.value=42;},Result::Err(_)=>return 4}\n    match &ok{Result::Ok(value)=>{if *value.value!=40{return 5;}},Result::Err(_)=>return 6}\n    target.clone_from(&ok);\n    let final_value=target.unwrap();\n    if *final_value.value!=40 || final_value.generation!=1{return 7;}\n    0\n}\npub fn option_equality_checks_tags_and_builtin_payload_evidence()->i64{\n    let mut left=Option::Some(item(42));let right=Option::Some(item(42));\n    let other=Option::Some(item(7));let none:Option<Item>=Option::None;\n    eq_contract(&left);\n    if !(left==right) || left!=right || !same(&left,&right) || different(&left,&right){return 1;}\n    if left==other || !(left!=other){return 2;}\n    if left==none || none==right || !(left!=none){return 3;}\n    if !(none==none) || none!=none{return 4;}\n    match &mut left{Option::Some(value)=>{*value.value=7;},Option::None=>return 5}\n    if !(left==other) || left==right{return 6;}\n    0\n}\npub fn result_equality_checks_tags_and_both_payloads()->i64{\n    let ok:Result<Item,Item>=Result::Ok(item(42));let ok_same:Result<Item,Item>=Result::Ok(item(42));\n    let error:Result<Item,Item>=Result::Err(item(42));let error_same:Result<Item,Item>=Result::Err(item(42));\n    let error_other:Result<Item,Item>=Result::Err(item(7));\n    eq_contract(&ok);eq_contract(&error);\n    if !same(&ok,&ok_same) || ok!=ok_same{return 1;}\n    if !same(&error,&error_same) || error!=error_same{return 2;}\n    if ok==error || !(ok!=error){return 3;}\n    if error==error_other || !different(&error,&error_other){return 4;}\n    0\n}\n#[derive(Clone,Copy)] struct NeOverride { value:i64 }\nimpl PartialEq for NeOverride {\n    fn eq(&self,other:&Self)->bool{self.value==other.value}\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool{false}\n}\npub fn derived_ne_negates_aggregate_eq_not_payload_ne()->i64{\n    let left=Option::Some(NeOverride{value:1});let right=Option::Some(NeOverride{value:2});\n    if left==right || !(left!=right) || !left.ne(&right){return 1;}\n    let first:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:1});\n    let second:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:2});\n    if first==second || !(first!=second) || !different(&first,&second){return 2;}\n    0\n}\n#[derive(Copy)] struct CopyPayload { value:i64 }\nimpl Clone for CopyPayload {fn clone(&self)->Self{CopyPayload{value:self.value+1}}}\npub fn generic_enum_clone_calls_clone_even_for_a_copy_payload()->i64{\n    let original=Option::Some(CopyPayload{value:41});\n    let copy=original.clone();\n    if original.unwrap().value!=41 || copy.unwrap().value!=42{return 1;}\n    let ok:Result<CopyPayload,CopyPayload>=Result::Ok(CopyPayload{value:41});\n    let error:Result<CopyPayload,CopyPayload>=Result::Err(CopyPayload{value:41});\n    if cloned(&ok).unwrap().value!=42 || ok.unwrap().value!=41{return 2;}\n    if error.clone().unwrap_err().value!=42 || error.unwrap_err().value!=41{return 3;}\n    0\n}\npub fn copy_is_conditional_and_keeps_original_values_usable()->i64{\n    let option=Option::Some(21i64);let first=copied_twice(option);\n    if first.unwrap()+option.unwrap()!=42{return 1;}\n    let result:Result<i64,i64>=Result::Err(21);let second=copied_twice(result);\n    if second.unwrap_err()+result.unwrap_err()!=42{return 2;}\n    0\n}\nfn option_view<'a>(external:&'a i64)->Option<&'a i64>{let carrier=Option::Some(external);cloned(&carrier)}\nfn result_ok_view<'a>(external:&'a i64)->Result<&'a i64,i64>{let carrier:Result<&i64,i64>=Result::Ok(external);carrier.clone()}\nfn result_err_view<'a>(external:&'a i64)->Result<i64,&'a i64>{let carrier:Result<i64,&i64>=Result::Err(external);cloned(&carrier)}\nfn nested_view<'a>(external:&'a i64)->Result<Option<&'a i64>,i64>{let carrier:Result<Option<&i64>,i64>=Result::Ok(Option::Some(external));carrier.clone()}\nfn absent_view<'a>()->Option<&'a i64>{let carrier:Option<&i64>=Option::None;carrier.clone()}\npub fn shared_payload_clones_outlive_their_enum_carriers()->i64{\n    let value:i64=42;\n    if *option_view(&value).unwrap()!=42{return 1;}\n    if *result_ok_view(&value).unwrap()!=42{return 2;}\n    if *result_err_view(&value).unwrap_err()!=42{return 3;}\n    if *nested_view(&value).unwrap().unwrap()!=42{return 4;}\n    let absent=absent_view();if !absent.is_none(){return 5;}\n    0\n}\npub fn exclusive_payloads_can_be_compared_without_being_copied()->i64{\n    let mut first:i64=42;let mut second:i64=42;\n    {\n        let left=Option::Some(&mut first);let right=Option::Some(&mut second);\n        if !(left==right) || left!=right{return 1;}\n    }\n    first=40;second=2;\n    first+second\n}\npub fn mutation_clone_probe()->i64{Option::Some(item(42)).clone().unwrap().generation}\npub fn mutation_equality_probe()->i64{if Option::Some(item(1))==Option::Some(item(2)){1}else{0}}\npub fn mutation_prelude_probe()->i64{if Option::Some(42i64).is_some(){1}else{0}}\n")),
], ""), errors);
        count += 1;
    }
    if 418usize % workers == shard {
        record(source_case("selected_source_clone_and_equality_bodies_change_derived_behavior_0005", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("\nfn cloned<T:Clone>(value:&T)->T{value.clone()}\nfn updated<T:Clone>(target:&mut T,source:&T){target.clone_from(source);}\nfn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\nfn eq_contract<T:Eq>(value:&T){}\nfn copied_twice<T:Copy>(value:T)->T{let other=value;drop(other);value}\n\nmod misleading {\n    pub trait Clone { fn clone(&self)->Self; }\n    pub trait PartialEq { fn eq(&self,other:&Self)->bool; }\n}\nstruct Token { phase:i64 }\nimpl Drop for Token {\n    fn drop(&mut self){assert!(self.phase==2);self.phase=3;}\n}\nstruct Item { value:Box<i64>, generation:i64, token:Box<Token> }\nimpl Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:99,token:Box::new(Token{phase:1})}}\n    fn eq(&self,other:&Self)->bool{false}\n}\nimpl misleading::Clone for Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:77,token:Box::new(Token{phase:1})}}\n}\nimpl misleading::PartialEq for Item {fn eq(&self,other:&Self)->bool{false}}\nimpl Clone for Item {\n    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:self.generation+1,token:Box::new(Token{phase:1})}}\n}\nimpl PartialEq for Item {\n    fn eq(&self,other:&Self)->bool{true}\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool{!(*self.value==*other.value)}\n}\nimpl Eq for Item {}\nimpl Drop for Item {fn drop(&mut self){assert!(self.token.phase==1);self.token.phase=2;}}\nfn item(value:i64)->Item{Item{value:Box::new(value),generation:0,token:Box::new(Token{phase:1})}}\n\npub fn affine_option_clone_keeps_independent_payloads()->i64{\n    let mut original=Option::Some(item(42));\n    let first=original.clone();\n    let second=cloned(&first);\n    match &mut original{Option::Some(value)=>{*value.value=7;},Option::None=>{}}\n    let second_value=second.unwrap();\n    if *second_value.value!=42 || second_value.generation!=2{return 1;}\n    drop(second_value);\n    let first_value=first.unwrap();\n    if *first_value.value!=42 || first_value.generation!=1{return 2;}\n    let direct=first_value.clone();\n    if direct.generation!=99{return 3;}\n    drop(direct);drop(first_value);\n    let original_value=original.unwrap();\n    if *original_value.value!=7 || original_value.generation!=0{return 4;}\n    0\n}\npub fn affine_result_clones_only_the_active_variant()->i64{\n    let original_ok:Result<Item,Item>=Result::Ok(item(40));\n    let original_err:Result<Item,Item>=Result::Err(item(2));\n    let copied_ok=cloned(&original_ok);\n    let copied_err=original_err.clone();\n    drop(original_ok);drop(original_err);\n    let ok=copied_ok.unwrap();let error=copied_err.unwrap_err();\n    if ok.generation!=1 || error.generation!=1{return 1;}\n    *ok.value+*error.value\n}\npub fn option_clone_from_covers_all_variant_transitions()->i64{\n    let mut target:Option<Item>=Option::None;\n    let some=Option::Some(item(42));\n    let none:Option<Item>=Option::None;\n    updated(&mut target,&some);\n    match &target{Option::Some(value)=>{if value.generation!=1{return 1;}},Option::None=>return 2}\n    target.clone_from(&some);\n    match &mut target{Option::Some(value)=>{*value.value=7;},Option::None=>return 3}\n    match &some{Option::Some(value)=>{if *value.value!=42{return 4;}},Option::None=>return 5}\n    target.clone_from(&none);\n    if target.is_some(){return 6;}\n    updated(&mut target,&none);\n    if !target.is_none(){return 7;}\n    0\n}\npub fn result_clone_from_covers_both_payload_types_and_transitions()->i64{\n    let mut target:Result<Item,Item>=Result::Ok(item(1));\n    let ok:Result<Item,Item>=Result::Ok(item(40));\n    let error:Result<Item,Item>=Result::Err(item(2));\n    target.clone_from(&error);\n    match &target{Result::Err(value)=>{if value.generation!=1 || *value.value!=2{return 1;}},Result::Ok(_)=>return 2}\n    updated(&mut target,&error);\n    updated(&mut target,&ok);\n    match &mut target{Result::Ok(value)=>{if value.generation!=1{return 3;}*value.value=42;},Result::Err(_)=>return 4}\n    match &ok{Result::Ok(value)=>{if *value.value!=40{return 5;}},Result::Err(_)=>return 6}\n    target.clone_from(&ok);\n    let final_value=target.unwrap();\n    if *final_value.value!=40 || final_value.generation!=1{return 7;}\n    0\n}\npub fn option_equality_checks_tags_and_builtin_payload_evidence()->i64{\n    let mut left=Option::Some(item(42));let right=Option::Some(item(42));\n    let other=Option::Some(item(7));let none:Option<Item>=Option::None;\n    eq_contract(&left);\n    if !(left==right) || left!=right || !same(&left,&right) || different(&left,&right){return 1;}\n    if left==other || !(left!=other){return 2;}\n    if left==none || none==right || !(left!=none){return 3;}\n    if !(none==none) || none!=none{return 4;}\n    match &mut left{Option::Some(value)=>{*value.value=7;},Option::None=>return 5}\n    if !(left==other) || left==right{return 6;}\n    0\n}\npub fn result_equality_checks_tags_and_both_payloads()->i64{\n    let ok:Result<Item,Item>=Result::Ok(item(42));let ok_same:Result<Item,Item>=Result::Ok(item(42));\n    let error:Result<Item,Item>=Result::Err(item(42));let error_same:Result<Item,Item>=Result::Err(item(42));\n    let error_other:Result<Item,Item>=Result::Err(item(7));\n    eq_contract(&ok);eq_contract(&error);\n    if !same(&ok,&ok_same) || ok!=ok_same{return 1;}\n    if !same(&error,&error_same) || error!=error_same{return 2;}\n    if ok==error || !(ok!=error){return 3;}\n    if error==error_other || !different(&error,&error_other){return 4;}\n    0\n}\n#[derive(Clone,Copy)] struct NeOverride { value:i64 }\nimpl PartialEq for NeOverride {\n    fn eq(&self,other:&Self)->bool{self.value==other.value}\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,other:&Self)->bool{false}\n}\npub fn derived_ne_negates_aggregate_eq_not_payload_ne()->i64{\n    let left=Option::Some(NeOverride{value:1});let right=Option::Some(NeOverride{value:2});\n    if left==right || !(left!=right) || !left.ne(&right){return 1;}\n    let first:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:1});\n    let second:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:2});\n    if first==second || !(first!=second) || !different(&first,&second){return 2;}\n    0\n}\n#[derive(Copy)] struct CopyPayload { value:i64 }\nimpl Clone for CopyPayload {fn clone(&self)->Self{CopyPayload{value:self.value+1}}}\npub fn generic_enum_clone_calls_clone_even_for_a_copy_payload()->i64{\n    let original=Option::Some(CopyPayload{value:41});\n    let copy=original.clone();\n    if original.unwrap().value!=41 || copy.unwrap().value!=42{return 1;}\n    let ok:Result<CopyPayload,CopyPayload>=Result::Ok(CopyPayload{value:41});\n    let error:Result<CopyPayload,CopyPayload>=Result::Err(CopyPayload{value:41});\n    if cloned(&ok).unwrap().value!=42 || ok.unwrap().value!=41{return 2;}\n    if error.clone().unwrap_err().value!=42 || error.unwrap_err().value!=41{return 3;}\n    0\n}\npub fn copy_is_conditional_and_keeps_original_values_usable()->i64{\n    let option=Option::Some(21i64);let first=copied_twice(option);\n    if first.unwrap()+option.unwrap()!=42{return 1;}\n    let result:Result<i64,i64>=Result::Err(21);let second=copied_twice(result);\n    if second.unwrap_err()+result.unwrap_err()!=42{return 2;}\n    0\n}\nfn option_view<'a>(external:&'a i64)->Option<&'a i64>{let carrier=Option::Some(external);cloned(&carrier)}\nfn result_ok_view<'a>(external:&'a i64)->Result<&'a i64,i64>{let carrier:Result<&i64,i64>=Result::Ok(external);carrier.clone()}\nfn result_err_view<'a>(external:&'a i64)->Result<i64,&'a i64>{let carrier:Result<i64,&i64>=Result::Err(external);cloned(&carrier)}\nfn nested_view<'a>(external:&'a i64)->Result<Option<&'a i64>,i64>{let carrier:Result<Option<&i64>,i64>=Result::Ok(Option::Some(external));carrier.clone()}\nfn absent_view<'a>()->Option<&'a i64>{let carrier:Option<&i64>=Option::None;carrier.clone()}\npub fn shared_payload_clones_outlive_their_enum_carriers()->i64{\n    let value:i64=42;\n    if *option_view(&value).unwrap()!=42{return 1;}\n    if *result_ok_view(&value).unwrap()!=42{return 2;}\n    if *result_err_view(&value).unwrap_err()!=42{return 3;}\n    if *nested_view(&value).unwrap().unwrap()!=42{return 4;}\n    let absent=absent_view();if !absent.is_none(){return 5;}\n    0\n}\npub fn exclusive_payloads_can_be_compared_without_being_copied()->i64{\n    let mut first:i64=42;let mut second:i64=42;\n    {\n        let left=Option::Some(&mut first);let right=Option::Some(&mut second);\n        if !(left==right) || left!=right{return 1;}\n    }\n    first=40;second=2;\n    first+second\n}\npub fn mutation_clone_probe()->i64{Option::Some(item(42)).clone().unwrap().generation}\npub fn mutation_equality_probe()->i64{if Option::Some(item(1))==Option::Some(item(2)){1}else{0}}\npub fn mutation_prelude_probe()->i64{if Option::Some(42i64).is_some(){1}else{0}}\n")),
], ""), errors);
        count += 1;
    }
    if 419usize % workers == shard {
        record(source_case("semantic_definition_errors_accumulate_without_poisoning_call_signatures_0001", "case/main.rs", None, vec![
source_file("case/main.rs", Some("mod child; fn first() -> i64 { true } fn dependent() -> i64 { first() }")),
source_file("case/child.rs", Some("pub fn second() -> bool { 1i64 }")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 420usize % workers == shard {
        record(source_case("shared_projected_results_keep_the_underlying_owner_borrowed_0000", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("\n#[derive(Clone,Copy)] struct Pair<'a>{left:&'a i64,right:&'a i64}\nstruct Numbers{left:i64,right:i64}\nfn swapped<'a>(carrier:&Pair<'a>)->Pair<'a>{Pair{left:carrier.right,right:carrier.left}}\nfn bad()->i64 {\n    let mut values=Numbers{left:1,right:2};\n    let returned={let carrier=Pair{left:&values.left,right:&values.right};swapped(&carrier)};\n    values.left=9;\n    *returned.right\n}\n")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 421usize % workers == shard {
        record(source_case("shared_views_do_not_require_the_referents_to_clone_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; #[derive(Clone)] struct View<'a> {value:&'a NoClone}")),
], ""), errors);
        count += 1;
    }
    if 422usize % workers == shard {
        record(source_case("shared_views_do_not_require_the_referents_to_clone_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; #[derive(Clone)] struct View<'a> {value:&'a mut NoClone}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 423usize % workers == shard {
        record(source_case("sibling_named_variants_keep_their_nominal_identity_0000", "main.rs", None, vec![
source_file("main.rs", Some("mod a{pub enum E{Named{value:i64}}} mod b{pub enum E{Named{value:i64}}} fn take(value:a::E){} fn bad(){take(b::E::Named{value:1});}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 424usize % workers == shard {
        record(source_case("sibling_named_variants_keep_their_nominal_identity_0001", "main.rs", None, vec![
source_file("main.rs", Some("mod a{pub enum E{Named{value:i64}}} mod b{pub enum E{Named{value:i64}}} fn bad(value:a::E)->i64{match value{b::E::Named{value}=>value}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 425usize % workers == shard {
        record(source_case("source_aggregate_static_references_return_through_a_checked_call_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("\n        struct Labels<'a> { left: &'a str, right: &'a str }\n        fn labels() -> Labels<'static> { Labels { left: \"left\", right: \"right\" } }\n        pub fn probe() -> i64 {\n            let pair = labels();\n            pair.left.len() as i64 + pair.right.len() as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 426usize % workers == shard {
        record(source_case("source_an_outer_static_reference_is_not_weakened_by_inner_lifetimes_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("struct View<'a>{value:&'a str} fn widen<'a>(value:&'a View<'a>)->&'static View<'a>{value}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 427usize % workers == shard {
        record(source_case("source_borrowed_text_cannot_claim_a_static_return_lifetime_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn widen(value:&str)->&'static str {value}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 428usize % workers == shard {
        record(source_case("source_carriers_can_die_after_their_stored_references_are_returned_0000", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("\nfn stored<'a>(carrier: &&'a i64) -> &'a i64 { *carrier }\nfn stored_twice<'a>(carrier: &&&'a i64) -> &'a i64 { **carrier }\nfn through_local<'a>(external: &'a i64) -> &'a i64 {\n    let carrier = external;\n    stored(&carrier)\n}\nfn through_two_locals<'a>(external: &'a i64) -> &'a i64 {\n    let first = external;\n    let second = &first;\n    stored_twice(&second)\n}\nfn cloned<T: Clone>(carrier: &T) -> T { carrier.clone() }\nfn generic_through_local<'a>(external: &'a i64) -> &'a i64 {\n    let carrier = external;\n    cloned(&carrier)\n}\n#[derive(Clone, Copy)]\nstruct View<'a> { value: &'a i64 }\nfn copied_view<'a>(carrier: &View<'a>) -> View<'a> { *carrier }\nfn copied_field<'a>(carrier: &View<'a>) -> &'a i64 { carrier.value }\nfn view_through_local<'a>(external: &'a i64) -> View<'a> {\n    let carrier = View { value: external };\n    copied_view(&carrier)\n}\nfn generic_view_through_local<'a>(external: &'a i64) -> View<'a> {\n    let carrier = View { value: external };\n    cloned(&carrier)\n}\nfn field_through_local<'a>(external: &'a i64) -> &'a i64 {\n    let carrier = View { value: external };\n    copied_field(&carrier)\n}\nfn stored_text<'a>(carrier: &&'a str) -> &'a str { *carrier }\nfn text_through_local<'a>(external: &'a str) -> &'a str {\n    let carrier = external;\n    stored_text(&carrier)\n}\npub fn direct_shared_reference() -> i64 {\n    let value: i64 = 42;\n    let returned = through_local(&value);\n    *returned\n}\npub fn twice_projected_shared_reference() -> i64 {\n    let value: i64 = 42;\n    let returned = through_two_locals(&value);\n    *returned\n}\npub fn generic_cloned_reference() -> i64 {\n    let value: i64 = 42;\n    let returned = generic_through_local(&value);\n    *returned\n}\npub fn copied_aggregate() -> i64 {\n    let value: i64 = 42;\n    let returned = view_through_local(&value);\n    *returned.value\n}\npub fn generic_cloned_aggregate() -> i64 {\n    let value: i64 = 42;\n    let returned = generic_view_through_local(&value);\n    *returned.value\n}\npub fn copied_aggregate_field() -> i64 {\n    let value: i64 = 42;\n    let returned = field_through_local(&value);\n    *returned\n}\n#[derive(Clone, Copy)]\nstruct Pair<'a> { left: &'a i64, right: &'a i64 }\nstruct Numbers { left: i64, right: i64 }\nfn swapped<'a>(carrier: &Pair<'a>) -> Pair<'a> {\n    Pair { left: carrier.right, right: carrier.left }\n}\nfn swapped_through_local<'a>(values: &'a Numbers) -> Pair<'a> {\n    let carrier = Pair { left: &values.left, right: &values.right };\n    swapped(&carrier)\n}\npub fn swapped_shared_fields_keep_their_real_targets() -> i64 {\n    let values = Numbers { left: 4, right: 2 };\n    let returned = swapped_through_local(&values);\n    *returned.left * 10 + *returned.right\n}\nfn mutable_carrier<'a>(carrier: &mut &'a i64) -> &'a i64 { *carrier }\npub fn carrier_replacement_keeps_the_original_referent() -> i64 {\n    let first: i64 = 7;\n    let second: i64 = 9;\n    let mut carrier = &first;\n    let returned = mutable_carrier(&mut carrier);\n    carrier = &second;\n    *returned + *carrier\n}\npub fn exclusive_parent_survives_carrier_replacement() -> i64 {\n    let mut first: i64 = 7;\n    let second: i64 = 9;\n    let exclusive = &mut first;\n    let mut carrier = &*exclusive;\n    let returned = mutable_carrier(&mut carrier);\n    carrier = &second;\n    *returned + *carrier\n}\nfn nested_exclusive<'a>(carrier: &&'a &mut i64) -> &'a i64 { &***carrier }\npub fn nested_exclusive_descriptor_survives_until_the_returned_view_dies() -> i64 {\n    let mut value: i64 = 7;\n    let exclusive = &mut value;\n    let inner = &exclusive;\n    let carrier = &inner;\n    let returned = nested_exclusive(carrier);\n    *returned\n}\npub fn copied_and_assigned_carrier_keeps_its_descriptor() -> i64 {\n    let value: i64 = 42;\n    let first = &value;\n    let second = &first;\n    let transferred = second;\n    let mut assigned = second;\n    assigned = transferred;\n    **assigned\n}\npub fn reassignment_releases_the_previous_carrier() -> i64 {\n    let mut value: i64 = 7;\n    let spare: i64 = 9;\n    let first = &value;\n    let second = &spare;\n    let mut carrier = &first;\n    let before = **carrier;\n    carrier = &second;\n    value = 33;\n    before + **carrier + value\n}\npub fn shared_and_exclusive_borrows_still_end_after_their_last_use() -> i64 {\n    let mut value: i64 = 1;\n    let shared = &value;\n    let first = *shared;\n    value = 20;\n    let exclusive = &mut value;\n    let reborrowed = &*exclusive;\n    let second = *reborrowed;\n    value = 21;\n    first + second + value\n}\npub fn branch_reassignment_does_not_retain_old_carrier_edges() -> i64 {\n    let mut value: i64 = 7;\n    let spare: i64 = 9;\n    let first = &value;\n    let second = &spare;\n    let mut carrier = &first;\n    let before = **carrier;\n    if before == 7 { carrier = &second; } else { carrier = &second; }\n    value = 33;\n    before + **carrier + value\n}\nfn returned_from_loop<'a>(external: &'a i64, first_path: bool) -> &'a i64 {\n    let first = external;\n    let second = &first;\n    let mut returned = external;\n    loop {\n        if first_path { returned = stored(&first); break; }\n        returned = stored_twice(&second);\n        break;\n    }\n    returned\n}\npub fn loop_break_paths_preserve_carrier_dependencies() -> i64 {\n    let value: i64 = 21;\n    let first = returned_from_loop(&value, true);\n    let second = returned_from_loop(&value, false);\n    *first + *second\n}\npub fn text_reference() -> usize {\n    let mut bytes: Vec<u8> = Vec::new();\n    bytes.push(206); bytes.push(177); bytes.push(230); bytes.push(176); bytes.push(180);\n    bytes.push(240); bytes.push(159); bytes.push(152); bytes.push(128);\n    let text = std::str::from_utf8(bytes.as_slice()).unwrap();\n    let returned = text_through_local(text);\n    returned.len()\n}\n")),
], ""), errors);
        count += 1;
    }
    if 429usize % workers == shard {
        record(source_case("source_exclusive_leaf_projection_is_a_named_unsupported_boundary_0000", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn bad<'a>(carrier:&&'a mut i64)->&'a i64{&**carrier}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 430usize % workers == shard {
        record(source_case("source_generic_trait_calls_preserve_static_result_contracts_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("\n        trait Text { fn get(&self) -> &'static str; }\n        impl Text for i64 { fn get(&self) -> &'static str { \"checked\" } }\n        fn via<T:Text>(value: &T) -> &'static str { value.get() }\n        pub fn probe() -> i64 { via(&1i64).len() as i64 }\n    ")),
], ""), errors);
        count += 1;
    }
    if 431usize % workers == shard {
        record(source_case("source_impl_methods_cannot_weaken_a_static_trait_result_contract_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("trait Text{fn get(&self)->&'static str;} impl Text for String{fn get(&self)->&str{self.as_str()}}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 432usize % workers == shard {
        record(source_case("source_literal_returns_and_byte_views_survive_local_reference_cleanup_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("\n        fn literal() -> &'static str { \"Aé水😀\" }\n        fn bytes() -> &'static [u8] { let text = literal(); text.as_bytes() }\n        pub fn probe() -> i64 {\n            let view = bytes();\n            assert_eq!(view.len(), 10);\n            view[0] as i64 + view[9] as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 433usize % workers == shard {
        record(source_case("source_local_referents_and_descriptors_still_cannot_escape_0000", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn stored<'a>(carrier:&&'a i64)->&'a i64{*carrier} fn cloned<T:Clone>(carrier:&T)->T{carrier.clone()} #[derive(Clone,Copy)] struct View<'a>{value:&'a i64,number:i64} fn copied_view<'a>(carrier:&View<'a>)->View<'a>{*carrier} fn leak<'a>(external:&'a i64)->&'a i64{let local:i64=7;let carrier=&local;stored(&carrier)}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 434usize % workers == shard {
        record(source_case("source_local_referents_and_descriptors_still_cannot_escape_0001", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn stored<'a>(carrier:&&'a i64)->&'a i64{*carrier} fn cloned<T:Clone>(carrier:&T)->T{carrier.clone()} #[derive(Clone,Copy)] struct View<'a>{value:&'a i64,number:i64} fn copied_view<'a>(carrier:&View<'a>)->View<'a>{*carrier} fn leak<'a>(external:&'a i64)->&'a i64{let local:i64=7;let carrier=&local;cloned(&carrier)}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 435usize % workers == shard {
        record(source_case("source_local_referents_and_descriptors_still_cannot_escape_0002", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn stored<'a>(carrier:&&'a i64)->&'a i64{*carrier} fn cloned<T:Clone>(carrier:&T)->T{carrier.clone()} #[derive(Clone,Copy)] struct View<'a>{value:&'a i64,number:i64} fn copied_view<'a>(carrier:&View<'a>)->View<'a>{*carrier} fn leak<'a>(external:&'a i64)->View<'a>{let local:i64=7;let carrier=View{value:&local,number:42};copied_view(&carrier)}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 436usize % workers == shard {
        record(source_case("source_local_referents_and_descriptors_still_cannot_escape_0003", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn stored<'a>(carrier:&&'a i64)->&'a i64{*carrier} fn cloned<T:Clone>(carrier:&T)->T{carrier.clone()} #[derive(Clone,Copy)] struct View<'a>{value:&'a i64,number:i64} fn copied_view<'a>(carrier:&View<'a>)->View<'a>{*carrier} fn leak<'a>(carrier:&View<'a>)->&'a i64{&carrier.number}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 437usize % workers == shard {
        record(source_case("source_local_referents_and_descriptors_still_cannot_escape_0004", "projected_returns.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("projected_returns.rs", Some("fn stored<'a>(carrier:&&'a i64)->&'a i64{*carrier} fn cloned<T:Clone>(carrier:&T)->T{carrier.clone()} #[derive(Clone,Copy)] struct View<'a>{value:&'a i64,number:i64} fn copied_view<'a>(carrier:&View<'a>)->View<'a>{*carrier} fn borrow_carrier<'a>(value:&'a &'a i64)->&'a &'a i64{value} fn leak<'a>(external:&'a i64)->&'a &'a i64{let carrier=external;borrow_carrier(&carrier)}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 438usize % workers == shard {
        record(source_case("source_owned_string_views_cannot_escape_their_owner_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn leak()->&'static str {let value=String::from_str(\"local\");value.as_str()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 439usize % workers == shard {
        record(source_case("source_owned_string_views_cannot_escape_their_owner_0001", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn leak()->&'static [u8] {let value=String::from_str(\"local\");value.as_str().as_bytes()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 440usize % workers == shard {
        record(source_case("source_owned_string_views_cannot_escape_their_owner_0002", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn leak()->&'static &'static str {let value=\"static\";&value}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 441usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn takes_static(value:&'static str)->usize{value.len()} fn probe()->usize{let owned=String::from_str(\"local\");takes_static(owned.as_str())}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 442usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0001", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn unused<T>(value:&'static str){}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 443usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0002", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn mixed<'a>(value:&'a str)->(&'static str,&'a str){(value,value)}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 444usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0003", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn mixed(value:&str)->(&'static str,&str){(value,value)}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 445usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0004", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("struct Fixed{value:&'static str}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 446usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0005", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("enum Fixed{Value(&'static str),Empty}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 447usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0006", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn local(value:&str){let claimed:&'static str=value;}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 448usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0007", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn dead<T>(value:T){return;let claimed:&'static str=\"literal\";}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 449usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0008", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("type Fixed=&'static str; fn unused(value:Fixed){}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 450usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0009", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("struct View<'a>{value:&'a str} fn unused(value:View<'static>){}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 451usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0010", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("trait Source{type Item;} struct Value{} impl Source for Value{type Item=&'static str;}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 452usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0011", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn bound<T:'static>(value:T){}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 453usize % workers == shard {
        record(source_case("source_static_constraints_without_leaf_contracts_are_named_unsupported_0012", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn lifetime_bound<'a:'static>(value:&'a str){}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 454usize % workers == shard {
        record(source_case("source_static_literals_can_share_a_shorter_dynamic_view_lifetime_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("\n        struct View<'a> { prefix: &'a str, value: &'a str }\n        fn view<'a>(value: &'a str) -> View<'a> { View { prefix: \"label\", value } }\n        pub fn probe() -> i64 {\n            let owned = String::from_str(\"data\");\n            let pair = view(owned.as_str());\n            pair.prefix.len() as i64 + pair.value.len() as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 455usize % workers == shard {
        record(source_case("source_string_copying_and_append_preserve_literals_and_borrowed_views_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("\n        fn literal() -> &'static str { \"é\" }\n        fn borrowed(value: &String) -> &str { value.as_str() }\n        pub fn probe() -> i64 {\n            let mut i: i64 = 0;\n            while i < 256 {\n                let mut owned = String::from_str(literal());\n                owned.push_str(\"::水\");\n                let view = borrowed(&owned);\n                assert_eq!(view.len(), 7);\n                assert_eq!(view.as_bytes()[2], 58);\n                i = i + 1;\n            }\n            let text = literal();\n            text.len() as i64 + text.as_bytes()[0] as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 456usize % workers == shard {
        record(source_case("source_unused_generic_bodies_cannot_return_local_owned_text_0000", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("fn leak<T>(unused:T)->&'static str{let owned=String::from_str(\"local\");owned.as_str()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 457usize % workers == shard {
        record(source_case("source_unused_generic_bodies_cannot_return_local_owned_text_0001", "text_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("text_probe.rs", Some("type View<'a>=&'a str;fn leak<T>(unused:T)->View<'static>{let owned=String::from_str(\"local\");owned.as_str()}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 458usize % workers == shard {
        record(source_case("startup_adapts_to_an_external_address_space_limit_0000", "limited-startup", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("limited-startup", Some("pub fn probe()->i64{let value=Box::new(42i64);*value}")),
], ""), errors);
        count += 1;
    }
    if 459usize % workers == shard {
        record(source_case("statement_and_value_contexts_match_the_same_rust_source_0000", "block_statements.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("block_statements.rs", Some("\npub fn if_before_dereference() -> i64 {\n    let value: i64 = 42; let reference = &value;\n    if true {} *reference\n}\npub fn if_before_negative() -> i64 { if true {} -42i64 }\npub fn block_before_dereference() -> i64 {\n    let value: i64 = 42; let reference = &value;\n    {} *reference\n}\npub fn block_before_group() -> i64 { {} (42i64) }\npub fn if_before_array() -> i64 { if true {} [42i64][0] }\npub fn if_before_borrow() -> i64 {\n    let value: i64 = 42;\n    let reference = { if true {} &value };\n    *reference\n}\npub fn if_before_not() -> i64 {\n    let value = { if true {} !false };\n    if value { 42 } else { 0 }\n}\npub fn while_before_dereference() -> i64 {\n    let value: i64 = 42; let reference = &value;\n    while false {} *reference\n}\npub fn loop_before_dereference() -> i64 {\n    let value: i64 = 42; let reference = &value;\n    loop { break; } *reference\n}\npub fn for_before_dereference() -> i64 {\n    let value: i64 = 42; let reference = &value;\n    for index in 0..2 {} *reference\n}\npub fn match_before_dereference() -> i64 {\n    let value: i64 = 42; let reference = &value;\n    match 0i64 { _ => {} } *reference\n}\npub fn parenthesized_if_binary() -> i64 { (if true { 6i64 } else { 7i64 }) * 7i64 }\npub fn parenthesized_block_binary() -> i64 { (({ 6i64 })) * 7i64 }\npub fn parenthesized_block_cast() -> i64 { ({ 42i64 }) as i64 }\npub fn initializer_if_binary() -> i64 {\n    let value = if true { 6i64 } else { 7i64 } * 7i64;\n    value\n}\npub fn return_if_binary() -> i64 { return if true { 6i64 } else { 7i64 } * 7i64; }\npub fn binary_right_block() -> i64 { 2i64 + { 40i64 } }\npub fn binary_right_if() -> i64 { 2i64 + if true { 40i64 } else { 39i64 } }\nfn identity(value: i64) -> i64 { value }\npub fn argument_if_binary() -> i64 { identity(if true { 6i64 } else { 7i64 } * 7i64) }\npub fn prefixed_block_value() -> i64 { -{ 42i64 } }\npub fn block_method_continuation() -> i64 { { 6i64 }.clone() * 7i64 }\npub fn if_method_continuation() -> i64 { if true { 6i64 } else { 7i64 }.clone() * 7i64 }\nstruct Field { value: i64 }\npub fn if_field_continuation() -> i64 {\n    if true { Field { value: 42 } } else { Field { value: 0 } }.value\n}\npub fn parenthesized_index() -> i64 { (if true { [42i64] } else { [0i64] })[0] }\npub fn if_arm_before_negative_pattern() -> i64 {\n    match 0i64 {\n        0 => if true { 42i64 } else { 0i64 }\n        -1 => 1i64,\n        _ => 2i64,\n    }\n}\npub fn block_arm_method_continuation() -> i64 {\n    match 0i64 { 0 => { 42i64 }.clone(), _ => 0i64 }\n}\npub fn control_flow_arms_can_omit_commas() -> i64 {\n    let mut value: i64 = 0;\n    match 0i64 {\n        0 => while value < 1 { value = value + 1; }\n        1 => for index in 0..1 { value = value + 1; }\n        _ => loop { break; }\n    }\n    value + 41\n}\npub fn explicit_semicolons_discard_block_values() -> i64 {\n    { 1i64 };\n    if true { 2i64 } else { 3i64 };\n    match 0i64 { _ => 4i64 };\n    42\n}\npub fn unparenthesized_block_tail_is_a_value() -> i64 { { 42i64 } }\npub fn parenthesized_block_tail_is_a_value() -> i64 { ({ 42i64 }) }\npub fn unparenthesized_if_tail_is_a_value() -> i64 { if true { 42 } else { 0 } }\npub fn return_still_exits_before_later_effects() -> i64 {\n    let mut value: i64 = 0;\n    return 42;\n    value = 1;\n    value\n}\npub fn diverging_implicit_statement() -> i64 {\n    if true { return 42; } else { return 0; }\n    1\n}\nstruct Tracked<'a> { count: &'a mut i64 }\nimpl<'a> Drop for Tracked<'a> { fn drop(&mut self) { *self.count = *self.count + 1; } }\npub fn discarded_affine_block_values_drop_once() -> i64 {\n    let mut count: i64 = 0;\n    { Tracked { count: &mut count } };\n    if true { Tracked { count: &mut count } } else { Tracked { count: &mut count } };\n    count * 21\n}\n")),
], ""), errors);
        count += 1;
    }
    if 460usize % workers == shard {
        record(source_case("statically_false_branches_still_check_generic_bodies_0000", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T>(x:T)->i64{if false{let wrong=1i64+true;}0}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 461usize % workers == shard {
        record(source_case("strings_validate_and_project_borrowed_bytes_0000", "probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("probe.rs", Some("use std::bytes::*; use std::collections::BTreeMap;\n\n        fn string_probe() -> i64 {\n            let input: Vec<u8> = vec![65, 240, 159, 152, 128];\n            let mut text = String::from_utf8(input).unwrap();\n            assert_eq!(text.len(), 5);\n            let first = {\n                let view: &[u8] = text.as_bytes();\n                view[0] as i64\n            };\n            text.clear();\n            assert!(text.is_empty());\n            let invalid: Vec<u8> = vec![255];\n            let failure = String::from_utf8(invalid).unwrap_err();\n            let recovered = failure.into_bytes();\n            first + recovered[0] as i64\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    if 462usize % workers == shard {
        record(source_case("structural_derive_contracts_lower_affine_clone_bodies_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("#[derive(Clone)] struct Item {value:Box<i64>} fn probe()->i64{let item=Item{value:Box::new(42)};let cloned=item.clone();drop(item);*cloned.value}")),
], ""), errors);
        count += 1;
    }
    if 463usize % workers == shard {
        record(source_case("structural_obligations_reach_rigid_children_0001", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Send>(x:T){} fn bad<U>(x:Vec<U>){need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 464usize % workers == shard {
        record(source_case("structural_obligations_reach_rigid_children_0003", "case.rs", None, vec![source_file("case.rs", Some("fn need<T:Send>(x:T){} fn good<U:Send>(x:Vec<U>){need(x);}"))], ""), errors);
        count += 1;
    }
    if 465usize % workers == shard {
        record(source_case("supertrait_evidence_is_available_at_the_definition_0001", "case.rs", None, vec![source_file("case.rs", Some("trait Base{fn score(&self)->i64;} trait Child:Base{} fn good<T:Child>(x:&T)->i64{x.score()}"))], ""), errors);
        count += 1;
    }
    if 466usize % workers == shard {
        record(source_case("supplied_prelude_bytes_control_behavior_and_diagnostics_0000", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("pub fn probe()->i64{let value=Option::Some(42i64).clone();if value.is_some(){1}else{0}}")),
], ""), errors);
        count += 1;
    }
    if 467usize % workers == shard {
        record(source_case("supplied_prelude_bytes_control_behavior_and_diagnostics_0001", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", Some("//! The source prelude. These enums own their payloads and work with affine values.\npub use crate::integer::{ByteAscii, CheckedArithmetic, ScalarOrder};\npub use crate::iter::{IntoIterator, IterSource, Iterator, IteratorExt};\npub use crate::sequence::{\n    SequencePrefix, SliceClone, SliceCopy, SliceSplitSource, VecAccess, VecAccessMut, VecClear,\n    VecContains, VecExtend, VecMutate, VecSort,\n};\npub use crate::str::{ParseInteger, TextView};\n#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]\npub enum Option<T> {\n    None,\n    Some(T),\n}\n\nimpl<T> Option<T> {\n    pub fn as_ref(&self) -> Option<&T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn as_mut(&mut self) -> Option<&mut T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn expect(self, _message: &str) -> T {\n        self.unwrap()\n    }\n    pub fn and<U>(self, other: Option<U>) -> Option<U> {\n        match self {\n            Option::Some(_) => other,\n            Option::None => Option::None,\n        }\n    }\n    pub fn or(self, other: Option<T>) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => other,\n        }\n    }\n    pub fn xor(self, other: Option<T>) -> Option<T> {\n        match (self, other) {\n            (Option::Some(value), Option::None) | (Option::None, Option::Some(value)) => {\n                Option::Some(value)\n            }\n            _ => Option::None,\n        }\n    }\n    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {\n        match (self, other) {\n            (Option::Some(left), Option::Some(right)) => Option::Some((left, right)),\n            _ => Option::None,\n        }\n    }\n    pub fn replace(&mut self, value: T) -> Option<T> {\n        std::mem::replace(self, Option::Some(value))\n    }\n    pub fn insert(&mut self, value: T) -> &mut T {\n        *self = Option::Some(value);\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n    pub fn get_or_insert(&mut self, value: T) -> &mut T {\n        if self.is_none() {\n            *self = Option::Some(value);\n        }\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n\n    pub fn is_some(&self) -> bool {\n        match self {\n            Option::None => false,\n            Option::Some(_) => false,\n        }\n    }\n\n    pub fn is_none(&self) -> bool {\n        !self.is_some()\n    }\n\n    pub fn is_some_and<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => false,\n        }\n    }\n\n    pub fn is_none_or<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => true,\n        }\n    }\n\n    pub fn ok_or<E>(self, error: E) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error),\n        }\n    }\n\n    pub fn ok_or_else<E, F: FnOnce() -> E>(self, error: F) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error()),\n        }\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => panic!(\"called Option::unwrap on None\"),\n        }\n    }\n\n    pub fn take(&mut self) -> Option<T> {\n        std::mem::replace(self, Option::None)\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => return default,\n        }\n    }\n}\n\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum Result<T, E> {\n    Ok(T),\n    Err(E),\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn as_ref(&self) -> Result<&T, &E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> {\n        match self {\n            Result::Ok(_) => other,\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or<F>(self, other: Result<T, F>) -> Result<T, F> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(_) => other,\n        }\n    }\n\n    pub fn is_ok(&self) -> bool {\n        match self {\n            Result::Ok(_) => true,\n            Result::Err(_) => false,\n        }\n    }\n\n    pub fn is_err(&self) -> bool {\n        !self.is_ok()\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => panic!(\"called Result::unwrap on Err\"),\n        }\n    }\n\n    pub fn unwrap_err(self) -> E {\n        match self {\n            Result::Err(value) => return value,\n            Result::Ok(_) => panic!(\"called Result::unwrap_err on Ok\"),\n        }\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => return default,\n        }\n    }\n\n    pub fn ok(self) -> Option<T> {\n        match self {\n            Result::Ok(value) => return Option::Some(value),\n            Result::Err(_) => return Option::None,\n        }\n    }\n\n    pub fn err(self) -> Option<E> {\n        match self {\n            Result::Err(value) => return Option::Some(value),\n            Result::Ok(_) => return Option::None,\n        }\n    }\n}\n\nimpl<T> Option<Option<T>> {\n    pub fn flatten(self) -> Option<T> {\n        match self {\n            Option::Some(value) => value,\n            Option::None => Option::None,\n        }\n    }\n}\nimpl<T, E> Option<Result<T, E>> {\n    pub fn transpose(self) -> Result<Option<T>, E> {\n        match self {\n            Option::Some(Result::Ok(value)) => Result::Ok(Option::Some(value)),\n            Option::Some(Result::Err(error)) => Result::Err(error),\n            Option::None => Result::Ok(Option::None),\n        }\n    }\n}\nimpl<T, E> Result<Option<T>, E> {\n    pub fn transpose(self) -> Option<Result<T, E>> {\n        match self {\n            Result::Ok(Option::Some(value)) => Option::Some(Result::Ok(value)),\n            Result::Ok(Option::None) => Option::None,\n            Result::Err(error) => Option::Some(Result::Err(error)),\n        }\n    }\n}\nimpl<T, U> Option<(T, U)> {\n    pub fn unzip(self) -> (Option<T>, Option<U>) {\n        match self {\n            Option::Some((left, right)) => (Option::Some(left), Option::Some(right)),\n            Option::None => (Option::None, Option::None),\n        }\n    }\n}\nimpl<T> Option<T> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => Option::Some(function(value)),\n            Option::None => Option::None,\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default(),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn or_else<F: FnOnce() -> Option<T>>(self, function: F) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => function(),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce() -> T>(self, function: F) -> T {\n        match self {\n            Option::Some(value) => value,\n            Option::None => function(),\n        }\n    }\n    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {\n        match self {\n            Option::Some(value) => {\n                if predicate(&value) {\n                    Option::Some(value)\n                } else {\n                    Option::None\n                }\n            }\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => Result::Ok(function(value)),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn map_err<U, F: FnOnce(E) -> U>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(function(error)),\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(_) => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(\n        self,\n        default: D,\n        function: F,\n    ) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => default(error),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or_else<U, F: FnOnce(E) -> Result<T, U>>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => function(error),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, function: F) -> T {\n        match self {\n            Result::Ok(value) => value,\n            Result::Err(error) => function(error),\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a mut T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a mut T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\n/// An ordinary source contract for a canonical initial value.\npub trait Default {\n    fn default() -> Self;\n}\n\nimpl Default for bool {\n    fn default() -> bool {\n        false\n    }\n}\n\nimpl Default for char {\n    fn default() -> char {\n        '\\0'\n    }\n}\n\nimpl Default for u8 {\n    fn default() -> u8 {\n        0u8\n    }\n}\n\nimpl Default for u16 {\n    fn default() -> u16 {\n        0u16\n    }\n}\n\nimpl Default for u32 {\n    fn default() -> u32 {\n        0u32\n    }\n}\n\nimpl Default for u64 {\n    fn default() -> u64 {\n        0u64\n    }\n}\n\nimpl Default for usize {\n    fn default() -> usize {\n        0usize\n    }\n}\n\nimpl Default for i8 {\n    fn default() -> i8 {\n        0i8\n    }\n}\n\nimpl Default for i16 {\n    fn default() -> i16 {\n        0i16\n    }\n}\n\nimpl Default for i32 {\n    fn default() -> i32 {\n        0i32\n    }\n}\n\nimpl Default for i64 {\n    fn default() -> i64 {\n        0i64\n    }\n}\n\nimpl Default for isize {\n    fn default() -> isize {\n        0isize\n    }\n}\n\nimpl Default for f64 {\n    fn default() -> f64 {\n        0.0\n    }\n}\n\nimpl<T> Default for Vec<T> {\n    fn default() -> Vec<T> {\n        Vec::new()\n    }\n}\nimpl<T> Default for Option<T> {\n    fn default() -> Option<T> {\n        Option::None\n    }\n}\nimpl Default for crate::string::String {\n    fn default() -> crate::string::String {\n        crate::string::String::new()\n    }\n}\n\n// Derive expansion resolves this declaration by standard-library identity.\npub fn __lkjscript_default<T: Default>() -> T {\n    T::default()\n}\n")),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("pub fn probe()->i64{let value=Option::Some(42i64).clone();if value.is_some(){1}else{0}}")),
], ""), errors);
        count += 1;
    }
    if 468usize % workers == shard {
        record(source_case_at("supplied_prelude_bytes_control_behavior_and_diagnostics_0004", "option_result_traits.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", Some("//! The source prelude. These enums own their payloads and work with affine values.\npub use crate::integer::{ByteAscii, CheckedArithmetic, ScalarOrder};\npub use crate::iter::{IntoIterator, IterSource, Iterator, IteratorExt};\npub use crate::sequence::{\n    SequencePrefix, SliceClone, SliceCopy, SliceSplitSource, VecAccess, VecAccessMut, VecClear,\n    VecContains, VecExtend, VecMutate, VecSort,\n};\npub use crate::str::{ParseInteger, TextView};\n#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]\npub enum Option<T> {\n    None,\n    Some(T),\n}\n\nimpl<T> Option<T> {\n    pub fn as_ref(&self) -> Option<&T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn as_mut(&mut self) -> Option<&mut T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn expect(self, _message: &str) -> T {\n        self.unwrap()\n    }\n    pub fn and<U>(self, other: Option<U>) -> Option<U> {\n        match self {\n            Option::Some(_) => other,\n            Option::None => Option::None,\n        }\n    }\n    pub fn or(self, other: Option<T>) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => other,\n        }\n    }\n    pub fn xor(self, other: Option<T>) -> Option<T> {\n        match (self, other) {\n            (Option::Some(value), Option::None) | (Option::None, Option::Some(value)) => {\n                Option::Some(value)\n            }\n            _ => Option::None,\n        }\n    }\n    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {\n        match (self, other) {\n            (Option::Some(left), Option::Some(right)) => Option::Some((left, right)),\n            _ => Option::None,\n        }\n    }\n    pub fn replace(&mut self, value: T) -> Option<T> {\n        std::mem::replace(self, Option::Some(value))\n    }\n    pub fn insert(&mut self, value: T) -> &mut T {\n        *self = Option::Some(value);\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n    pub fn get_or_insert(&mut self, value: T) -> &mut T {\n        if self.is_none() {\n            *self = Option::Some(value);\n        }\n        match self {\n            Option::Some(value) => value,\n            Option::None => unreachable!(),\n        }\n    }\n\n    pub fn is_some(&self) -> bool {\n        match self {\n            Option::None => false,\n            Option::Some(_) => 1i64,\n        }\n    }\n\n    pub fn is_none(&self) -> bool {\n        !self.is_some()\n    }\n\n    pub fn is_some_and<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => false,\n        }\n    }\n\n    pub fn is_none_or<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {\n        match self {\n            Option::Some(value) => predicate(value),\n            Option::None => true,\n        }\n    }\n\n    pub fn ok_or<E>(self, error: E) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error),\n        }\n    }\n\n    pub fn ok_or_else<E, F: FnOnce() -> E>(self, error: F) -> Result<T, E> {\n        match self {\n            Option::Some(value) => Result::Ok(value),\n            Option::None => Result::Err(error()),\n        }\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => panic!(\"called Option::unwrap on None\"),\n        }\n    }\n\n    pub fn take(&mut self) -> Option<T> {\n        std::mem::replace(self, Option::None)\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Option::Some(value) => return value,\n            Option::None => return default,\n        }\n    }\n}\n\n#[derive(Clone, Copy, PartialEq, Eq)]\npub enum Result<T, E> {\n    Ok(T),\n    Err(E),\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn as_ref(&self) -> Result<&T, &E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> {\n        match self {\n            Result::Ok(_) => other,\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or<F>(self, other: Result<T, F>) -> Result<T, F> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(_) => other,\n        }\n    }\n\n    pub fn is_ok(&self) -> bool {\n        match self {\n            Result::Ok(_) => true,\n            Result::Err(_) => false,\n        }\n    }\n\n    pub fn is_err(&self) -> bool {\n        !self.is_ok()\n    }\n\n    pub fn unwrap(self) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => panic!(\"called Result::unwrap on Err\"),\n        }\n    }\n\n    pub fn unwrap_err(self) -> E {\n        match self {\n            Result::Err(value) => return value,\n            Result::Ok(_) => panic!(\"called Result::unwrap_err on Ok\"),\n        }\n    }\n\n    pub fn unwrap_or(self, default: T) -> T {\n        match self {\n            Result::Ok(value) => return value,\n            Result::Err(_) => return default,\n        }\n    }\n\n    pub fn ok(self) -> Option<T> {\n        match self {\n            Result::Ok(value) => return Option::Some(value),\n            Result::Err(_) => return Option::None,\n        }\n    }\n\n    pub fn err(self) -> Option<E> {\n        match self {\n            Result::Err(value) => return Option::Some(value),\n            Result::Ok(_) => return Option::None,\n        }\n    }\n}\n\nimpl<T> Option<Option<T>> {\n    pub fn flatten(self) -> Option<T> {\n        match self {\n            Option::Some(value) => value,\n            Option::None => Option::None,\n        }\n    }\n}\nimpl<T, E> Option<Result<T, E>> {\n    pub fn transpose(self) -> Result<Option<T>, E> {\n        match self {\n            Option::Some(Result::Ok(value)) => Result::Ok(Option::Some(value)),\n            Option::Some(Result::Err(error)) => Result::Err(error),\n            Option::None => Result::Ok(Option::None),\n        }\n    }\n}\nimpl<T, E> Result<Option<T>, E> {\n    pub fn transpose(self) -> Option<Result<T, E>> {\n        match self {\n            Result::Ok(Option::Some(value)) => Option::Some(Result::Ok(value)),\n            Result::Ok(Option::None) => Option::None,\n            Result::Err(error) => Option::Some(Result::Err(error)),\n        }\n    }\n}\nimpl<T, U> Option<(T, U)> {\n    pub fn unzip(self) -> (Option<T>, Option<U>) {\n        match self {\n            Option::Some((left, right)) => (Option::Some(left), Option::Some(right)),\n            Option::None => (Option::None, Option::None),\n        }\n    }\n}\nimpl<T> Option<T> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => Option::Some(function(value)),\n            Option::None => Option::None,\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, function: F) -> U {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => default(),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, function: F) -> Option<U> {\n        match self {\n            Option::Some(value) => function(value),\n            Option::None => Option::None,\n        }\n    }\n    pub fn or_else<F: FnOnce() -> Option<T>>(self, function: F) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(value),\n            Option::None => function(),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce() -> T>(self, function: F) -> T {\n        match self {\n            Option::Some(value) => value,\n            Option::None => function(),\n        }\n    }\n    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {\n        match self {\n            Option::Some(value) => {\n                if predicate(&value) {\n                    Option::Some(value)\n                } else {\n                    Option::None\n                }\n            }\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<T, E> Result<T, E> {\n    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => Result::Ok(function(value)),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn map_err<U, F: FnOnce(E) -> U>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => Result::Err(function(error)),\n        }\n    }\n    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(_) => default,\n        }\n    }\n    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(\n        self,\n        default: D,\n        function: F,\n    ) -> U {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => default(error),\n        }\n    }\n    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, function: F) -> Result<U, E> {\n        match self {\n            Result::Ok(value) => function(value),\n            Result::Err(error) => Result::Err(error),\n        }\n    }\n    pub fn or_else<U, F: FnOnce(E) -> Result<T, U>>(self, function: F) -> Result<T, U> {\n        match self {\n            Result::Ok(value) => Result::Ok(value),\n            Result::Err(error) => function(error),\n        }\n    }\n    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, function: F) -> T {\n        match self {\n            Result::Ok(value) => value,\n            Result::Err(error) => function(error),\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Copy> Option<&'a mut T> {\n    pub fn copied(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some(*value),\n            Option::None => Option::None,\n        }\n    }\n}\n\nimpl<'a, T: Clone> Option<&'a mut T> {\n    pub fn cloned(self) -> Option<T> {\n        match self {\n            Option::Some(value) => Option::Some((*value).clone()),\n            Option::None => Option::None,\n        }\n    }\n}\n\n/// An ordinary source contract for a canonical initial value.\npub trait Default {\n    fn default() -> Self;\n}\n\nimpl Default for bool {\n    fn default() -> bool {\n        false\n    }\n}\n\nimpl Default for char {\n    fn default() -> char {\n        '\\0'\n    }\n}\n\nimpl Default for u8 {\n    fn default() -> u8 {\n        0u8\n    }\n}\n\nimpl Default for u16 {\n    fn default() -> u16 {\n        0u16\n    }\n}\n\nimpl Default for u32 {\n    fn default() -> u32 {\n        0u32\n    }\n}\n\nimpl Default for u64 {\n    fn default() -> u64 {\n        0u64\n    }\n}\n\nimpl Default for usize {\n    fn default() -> usize {\n        0usize\n    }\n}\n\nimpl Default for i8 {\n    fn default() -> i8 {\n        0i8\n    }\n}\n\nimpl Default for i16 {\n    fn default() -> i16 {\n        0i16\n    }\n}\n\nimpl Default for i32 {\n    fn default() -> i32 {\n        0i32\n    }\n}\n\nimpl Default for i64 {\n    fn default() -> i64 {\n        0i64\n    }\n}\n\nimpl Default for isize {\n    fn default() -> isize {\n        0isize\n    }\n}\n\nimpl Default for f64 {\n    fn default() -> f64 {\n        0.0\n    }\n}\n\nimpl<T> Default for Vec<T> {\n    fn default() -> Vec<T> {\n        Vec::new()\n    }\n}\nimpl<T> Default for Option<T> {\n    fn default() -> Option<T> {\n        Option::None\n    }\n}\nimpl Default for crate::string::String {\n    fn default() -> crate::string::String {\n        crate::string::String::new()\n    }\n}\n\n// Derive expansion resolves this declaration by standard-library identity.\npub fn __lkjscript_default<T: Default>() -> T {\n    T::default()\n}\n")),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("option_result_traits.rs", Some("pub fn probe()->i64{let value=Option::Some(42i64).clone();if value.is_some(){1}else{0}}")),
], "E_TYPE", "$std/prelude.rs", 0, 0), errors);
        count += 1;
    }
    if 469usize % workers == shard {
        record(source_case("the_same_recovering_parser_preserves_valid_compilation_0000", "case/main.rs", None, vec![
source_file("case/main.rs", Some("fn square(x: i64) -> i64 { x * x } fn probe() -> i64 { square(6) + 6 }")),
], ""), errors);
        count += 1;
    }
    if 470usize % workers == shard {
        record(source_case("trait_type_argument_requirements_retain_declared_identity_0000", "trait_identity.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("trait_identity.rs", Some("trait Clone{} trait Contract<T:crate::Clone>{}\n         mod nested{pub struct Holder<T>{value:T} impl<T:Clone> crate::Contract<T> for Holder<T>{}}\n         pub fn probe()->i64{0}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 471usize % workers == shard {
        record(source_case("trait_type_argument_requirements_retain_declared_identity_0001", "trait_identity.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("trait_identity.rs", Some("trait Clone{} trait Contract<T:crate::Clone>{}\n         mod nested{pub struct Holder<T>{value:T} impl<T:crate::Clone> crate::Contract<T> for Holder<T>{}}\n         pub fn probe()->i64{0}")),
], ""), errors);
        count += 1;
    }
    if 472usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{value:i64}} fn bad()->E{E::Named(1)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 473usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0001", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Tuple(i64)} fn bad()->E{E::Tuple{value:1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 474usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0002", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{value:i64}} fn bad(value:E)->i64{match value{E::Named(item)=>item}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 475usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0003", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Tuple(i64)} fn bad(value:E)->i64{match value{E::Tuple{value}=>value}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 476usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0004", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{}} fn bad()->E{E::Named}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 477usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0005", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{}} fn bad()->E{E::Named()}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 478usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0006", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Tuple()} fn bad()->E{E::Tuple}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 479usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0007", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Empty} fn bad()->E{E::Empty()}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 480usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0008", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Tuple()} fn bad(value:E)->i64{match value{E::Tuple=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 481usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0009", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Empty} fn bad(value:E)->i64{match value{E::Empty()=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 482usize % workers == shard {
        record(source_case("tuple_and_named_variant_syntax_are_distinct_0010", "main.rs", None, vec![
source_file("main.rs", Some("enum E{Named{}} fn bad(value:E)->i64{match value{E::Named=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 483usize % workers == shard {
        record(source_case("unconstrained_parameters_remain_affine_0001", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T>(x:T)->T {let moved=x; x}"))], "E_MOVE"), errors);
        count += 1;
    }
    if 484usize % workers == shard {
        record(source_case("unconstrained_parameters_remain_affine_0003", "case.rs", None, vec![source_file("case.rs", Some("fn identity<T>(x:T)->T{x}"))], ""), errors);
        count += 1;
    }
    if 485usize % workers == shard {
        record(source_case("unconstrained_parameters_remain_affine_0005", "case.rs", None, vec![source_file("case.rs", Some("fn duplicate<T:Copy>(x:T)->(T,T){(x,x)}"))], ""), errors);
        count += 1;
    }
    if 486usize % workers == shard {
        record(source_case("unimplemented_explicit_builtin_overrides_are_named_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Copy for Item{}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 487usize % workers == shard {
        record(source_case("unimplemented_explicit_builtin_overrides_are_named_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl PartialOrd for Item{}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 488usize % workers == shard {
        record(source_case("unimplemented_explicit_builtin_overrides_are_named_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Ord for Item{}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 489usize % workers == shard {
        record(source_case("unimplemented_explicit_builtin_overrides_are_named_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Data for Item{}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 490usize % workers == shard {
        record(source_case("unimplemented_explicit_builtin_overrides_are_named_0004", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Send for Item{}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 491usize % workers == shard {
        record(source_case("unimplemented_explicit_builtin_overrides_are_named_0005", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct Item{value:i64} impl Sync for Item{}")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 492usize % workers == shard {
        record(source_case("unreachable_generic_expressions_still_have_to_type_check_0000", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T>(x:T)->i64{return 0; 1i64+true}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 493usize % workers == shard {
        record(source_case("unrelated_traits_and_inherent_methods_do_not_grant_builtin_equality_0000", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("mod other{pub trait PartialEq{fn eq(&self,other:&Self)->bool;}} struct Value{} impl other::PartialEq for Value{fn eq(&self,other:&Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 494usize % workers == shard {
        record(source_case("unrelated_traits_and_inherent_methods_do_not_grant_builtin_equality_0001", "partial_eq.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("partial_eq.rs", Some("struct Value{} impl Value{fn eq(&self,other:&Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 495usize % workers == shard {
        record(source_case("unsized_source_locals_keep_their_module_and_construction_location_0000", "case/main.rs", None, vec![
source_file("case/main.rs", Some("mod nested;")),
source_file("case/nested.rs", Some("\nfn consume(text: &str) {\n    let value = *text;\n}\n")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 496usize % workers == shard {
        record(source_case("unsized_source_locals_keep_their_module_and_construction_location_0001", "case/main.rs", None, vec![
source_file("case/main.rs", Some("mod nested;")),
source_file("case/nested.rs", Some("\n\nfn consume(value: str) {}\n")),
], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 497usize % workers == shard {
        record(source_case("unused_clone_requires_every_concrete_field_0000", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; #[derive(Clone)] struct Item {value:NoClone} pub fn probe()->i64{0}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 498usize % workers == shard {
        record(source_case("unused_clone_requires_every_concrete_field_0001", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; #[derive(Clone)] enum Item {Empty, Value(NoClone)}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 499usize % workers == shard {
        record(source_case("unused_clone_requires_every_concrete_field_0002", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; type Alias=NoClone; #[derive(Clone)] struct Item(Alias);")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 500usize % workers == shard {
        record(source_case("unused_clone_requires_every_concrete_field_0003", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; #[derive(Clone)] struct Item {value:Vec<Box<NoClone>>}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 501usize % workers == shard {
        record(source_case("unused_clone_requires_every_concrete_field_0004", "derive_case.rs", None, vec![
source_file("derive_case.rs", Some("struct NoClone; #[derive(Clone)] struct Item<T> {value:T, bad:NoClone}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 502usize % workers == shard {
        record(source_case("unused_generic_arithmetic_is_checked_0001", "case.rs", None, vec![source_file("case.rs", Some("fn bad<T>(x:T)->i64 {1i64 + true} fn probe()->i64{0}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 503usize % workers == shard {
        record(source_case("unused_generic_bodies_check_range_endpoints_and_dead_matches_0000", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad<T>(value:T)->i64{match 0u8{0..=256=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 504usize % workers == shard {
        record(source_case("unused_generic_bodies_check_range_endpoints_and_dead_matches_0001", "range_probe.rs", None, vec![
source_file("range_probe.rs", Some("fn bad<T>(value:T)->i64{return 0;match 0u8{0..=256=>1,_=>0}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 505usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0000", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn missing<T>(value:&T)->T{value.clone()}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 506usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0001", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn missing<T>(left:&T,right:&T)->bool{left.eq(right)}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 507usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0002", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn missing<T:Eq>(value:&T)->T{value.clone()}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 508usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0003", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn wrong<T:Clone>(value:&T)->bool{value.clone()}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 509usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0004", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn wrong<T:Clone>(value:&T)->T{value.clone(1i64)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 510usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0005", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn wrong<T:Clone>(value:&T)->T{value.clone::<i64>()}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 511usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0006", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn wrong<T:Clone>(target:&mut T,source:&T)->T{target.clone_from(source)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 512usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0007", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn wrong<T:PartialEq>(value:&T)->bool{value.eq()}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 513usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0008", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn wrong<T:PartialEq>(left:&T,right:&i64)->bool{left.eq(right)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 514usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0009", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn wrong<T:PartialEq>(left:&T,right:&T)->T{left.eq(right)}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 515usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0010", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn twice<T:Clone>(value:&T)->(T,T){let copied=value.clone();(copied,copied)}")),
], "E_MOVE"), errors);
        count += 1;
    }
    if 516usize % workers == shard {
        record(source_case("unused_generic_builtin_methods_enforce_bounds_types_and_ownership_0011", "string_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("string_traits_probe.rs", Some("fn overlapping<T:Clone>(value:&mut T){value.clone_from(value);}")),
], "E_BORROW"), errors);
        count += 1;
    }
    if 517usize % workers == shard {
        record(source_case("unused_generic_named_constructors_are_checked_0000", "main.rs", None, vec![
source_file("main.rs", Some("enum E<T>{Named{value:T,count:i64}} fn bad<T>(value:T)->E<T>{E::Named{value,count:true}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 518usize % workers == shard {
        record(source_case("unused_generic_named_constructors_are_checked_0001", "main.rs", None, vec![
source_file("main.rs", Some("enum E<T>{Named{value:T,count:i64}} fn bad<T>(value:T)->i64{let item=E::Named{value,count:1};match item{_=>0,E::Named{unknown,..}=>1}}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 519usize % workers == shard {
        record(source_case("unused_trait_default_bodies_are_checked_0000", "case.rs", None, vec![source_file("case.rs", Some("trait Score{fn score(&self)->i64{true}}"))], "E_TYPE"), errors);
        count += 1;
    }
    if 520usize % workers == shard {
        record(source_case("user_names_do_not_change_source_library_generic_method_selection_0000", "scope.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("scope.rs", Some("enum E{Nothing} pub fn probe()->usize{let value=\"scope\".to_string();value.as_str().len()}")),
], ""), errors);
        count += 1;
    }
    if 521usize % workers == shard {
        record(source_case("user_names_do_not_change_source_library_generic_method_selection_0002", "scope.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("scope.rs", Some("enum T{Nothing} pub fn probe()->usize{let value=\"scope\".to_string();value.as_str().len()}")),
], ""), errors);
        count += 1;
    }
    if 522usize % workers == shard {
        record(source_case("user_names_do_not_change_source_library_generic_method_selection_0004", "scope.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("scope.rs", Some("enum U{Nothing} pub fn probe()->usize{let value=\"scope\".to_string();value.as_str().len()}")),
], ""), errors);
        count += 1;
    }
    if 523usize % workers == shard {
        record(source_case("user_names_do_not_change_source_library_generic_method_selection_0006", "scope.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("scope.rs", Some("enum Packet{Nothing} pub fn probe()->usize{let value=\"scope\".to_string();value.as_str().len()}")),
], ""), errors);
        count += 1;
    }
    if 524usize % workers == shard {
        record(source_case("user_traits_do_not_satisfy_same_spelled_builtin_predicates_0001", "case.rs", None, vec![source_file("case.rs", Some("mod builtin{pub fn need<T:Copy>(x:T){}} trait Copy{} fn bad<U:Copy>(x:U){builtin::need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 525usize % workers == shard {
        record(source_case("user_traits_do_not_satisfy_same_spelled_builtin_predicates_0003", "case.rs", None, vec![source_file("case.rs", Some("mod builtin{pub fn need<T:Data>(x:T){}} trait Data{} fn bad<U:Data>(x:U){builtin::need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 526usize % workers == shard {
        record(source_case("user_traits_do_not_satisfy_same_spelled_builtin_predicates_0004", "case.rs", None, vec![source_file("case.rs", Some("mod builtin{pub fn need<T:Send>(x:T){}} trait Send{} fn bad<U:Send>(x:U){builtin::need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 527usize % workers == shard {
        record(source_case("user_traits_do_not_satisfy_same_spelled_builtin_predicates_0005", "case.rs", None, vec![source_file("case.rs", Some("mod builtin{pub fn need<T:Sync>(x:T){}} trait Sync{} fn bad<U:Sync>(x:U){builtin::need(x);}"))], "E_TRAIT"), errors);
        count += 1;
    }
    if 528usize % workers == shard {
        record(source_case("vector_equality_uses_eq_even_when_an_element_ne_override_disagrees_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("use std::{__lkjscript_vec_equal};\n\nstruct Item { value:Box<i64> }\nimpl PartialEq for Item {\n    fn eq(&self,other:&Self)->bool { *self.value==*other.value }\n    #[allow(clippy::partialeq_ne_impl)]\n    fn ne(&self,_other:&Self)->bool { false }\n}\nfn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\npub fn exact_helper()->i64 {\n    let left:Vec<Item>=vec![Item{value:Box::new(1)}];\n    let right:Vec<Item>=vec![Item{value:Box::new(2)}];\n    if __lkjscript_vec_equal(&left,&right) {1} else {0}\n}\npub fn vector_rule()->i64 {\n    let left:Vec<Item>=vec![Item{value:Box::new(1)}];\n    let right:Vec<Item>=vec![Item{value:Box::new(2)}];\n    if left==right || !(left!=right) {return 1;}\n    if left.eq(&right) || !left.ne(&right) {return 2;}\n    if same(&left,&right) || !different(&left,&right) {return 3;}\n    0\n}\n")),
], ""), errors);
        count += 1;
    }
    if 529usize % workers == shard {
        record(source_case("vector_helpers_have_source_library_identity_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("\nfn __lkjscript_vec_clone<T:Clone>(_value:&Vec<T>)->Vec<T>{Vec::new()}\nfn __lkjscript_vec_equal<T:PartialEq>(_left:&Vec<T>,_right:&Vec<T>)->bool{false}\nmod std {\n    pub fn __lkjscript_vec_clone<T:Clone>(_value:&Vec<T>)->Vec<T>{Vec::new()}\n    pub fn __lkjscript_vec_equal<T:PartialEq>(_left:&Vec<T>,_right:&Vec<T>)->bool{false}\n}\npub fn probe()->i64 {\n    let values:Vec<i64>=vec![4,2];let copied=values.clone();\n    if copied==values {copied[0]*10+copied[1]} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 530usize % workers == shard {
        record(source_case("vector_helpers_have_source_library_identity_0004", "case.rs", None, vec![source_file("case.rs", Some("\nfn __lkjscript_vec_clone<T:Clone>(_value:&Vec<T>)->Vec<T>{Vec::new()}\nfn __lkjscript_vec_equal<T:PartialEq>(_left:&Vec<T>,_right:&Vec<T>)->bool{false}\nmod std {\n    pub fn __lkjscript_vec_clone<T:Clone>(_value:&Vec<T>)->Vec<T>{Vec::new()}\n    pub fn __lkjscript_vec_equal<T:PartialEq>(_left:&Vec<T>,_right:&Vec<T>)->bool{false}\n}\npub fn probe()->i64 {\n    let values:Vec<i64>=vec![4,2];let copied=values.clone();\n    if copied==values {copied[0]*10+copied[1]} else {0}\n}\n"))], "E_UNSUPPORTED"), errors);
        count += 1;
    }
    if 531usize % workers == shard {
        record(source_case("verification_types_and_functions_do_not_enter_a_generic_string_artifact_0000", "builtin_contracts.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("builtin_contracts.rs", Some("\nfn copied<T:Clone>(value:&T)->T{value.clone()}\nfn scalar_copy<T:Copy>(value:&T)->T{value.clone()}\nfn update<T:Clone>(target:&mut T,source:&T){target.clone_from(source);}\nfn same<T:Eq>(left:&T,right:&T)->bool{left.eq(right)}\nfn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}\npub fn probe()->i64 {\n    let original=String::from_str(\"hé😀\");\n    let mut result=copied(&original);\n    let other=String::from_str(\"other\");\n    update(&mut result,&other);\n    if same(&result,&other) && different(&original,&result) {scalar_copy(&42i64)} else {0}\n}\n")),
], ""), errors);
        count += 1;
    }
    if 532usize % workers == shard {
        record(source_case("weakened_library_helpers_cannot_grant_missing_element_contracts_0000", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", Some("//! Vec operations are ordinary source over checked element access.\nuse crate::prelude::Option;\npub fn __lkjscript_vec_clone<T>(_source:&Vec<T>)->Vec<T>{Vec::new()}\npub fn __lkjscript_vec_equal<T: PartialEq>(left: &Vec<T>, right: &Vec<T>) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index: usize = 0;\n    while index < left.len() {\n        // Equality uses the element's eq contract; ne may be independently overridden.\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index = index + 1;\n    }\n    true\n}\n\n// Clone the first count-1 elements and move the original into the last slot.\n// The element expression is evaluated even when count is zero.\npub fn __lkjscript_vec_repeat<T: Clone>(value: T, count: usize) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    if count == 0 {\n        return result;\n    }\n    let mut remaining = count - 1;\n    while remaining > 0 {\n        result.push(value.clone());\n        remaining = remaining - 1;\n    }\n    result.push(value);\n    result\n}\n\n/// Shared inspection does not require an element equality or copying contract.\npub trait VecAccess<T> {\n    fn is_empty(&self) -> bool;\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T>;\n    fn first<'a>(&'a self) -> Option<&'a T>;\n    fn last<'a>(&'a self) -> Option<&'a T>;\n}\n\nimpl<T> VecAccess<T> for Vec<T> {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\n\n/// Equality search borrows its elements and stops at the first match.\npub trait VecContains<T: PartialEq> {\n    fn contains(&self, value: &T) -> bool;\n}\n\nimpl<T: PartialEq> VecContains<T> for Vec<T> {\n    fn contains(&self, value: &T) -> bool {\n        let mut index: usize = 0;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index = index + 1;\n        }\n        false\n    }\n}\n\n/// Append independent clones in source order; the shared source remains usable.\npub trait VecExtend<T: Clone> {\n    fn extend_from_slice(&mut self, values: &[T]);\n}\n\nimpl<T: Clone> VecExtend<T> for Vec<T> {\n    fn extend_from_slice(&mut self, values: &[T]) {\n        let mut index: usize = 0;\n        while index < values.len() {\n            self.push(values[index].clone());\n            index = index + 1;\n        }\n    }\n}\n\n/// Mutation visits values in their original order and keeps affine ownership.\npub trait VecMutate<T> {\n    fn retain<F: FnMut(&T) -> bool>(&mut self, predicate: F);\n}\nimpl<T> VecMutate<T> for Vec<T> {\n    // Explicit replacement uses the declared primitive; mem::take is not provided.\n    #[allow(clippy::mem_replace_with_default)]\n    fn retain<F: FnMut(&T) -> bool>(&mut self, mut predicate: F) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            if predicate(&value) {\n                self.push(value);\n            }\n        }\n    }\n}\n\n/// Stable merge sorting requires only checked indexing and owned Option slots.\n/// Each pass is linear; the number of passes grows logarithmically with length.\npub trait VecSort<T: Ord> {\n    fn sort(&mut self);\n}\nimpl<T: Ord> VecSort<T> for Vec<T> {\n    #[allow(clippy::mem_replace_with_default)]\n    fn sort(&mut self) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        let mut values: Vec<Option<T>> = Vec::new();\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            values.push(Option::Some(value));\n        }\n        let count = values.len();\n        let mut width = 1usize;\n        while width < count {\n            let mut merged: Vec<Option<T>> = Vec::new();\n            let mut start = 0usize;\n            while start < count {\n                let left_count = if width < count - start {\n                    width\n                } else {\n                    count - start\n                };\n                let middle = start + left_count;\n                let right_count = if width < count - middle {\n                    width\n                } else {\n                    count - middle\n                };\n                let end = middle + right_count;\n                let mut left = start;\n                let mut right = middle;\n                while left < middle || right < end {\n                    let use_left = if left == middle {\n                        false\n                    } else if right == end {\n                        true\n                    } else {\n                        *values[left].as_ref().unwrap() <= *values[right].as_ref().unwrap()\n                    };\n                    if use_left {\n                        merged.push(values[left].take());\n                        left = left + 1;\n                    } else {\n                        merged.push(values[right].take());\n                        right = right + 1;\n                    }\n                }\n                start = end;\n            }\n            values = merged;\n            width = if width > count / 2 { count } else { width * 2 };\n        }\n        let mut index = 0usize;\n        while index < count {\n            self.push(values[index].take().unwrap());\n            index = index + 1;\n        }\n    }\n}\n\n/// Slice inspection has the same borrowing contract as vector inspection.\nimpl<T> VecAccess<T> for [T] {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\nimpl<T: PartialEq> VecContains<T> for [T] {\n    fn contains(&self, value: &T) -> bool {\n        let mut index = 0usize;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index += 1;\n        }\n        false\n    }\n}\n\npub trait VecAccessMut<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T>;\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T>;\n}\nimpl<T> VecAccessMut<T> for Vec<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\nimpl<T> VecAccessMut<T> for [T] {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\n\npub trait SliceClone<T: Clone> {\n    fn to_vec(&self) -> Vec<T>;\n}\nimpl<T: Clone> SliceClone<T> for [T] {\n    fn to_vec(&self) -> Vec<T> {\n        let mut result: Vec<T> = Vec::new();\n        let mut index = 0usize;\n        while index < self.len() {\n            result.push(self[index].clone());\n            index += 1;\n        }\n        result\n    }\n}\n\npub trait SequencePrefix<T: PartialEq> {\n    fn starts_with(&self, prefix: &[T]) -> bool;\n}\nimpl<T: PartialEq> SequencePrefix<T> for [T] {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\nimpl<T: PartialEq> SequencePrefix<T> for Vec<T> {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\n\npub trait SliceCopy<T: Copy> {\n    fn copy_from_slice(&mut self, source: &[T]);\n}\nimpl<T: Copy> SliceCopy<T> for [T] {\n    fn copy_from_slice(&mut self, source: &[T]) {\n        assert!(self.len() == source.len());\n        let mut index = 0usize;\n        while index < self.len() {\n            self[index] = source[index];\n            index += 1;\n        }\n    }\n}\n\npub trait VecClear<T> {\n    fn clear(&mut self);\n    fn truncate(&mut self, length: usize);\n}\nimpl<T> VecClear<T> for Vec<T> {\n    fn clear(&mut self) {\n        self.retain(|_| false);\n    }\n    fn truncate(&mut self, length: usize) {\n        if length >= self.len() {\n            return;\n        }\n        let mut index = 0usize;\n        self.retain(|_| {\n            let keep = index < length;\n            index += 1;\n            keep\n        });\n    }\n}\n\n/// A four-way lexicographic result: less, equal, greater, or unordered.\n/// Relational operators are used directly so custom equality does not affect order.\n#[allow(clippy::neg_cmp_op_on_partial_ord)]\npub fn __lkjscript_slice_order<T: PartialOrd>(left: &[T], right: &[T]) -> i32 {\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1i32;\n        }\n        if left[index] > right[index] {\n            return 1i32;\n        }\n        if !(left[index] <= right[index]) {\n            return 2i32;\n        }\n        index += 1;\n    }\n    if left.len() < right.len() {\n        -1i32\n    } else if left.len() > right.len() {\n        1i32\n    } else {\n        0i32\n    }\n}\n\npub fn __lkjscript_vec_order<T: PartialOrd>(left: &Vec<T>, right: &Vec<T>) -> i32 {\n    __lkjscript_slice_order(&left[..], &right[..])\n}\n\npub fn __lkjscript_slice_equal<T: PartialEq>(left: &[T], right: &[T]) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index = 0usize;\n    while index < left.len() {\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index += 1;\n    }\n    true\n}\n\n/// Split borrows the original elements and evaluates the predicate once per element.\npub struct SliceSplit<'a, T, F> {\n    items: &'a [T],\n    predicate: F,\n    start: usize,\n    done: bool,\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::Iterator for SliceSplit<'a, T, F> {\n    type Item = &'a [T];\n    fn next(&mut self) -> Option<&'a [T]> {\n        if self.done {\n            return Option::None;\n        }\n        let items = self.items;\n        let start = self.start;\n        let mut end = start;\n        while end < items.len() {\n            if (self.predicate)(&items[end]) {\n                self.start = end + 1;\n                return Option::Some(&items[start..end]);\n            }\n            end += 1;\n        }\n        self.done = true;\n        Option::Some(&items[start..])\n    }\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::IntoIterator for SliceSplit<'a, T, F> {\n    type IntoIter = Self;\n    fn into_iter(self) -> Self {\n        self\n    }\n}\npub trait SliceSplitSource<T> {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F>;\n}\nimpl<T> SliceSplitSource<T> for [T] {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F> {\n        SliceSplit {\n            items: self,\n            predicate,\n            start: 0,\n            done: false,\n        }\n    }\n}\n")),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("struct Item;fn probe(){let values:Vec<Item>=Vec::new();values.clone();}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 533usize % workers == shard {
        record(source_case("weakened_library_helpers_cannot_grant_missing_element_contracts_0001", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", Some("//! Vec operations are ordinary source over checked element access.\nuse crate::prelude::Option;\npub fn __lkjscript_vec_clone<T: Clone>(source: &Vec<T>) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    let mut index: usize = 0;\n    while index < source.len() {\n        result.push(source[index].clone());\n        index = index + 1;\n    }\n    result\n}\n\npub fn __lkjscript_vec_equal<T>(_left:&Vec<T>,_right:&Vec<T>)->bool{true}\npub fn __lkjscript_vec_repeat<T: Clone>(value: T, count: usize) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    if count == 0 {\n        return result;\n    }\n    let mut remaining = count - 1;\n    while remaining > 0 {\n        result.push(value.clone());\n        remaining = remaining - 1;\n    }\n    result.push(value);\n    result\n}\n\n/// Shared inspection does not require an element equality or copying contract.\npub trait VecAccess<T> {\n    fn is_empty(&self) -> bool;\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T>;\n    fn first<'a>(&'a self) -> Option<&'a T>;\n    fn last<'a>(&'a self) -> Option<&'a T>;\n}\n\nimpl<T> VecAccess<T> for Vec<T> {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\n\n/// Equality search borrows its elements and stops at the first match.\npub trait VecContains<T: PartialEq> {\n    fn contains(&self, value: &T) -> bool;\n}\n\nimpl<T: PartialEq> VecContains<T> for Vec<T> {\n    fn contains(&self, value: &T) -> bool {\n        let mut index: usize = 0;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index = index + 1;\n        }\n        false\n    }\n}\n\n/// Append independent clones in source order; the shared source remains usable.\npub trait VecExtend<T: Clone> {\n    fn extend_from_slice(&mut self, values: &[T]);\n}\n\nimpl<T: Clone> VecExtend<T> for Vec<T> {\n    fn extend_from_slice(&mut self, values: &[T]) {\n        let mut index: usize = 0;\n        while index < values.len() {\n            self.push(values[index].clone());\n            index = index + 1;\n        }\n    }\n}\n\n/// Mutation visits values in their original order and keeps affine ownership.\npub trait VecMutate<T> {\n    fn retain<F: FnMut(&T) -> bool>(&mut self, predicate: F);\n}\nimpl<T> VecMutate<T> for Vec<T> {\n    // Explicit replacement uses the declared primitive; mem::take is not provided.\n    #[allow(clippy::mem_replace_with_default)]\n    fn retain<F: FnMut(&T) -> bool>(&mut self, mut predicate: F) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            if predicate(&value) {\n                self.push(value);\n            }\n        }\n    }\n}\n\n/// Stable merge sorting requires only checked indexing and owned Option slots.\n/// Each pass is linear; the number of passes grows logarithmically with length.\npub trait VecSort<T: Ord> {\n    fn sort(&mut self);\n}\nimpl<T: Ord> VecSort<T> for Vec<T> {\n    #[allow(clippy::mem_replace_with_default)]\n    fn sort(&mut self) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        let mut values: Vec<Option<T>> = Vec::new();\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            values.push(Option::Some(value));\n        }\n        let count = values.len();\n        let mut width = 1usize;\n        while width < count {\n            let mut merged: Vec<Option<T>> = Vec::new();\n            let mut start = 0usize;\n            while start < count {\n                let left_count = if width < count - start {\n                    width\n                } else {\n                    count - start\n                };\n                let middle = start + left_count;\n                let right_count = if width < count - middle {\n                    width\n                } else {\n                    count - middle\n                };\n                let end = middle + right_count;\n                let mut left = start;\n                let mut right = middle;\n                while left < middle || right < end {\n                    let use_left = if left == middle {\n                        false\n                    } else if right == end {\n                        true\n                    } else {\n                        *values[left].as_ref().unwrap() <= *values[right].as_ref().unwrap()\n                    };\n                    if use_left {\n                        merged.push(values[left].take());\n                        left = left + 1;\n                    } else {\n                        merged.push(values[right].take());\n                        right = right + 1;\n                    }\n                }\n                start = end;\n            }\n            values = merged;\n            width = if width > count / 2 { count } else { width * 2 };\n        }\n        let mut index = 0usize;\n        while index < count {\n            self.push(values[index].take().unwrap());\n            index = index + 1;\n        }\n    }\n}\n\n/// Slice inspection has the same borrowing contract as vector inspection.\nimpl<T> VecAccess<T> for [T] {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\nimpl<T: PartialEq> VecContains<T> for [T] {\n    fn contains(&self, value: &T) -> bool {\n        let mut index = 0usize;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index += 1;\n        }\n        false\n    }\n}\n\npub trait VecAccessMut<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T>;\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T>;\n}\nimpl<T> VecAccessMut<T> for Vec<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\nimpl<T> VecAccessMut<T> for [T] {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\n\npub trait SliceClone<T: Clone> {\n    fn to_vec(&self) -> Vec<T>;\n}\nimpl<T: Clone> SliceClone<T> for [T] {\n    fn to_vec(&self) -> Vec<T> {\n        let mut result: Vec<T> = Vec::new();\n        let mut index = 0usize;\n        while index < self.len() {\n            result.push(self[index].clone());\n            index += 1;\n        }\n        result\n    }\n}\n\npub trait SequencePrefix<T: PartialEq> {\n    fn starts_with(&self, prefix: &[T]) -> bool;\n}\nimpl<T: PartialEq> SequencePrefix<T> for [T] {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\nimpl<T: PartialEq> SequencePrefix<T> for Vec<T> {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\n\npub trait SliceCopy<T: Copy> {\n    fn copy_from_slice(&mut self, source: &[T]);\n}\nimpl<T: Copy> SliceCopy<T> for [T] {\n    fn copy_from_slice(&mut self, source: &[T]) {\n        assert!(self.len() == source.len());\n        let mut index = 0usize;\n        while index < self.len() {\n            self[index] = source[index];\n            index += 1;\n        }\n    }\n}\n\npub trait VecClear<T> {\n    fn clear(&mut self);\n    fn truncate(&mut self, length: usize);\n}\nimpl<T> VecClear<T> for Vec<T> {\n    fn clear(&mut self) {\n        self.retain(|_| false);\n    }\n    fn truncate(&mut self, length: usize) {\n        if length >= self.len() {\n            return;\n        }\n        let mut index = 0usize;\n        self.retain(|_| {\n            let keep = index < length;\n            index += 1;\n            keep\n        });\n    }\n}\n\n/// A four-way lexicographic result: less, equal, greater, or unordered.\n/// Relational operators are used directly so custom equality does not affect order.\n#[allow(clippy::neg_cmp_op_on_partial_ord)]\npub fn __lkjscript_slice_order<T: PartialOrd>(left: &[T], right: &[T]) -> i32 {\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1i32;\n        }\n        if left[index] > right[index] {\n            return 1i32;\n        }\n        if !(left[index] <= right[index]) {\n            return 2i32;\n        }\n        index += 1;\n    }\n    if left.len() < right.len() {\n        -1i32\n    } else if left.len() > right.len() {\n        1i32\n    } else {\n        0i32\n    }\n}\n\npub fn __lkjscript_vec_order<T: PartialOrd>(left: &Vec<T>, right: &Vec<T>) -> i32 {\n    __lkjscript_slice_order(&left[..], &right[..])\n}\n\npub fn __lkjscript_slice_equal<T: PartialEq>(left: &[T], right: &[T]) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index = 0usize;\n    while index < left.len() {\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index += 1;\n    }\n    true\n}\n\n/// Split borrows the original elements and evaluates the predicate once per element.\npub struct SliceSplit<'a, T, F> {\n    items: &'a [T],\n    predicate: F,\n    start: usize,\n    done: bool,\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::Iterator for SliceSplit<'a, T, F> {\n    type Item = &'a [T];\n    fn next(&mut self) -> Option<&'a [T]> {\n        if self.done {\n            return Option::None;\n        }\n        let items = self.items;\n        let start = self.start;\n        let mut end = start;\n        while end < items.len() {\n            if (self.predicate)(&items[end]) {\n                self.start = end + 1;\n                return Option::Some(&items[start..end]);\n            }\n            end += 1;\n        }\n        self.done = true;\n        Option::Some(&items[start..])\n    }\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::IntoIterator for SliceSplit<'a, T, F> {\n    type IntoIter = Self;\n    fn into_iter(self) -> Self {\n        self\n    }\n}\npub trait SliceSplitSource<T> {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F>;\n}\nimpl<T> SliceSplitSource<T> for [T] {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F> {\n        SliceSplit {\n            items: self,\n            predicate,\n            start: 0,\n            done: false,\n        }\n    }\n}\n")),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("struct Item;fn probe()->bool{let values:Vec<Item>=Vec::new();values==values}")),
], "E_TRAIT"), errors);
        count += 1;
    }
    if 534usize % workers == shard {
        record(source_case("weakened_library_helpers_cannot_grant_missing_element_contracts_0002", "vector_traits_probe.rs", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", Some("//! Vec operations are ordinary source over checked element access.\nuse crate::prelude::Option;\npub fn __lkjscript_vec_clone<T:Clone>(_source:&Vec<T>)->bool{true}\npub fn __lkjscript_vec_equal<T: PartialEq>(left: &Vec<T>, right: &Vec<T>) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index: usize = 0;\n    while index < left.len() {\n        // Equality uses the element's eq contract; ne may be independently overridden.\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index = index + 1;\n    }\n    true\n}\n\n// Clone the first count-1 elements and move the original into the last slot.\n// The element expression is evaluated even when count is zero.\npub fn __lkjscript_vec_repeat<T: Clone>(value: T, count: usize) -> Vec<T> {\n    let mut result: Vec<T> = Vec::new();\n    if count == 0 {\n        return result;\n    }\n    let mut remaining = count - 1;\n    while remaining > 0 {\n        result.push(value.clone());\n        remaining = remaining - 1;\n    }\n    result.push(value);\n    result\n}\n\n/// Shared inspection does not require an element equality or copying contract.\npub trait VecAccess<T> {\n    fn is_empty(&self) -> bool;\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T>;\n    fn first<'a>(&'a self) -> Option<&'a T>;\n    fn last<'a>(&'a self) -> Option<&'a T>;\n}\n\nimpl<T> VecAccess<T> for Vec<T> {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\n\n/// Equality search borrows its elements and stops at the first match.\npub trait VecContains<T: PartialEq> {\n    fn contains(&self, value: &T) -> bool;\n}\n\nimpl<T: PartialEq> VecContains<T> for Vec<T> {\n    fn contains(&self, value: &T) -> bool {\n        let mut index: usize = 0;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index = index + 1;\n        }\n        false\n    }\n}\n\n/// Append independent clones in source order; the shared source remains usable.\npub trait VecExtend<T: Clone> {\n    fn extend_from_slice(&mut self, values: &[T]);\n}\n\nimpl<T: Clone> VecExtend<T> for Vec<T> {\n    fn extend_from_slice(&mut self, values: &[T]) {\n        let mut index: usize = 0;\n        while index < values.len() {\n            self.push(values[index].clone());\n            index = index + 1;\n        }\n    }\n}\n\n/// Mutation visits values in their original order and keeps affine ownership.\npub trait VecMutate<T> {\n    fn retain<F: FnMut(&T) -> bool>(&mut self, predicate: F);\n}\nimpl<T> VecMutate<T> for Vec<T> {\n    // Explicit replacement uses the declared primitive; mem::take is not provided.\n    #[allow(clippy::mem_replace_with_default)]\n    fn retain<F: FnMut(&T) -> bool>(&mut self, mut predicate: F) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            if predicate(&value) {\n                self.push(value);\n            }\n        }\n    }\n}\n\n/// Stable merge sorting requires only checked indexing and owned Option slots.\n/// Each pass is linear; the number of passes grows logarithmically with length.\npub trait VecSort<T: Ord> {\n    fn sort(&mut self);\n}\nimpl<T: Ord> VecSort<T> for Vec<T> {\n    #[allow(clippy::mem_replace_with_default)]\n    fn sort(&mut self) {\n        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));\n        let mut values: Vec<Option<T>> = Vec::new();\n        while let Option::Some(value) = crate::iter::__lkjscript_next(&mut remaining) {\n            values.push(Option::Some(value));\n        }\n        let count = values.len();\n        let mut width = 1usize;\n        while width < count {\n            let mut merged: Vec<Option<T>> = Vec::new();\n            let mut start = 0usize;\n            while start < count {\n                let left_count = if width < count - start {\n                    width\n                } else {\n                    count - start\n                };\n                let middle = start + left_count;\n                let right_count = if width < count - middle {\n                    width\n                } else {\n                    count - middle\n                };\n                let end = middle + right_count;\n                let mut left = start;\n                let mut right = middle;\n                while left < middle || right < end {\n                    let use_left = if left == middle {\n                        false\n                    } else if right == end {\n                        true\n                    } else {\n                        *values[left].as_ref().unwrap() <= *values[right].as_ref().unwrap()\n                    };\n                    if use_left {\n                        merged.push(values[left].take());\n                        left = left + 1;\n                    } else {\n                        merged.push(values[right].take());\n                        right = right + 1;\n                    }\n                }\n                start = end;\n            }\n            values = merged;\n            width = if width > count / 2 { count } else { width * 2 };\n        }\n        let mut index = 0usize;\n        while index < count {\n            self.push(values[index].take().unwrap());\n            index = index + 1;\n        }\n    }\n}\n\n/// Slice inspection has the same borrowing contract as vector inspection.\nimpl<T> VecAccess<T> for [T] {\n    fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {\n        if index < self.len() {\n            Option::Some(&self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn first<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[0usize])\n        }\n    }\n    fn last<'a>(&'a self) -> Option<&'a T> {\n        if self.len() == 0 {\n            Option::None\n        } else {\n            Option::Some(&self[self.len() - 1])\n        }\n    }\n}\nimpl<T: PartialEq> VecContains<T> for [T] {\n    fn contains(&self, value: &T) -> bool {\n        let mut index = 0usize;\n        while index < self.len() {\n            if self[index] == *value {\n                return true;\n            }\n            index += 1;\n        }\n        false\n    }\n}\n\npub trait VecAccessMut<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T>;\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T>;\n}\nimpl<T> VecAccessMut<T> for Vec<T> {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\nimpl<T> VecAccessMut<T> for [T] {\n    fn get_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut T> {\n        if index < self.len() {\n            Option::Some(&mut self[index])\n        } else {\n            Option::None\n        }\n    }\n    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T> {\n        let len = self.len();\n        if len == 0 {\n            Option::None\n        } else {\n            Option::Some(&mut self[len - 1])\n        }\n    }\n}\n\npub trait SliceClone<T: Clone> {\n    fn to_vec(&self) -> Vec<T>;\n}\nimpl<T: Clone> SliceClone<T> for [T] {\n    fn to_vec(&self) -> Vec<T> {\n        let mut result: Vec<T> = Vec::new();\n        let mut index = 0usize;\n        while index < self.len() {\n            result.push(self[index].clone());\n            index += 1;\n        }\n        result\n    }\n}\n\npub trait SequencePrefix<T: PartialEq> {\n    fn starts_with(&self, prefix: &[T]) -> bool;\n}\nimpl<T: PartialEq> SequencePrefix<T> for [T] {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\nimpl<T: PartialEq> SequencePrefix<T> for Vec<T> {\n    fn starts_with(&self, prefix: &[T]) -> bool {\n        if prefix.len() > self.len() {\n            return false;\n        }\n        let mut index = 0usize;\n        while index < prefix.len() {\n            if !(self[index] == prefix[index]) {\n                return false;\n            }\n            index += 1;\n        }\n        true\n    }\n}\n\npub trait SliceCopy<T: Copy> {\n    fn copy_from_slice(&mut self, source: &[T]);\n}\nimpl<T: Copy> SliceCopy<T> for [T] {\n    fn copy_from_slice(&mut self, source: &[T]) {\n        assert!(self.len() == source.len());\n        let mut index = 0usize;\n        while index < self.len() {\n            self[index] = source[index];\n            index += 1;\n        }\n    }\n}\n\npub trait VecClear<T> {\n    fn clear(&mut self);\n    fn truncate(&mut self, length: usize);\n}\nimpl<T> VecClear<T> for Vec<T> {\n    fn clear(&mut self) {\n        self.retain(|_| false);\n    }\n    fn truncate(&mut self, length: usize) {\n        if length >= self.len() {\n            return;\n        }\n        let mut index = 0usize;\n        self.retain(|_| {\n            let keep = index < length;\n            index += 1;\n            keep\n        });\n    }\n}\n\n/// A four-way lexicographic result: less, equal, greater, or unordered.\n/// Relational operators are used directly so custom equality does not affect order.\n#[allow(clippy::neg_cmp_op_on_partial_ord)]\npub fn __lkjscript_slice_order<T: PartialOrd>(left: &[T], right: &[T]) -> i32 {\n    let mut index = 0usize;\n    while index < left.len() && index < right.len() {\n        if left[index] < right[index] {\n            return -1i32;\n        }\n        if left[index] > right[index] {\n            return 1i32;\n        }\n        if !(left[index] <= right[index]) {\n            return 2i32;\n        }\n        index += 1;\n    }\n    if left.len() < right.len() {\n        -1i32\n    } else if left.len() > right.len() {\n        1i32\n    } else {\n        0i32\n    }\n}\n\npub fn __lkjscript_vec_order<T: PartialOrd>(left: &Vec<T>, right: &Vec<T>) -> i32 {\n    __lkjscript_slice_order(&left[..], &right[..])\n}\n\npub fn __lkjscript_slice_equal<T: PartialEq>(left: &[T], right: &[T]) -> bool {\n    if left.len() != right.len() {\n        return false;\n    }\n    let mut index = 0usize;\n    while index < left.len() {\n        if !(left[index] == right[index]) {\n            return false;\n        }\n        index += 1;\n    }\n    true\n}\n\n/// Split borrows the original elements and evaluates the predicate once per element.\npub struct SliceSplit<'a, T, F> {\n    items: &'a [T],\n    predicate: F,\n    start: usize,\n    done: bool,\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::Iterator for SliceSplit<'a, T, F> {\n    type Item = &'a [T];\n    fn next(&mut self) -> Option<&'a [T]> {\n        if self.done {\n            return Option::None;\n        }\n        let items = self.items;\n        let start = self.start;\n        let mut end = start;\n        while end < items.len() {\n            if (self.predicate)(&items[end]) {\n                self.start = end + 1;\n                return Option::Some(&items[start..end]);\n            }\n            end += 1;\n        }\n        self.done = true;\n        Option::Some(&items[start..])\n    }\n}\nimpl<'a, T, F: FnMut(&T) -> bool> crate::iter::IntoIterator for SliceSplit<'a, T, F> {\n    type IntoIter = Self;\n    fn into_iter(self) -> Self {\n        self\n    }\n}\npub trait SliceSplitSource<T> {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F>;\n}\nimpl<T> SliceSplitSource<T> for [T] {\n    fn split<'a, F: FnMut(&T) -> bool>(&'a self, predicate: F) -> SliceSplit<'a, T, F> {\n        SliceSplit {\n            items: self,\n            predicate,\n            start: 0,\n            done: false,\n        }\n    }\n}\n")),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("vector_traits_probe.rs", Some("fn probe(){let values:Vec<i64>=Vec::new();values.clone();}")),
], "E_TYPE"), errors);
        count += 1;
    }
    if 535usize % workers == shard {
        record(source_case("zero_sized_unit_values_survive_references_and_collections_0000", "unit-values", Some("$std/lib.rs"), vec![
source_file("$std/lib.rs", None),
source_file("$std/prelude.rs", None),
source_file("$std/sequence.rs", None),
source_file("$std/buffer.rs", None),
source_file("$std/timers.rs", None),
source_file("$std/json.rs", None),
source_file("$std/hmac.rs", None),
source_file("$std/time.rs", None),
source_file("$std/bytes.rs", None),
source_file("$std/string.rs", None),
source_file("$std/str.rs", None),
source_file("$std/sha256.rs", None),
source_file("$std/char.rs", None),
source_file("$std/fmt.rs", None),
source_file("$std/iter.rs", None),
source_file("$std/integer.rs", None),
source_file("$std/io.rs", None),
source_file("$std/ordered_map.rs", None),
source_file("$std/ordered_set.rs", None),
source_file("unit-values", Some("\n        struct Pair { empty: (), number: i64 }\n        fn unit(value:()) { let borrowed = &value; let copied = *borrowed; copied }\n        pub fn probe()->i64 {\n            let value = Pair { empty: (), number: 42 };\n            unit(value.empty);\n            let mut values:Vec<()> = Vec::new();\n            values.push(());\n            values.push(());\n            let item = values.pop().unwrap();\n            unit(item);\n            value.number + values.len() as i64 - 1\n        }\n    ")),
], ""), errors);
        count += 1;
    }
    count
}
