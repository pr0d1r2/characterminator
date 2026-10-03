//! Guard's two tiers of hazard (V110): which ones stop a call, and which
//! ones only earn a note.
//!
//! Every hazard is still a hazard -- `check` and `fix` hold all of them
//! at `forbid` (`src/lint:V36`). This is guard's DECISION policy, and the
//! line is drawn at intent. A tag character, or a bidi embedding,
//! override or isolate, has no use in ordinary prose and is how
//! instructions are smuggled (ASCII smuggling, Trojan Source). A soft
//! hyphen on a German page, a right-to-left mark on an Arabic one, the
//! ESC of terminal colour and the form feed in a C source are ordinary
//! content: blocking them is how a hook gets switched off.

use crate::lint::Finding;

/// The one control class: the characters a terminal acts on rather than
/// hides. Named so a note can say so, rather than call ESC "invisible".
const CONTROL: &str = "control-character";

/// Whether a hazard's character blocks: a tag character (U+E0000 to
/// U+E007F), or a bidi embedding, override or isolate (U+202A to U+202E,
/// U+2066 to U+2069). Marks (U+200E, U+200F, U+061C) do not: they steer
/// no text on their own, and RTL pages are full of them.
pub(super) fn blocks(finding: &Finding) -> bool {
    matches!(
        finding.hit.character,
        '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{E0000}'..='\u{E007F}'
    )
}

/// What a set of noted hazards IS, in words a reader recognises: ESC and
/// form feed are control characters, not invisible text.
pub(super) fn class<'a>(
    noted: impl Iterator<Item = &'a Finding>,
) -> &'static str {
    let (mut controls, mut all) = (0_usize, 0_usize);
    for finding in noted {
        all = all.saturating_add(1);
        if finding.lint.name == CONTROL {
            controls = controls.saturating_add(1);
        }
    }
    match controls {
        0 => "invisible formatting character(s)",
        n if n == all => "terminal or control character(s)",
        _ => "invisible formatting or control character(s)",
    }
}
