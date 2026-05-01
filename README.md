# RJSI - Rust JavaScript Interface

RJSI provides a unified, minimal-overhead interface for interacting with different JavaScript engines in Rust and allows swapping them out at build time. Additionally, a default set of Web APIs is provided compatible with all engines.

## Supported Engines

RJSI currently supports V8, QuickJS, JavaScriptCore, Boa and Hermes. JavaScriptCore currently only compiles on macOS and iOS and will be dynamically linked.
