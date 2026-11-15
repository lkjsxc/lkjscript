use crate::ast::{Path, Span};
use crate::resolve::Resolver;

fn lookup_keeps_file_name_and_enclosing_span_boundaries() {
    let source = crate::syntax::parse("  fn first<T>() {} fn second<U>() {}").unwrap();
    let resolver = Resolver::new(&source).unwrap();
    let range = source.functions[0].span;
    assert!(generic(&resolver, range, "T"));
    assert!(generic(
        &resolver,
        Span {
            end: range.start,
            ..range
        },
        "T"
    ));
    assert!(generic(
        &resolver,
        Span {
            start: range.end,
            ..range
        },
        "T"
    ));
    assert!(!generic(
        &resolver,
        Span {
            start: range.start - 1,
            ..range
        },
        "T"
    ));
    assert!(!generic(
        &resolver,
        Span {
            end: range.end + 1,
            ..range
        },
        "T"
    ));
    assert!(!generic(
        &resolver,
        Span {
            file: range.file + 1,
            ..range
        },
        "T"
    ));
    assert!(!generic(&resolver, range, "U"));
    assert!(!generic(&resolver, range, "TT"));
    assert!(!generic(&resolver, source.functions[1].span, "T"));
    assert!(generic(&resolver, source.functions[1].span, "U"));
    assert!(generic(&resolver, Span::default(), "Self"));
    assert!(generic(&resolver, Span::default(), "'a"));
}

fn nested_method_and_impl_ranges_both_supply_parameters() {
    let source = crate::syntax::parse(
            "struct Holder<T>{value:T} impl<T> Holder<T>{fn pair<U>(self,extra:U)->(T,U){(self.value,extra)}}",
        ).unwrap();
    let resolver = Resolver::new(&source).unwrap();
    let method = source.functions[0].span;
    let implementation = source.impls[0].span;
    assert!(generic(&resolver, method, "T"));
    assert!(generic(&resolver, method, "U"));
    assert!(generic(&resolver, implementation, "T"));
    assert!(!generic(&resolver, implementation, "U"));
}

fn generic(resolver: &Resolver<'_>, span: Span, name: &str) -> bool {
    resolver.generic(span, name)
}
pub fn run() -> usize {
    lookup_keeps_file_name_and_enclosing_span_boundaries();
    nested_method_and_impl_ranges_both_supply_parameters();
    2
}
