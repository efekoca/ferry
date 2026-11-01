pub mod analyzer;
#[rustfmt::skip]
mod builtin;
pub mod deps;
pub mod diff;
pub mod extensions;
#[rustfmt::skip]
mod ext_map;
pub mod framework;
mod guards;
pub mod i18n;
pub mod ignore;
pub mod index;
pub mod lexer;
pub mod packagist;
pub mod project;
pub mod rector;
pub mod report;
pub mod rules;
pub mod server;
pub mod snapshot;
pub mod toolchain;
pub mod version;

pub use version::PhpVersion;
