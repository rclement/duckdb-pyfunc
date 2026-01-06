//! WASM library wrapper
//!
//! This file provides a staticlib entry point for Emscripten/WASM builds.
//! It includes the main library directly to work around crate-type limitations.

// Include the main lib.rs directly for WASM builds
include!("lib.rs");
