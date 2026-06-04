//! AST matching algorithm.
//!
//! Implements a simplified bottom-up container matching algorithm
//! (GumTree-style) for comparing two lists of AstNode records.

use crate::parser::AstNode;
use std::collections::HashMap;

/// The result of comparing two ASTs.
#[derive(Debug, Clone)]
pub struct AstDiff {
    /// Matched node pairs (left_id, right_id, classification).
    pub matches: Vec<NodeMatch>,
    /// Left node IDs with no right partner.
    pub left_unmatched: Vec<usize>,
    /// Right node IDs with no left partner.
    pub right_unmatched: Vec<usize>,
}

/// A single matched node pair.
#[derive(Debug, Clone)]
pub struct NodeMatch {
    pub left_id: usize,
    pub right_id: usize,
    pub classification: ChangeClass,
    /// Similarity score 0.0–1.0.
    pub similarity: f64,
}

/// Semantic classification for a matched or unmatched node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeClass {
    Identical,
    Modified,
    Moved,
    Renamed,
    Added,
    Deleted,
    Reformatted,
}

/// Compute the AST diff between two flat node lists.
///
/// Runs three passes:
/// 1. Hash-based container matching (bottom-up)
/// 2. Positional heuristic for moves/renames
/// 3. Remaining nodes are additions/deletions
pub fn compute_ast_diff(left: &[AstNode], right: &[AstNode]) -> AstDiff {
    // Pass 1: hash-based container matching.
    let mut matches: Vec<(usize, usize, ChangeClass, f64)> = Vec::new();
    let mut left_matched = vec![false; left.len()];
    let mut right_matched = vec![false; right.len()];

    // Build hash → Vec<index> maps.
    let mut left_by_hash: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut right_by_hash: HashMap<u64, Vec<usize>> = HashMap::new();

    for (i, node) in left.iter().enumerate() {
        left_by_hash.entry(node.hash).or_default().push(i);
    }
    for (i, node) in right.iter().enumerate() {
        right_by_hash.entry(node.hash).or_default().push(i);
    }

    // Match nodes with unique hash on each side.
    for (hash, left_indices) in &left_by_hash {
        if left_indices.len() != 1 {
            continue;
        }
        let right_indices = match right_by_hash.get(hash) {
            Some(r) if r.len() == 1 => r,
            _ => continue,
        };
        let l = left_indices[0];
        let r = right_indices[0];

        // Check that the node kinds match.
        if left[l].kind != right[r].kind {
            continue;
        }

        let class = classify_node_match(&left[l], &right[r]);
        let sim = compute_similarity(&left[l], &right[r]);
        matches.push((l, r, class, sim));
        left_matched[l] = true;
        right_matched[r] = true;
    }

    // Bottom-up parent matching: iteratively match parent nodes whose
    // children are all already matched. Keep looping since matching a parent
    // may enable matching its own parent (grandparent).
    loop {
        let before = matches.len();
        for l in 0..left.len() {
            if left_matched[l] {
                continue;
            }
            let left_node = &left[l];
            if left_node.children.is_empty() {
                continue;
            }
            let all_children_matched = left_node
                .children
                .iter()
                .all(|&c| c < left_matched.len() && left_matched[c]);

            if !all_children_matched {
                continue;
            }

            for r in 0..right.len() {
                if right_matched[r] {
                    continue;
                }
                let right_node = &right[r];
                if right_node.kind != left_node.kind {
                    continue;
                }
                if right_node.children.is_empty() {
                    continue;
                }
                let all_right_children_matched = right_node
                    .children
                    .iter()
                    .all(|&c| c < right_matched.len() && right_matched[c]);

                if !all_right_children_matched {
                    continue;
                }

                let child_pairs_match = left_node
                    .children
                    .iter()
                    .zip(right_node.children.iter())
                    .all(|(&lc, &rc)| {
                        matches.iter().any(|m| m.0 == lc && m.1 == rc)
                    });

                if child_pairs_match {
                    let class = classify_node_match(left_node, right_node);
                    let sim = compute_similarity(left_node, right_node);
                    matches.push((l, r, class, sim));
                    left_matched[l] = true;
                    right_matched[r] = true;
                    break;
                }
            }
        }
        if matches.len() == before {
            break;
        }
    }

    // Pass 2: propagate matches from parents to children in order.
    // For each matched parent pair, walk their children and match them
    // 1-to-1 by position.
    let mut added_new = true;
    while added_new {
        added_new = false;
        for m_idx in 0..matches.len() {
            let (l, r, _, _) = matches[m_idx];
            let left_children = &left[l].children;
            let right_children = &right[r].children;
            let child_count = left_children.len().min(right_children.len());
            for i in 0..child_count {
                let lc = left_children[i];
                let rc = right_children[i];
                if lc < left_matched.len() && !left_matched[lc]
                    && rc < right_matched.len() && !right_matched[rc]
                {
                    let class = classify_node_match(&left[lc], &right[rc]);
                    let sim = compute_similarity(&left[lc], &right[rc]);
                    matches.push((lc, rc, class, sim));
                    left_matched[lc] = true;
                    right_matched[rc] = true;
                    added_new = true;
                }
            }
        }
    }

    // Pass 3: positional heuristic for moves/renames.
    // For each unmatched pair with the same kind, check if they have
    // similar content.
    for l in 0..left.len() {
        if left_matched[l] {
            continue;
        }
        for r in 0..right.len() {
            if right_matched[r] {
                continue;
            }
            if left[l].kind != right[r].kind {
                continue;
            }

            let sim = compute_similarity(&left[l], &right[r]);
            if sim > 0.5 {
                let class = classify_node_match(&left[l], &right[r]);
                matches.push((l, r, class, sim));
                left_matched[l] = true;
                right_matched[r] = true;
                break;
            }
        }
    }

    // Pass 4: remaining unmatched nodes.
    let mut left_unmatched: Vec<usize> = left
        .iter()
        .enumerate()
        .filter(|(i, _)| !left_matched[*i])
        .map(|(i, _)| i)
        .collect();
    let mut right_unmatched: Vec<usize> = right
        .iter()
        .enumerate()
        .filter(|(i, _)| !right_matched[*i])
        .map(|(i, _)| i)
        .collect();

    left_unmatched.sort();
    right_unmatched.sort();

    let node_matches: Vec<NodeMatch> = matches
        .into_iter()
        .map(|(l, r, c, s)| NodeMatch {
            left_id: l,
            right_id: r,
            classification: c,
            similarity: s,
        })
        .collect();

    AstDiff {
        matches: node_matches,
        left_unmatched,
        right_unmatched,
    }
}

