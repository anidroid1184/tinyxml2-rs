# WASM and no_std Support

The core `tinyxml2` crate supports `no_std` builds with `alloc` and keeps the
standard library enabled by default for backward compatibility.

```toml
[dependencies]
tinyxml2 = { version = "1", default-features = false }
```

Use the default feature set when an application needs file loading, file saving,
or `std::io::Write` output helpers. Disable default features for embedded,
kernel, and WebAssembly environments where parsing happens from memory.

## Supported Without std

These APIs are available with `default-features = false`:

- Parsing from memory with `Document::parse`, `Document::parse_str`,
  `Document::parse_bytes`, and `Document::parse_bytes_mut`.
- DOM creation, mutation, traversal, handles, references, and iterators.
- Entity decoding and encoding.
- Visitor traversal.
- Serialization to `String` with `Document::to_string`,
  `Document::to_string_compact`, and `XmlPrinter`.

The implementation still requires a global allocator because the DOM stores
node names, attribute values, text, and child lists in `String` and `Vec`.

## std-Only APIs

The following APIs are compiled only when the `std` feature is enabled:

- `Document::load_file` and `Document::load_file_mut`
- `Document::save_file` and `Document::save_file_compact`
- `Document::save_writer` and `Document::save_writer_compact`
- `XmlError::Io`, `From<std::io::Error> for XmlError`, and
  `std::error::Error for XmlError`

This keeps no_std builds free of filesystem, path, and writer dependencies while
preserving the existing default API for native users.

## WebAssembly Targets

The core crate is expected to compile for:

- `wasm32-unknown-unknown`
- `wasm32-wasip1`

The `wasm32-unknown-unknown` target is best for browser or JavaScript-hosted
applications. The core crate intentionally does not depend on `wasm-bindgen`,
`web-sys`, or `js-sys`; applications can add those bindings at their own
boundary and pass XML into `tinyxml2` as `&str` or `&[u8]`.

The `wasm32-wasip1` target is useful for WASI hosts. Prefer the in-memory APIs
for portable code. Host-specific filesystem access should live in the
application layer.

## Host Boundary

The core parser should not own JavaScript, WASI, RTOS, or firmware integration.
Host code is responsible for loading XML bytes, validating or decoding UTF-8,
choosing an allocator, and translating results into host-specific values.

Recommended boundary shape:

1. Accept XML as `&str` or `&[u8]`.
2. Parse and traverse inside Rust.
3. Return compact owned values, numeric status codes, or serialized `String`
   output.
4. Keep `NodeId`, DOM references, and borrowed text inside the Rust call.

This keeps lifetimes and arena ownership local to `Document` and avoids exposing
Rust references across a WebAssembly or embedded ABI.

## Example

`examples/wasm_parse.rs` demonstrates the portable subset:

```bash
cargo run -p tinyxml2 --example wasm_parse
cargo build -p tinyxml2 --target wasm32-unknown-unknown --example wasm_parse
cargo check -p tinyxml2 --no-default-features --target wasm32-unknown-unknown --lib
```

Registered Cargo examples are executable targets and require the default `std`
feature. Validate no_std support with `--lib`, or from a consumer crate that
supplies the allocator, panic strategy, and host boundary required by that
environment.

## Validation

Use these commands when changing parser, DOM, or serialization code:

```bash
cargo check -p tinyxml2
cargo check -p tinyxml2 --no-default-features --lib
cargo check -p tinyxml2 --no-default-features --target wasm32-unknown-unknown --lib
cargo check -p tinyxml2 --target wasm32-wasip1 --lib
cargo build -p tinyxml2 --target wasm32-unknown-unknown --example wasm_parse
cargo test -p tinyxml2 --all-targets
```

Workspace crates that bind to native C/C++ code, such as `tinyxml2-capi`,
`tinyxml2-bench`, and `tinyxml2-cpp-helper`, remain native-only.
