//! Word document extraction and comparison.
//!
//! Handles `.docx` files: parses `word/document.xml`, `word/comments.xml`,
//! `word/styles.xml`, headers, footers, and other WordprocessingML parts.
//!
//! Three diff approaches are implemented:
//! - `extract` — plain-text extraction for line-level diff (Approach 1)
//! - `xml_diff` — structural XML tree comparison (Approach 2)
//! - `compare` — semantic document model comparison (Approach 3, recommended)

pub mod compare;
pub mod extract;
pub mod xml_diff;
