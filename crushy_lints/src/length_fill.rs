use crushy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{Expr, ExprKind, Path};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_crushy_lint! {
    /// ### What it does
    /// Flags `Length::Fill` and `Length::Fixed(_)` when passed as a call argument
    /// (e.g. `.width(Length::Fill)`). Other positions — struct fields, locals,
    /// `match` arms — are left alone, since the `Length` wrapper is unavoidable there.
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
        // Only flag `Length::Fill` / `Length::Fixed(_)` when passed as a call
        // argument (e.g. `.width(Length::Fill)`). This avoids tripping on struct
        // fields, locals, and other positions where the wrapper is unavoidable.
        let args = match &expr.kind {
            ExprKind::Call(_, args) => args,
            ExprKind::MethodCall(call) => &call.args,
            _ => return,
        };
        for arg in args {
            check_arg(cx, arg);
        }
    }
}

fn check_arg(cx: &EarlyContext<'_>, arg: &Expr) {
    match &arg.kind {
        ExprKind::Path(_, path) if last_two(path, "Length", "Fill") => {
            emit(
                cx,
                arg.span,
                "use `Fill` from `iced` instead of `Length::Fill`",
                "import `iced::Fill` and pass `Fill` directly",
            );
        },
        ExprKind::Call(callee, _) => {
            if let ExprKind::Path(_, path) = &callee.kind
                && last_two(path, "Length", "Fixed")
            {
                emit(
                    cx,
                    arg.span,
                    "use a bare number instead of `Length::Fixed(_)`",
                    "iced length-accepting methods take `impl Into<Length>`, so e.g. `.width(20)` works",
                );
            }
        },
        _ => {},
    }
}

fn emit(cx: &EarlyContext<'_>, span: Span, msg: &'static str, help: &'static str) {
    span_lint_and_help(cx, LENGTH_FILL, span, msg, None, help);
}

fn last_two(path: &Path, penultimate: &str, last: &str) -> bool {
    let segs = &path.segments;
    let n = segs.len();
    n >= 2 && segs[n - 2].ident.as_str() == penultimate && segs[n - 1].ident.as_str() == last
}
