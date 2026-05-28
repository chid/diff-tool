//! XML tree structural comparison (Approach 2: XML Tree Diff).
//!
//! Compares two Word document XML files structurally by walking their
//! element trees in parallel using `quick-xml` event streams.
//!
//! Unlike Approach 1 (text-only) and Approach 3 (semantic model), this
//! approach sees the raw XML structure: every element addition, deletion,
//! or modification is detected regardless of its semantic meaning.
//!
//! This makes it best suited for:
//! - Debugging and validating the semantic parser
//! - Detecting XML-level equivalence (ignoring attribute order)
//! - Automated testing of Office document generators
//!
//! It is less suitable as a user-facing diff because it produces
//! noisy, path-based output that is hard to interpret without
//! understanding the OOXML schema.

use crate::xml_util::{self, XmlError, XmlResult};
use quick_xml::events::Event;
use quick_xml::name::QName;
use serde::{Deserialize, Serialize};

// ── Output types ────────────────────────────────────────────────────────

/// An individual structural change found by the XML tree diff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XmlTreeChange {
    /// The XML path where the change occurred (e.g., `/w:document/w:body/w:p[2]`).
    pub path: String,
    /// The kind of structural change.
    pub kind: XmlChangeKind,
    /// For modifications: the old value (if applicable).
    pub old_value: Option<String>,
    /// For modifications: the new value (if applicable).
    pub new_value: Option<String>,
    /// Additional detail about the changed element.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum XmlChangeKind {
    ElementAdded,
    ElementDeleted,
    ElementModified,
    TextChanged,
    AttributeChanged,
    ChildCountChanged,
}

/// Result of an XML tree comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XmlTreeDiffResult {
    pub changes: Vec<XmlTreeChange>,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub total_modifications: usize,
}

// ── Public API ──────────────────────────────────────────────────────────

/// Compare two Word document XMLs structurally.
///
/// Walks both event streams in parallel depth-first, matching elements
/// by qualified name. Reports structural differences as a flat list
/// of `XmlTreeChange` entries.
pub fn compare_xml_trees(left_xml: &[u8], right_xml: &[u8]) -> XmlResult<XmlTreeDiffResult> {
    let mut changes = Vec::new();

    let left_events = collect_events(left_xml)?;
    let right_events = collect_events(right_xml)?;

    walk_and_compare(&left_events, &right_events, &mut String::new(), &mut changes);

    let total_additions = changes
        .iter()
        .filter(|c| c.kind == XmlChangeKind::ElementAdded)
        .count();
    let total_deletions = changes
        .iter()
        .filter(|c| c.kind == XmlChangeKind::ElementDeleted)
        .count();
    let total_modifications = changes.len() - total_additions - total_deletions;

    Ok(XmlTreeDiffResult {
        changes,
        total_additions,
        total_deletions,
        total_modifications,
    })
}

// ── Event collection ───────────────────────────────────────────────────

/// A simplified representation of an XML event for comparison purposes.
#[derive(Debug, Clone)]
enum SimpleEvent {
    Start {
        name: String,
        /// Sorted, normalized attributes.
        attrs: Vec<(String, String)>,
    },
    End {
        _name: String,
    },
    Text {
        content: String,
    },
}

