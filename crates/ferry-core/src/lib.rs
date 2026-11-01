pub mod analyzer;
#[rustfmt::skip]
mod builtin;
#[rustfmt::skip]
mod arity;
pub mod deps;
pub mod diff;
pub mod extensions;
#[rustfmt::skip]
mod ext_map;
pub mod framework;
mod guards;
pub mod history;
pub mod i18n;
pub mod ignore;
pub mod index;
pub mod lexer;
pub mod packagist;
pub mod probe;
pub mod project;
pub mod rector;
pub mod report;
pub mod rules;
pub mod runtime;
pub mod server;
pub mod snapshot;
pub mod toolchain;
pub mod version;

pub use version::PhpVersion;
