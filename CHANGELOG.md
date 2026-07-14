# Changelog

All notable changes to this project will be documented in this file.

## [1.1.1] - 2026-07-14

### Added
- **WebAssembly Support for C/C++ FFI**: Enabled compilation of the `tinyxml2-capi` FFI bindings crate for `wasm32-unknown-unknown` and `wasm32-wasip1` targets. C and C++ projects compiled to WebAssembly (via Emscripten or WASI SDK) can now link with the static library (`libtinyxml2_capi.a`) as a drop-in replacement.
- **CI Validation for C/C++ WASM**: Added automated GitHub Actions check to compile `tinyxml2-capi` for WASM targets.
- **WASM FFI Integration Guide**: Added detailed compilation and linking instructions for Emscripten (`emcc`) and WASI SDK (`clang`) under `docs/architecture/wasm.md`.

## [1.1.0] - 2026-07-01

### Added
- **`no_std` Support**: Added `no_std` support with `alloc` allocator feature-gated for resource-constrained environments.
- **Rust WASM Support**: Verified compilation and execution of `tinyxml2` core library under `wasm32-unknown-unknown` and `wasm32-wasip1`.
- **Abstract I/O**: File operations and stream writers are now feature-gated under the `std` feature.

## [1.0.0] - 2026-07-01

### Added
- **Stable 1.0.0 Release**: Promoted `tinyxml2-rs` to a stable, production-ready release representing a complete, fully-verified TinyXML2 compatible API.
- **100% Documentation Coverage**: Enforced `#![warn(missing_docs)]` across both core `tinyxml2` and FFI `tinyxml2-capi` crates, achieving 100% Rustdoc coverage with rich usage examples.
- **Migration Guide**: Created a comprehensive C++ to Rust `MIGRATION.md` detailing namespace, type, and method equivalents, side-by-side snippets, iteration transformations, memory ownership, and C FFI integrations.
- **New Code Examples**: Added 5 compilation-verified, standalone example programs:
  - `parse_file.rs`: File parsing, attribute querying, error handling, and pretty-printing.
  - `build_dom.rs`: Programmatic DOM construction (elements, attributes, comments, CDATA, declarations).
  - `visitor.rs`: Custom DOM traversal statistics collector implementing `XmlVisitor`.
  - `streaming_writer.rs`: Push-based XML creation using `XmlPrinter` directly.
  - `c_interop.rs`: Rust client example demonstrating usage of FFI functions exported by `tinyxml2-capi`.
- **Metadata Configuration**: Fully polished keywords, categories, repository links, license, and rust-version declarations across all crates.
- **Automated CI Integration**: Enhanced test runner stability under CI to dynamically compile helper C++ dependencies during integration testing.

## [0.1.17] - 2026-07-01

### Added

- Compatibility Testing Harness: Differential testing comparing `tinyxml2-rs` DOMs and errors against C++ TinyXML2 reference runner
- Property-Based Testing: Property verification via `proptest` for DOM navigation consistency, clone independence, and entity round-tripping
- Curated XML Corpus: 100+ files categorized under `valid`, `invalid`, and `unicode` structures
- Fuzz Testing Infrastructure: `cargo-fuzz` setup with 4 targets (`parse_fuzz`, `roundtrip_fuzz`, `serialize_fuzz`, `streaming_fuzz`)
- Criterion Benchmarking Suite: Statistically rigorous benchmarks for parse speeds (1KB, 100KB, 10MB), serialization (compact and pretty), DOM traversal, and side-by-side C++ comparison
- Nightly and Push CI/CD workflows for automated fuzz scheduling and benchmark stats tracking

## [0.1.16] - 2026-06-30

### Added

