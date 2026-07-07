# tinyxml2-rs

**A ground-up Rust implementation of the TinyXML2 API.**

[![Build Status](https://img.shields.io/github/actions/workflow/status/iTeebot/tinyxml2-rs/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/iTeebot/tinyxml2-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/tinyxml2?style=flat-square&logo=rust)](https://crates.io/crates/tinyxml2)
[![docs.rs](https://img.shields.io/docsrs/tinyxml2?style=flat-square&logo=docs.rs)](https://docs.rs/tinyxml2)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-orange?style=flat-square&logo=rust)](rust-toolchain.toml)

---

## What is tinyxml2-rs?

**tinyxml2-rs** is a native Rust implementation that provides behavioral and API compatibility with [TinyXML-2](https://github.com/leethomason/tinyxml2). It treats TinyXML-2 as a *behavioral specification* — not as source code to translate.

### Key Architectural Advantages

- **100% Safe Rust Core**: The core `tinyxml2` crate enforces `#![forbid(unsafe_code)]`. Memory safety is statically guaranteed by the compiler.
- **Generational Arena DOM**: Nodes are stored in a cache-friendly, allocation-efficient generational arena managed by `Document`. Use-after-free or dangling references return `None` rather than panicking or causing UB.
- **Strict FFI Isolation**: Raw pointers and `unsafe` code are strictly confined to the `tinyxml2-capi` FFI boundary.
- **Zero Dependencies**: The core parser has no external runtime crate dependencies.

---

## Feature Matrix

| Feature | tinyxml2-rs | C++ TinyXML2 | Advantages |
| :--- | :--- | :--- | :--- |
| **Safety** | ✅ 100% Safe Core | ❌ Raw Pointers (unsafe) | Statically prevents use-after-free, memory leaks, and undefined behavior. |
| **Memory Model** | ✅ Generational Arena | ✅ Document Ownership | Cache-friendly layouts, no dangling pointer issues. |
| **API Parity** | ✅ Complete | ✅ Native | Fully mirrors element creation, querying, and hierarchy manipulation. |
| **FFI Boundaries** | ✅ Clear isolation | ❌ Built-in FFI | The FFI boundary isolates unsafe operations and catches panics. |
| **Dependencies** | ✅ Zero dependencies | ✅ Zero dependencies | Highly portable, fast compile times. |
| **Error Handling** | ✅ Idiomatic `Result` | ❌ Document error state | Rust `Result` forces proper check-before-use patterns. |

---

## Quick Start

### Rust Quick Start
Add the dependency to your `Cargo.toml`:
```bash
cargo add tinyxml2
```

```rust
use tinyxml2::Document;

fn main() {
    let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <bookstore name="Sci-Fi Zone">
            <book category="fiction">
                <title lang="en">Dune</title>
                <author>Frank Herbert</author>
                <price>9.99</price>
            </book>
        </bookstore>
    "#;

    // Parse XML
    let doc = Document::parse(xml).expect("Failed to parse XML");

    // Navigate elements
    let root = doc.root_element().expect("No root bookstore found");
    let bookstore = doc.element_ref(root).unwrap();
    println!("Store Name: {}", bookstore.attribute("name").unwrap());

    // Iterate child elements
    for book in doc.child_elements(root, Some("book")) {
        let title_id = doc.first_child_element(book.id(), Some("title")).unwrap();
        let title = doc.element_ref(title_id).unwrap().text().unwrap();
        let price: f64 = doc.query_double_attribute(book.id(), "price").unwrap();
        println!("- Book: \"{}\" (${})", title, price);
    }
}
```

### C/C++ FFI Quick Start

Because Rust compiles to standard native dynamic and static libraries, `tinyxml2-rs` can be dropped directly into existing C and C++ projects as a binary replacement (producing `libtinyxml2.so` on Linux, `libtinyxml2.dylib` on macOS, or `tinyxml2.dll` on Windows).

By swapping in `tinyxml2-capi`, C/C++ developers get the memory safety of a Rust-backed parser under the hood. Furthermore, in upcoming releases, they will gain access to advanced features—like **XPath queries** and **XML Namespace validation**—that the original C++ TinyXML2 lacks, fully exposed through C FFI functions!

```c
#include "tinyxml2_capi.h"
#include <stdio.h>

int main() {
    // Allocate document
    TxDocument* doc = tx_document_new();
    
    // Parse
    const char* xml = "<config><theme>dark</theme></config>";
    tx_document_parse(doc, xml);

    // Navigate
    TxNodeId root = tx_root_element(doc);
    TxNodeId theme = tx_first_child_element(doc, root, "theme");
    
    printf("Theme: %s\n", tx_element_get_text(doc, theme));

    // Cleanup
    tx_document_free(doc);
    return 0;
}
```

---

## Workspace Structure

```
tinyxml2-rs/
├── crates/
│   ├── tinyxml2/          # Core library — DOM, parser, writer, entities, errors
│   ├── tinyxml2-capi/     # C FFI compatibility layer (cdylib + staticlib)
│   └── tinyxml2-bench/    # Benchmark harness (criterion)
├── tests/                 # Integration & conformance tests
├── fuzz/                  # Fuzz testing targets
├── benches/               # Benchmark data & fixtures
├── spec/                  # TinyXML2 behavioral specification tests
└── examples/              # Usage examples
```

---

## Minimum Supported Rust Version (MSRV)

Our MSRV policy guarantees compatibility with **Rust 1.85.0** or newer. Changes to MSRV are considered breaking changes and will only occur with minor or major version bumps.

---

## Release Status & Roadmap

> **Phase 8 — Documentation & Release (v1.0.0)** ✅ Complete

| Milestone | Status |
| :--- | :--- |
| Project Scaffolding & Architecture | ✅ Complete |
| Error Types & Entity Handling | ✅ Complete |
| Arena-Based DOM Representation | ✅ Complete |
| Recursive Descent Parser | ✅ Complete |
| XML Writer (Compact + Pretty) | ✅ Complete |
| Visitor Pattern Traversal | ✅ Complete |
| C FFI Layer & Header Generation | ✅ Complete |
| Conformance Testing & Differential Fuzzing | ✅ Complete |
| Stable v1.0.0 Stable Release | ✅ Complete |
| Phase 9 — WASM & `no_std` Support (v1.1.0) | ✅ Complete |
| Phase 10 — XPath & Serde Integration (v1.2.0) | 🔲 Planned |
| Phase 11 — Advanced Perf & SIMD (v2.0.0) | 🔲 Planned |

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a pull request.
We enforce the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

---

## License

Licensed under the [MIT License](LICENSE).
Copyright (c) 2026 Teebot.
