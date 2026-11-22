use std::sync::Arc;

use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use quick_xml::events::Event;
use quick_xml::Reader;
use wasm_bindgen::prelude::*;

#[cfg(feature = "codegen")]
mod codegen;
#[cfg(feature = "codegen")]
pub use codegen::compile_xsd;

pub(crate) const MAX_XSD_DEPTH: usize = 128;

fn error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

/// Convert XML bytes to JSON bytes without a schema.
#[wasm_bindgen]
pub fn xml_to_json(xml: &[u8]) -> Result<Vec<u8>, JsValue> {
    polyxml::xml_to_json(xml, None, None, true).map_err(error)
}

/// Convert JSON bytes to XML bytes without a schema.
#[wasm_bindgen]
pub fn json_to_xml(json: &[u8], root_name: Option<String>) -> Result<Vec<u8>, JsValue> {
    polyxml::json_to_xml(json, None, root_name.as_deref(), None, None, None).map_err(error)
}

/// An inline XSD compiled once and reused for typed XML/JSON conversion.
#[wasm_bindgen]
pub struct WasmSchema {
    schema: Arc<ModelSchema>,
    root_name: String,
}

#[wasm_bindgen]
impl WasmSchema {
    /// Compile a self-contained XSD. File-based includes/imports are unavailable in browsers.
    #[wasm_bindgen(constructor)]
    pub fn new(xsd: &str, root_name: Option<String>) -> Result<WasmSchema, JsValue> {
        reject_external_schema_references(xsd)?;
        let ir = XsdParser::new().parse_str(xsd).map_err(error)?;
        let root_name = root_name
            .or_else(|| ir.elements.keys().map(|name| &name.local).min().cloned())
            .or_else(|| ir.types.keys().map(|name| &name.local).min().cloned())
            .ok_or_else(|| error("XSD contains no root elements or types"))?;
        let schema = ModelSchema::from_ir(&ir, Some(&root_name)).map_err(error)?;
        Ok(Self { schema, root_name })
    }

    /// Transcode XML into JSON using this schema's field types and aliases.
    pub fn xml_to_json(&self, xml: &[u8]) -> Result<Vec<u8>, JsValue> {
        polyxml::xml_to_json(xml, Some(Arc::clone(&self.schema)), None, true).map_err(error)
    }

    /// Transcode JSON into XML using this schema's root element.
    pub fn json_to_xml(&self, json: &[u8]) -> Result<Vec<u8>, JsValue> {
        polyxml::json_to_xml(
            json,
            Some(Arc::clone(&self.schema)),
            Some(&self.root_name),
            None,
            None,
            None,
        )
        .map_err(error)
    }
}

#[cfg(feature = "codegen")]
#[wasm_bindgen]
impl WasmSchema {
    /// Transcode JSON into pretty-printed XML using this schema's root element.
    pub fn json_to_xml_pretty(&self, json: &[u8], indent: usize) -> Result<Vec<u8>, JsValue> {
        let clamped = indent.min(8);
        polyxml::json_to_xml(
            json,
            Some(Arc::clone(&self.schema)),
            Some(&self.root_name),
            Some(clamped),
            None,
            None,
        )
        .map_err(error)
    }
}

fn reject_external_schema_references(xsd: &str) -> Result<(), JsValue> {
    validate_inline_xsd(xsd).map_err(error)
}

pub(crate) fn validate_inline_xsd(xsd: &str) -> Result<(), String> {
    let mut reader = Reader::from_str(xsd);
    let mut depth = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::DocType(_)) => {
                return Err("Inline XSD must not contain DOCTYPE declarations".into());
            }
            Ok(Event::Start(tag)) => {
                depth += 1;
                if depth > MAX_XSD_DEPTH {
                    return Err(format!(
                        "Schema exceeds maximum XML nesting depth ({MAX_XSD_DEPTH})"
                    ));
                }
                let name = tag.local_name();
                if matches!(
                    name.as_ref(),
                    "include" | "import" | "redefine" | "override"
                ) {
                    return Err("Inline XSD must not use include/import/redefine/override".into());
                }
            }
            Ok(Event::Empty(tag)) => {
                if depth + 1 > MAX_XSD_DEPTH {
                    return Err(format!(
                        "Schema exceeds maximum XML nesting depth ({MAX_XSD_DEPTH})"
                    ));
                }
                let name = tag.local_name();
                if matches!(
                    name.as_ref(),
                    "include" | "import" | "redefine" | "override"
                ) {
                    return Err("Inline XSD must not use include/import/redefine/override".into());
                }
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => return Ok(()),
            Err(err) => return Err(err.to_string()),
            _ => {}
        }
    }
}
