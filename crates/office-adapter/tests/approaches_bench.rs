//! Integration test comparing all three Office document diff approaches.
//!
//! Generates reference `.docx` files with varied content and runs each
//! approach against them. Outputs are compared and analyzed to validate
//! the pros/cons documented in `docs/office-research/approaches.md`.
//!
//! Run with:
//! ```bash
//! cargo test -p office-adapter --test approaches_bench -- --nocapture
//! ```

mod common;

use diff_core::{compare_texts, DiffAlgorithm, IgnoreMode};
use office_adapter::models::*;
use office_adapter::word::{compare, extract, xml_diff};

// ── Scenario types ─────────────────────────────────────────────────────

/// A named test scenario with left/right docx body content.
struct Scenario {
    name: &'static str,
    description: &'static str,
    left_body: &'static str,
    right_body: &'static str,
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "simple-text-change",
            description: "Single paragraph text replacement",
            left_body: common::docx_left_body(),
            right_body: common::docx_right_body(),
        },
        Scenario {
            name: "formatting-only-change",
            description: "Bold removed but text unchanged — formatting only",
            left_body: common::docx_body_with_formatting(),
            right_body: common::docx_body_without_formatting(),
        },
        Scenario {
            name: "multi-paragraph-change",
            description: "Heading + 2 paragraphs, values changed",
            left_body: common::docx_body_heading_and_paras(),
            right_body: common::docx_body_heading_and_paras_modified(),
        },
        Scenario {
            name: "table-data-change",
            description: "Table with numeric data, Q2 values updated",
            left_body: common::docx_body_with_table_data(),
            right_body: common::docx_body_with_table_data_modified(),
        },
        Scenario {
            name: "tracked-changes",
            description: "Tracked changes in XML vs resolved version",
            left_body: common::docx_body_with_tracked_changes(),
            right_body: common::docx_body_tracked_changes_resolved(),
        },
    ]
}

// ── Test runner ────────────────────────────────────────────────────────

