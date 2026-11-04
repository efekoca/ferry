pub mod analyzer;
pub mod arguments;
mod calls;
mod flow;
#[rustfmt::skip]
mod builtin;
#[rustfmt::skip]
mod arity;
pub mod compare;
mod declarations;
pub mod deps;
pub mod diff;
pub mod docker;
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
pub mod localcheck;
mod objects;
pub mod packagist;
pub mod probe;
pub mod project;
pub mod rector;
pub mod report;
pub mod rules;
pub mod runtime;
pub mod safefs;
pub mod server;
pub mod snapshot;
mod strings;
mod syntax;
#[rustfmt::skip]
pub mod symbols;
pub mod toolchain;
pub mod version;
pub mod web;
pub mod wordpress;

pub use version::PhpVersion;
