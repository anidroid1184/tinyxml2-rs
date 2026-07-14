# tinyxml2-rs — Development Roadmap

> A ground-up Rust implementation of the [TinyXML2](https://github.com/leethomason/tinyxml2) C++ API.
>
> **Crate Architecture:** `tinyxml2` (core library) · `tinyxml2-capi` (C FFI bindings) · `tinyxml2-bench` (benchmarks & comparisons)

---

## Overview

This roadmap defines eight sequential phases for building a complete, production-grade Rust
replacement for TinyXML2. The implementation uses a **generational arena** for memory-safe tree
storage and a **recursive-descent parser** for XML processing. Each phase builds on the previous
one and is designed to produce a shippable, testable increment.

| Phase | Title                        | Complexity | Status         |
|-------|------------------------------|------------|----------------|
| 1     | Foundation                   | Medium     | ✅ **Completed** |
| 2     | DOM Core                     | High       | ✅ **Completed** |
| 3     | XML Parser                   | High       | ✅ **Completed** |
| 4     | Writer / Serializer          | Medium     | ✅ **Completed** |
| 5     | Visitor Pattern & Ergonomics | Medium     | ✅ **Completed** |
| 6     | C API (`tinyxml2-capi`)      | Medium     | ✅ **Completed** |
| 7     | Testing & Benchmarks         | Medium     | ✅ **Completed** |
| 8     | Documentation & Release      | Low        | ✅ **Completed** |
| 9     | WASM & `no_std` Support      | Medium     | ✅ **Completed** |
| 10    | XPath & Serde Integration    | High       | 🔲 Planned     |
| 11    | Advanced Perf & SIMD         | High       | 🔲 Planned     |

---

## Phase 1: Foundation ✅ **COMPLETED**

> Establish the core infrastructure, error handling, and build pipeline that every subsequent
> phase depends on.

**Estimated Complexity:** Medium

### Key Deliverables

- [x] **Error types** — `XmlError` enum covering parse errors, I/O errors, missing attributes,
  value conversion failures, and arena allocation faults. Maps to TinyXML2's `XMLError` codes.
- [x] **Entity handling** — Encode/decode the five predefined XML entities (`&amp;`, `&lt;`,
  `&gt;`, `&quot;`, `&apos;`) plus numeric character references (`&#xHHHH;`, `&#DDDD;`).
- [x] **Generational arena allocator** — Arena-based node storage using generational indices
  (`NodeId`) for O(1) access with dangling-reference detection. Supports allocation, deallocation,
  and safe index reuse.
- [x] **Configuration system** — `ParserConfig` struct with builder pattern for whitespace mode,
  entity processing toggle, maximum parse depth, and encoding settings.
- [x] **CI pipeline** — GitHub Actions workflows for `cargo check`, `cargo test`, `cargo clippy`,
  `cargo fmt --check`, and MSRV verification.
- [x] **Documentation infrastructure** — Crate-level docs, module-level docs, and `#[doc(hidden)]`
  annotations for internal APIs.
- [x] **Test suite** — **87 unit tests** covering error construction, entity round-tripping, arena
  operations, and configuration validation. **15 doc tests** embedded in public API documentation.

### Architecture Decisions Locked

| Decision                | Choice                              | Rationale                                       |
|-------------------------|-------------------------------------|-------------------------------------------------|
| Node storage            | Generational arena (`Vec<Entry<T>>`)| Cache-friendly, no `Rc`/`RefCell`, safe reuse   |
| Index type              | `NodeId(u32 index, u32 generation)` | Compact, detects use-after-free                  |
| Error strategy          | `Result<T, XmlError>` everywhere    | Idiomatic Rust, no panics in library code        |
| String storage          | Owned `String` per node             | Simplicity first; interning deferred to Phase 8  |
| Configuration           | Builder pattern on `ParserConfig`   | Ergonomic, extensible, `#[non_exhaustive]`       |

---

## Phase 2: DOM Core

> Implement the full XML Document Object Model tree with all node types, tree manipulation
> operations, and deep cloning.

**Estimated Complexity:** High — this is the largest single phase by line count.

### Key Deliverables

- [x] **Node type enum** — `XmlNode` variants: `Document`, `Element`, `Text`, `Comment`,
  `Declaration`, `ProcessingInstruction`, `Unknown`. Each variant holds type-specific data.
- [x] **`Document` root** — The top-level container. Owns the arena. Provides `new()`,
  `load_file()`, `save_file()`, and the `root_element()` accessor.
- [x] **`Element` operations** — `name()`, `set_name()`, attribute CRUD (`set_attribute`,
  `find_attribute`, `delete_attribute`), child element queries (`first_child_element`,
  `next_sibling_element` with optional name filter).
- [x] **`Attribute` storage** — Linked-list of attributes per element, supporting typed getters
  (`int_value`, `float_value`, `bool_value`) with fallback defaults, mirroring the C++ API.
- [x] **Tree operations** — `insert_first_child`, `insert_end_child`, `insert_after_node`,
  `delete_child`, `delete_children`. All operations maintain parent/child/sibling pointer
  consistency within the arena.
- [x] **Deep cloning** — `deep_clone(node, target_document)` that recursively copies a subtree,
  including across documents (re-allocating into the target arena).
- [x] **Memory management** — `Document::clear()` to reset the arena. Individual node deletion
  with generation bump to invalidate stale `NodeId` handles.

### Estimated Test Count: 120–150 unit tests

### Design Notes

```
Document (arena owner)
  └─ Element "root"
       ├─ Attribute "version" = "1.0"
       ├─ Element "child1"
       │    └─ Text "hello"
       ├─ Comment "<!-- note -->"
       └─ Element "child2"
            └─ Element "nested"
```

All parent ↔ child ↔ sibling links are stored as `Option<NodeId>` inside each node's metadata
struct, enabling O(1) traversal without pointer chasing outside the arena.

---

## Phase 3: XML Parser

> Build the recursive-descent XML parser that converts raw XML text into the DOM tree from Phase 2.

**Estimated Complexity:** High — correctness-critical with many edge cases.

### Key Deliverables

- [x] **Recursive-descent parser** — Hand-written parser (no parser combinators or generated code)
  operating on `&[u8]` input. Matches TinyXML2's parsing strategy for behavioral compatibility.
- [x] **Entity resolution** — Inline expansion of the five predefined entities and numeric
  character references during text and attribute value parsing (leveraging Phase 1 entity module).
- [x] **Whitespace handling** — Two modes mirroring TinyXML2:
  - `PRESERVE_WHITESPACE` — Keep all whitespace as-is.
  - `COLLAPSE_WHITESPACE` — Collapse runs of whitespace to a single space, trim leading/trailing.
- [x] **BOM detection** — Auto-detect and skip UTF-8 BOM (`0xEF 0xBB 0xBF`) at the start of
  input. Reject non-UTF-8 BOMs with a clear error.
- [x] **Depth limits** — Configurable maximum nesting depth (default: 100) to prevent stack
  overflow on malicious input. Returns `XmlError::ElementDepthExceeded` on violation.
- [x] **Error recovery** — Rich error reporting with byte offset, line number, and column number.
  No panic paths.
- [x] **Parse entry points** — `Document::parse(xml: &str)`, `Document::load_file(path)`,
  `Document::parse_bytes(bytes: &[u8])`.

### Parser Architecture

```
parse_document()
  ├─ skip_bom()
  ├─ parse_declaration()?     // <?xml ... ?>
  └─ parse_element()          // recursive
       ├─ parse_attributes()
       ├─ parse_children()
       │    ├─ parse_element()      // recurse
       │    ├─ parse_text()
       │    ├─ parse_comment()      // <!-- ... -->
       │    ├─ parse_cdata()        // <![CDATA[ ... ]]>
       │    └─ parse_pi()           // <?target ... ?>
       └─ parse_close_tag()
```

### Estimated Test Count: 200+ unit tests (including malformed XML, edge cases, encoding)

---

## Phase 4: Writer / Serializer

> Implement XML output with pretty-printing and compact modes, equivalent to TinyXML2's
> `XMLPrinter`.

**Estimated Complexity:** Medium

### Key Deliverables

- [x] **`XmlPrinter` struct** — Stateful writer that traverses the DOM and emits well-formed XML.
  Implements the `XmlVisitor` trait (Phase 5) internally for traversal.
- [x] **Pretty-print mode** — Indented output with configurable indent string (default: 4 spaces).
  Newlines between elements. Mirrors `XMLPrinter(FILE*, true)`.
- [x] **Compact mode** — Minimal whitespace output for network/storage efficiency. Mirrors
  `XMLPrinter(FILE*, false)`.
- [x] **Write targets** —
  - `to_string() -> String` — In-memory serialization.
  - `to_writer(impl Write)` — Streaming output to any `std::io::Write` sink.
  - `to_file(path)` — Convenience wrapper for file output.
- [x] **Streaming API** — Push-based API for building XML without a DOM tree:
  `open_element("tag")`, `push_attribute("key", "value")`, `push_text("content")`,
  `close_element()`. Useful for high-performance serialization.
- [x] **Entity escaping** — Automatic escaping of `<`, `>`, `&`, `"`, `'` in text content and
  attribute values during output.
- [x] **Declaration output** — Optional XML declaration (`<?xml version="1.0" encoding="UTF-8"?>`)
  controlled by configuration.

### Estimated Test Count: 80–100 unit tests

---

## Phase 5: Visitor Pattern & Ergonomics

> Add the Visitor pattern for DOM traversal and ergonomic wrapper types for a Rust-idiomatic API
> surface.

**Estimated Complexity:** Medium

### Key Deliverables

- [x] **`XmlVisitor` trait** — Mirrors TinyXML2's `XMLVisitor` with `visit_enter` / `visit_exit`
  methods for each node type. Returns `bool` to control traversal continuation.
- [x] **`Document::accept(visitor)`** — Depth-first traversal that drives the visitor.
- [x] **Handle types** — `Handle<'a>`, `HandleMut<'a>` — null-safe navigation wrappers around
  `NodeId` for fluent DOM traversal chains with automatic `None` propagation.
- [x] **`NodeRef<'a>` / `ElementRef<'a>`** — Typed, lifetime-bounded reference wrappers for
  safe, ergonomic access. Prevents iterator invalidation.
- [x] **Iterators** — `Children`, `ChildElements`, `Siblings`, `Attributes`, `Descendants` —
  standard Rust iterators with `DoubleEndedIterator` and `FusedIterator` support.
- [x] **Convenience methods** — `Document::children()`, `child_elements()`, `siblings()`,
  `descendants()`, `attributes()`, `handle()`, `handle_mut()`, `node_ref()`, `element_ref()`.

### Estimated Test Count: 80–100 unit tests

---

## Phase 6: C API (`tinyxml2-capi` crate) ✅ **COMPLETED**

> Expose the Rust library through a C-compatible FFI for drop-in replacement in C/C++ projects.

**Estimated Complexity:** Medium — mechanically intensive but architecturally straightforward.

### Key Deliverables

- [x] **`extern "C"` function exports** — ~56 C functions covering document lifecycle, DOM
  factory, tree mutation, navigation, element/attribute access, typed attribute queries,
  streaming printer, and node inspection. Naming convention: `tx_document_new`, `tx_element_name`, etc.
- [x] **Opaque handle types** — `TxDocument` (boxed `Document`) and `TxPrinter` (boxed
  `XmlPrinter`) wrapped in opaque structs. `TxNodeId` as `#[repr(C)]` value type.
- [x] **Error codes** — C-compatible `TxError` enum with integer values matching TinyXML2's
  error enum for maximum compatibility.
- [x] **Header generation** — Auto-generated `tinyxml2.h` header via [`cbindgen`](https://github.com/eyre-rs/cbindgen).
  Configured in `cbindgen.toml` with C-style output.
- [x] **Build outputs** —
  - Static library (`libtinyxml2_capi.a`)
  - Shared library (`libtinyxml2_capi.dylib` / `.so` / `.dll`)
  - Configured via `crate-type = ["staticlib", "cdylib"]`
- [x] **Memory safety contract** — All FFI functions validate non-null pointers. Null inputs
  return error codes or no-op. No panics across FFI boundary (`catch_unwind` guards).
- [x] **Lifetime management** — `tx_document_free()`, `tx_printer_free()` for
  heap-allocated returns. Clear ownership documentation in header comments.

### Estimated Test Count: 60–80 integration tests (C caller tests via `cc` crate in build script)

---

## Phase 7: Testing & Benchmarks (`tinyxml2-bench` crate) ✅ **COMPLETED**

> Comprehensive compatibility testing against TinyXML2 C++ and performance benchmarking.

**Estimated Complexity:** Medium

### Key Deliverables

- [x] **Compatibility test suite** — Port TinyXML2's own test cases (`xmltest.cpp`) to Rust.
  Verify identical parse results, error behavior, and output formatting for ~300 test vectors.
- [x] **Round-trip tests** — Parse → serialize → re-parse cycle for a corpus of real-world XML
  files. Assert structural equality.
- [x] **Fuzz testing** — `cargo-fuzz` harnesses for:
  - `fuzz_parse` — Random byte sequences → parser (must not panic/crash).
  - `fuzz_roundtrip` — Valid XML → parse → serialize → parse → assert equality.
  - `fuzz_capi` — Random FFI call sequences → C API (must not segfault).
- [x] **Criterion benchmarks** — Comparative benchmarks against the original TinyXML2 C++ library
  (linked via `cc` crate):
  | Benchmark                  | Measures                                |
  |----------------------------|-----------------------------------------|
  | `parse_small`              | Parse a 1 KB XML document               |
  | `parse_medium`             | Parse a 100 KB XML document             |
  | `parse_large`              | Parse a 10 MB XML document              |
  | `serialize_pretty`         | Pretty-print a parsed DOM               |
  | `serialize_compact`        | Compact-print a parsed DOM              |
  | `dom_traversal`            | Walk all nodes depth-first              |
  | `attribute_lookup`         | Query attributes by name (100K lookups) |
  | `arena_alloc_dealloc`      | Allocate/deallocate 100K nodes          |
- [x] **Performance targets** — Within 1.5× of TinyXML2 C++ for parsing, within 1.0× for
  serialization (Rust I/O should match or beat C++ `fprintf`).
- [x] **CI integration** — Benchmark results tracked via `criterion`'s JSON output. Regression
  alerts on >10% slowdown.

### Estimated Test Count: 300+ compatibility tests, 3 fuzz targets, 8 benchmark groups

---

## Phase 8: Documentation & Release ✅ **COMPLETED**

> Polish documentation, write migration guides, and publish the initial release.

**Estimated Complexity:** Low

### Key Deliverables

- [x] **Migration guide** — `MIGRATION.md` mapping every TinyXML2 C++ class and method to its
  Rust equivalent. Organized by class (`XMLDocument` → `Document`, `XMLElement` → `Element`, etc.)
  with code examples for each.
- [x] **Example programs** — `examples/` directory with:
  - `parse_file.rs` — Parse an XML file and print element names.
  - `build_dom.rs` — Programmatically construct a DOM and serialize.
  - `visitor.rs` — Implement a custom visitor to extract data.
  - `streaming_writer.rs` — Use the push-based API for large output.
  - `c_interop.rs` — Demonstrate calling the C API from Rust.
- [x] **API documentation** — 100% documentation coverage on public items. Rich examples in doc
  comments. Cross-linked with `#[doc]` attributes.
- [x] **`README.md` overhaul** — Feature matrix, quick start, performance comparison table,
  MSRV policy, and contribution guidelines.
- [x] **`CHANGELOG.md`** — Initial changelog following [Keep a Changelog](https://keepachangelog.com/) format.
- [x] **Crate metadata** — `Cargo.toml` keywords, categories, repository link, license (MIT/Apache-2.0
  dual license), and `rust-version` field.
- [x] **1.0.0 release** — Publish `tinyxml2` and `tinyxml2-capi` to [crates.io](https://crates.io).
  Tag `v1.0.0` in git.

---

## Phase 9: WASM & `no_std` Support (Target: Versions 1.1.0 & 1.1.1) ✅ **COMPLETED**

Enable tinyxml2-rs to run in resource-constrained embedded environments and web browsers.

### Key Deliverables (v1.1.0)
- [x] **no_std compatibility** — Feature-gate standard library dependencies and use the `alloc` crate for all dynamic memory allocations (`Vec`, `String`, `Box`). Target: `#![no_std]` with `extern crate alloc`.
- [x] **WASM target support** — Validate the core crate on `wasm32-unknown-unknown` and `wasm32-wasip1`. JavaScript bindings are intentionally left to the host application boundary; see `docs/architecture/wasm.md`.
- [x] **Abstract I/O** — Gate file/stream writers behind the `std` feature and keep the core parser, DOM, and string serialization available without `std::io::Write`.

### Key Deliverables (v1.1.1)
- [x] **C/C++ FFI WASM Support** — Support compiling and linking the FFI bindings (`tinyxml2-capi`) on `wasm32-unknown-unknown` and `wasm32-wasip1` targets for C/C++ WebAssembly integration.

---

## Phase 10: XPath & Serde Integration (Target: Version 1.2.0) 🔲 **PLANNED**

Improve the ergonomics of data extraction and structured serialization.

### Key Deliverables
- [ ] **XPath Subset** — Implement a lightweight, fast, and spec-compliant subset of XPath 1.0 (e.g., path selection, tag filtering) to query DOM nodes dynamically. Expose these querying interfaces to C/C++ via FFI functions in `tinyxml2-capi`.
- [ ] **Serde Serialization** — Introduce a separate `tinyxml2-serde` crate allowing automatic Rust struct serialization/deserialization into XML structures.
- [ ] **Attributes vs. Elements Annotations** — Custom serde macros (e.g., `#[xml(attribute)]`, `#[xml(element)]`) for direct structure control.

---

## Phase 11: Advanced Perf & SIMD (Target: Version 2.0.0) 🔲 **PLANNED**

Unlock high-performance processing for large-scale XML workloads.

### Key Deliverables
- [ ] **SIMD Character Scanning** — Use SIMD vector instructions (SSE2/AVX2/NEON) via `memchr` or custom intrinsics to accelerate whitespace scanning and token delimiter matching.
- [ ] **String Interning** — Intern tag and attribute names in the Document arena to reduce memory usage and improve lookup speed.
- [ ] **Parallel Traversal** — Add Rayon-based parallel visitor traversal for multi-threaded read-only traversals of very large documents.
- [ ] **XML Namespace Support** — Namespace-qualified element/attribute lookup and context propagation. Expose these features via the FFI layer (`tinyxml2-capi`) to give C and C++ users native namespace capabilities that the original C++ TinyXML2 lacks.

---

## Contributing

Contributions are welcome at any phase. Please see the issue tracker for phase-specific tasks
labeled `phase-N`. Each phase has a tracking issue with a checklist of deliverables.

**Priority for contributors:**
1. Phase 2 (DOM Core) — Most impactful, enables all downstream phases.
2. Phase 3 (Parser) — Second most impactful, enables real-world usage.
3. Phase 7 (Testing) — Can begin in parallel once Phase 3 is complete.

---

*Last updated: 2026-07-01*
