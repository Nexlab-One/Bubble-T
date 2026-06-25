//! FinalTerm / iTerm2 shell-integration OSC 133 sequences.

use crate::seq::osc_bel;

/// Builds an OSC 133 shell-integration sequence.
pub fn final_term(parts: &[&str]) -> String {
    osc_bel(&format!("133;{}", parts.join(";")))
}

/// Prompt mark (`OSC 133 ; A`).
pub fn final_term_prompt(extra: &[&str]) -> String {
    if extra.is_empty() {
        final_term(&["A"])
    } else {
        let mut parts = vec!["A"];
        parts.extend(extra.iter().copied());
        final_term(&parts)
    }
}

/// Command-start mark (`OSC 133 ; B`).
pub fn final_term_cmd_start(extra: &[&str]) -> String {
    if extra.is_empty() {
        final_term(&["B"])
    } else {
        let mut parts = vec!["B"];
        parts.extend(extra.iter().copied());
        final_term(&parts)
    }
}

/// Command-executed mark (`OSC 133 ; C`).
pub fn final_term_cmd_executed(extra: &[&str]) -> String {
    if extra.is_empty() {
        final_term(&["C"])
    } else {
        let mut parts = vec!["C"];
        parts.extend(extra.iter().copied());
        final_term(&parts)
    }
}

/// Command-finished mark (`OSC 133 ; D`).
pub fn final_term_cmd_finished(extra: &[&str]) -> String {
    if extra.is_empty() {
        final_term(&["D"])
    } else {
        let mut parts = vec!["D"];
        parts.extend(extra.iter().copied());
        final_term(&parts)
    }
}
