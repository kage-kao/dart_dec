//! dart_dec_profiles — Dart VM version profiles and layout engine.
//!
//! Provides JSON-based profiles describing internal Dart VM struct layouts
//! for each supported version. Handles fuzzy matching for unknown patch versions.

mod resolver;
mod schema;

pub use resolver::ProfileResolver;
pub use schema::*;

#[cfg(test)]
mod tests;
