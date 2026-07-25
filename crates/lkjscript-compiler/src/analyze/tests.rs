
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use lkjscript_core::{Op, Result};

    use super::analyze_program;
    use crate::codegen::compile_program;
    use crate::hir::{
        BindingKind, BindingStorage, CoreTrait, EffectSet, ExprKind, Operation, Origin,
        TraitWitnessKind, Type,
    };
    use crate::source::{validate_source_set_for_analysis, ValidatedSourceTree};
    use crate::ssa::lower_program;

    fn parsed_program(files: &[(&str, &str)]) -> Result<ValidatedSourceTree> {
        let root = files.last().map_or("test.lkjscript", |(path, _)| *path);
        let limits = lkjscript_core::Limits {
            max_tokens_per_file: u32::MAX,
            max_toplevel_forms: u32::MAX,
            ..lkjscript_core::Limits::default()
        };
        validate_source_set_for_analysis(files, root, &limits)
    }

    fn analyze_one(source: &str) -> Result<crate::hir::Program> {
        analyze_program(&parsed_program(&[("test.lkjscript", source)])?)
    }

    fn compile_hir(program: &crate::hir::Program) -> Result<lkjscript_core::Chunk> {
        let ssa = lower_program(program)?;
        compile_program(&ssa).map(|(chunk, _links)| chunk)
    }

    fn analysis_error(source: &str) -> String {
        analyze_one(source).expect_err("analysis must fail").to_string()
    }

    fn main_source(return_type: &str, body: &str) -> String {
        format!("main/\nsig/\n->\n{return_type}\n/sig\n{body}\n/main\n")
    }

    fn function_source(
        name: &str,
        forall: &[&str],
        signature: &str,
        params: &str,
        body: &str,
    ) -> String {
        let forall = if forall.is_empty() {
            String::new()
        } else {
            format!("forall/\n{}\n/forall\n", forall.join("\n"))
        };
        format!(
            "def/\nname/\n{name}\n/name\nfn/\n{forall}sig/\n{signature}\n/sig\nparams/\n{params}\n/params\n{body}\n/fn\n/def\n"
        )
    }

    fn summary(program: &crate::hir::Program, name: &str) -> EffectSet {
        let binding = program
            .bindings
            .iter()
            .find(|binding| binding.name == name)
            .expect("named function binding")
            .id;
        program
            .functions
            .iter()
            .find(|function| function.binding == binding)
            .expect("named HIR function")
            .summary
    }

    const POINT_PRODUCT: &str = "product/\nname/\nPoint\n/name\nfields/\nfield/\nname/\nx\n/name\ntype/\nI64\n/type\n/field\nfield/\nname/\ny\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\n";

    fn marker_trait(name: &str) -> String {
        format!("trait/\nname/\n{name}\n/name\n/trait\n")
    }

    fn marker_impl(trait_name: &str, product_name: &str) -> String {
        format!("impl/\ntrait/\n{trait_name}\n/trait\nfor/\nProduct\n{product_name}\n/for\n/impl\n")
    }

    fn bounded_identity(name: &str, trait_name: &str) -> String {
        format!(
            "def/\nname/\n{name}\n/name\nfn/\nforall/\nT\n/forall\nbounds/\nbound/\nT\n{trait_name}\n/bound\n/bounds\nsig/\nT\n->\nT\n/sig\nparams/\nvalue\nT\n/params\nvalue\n/fn\n/def\n"
        )
    }

    #[test]
    fn explicit_main_is_unique_root_only_and_exactly_typed() {
        assert!(analysis_error("").contains("exactly one main"));
        let duplicate = "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
        assert!(analysis_error(duplicate).contains("duplicate main"));
        let mismatch = main_source("I64", "unit");
        assert!(analysis_error(&mismatch).contains("does not exactly equal"));
        let parameter = "main/\nsig/\nI64\n->\nI64\n/sig\n1\n/main\n";
        assert!(analysis_error(parameter).contains("no parameters"));

        let ast = parsed_program(&[
            (
                "lib.lkjscript",
                "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
            ),
            (
                "root.lkjscript",
                "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
            ),
        ])
        .expect("parse imported main");
        assert!(analyze_program(&ast)
            .expect_err("imported main must fail")
            .to_string()
            .contains("imported file"));
    }

    #[test]
    fn top_level_do_and_runtime_value_definitions_are_removed() {
        assert!(analysis_error("do/\nunit\n/do\n").contains("top-level do"));
        let value = "def/\nname/\nx\n/name\ntype/\nI64\n/type\n1\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
        assert!(analysis_error(value).contains("malformed top-level def declaration shape"));
    }

    #[test]
    fn main_and_function_are_explicit_hir_nodes() {
        let source = "def/\nname/\nanswer\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\n42\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nanswer/\n/answer\n/main\n";
        let program = analyze_one(source).expect("analyze explicit callable program");
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.main.return_type, Type::I64);
        assert_eq!(program.main.body.ty, Type::I64);
        assert_eq!(program.global_layout, vec![program.functions[0].binding]);
        assert_eq!(program.global_layout.len(), 1);
        let ExprKind::Call { callee, .. } = program.main.body.kind else {
            panic!("expected resolved function call");
        };
        assert_eq!(callee.storage, BindingStorage::Function);

        let chunk = compile_hir(&program).expect("lower explicit main through SSA");
        assert_eq!(chunk.global_names, vec!["answer"]);
        assert_eq!(chunk.main.name, "main");
        assert!(chunk.main.code.ends_with(&[Op::Return as u8]));
    }

    #[test]
    fn mutable_local_has_stable_binding_and_slot_and_set_resolves_nearest() {
        let source = main_source(
            "I64",
            "var/\nname/\nx\n/name\ntype/\nI64\n/type\n1\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\nx\ndo/\nset/\nx\n2\n/set\nx\n/do\n/var\n/var",
        );
        let program = analyze_one(&source).expect("analyze nested vars");
        let ExprKind::MutableLocal {
            binding: outer,
            slot: outer_slot,
            body: outer_body,
            ..
        } = &program.main.body.kind
        else {
            panic!("expected outer mutable local");
        };
        let ExprKind::MutableLocal {
            binding: inner,
            slot: inner_slot,
            initial,
            body: inner_body,
            ..
        } = &outer_body.kind
        else {
            panic!("expected inner mutable local");
        };
        assert_ne!(outer, inner);
        assert_eq!((*outer_slot, *inner_slot), (0, 1));
        assert_eq!(program.main.local_count, 2);
        assert_eq!(program.binding(*outer).map(|binding| &binding.kind), Some(&BindingKind::MutableLocal));
        let ExprKind::Load(initial_ref) = initial.kind else {
            panic!("inner initializer must resolve outer binding");
        };
        assert_eq!(initial_ref.binding, *outer);
        assert_eq!(initial_ref.storage, BindingStorage::Local(0));
        let ExprKind::Do(expressions) = &inner_body.kind else {
            panic!("expected inner body sequence");
        };
        let ExprKind::SetLocal { target, slot, .. } = expressions[0].kind else {
            panic!("expected local set");
        };
        assert_eq!(target, *inner);
        assert_eq!(slot, 1);
        assert_eq!(expressions[0].effects, EffectSet::MUTATES_LOCAL);
    }

    #[test]
    fn var_initializer_set_type_and_binding_kinds_are_checked() {
        let self_ref = main_source(
            "I64",
            "var/\nname/\nx\n/name\ntype/\nI64\n/type\nx\nx\n/var",
        );
        assert!(analysis_error(&self_ref).contains("unknown symbol x"));
        let wrong_initial = main_source(
            "Unit",
            "var/\nname/\nx\n/name\ntype/\nI64\n/type\n1.0\nunit\n/var",
        );
        assert!(analysis_error(&wrong_initial).contains("exactly equal"));
        let wrong_set = main_source(
            "Unit",
            "var/\nname/\nx\n/name\ntype/\nI64\n/type\n1\nset/\nx\n1.0\n/set\n/var",
        );
        assert!(analysis_error(&wrong_set).contains("exactly equal"));
        let immutable = main_source(
            "Unit",
            "let/\nbind/\nx\n1\n/bind\nset/\nx\n2\n/set\n/let",
        );
        assert!(analysis_error(&immutable).contains("not a function-local mutable var"));
    }

    #[test]
    fn resolution_never_crosses_a_function_boundary() {
        let source = "def/\nname/\nmutate\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nset/\nstate\n1\n/set\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nvar/\nname/\nstate\n/name\ntype/\nI64\n/type\n0\nmutate/\n/mutate\n/var\n/main\n";
        assert!(analysis_error(source).contains("unknown set target state"));
    }

    #[test]
    fn operation_identity_and_local_mutation_reach_bytecode() {
        let source = main_source(
            "I64",
            "var/\nname/\nx\n/name\ntype/\nI64\n/type\nargc/\n/argc\ndo/\nset/\nx\n+/\nx\n2\n/+\n/set\nx\n/do\n/var",
        );
        let program = analyze_one(&source).expect("analyze operation and set");
        let chunk = compile_hir(&program).expect("lower operation and set through SSA");
        assert!(chunk.main.code.contains(&(Op::Add as u8)));
        assert!(chunk.main.code.contains(&(Op::StoreLocal as u8)));
        assert!(chunk.global_names.is_empty());
    }

    #[test]
    fn nominal_products_remain_resolved_and_state_threadable() {
        let source = format!(
            "{POINT_PRODUCT}def/\nname/\nmove-x\n/name\nfn/\nsig/\nProduct\nPoint\nI64\n->\nProduct\nPoint\n/sig\nparams/\npoint\nProduct/\nPoint\n/Product\nx\nI64\n/params\nwith-field/\npoint\nx\nx\n/with-field\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nvar/\nname/\npoint\n/name\ntype/\nProduct\nPoint\n/type\nproduct-value/\nPoint\nfield/\nx\n1\n/field\nfield/\ny\n2\n/field\n/product-value\ndo/\nset/\npoint\nmove-x/\npoint\n7\n/move-x\n/set\nfield/\npoint\nx\n/field\n/do\n/var\n/main\n"
        );
        let program = analyze_one(&source).expect("analyze product state threading");
        assert_eq!(program.products.len(), 1);
        assert_eq!(program.products[0].name, "Point");
        assert!(!program
            .global_layout
            .iter()
            .any(|binding| program.binding(*binding).is_some_and(|binding| binding.name == "Point")));
        let chunk = compile_hir(&program).expect("lower product state threading through SSA");
        assert!(chunk.main.code.contains(&(Op::MakeProduct as u8)));
        assert!(chunk.main.code.contains(&(Op::StoreLocal as u8)));
        assert!(!chunk.product_fields.is_empty());
    }

    #[test]
    fn product_field_boundaries_and_import_origins_remain_stable() {
        let mut fifteen = String::from("product/\nname/\nWide\n/name\nfields/\n");
        for index in 0..15 {
            fifteen.push_str(&format!(
                "field/\nname/\nf{index}\n/name\ntype/\nI64\n/type\n/field\n"
            ));
        }
        fifteen.push_str("/fields\n/product\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n");
        assert_eq!(analyze_one(&fifteen).expect("15 fields").products[0].fields.len(), 15);
        let sixteen = fifteen.replacen(
            "/fields\n/product\n",
            "field/\nname/\nf15\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\n",
            1,
        );
        assert!(analysis_error(&sixteen).contains("too many fields"));

        let dependency = "def/\nname/\nanswer\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\n42\n/fn\n/def\n";
        let root = "main/\nsig/\n->\nI64\n/sig\nanswer/\n/answer\n/main\n";
        let program = analyze_program(
            &parsed_program(&[("imports/dependency.lkjscript", dependency), ("app/main.lkjscript", root)])
                .expect("parse source files"),
        )
        .expect("analyze imports");
        let binding = program
            .bindings
            .iter()
            .find(|binding| binding.name == "answer")
            .expect("answer binding");
        let Origin::Source(source_id) = binding.origin else {
            panic!("answer must have source origin");
        };
        assert_eq!(
            source_id
                .index()
                .and_then(|index| program.sources.get(index))
                .map(|source| &source.path),
            Some(&PathBuf::from("imports/dependency.lkjscript"))
        );
    }

    #[test]
    fn pure_leaf_and_direct_transitive_calls_remain_exact() {
        let pure_leaf = function_source("pure-leaf", &[], "->\nI64", "", "7");
        let pure_middle = function_source(
            "pure-middle",
            &[],
            "->\nI64",
            "",
            "pure-leaf/\n/pure-leaf",
        );
        let source = format!(
            "{pure_leaf}{pure_middle}{}",
            main_source("I64", "pure-middle/\n/pure-middle")
        );
        let program = analyze_one(&source).expect("analyze pure direct calls");
        assert_eq!(summary(&program, "pure-leaf"), EffectSet::PURE);
        assert_eq!(summary(&program, "pure-middle"), EffectSet::PURE);
        assert_eq!(program.main.body.effects, EffectSet::PURE);

        let trap_leaf = function_source("trap-leaf", &[], "->\nI64", "", "div/\n8\n2\n/div");
        let middle = function_source(
            "middle",
            &[],
            "->\nI64",
            "",
            "trap-leaf/\n/trap-leaf",
        );
        let outer = function_source("outer", &[], "->\nI64", "", "middle/\n/middle");
        let source = format!(
            "{trap_leaf}{middle}{outer}{}",
            main_source("I64", "outer/\n/outer")
        );
        let program = analyze_one(&source).expect("analyze transitive direct effects");
        assert_eq!(summary(&program, "trap-leaf"), EffectSet::MAY_TRAP);
        assert_eq!(summary(&program, "middle"), EffectSet::MAY_TRAP);
        assert_eq!(summary(&program, "outer"), EffectSet::MAY_TRAP);
        assert_eq!(program.main.body.effects, EffectSet::MAY_TRAP);
    }

    #[test]
    fn pure_direct_recursion_adds_only_divergence() {
        let recurse = function_source(
            "recurse",
            &[],
            "->\nUnit",
            "",
            "recurse/\n/recurse",
        );
        let source = format!(
            "{recurse}{}",
            main_source("Unit", "recurse/\n/recurse")
        );
        let program = analyze_one(&source).expect("analyze pure recursion");
        assert_eq!(summary(&program, "recurse"), EffectSet::MAY_DIVERGE);
        assert_eq!(program.functions[0].body.effects, EffectSet::MAY_DIVERGE);
        assert_eq!(program.main.body.effects, EffectSet::MAY_DIVERGE);
    }

    #[test]
    fn effectful_recursion_retains_only_its_real_effects_and_divergence() {
        let recurse = function_source(
            "recurse",
            &[],
            "->\nUnit",
            "",
            "do/\ndiv/\n8\n2\n/div\nrecurse/\n/recurse\n/do",
        );
        let source = format!("{recurse}{}", main_source("Unit", "unit"));
        let program = analyze_one(&source).expect("analyze effectful recursion");
        assert_eq!(
            summary(&program, "recurse"),
            EffectSet::MAY_TRAP.union(EffectSet::MAY_DIVERGE)
        );
    }

    #[test]
    fn mutual_recursion_and_declaration_order_have_identical_summaries() {
        let a = function_source("a", &[], "->\nUnit", "", "b/\n/b");
        let b = function_source("b", &[], "->\nUnit", "", "a/\n/a");
        let caller = function_source("caller", &[], "->\nUnit", "", "a/\n/a");
        let main = main_source("Unit", "caller/\n/caller");
        let first = analyze_one(&format!("{a}{b}{caller}{main}"))
            .expect("analyze mutual recursion in first declaration order");
        let second = analyze_one(&format!("{b}{caller}{a}{main}"))
            .expect("analyze mutual recursion in second declaration order");
        for name in ["a", "b", "caller"] {
            assert_eq!(summary(&first, name), EffectSet::MAY_DIVERGE);
            assert_eq!(summary(&first, name), summary(&second, name));
        }
        assert_eq!(first.main.body.effects, EffectSet::MAY_DIVERGE);
        assert_eq!(first.main.body.effects, second.main.body.effects);
    }

    #[test]
    fn direct_effect_categories_are_inferred_without_unrelated_bits() {
        let allocation = function_source(
            "allocation",
            &[],
            "->\nList\nI64",
            "",
            "cons/\n1\nempty-list/\nI64\n/empty-list\n/cons",
        );
        let trap = function_source("trap", &[], "->\nI64", "", "div/\n8\n2\n/div");
        let host = function_source(
            "host",
            &[],
            "->\nUnit",
            "",
            "print/\nstr/\nhello\n/str\n/print",
        );
        let outcome = function_source("outcome", &[], "->\nUnit", "", "exit/\n0\n/exit");
        let mutation = function_source(
            "mutation",
            &[],
            "->\nUnit",
            "",
            "var/\nname/\nx\n/name\ntype/\nI64\n/type\n0\nset/\nx\n1\n/set\n/var",
        );
        let read = function_source(
            "memory-read",
            &[],
            "Buf\n->\nI64",
            "value\nBuf",
            "buf-len/\nvalue\n/buf-len",
        );
        let write = function_source(
            "memory-write",
            &[],
            "Buf\n->\nUnit",
            "value\nBuf",
            "buf-set/\nvalue\n0\n1\n/buf-set",
        );
        let source = format!(
            "{allocation}{trap}{host}{outcome}{mutation}{read}{write}{}",
            main_source("Unit", "unit")
        );
        let program = analyze_one(&source).expect("analyze direct effect categories");
        assert_eq!(
            summary(&program, "allocation"),
            EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP)
        );
        assert_eq!(summary(&program, "trap"), EffectSet::MAY_TRAP);
        assert_eq!(
            summary(&program, "host"),
            EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::MAY_TRAP)
        );
        assert_eq!(
            summary(&program, "outcome"),
            EffectSet::HOST_IO
                .union(EffectSet::MAY_EXIT)
                .union(EffectSet::MAY_TRAP)
        );
        assert_eq!(summary(&program, "mutation"), EffectSet::MUTATES_LOCAL);
        assert_eq!(summary(&program, "memory-read"), EffectSet::READS_MEMORY);
        assert_eq!(
            summary(&program, "memory-write"),
            EffectSet::WRITES_MEMORY.union(EffectSet::MAY_TRAP)
        );
    }

    #[test]
    fn generic_direct_call_uses_canonical_binding_and_keeps_argument_effects() {
        let identity = function_source("identity", &["T"], "T\n->\nT", "value\nT", "value");
        let source = format!(
            "{identity}{}",
            main_source("I64", "identity/\ndiv/\n8\n2\n/div\n/identity")
        );
        let program = analyze_one(&source).expect("analyze generic direct call");
        let identity = &program.functions[0];
        assert_eq!(identity.summary, EffectSet::PURE);
        let ExprKind::Call { callee, args, .. } = &program.main.body.kind else {
            panic!("expected generic direct call");
        };
        assert_eq!(callee.binding, identity.binding);
        assert_eq!(callee.storage, BindingStorage::Function);
        assert_eq!(args[0].effects, EffectSet::MAY_TRAP);
        assert_eq!(program.main.body.effects, EffectSet::MAY_TRAP);
    }

    #[test]
    fn indirect_local_call_is_conservative_and_loses_no_effect_bit() {
        let leaf = function_source("leaf", &[], "->\nI64", "", "7");
        let indirect = function_source(
            "indirect",
            &[],
            "->\nI64",
            "",
            "let/\nbind/\nf\nleaf\n/bind\nf/\n/f\n/let",
        );
        let source = format!("{leaf}{indirect}{}", main_source("Unit", "unit"));
        let program = analyze_one(&source).expect("analyze indirect local call");
        assert_eq!(summary(&program, "leaf"), EffectSet::PURE);
        assert_eq!(
            summary(&program, "indirect"),
            EffectSet::CONSERVATIVE_CALL
        );
        for required in [
            EffectSet::ALLOCATES,
            EffectSet::READS_MEMORY,
            EffectSet::WRITES_MEMORY,
            EffectSet::MUTATES_LOCAL,
            EffectSet::HOST_IO,
            EffectSet::MAY_TRAP,
            EffectSet::MAY_EXIT,
            EffectSet::MAY_DIVERGE,
        ] {
            assert!(EffectSet::CONSERVATIVE_CALL.contains(required));
        }
    }

    #[test]
    fn marker_trait_impl_and_bound_resolve_to_dense_canonical_identities() {
        let source = format!(
            "{}{}{}{}{}",
            marker_trait("Marked"),
            POINT_PRODUCT,
            marker_impl("Marked", "Point"),
            bounded_identity("keep-marked", "Marked"),
            main_source(
                "Product\nPoint",
                "keep-marked/\nproduct-value/\nPoint\nfield/\nx\n1\n/field\nfield/\ny\n2\n/field\n/product-value\n/keep-marked"
            )
        );
        let program = analyze_one(&source).expect("analyze marker trait program");
        assert_eq!(program.traits.len(), CoreTrait::ALL.len() + 1);
        for (index, core) in CoreTrait::ALL.iter().enumerate() {
            assert_eq!(program.traits[index].name, core.name());
            assert_eq!(program.traits[index].id.raw(), index as u32);
        }
        let marker = &program.traits[CoreTrait::ALL.len()];
        assert_eq!(marker.name, "Marked");
        assert_eq!(program.implementations.len(), 1);
        assert_eq!(program.implementations[0].trait_id, marker.id);
        assert_eq!(program.functions[0].bounds[0].trait_id, marker.id);
        let ExprKind::Call {
            instantiation: Some(instantiation),
            ..
        } = &program.main.body.kind
        else {
            panic!("expected resolved bounded generic call");
        };
        assert_eq!(instantiation.substitutions[0].ty, Type::Product("Point".into()));
        assert_eq!(instantiation.witnesses[0].trait_id, marker.id);
        assert_eq!(
            instantiation.witnesses[0].kind,
            TraitWitnessKind::Explicit(program.implementations[0].id)
        );

        let ssa = lower_program(&program).expect("lower bounded marker program");
        assert_eq!(ssa.program().traits.len(), CoreTrait::ALL.len() + 1);
        assert_eq!(ssa.program().implementations.len(), 1);
        assert_eq!(ssa.program().functions[0].signature.bounds.len(), 1);
        let call = ssa.program().functions.last().expect("main").blocks.iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match &instruction.kind {
                lkjscript_ir::InstructionKind::Call { instantiation, .. } => instantiation.as_ref(),
                _ => None,
            })
            .expect("SSA bounded call");
        assert_eq!(call.witnesses.len(), 1);
    }

    #[test]
    fn malformed_marker_declarations_bounds_and_namespace_collisions_are_rejected() {
        let main = main_source("Unit", "unit");
        for malformed in [
            format!("trait/\nname/\nMarked\n/name\nmethod/\nclone\n/method\n/trait\n{main}"),
            format!("trait/\nname/\nlower\n/name\n/trait\n{main}"),
            format!("impl/\ntrait/\nMarked\n/trait\nfor/\nI64\n/for\nvalue/\n1\n/value\n/impl\n{main}"),
        ] {
            assert!(analyze_one(&malformed).is_err(), "accepted malformed marker declaration");
        }
        let duplicate = format!("{}{}{}", marker_trait("Marked"), marker_trait("Marked"), main);
        assert!(analysis_error(&duplicate).contains("duplicate trait"));
        let product_collision = format!("{}{}{}", marker_trait("Point"), POINT_PRODUCT, main);
        assert!(analysis_error(&product_collision).contains("collides with a trait"));
        let function_collision = format!(
            "{}{}{}",
            marker_trait("Marked"),
            function_source("Marked", &[], "->\nUnit", "", "unit"),
            main
        );
        assert!(analysis_error(&function_collision).contains("duplicate global"));
        for reserved in ["Copy", "Clone", "Drop", "Send", "Sync", "I64"] {
            let source = format!("{}{main}", marker_trait(reserved));
            assert!(analyze_one(&source).is_err(), "accepted reserved trait {reserved}");
        }
    }

    #[test]
    fn impl_coherence_unknown_names_and_core_auto_trait_assertions_are_rejected() {
        let main = main_source("Unit", "unit");
        let overlap = format!(
            "{}{}{}{}{}",
            marker_trait("Marked"), POINT_PRODUCT,
            marker_impl("Marked", "Point"), marker_impl("Marked", "Point"), main
        );
        assert!(analysis_error(&overlap).contains("LKJ-DECL-DUPLICATE"));
        let unknown_trait = format!("{}{}{main}", POINT_PRODUCT, marker_impl("Missing", "Point"));
        assert!(analysis_error(&unknown_trait).contains("unknown trait"));
        let unknown_product = format!("{}{}{main}", marker_trait("Marked"), marker_impl("Marked", "Missing"));
        assert!(analysis_error(&unknown_product).contains("unknown product"));
        for core in ["Copy", "Clone", "Drop", "Send", "Sync"] {
            let source = format!("{}{}{main}", POINT_PRODUCT, marker_impl(core, "Point"));
            assert!(analysis_error(&source).contains("cannot be explicitly implemented"));
        }
        let declaration_after_impl = format!(
            "{}{}{}{}",
            marker_impl("Marked", "Point"), marker_trait("Marked"), POINT_PRODUCT,
            main_source("Unit", "unit")
        );
        assert!(analyze_one(&declaration_after_impl).is_ok());
    }

    #[test]
    fn bounds_require_declared_parameters_known_traits_and_satisfied_facts() {
        let main = main_source("Unit", "unit");
        let undeclared = bounded_identity("bad", "Marked").replace("bound/\nT\n", "bound/\nU\n");
        let source = format!("{}{}{main}", marker_trait("Marked"), undeclared);
        assert!(analysis_error(&source).contains("not declared by forall"));
        let unknown = format!("{}{main}", bounded_identity("bad", "Missing"));
        assert!(analysis_error(&unknown).contains("unknown trait"));
        let duplicate = bounded_identity("bad", "Copy").replace(
            "/bounds",
            "bound/\nT\nCopy\n/bound\n/bounds",
        );
        let source = format!("{duplicate}{main}");
        assert!(analysis_error(&source).contains("duplicate bound"));
        for unavailable in ["Clone", "Drop"] {
            let source = format!("{}{main}", bounded_identity("bad", unavailable));
            assert!(
                analysis_error(&source).contains("requires methods"),
                "accepted unavailable core bound {unavailable}"
            );
        }

        let satisfied = format!(
            "{}{}",
            bounded_identity("copy-value", "Copy"),
            main_source("I64", "copy-value/\n7\n/copy-value")
        );
        let program = analyze_one(&satisfied).expect("Copy bound is structurally satisfied");
        let ExprKind::Call { instantiation: Some(instantiation), .. } = &program.main.body.kind else {
            panic!("expected Copy instantiation");
        };
        assert_eq!(instantiation.witnesses[0].kind, TraitWitnessKind::AutoTrait);

        let unsatisfied = format!(
            "{}{}",
            bounded_identity("copy-value", "Copy"),
            main_source("Buf", "copy-value/\nbuf-new/\n1\n/buf-new\n/copy-value")
        );
        assert!(analysis_error(&unsatisfied).contains("does not satisfy trait Copy"));

        let first_class = format!(
            "{}{}",
            bounded_identity("copy-value", "Copy"),
            main_source(
                "I64",
                "let/\nbind/\nf\ncopy-value\n/bind\nf/\n7\n/f\n/let"
            )
        );
        assert!(analysis_error(&first_class).contains("not a first-class value"));

        let forwarding = bounded_identity("forward", "Copy").replace(
            "\nvalue\n/fn",
            "\ncopy-value/\nvalue\n/copy-value\n/fn",
        );
        let source = format!(
            "{}{}{}",
            bounded_identity("copy-value", "Copy"),
            forwarding,
            main_source("Unit", "unit")
        );
        assert!(analysis_error(&source).contains("generic context is unavailable"));
    }

    #[test]
    fn structural_auto_traits_cover_nested_products_and_reject_resources_and_cycles() {
        let nested_products = "product/\nname/\nLeaf\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nStr\n/type\n/field\n/fields\n/product\nproduct/\nname/\nNest\n/name\nfields/\nfield/\nname/\nitems\n/name\ntype/\nOption\nResult\nList\nProduct\nLeaf\nI64\n/type\n/field\n/fields\n/product\n";
        let value = "product-value/\nNest\nfield/\nitems\nnone/\nResult\nList\nProduct\nLeaf\nI64\n/none\n/field\n/product-value";
        let copy_source = format!(
            "{nested_products}{}{}",
            bounded_identity("accept", "Copy"),
            main_source("Product\nNest", &format!("accept/\n{value}\n/accept"))
        );
        analyze_one(&copy_source).expect("nested immutable GC handles are Copy within one worker");
        for worker_trait in ["Send", "Sync"] {
            let source = format!(
                "{nested_products}{}{}",
                bounded_identity("accept", worker_trait),
                main_source("Product\nNest", &format!("accept/\n{value}\n/accept"))
            );
            assert!(
                analysis_error(&source).contains("does not satisfy trait"),
                "worker-local GC product unexpectedly satisfied {worker_trait}"
            );
        }
        for worker_trait in ["Send", "Sync"] {
            let source = format!(
                "{}{}",
                bounded_identity("accept-scalar", worker_trait),
                main_source("I64", "accept-scalar/\n7\n/accept-scalar")
            );
            analyze_one(&source).unwrap_or_else(|error| {
                panic!("scalar {worker_trait} fact unexpectedly failed: {error}")
            });
        }
        for ty in ["Buf", "Handle"] {
            let body = if ty == "Buf" { "buf-new/\n0\n/buf-new" } else { "stdin-handle/\n/stdin-handle" };
            let source = format!(
                "{}{}",
                bounded_identity("send-value", "Send"),
                main_source(ty, &format!("send-value/\n{body}\n/send-value"))
            );
            assert!(analysis_error(&source).contains("does not satisfy trait Send"));
        }

        let recursive = "product/\nname/\nRecursive\n/name\nfields/\nfield/\nname/\nnext\n/name\ntype/\nOption\nProduct\nRecursive\n/type\n/field\n/fields\n/product\n";
        let source = format!(
            "{recursive}{}{}",
            bounded_identity("copy-recursive", "Copy"),
            main_source("Product\nRecursive", "copy-recursive/\nproduct-value/\nRecursive\nfield/\nnext\nnone/\nProduct\nRecursive\n/none\n/field\n/product-value\n/copy-recursive")
        );
        assert!(analysis_error(&source).contains("recursive product cycle"));

        let mut deep = String::new();
        for index in 0..20 {
            let field_type = if index == 19 {
                "I64".to_string()
            } else {
                format!("Option\nProduct\nP{}", index + 1)
            };
            deep.push_str(&format!(
                "product/\nname/\nP{index}\n/name\nfields/\nfield/\nname/\nnext\n/name\ntype/\n{field_type}\n/type\n/field\n/fields\n/product\n"
            ));
        }
        deep.push_str(&bounded_identity("copy-deep", "Copy"));
        deep.push_str(&main_source(
            "Product\nP0",
            "copy-deep/\nproduct-value/\nP0\nfield/\nnext\nnone/\nProduct\nP1\n/none\n/field\n/product-value\n/copy-deep",
        ));
        let first = analysis_error(&deep);
        let second = analysis_error(&deep);
        assert!(first.contains("trait solver depth exceeded"));
        assert_eq!(first, second);
    }

    #[test]
    fn trait_and_impl_metadata_follow_source_closure_and_declaration_order() {
        let dependency = format!("{}{}{}", marker_trait("First"), POINT_PRODUCT, marker_impl("First", "Point"));
        let root = format!("{}{}", marker_trait("Second"), main_source("Unit", "unit"));
        let first = analyze_program(&parsed_program(&[("dep.lkjscript", &dependency), ("root.lkjscript", &root)]).expect("parse closure"))
            .expect("analyze closure");
        let second = analyze_program(&parsed_program(&[("dep.lkjscript", &dependency), ("root.lkjscript", &root)]).expect("parse closure again"))
            .expect("analyze closure again");
        let facts = |program: &crate::hir::Program| {
            (
                program.traits.iter().map(|definition| (definition.id.raw(), definition.name.clone())).collect::<Vec<_>>(),
                program.implementations.iter().map(|implementation| (implementation.id.raw(), implementation.trait_id.raw(), implementation.product.raw())).collect::<Vec<_>>(),
            )
        };
        assert_eq!(facts(&first), facts(&second));
        assert_eq!(first.traits[CoreTrait::ALL.len()].name, "First");
        assert_eq!(first.traits[CoreTrait::ALL.len() + 1].name, "Second");
    }

    #[test]
    fn generic_and_equality_operation_types_remain_exact() {
        let source = main_source(
            "Bool",
            "list-equal/\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\n/list-equal",
        );
        let program = analyze_one(&source).expect("analyze exact list equality");
        let ExprKind::Operation {
            operation,
            resolved_signature,
            ..
        } = &program.main.body.kind
        else {
            panic!("expected resolved operation");
        };
        assert_eq!(*operation, Operation::ListEqual);
        assert_eq!(program.main.body.ty, Type::Bool);
        assert_eq!(
            resolved_signature,
            &Type::Fn {
                params: vec![
                    Type::List(Box::new(Type::I64)),
                    Type::List(Box::new(Type::I64)),
                ],
                ret: Box::new(Type::Bool),
            }
        );
        assert!(program.main.body.effects != EffectSet::PURE);
    }
}