/// Classify the relationship between two matched AST nodes.
fn classify_node_match(left: &AstNode, right: &AstNode) -> ChangeClass {
    // Check for moves first: same text, different position.
    if left.text == right.text {
        if left.start_byte != right.start_byte || left.end_byte != right.end_byte {
            return ChangeClass::Moved;
        }
        return ChangeClass::Identical;
    }

    // Check if it's a reformat (only whitespace differs).
    if left.hash == right.hash {
        return ChangeClass::Reformatted;
    }

    // Check if it's a rename (kind is name-bearing and content is similar).
    if is_name_node(left) || is_name_node(right) {
        return ChangeClass::Renamed;
    }

    ChangeClass::Modified
}

/// Check if a node is name-bearing (identifier, variable name, etc.).
fn is_name_node(node: &AstNode) -> bool {
    matches!(node.kind.as_str(), "identifier" | "variable_name" | "type_identifier")
}

/// Compute a similarity score between two AST nodes (0.0–1.0).
fn compute_similarity(left: &AstNode, right: &AstNode) -> f64 {
    if left.text == right.text {
        return 1.0;
    }
    if left.hash == right.hash {
        return 0.95;
    }
    if left.kind != right.kind {
        return 0.0;
    }

    // Levenshtein-based similarity on normalized text.
    let left_chars: Vec<char> = left.text.chars().collect();
    let right_chars: Vec<char> = right.text.chars().collect();
    let dist = levenshtein_distance(&left_chars, &right_chars);
    let max_len = left_chars.len().max(right_chars.len()) as f64;
    if max_len == 0.0 {
        return 1.0;
    }
    1.0 - (dist as f64 / max_len)
}

