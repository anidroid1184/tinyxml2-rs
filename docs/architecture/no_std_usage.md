# `no_std` Usage

The `tinyxml2` crate supports `no_std` builds that rely on the `alloc` crate for
dynamic memory (`Vec`, `String`, `Box`). It does **not** target bare-metal
`core`-only environments — a global allocator must be present.

The `std` feature is **enabled by default**. Disable it for embedded kernels,
RTOS environments, or custom targets where the standard library is unavailable.

> [!NOTE]
> This page focuses on embedded / non-WASM `no_std` targets. For
> WebAssembly-specific details (Emscripten, WASI SDK, browser host boundary),
> see [`docs/architecture/wasm.md`](wasm.md).

## Cargo.toml Configuration

```toml
[dependencies]
tinyxml2 = { version = "1", default-features = false }
```

Disabling `default-features` removes the `std` feature flag. The crate then
applies `#![no_std]` and links against `alloc` exclusively.

If you are building a `no_std` binary crate (not just a library), you must also
provide:

```rust
// src/main.rs or src/lib.rs of the consuming crate
#![no_std]

extern crate alloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Example: use `embedded-alloc`, `wee_alloc`, or a custom allocator
// #[global_allocator]
// static ALLOCATOR: MyAllocator = MyAllocator;
```

## Minimal `no_std` Example

```rust
#![no_std]

extern crate alloc;

use tinyxml2::Document;

fn parse_and_extract(xml: &str) -> Option<()> {
    let mut doc = Document::parse(xml).ok()?;
    let root = doc.first_child_element(doc.root(), Some("config"))?;

    // Read an attribute
    let version = doc
        .element_ref(root)?
        .attribute("version")
        .unwrap_or("unknown");
    // -- snip: use `version` via a no_std-compatible output like defmt or serial

    // Serialize back to compact XML (always available, returns alloc::string::String)
    let compact = doc.to_string_compact();
    // -- snip: send `compact` over UART, defmt, log buffer, etc.

    Some(())
}
```

All in-memory parsing and DOM operations work without `std`:

- `Document::parse`, `Document::parse_str`, `Document::parse_bytes`,
  `Document::parse_bytes_mut`
- DOM creation, mutation, and traversal (`NodeId`, iterators, `NodeRef`,
  `ElementRef`)
- Entity encode/decode
- `XmlVisitor` trait and `Document::accept`
- `Document::to_string`, `Document::to_string_compact`
- `XmlPrinter` (push-based streaming serialiser)
- `Handle` / `HandleMut` navigation wrappers

## API Availability

| API | `default-features = true` (std) | `default-features = false` (no\_std) |
|---|---|---|
| `Document::parse` / `parse_str` / `parse_bytes` / `parse_bytes_mut` | ✅ | ✅ |
| `Document::load_file` / `load_file_mut` | ✅ | ❌ |
| `Document::save_file` / `save_file_compact` | ✅ | ❌ |
| `Document::save_writer` / `save_writer_compact` | ✅ | ❌ |
| `Document::to_string` / `to_string_compact` | ✅ | ✅ |
| DOM mutation, iterators, visitors | ✅ | ✅ |
| `XmlPrinter` | ✅ | ✅ |
| `XmlError::Io` variant | ✅ | ❌ |
| `impl std::error::Error for XmlError` | ✅ | ❌ |
| `impl From<std::io::Error> for XmlError` | ✅ | ❌ |

The six file-oriented methods (`load_file`, `load_file_mut`, `save_file`,
`save_file_compact`, `save_writer`, `save_writer_compact`) and the
`XmlError::Io` variant are compiled only when the `std` feature is active. Their
signatures reference `std::path::Path` and `std::io::Write`, which are
unavailable without the standard library.

## Known Limitations

1. **A global allocator is required.** The DOM stores node names, attribute
   values, text content, and child lists in `String` and `Vec` from `alloc`.
   `core`-only (no heap) targets are not supported.

2. **No file I/O.** `Document::load_file` and `Document::save_file` need the
   `std` feature. Use `Document::parse_bytes` / `to_string` and let the
   application layer handle loading bytes from a filesystem, flash, or network.

3. **No `std::io::Write` streaming.** `Document::save_writer` and
   `save_writer_compact` are `std`-only. Use `to_string` / `to_string_compact`
   or the `XmlPrinter` streaming builder instead.

4. **No `std::error::Error` impl.** The `std::error::Error` trait
   implementation for `XmlError` is absent without `std`. Use
   `XmlError::code()` (returns `u32`) or match on variants directly for
   error handling.

5. **All Cargo examples require `std`.** The registered `[[example]]` targets
   are executables and need `fn main()`. The code patterns inside
   `examples/wasm_parse.rs` are portable and can be copied into a `no_std`
   binary crate.

## Verification

The project's CI validates these build variants (`wasm-build.yml`):

```bash
# Native target, no_std (library only)
cargo build -p tinyxml2 --no-default-features --lib

# wasm32-unknown-unknown (browser), with and without std
cargo build -p tinyxml2 --target wasm32-unknown-unknown
cargo build -p tinyxml2 --no-default-features --target wasm32-unknown-unknown --lib

# wasm32-wasip1 (WASI hosts), with and without std
cargo build -p tinyxml2 --target wasm32-wasip1
cargo build -p tinyxml2 --no-default-features --target wasm32-wasip1 --lib
```

To check against an embedded ARM target (requires `rustup target add`):

```bash
rustup target add thumbv7em-none-eabihf
cargo check -p tinyxml2 --no-default-features --target thumbv7em-none-eabihf --lib
```

Replace `thumbv7em-none-eabihf` with your target triple (e.g.
`riscv32imac-unknown-none-elf`, `aarch64-unknown-none`).

> [!TIP]
> Use `cargo check --lib` for no\_std targets — it validates the crate
> compiles without needing a linker script, panic handler, or allocator in
> scope. Full `cargo build` requires a binary entry point from a consuming
> crate.
