//! Semantic PowerPoint comparison (Approach 3: Semantic Model).
//!
//! Compares two PowerPoint presentations at the slide level. Each slide's
//! text is extracted via `extract_slide_text()`, then slides are matched
//! by index and compared section-by-section.
//!
//! This produces `OfficeSectionDiff` entries that can be rendered by the
//! CLI, desktop UI, and AI pipelines.

use crate::models::*;
use crate::powerpoint::extract;
use crate::xml_util::XmlResult;

/// Compare two presentations, slide-by-slide by index.
///
/// `left_slides` and `right_slides` are ordered lists of slide XML bytes
/// (slide 1, slide 2, …). The caller is responsible for sorting the slide
/// parts numerically before calling this function.
///
/// Each slide is extracted to plain text and compared. Slides present in
/// both presentations are compared character-by-character; slides present
/// in only one are reported as Added or Deleted.
pub fn compare_presentations(
    left_slides: &[&[u8]],
    right_slides: &[&[u8]],
) -> XmlResult<Vec<OfficeSectionDiff>> {
    let max_slides = left_slides.len().max(right_slides.len());
    let mut sections = Vec::with_capacity(max_slides);

    for i in 0..max_slides {
        let left_xml = left_slides.get(i);
        let right_xml = right_slides.get(i);

        let (status, changes, label) = match (left_xml, right_xml) {
            (Some(_lx), None) => {
                // Slide deleted in right.
                let left_text = extract::extract_slide_text(left_xml.unwrap())?;
                let trimmed: String = left_text.chars().take(50).collect();
                let lbl = if trimmed.len() < left_text.len() {
                    format!("Slide {}: {}…", i + 1, trimmed)
                } else {
                    format!("Slide {}: {}", i + 1, trimmed)
                };
                (
                    ChangeStatus::Deleted,
                    vec![OfficeAtomicChange {
                        path: format!("slide={}", i + 1),
                        kind: OfficeAtomicChangeKind::TextDeleted,
                        before: Some(left_text),
                        after: None,
                        metadata: serde_json::Value::Null,
                    }],
                    lbl,
                )
            }
            (None, Some(_rx)) => {
                // Slide added in right.
                let right_text =
                    extract::extract_slide_text(right_xml.unwrap())?;
                let trimmed: String = right_text.chars().take(50).collect();
                let lbl = if trimmed.len() < right_text.len() {
                    format!("Slide {}: {}…", i + 1, trimmed)
                } else {
                    format!("Slide {}: {}", i + 1, trimmed)
                };
                (
                    ChangeStatus::Added,
                    vec![OfficeAtomicChange {
                        path: format!("slide={}", i + 1),
                        kind: OfficeAtomicChangeKind::TextAdded,
                        before: None,
                        after: Some(right_text),
                        metadata: serde_json::Value::Null,
                    }],
                    lbl,
                )
            }
            (Some(lx), Some(rx)) => {
                let left_text = extract::extract_slide_text(lx)?;
                let right_text = extract::extract_slide_text(rx)?;

                let trimmed: String = left_text.chars().take(50).collect();
                let lbl = if trimmed.len() < left_text.len() {
                    format!("Slide {}: {}…", i + 1, trimmed)
                } else {
                    format!("Slide {}: {}", i + 1, trimmed)
                };

                if left_text == right_text {
                    (ChangeStatus::Unchanged, Vec::new(), lbl)
                } else {
                    (
                        ChangeStatus::Modified,
                        vec![OfficeAtomicChange {
                            path: format!("slide={}", i + 1),
                            kind: OfficeAtomicChangeKind::TextChanged,
                            before: Some(left_text),
                            after: Some(right_text),
                            metadata: serde_json::Value::Null,
                        }],
                        lbl,
                    )
                }
            }
            (None, None) => unreachable!(),
        };

        sections.push(OfficeSectionDiff {
            id: format!("slide-{}", i + 1),
            label,
            kind: OfficeSectionKind::PowerPointSlide,
            status,
            changes,
        });
    }

    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal slide XML helper — a single shape with the given text, no placeholder.
    fn minimal_slide(text: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Text"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p><a:r><a:t>{text}</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#
        )
    }

    /// Slide with a title placeholder and body text.
    fn title_body_slide(title: &str, body: &str) -> String {
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
          <a:p><a:r><a:t>{title}</a:t></a:r></a:p>
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
          <a:p><a:r><a:t>{body}</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#
        )
    }

    #[test]
    fn test_identical_slides_no_changes() {
        let slide = minimal_slide("Hello world");
        let slide_bytes = slide.as_bytes();

        let sections = compare_presentations(&[slide_bytes], &[slide_bytes]).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert!(sections[0].changes.is_empty());
        assert_eq!(sections[0].kind, OfficeSectionKind::PowerPointSlide);
    }

    #[test]
    fn test_text_change_detected() {
        let left = minimal_slide("Quarterly Results");
        let right = minimal_slide("Annual Results");

        let sections =
            compare_presentations(&[left.as_bytes()], &[right.as_bytes()]).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        assert_eq!(sections[0].changes.len(), 1);
        assert_eq!(
            sections[0].changes[0].kind,
            OfficeAtomicChangeKind::TextChanged
        );
        assert!(sections[0].changes[0]
            .before
            .as_deref()
            .unwrap()
            .contains("Quarterly Results"));
        assert!(sections[0].changes[0]
            .after
            .as_deref()
            .unwrap()
            .contains("Annual Results"));
    }

    #[test]
    fn test_added_slide() {
        let left = minimal_slide("Slide 1");
        let right1 = minimal_slide("Slide 1");
        let right2 = minimal_slide("Slide 2");

        let sections = compare_presentations(
            &[left.as_bytes()],
            &[right1.as_bytes(), right2.as_bytes()],
        )
        .unwrap();

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[1].status, ChangeStatus::Added);
        assert_eq!(sections[1].changes[0].kind, OfficeAtomicChangeKind::TextAdded);
    }

    #[test]
    fn test_deleted_slide() {
        let left1 = minimal_slide("Slide 1");
        let left2 = minimal_slide("Slide 2");
        let right = minimal_slide("Slide 1");

        let sections = compare_presentations(
            &[left1.as_bytes(), left2.as_bytes()],
            &[right.as_bytes()],
        )
        .unwrap();

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[1].status, ChangeStatus::Deleted);
        assert_eq!(
            sections[1].changes[0].kind,
            OfficeAtomicChangeKind::TextDeleted
        );
    }

    #[test]
    fn test_multiple_slides_mixed_changes() {
        let l1 = minimal_slide("Title slide");
        let l2 = minimal_slide("Agenda v1");
        let l3 = minimal_slide("Summary");

        let r1 = minimal_slide("Title slide"); // unchanged
        let r2 = minimal_slide("Agenda v2"); // modified
        let r3_new = minimal_slide("Q&A"); // slide 3 different content (Modified, not Deleted)

        let sections = compare_presentations(
            &[l1.as_bytes(), l2.as_bytes(), l3.as_bytes()],
            &[r1.as_bytes(), r2.as_bytes(), r3_new.as_bytes()],
        )
        .unwrap();

        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[1].status, ChangeStatus::Modified);
        // Slide 3: left had "Summary", right has "Q&A" — both exist so it's Modified,
        // not Deleted. This is correct since both sides have a slide at index 2.
        assert_eq!(sections[2].status, ChangeStatus::Modified);
    }

    #[test]
    fn test_title_body_slide_label() {
        let left = title_body_slide("My Title", "Some body text.");
        let right = title_body_slide("My Title", "Different body text.");

        let sections =
            compare_presentations(&[left.as_bytes()], &[right.as_bytes()]).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        assert_eq!(sections[0].kind, OfficeSectionKind::PowerPointSlide);
        // Label is derived from the left side's text.
        assert!(sections[0].label.contains("Slide 1:"));
        assert!(sections[0].label.contains("[title]"));
    }

    #[test]
    fn test_empty_slide() {
        let empty = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree/>
  </p:cSld>
</p:sld>"#;

        let sections =
            compare_presentations(&[empty.as_bytes()], &[empty.as_bytes()]).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
    }

    #[test]
    fn test_numeric_sorting_independent_of_caller() {
        // The caller passes slides in order; the function just compares by index.
        // Verify that comparing 3 identical slides at different positions works.
        let s1 = minimal_slide("A");
        let s2 = minimal_slide("B");
        let s3 = minimal_slide("C");

        let sections = compare_presentations(
            &[s1.as_bytes(), s2.as_bytes(), s3.as_bytes()],
            &[s1.as_bytes(), s2.as_bytes(), s3.as_bytes()],
        )
        .unwrap();

        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[1].status, ChangeStatus::Unchanged);
        assert_eq!(sections[2].status, ChangeStatus::Unchanged);
        assert!(sections[0].label.contains("Slide 1"));
        assert!(sections[2].label.contains("Slide 3"));
    }

    #[test]
    fn test_serialize_sections() {
        let sections = vec![OfficeSectionDiff {
            id: "slide-1".to_owned(),
            label: "Slide 1: Quarterly Results".to_owned(),
            kind: OfficeSectionKind::PowerPointSlide,
            status: ChangeStatus::Modified,
            changes: vec![OfficeAtomicChange {
                path: "slide=1".to_owned(),
                kind: OfficeAtomicChangeKind::TextChanged,
                before: Some("Q1 Revenue: $1M".to_owned()),
                after: Some("Q2 Revenue: $1.2M".to_owned()),
                metadata: serde_json::Value::Null,
            }],
        }];

        let json = serde_json::to_string_pretty(&sections).unwrap();
        assert!(json.contains("power-point-slide"));
        assert!(json.contains("Q1 Revenue: $1M"));
        assert!(json.contains("Q2 Revenue: $1.2M"));
    }
}
