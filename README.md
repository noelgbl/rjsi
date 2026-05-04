# RJSI - Rust JavaScript Interface

RJSI provides a unified, minimal-overhead interface for interacting with different JavaScript engines in Rust and allows swapping them out at build time. Additionally, a default set of Web APIs is provided compatible with all engines.

## Why?

Different JavaScript engines serve a different purpose and have different strengths and weaknesses. For example, V8 and JSC are powerful engines for embedding in desktop applications and servers, while lightweight engines like QuickJS and Hermes are better suited for mobile applications and embedded systems due to their smaller footprint. Writing native extensions for each engine is time-consuming and error-prone and requires a lot of understanding of the engine's internals. RJSI lets you write these extensions once and choose the suitable engine for different deployment targets. It builds on the ideas of Facebook's JSI used in React Native.

## Supported Engines

RJSI currently supports V8, QuickJS, JavaScriptCore, Boa and Hermes. JavaScriptCore currently only compiles on macOS and iOS and will be dynamically linked.
