//! Fuzzy-lite search over cheatsheets: case-insensitive subsequence matching
//! with a small scoring model so the best matches sort first.

use crate::sheet::{Entry, Sheet};

/// Score `text` against `query`: `Some(score)` when every whitespace-separated
/// query word appears as an in-order subsequence (case-insensitive), in any
/// word order, `None` otherwise. Higher is better.
///
/// Words are matched independently because people search by recall
/// ("pane split"), not by the curated wording ("Split pane"). Scoring rewards
/// consecutive runs and word/key-boundary hits so that e.g. "sp" prefers
/// "split pane" over "swap pane". Kept deliberately simple — the corpus is
/// hundreds of entries, not millions.
pub fn score(query: &str, text: &str) -> Option<u32> {
    let text: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    query.split_whitespace().try_fold(0u32, |total, token| {
        Some(total + score_token(token, &text)?)
    })
}

/// Subsequence score of one query word against the lowercased `text`.
fn score_token(token: &str, text: &[char]) -> Option<u32> {
    let query: Vec<char> = token.chars().flat_map(char::to_lowercase).collect();
    let mut score = 0u32;
    let mut qi = 0;
    let mut prev_matched = false;
    for (i, &c) in text.iter().enumerate() {
        if qi < query.len() && c == query[qi] {
            score += 1;
            if prev_matched {
                score += 3; // consecutive run
            }
            if i == 0 || text[i - 1].is_whitespace() || "-_+/".contains(text[i - 1]) {
                score += 2; // word boundary
            }
            qi += 1;
            prev_matched = true;
        } else {
            prev_matched = false;
        }
    }
    (qi == query.len()).then_some(score)
}

/// One entry plus its match score, for sorted rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredEntry {
    /// The matched binding.
    pub entry: Entry,
    /// Match quality; meaningless when the query is empty.
    pub score: u32,
}

/// A section that survived filtering, with its matched entries.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredSection {
    /// Section name.
    pub name: String,
    /// Sheet this section belongs to (merge key of the parent).
    pub sheet: String,
    /// Display title of the parent sheet, for grouping headers.
    pub sheet_title: String,
    /// Matched entries, best score first.
    pub entries: Vec<ScoredEntry>,
}

/// Filter and rank every entry across `sheets`. Empty query returns all
/// entries in their curated order with score 0.
pub fn filter(sheets: &[Sheet], query: &str) -> Vec<ScoredSection> {
    let query = query.trim();
    let mut out = Vec::new();
    for sheet in sheets {
        for section in &sheet.sections {
            let mut entries: Vec<ScoredEntry> = section
                .entries
                .iter()
                .filter_map(|entry| {
                    if query.is_empty() {
                        return Some(ScoredEntry {
                            entry: entry.clone(),
                            score: 0,
                        });
                    }
                    score(query, &entry.haystack()).map(|score| ScoredEntry {
                        entry: entry.clone(),
                        score,
                    })
                })
                .collect();
            if !query.is_empty() {
                entries.sort_by_key(|e| std::cmp::Reverse(e.score));
            }
            if !entries.is_empty() {
                out.push(ScoredSection {
                    name: section.name.clone(),
                    sheet: sheet.name.clone(),
                    sheet_title: sheet.display_title().to_string(),
                    entries,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything_with_zero_score() {
        assert_eq!(score("", "anything"), Some(0));
    }

    #[test]
    fn subsequence_must_match_in_order() {
        assert!(score("sp", "split pane").is_some());
        assert!(score("ps", "split pane").is_none());
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert!(score("CSP", "Ctrl Space prefix").is_some());
    }

    #[test]
    fn consecutive_word_boundary_beats_scattered() {
        let good = score("split", "split window").expect("matches");
        let bad = score("split", "swap pane left it").expect("matches");
        assert!(good > bad);
    }

    #[test]
    fn tokens_match_in_any_order() {
        assert!(score("pane split", "Split pane vertically").is_some());
    }

    #[test]
    fn every_token_must_match() {
        assert!(score("split xyz", "split pane").is_none());
    }

    #[test]
    fn extra_whitespace_is_ignored() {
        assert!(score("  split   pane ", "split pane").is_some());
        assert_eq!(score("   ", "anything"), Some(0));
    }

    #[test]
    fn whole_query_subsequence_still_matches() {
        // Regression: tokenizing must only widen matches, never drop one.
        for (query, text) in [
            ("sp", "split pane"),
            ("CSP", "Ctrl Space prefix"),
            ("split pane", "split pane"),
            ("split window", "split window"),
            ("prefix h", "prefix h Split pane vertically"),
        ] {
            assert!(score(query, text).is_some(), "{query:?} vs {text:?}");
        }
    }

    #[test]
    fn filter_drops_empty_sections_and_keeps_structure() {
        let sheet = Sheet {
            name: "tmux".into(),
            title: None,
            icon: None,
            command: None,
            sections: vec![
                crate::sheet::Section {
                    name: "Panes".into(),
                    entries: vec![Entry {
                        keys: vec!["Alt".into(), "Enter".into()],
                        description: "split pane vertically".into(),
                        tags: vec![],
                        probe: None,
                    }],
                },
                crate::sheet::Section {
                    name: "Sessions".into(),
                    entries: vec![Entry {
                        keys: vec!["d".into()],
                        description: "detach session".into(),
                        tags: vec![],
                        probe: None,
                    }],
                },
            ],
        };
        let hits = filter(&[sheet], "split");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Panes");
        assert_eq!(hits[0].sheet, "tmux");
    }
}
