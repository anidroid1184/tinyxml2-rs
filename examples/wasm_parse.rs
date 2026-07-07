//! Parses XML entirely from memory using the portable `tinyxml2` API subset.
//!
//! The registered Cargo example uses the default `std` feature so it can run as
//! an executable, but it avoids filesystem and process APIs. The
//! `parse_device_summary` function shows the in-memory shape to reuse from
//! WebAssembly or embedded hosts that provide their own boundary code.

use tinyxml2::{Document, Result};

const SAMPLE_XML: &str = r#"
<device id="sensor-7" enabled="true">
    <reading unit="celsius">23.5</reading>
    <reading unit="humidity">41</reading>
</device>
"#;

/// Compact value returned across a WASM or embedded boundary.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DeviceSummary {
    /// Whether the root device is enabled.
    pub enabled: bool,
    /// Number of direct `<reading>` child elements.
    pub reading_count: usize,
    /// Length of the compact serialized XML.
    pub compact_len: usize,
}

/// Parse an in-memory XML document and extract a small owned summary.
pub fn parse_device_summary(xml: &str) -> Result<DeviceSummary> {
    let mut doc = Document::parse(xml)?;
    let Some(device) = doc.first_child_element(doc.root(), Some("device")) else {
        return Ok(DeviceSummary::default());
    };

    doc.set_attribute(device, "transport", "wasm")?;

    let enabled = doc
        .element_ref(device)
        .is_some_and(|element| element.bool_attribute("enabled", false));
    let reading_count = doc.child_elements(device, Some("reading")).count();
    let compact_len = doc.to_string_compact().len();

    Ok(DeviceSummary {
        enabled,
        reading_count,
        compact_len,
    })
}

fn main() -> Result<()> {
    let summary = parse_device_summary(SAMPLE_XML)?;

    println!(
        "enabled={} reading_count={} compact_len={}",
        summary.enabled, summary.reading_count, summary.compact_len
    );

    Ok(())
}