/// Flatten an XML byte slice into a sequence of `SimpleEvent`s.
fn collect_events(xml: &[u8]) -> XmlResult<Vec<SimpleEvent>> {
    let mut reader = xml_util::create_reader(xml);
    let mut buf = Vec::new();
    let mut events = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = format_name(e.name());
                let mut attrs: Vec<(String, String)> = e
                    .attributes()
                    .flatten()
                    .map(|a| {
                        let key = format_attr_name(&a);
                        let val = String::from_utf8_lossy(&a.value).to_string();
                        (key, val)
                    })
                    .collect();
                attrs.sort_by(|a, b| a.0.cmp(&b.0)); // Normalize attribute order
                events.push(SimpleEvent::Start { name, attrs });
            }
            Ok(Event::End(ref e)) => {
                let name = format_name(e.name());
                events.push(SimpleEvent::End { _name: name });
            }
            Ok(Event::Text(ref t)) => {
                let content = t.unescape().unwrap_or_default().trim().to_owned();
                if !content.is_empty() {
                    events.push(SimpleEvent::Text { content });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(events)
}

// ── Tree walking ───────────────────────────────────────────────────────

/// Walk two event lists in parallel and report structural differences.
fn walk_and_compare(
    left: &[SimpleEvent],
    right: &[SimpleEvent],
    path: &mut String,
    changes: &mut Vec<XmlTreeChange>,
) {
    let mut li = 0usize;
    let mut ri = 0usize;

    while li < left.len() && ri < right.len() {
        match (&left[li], &right[ri]) {
            // Both are Start events with same name → recurse into children.
            (SimpleEvent::Start { name: ln, attrs: la }, SimpleEvent::Start { name: rn, attrs: ra })
                if ln == rn =>
            {
                // Compare attributes.
                if la != ra {
                    changes.push(XmlTreeChange {
                        path: path_with(path, ln),
                        kind: XmlChangeKind::AttributeChanged,
                        old_value: Some(format!("{:?}", la)),
                        new_value: Some(format!("{:?}", ra)),
                        detail: format!("Attributes differ on <{}>", ln),
                    });
                }

                // Find end events for both and compare children.
                let left_children = extract_children(left, li);
                let right_children = extract_children(right, ri);

                let old_path_len = path.len();
                push_path(path, ln);

                walk_and_compare(
                    &left[li + 1..li + 1 + left_children],
                    &right[ri + 1..ri + 1 + right_children],
                    path,
                    changes,
                );

                path.truncate(old_path_len);
                li += 1 + left_children + 1; // skip past End
                ri += 1 + right_children + 1;
            }

            // Left has element, right doesn't → deletion.
            (SimpleEvent::Start { name: ln, .. }, _) => {
                changes.push(XmlTreeChange {
                    path: path_with(path, ln),
                    kind: XmlChangeKind::ElementDeleted,
                    old_value: None,
                    new_value: None,
                    detail: format!("Element <{}> deleted", ln),
                });
                li += skip_element(left, li);
            }

            // Right has element, left doesn't → addition.
            (_, SimpleEvent::Start { name: rn, .. }) => {
                changes.push(XmlTreeChange {
                    path: path_with(path, rn),
                    kind: XmlChangeKind::ElementAdded,
                    old_value: None,
                    new_value: None,
                    detail: format!("Element <{}> added", rn),
                });
                ri += skip_element(right, ri);
            }

            // Text node comparison.
            (SimpleEvent::Text { content: lc }, SimpleEvent::Text { content: rc }) => {
                if lc != rc {
                    changes.push(XmlTreeChange {
                        path: path.clone(),
                        kind: XmlChangeKind::TextChanged,
                        old_value: Some(lc.clone()),
                        new_value: Some(rc.clone()),
                        detail: "Text content changed".to_owned(),
                    });
                }
                li += 1;
                ri += 1;
            }

            _ => {
                // Mismatch: skip ahead on both sides.
                li += 1;
                ri += 1;
            }
        }
    }

    // Remaining left events → deletions.
    while li < left.len() {
        if let SimpleEvent::Start { name, .. } = &left[li] {
            changes.push(XmlTreeChange {
                path: path_with(path, name),
                kind: XmlChangeKind::ElementDeleted,
                old_value: None,
                new_value: None,
                detail: format!("Element <{}> deleted (trailing)", name),
            });
            li += skip_element(left, li);
        } else {
            li += 1;
        }
    }

    // Remaining right events → additions.
    while ri < right.len() {
        if let SimpleEvent::Start { name, .. } = &right[ri] {
            changes.push(XmlTreeChange {
                path: path_with(path, name),
                kind: XmlChangeKind::ElementAdded,
                old_value: None,
                new_value: None,
                detail: format!("Element <{}> added (trailing)", name),
            });
            ri += skip_element(right, ri);
        } else {
            ri += 1;
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Extract the child events between a Start and its matching End.
/// Returns the number of child events (not counting outer Start/End).
fn extract_children(events: &[SimpleEvent], start_idx: usize) -> usize {
    let mut depth = 0u32;
    let mut count = 0usize;
    for i in (start_idx + 1)..events.len() {
        match &events[i] {
            SimpleEvent::Start { .. } => {
                depth += 1;
                count += 1;
            }
            SimpleEvent::End { .. } => {
                if depth == 0 {
                    return count;
                }
                depth -= 1;
                count += 1;
            }
            SimpleEvent::Text { .. } => {
                count += 1;
            }
        }
    }
    count
}

/// Skip past an element and all its children. Returns the number of
/// events to advance (including the End event).
fn skip_element(events: &[SimpleEvent], start_idx: usize) -> usize {
    1 + extract_children(events, start_idx) + 1 // Start + children + End
}

/// Format a QName into a prefixed string like `w:p`.
///
/// In quick-xml 0.37, QName does not expose namespace resolution.
/// We check the raw bytes for known prefixes.
fn format_name(name: QName<'_>) -> String {
    let raw = name.as_ref();
    if raw.len() > 2 && &raw[..2] == b"w:" {
        String::from_utf8_lossy(raw).to_string()
    } else if raw.len() > 2 && &raw[..2] == b"a:" {
        String::from_utf8_lossy(raw).to_string()
    } else if raw.len() > 2 && &raw[..2] == b"r:" {
        String::from_utf8_lossy(raw).to_string()
    } else {
        String::from_utf8_lossy(raw).to_string()
    }
}

/// Format an attribute name into a prefixed string.
fn format_attr_name(attr: &quick_xml::events::attributes::Attribute<'_>) -> String {
    let raw = attr.key.as_ref();
    String::from_utf8_lossy(raw).to_string()
}

/// Append a segment to the mutable path.
fn push_path(path: &mut String, name: &str) {
    if !path.is_empty() {
        path.push('/');
    }
    path.push_str(name);
}

/// Build a path string without mutating (for change reporting).
fn path_with(path: &str, name: &str) -> String {
    if path.is_empty() {
        format!("/{}", name)
    } else {
        format!("{}/{}", path, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_xml_no_changes() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello</w:t></w:r></w:p></w:body></w:document>"#;

        let result = compare_xml_trees(xml.as_bytes(), xml.as_bytes()).unwrap();
        assert!(result.changes.is_empty());
        assert_eq!(result.total_additions, 0);
    }

    #[test]
    fn test_text_change_detected() {
        let left = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello left</w:t></w:r></w:p></w:body></w:document>"#;
        let right = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello right</w:t></w:r></w:p></w:body></w:document>"#;

        let result = compare_xml_trees(left.as_bytes(), right.as_bytes()).unwrap();
        assert!(!result.changes.is_empty());
        assert!(
            result
                .changes
                .iter()
                .any(|c| c.kind == XmlChangeKind::TextChanged),
            "should detect text change"
        );
    }

    #[test]
    fn test_added_paragraph_detected() {
        let left = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Para 1</w:t></w:r></w:p></w:body></w:document>"#;
        let right = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Para 1</w:t></w:r></w:p><w:p><w:r><w:t>Para 2</w:t></w:r></w:p></w:body></w:document>"#;

        let result = compare_xml_trees(left.as_bytes(), right.as_bytes()).unwrap();
        assert!(result.total_additions > 0, "should detect added paragraph");
    }

    #[test]
    fn test_deleted_paragraph_detected() {
        let left = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Para 1</w:t></w:r></w:p><w:p><w:r><w:t>Para 2</w:t></w:r></w:p></w:body></w:document>"#;
        let right = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Para 1</w:t></w:r></w:p></w:body></w:document>"#;

        let result = compare_xml_trees(left.as_bytes(), right.as_bytes()).unwrap();
        assert!(result.total_deletions > 0, "should detect deleted paragraph");
    }

    #[test]
    fn test_serialize_result() {
        let result = XmlTreeDiffResult {
            changes: vec![XmlTreeChange {
                path: "/w:document/w:body/w:p".to_owned(),
                kind: XmlChangeKind::TextChanged,
                old_value: Some("old".to_owned()),
                new_value: Some("new".to_owned()),
                detail: "Text changed".to_owned(),
            }],
            total_additions: 0,
            total_deletions: 0,
            total_modifications: 1,
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("text-changed"));
        assert!(json.contains("old"));
        assert!(json.contains("new"));
    }
}
