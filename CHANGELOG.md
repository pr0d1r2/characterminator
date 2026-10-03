# Changelog

All notable changes to `characterminator` (the `ctrm` binary) are recorded
here. The format follows [Keep a Changelog](https://keepachangelog.com), and
the project uses [Semantic Versioning](https://semver.org).

## Version ladder

A minor version is a level of **guarantee**, not a feature count. An
**odd** minor is functional and tested, but not yet for production; an
**even** minor is stable. This is the convention `microlith` and `sherd`
use, so one reading serves the whole family of tools.

| version | what you can rely on |
|---|---|
| 0.1 | every verb works, is tested and gates its own repository; the `--format json` contract and the library surface are new and may still move before 0.2 |

## [0.1.0] - unreleased

The first release.

### Added

- **`ctrm check`**: report every character outside the set its file is
  granted, as `path:line:col U+XXXX <set>`, then a one-line tally on
  stderr naming the most frequent code points. Exit 1 on a violation.
  `--summary` folds the rows to one per file and code point with a count;
  `--max <n>` stops after n rows, so an agent's context stays bounded.
- **`ctrm fix`**: rewrite what the transliteration map can rewrite (dashes,
  curly quotes, ellipsis, no-break space and more), delete hazard
  characters, and report what is left. `fix --check` writes nothing and
  gates like `check`.
- **`ctrm stats`**: what each file costs in tokens now and after a fix,
  labelled with the method that counted it (`~` estimate or `(o200k)`).
- **`ctrm explain`**: the set in force for a path, the rule that decided it
  and where that rule was written; `--as args|lines|prompt` exports the
  configuration as flags, data-file lines or an agent instruction.
- **`ctrm sets`**: the curated presets first, each with what it is for,
  then the declared sets and their members. `--containing <c>` (a
  character or `U+XXXX`) lists only the sets that hold it; `--locales`
  lists the CLDR locale sets.
- **`ctrm init`**: survey the tracked files by type and draft a `.ctrm`
  that grants each type the fewest presets covering what it already
  holds, every line commented. It never overwrites an existing `.ctrm`;
  `--print` writes the draft to stdout.
- **`ctrm guard`**: a Claude Code hook in two tiers. Tag characters and
  bidi embeddings, overrides and isolates -- the carriers of smuggled
  instructions and Trojan Source -- deny a `Read` and block tool output.
  Every other hazard is reported to the model as a note and the task goes
  on. Every denial ends in a next step (`ctrm fix <path>`, then read
  again). Input that is not valid UTF-8 or not valid JSON is still
  scanned, and a broken hook exits 1, never 2.
- **Configuration discovery**: run from a subdirectory, every verb finds
  the repository root (the nearest `.git`) and reads its dotfiles, and
  paths are matched and shown relative to that root. `-C <dir>` names the
  root instead.
- **Refusals instead of silence**: an unknown flag, a flag that belongs to
  another verb, a set redefining `ascii`, `any` or a hazard class, a
  hazard lowered below `forbid`, or a bare run with no tracked files is
  refused with exit 2 and the line that caused it. A mistyped verb gets a
  suggestion (`did you mean 'check'?`), and flags may come before the
  verb.
- **Help**: `ctrm --help` gives every verb and flag a one-line meaning and
  the exit codes; `ctrm <verb> --help` gives that verb's own page with an
  example. Messages name flags as typed and carry no internal spec ids.
- **Configuration** in three dotfiles (`.ctrm`, `.ctrm-sets`, `.ctrm-map`),
  each line with a flag twin (`--rule`, `--set`, `--map`), last match wins.
- **Character sets**: function presets (`caveman`, `typography`, `math`,
  `legal`, `marks`, `box`, `emoji`), coarse script blocks, `persian` and
  `hindi`, and letter presets for every CLDR locale (`pl`, `de`, `ja`, ...).
- **Hazards**: bidi controls, tag characters, stray byte order marks,
  control characters and default-ignorable characters fire at `forbid`,
  which no rule can lower, whatever the file's set grants, and `fix`
  removes them even from a file granted `any`. Joiners inside
  official emoji sequences and the joiners Persian and Hindi spell with are
  exempt, exactly and only there.
- **Lint levels** in the rustc/clippy shape (`allow`, `warn`, `deny`,
  `forbid`) with `--strict` and an opt-in `--pedantic` group: `crlf`,
  `trailing-whitespace`, `final-newline`, `unicode-space`, `not-nfc`,
  `nfkc-compat`, `mixed-script`, `confusable`.
- **Emoji compression**: presentation selectors and skin tones are deleted,
  keycaps become digits, flags become region codes (`PL`), and family,
  couple and kiss sequences become their single emoji.
- **An opt-in `words` map** (`use words`) rewriting logic notation to plain
  words, measured to save tokens under the o200k tokenizer.
- **Output formats**: human, `--format json` and SARIF for GitHub code
  scanning (`check --format sarif`). Every json document opens with
  `"schema":1` and its verb, failures included; a violation carries the
  text `fix` would write (`replacement`) and whether it clears it
  (`fixable`). [docs/JSON.md](docs/JSON.md) lists every key, unit and
  enum, and when the schema number changes. A file that is not valid
  UTF-8 is named with its line and column, in every format.
- **Adoption**: pre-commit hooks (`ctrm-check`, `ctrm-fix`) and a composite
  GitHub Action.
- **A library**: `scan_str`, `scan_bytes`, `fix`, `Hazards::lints_in`
  (every hazard in a text, with the same emoji-sequence and byte-order-mark
  exemptions `check` applies) and the set, map, hazard and lint types,
  with every public function doc-tested.

### Known limits

- `guard` maps Claude Code's hook format only, and judges at most the first
  16 MiB of a file before a `Read`.
- The `locale-literal` pedantic lint is not built yet.
- `ctrm init` drafts one line per file type, so a type whose files mix
  scripts (most `*.md` in a multilingual tree) is drafted as `any`, with a
  comment saying so. Splitting a type by directory is planned.
- Linux and macOS are tested in CI; Windows is not.

See [`SPEC.md`](SPEC.md) for the rules every one of these behaviours is held
to, and `mth tasks SPEC.md` for what remains.
