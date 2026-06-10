use crushy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use rustc_ast::{Expr, ExprKind, Path};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_crushy_lint! {
    /// ### What it does
    /// Flags `Length::Fill` when passed as a call argument (e.g.
    /// `.width(Length::Fill)`). Other positions — struct fields, locals,
    /// `match` arms — are left alone, since the `Length` wrapper is unavoidable there.
    ///
    /// ### Why is this bad?
    /// iced re-exports `Fill` (and `Shrink`) at the crate root, and length-accepting
    /// methods take `impl Into<Length>`. `Length::Fill` is a noisy wrapper around
    /// what should be a bare `Fill`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// container.width(Length::Fill);
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// container.width(Fill);
    /// ```
    #[crushy::version = "0.1.0"]
    pub LENGTH_FILL,
    style,
    "use of `Length::Fill` instead of iced's re-exported `Fill`"
}

declare_crushy_lint! {
    /// ### What it does
    /// Flags `Length::Fixed(_)` when passed as a call argument (e.g.
    /// `.width(Length::Fixed(20.0))`). Other positions — struct fields, locals,
    /// `match` arms — are left alone, since the `Length` wrapper is unavoidable there.
    ///
    /// ### Why is this bad?
    /// iced's length-accepting methods take `impl Into<Length>`, and `f32: Into<Length>`,
    /// so a bare number works. `Length::Fixed(20.0)` is a noisy wrapper around `20`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// container.width(Length::Fixed(20.0));
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// container.width(20);
    /// ```
    #[crushy::version = "0.1.0"]
    pub LENGTH_FIXED,
    style,
    "use of `Length::Fixed(_)` instead of a bare number"
}

declare_lint_pass!(LengthFill => [LENGTH_FILL, LENGTH_FIXED]);

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
            // `Fill` needs to be imported, so show the rewrite as a suggestion
            // rustc renders but `--fix` won't auto-apply (Unspecified).
            span_lint_and_sugg(
                cx,
                LENGTH_FILL,
                arg.span,
                "use `Fill` from `iced` instead of `Length::Fill`",
                "import `iced::Fill` and use it directly",
                "Fill".to_string(),
                Applicability::Unspecified,
            );
        },
        ExprKind::Call(callee, args) => {
            if let ExprKind::Path(_, path) = &callee.kind
                && last_two(path, "Length", "Fixed")
                && let [inner] = &args[..]
            {
                let msg = "use a bare number instead of `Length::Fixed(_)`";
                // `Length::Fixed(x)` -> `x`: iced length methods take
                // `impl Into<Length>` and `f32: Into<Length>`, so the inner
                // number stands on its own. Machine-applicable.
                match cx.sess().source_map().span_to_snippet(inner.span) {
                    Ok(number) => span_lint_and_sugg(
                        cx,
                        LENGTH_FIXED,
                        arg.span,
                        msg,
                        "use the number directly",
                        number,
                        Applicability::MachineApplicable,
                    ),
                    Err(_) => emit(
                        cx,
                        arg.span,
                        msg,
                        "iced length-accepting methods take `impl Into<Length>`, so e.g. `.width(20)` works",
                    ),
                }
            }
        },
        _ => {},
    }
}

fn emit(cx: &EarlyContext<'_>, span: Span, msg: &'static str, help: &'static str) {
    span_lint_and_help(cx, LENGTH_FIXED, span, msg, None, help);
}

fn last_two(path: &Path, penultimate: &str, last: &str) -> bool {
    let segs = &path.segments;
    let n = segs.len();
    n >= 2 && segs[n - 2].ident.as_str() == penultimate && segs[n - 1].ident.as_str() == last
}
