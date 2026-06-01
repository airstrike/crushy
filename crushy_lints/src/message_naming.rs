use crushy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{Item, ItemKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_crushy_lint! {
    /// ### What it does
    /// Flags `enum` definitions whose name is the `Msg` abbreviation, or carries
    /// `Message` / `Msg` as an affix (e.g. `AppMessage`, `MessageKind`, `FooMsg`).
    /// The canonical iced message enum is named exactly `Message`.
    ///
    /// ### Why is this bad?
    /// iced's architecture expects each module to own a `Message` enum, referred
    /// to from elsewhere as `module::Message`. The module path already
    /// disambiguates, so an affix like `App` in `AppMessage` is redundant, and
    /// the `Msg` abbreviation is just noise. Naming every message enum `Message`
    /// keeps the convention uniform and grep-able.
    ///
    /// ### Example
    /// ```rust,ignore
    /// enum Msg { /* ... */ }
    /// enum AppMessage { /* ... */ }
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// enum Message { /* ... */ }
    /// ```
    #[crushy::version = "0.1.0"]
    pub MESSAGE_NAMING,
    style,
    "iced message enum named something other than `Message`"
}

declare_lint_pass!(MessageNaming => [MESSAGE_NAMING]);

impl EarlyLintPass for MessageNaming {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        let ItemKind::Enum(ident, ..) = &item.kind else {
            return;
        };
        let name = ident.as_str();
        // `Message` itself is the canonical name; everything else carrying the
        // `Message`/`Msg` smell is flagged.
        if name == "Message" || !is_message_smell(name) {
            return;
        }
        span_lint_and_help(
            cx,
            MESSAGE_NAMING,
            ident.span,
            format!("iced message enums should be named `Message`, not `{name}`"),
            None,
            "rename it to `Message`; refer to it as `module::Message` from other modules",
        );
    }
}

/// Whether an enum name reads as an iced message: the `Msg` abbreviation or a
/// name carrying `Message`/`Msg` as an affix (`FooMessage`, `MessageFoo`, `FooMsg`).
fn is_message_smell(name: &str) -> bool {
    name.contains("Message") || name.contains("Msg")
}
