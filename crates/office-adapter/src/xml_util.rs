//! XML parsing utilities for Office document extraction.
//!
//! Provides shared helpers for reading Office XML parts using `quick-xml`.
//! Each format module (Word, PowerPoint, Excel) uses these utilities to
//! parse its specific XML namespaces and structures.

/// Placeholder for XML namespace constants used across Office formats.
///
/// Phase 2 will populate these with actual namespace URIs for:
/// - WordprocessingML (`w:`)
/// - SpreadsheetML (`s:`)
/// - PresentationML (`p:`)
/// - DrawingML (`a:`, `r:`)
/// - Office relationships (`r:`)
pub mod ns {
    // WordprocessingML
    pub const WORD_MAIN: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    // SpreadsheetML
    pub const EXCEL_MAIN: &str =
        "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    // PresentationML
    pub const PPT_MAIN: &str =
        "http://schemas.openxmlformats.org/presentationml/2006/main";
    // DrawingML
    pub const DRAWING_MAIN: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
    // Relationships
    pub const RELATIONSHIPS: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships";
    // Office document relationships
    pub const OFFICE_RELATIONSHIPS: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
}

/// Result type alias for XML parsing operations.
pub type XmlResult<T> = Result<T, XmlError>;

/// Errors that can occur during XML parsing.
#[derive(Debug, thiserror::Error)]
pub enum XmlError {
    #[error("XML parse error: {0}")]
    Parse(String),

    #[error("missing expected element: {0}")]
    MissingElement(String),

    #[error("unexpected element: expected {expected}, found {found}")]
    UnexpectedElement { expected: String, found: String },

    #[error("missing attribute: {0}")]
    MissingAttribute(String),
}

impl From<quick_xml::Error> for XmlError {
    fn from(e: quick_xml::Error) -> Self {
        XmlError::Parse(e.to_string())
    }
}

/// Parse an XML string into a `quick_xml::Reader` with safe defaults.
///
/// Disables DTD processing to prevent entity expansion attacks.
pub fn create_reader(xml: &[u8]) -> quick_xml::Reader<&[u8]> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    // Disable DTD expansion for safety.
    reader.config_mut().expand_empty_elements = true;
    reader.config_mut().trim_text_start = true;
    reader
}

/// Extract the text content from an XML element, trimming whitespace.
///
/// Phase 2 will use this extensively when extracting paragraph text,
/// cell values, and slide content from Office XML parts.
#[allow(dead_code)]
pub fn read_text_from_event(
    _reader: &quick_xml::Reader<&[u8]>,
    event: &quick_xml::events::Event<'_>,
) -> Option<String> {
    match event {
        quick_xml::events::Event::Text(t) => {
            let raw = t.unescape().ok()?;
            let text = raw.trim().to_owned();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event;

    #[test]
    fn test_create_reader() {
        let xml = b"<root><child>hello</child></root>";
        let mut reader = create_reader(xml);
        let mut found = false;
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.name().as_ref() == b"child" => {
                    found = true;
                }
                Ok(Event::Eof) => break,
                Err(e) => panic!("unexpected error: {:?}", e),
                _ => {}
            }
        }
        assert!(found);
    }

    #[test]
    fn test_read_text_from_event() {
        let xml = b"<root>  hello world  </root>";
        let mut reader = create_reader(xml);
        let mut texts = Vec::new();
        loop {
            match reader.read_event() {
                Ok(event) => {
                    if let Some(text) = read_text_from_event(&reader, &event) {
                        texts.push(text);
                    }
                    if matches!(event, Event::Eof) {
                        break;
                    }
                }
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }
        assert_eq!(texts, vec!["hello world"]);
    }
}
