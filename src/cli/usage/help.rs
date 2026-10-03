//! The help text (V120): the global usage and one page per verb.
//!
//! Each page is one constant, so a reader of this file sees exactly what
//! a reader of the terminal does, and a test can hold every line to 80
//! columns and every flag of the table to a mention.

/// `ctrm --help`: every verb and flag with a meaning, the exit codes,
/// and where the documentation lives.
pub(in crate::cli) const USAGE: &str =
    "ctrm -- find and eliminate characters outside an allowed set

usage: ctrm <verb> [flags] [<path>...]    one verb's page: ctrm <verb> --help

verbs:
  check [<path>...]     report characters outside the set (exit 1 if any)
  fix [<path>...]       rewrite them through the map
  stats [<path>...]     what they cost in tokens now, and after a fix
  explain [<path>]      the set in force for a path, and the rule behind it
  sets [<name>...]      the sets a run can name, and what each holds
  init                  draft a .ctrm from the tracked files
  guard                 agent hook: hook JSON on stdin, decision on stdout

configuration (every verb but guard; repeatable; later wins):
  --rule <line>         one .ctrm line: <glob> <set>[+<set>] [@fam] [!lvl]
  --rules-file <f>      a file of .ctrm lines
  --set <line>          one .ctrm-sets line: <name> <member>...
  --sets-file <f>       a file of .ctrm-sets lines
  --map <line>          one .ctrm-map line: <from> [<to>]
  --map-file <f>        a file of .ctrm-map lines
  --no-files            do not read .ctrm, .ctrm-sets, .ctrm-map
  --no-builtin-map      start from an empty map
  --no-builtin-sets     start with no presets (ascii stays)
  --fidelity <family>   the same as --rule '* @<family>'
  --strict              a warning fails the run
  --pedantic            the same as --rule '* !pedantic=warn'

output and place:
  --format <f>          human (default) or json; check also takes sarif
  -C <dir>              the root (default: the top of the git work tree)
  --no-color            accepted, changes nothing: ctrm prints no colour
  --help, -h            this page, or one verb's after the verb
  --version, -V         the version
  --                    every word after it is a path

verb flags (see the verb's page):
  check --summary --max <n>    fix --check    stats --bpe
  explain --as <form>    sets --locales --containing <c>    init --print

exit codes: 0 clean; 1 a violation, or drift under fix --check;
2 a usage or configuration error. guard never exits 2.

docs: https://github.com/pr0d1r2/characterminator";

const CHECK: &str =
    "ctrm check [<path>...] -- report characters outside the set

With no path, every file git tracks; a directory means the tracked
files under it. One line per character: path:line:col, code point, set.

  --summary             one row per file and code point, with a count
  --max <n>             report at most n rows
  --format <f>          human, json or sarif (code scanning)
  configuration flags   see ctrm --help

exit codes: 0 clean, 1 a violation, 2 a usage or configuration error

example: ctrm check --format json docs/";

const FIX: &str =
    "ctrm fix [--check] [<path>...] -- rewrite characters outside the set

Rewrites each one through the map (.ctrm-map, then the builtin); a
character with no mapping is kept and reported, never dropped.

  --check               write nothing; report the drift, exit 1 on any
  --format <f>          human or json
  configuration flags   see ctrm --help

exit codes: 0 done, 1 drift (--check) or a character left unmapped,
2 a usage or configuration error

example: ctrm fix --check README.md";

const STATS: &str = "ctrm stats [--bpe] [<path>...] -- what the characters cost

Per file: characters outside the set, bytes, and tokens now against
after a fix. Reports only.

  --bpe                 count with the real tokenizer, not the estimate
  --format <f>          human or json
  configuration flags   see ctrm --help

exit codes: 0 reported, 2 a usage or configuration error

example: ctrm stats --bpe src/";

const EXPLAIN: &str = "ctrm explain [<path>] -- the set in force, and why

For a path: the set, the rule that granted it, its family and levels,
each with the line that set it. --as writes the whole configuration.

  --as args             as command-line flags
  --as lines            as .ctrm, .ctrm-sets and .ctrm-map lines
  --as prompt           as an instruction for an agent writing code
  --format <f>          human or json (not with --as)
  configuration flags   see ctrm --help

exit codes: 0 answered, 2 a usage or configuration error

example: ctrm explain docs/a.md";

const SETS: &str = "ctrm sets [<name>...] -- the sets a run can name

The presets, what each is for, and its members; with names, only those.

  --locales             the CLDR locale sets instead
  --containing <c>      only the sets holding c (a character or U+XXXX)
  --fidelity <family>   resolve members at that family
  --format <f>          human or json
  configuration flags   see ctrm --help

exit codes: 0 answered, 2 a usage or configuration error

example: ctrm sets --containing U+2014";

const INIT: &str = "ctrm init [--print] -- draft a .ctrm from the tracked files

Grants each kind of file the sets its characters already need.

  --print               write the draft to stdout, not to .ctrm
  configuration flags   see ctrm --help

exit codes: 0 drafted, 2 a usage or configuration error

example: ctrm init --print";

/// One verb's page, or `None` for a word that names no verb.
#[must_use]
pub(in crate::cli) fn verb_help(verb: &str) -> Option<&'static str> {
    let page = match verb {
        "check" => CHECK,
        "fix" => FIX,
        "stats" => STATS,
        "explain" => EXPLAIN,
        "sets" => SETS,
        "init" => INIT,
        "guard" => super::GUARD,
        _ => return None,
    };
    Some(page)
}

#[cfg(test)]
mod tests {
    use super::{USAGE, verb_help};

    /// V120: every page fits a terminal, and every verb page has an
    /// example and the exit codes.
    #[test]
    fn every_page_is_scannable() {
        let pages = super::super::VERBS.iter().filter_map(|v| verb_help(v));
        for page in pages.chain([USAGE]) {
            let wide = page.lines().find(|line| line.chars().count() > 80);
            assert_eq!(wide, None, "{page}");
            assert!(page.contains("exit codes"), "{page}");
        }
    }

    #[test]
    fn a_verb_page_has_an_example_and_the_global_page_the_docs() {
        for verb in super::super::VERBS {
            let page = verb_help(verb).unwrap_or_default();
            assert!(page.contains("example: ctrm"), "{verb}");
        }
        assert!(USAGE.contains("https://github.com/pr0d1r2/characterminator"));
    }
}
