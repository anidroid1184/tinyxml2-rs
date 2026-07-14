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

## C and C++ WebAssembly Support

The C FFI compatibility layer (`tinyxml2-capi`) compiles to WebAssembly targets out of the box. C and C++ projects compiled to WebAssembly (via Emscripten or WASI SDK) can link against this compiled Rust artifact as a drop-in replacement.

> [!NOTE]
> The project's CI validates compiling the Rust artifacts (`libtinyxml2_capi.a` and `tinyxml2_capi.wasm`) for WASM targets. Compiling and linking the final C/C++ application remains the responsibility of the consumer's C toolchain (e.g. Emscripten or WASI SDK).

When compiled to `wasm32-unknown-unknown` or `wasm32-wasip1`, it generates:
- A WebAssembly binary (`tinyxml2_capi.wasm`)
- A static library (`libtinyxml2_capi.a`)

### Compiling the C FFI for WebAssembly

To compile the C FFI bindings library for WebAssembly:

```bash
# Target the browser / JavaScript environment (e.g. for Emscripten)
cargo build -p tinyxml2-capi --target wasm32-unknown-unknown --release

# Target WASI environments (e.g. for Wasmtime, Wasmer)
cargo build -p tinyxml2-capi --target wasm32-wasip1 --release
```

The resulting library `libtinyxml2_capi.a` will be located under `target/wasm32-unknown-unknown/release/` or `target/wasm32-wasip1/release/`.

### Linking in C/C++ WebAssembly Projects

#### 1. Browser/Emscripten Toolchain (`emcc`)
To compile a C/C++ file with Emscripten and link the Rust WASM static library:

```bash
emcc -Icrates/tinyxml2-capi/include -O3 \
  crates/tinyxml2-capi/examples/basic.c \
  target/wasm32-unknown-unknown/release/libtinyxml2_capi.a \
  -o basic.js \
  -s WASM=1 \
  -s ALLOW_MEMORY_GROWTH=1
```

#### 2. WASI SDK Toolchain (`clang`)
To compile for a standalone WASI runtime (e.g., Wasmtime) using the WASI SDK:

```bash
/path/to/wasi-sdk/bin/clang -Icrates/tinyxml2-capi/include -O3 \
  --sysroot=/path/to/wasi-sysroot \
  crates/tinyxml2-capi/examples/basic.c \
  target/wasm32-wasip1/release/libtinyxml2_capi.a \
  -o basic.wasm
```

### ABI and Memory Considerations
- **Shared Memory**: Since Rust and C/C++ compile into a single WebAssembly module when linked statically, they share the same linear memory and allocator (supplied by the C runtime or Rust's target).
- **String Lifetimes**: Pointers returned by `tx_document_to_string`, `tx_element_name`, or other getters returning `*const c_char` point to UTF-8 C-strings borrowed from the `TxDocument`/`TxPrinter`-owned `CString` caches. Callers **must not free** these pointers. They become invalid as soon as the document is modified (mutated) or when the owning document/printer wrapper is freed.

## Validation

Use these commands when changing parser, DOM, serialization, or FFI code:

```bash
cargo check -p tinyxml2
cargo check -p tinyxml2 --no-default-features --lib
cargo check -p tinyxml2 --no-default-features --target wasm32-unknown-unknown --lib
cargo check -p tinyxml2 --target wasm32-wasip1 --lib
cargo build -p tinyxml2 --target wasm32-unknown-unknown --example wasm_parse
cargo build -p tinyxml2-capi --target wasm32-unknown-unknown --release
cargo build -p tinyxml2-capi --target wasm32-wasip1 --release
cargo test -p tinyxml2 --all-targets
cargo test -p tinyxml2-capi
```

Workspace crates that bind to native C/C++ code, such as `tinyxml2-bench` and `tinyxml2-cpp-helper`, remain native-only.
