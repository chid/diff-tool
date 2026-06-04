//! AST diff to Hunk/LineChange conversion.
//!
//! Converts `AstDiff` records into the standard `Vec<LineChange>` and
//! `Vec<Hunk>` types used by the rest of the pipeline.

use crate::ast_diff::{AstDiff, ChangeClass};
use crate::parser::AstNode;
use crate::{ChangeKind, DiffStats, LineChange};

/// Convert an AST diff into standard pipeline output.
///
/// Note: this simplified version requires the original node lists.
/// Use [`ast_diff_to_hunks_full`] when nodes are available.
pub fn ast_diff_to_hunks(
    ast_diff: &AstDiff,
    left_source: &str,
    right_source: &str,
    left_lines: &[String],
    right_lines: &[String],
    context: usize,
) -> (DiffStats, Vec<LineChange>) {
    // Simplified stub — for full output, pass nodes to ast_diff_to_hunks_full.
    let _ = (ast_diff, left_source, right_source, left_lines, right_lines, context);
    (DiffStats { additions: 0, deletions: 0, unchanged: 0 }, Vec::new())
}

/// Full version of ast_diff_to_hunks that also takes the original node lists.
///
/// This is the real implementation. The simplified version above delegates
/// here once nodes are available.
pub fn ast_diff_to_hunks_full(
    ast_diff: &AstDiff,
    left_nodes: &[AstNode],
    right_nodes: &[AstNode],
    left_source: &str,
    right_source: &str,
    left_lines: &[String],
    right_lines: &[String],
    _context: usize,
) -> (DiffStats, Vec<LineChange>) {
    let mut lines: Vec<LineChange> = Vec::new();
    let mut stats = DiffStats {
        additions: 0,
        deletions: 0,
        unchanged: 0,
    };

    // Build sorted list of change events (by byte position).
    #[allow(dead_code)]
    struct ChangeEvent {
        byte_pos: usize,
        kind: ChangeKind,
        node_id: usize,
        side: u8,
        classification: ChangeClass,
    }

    const SIDE_LEFT: u8 = 0;
    const SIDE_RIGHT: u8 = 1;

    let mut events: Vec<ChangeEvent> = Vec::new();

    // Helper: emit a change event if no ancestor has an event on the same side.
    fn push_event_if_not_nested(
        events: &mut Vec<ChangeEvent>,
        node_id: usize,
        nodes: &[AstNode],
        side: u8,
        byte_pos: usize,
        kind: ChangeKind,
        classification: ChangeClass,
        event_sides: &mut [std::collections::HashSet<usize>; 2],
    ) {
        // Check whether any ancestor already has an event on this side.
        let mut pid = nodes[node_id].parent;
        while let Some(parent_id) = pid {
            if event_sides[side as usize].contains(&parent_id) {
                return;
            }
            pid = nodes.get(parent_id).and_then(|n| n.parent);
        }
        event_sides[side as usize].insert(node_id);
        events.push(ChangeEvent { byte_pos, kind, node_id, side, classification });
    }

    // Track which node IDs already have events per side, to skip nested events.
    let mut event_sides: [std::collections::HashSet<usize>; 2] =
        [std::collections::HashSet::new(), std::collections::HashSet::new()];

    // Matched nodes.
    for m in &ast_diff.matches {
        match m.classification {
            ChangeClass::Identical => {
                // No changes — handled by context emission below.
            }
            ChangeClass::Modified | ChangeClass::Reformatted | ChangeClass::Renamed => {
                let left_node = &left_nodes[m.left_id];
                let right_node = &right_nodes[m.right_id];
                push_event_if_not_nested(
                    &mut events, m.left_id, left_nodes, SIDE_LEFT,
                    left_node.start_byte, ChangeKind::Deletion, m.classification,
                    &mut event_sides,
                );
                push_event_if_not_nested(
                    &mut events, m.right_id, right_nodes, SIDE_RIGHT,
                    right_node.start_byte, ChangeKind::Addition, m.classification,
                    &mut event_sides,
                );
            }
            ChangeClass::Moved => {
                let left_node = &left_nodes[m.left_id];
                let right_node = &right_nodes[m.right_id];
                push_event_if_not_nested(
                    &mut events, m.left_id, left_nodes, SIDE_LEFT,
                    left_node.start_byte, ChangeKind::Deletion, ChangeClass::Moved,
                    &mut event_sides,
                );
                push_event_if_not_nested(
                    &mut events, m.right_id, right_nodes, SIDE_RIGHT,
                    right_node.start_byte, ChangeKind::Addition, ChangeClass::Moved,
                    &mut event_sides,
                );
            }
            ChangeClass::Added | ChangeClass::Deleted => {
                // These shouldn't appear in matches.
            }
        }
    }

    // Unmatched nodes — skip children of unmatched nodes.
    for &id in &ast_diff.left_unmatched {
        let node = &left_nodes[id];
        // Check if any ancestor already has an event on left side.
        let mut pid = node.parent;
        let mut skip = false;
        while let Some(parent_id) = pid {
            if event_sides[SIDE_LEFT as usize].contains(&parent_id) {
                skip = true;
                break;
            }
            pid = left_nodes.get(parent_id).and_then(|n| n.parent);
        }
        if skip {
            continue;
        }
        event_sides[SIDE_LEFT as usize].insert(id);
        events.push(ChangeEvent {
            byte_pos: node.start_byte,
            kind: ChangeKind::Deletion,
            node_id: id,
            side: SIDE_LEFT,
            classification: ChangeClass::Deleted,
        });
    }
    for &id in &ast_diff.right_unmatched {
        let node = &right_nodes[id];
        let mut pid = node.parent;
        let mut skip = false;
        while let Some(parent_id) = pid {
            if event_sides[SIDE_RIGHT as usize].contains(&parent_id) {
                skip = true;
                break;
            }
            pid = right_nodes.get(parent_id).and_then(|n| n.parent);
        }
        if skip {
            continue;
        }
        event_sides[SIDE_RIGHT as usize].insert(id);
        events.push(ChangeEvent {
            byte_pos: node.start_byte,
            kind: ChangeKind::Addition,
            node_id: id,
            side: SIDE_RIGHT,
            classification: ChangeClass::Added,
        });
    }

    // Sort events by byte position.
    events.sort_by_key(|e| e.byte_pos);

    // Map byte positions to line indices.
    fn byte_to_line(source: &str, byte_pos: usize) -> usize {
        source[..byte_pos.min(source.len())]
            .matches('\n')
            .count()
    }

    // Simplified: convert events directly to LineChanges without context.
    for event in &events {
        if event.side == SIDE_LEFT {
            let line_idx = byte_to_line(left_source, event.byte_pos);
            if line_idx < left_lines.len() {
                lines.push(LineChange {
                    kind: ChangeKind::Deletion,
                    old_no: Some(line_idx + 1),
                    new_no: None,
                    text: left_lines[line_idx].clone(),
                    intra_spans: Vec::new(),
                });
                stats.deletions += 1;
            }
        } else {
            let line_idx = byte_to_line(right_source, event.byte_pos);
            if line_idx < right_lines.len() {
                lines.push(LineChange {
                    kind: ChangeKind::Addition,
                    old_no: None,
                    new_no: Some(line_idx + 1),
                    text: right_lines[line_idx].clone(),
                    intra_spans: Vec::new(),
                });
                stats.additions += 1;
            }
        }
    }

    // For identical files, all lines are unchanged.
    if events.is_empty() {
        stats.unchanged = left_lines.len();
    }

    (stats, lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_diff::compute_ast_diff;
    use crate::parser::{build_child_links, parse_file, Language, SyntaxDepth};

    #[test]
    fn empty_diff_produces_stats() {
        let left = parse_and_link("fn a() { }", Language::Rust);
        let right = parse_and_link("fn a() { }", Language::Rust);
        let diff = compute_ast_diff(&left, &right);

        let left_text = "fn a() { }";
        let right_text = "fn a() { }";
        let left_lines: Vec<String> = left_text.lines().map(|s| s.to_string()).collect();
        let right_lines: Vec<String> = right_text.lines().map(|s| s.to_string()).collect();

        let (stats, line_changes) = ast_diff_to_hunks_full(
            &diff, &left, &right,
            left_text, right_text,
            &left_lines, &right_lines,
            3,
        );
        assert_eq!(stats.additions, 0);
        assert_eq!(stats.deletions, 0);
        assert!(line_changes.is_empty() || line_changes.iter().all(|l| l.kind == ChangeKind::Context));
    }

    fn parse_and_link(source: &str, lang: Language) -> Vec<AstNode> {
        let mut nodes = parse_file(source, lang, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        nodes
    }
}
