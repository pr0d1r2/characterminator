//! V138: a function preset grows only on evidence, so its size is pinned
//! here. Adding a member fails this test until the count is raised, and
//! the commit that raises it cites the research row that met the bar.

use super::{CharSet, catalog};

/// Every function preset and how many code points it grants as text.
const SIZES: [(&str, usize); 8] = [
    ("caveman", 32),
    ("box", 224),
    ("math", 15),
    ("legal", 3),
    ("typography", 15),
    ("emoji", 2491),
    ("marks", 4),
    ("cr", 1),
];

fn size(name: &str) -> usize {
    let set = catalog()
        .ok()
        .and_then(|sets| sets.resolve(name, "text").ok())
        .unwrap_or_else(|| CharSet::new(name.to_owned(), Vec::new()));
    (0..=0x0010_FFFF_u32)
        .filter_map(char::from_u32)
        .filter(|point| set.contains(*point))
        .count()
}

#[test]
fn function_presets_keep_their_pinned_size() {
    for (name, pinned) in SIZES {
        assert_eq!(size(name), pinned, "{name}: growth needs V138 evidence");
    }
}
