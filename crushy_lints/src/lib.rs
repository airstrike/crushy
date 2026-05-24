#![feature(macro_metavar_expr_concat)]
#![feature(rustc_private)]
#![recursion_limit = "512"]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications
)]

extern crate rustc_ast;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

#[macro_use]
extern crate declare_crushy_lint;

use crushy_config::{Conf, get_configuration_metadata, sanitize_explanation};

pub mod declared_lints;
pub mod deprecated_lints;

mod length_fill;
mod use_as_rename;

pub fn explain(name: &str) -> i32 {
    let target = format!("crushy::{}", name.to_ascii_uppercase());

    if let Some(info) = declared_lints::LINTS.iter().find(|info| info.lint.name == target) {
        println!("{}", sanitize_explanation(info.explanation));
        let mut mdconf = get_configuration_metadata();
        let name = name.to_ascii_lowercase();
        mdconf.retain(|cconf| cconf.lints.contains(&&*name));
        if !mdconf.is_empty() {
            println!("### Configuration for {}:\n", info.lint.name_lower());
            for conf in mdconf {
                println!("{conf}");
            }
        }
        0
    } else {
        println!("unknown lint: {name}");
        1
    }
}

pub fn register_lint_passes(store: &mut rustc_lint::LintStore, _conf: &'static Conf) {
    for (old_name, new_name) in deprecated_lints::RENAMED {
        store.register_renamed(old_name, new_name);
    }
    for (name, reason) in deprecated_lints::DEPRECATED {
        store.register_removed(name, reason);
    }

    store.register_early_pass(|| Box::new(length_fill::LengthFill));
    store.register_early_pass(|| Box::new(use_as_rename::UseAsRename));
}
