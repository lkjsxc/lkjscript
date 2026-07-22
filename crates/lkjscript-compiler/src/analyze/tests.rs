
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use lkjscript_core::{Op, Result};

    use super::analyze_program;
    use crate::codegen::compile_program;
    use crate::hir::{BindingKind, EffectSet, ExprKind, Operation, Origin, TopLevel, Type};
    use crate::import::{Program as AstProgram, SourceFile};
    use crate::lex::lex;
    use crate::parse::parse_tokens;

    fn parsed_program(files: &[(&str, &str)]) -> Result<AstProgram> {
        let mut parsed_files = Vec::with_capacity(files.len());
        for (path, source) in files {
            let tokens = lex(source)?;
            let forms = parse_tokens(&tokens)?;
            parsed_files.push(SourceFile {
                path: PathBuf::from(path),
                forms,
            });
        }
        Ok(AstProgram {
            root: PathBuf::from(files.last().map_or("test.lkjscript", |(path, _)| *path)),
            files: parsed_files,
        })
    }

    fn analyze_one(source: &str) -> Result<crate::hir::Program> {
        let program = parsed_program(&[("test.lkjscript", source)])?;
        analyze_program(&program)
    }

    fn analysis_error(source: &str) -> String {
        analyze_one(source).expect_err("analysis must fail").to_string()
    }

    #[test]
    fn rejects_duplicate_unknown_and_non_function_declarations() {
        let duplicate = "def/\nname/\nx\n/name\ntype/\nI64\n/type\n1\n/def\ndef/\nname/\nx\n/name\ntype/\nI64\n/type\n2\n/def\n";
        assert!(analysis_error(duplicate).contains("duplicate global declaration x"));

        assert!(analysis_error("do/\nmissing\n/do\n").contains("unknown symbol missing"));
        assert!(analysis_error("do/\nmissing/\n/missing\n/do\n").contains("unknown call missing"));

        let non_function = "def/\nname/\nx\n/name\ntype/\nI64\n/type\n1\n/def\ndo/\nx/\n/x\n/do\n";
        assert!(analysis_error(non_function).contains("x is not a function"));

        let duplicate_parameter = "def/\nname/\nf\n/name\nfn/\nsig/\nI64\nI64\n->\nI64\n/sig\nparams/\nx\nI64\nx\nI64\n/params\nx\n/fn\n/def\n";
        assert!(analysis_error(duplicate_parameter).contains("duplicate parameter x"));
    }

    #[test]
    fn header_phase_supports_forward_references_and_recursion() {
        let source = "def/\nname/\nfirst\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\nsecond/\n/second\n/fn\n/def\ndef/\nname/\nsecond\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\n1\n/fn\n/def\ndef/\nname/\nself\n/name\nfn/\nsig/\nBool\n->\nI64\n/sig\nparams/\nstop\nBool\n/params\nif/\nstop\n1\nself/\ntrue\n/self\n/if\n/fn\n/def\n";
        let program = analyze_one(source).expect("forward and recursive analysis");
        assert_eq!(program.forms.len(), 3);
    }

    #[test]
    fn source_ids_preserve_imported_origins_and_diagnostics() {
        let dependency = "def/\nname/\nanswer\n/name\ntype/\nI64\n/type\n42\n/def\n";
        let root = "do/\nanswer\n/do\n";
        let ast = parsed_program(&[
            ("imports/dependency.lkjscript", dependency),
            ("app/main.lkjscript", root),
        ])
        .expect("parse source files");
        let program = analyze_program(&ast).expect("analyze source files");
        let binding = program
            .bindings
            .iter()
            .find(|binding| binding.name == "answer")
            .expect("answer binding");
        let Origin::Source(source_id) = binding.origin else {
            panic!("answer must have a source origin");
        };
        assert_eq!(
            source_id
                .index()
                .and_then(|index| program.sources.get(index))
                .map(|source| &source.path),
            Some(&PathBuf::from("imports/dependency.lkjscript"))
        );

        let bad_ast = parsed_program(&[("imports/bad.lkjscript", "do/\nunknown\n/do\n")])
            .expect("parse bad import");
        let error = analyze_program(&bad_ast)
            .expect_err("unknown imported symbol")
            .to_string();
        assert!(error.contains("imports/bad.lkjscript"));
    }

    #[test]
    fn lexical_shadowing_resolves_each_load_to_the_nearest_binding_id() {
        let source = "def/\nname/\nshadow\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nx\nI64\n/params\nlet/\nbind/\nx\nx\n/bind\nlet/\nbind/\nx\nx\n/bind\nx\n/let\n/let\n/fn\n/def\n";
        let program = analyze_one(source).expect("analyze shadowing");
        let TopLevel::Function(function) = &program.forms[0] else {
            panic!("expected function");
        };
        let parameter = function.params[0];
        let ExprKind::Let {
            bindings: outer,
            body: outer_body,
        } = &function.body.kind
        else {
            panic!("expected outer let");
        };
        let outer_binding = outer[0].binding;
        assert_eq!(outer[0].value.kind, ExprKind::Load(parameter));
        let ExprKind::Let {
            bindings: inner,
            body: inner_body,
        } = &outer_body.kind
        else {
            panic!("expected inner let");
        };
        let inner_binding = inner[0].binding;
        assert_ne!(parameter, outer_binding);
        assert_ne!(outer_binding, inner_binding);
        assert_eq!(inner[0].value.kind, ExprKind::Load(outer_binding));
        assert_eq!(inner_body.kind, ExprKind::Load(inner_binding));
        assert_eq!(outer[0].slot, 1);
        assert_eq!(inner[0].slot, 2);
        assert_eq!(function.local_count, 3);
    }

    #[test]
    fn canonical_operation_identity_drives_exact_bytecode_lowering() {
        let program = analyze_one("do/\n+/\n1\n2\n/+\n/do\n").expect("analyze add");
        let TopLevel::Do { expression, .. } = &program.forms[0] else {
            panic!("expected top-level do");
        };
        let ExprKind::Do(expressions) = &expression.kind else {
            panic!("expected typed do");
        };
        let ExprKind::Operation {
            binding,
            operation,
            ..
        } = expressions[0].kind
        else {
            panic!("expected canonical operation");
        };
        assert_eq!(operation, Operation::Add);
        assert_eq!(
            program.binding(binding).map(|binding| &binding.kind),
            Some(&BindingKind::BuiltinOperation(Operation::Add))
        );
        assert_eq!(expressions[0].ty, Type::I64);
        assert_eq!(expressions[0].effects, EffectSet::MAY_TRAP);
        let ExprKind::Operation {
            resolved_signature,
            ..
        } = &expressions[0].kind
        else {
            panic!("expected resolved operation signature");
        };
        assert_eq!(
            resolved_signature,
            &Type::Fn {
                params: vec![Type::I64, Type::I64],
                ret: Box::new(Type::I64),
            }
        );

        let chunk = compile_program(&program).expect("lower HIR");
        assert_eq!(
            chunk.main.code,
            vec![
                Op::LoadConst as u8,
                0,
                0,
                Op::LoadConst as u8,
                1,
                0,
                Op::Add as u8,
                Op::Unit as u8,
                Op::Return as u8,
            ]
        );
        assert_eq!(
            chunk.global_names,
            Operation::LEGACY_GLOBALS
                .iter()
                .map(|operation| operation.name().to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn set_resolves_mutable_global_targets_and_checks_types() {
        let valid = "def/\nname/\ncount\n/name\ntype/\nI64\n/type\n1\n/def\ndo/\nset/\ncount\n2\n/set\n/do\n";
        let program = analyze_one(valid).expect("valid global set");
        let value_binding = match &program.forms[0] {
            TopLevel::Value(value) => value.binding,
            _ => panic!("expected value definition"),
        };
        let TopLevel::Do { expression, .. } = &program.forms[1] else {
            panic!("expected do");
        };
        let ExprKind::Do(expressions) = &expression.kind else {
            panic!("expected do expression");
        };
        let ExprKind::SetGlobal { target, .. } = expressions[0].kind else {
            panic!("expected resolved set");
        };
        assert_eq!(target, value_binding);
        assert_eq!(expressions[0].ty, Type::Unit);

        assert!(analysis_error("do/\nset/\nmissing\n1\n/set\n/do\n")
            .contains("unknown set target missing"));
        let wrong_type = "def/\nname/\ncount\n/name\ntype/\nI64\n/type\n1\n/def\ndo/\nset/\ncount\n2.0\n/set\n/do\n";
        assert!(analysis_error(wrong_type).contains("not assignable to I64"));
        let function_target = "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nunit\n/fn\n/def\ndo/\nset/\nf\nunit\n/set\n/do\n";
        assert!(analysis_error(function_target).contains("not a mutable global value"));

        let typed_list = "def/\nname/\nxs\n/name\ntype/\nList\nI64\n/type\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\n/def\ndo/\nset/\nxs\nempty-list/\nI64\n/empty-list\n/set\n/do\n";
        assert!(analyze_one(typed_list).is_ok());
        let nil_list = "def/\nname/\nxs\n/name\ntype/\nList\nI64\n/type\nnil\n/def\n";
        assert!(analysis_error(nil_list).contains("Nil not assignable to List"));
    }

    #[test]
    fn generic_list_and_result_calls_instantiate_once_into_exact_types() {
        let source = "do/\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\nunwrap-ok/\nsys-now-ms/\n/sys-now-ms\n/unwrap-ok\n/do\n";
        let program = analyze_one(source).expect("analyze generic calls");
        let TopLevel::Do { expression, .. } = &program.forms[0] else {
            panic!("expected do");
        };
        let ExprKind::Do(expressions) = &expression.kind else {
            panic!("expected do expression");
        };
        assert_eq!(expressions[0].ty, Type::List(Box::new(Type::I64)));
        assert_eq!(expressions[1].ty, Type::I64);
    }

    #[test]
    fn typed_empty_lists_have_exact_hir_and_contextual_types() {
        let program = analyze_one("do/\nempty-list/\nStr\n/empty-list\n/do\n")
            .expect("analyze typed empty list");
        let TopLevel::Do { expression, .. } = &program.forms[0] else {
            panic!("expected do");
        };
        let ExprKind::Do(expressions) = &expression.kind else {
            panic!("expected do expression");
        };
        assert_eq!(expressions[0].kind, ExprKind::EmptyList);
        assert_eq!(expressions[0].ty, Type::List(Box::new(Type::Str)));
        assert_eq!(expressions[0].effects, EffectSet::PURE);

        let chunk = compile_program(&program).expect("lower typed empty list");
        assert!(chunk.main.code.contains(&(Op::EmptyList as u8)));

        let generic = "def/\nname/\nempty\n/name\nfn/\nforall/\nT\n/forall\nsig/\n->\nList\nT\n/sig\nparams/\n/params\nempty-list/\nT\n/empty-list\n/fn\n/def\n";
        assert!(analyze_one(generic).is_ok());
        assert!(analysis_error("do/\nempty-list/\nT\n/empty-list\n/do\n")
            .contains("type parameter T is not declared by forall"));
        assert!(analysis_error("do/\nempty-list/\n/empty-list\n/do\n")
            .contains("empty-list: expected type"));
        assert!(analysis_error("do/\nempty-list/\nI64\nF64\n/empty-list\n/do\n")
            .contains("trailing tokens"));
        assert!(analysis_error(
            "do/\ncons/\n1\nempty-list/\nF64\n/empty-list\n/cons\n/do\n"
        )
        .contains("type parameter T conflict"));
        assert!(analysis_error("do/\nnull?/\nempty-list/\nI64\n/empty-list\n/null?\n/do\n")
            .contains("unknown call null?"));
    }

    #[test]
    fn operation_names_and_generic_variables_are_resolved_without_capture() {
        assert!(analysis_error("do/\n+\n/do\n").contains("not a first-class value"));
        let collision = "def/\nname/\nprint\n/name\nfn/\nsig/\nStr\n->\nUnit\n/sig\nparams/\ntext\nStr\n/params\nunit\n/fn\n/def\n";
        assert!(analysis_error(collision).contains("collides with a reserved operation"));

        assert!(analysis_error("do/\ncar/\nnil\n/car\n/do\n")
            .contains("cannot instantiate List(Param(\"T\")) from Nil"));
        assert!(analysis_error("do/\nok/\n1\n/ok\n/do\n")
            .contains("cannot infer type parameter E"));

        let free = "def/\nname/\nf\n/name\nfn/\nsig/\nT\n->\nT\n/sig\nparams/\nx\nT\n/params\nx\n/fn\n/def\n";
        assert!(analysis_error(free).contains("not declared by forall"));
        let free_value = "def/\nname/\nx\n/name\ntype/\nT\n/type\nx\n/def\n";
        assert!(analysis_error(free_value).contains("value type contains unbound parameter T"));
        let duplicate_forall = "def/\nname/\nf\n/name\nfn/\nforall/\nT\nT\n/forall\nsig/\nT\n->\nT\n/sig\nparams/\nx\nT\n/params\nx\n/fn\n/def\n";
        assert!(analysis_error(duplicate_forall).contains("duplicate forall variable T"));
    }

    #[test]
    fn duplicate_headers_bindings_and_mismatched_parameter_types_are_rejected() {
        let duplicate_let = "do/\nlet/\nbind/\nx\n1\n/bind\nbind/\nx\n2\n/bind\nx\n/let\n/do\n";
        assert!(analysis_error(duplicate_let).contains("duplicate let binding x"));

        let duplicate_sig = "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\nsig/\n->\nUnit\n/sig\nparams/\n/params\nunit\n/fn\n/def\n";
        assert!(analysis_error(duplicate_sig).contains("multiple sig blocks"));

        let mismatch = "def/\nname/\nf\n/name\nfn/\nsig/\nBuf\n->\nBuf\n/sig\nparams/\nx\nNil\n/params\nx\n/fn\n/def\n";
        assert!(analysis_error(mismatch).contains("parameter type mismatch"));

        assert!(analysis_error("do/\nquote/\n1\n/quote\n/do\n")
            .contains("quote accepts only a symbol"));
        assert!(analysis_error("do/\nquote/\nitem/\n/item\n/quote\n/do\n")
            .contains("quote accepts only a symbol"));
    }

    #[test]
    fn user_calls_have_conservative_effects_until_fixed_point_summaries_land() {
        let source = "def/\nname/\nf\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\n1\n/fn\n/def\ndo/\nf/\n/f\n/do\n";
        let program = analyze_one(source).expect("analyze call effects");
        let TopLevel::Do { expression, .. } = &program.forms[1] else {
            panic!("expected do");
        };
        let ExprKind::Do(expressions) = &expression.kind else {
            panic!("expected do expression");
        };
        assert_eq!(expressions[0].effects, EffectSet::CONSERVATIVE_CALL);
    }

    #[test]
    fn strict_if_and_quoted_names_are_exact() {
        assert!(analysis_error("do/\nif/\ntrue\n7\n/if\n/do\n")
            .contains("if expects condition, then, and else"));
        assert!(analysis_error("do/\nif/\ntrue\n7\nunit\n/if\n/do\n")
            .contains("if branches must have the same type"));

        let source = "do/\nif/\ntrue\n7\n8\n/if\nquote/\nnot-a-binding\n/quote\n/do\n";
        let program = analyze_one(source).expect("analyze strict forms");
        let TopLevel::Do { expression, .. } = &program.forms[0] else {
            panic!("expected do");
        };
        let ExprKind::Do(expressions) = &expression.kind else {
            panic!("expected do expression");
        };
        let ExprKind::If { else_branch, .. } = &expressions[0].kind else {
            panic!("expected if");
        };
        assert_eq!(else_branch.ty, Type::I64);
        assert_eq!(expressions[0].ty, Type::I64);
        assert_eq!(
            expressions[1].kind,
            ExprKind::QuoteSymbol("not-a-binding".into())
        );
        assert_eq!(expressions[1].ty, Type::Symbol);

        let chunk = compile_program(&program).expect("lower compatibility forms");
        let mut offset = 0;
        let mut decoded = Vec::new();
        while let Some(byte) = chunk.main.code.get(offset) {
            let operation = Op::from_byte(*byte).expect("known bytecode operation");
            decoded.push(operation);
            offset += 1 + operation.operand_width();
        }
        assert!(decoded.contains(&Op::JumpIfFalse));
        assert!(decoded.contains(&Op::Unit));
    }
}
