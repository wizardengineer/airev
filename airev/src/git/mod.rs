//! Git integration for airev.
//!
//! The git module exposes an `AsyncGit` facade that owns a background
//! `std::thread::spawn` thread. The thread holds the `git2::Repository` for
//! its lifetime — Repository is !Send, so it must never cross a thread boundary.
pub mod types;
pub mod worker;
