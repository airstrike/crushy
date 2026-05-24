use crushy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{Expr, ExprKind, Path};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_crushy_lint! {
    /// ### What it does
    /// Flags `Length::Fill` and `Length::Fixed(_)` in iced layout calls.
    ///
    /// ### Why is this bad?
    /// iced re-exports `Fill` and `Shrink` at the crate root, and length-accepting
    /// methods take `impl Into<Length>` — so bare numbers work too. `Length::Fill`
    /// and `Length::Fixed(20.0)` are noisy wrappers around what should be `Fill`
    /// and `20`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// container.width(Length::Fill);
    /// container.width(Length::Fixed(20.0));
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// container.width(Fill);
    /// container.width(20);
    /// ```
    #[crushy::version = "0.1.0"]
    pub LENGTH_FILL,
    style,
    "use of `Length::Fill` or `Length::Fixed(_)` instead of iced's `Fill`/`Shrink` or bare numbers"
}

declare_lint_pass!(LengthFill => [LENGTH_FILL]);

impl EarlyLintPass for LengthFill {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        match &expr.kind {
            ExprKind::Path(_, path) if last_two(path, "Length", "Fill") => {
                span_lint_and_help(
                    cx,
                    LENGTH_FILL,
                    expr.span,
                    "use `Fill` from `iced` instead of `Length::Fill`",
                    None,
                    "import `iced::Fill` and pass `Fill` directly",
                );
            },
            ExprKind::Call(callee, _) => {
                if let ExprKind::Path(_, path) = &callee.kind
                    && last_two(path, "Length", "Fixed")
                {
                    span_lint_and_help(
                        cx,
                        LENGTH_FILL,
                        expr.span,
                        "use a bare number instead of `Length::Fixed(_)`",
                        None,
                        "iced length-accepting methods take `impl Into<Length>`, so e.g. `.width(20)` works",
                    );
                }
            },
            _ => {},
        }
    }
}

fn last_two(path: &Path, penultimate: &str, last: &str) -> bool {
    let segs = &path.segments;
    let n = segs.len();
    n >= 2
        && segs[n - 2].ident.as_str() == penultimate
        && segs[n - 1].ident.as_str() == last
}
