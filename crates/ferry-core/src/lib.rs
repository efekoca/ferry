pub mod analyzer;
#[rustfmt::skip]
mod builtin;
pub mod deps;
pub mod diff;
pub mod framework;
mod guards;
pub mod i18n;
pub mod index;
pub mod lexer;
pub mod project;
pub mod rector;
pub mod report;
pub mod rules;
pub mod snapshot;
pub mod toolchain;
pub mod version;

pub use version::PhpVersion;
