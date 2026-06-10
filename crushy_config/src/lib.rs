#![feature(rustc_private)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications
)]

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod conf;
mod metadata;
pub mod types;

pub use conf::{Conf, get_configuration_metadata, lookup_conf_file, read_lint_levels, sanitize_explanation};
pub use metadata::CrushyConfiguration;
