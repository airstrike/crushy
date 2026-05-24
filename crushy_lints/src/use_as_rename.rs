use crushy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{Item, ItemKind, UseTree, UseTreeKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::symbol::kw;

declare_crushy_lint! {
    /// ### What it does
    /// Flags `use ... as Name` import aliases. `as _` (trait-in-scope-only
    /// imports) and `as self` (re-exports) are allowed.
    ///
    /// ### Why is this bad?
    /// Aliasing types at the import site hides the canonical name and makes
    /// grep harder. Rename the item at its source, or use a fully qualified
    /// path at the call site.
    ///
    /// ### Example
    /// ```rust,ignore
    /// use some_crate::Foo as Bar;
    /// ```
    /// Use instead: rename `Foo` to `Bar` upstream, or refer to it as
    /// `some_crate::Foo` at the call site.
    #[crushy::version = "0.1.0"]
    pub USE_AS_RENAME,
    restriction,
    "use of `use ... as Name` import alias"
}

declare_lint_pass!(UseAsRename => [USE_AS_RENAME]);

impl EarlyLintPass for UseAsRename {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Use(use_tree) = &item.kind {
            check_use_tree(cx, use_tree);
        }
    }
}

fn check_use_tree(cx: &EarlyContext<'_>, tree: &UseTree) {
    match &tree.kind {
        UseTreeKind::Simple(Some(rename)) => {
            let name = rename.name;
            if name != kw::Underscore && name != kw::SelfLower {
                span_lint_and_help(
                    cx,
                    USE_AS_RENAME,
                    rename.span,
                    "import alias hides the canonical name",
                    None,
                    "rename the item at its source, or use a fully qualified path at the call site",
                );
            }
        },
        UseTreeKind::Nested { items, .. } => {
            for (subtree, _) in items {
                check_use_tree(cx, subtree);
            }
        },
        UseTreeKind::Simple(None) | UseTreeKind::Glob(_) => {},
    }
}
