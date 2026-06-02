use crushy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{Expr, ExprKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::Span;

declare_crushy_lint! {
    /// ### What it does
    /// Flags an inline `container(content)` whose `Fill` alignment chain is
    /// exactly what an `iced::widget` positional helper already builds — e.g.
    /// `container(x).center_x(Fill).center_y(Fill)` is just `center(x)`.
    ///
    /// ### Why is this bad?
    /// iced ships `center`, `center_x`, `center_y`, `right`, `right_center`,
    /// `bottom`, `bottom_center`, and `bottom_right` for exactly these
    /// fill-and-align patterns. The helper is shorter and states the intent
    /// directly. (There are no `top`/`left`/corner-top helpers because a
    /// container already defaults to top-left.)
    ///
    /// ### Example
    /// ```rust,ignore
    /// container(x).center_x(Fill).center_y(Fill)
    /// container(x).align_right(Fill).align_bottom(Fill)
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// center(x)
    /// bottom_right(x)
    /// ```
    #[crushy::version = "0.1.0"]
    pub CONTAINER_ALIGN_HELPER,
    style,
    "a `container(_)` fill-alignment chain that an `iced::widget` helper expresses directly"
}

#[derive(Default)]
pub struct ContainerAlignHelper {
    /// Spans already flagged. A maximal chain (`.center_x().center_y()` →
    /// `center`) is visited before its sub-chains (pre-order), so recording its
    /// span lets us skip the shorter `.center_x()` match nested inside it.
    emitted: Vec<Span>,
}

impl_lint_pass!(ContainerAlignHelper => [CONTAINER_ALIGN_HELPER]);

impl EarlyLintPass for ContainerAlignHelper {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if self.emitted.iter().any(|s| s.contains(expr.span)) {
            return;
        }
        let Some(methods) = container_align_chain(expr) else {
            return;
        };
        let Some(helper) = helper_for(&methods) else {
            return;
        };
        self.emitted.push(expr.span);
        span_lint_and_help(
            cx,
            CONTAINER_ALIGN_HELPER,
            expr.span,
            format!("this `container(…)` alignment chain is `iced::widget::{helper}`"),
            None,
            format!("replace it with `{helper}(…)`"),
        );
    }
}

/// One recognized fill-alignment method on a `Container`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Center,
    CenterX,
    CenterY,
    Right,
    Bottom,
}

impl Align {
    fn from_method(name: &str) -> Option<Self> {
        Some(match name {
            "center" => Align::Center,
            "center_x" => Align::CenterX,
            "center_y" => Align::CenterY,
            "align_right" => Align::Right,
            "align_bottom" => Align::Bottom,
            _ => return None,
        })
    }
}

/// The fill-alignment methods chained directly onto an inline `container(_)`,
/// or `None` if `expr` isn't such a chain (every method must be a recognized
/// alignment method taking a single `Fill`, and the base must be `container(x)`).
fn container_align_chain(expr: &Expr) -> Option<Vec<Align>> {
    let mut methods = Vec::new();
    let mut e = expr;
    loop {
        match &e.kind {
            ExprKind::MethodCall(call) => {
                let align = Align::from_method(call.seg.ident.as_str())?;
                if call.args.len() != 1 || !is_fill(&call.args[0]) {
                    return None;
                }
                methods.push(align);
                e = &call.receiver;
            },
            ExprKind::Call(callee, args) => {
                let is_container = path_tail(callee) == Some("container") && args.len() == 1;
                return (is_container && !methods.is_empty()).then_some(methods);
            },
            _ => return None,
        }
    }
}

/// The `iced::widget` helper equivalent to a set of fill-alignment methods, if any.
fn helper_for(methods: &[Align]) -> Option<&'static str> {
    use Align::{Bottom, Center, CenterX, CenterY, Right};
    let n = methods.len();
    let has = |a: Align| methods.contains(&a);
    // `.center(Fill)` is the single-method form of `center`.
    if has(Center) {
        return (n == 1).then_some("center");
    }
    Some(match (has(CenterX), has(CenterY), has(Right), has(Bottom), n) {
        (true, true, false, false, 2) => "center",
        (true, false, false, false, 1) => "center_x",
        (false, true, false, false, 1) => "center_y",
        (false, false, true, false, 1) => "right",
        (false, false, false, true, 1) => "bottom",
        (false, true, true, false, 2) => "right_center",
        (true, false, false, true, 2) => "bottom_center",
        (false, false, true, true, 2) => "bottom_right",
        _ => return None,
    })
}

fn is_fill(e: &Expr) -> bool {
    path_tail(e) == Some("Fill")
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
    use super::Align::{Bottom, Center, CenterX, CenterY, Right};
    use super::helper_for;

    #[test]
    fn families() {
        assert_eq!(helper_for(&[Center]), Some("center"));
        assert_eq!(helper_for(&[CenterX, CenterY]), Some("center"));
        assert_eq!(helper_for(&[CenterY, CenterX]), Some("center")); // order-free
        assert_eq!(helper_for(&[CenterX]), Some("center_x"));
        assert_eq!(helper_for(&[CenterY]), Some("center_y"));
        assert_eq!(helper_for(&[Right]), Some("right"));
        assert_eq!(helper_for(&[Bottom]), Some("bottom"));
        assert_eq!(helper_for(&[Right, CenterY]), Some("right_center"));
        assert_eq!(helper_for(&[CenterX, Bottom]), Some("bottom_center"));
        assert_eq!(helper_for(&[Right, Bottom]), Some("bottom_right"));
    }

    #[test]
    fn non_matches() {
        assert_eq!(helper_for(&[]), None);
        assert_eq!(helper_for(&[Center, CenterX]), None); // center isn't combinable
        assert_eq!(helper_for(&[CenterX, CenterX]), None); // duplicate
        assert_eq!(helper_for(&[CenterY, Bottom]), None); // no center_y+bottom helper
        assert_eq!(helper_for(&[CenterX, CenterY, Right]), None); // no 3-way helper
    }
}
