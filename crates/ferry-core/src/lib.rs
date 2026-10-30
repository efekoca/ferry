pub mod analyzer;
pub mod deps;
pub mod diff;
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