- C FFI compatibility layer (`tinyxml2-capi` crate) with ~56 `extern "C"` functions
- Opaque handle types: `TxDocument`, `TxPrinter` for safe FFI usage
- C-compatible types: `TxNodeId`, `TxNodeType`, `TxError` enums
- Document lifecycle: `tx_document_new`, `tx_document_free`, `tx_document_parse`, `tx_document_load_file`, `tx_document_save_file`, `tx_document_to_string`, `tx_document_clear`
- DOM factory: `tx_new_element`, `tx_new_text`, `tx_new_comment`, `tx_new_declaration`, `tx_new_unknown`
- DOM mutations: `tx_insert_end_child`, `tx_insert_first_child`, `tx_insert_after_child`, `tx_delete_child`, `tx_delete_children`, `tx_delete_node`
- DOM navigation: `tx_parent`, `tx_first_child`, `tx_last_child`, `tx_prev_sibling`, `tx_next_sibling`, `tx_first_child_element`, `tx_next_sibling_element`, `tx_root_element`
- Element & attribute helpers: `tx_element_name`, `tx_element_attribute`, `tx_element_set_attribute`, `tx_element_delete_attribute`, `tx_element_get_text`, `tx_element_set_text`
- Typed attribute accessors: `tx_query_int_attribute`, `tx_query_bool_attribute`, `tx_query_double_attribute`, `tx_int_attribute`, `tx_bool_attribute`, `tx_double_attribute`
- Streaming printer API: `tx_printer_new`, `tx_printer_new_compact`, `tx_printer_open_element`, `tx_printer_push_attribute`, `tx_printer_close_element`, `tx_printer_push_text`, `tx_printer_push_comment`, `tx_printer_result`, `tx_printer_clear`
- Node inspection: `tx_node_type`, `tx_node_is_null`, `tx_node_value`, `tx_node_line`
- Auto-generated C header via `cbindgen` at `include/tinyxml2.h`
- Panic-safe FFI boundary with `catch_unwind` on all exported functions
- `NodeId::from_raw_parts` and `NodeId::raw_parts` public FFI conversion methods on core crate

## [0.1.15] - 2026-06-30

### Added

- **Typed Reference Wrappers**: `NodeRef<'a>` and `ElementRef<'a>` for safe, ergonomic node access with lifetime-bounded references.
- **Null-safe Navigation Handles**: `Handle<'a>` and `HandleMut<'a>` for fluent DOM traversal chains with automatic `None` propagation.
- **Iterator Adapters**:
  - `Children<'a>` — iterates direct child nodes with `DoubleEndedIterator` support.
  - `ChildElements<'a>` — iterates direct child elements with optional name filtering.
  - `Siblings<'a>` — iterates following sibling nodes.
  - `Attributes<'a>` — iterates element attributes as `(&str, &str)` pairs with `DoubleEndedIterator` support.
  - `Descendants<'a>` — depth-first pre-order iteration over all descendant nodes.
- **Convenience methods on `Document`**: `children()`, `child_elements()`, `siblings()`, `descendants()`, `attributes()`, `handle()`, `handle_mut()`, `node_ref()`, `element_ref()`.

## [0.1.14] - 2026-06-30

### Added

- `XmlVisitor` trait defining standard DOM traversal callbacks (`visit_enter_document`, `visit_exit_document`, `visit_enter_element`, `visit_exit_element`, `visit_text`, `visit_comment`, `visit_declaration`, `visit_unknown`)
- `XmlPrinter` struct supporting pretty-printed (indented, newline-formatted) and compact XML serialization
- Stateful streaming (push) API on `XmlPrinter` (`open_element`, `push_attribute`, `close_element`, `push_text`, `push_text_raw`, `push_cdata`, `push_comment`, `push_declaration`, `push_unknown`, `push_header`)
- DOM-driven serialization methods on `Document` (`to_string`, `to_string_compact`, `save_file`, `save_file_compact`, `save_writer`, `save_writer_compact`)
- `std::fmt::Display` implementation for `Document`
- Comprehensive unit and round-trip integration test suites verifying printer correctness and parse-print-parse equivalence
- Automated release publication GitHub Actions workflow (`publish.yml`) to publish crates to crates.io on tag push

## [0.1.13] - 2026-06-29

### Added

- Project scaffolding with 3-crate workspace structure
- `XmlError` enum with all TinyXML2-compatible error codes
- `ParseErrorKind` enum for parse error classification
- XML entity encoding and decoding (5 predefined entities + numeric character references)
- Zero-allocation fast paths for entity handling (`Cow<str>`)
- Character classification utilities (XML name characters, whitespace)
- UTF-8 BOM detection and stripping
- Whitespace collapsing utility
- XML name parsing utility
- Generational arena allocator with use-after-free detection
- `NodeId` type with generation-checked access
- Arena iterators with `IntoIterator` implementations
- `Whitespace` enum (Preserve, Collapse, Pedantic)
- `ParseOptions` builder struct
- Workspace-level lint configuration (unsafe_code = forbid, clippy pedantic)
- CI pipeline (GitHub Actions: test matrix, lint, MSRV)
- Complete TinyXML2 API specification reference
- Project documentation (README, CONTRIBUTING, ARCHITECTURE, ROADMAP, SECURITY, CODE_OF_CONDUCT)
- Issue templates and PR template
- Editor and formatter configuration (.editorconfig, rustfmt.toml, .clippy.toml)