/// Compute Levenshtein edit distance between two character sequences.
fn levenshtein_distance(a: &[char], b: &[char]) -> usize {
    let m = a.len();
    let n = b.len();

    // Use a single row optimization for small-ish inputs.
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // For inputs larger than 200 chars, use a cheap approximation
    // to avoid O(n²) blowup on large text blocks.
    if m.max(n) > 200 {
        // Approximate: hamming-like distance based on length difference
        // plus hash comparison.
        return m.abs_diff(n) + if a != b { m.min(n) / 2 } else { 0 };
    }

    let mut prev_row: Vec<usize> = (0..=n).collect();
    let mut curr_row: Vec<usize> = vec![0; n + 1];

    for i in 1..=m {
        curr_row[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[n]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_file, build_child_links, Language, SyntaxDepth};

    fn parse(text: &str, lang: Language) -> Vec<AstNode> {
        let mut nodes = parse_file(text, lang, SyntaxDepth::Statement).unwrap();
        build_child_links(&mut nodes);
        nodes
    }

    #[test]
    fn identical_trees_match_all() {
        let left = parse("fn a() { }\nfn b() { }", Language::Rust);
        let right = parse("fn a() { }\nfn b() { }", Language::Rust);
        let diff = compute_ast_diff(&left, &right);
        assert_eq!(diff.left_unmatched.len(), 0);
        assert_eq!(diff.right_unmatched.len(), 0);
        assert!(diff.matches.iter().all(|m| m.classification == ChangeClass::Identical));
    }

    #[test]
    fn modified_function_detected() {
        let left = parse("fn a() { 1 }", Language::Rust);
        let right = parse("fn a() { 2 }", Language::Rust);
        let diff = compute_ast_diff(&left, &right);
        let modified: Vec<_> = diff.matches.iter().filter(|m| m.classification == ChangeClass::Modified).collect();
        assert!(!modified.is_empty(), "expected at least one modified match");
    }

    #[test]
    fn added_function_detected() {
        let left = parse("fn a() { }", Language::Rust);
        let right = parse("fn a() { }\nfn b() { }", Language::Rust);
        let diff = compute_ast_diff(&left, &right);
        assert!(!diff.right_unmatched.is_empty(), "expected unmatched right nodes");
    }

    #[test]
    fn deleted_function_detected() {
        let left = parse("fn a() { }\nfn b() { }", Language::Rust);
        let right = parse("fn a() { }", Language::Rust);
        let diff = compute_ast_diff(&left, &right);
        assert!(!diff.left_unmatched.is_empty(), "expected unmatched left nodes");
    }

    #[test]
    fn moved_function_detected() {
        let left = parse("fn a() { }\nfn b() { }", Language::Rust);
        let right = parse("fn b() { }\nfn a() { }", Language::Rust);
        let diff = compute_ast_diff(&left, &right);
        let moved: Vec<_> = diff.matches.iter().filter(|m| m.classification == ChangeClass::Moved).collect();
        assert!(!moved.is_empty(), "expected at least one moved match");
    }

    #[test]
    fn reformatted_function_identified() {
        let left = parse("fn  a ( ) { }", Language::Rust);
        let right = parse("fn a() { }", Language::Rust);
        let diff = compute_ast_diff(&left, &right);
        let reformatted: Vec<_> = diff.matches.iter().filter(|m| m.classification == ChangeClass::Reformatted).collect();
        assert!(!reformatted.is_empty(), "expected at least one reformatted match");
    }

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein_distance(&['a', 'b', 'c'], &['a', 'b', 'c']), 0);
    }

    #[test]
    fn levenshtein_completely_different() {
        assert_eq!(levenshtein_distance(&['a', 'b', 'c'], &['x', 'y', 'z']), 3);
    }

    #[test]
    fn levenshtein_partial() {
        assert_eq!(levenshtein_distance(&['k', 'i', 't', 't', 'e', 'n'], &['s', 'i', 't', 't', 'i', 'n', 'g']), 3);
    }
}
