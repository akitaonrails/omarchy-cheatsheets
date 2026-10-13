//! Fuzzy-lite search over cheatsheets: case-insensitive subsequence matching
//! with a small scoring model so the best matches sort first.

use crate::sheet::{Entry, Section, Sheet};

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

/// Score one entry in context: its own keys/description/tags first, then
/// with its section and app names appended, so "copy mode" or "tmux quit"
/// find entries whose own text never says so. Context-only matches score at
/// half so that "pane" lists "Split pane" before every other row in Panes.
fn entry_score(query: &str, sheet: &Sheet, section: &Section, entry: &Entry) -> Option<u32> {
    let own = entry.haystack();
    score(query, &own).or_else(|| {
        let context = format!(
            "{own} {} {} {}",
            section.name,
            sheet.name,
            sheet.display_title()
        );
        score(query, &context).map(|score| score / 2)
    })
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
                    entry_score(query, sheet, section, entry).map(|score| ScoredEntry {
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

/// One matched entry with the labels needed to show it outside its sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// Sheet merge key, e.g. "tmux".
    pub sheet: String,
    /// Display title of the sheet.
    pub sheet_title: String,
    /// Nerd Font glyph of the sheet, when it has one.
    pub icon: Option<String>,
    /// Section the entry belongs to.
    pub section: String,
    /// The matched binding.
    pub entry: Entry,
    /// Match quality, higher is better.
    pub score: u32,
}

/// Rank every entry across `sheets` into one flat list, best match first,
/// capped at `limit`. Ties keep curated order. A blank query yields nothing:
/// the quick-search window shows a prompt instead of the whole corpus.
pub fn rank(sheets: &[Sheet], query: &str, limit: usize) -> Vec<Hit> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<Hit> = sheets
        .iter()
        .flat_map(|sheet| {
            sheet.sections.iter().flat_map(move |section| {
                section.entries.iter().filter_map(move |entry| {
                    entry_score(query, sheet, section, entry).map(|score| Hit {
                        sheet: sheet.name.clone(),
                        sheet_title: sheet.display_title().to_string(),
                        icon: sheet.icon.clone(),
                        section: section.name.clone(),
                        entry: entry.clone(),
                        score,
                    })
                })
            })
        })
        .collect();
    // sort_by_key is stable, which is what keeps ties in curated order.
    hits.sort_by_key(|hit| std::cmp::Reverse(hit.score));
    hits.truncate(limit);
    hits
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

    fn entry(description: &str) -> Entry {
        Entry {
            keys: vec!["x".into()],
            description: description.into(),
            tags: vec![],
            probe: None,
        }
    }

    fn sheet(name: &str, section: &str, descriptions: &[&str]) -> Sheet {
        Sheet {
            name: name.into(),
            title: None,
            icon: None,
            command: None,
            sections: vec![crate::sheet::Section {
                name: section.into(),
                entries: descriptions.iter().map(|d| entry(d)).collect(),
            }],
        }
    }

    fn descriptions(hits: &[Hit]) -> Vec<(&str, &str)> {
        hits.iter()
            .map(|h| (h.sheet.as_str(), h.entry.description.as_str()))
            .collect()
    }

    #[test]
    fn rank_orders_globally_across_sheets() {
        let sheets = [
            sheet("nvim", "Windows", &["swap pane left it"]),
            sheet("tmux", "Panes", &["split window"]),
        ];
        let hits = rank(&sheets, "split", 10);
        assert_eq!(
            descriptions(&hits),
            [("tmux", "split window"), ("nvim", "swap pane left it")]
        );
    }

    #[test]
    fn rank_ties_keep_curated_order() {
        let sheets = [
            sheet("nvim", "Buffers", &["close buffer", "close all"]),
            sheet("tmux", "Panes", &["close buffer"]),
        ];
        let hits = rank(&sheets, "close", 10);
        assert_eq!(
            descriptions(&hits),
            [
                ("nvim", "close buffer"),
                ("nvim", "close all"),
                ("tmux", "close buffer"),
            ]
        );
    }

    #[test]
    fn rank_respects_limit() {
        let sheets = [sheet("tmux", "Panes", &["close a", "close b", "close c"])];
        assert_eq!(rank(&sheets, "close", 2).len(), 2);
        assert!(rank(&sheets, "close", 0).is_empty());
    }

    #[test]
    fn rank_blank_query_returns_nothing() {
        let sheets = [sheet("tmux", "Panes", &["close pane"])];
        assert!(rank(&sheets, "", 10).is_empty());
        assert!(rank(&sheets, "   ", 10).is_empty());
    }

    #[test]
    fn rank_carries_sheet_and_section_labels() {
        let mut tmux = sheet("tmux", "Panes", &["split window"]);
        tmux.title = Some("tmux (prefix C-Space)".into());
        tmux.icon = Some("\u{ebc8}".into());
        let hits = rank(&[tmux], "split", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].sheet, "tmux");
        assert_eq!(hits[0].sheet_title, "tmux (prefix C-Space)");
        assert_eq!(hits[0].icon.as_deref(), Some("\u{ebc8}"));
        assert_eq!(hits[0].section, "Panes");
        assert!(hits[0].score > 0);
    }

    #[test]
    fn rank_matches_section_name() {
        let sheets = [
            sheet("tmux", "Copy mode", &["Begin selection"]),
            sheet("tmux", "Panes", &["Zoom"]),
        ];
        assert_eq!(
            descriptions(&rank(&sheets, "copy mode", 10)),
            [("tmux", "Begin selection")]
        );
    }

    #[test]
    fn rank_matches_app_name_and_title() {
        let mut nvim = sheet("nvim", "Basics", &["Quit"]);
        nvim.title = Some("Neovim (LazyVim)".into());
        let sheets = [nvim, sheet("tmux", "Basics", &["Quit"])];
        assert_eq!(
            descriptions(&rank(&sheets, "tmux quit", 10)),
            [("tmux", "Quit")]
        );
        assert_eq!(
            descriptions(&rank(&sheets, "lazyvim quit", 10)),
            [("nvim", "Quit")]
        );
    }

    #[test]
    fn own_text_match_outranks_section_only_match() {
        let sheets = [sheet("tmux", "Panes", &["Zoom", "Split pane"])];
        assert_eq!(
            descriptions(&rank(&sheets, "pane", 10)),
            [("tmux", "Split pane"), ("tmux", "Zoom")]
        );
    }

    #[test]
    fn filter_matches_section_name_too() {
        // The browser filter shares the matcher with quick search.
        let sheets = [sheet("tmux", "Copy mode", &["Begin selection"])];
        let hits = filter(&sheets, "copy mode");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entries[0].entry.description, "Begin selection");
    }
}
