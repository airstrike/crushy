use crushy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{Item, ItemKind, UseTree, UseTreeKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_crushy_lint! {
    /// ### What it does
    /// Flags `use` imports of an item named `Message` from another module
    /// (e.g. `use crate::settings::Message;`). Referring to it by path —
    /// `settings::Message` — is fine; only the import is flagged.
    ///
    /// ### Why is this bad?
    /// In iced's Elm-style architecture each module owns its own `Message`
    /// enum, and a parent composes children by mapping their messages
    /// (`child.update(..).map(Message::Child)`). Importing another module's
    /// `Message` collapses two distinct message types into one ambiguous
    /// `Message` at the use site and invites reaching past the boundary.
    /// Keep `Message` local; reference a foreign one by its module path.
    ///
    /// ### Example
    /// ```rust,ignore
    /// use crate::settings::Message;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // reference it by path where needed
    /// settings::Message::Saved
    /// ```
    #[crushy::version = "0.1.0"]
    pub IMPORTED_MESSAGE,
    style,
    "importing a `Message` enum from another module instead of referencing it by path"
}

declare_lint_pass!(ImportedMessage => [IMPORTED_MESSAGE]);

impl EarlyLintPass for ImportedMessage {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Use(use_tree) = &item.kind {
            check_use_tree(cx, use_tree);
        }
    }
}

fn check_use_tree(cx: &EarlyContext<'_>, tree: &UseTree) {
    match &tree.kind {
        // `use path::Message;` / `use path::Message as Alias;` — the imported
        // item is the last segment of this tree's prefix. A rename still imports
        // it, so flag regardless. Glob (`use path::Message::*`) brings *variants*
        // into scope, a different thing, and is left alone.
        UseTreeKind::Simple(_) => {
            if let Some(seg) = tree.prefix.segments.last()
                && seg.ident.as_str() == "Message"
            {
                span_lint_and_help(
                    cx,
                    IMPORTED_MESSAGE,
                    seg.ident.span,
                    "`Message` enums should stay local; don't import one from another module",
                    None,
                    "reference it by path instead, e.g. `module::Message`",
                );
            }
        },
        UseTreeKind::Nested { items, .. } => {
            for (subtree, _) in items {
                check_use_tree(cx, subtree);
            }
        },
        UseTreeKind::Glob(_) => {},
    }
}
