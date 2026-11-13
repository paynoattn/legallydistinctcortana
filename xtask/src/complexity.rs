//! Cyclomatic complexity per function, from the syntax tree.
//!
//! Counting rule: start at 1, then add 1 for each `if` (so each `else if`), `while`,
//! `for`, `&&`, `||`, `let ... else`, match arm after the first, and match guard.
//! Not counted: `loop` (its exits are the `if`/`match` around `break`, already counted),
//! `?` (an early return, like clippy's cognitive complexity), and code inside macro
//! calls (`syn` sees macro input as opaque tokens).
//!
//! Closures count toward the function that contains them. Nested `fn` items are scored
//! on their own. Test code (`#[cfg(test)]` modules and items, `#[test]` functions) is
//! skipped.

use proc_macro2::Span;
use quote::ToTokens;
use syn::visit::{self, Visit};
use syn::{Attribute, BinOp, Block, Signature};

/// One function or method found in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) first_line: usize,
    pub(crate) last_line: usize,
    pub(crate) complexity: u32,
}

/// Every non-test function in `source`, in source order.
pub(crate) fn functions(source: &str) -> syn::Result<Vec<Function>> {
    let file = syn::parse_file(source)?;
    let mut collector = Collector::default();
    collector.visit_file(&file);
    Ok(collector.functions)
}

#[derive(Default)]
struct Collector {
    functions: Vec<Function>,
    owner: Vec<String>,
}

impl Collector {
    fn record(&mut self, sig: &Signature, body: &Block) {
        let name = match self.owner.last() {
            Some(owner) => format!("{owner}::{}", sig.ident),
            None => sig.ident.to_string(),
        };
        self.functions.push(Function {
            name,
            first_line: line(sig.fn_token.span),
            last_line: line(body.brace_token.span.close()),
            complexity: Counter::complexity_of(body),
        });
    }

    fn in_owner(&mut self, owner: String, walk: impl FnOnce(&mut Self)) {
        self.owner.push(owner);
        walk(self);
        self.owner.pop();
    }
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !is_test_code(&node.attrs) {
            visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if is_test_code(&node.attrs) {
            return;
        }
        let owner = node.self_ty.to_token_stream().to_string().replace(' ', "");
        self.in_owner(owner, |this| visit::visit_item_impl(this, node));
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if !is_test_code(&node.attrs) {
            self.in_owner(node.ident.to_string(), |this| {
                visit::visit_item_trait(this, node);
            });
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !is_test_code(&node.attrs) {
            self.record(&node.sig, &node.block);
            visit::visit_item_fn(self, node);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if !is_test_code(&node.attrs) {
            self.record(&node.sig, &node.block);
            visit::visit_impl_item_fn(self, node);
        }
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(body) = node.default.as_ref().filter(|_| !is_test_code(&node.attrs)) {
            self.record(&node.sig, body);
            visit::visit_trait_item_fn(self, node);
        }
    }
}

struct Counter {
    complexity: u32,
}

impl Counter {
    fn complexity_of(body: &Block) -> u32 {
        let mut counter = Self { complexity: 1 };
        counter.visit_block(body);
        counter.complexity
    }

    fn add(&mut self, decisions: usize) {
        let decisions = u32::try_from(decisions).unwrap_or(u32::MAX);
        self.complexity = self.complexity.saturating_add(decisions);
    }
}

impl<'ast> Visit<'ast> for Counter {
    fn visit_item(&mut self, _nested: &'ast syn::Item) {}

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.add(1);
        visit::visit_expr_if(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.add(1);
        visit::visit_expr_while(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.add(1);
        visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        let guards = node
            .arms
            .iter()
            .filter(|arm| matches!(arm.pat, syn::Pat::Guard(_)))
            .count();
        self.add(node.arms.len().saturating_sub(1) + guards);
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(node.op, BinOp::And(_) | BinOp::Or(_)) {
            self.add(1);
        }
        visit::visit_expr_binary(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if node
            .init
            .as_ref()
            .is_some_and(|init| init.diverge.is_some())
        {
            self.add(1);
        }
        visit::visit_local(self, node);
    }
}

fn line(span: Span) -> usize {
    span.start().line
}

fn is_test_code(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let is_test_attr = attr
            .path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "test");
        is_test_attr || is_cfg_test(attr)
    })
}

fn is_cfg_test(attr: &Attribute) -> bool {
    attr.path().is_ident("cfg")
        && attr
            .meta
            .require_list()
            .is_ok_and(|list| list.tokens.to_string() == "test")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complexity(source: &str) -> Vec<(String, u32)> {
        functions(source)
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.complexity))
            .collect()
    }

    #[test]
    fn straight_line_code_is_one() {
        assert_eq!(
            complexity("fn a() { let x = 1; x + 1; }"),
            vec![("a".into(), 1)]
        );
    }

    #[test]
    fn counts_branches_loops_and_boolean_operators() {
        let source = "fn a(x: i32) {
            if x > 0 && x < 9 { } else if x == 0 || x == 1 { } else { }
            while x > 0 { }
            for _ in 0..x { }
            loop { break; }
        }";
        assert_eq!(complexity(source), vec![("a".into(), 7)]);
    }

    #[test]
    fn counts_match_arms_after_the_first_and_guards() {
        let source = "fn a(x: i32) { match x { 0 => {}, n if n > 5 => {}, _ => {} } }";
        assert_eq!(complexity(source), vec![("a".into(), 4)]);
    }

    #[test]
    fn counts_let_else_and_ignores_question_mark() {
        let source = "fn a(x: Option<i32>) -> Option<i32> { let Some(y) = x else { return None }; x? ; Some(y) }";
        assert_eq!(complexity(source), vec![("a".into(), 2)]);
    }

    #[test]
    fn closures_count_toward_their_function_and_nested_fns_stand_alone() {
        let source = "fn outer() { let f = |x: i32| if x > 0 { 1 } else { 2 }; fn inner(y: bool) { if y {} } }";
        assert_eq!(
            complexity(source),
            vec![("outer".into(), 2), ("inner".into(), 2)]
        );
    }

    #[test]
    fn names_methods_by_their_type_or_trait() {
        let source = "impl Foo<T> { fn a(&self) {} } trait Bar { fn b(&self) { } fn c(&self); }";
        assert_eq!(
            complexity(source),
            vec![("Foo<T>::a".into(), 1), ("Bar::b".into(), 1)]
        );
    }

    #[test]
    fn skips_test_code() {
        let source = "fn real() {}
            #[cfg(test)] mod tests { fn helper() {} }
            #[test] fn a_test() {}
            #[tokio::test] async fn an_async_test() {}
            #[cfg(test)] impl Foo { fn only_in_tests() {} }";
        assert_eq!(complexity(source), vec![("real".into(), 1)]);
    }

    #[test]
    fn records_the_line_span_from_fn_keyword_to_closing_brace() {
        let found = functions("\n/// doc\nfn a() {\n    let x = 1;\n}\n").unwrap();
        assert_eq!((found[0].first_line, found[0].last_line), (3, 5));
    }
}
