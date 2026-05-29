use crushy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{Expr, ExprKind, Pat, PatKind, Path, QSelf, Ty, TyKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::symbol::kw;

declare_crushy_lint! {
    /// ### What it does
    /// Flags inline paths with four or more segments (three or more `::`),
    /// e.g. `some::deeply::nested::name::call_site()`.
    ///
    /// ### Why is this bad?
    /// Deeply-qualified inline paths are noise at the call site and hide a crate's
    /// dependency surface. Bringing the item into scope with a `use` import keeps
    /// call sites short and the imports visible at the top of the file.
    ///
    /// Turbofish generics (`::<T>`) don't count as segments, and macro-generated
    /// paths (`$crate::...`) are skipped.
    ///
    /// ### Example
    /// ```rust,ignore
    /// some::deeply::nested::name::call_site();
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// use some::deeply::nested::name::call_site;
    ///
    /// call_site();
    /// ```
    #[crushy::version = "0.1.0"]
    pub DEEP_PATH,
    style,
    "inline path with 4+ segments that should be brought into scope with `use`"
}

declare_lint_pass!(DeepPath => [DEEP_PATH]);

/// Maximum number of segments allowed inline. `a::b::c` is fine; a fourth
/// segment trips the lint. Matches the `\w+::\w+::\w+::\w+` pre-commit rule.
const MAX_SEGMENTS: usize = 3;

impl EarlyLintPass for DeepPath {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        match &expr.kind {
            ExprKind::Path(qself, path) => check_path(cx, qself, path),
            ExprKind::Struct(s) => check_path(cx, &s.qself, &s.path),
            _ => {},
        }
    }

    fn check_ty(&mut self, cx: &EarlyContext<'_>, ty: &Ty) {
        if let TyKind::Path(qself, path) = &ty.kind {
            check_path(cx, qself, path);
        }
    }

    fn check_pat(&mut self, cx: &EarlyContext<'_>, pat: &Pat) {
        match &pat.kind {
            PatKind::Path(qself, path)
            | PatKind::Struct(qself, path, ..)
            | PatKind::TupleStruct(qself, path, ..) => check_path(cx, qself, path),
            _ => {},
        }
    }
}

fn check_path(cx: &EarlyContext<'_>, qself: &Option<Box<QSelf>>, path: &Path) {
    // Skip macro-generated paths — the user can't shorten what they didn't write
    // (this also covers `$crate::...`).
    if path.span.from_expansion() {
        return;
    }
    // For a qualified-self path `<T as a::b::Trait>::C::d`, the `a::b::Trait`
    // segments are structural disambiguation a `use` can't collapse — only the
    // segments after `>::` are shortenable, so start counting from there.
    let start = qself.as_ref().map_or(0, |q| q.position);
    // The leading `{{root}}` segment of a global `::a::b` path isn't a real
    // segment; exclude it so counting matches the `\w+::\w+::...` regex.
    let segments = path.segments[start..]
        .iter()
        .filter(|seg| seg.ident.name != kw::PathRoot)
        .count();
    if segments > MAX_SEGMENTS {
        span_lint_and_help(
            cx,
            DEEP_PATH,
            path.span,
            "deeply-nested path; bring it into scope with a `use` import",
            None,
            "import the item and refer to it by its final segment(s)",
        );
    }
}
