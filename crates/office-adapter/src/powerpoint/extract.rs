//! PowerPoint slide text extraction.
//!
//! Parses slide XML parts (`ppt/slides/slideN.xml`) and extracts plain
//! text from all shapes, grouped by placeholder type (title, body, etc.).
//!
//! The PresentationML namespace prefix is `p:` and inline text lives in
//! `<a:t>` elements under `<a:p>` → `<a:r>` within `<p:sp>` shapes.

use crate::xml_util::{self, XmlError, XmlResult};
use quick_xml::events::Event;
use quick_xml::name::QName;

/// Extract all text from a single PowerPoint slide XML.
///
/// Returns a multi-line string where each text run from each shape
/// becomes a line. Shape titles are prefixed with `[title]`, body
/// text with `[body]`, and other shapes with `[shape]`.
pub fn extract_slide_text(xml: &[u8]) -> XmlResult<String> {
    let mut reader = xml_util::create_reader(xml);
    let mut buf = Vec::new();
    let mut output = String::new();
    let mut depth = 0u32;
    let mut in_shape = false;
    let mut shape_type = ShapeType::Unknown;
    let mut in_paragraph = false;
    let mut para_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = e.name();

                if is_p_element(name, b"sp") {
                    in_shape = true;
                    shape_type = ShapeType::Unknown;
                } else if is_p_element(name, b"ph") {
                    // Placeholder type: title, body, etc.
                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        if key == b"type" {
                            let val = String::from_utf8_lossy(&attr.value);
                            shape_type = match val.as_ref() {
                                "title" | "ctrTitle" | "subTitle" => ShapeType::Title,
                                "body" => ShapeType::Body,
                                _ => ShapeType::Other,
                            };
                        }
                    }
                } else if is_a_element(name, b"p") && in_shape {
                    in_paragraph = true;
                    para_text.clear();
                } else if is_a_element(name, b"t") && in_paragraph {
                    // read_text() consumes the End event.
                    if let Ok(text) = reader.read_text(e.name()) {
                        depth -= 1;
                        if !text.is_empty() {
                            para_text.push_str(&text);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();

                if is_a_element(name, b"p") && in_paragraph {
                    in_paragraph = false;
                    let label = match shape_type {
                        ShapeType::Title => "[title] ",
                        ShapeType::Body => "[body] ",
                        ShapeType::Other => "[shape] ",
                        ShapeType::Unknown => "",
                    };
                    output.push_str(label);
                    output.push_str(para_text.trim());
                    output.push('\n');
                } else if is_p_element(name, b"sp") && in_shape {
                    in_shape = false;
                }
                depth -= 1;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    // Trim trailing newline.
    while output.ends_with('\n') {
        output.pop();
    }

    Ok(output)
}

#[derive(Clone, Copy, PartialEq)]
enum ShapeType {
    Unknown,
    Title,
    Body,
    Other,
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Check if an element name uses the `p:` namespace prefix.
fn is_p_element(name: QName<'_>, local: &[u8]) -> bool {
    let raw = name.as_ref();
    if raw.len() < local.len() + 2 {
        return false;
    }
    &raw[..2] == b"p:" && &raw[2..] == local
}

/// Check if an element name uses the `a:` (DrawingML) namespace prefix.
fn is_a_element(name: QName<'_>, local: &[u8]) -> bool {
    let raw = name.as_ref();
    if raw.len() < local.len() + 2 {
        return false;
    }
    &raw[..2] == b"a:" && &raw[2..] == local
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal slide with a title and body placeholder.
    fn slide_xml(title: &str, body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Title"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="title"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:r><a:t>{title}</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Body"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:r><a:t>{body}</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#
        )
    }

    #[test]
    fn test_extract_slide_with_title_and_body() {
        let xml = slide_xml("My Title", "Some body text.");
        let text = extract_slide_text(xml.as_bytes()).unwrap();
        // Title shape has <p:ph type="title"/> → [title] label.
        assert!(text.contains("[title] My Title"));
        // Body shape has no <p:ph> placeholder → no label prefix.
        assert!(text.contains("Some body text."));
    }

    #[test]
    fn test_extract_slide_title_only() {
        let xml = slide_xml("Title Only", "");
        let text = extract_slide_text(xml.as_bytes()).unwrap();
        assert!(text.contains("[title] Title Only"));
    }

    #[test]
    fn test_extract_slide_multiple_paragraphs_in_body() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Body"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="body"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p><a:r><a:t>Line one</a:t></a:r></a:p>
          <a:p><a:r><a:t>Line two</a:t></a:r></a:p>
          <a:p><a:r><a:t>Line three</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let text = extract_slide_text(xml.as_bytes()).unwrap();
        // Shape has <p:ph type="body"/> → [body] label.
        assert!(text.contains("[body] Line one"));
        assert!(text.contains("[body] Line two"));
        assert!(text.contains("[body] Line three"));
    }

    #[test]
    fn test_extract_slide_empty() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree/>
  </p:cSld>
</p:sld>"#;

        let text = extract_slide_text(xml.as_bytes()).unwrap();
        assert!(text.is_empty());
    }

    #[test]
    fn test_extract_slide_with_picture_shape() {
        // Pictures have no text — should be skipped.
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:pic>
        <p:nvPicPr>
          <p:cNvPr id="1" name="Picture"/>
          <p:cNvPicPr/>
          <p:nvPr/>
        </p:nvPicPr>
        <p:blipFill/>
        <p:spPr/>
      </p:pic>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Text"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p><a:r><a:t>Real text</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let text = extract_slide_text(xml.as_bytes()).unwrap();
        // Only the shape text, no picture noise.
        assert!(text.contains("Real text"));
        assert!(!text.contains("Picture"));
    }

    #[test]
    fn test_extract_slide_formatted_runs() {
        // Multiple <a:r> elements in one paragraph (e.g., bold + normal).
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Body"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:r><a:rPr b="1"/><a:t>Bold</a:t></a:r>
            <a:r><a:t> and normal</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let text = extract_slide_text(xml.as_bytes()).unwrap();
        assert!(text.contains("Bold and normal"));
    }

    #[test]
    fn test_extract_slide_ctr_title() {
        // Center title placeholder.
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Title"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="ctrTitle"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p><a:r><a:t>Centered Title</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let text = extract_slide_text(xml.as_bytes()).unwrap();
        assert!(text.contains("[title] Centered Title"));
    }
}