/// Run all three approaches against all scenarios and print results.
#[test]
fn compare_all_approaches() {
    let cases = scenarios();

    println!("\n");
    println!("═══════════════════════════════════════════════════════════════");
    println!("  OFFICE DOCUMENT DIFF — APPROACH COMPARISON");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    for scenario in &cases {
        println!("───────────────────────────────────────────────────────────────");
        println!("  SCENARIO: {} — {}", scenario.name, scenario.description);
        println!("───────────────────────────────────────────────────────────────");
        println!();

        // Generate .docx files from XML bodies.
        let left_docx = common::generate_docx(scenario.left_body);
        let right_docx = common::generate_docx(scenario.right_body);

        // Extract the word/document.xml for each approach.
        let left_xml = get_document_xml(&left_docx);
        let right_xml = get_document_xml(&right_docx);

        // ── Approach 1: Convert-to-text ────────────────────────────────
        println!("  📝 Approach 1: Convert-to-Text");
        let a1_text_change = match run_approach_1(&left_xml, &right_xml) {
            Ok(result) => {
                println!(
                    "     Text lines changed: {} additions, {} deletions, {} unchanged",
                    result.stats.additions, result.stats.deletions, result.stats.unchanged
                );
                if result.stats.additions == 0 && result.stats.deletions == 0 {
                    println!("     ⚠️  No changes detected!");
                }
                result.stats.additions + result.stats.deletions > 0
            }
            Err(e) => {
                println!("     ❌ Error: {}", e);
                false
            }
        };

        // ── Approach 2: XML tree diff ──────────────────────────────────
        println!("  🌲 Approach 2: XML Tree Diff");
        let a2_struct_change = match run_approach_2(&left_xml, &right_xml) {
            Ok(result) => {
                println!(
                    "     Structural changes: {} added, {} deleted, {} modified",
                    result.total_additions, result.total_deletions, result.total_modifications
                );
                if result.changes.is_empty() {
                    println!("     ⚠️  No changes detected!");
                }
                !result.changes.is_empty()
            }
            Err(e) => {
                println!("     ❌ Error: {}", e);
                false
            }
        };

        // ── Approach 3: Semantic model comparison ──────────────────────
        println!("  🧠 Approach 3: Semantic Model Comparison");
        let a3_semantic_change = match run_approach_3(&left_xml, &right_xml) {
            Ok(sections) => {
                let changed: Vec<_> = sections
                    .iter()
                    .filter(|s| s.status != ChangeStatus::Unchanged)
                    .collect();
                println!("     Sections with changes: {}", changed.len());
                for section in &changed {
                    println!(
                        "       • {} ({:?}): {} change(s)",
                        section.label,
                        section.status,
                        section.changes.len()
                    );
                    for c in &section.changes {
                        println!(
                            "         - {:?}: \"{}\" → \"{}\"",
                            c.kind,
                            c.before.as_deref().unwrap_or("(none)"),
                            c.after.as_deref().unwrap_or("(none)")
                        );
                    }
                }
                if changed.is_empty() {
                    println!("     ⚠️  No changes detected!");
                }
                !changed.is_empty()
            }
            Err(e) => {
                println!("     ❌ Error: {}", e);
                false
            }
        };

        // ── Summary for this scenario ──────────────────────────────────
        println!();
        println!("     📊 SUMMARY:");
        println!(
            "        A1 (text):       {}",
            if a1_text_change {
                "✅ detected"
            } else {
                "❌ missed"
            }
        );
        println!(
            "        A2 (xml tree):   {}",
            if a2_struct_change {
                "✅ detected"
            } else {
                "❌ missed"
            }
        );
        println!(
            "        A3 (semantic):   {}",
            if a3_semantic_change {
                "✅ detected"
            } else {
                "❌ missed"
            }
        );
        println!();
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("  Key observations:");
    println!("  - A1 (text) misses formatting-only changes (text is identical)");
    println!("  - A2 (xml tree) detects everything but can be overly sensitive");
    println!("  - A3 (semantic) gives the richest, most user-friendly output");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
}

// ── Approach implementations for the test ──────────────────────────────

/// Approach 1: Extract text and compare with diff-core.
fn run_approach_1(
    left_xml: &str,
    right_xml: &str,
) -> Result<diff_core::DiffResult, String> {
    let left_text =
        extract::extract_text(left_xml.as_bytes()).map_err(|e| e.to_string())?;
    let right_text =
        extract::extract_text(right_xml.as_bytes()).map_err(|e| e.to_string())?;

    Ok(compare_texts(
        "left.docx".into(),
        "right.docx".into(),
        &left_text,
        &right_text,
        IgnoreMode::None,
        false,
        3,
        Vec::new(),
        DiffAlgorithm::Myers,
    ))
}

/// Approach 2: XML tree structural comparison.
fn run_approach_2(
    left_xml: &str,
    right_xml: &str,
) -> Result<xml_diff::XmlTreeDiffResult, String> {
    xml_diff::compare_xml_trees(left_xml.as_bytes(), right_xml.as_bytes())
        .map_err(|e| e.to_string())
}

/// Approach 3: Semantic model comparison.
fn run_approach_3(
    left_xml: &str,
    right_xml: &str,
) -> Result<Vec<OfficeSectionDiff>, String> {
    compare::compare_documents(left_xml.as_bytes(), right_xml.as_bytes())
        .map_err(|e| e.to_string())
}

// ── Assertion tests (non-printing) ─────────────────────────────────────

#[test]
fn approach_1_detects_text_change() {
    let left = common::generate_docx(common::docx_left_body());
    let right = common::generate_docx(common::docx_right_body());
    let left_xml = get_document_xml(&left);
    let right_xml = get_document_xml(&right);

    let result = run_approach_1(&left_xml, &right_xml).unwrap();
    assert!(
        result.stats.additions + result.stats.deletions > 0,
        "Approach 1 should detect text change"
    );
}

#[test]
fn approach_2_detects_formatting_change() {
    let left = common::generate_docx(common::docx_body_with_formatting());
    let right = common::generate_docx(common::docx_body_without_formatting());
    let left_xml = get_document_xml(&left);
    let right_xml = get_document_xml(&right);

    let result = run_approach_2(&left_xml, &right_xml).unwrap();
    assert!(
        !result.changes.is_empty(),
        "Approach 2 should detect formatting change (bold removed)"
    );
}

#[test]
fn approach_3_gives_rich_semantic_output() {
    let left = common::generate_docx(common::docx_body_heading_and_paras());
    let right = common::generate_docx(common::docx_body_heading_and_paras_modified());
    let left_xml = get_document_xml(&left);
    let right_xml = get_document_xml(&right);

    let sections = run_approach_3(&left_xml, &right_xml).unwrap();

    // Should have 3 sections (heading + 2 body paragraphs)
    assert_eq!(sections.len(), 3, "should have 3 paragraphs");

    // Heading should be unchanged.
    assert_eq!(
        sections[0].status,
        ChangeStatus::Unchanged,
        "heading should be unchanged"
    );
    assert_eq!(sections[0].kind, OfficeSectionKind::WordHeading);

    // Body paragraphs should be modified.
    assert_eq!(
        sections[1].status,
        ChangeStatus::Modified,
        "first body paragraph should be modified"
    );
    assert_eq!(
        sections[2].status,
        ChangeStatus::Modified,
        "second body paragraph should be modified"
    );
}

#[test]
fn approach_3_detects_table_cell_changes() {
    let left = common::generate_docx(common::docx_body_with_table_data());
    let right = common::generate_docx(common::docx_body_with_table_data_modified());
    let left_xml = get_document_xml(&left);
    let right_xml = get_document_xml(&right);

    let sections = run_approach_3(&left_xml, &right_xml).unwrap();

    // Should have 2 sections: 1 paragraph + 1 table
    assert_eq!(sections.len(), 2);

    // The table should have changes.
    let table_section = sections.iter().find(|s| s.kind == OfficeSectionKind::WordTable);
    assert!(table_section.is_some(), "should find a table section");
    let table = table_section.unwrap();
    assert_eq!(table.status, ChangeStatus::Modified);
    assert!(!table.changes.is_empty(), "table should have cell changes");
}

#[test]
fn approach_1_misses_formatting_only_change() {
    // This test validates the documented limitation: Approach 1 cannot
    // see formatting changes because the extracted text is identical.
    let left = common::generate_docx(common::docx_body_with_formatting());
    let right = common::generate_docx(common::docx_body_without_formatting());
    let left_xml = get_document_xml(&left);
    let right_xml = get_document_xml(&right);

    let result = run_approach_1(&left_xml, &right_xml).unwrap();

    // Approach 1 should report 0 changes (text is "Important text here." in both).
    assert_eq!(
        result.stats.additions, 0,
        "Approach 1 should NOT detect formatting-only change"
    );
    assert_eq!(
        result.stats.deletions, 0,
        "Approach 1 should NOT detect formatting-only change"
    );
}

#[test]
fn approach_3_deserializes_to_json() {
    let left = common::generate_docx(common::docx_left_body());
    let right = common::generate_docx(common::docx_right_body());
    let left_xml = get_document_xml(&left);
    let right_xml = get_document_xml(&right);

    let sections = run_approach_3(&left_xml, &right_xml).unwrap();

    // Should serialize without error.
    let json = serde_json::to_string_pretty(&sections).unwrap();
    assert!(json.contains("Hello from the left file"));
    assert!(json.contains("Hello from the right file"));

    // Round-trip.
    let deserialized: Vec<OfficeSectionDiff> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), sections.len());
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Extract `word/document.xml` as a string from a generated .docx zip.
fn get_document_xml(zip_data: &[u8]) -> String {
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut doc_entry = archive.by_name("word/document.xml").unwrap();
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut doc_entry, &mut buf).unwrap();
    buf
}
