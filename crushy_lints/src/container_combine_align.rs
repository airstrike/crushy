use crushy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use rustc_ast::{Expr, ExprKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_crushy_lint! {
    /// ### What it does
    /// Flags a `Container` that sets size and alignment on one axis with two
    /// calls — `.width(L).align_x(Center)` — when a single combining method does
    /// both: `.center_x(L)`. Covers both axes and every alignment.
    ///
    /// ### Why is this bad?
    /// `Container::center_x`/`center_y`/`align_left`/`align_right`/`align_top`/
    /// `align_bottom` each set the length *and* the alignment in one call. The
    /// split form is longer and easy to get half-right. Only fires on an inline
    /// `container(_)` chain, so it never trips on a `column`/`row`
    /// `.width().align_x()` (those have no `center_x`).
    ///
    /// ### Example
    /// ```rust,ignore
    /// container(x).width(Fill).align_x(Center)
    /// container(x).height(200).align_y(Bottom)
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// container(x).center_x(Fill)
    /// container(x).align_bottom(200)
    /// ```
    #[crushy::version = "0.1.0"]
    pub CONTAINER_COMBINE_ALIGN,
    style,
    "separate `.width().align_x()` on a container that a single combining method replaces"
}

declare_lint_pass!(ContainerCombineAlign => [CONTAINER_COMBINE_ALIGN]);

impl EarlyLintPass for ContainerCombineAlign {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        let ExprKind::MethodCall(outer) = &expr.kind else {
            return;
        };
        let ExprKind::MethodCall(inner) = &outer.receiver.kind else {
            return;
        };
        // A size+align pair on one axis, in either order. `align_is_outer` says
        // which of the two calls carries the alignment argument.
        let (axis_x, align_is_outer) = match (outer.seg.ident.as_str(), inner.seg.ident.as_str()) {
            ("align_x", "width") => (true, true),
            ("width", "align_x") => (true, false),
            ("align_y", "height") => (false, true),
            ("height", "align_y") => (false, false),
            _ => return,
        };
        // Anchor to an inline `container(_)` below the pair, so we never suggest
        // container-only methods on a `column`/`row`.
        if !bottoms_at_container(&inner.receiver) {
            return;
        }
        let align_args = if align_is_outer { &outer.args } else { &inner.args };
        let Some(align) = align_args.first().and_then(|a| path_tail(a)) else {
            return;
        };
        let Some(method) = combining_for(axis_x, align) else {
            return;
        };
        let split = if axis_x {
            "width(…).align_x"
        } else {
            "height(…).align_y"
        };
        let msg = format!("`.{split}(…)` on a container is `.{method}(…)`");
        // `center_x`/`align_left`/... are Container methods (no import), so this
        // is a clean machine-applicable rewrite: `<recv>.<method>(<len>)`.
        let length_args = if align_is_outer { &inner.args } else { &outer.args };
        let sm = cx.sess().source_map();
        if let (Ok(recv), Some(len)) = (sm.span_to_snippet(inner.receiver.span), length_args.first())
            && let Ok(len) = sm.span_to_snippet(len.span)
        {
            span_lint_and_sugg(
                cx,
                CONTAINER_COMBINE_ALIGN,
                expr.span,
                msg,
                "combine length and alignment in one call",
                format!("{recv}.{method}({len})"),
                Applicability::MachineApplicable,
            );
        } else {
            span_lint_and_help(
                cx,
                CONTAINER_COMBINE_ALIGN,
                expr.span,
                msg,
                None,
                format!("set length and alignment in one call with `.{method}(…)`"),
            );
        }
    }
}

/// The combining `Container` method for an axis + alignment, if one exists.
fn combining_for(axis_x: bool, align: &str) -> Option<&'static str> {
    Some(match (axis_x, align) {
        (true, "Center") => "center_x",
        (true, "Left") => "align_left",
        (true, "Right") => "align_right",
        (false, "Center") => "center_y",
        (false, "Top") => "align_top",
        (false, "Bottom") => "align_bottom",
        _ => return None,
    })
}

/// Whether the receiver chain bottoms out at an inline `container(content)`.
fn bottoms_at_container(mut e: &Expr) -> bool {
    loop {
        match &e.kind {
            ExprKind::MethodCall(call) => e = &call.receiver,
            ExprKind::Call(callee, args) => return path_tail(callee) == Some("container") && args.len() == 1,
            _ => return false,
        }
    }
}

/// The last segment of `e` if it's a path expression (`a::b::Name` → `Name`).
fn path_tail(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Path(_, path) => path.segments.last().map(|s| s.ident.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::combining_for;

    #[test]
    fn horizontal() {
        assert_eq!(combining_for(true, "Center"), Some("center_x"));
        assert_eq!(combining_for(true, "Left"), Some("align_left"));
        assert_eq!(combining_for(true, "Right"), Some("align_right"));
        assert_eq!(combining_for(true, "Top"), None); // Top isn't a horizontal alignment
        assert_eq!(combining_for(true, "Whatever"), None);
    }

    #[test]
    fn vertical() {
        assert_eq!(combining_for(false, "Center"), Some("center_y"));
        assert_eq!(combining_for(false, "Top"), Some("align_top"));
        assert_eq!(combining_for(false, "Bottom"), Some("align_bottom"));
        assert_eq!(combining_for(false, "Left"), None); // Left isn't a vertical alignment
    }
}
