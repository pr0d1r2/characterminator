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
  granted, as `path:line:col U+XXXX <set>`. Exit 1 on a violation.
- **`ctrm fix`**: rewrite what the transliteration map can rewrite (dashes,
  curly quotes, ellipsis, no-break space and more), delete hazard
  characters, and report what is left. `fix --check` writes nothing and
  gates like `check`.
- **`ctrm stats`**: what each file costs in tokens now and after a fix,
  labelled with the method that counted it (`~` estimate or `(o200k)`).
- **`ctrm explain`**: the set in force for a path, the rule that decided it
  and where that rule was written; `--as args|lines|prompt` exports the
  configuration as flags, data-file lines or an agent instruction.
- **`ctrm sets`**: every declared set and its members.
- **`ctrm guard`**: a Claude Code hook. It denies a `Read` of a file holding
  invisible or direction-changing characters, and blocks tool output that
  carries them, so smuggled instructions do not reach the model unseen.
- **Configuration** in three dotfiles (`.ctrm`, `.ctrm-sets`, `.ctrm-map`),
  each line with a flag twin (`--rule`, `--set`, `--map`), last match wins.
- **Character sets**: function presets (`caveman`, `typography`, `math`,
  `legal`, `marks`, `box`, `emoji`), coarse script blocks, `persian` and
  `hindi`, and letter presets for every CLDR locale (`pl`, `de`, `ja`, ...).
- **Hazards**: bidi controls, tag characters, stray byte order marks,
  control characters and default-ignorable characters fire at `forbid`,
  which no rule can lower, whatever the file's set grants. Joiners inside
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
- **Output formats**: human, a stable `--format json` contract, and SARIF
  for GitHub code scanning (`check --format sarif`).
- **Adoption**: pre-commit hooks (`ctrm-check`, `ctrm-fix`) and a composite
  GitHub Action.
- **A library**: `scan_str`, `scan_bytes`, `fix` and the set, map, hazard
  and lint types, with every public function doc-tested.

### Known limits

- `guard` maps Claude Code's hook format only, and judges at most the first
  16 MiB of a file before a `Read`.
- The `locale-literal` pedantic lint is not built yet.
- Linux and macOS are tested in CI; Windows is not.

See [`SPEC.md`](SPEC.md) for the rules every one of these behaviours is held
to, and `mth tasks SPEC.md` for what remains.
