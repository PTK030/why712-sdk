//! WebAssembly bindings for `why712-core`.
//!
//! The demo page runs the same diagnosis code as the CLI — there is no second implementation
//! and no backend. This crate only translates between JS values and core types, and supplies
//! a `fetch`-based JSON-RPC transport in place of the CLI's native one.

use wasm_bindgen::prelude::wasm_bindgen;

/// Installs a panic hook that reports Rust panics to the browser console.
///
/// Without it a panic surfaces in JS as an opaque `unreachable` trap with no message, which
/// is unusable for the bug reports this page is meant to produce.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}
