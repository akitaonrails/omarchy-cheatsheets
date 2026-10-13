//! Integration test suite: one test binary per crate (house style).

// One integration test binary per crate (house style): each suite file is
// declared here instead of living as a top-level tests/*.rs target.

mod cli;
mod install;
mod load_merge;
mod quick_search;
mod resolve;
mod settings;
