pub mod ast;
pub mod decoder;
pub mod parser;
pub mod util;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use roxmltree::Document;

use crate::ast::types::DataType;
use crate::decoder::Decoder;
pub use crate::decoder::value::Value;
use crate::parser::ap::ApParser;
use crate::parser::cp::CpParser;
use crate::util::xml;

/// High-level entry point for AUTOSAR ARXML type resolution and
/// binary decoding.  Supports both CP (Classic Platform) and AP
/// (Adaptive Platform), auto-detected on load.
///
/// # Usage
///
/// ```no_run
/// use arxml_converter_rs::ArxmlCodec;
///
/// let codec = ArxmlCodec::load("path/to/file.arxml").unwrap();
///
/// // Resolve type definition
/// let _dt = codec.resolve_cp(10, 100).unwrap();       // CP: (svc_id, header_id)
/// let _dt = codec.resolve_ap(100, 1).unwrap();        // AP: (svc_id, event_id)
///
/// // Decode binary payload
/// let _value = codec.decode_cp(10, 100, &[0; 4]).unwrap();
/// let _value = codec.decode_ap(100, 1, &[0; 4]).unwrap();
/// ```
#[derive(Debug)]
pub struct ArxmlCodec {
    variant: CodecVariant,
}

#[derive(Debug)]
enum CodecVariant {
    Cp {
        parser: Box<CpParser>,
        decoder: Decoder<'static>,
    },
    Ap {
        parser: Box<ApParser>,
        decoder: Decoder<'static>,
    },
}

impl ArxmlCodec {
    /// Load an AUTOSAR ARXML file, auto-detecting CP vs AP.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let xml = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("failed to read {}: {e}", path.as_ref().display()))?;

        let doc = Document::parse(&xml).map_err(|e| format!("XML parse error: {e}"))?;

        let root = doc.root_element();
        let autosar = if root.has_tag_name("AUTOSAR") {
            root
        } else {
            root.children()
                .find(|n| n.has_tag_name("AUTOSAR"))
                .ok_or("no <AUTOSAR> root element")?
        };

        let ar_packages = xml::require_child(autosar, "AR-PACKAGES")?;

        if is_ap_arxml(ar_packages) {
            let mut parser = ApParser::new();
            parser.parse(&doc)?;
            let decoder = unsafe_decoder(parser.data_types());
            Ok(Self {
                variant: CodecVariant::Ap {
                    parser: Box::new(parser),
                    decoder,
                },
            })
        } else if is_cp_arxml(ar_packages) {
            let mut parser = CpParser::new();
            parser.parse(&doc)?;
            let decoder = unsafe_decoder(parser.application_data_types());
            Ok(Self {
                variant: CodecVariant::Cp {
                    parser: Box::new(parser),
                    decoder,
                },
            })
        } else {
            Err("ARXML does not appear to be CP or AP".into())
        }
    }

    /// CP type lookup: resolve `(service_id, header_id) → DataType`.
    ///
    /// Panics if this codec was loaded from an AP ARXML.
    pub fn resolve_cp(&self, service_id: u16, header_id: u32) -> Result<&DataType, String> {
        match &self.variant {
            CodecVariant::Cp { parser, .. } => parser.resolve_type(service_id, header_id),
            CodecVariant::Ap { .. } => panic!("resolve_cp called on AP codec"),
        }
    }

    /// AP type lookup: resolve `(service_id, event_id) → DataType`.
    ///
    /// Panics if this codec was loaded from a CP ARXML.
    pub fn resolve_ap(&self, service_id: u16, event_id: u16) -> Result<&DataType, String> {
        match &self.variant {
            CodecVariant::Ap { parser, .. } => parser.resolve_type(service_id, event_id),
            CodecVariant::Cp { .. } => panic!("resolve_ap called on CP codec"),
        }
    }

    /// CP decode: resolve `(service_id, header_id)` then decode `data`.
    pub fn decode_cp(&self, service_id: u16, header_id: u32, data: &[u8]) -> Result<Value, String> {
        match &self.variant {
            CodecVariant::Cp { parser, decoder } => {
                let dt = parser.resolve_type(service_id, header_id)?;
                let (_consumed, value) = decoder.decode(data, dt)?;
                Ok(value)
            }
            CodecVariant::Ap { .. } => panic!("decode_cp called on AP codec"),
        }
    }

    /// AP decode: resolve `(service_id, event_id)` then decode `data`.
    pub fn decode_ap(&self, service_id: u16, event_id: u16, data: &[u8]) -> Result<Value, String> {
        match &self.variant {
            CodecVariant::Ap { parser, decoder } => {
                let dt = parser.resolve_type(service_id, event_id)?;
                let (_consumed, value) = decoder.decode(data, dt)?;
                Ok(value)
            }
            CodecVariant::Cp { .. } => panic!("decode_ap called on CP codec"),
        }
    }
}

// ---------------------------------------------------------------------------
// Auto-detection helpers
// ---------------------------------------------------------------------------

fn has_ar_package(node: roxmltree::Node, name: &str) -> bool {
    node.children()
        .filter(|c| c.has_tag_name("AR-PACKAGE"))
        .any(|c| xml::child_text(c, "SHORT-NAME") == Some(name))
}

fn is_ap_arxml(ar_packages: roxmltree::Node) -> bool {
    has_ar_package(ar_packages, "interfaces")
        && has_ar_package(ar_packages, "dataTypes")
        && has_ar_package(ar_packages, "IAUTOSAR")
}

fn is_cp_arxml(ar_packages: roxmltree::Node) -> bool {
    has_ar_package(ar_packages, "DataTypes")
        && has_ar_package(ar_packages, "Communication")
        && has_ar_package(ar_packages, "SoftwareTypes")
        && has_ar_package(ar_packages, "System")
        && has_ar_package(ar_packages, "Topology")
        && has_ar_package(ar_packages, "DataTypeMappingSets")
}

// ---------------------------------------------------------------------------
// Unsafe helper — see SAFETY comment
// ---------------------------------------------------------------------------

fn unsafe_decoder(types: &HashMap<String, DataType>) -> Decoder<'static> {
    // SAFETY: the ArxmlCodec owns the parser which owns the HashMap.
    // The Decoder's reference is pinned to 'static which is safe as
    // long as ArxmlCodec (and therefore the HashMap) outlives all
    // Decoder uses.  Since Decoder is stored inside the same
    // ArxmlCodec, this holds.
    let types: &'static _ = unsafe {
        std::mem::transmute::<&HashMap<String, DataType>, &'static HashMap<String, DataType>>(types)
    };
    Decoder::new(types)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const CP_ARXML: &str = include_str!("../tests/test_data/minimal_cp.arxml");

    #[test]
    fn detect_cp_arxml() {
        let doc = Document::parse(CP_ARXML).unwrap();
        let ap = doc.root_element();
        let ar_packages = xml::require_child(ap, "AR-PACKAGES").unwrap();
        assert!(is_cp_arxml(ar_packages));
        assert!(!is_ap_arxml(ar_packages));
    }

    #[test]
    fn end_to_end_cp_decode() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_cp.arxml");
        fs::write(&path, CP_ARXML).unwrap();

        let codec = ArxmlCodec::load(&path).unwrap();

        // CP: service_id=10, header_id=100 → SpeedType (u32)
        let dt = codec.resolve_cp(10, 100).unwrap();
        assert_eq!(dt.short_name, "SpeedType");

        let v = codec.decode_cp(10, 100, &[0, 0, 0, 42]).unwrap();
        assert_eq!(v, Value::U32(42));

        let _ = fs::remove_file(&path);
    }
}
