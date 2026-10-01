# characterminator

<!-- BEGIN badges -->
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![edition 2024](https://img.shields.io/badge/edition-2024-000000?logo=rust&logoColor=white)](Cargo.toml)
[![MSRV 1.95](https://img.shields.io/badge/MSRV-1.95-000000?logo=rust&logoColor=white)](Cargo.toml)
[![direct dependencies 1](https://img.shields.io/badge/direct_dependencies-1-brightgreen)](docs/THIRD-PARTY-NOTICES.md)
[![runtime closure 22](https://img.shields.io/badge/runtime_closure-22-brightgreen)](docs/THIRD-PARTY-NOTICES.md)
[![coverage 93.9%](https://img.shields.io/badge/coverage-93.9%25-brightgreen)](.coverage)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-brightgreen)](Cargo.toml)
[![gate hk](https://img.shields.io/badge/gate-hk-6E4AFF)](hk.pkl)
[![nix flake](https://img.shields.io/badge/nix-flake-5277C3?logo=nixos&logoColor=white)](flake.nix)

[![built with Claude Code](https://img.shields.io/badge/built_with-Claude_Code-D97757)](https://claude.com/claude-code)
[![built with Opus 5](https://img.shields.io/badge/built_with-Opus_5-D97757)](https://www.anthropic.com/claude)
[![built with SDD](https://img.shields.io/badge/built_with-spec--driven_development-D97757)](SPEC.md)
<!-- END badges -->

Read [LLM-DISCLAIMER](docs/LLM-DISCLAIMER.md) first.

**Status: prototype.** Every verb below runs and is tested, the tool gates
its own tree, and nothing is published yet. See
[what is not done](#what-is-not-done).

Find and eliminate characters outside an allowed set, per file type and per
file. The default is pure ASCII; anything wider is a grant somebody wrote
down.

```text
$ ctrm check notes.md
notes.md:1:6 U+2014 ascii
notes.md:1:12 U+201C ascii
notes.md:1:19 U+201D ascii

$ ctrm fix notes.md
notes.md:1:6 U+2014 -> "--"
notes.md:1:12 U+201C -> "\""
notes.md:1:19 U+201D -> "\""
```

- `ascii` is the **set the file was judged against**, printed on every row,
  so the answer says what the rule was and not only that one was broken.
- `U+2014` rather than the character itself: a report about invisible and
  confusable characters cannot be written in them.

## Why

Three reasons, and the third is the one that is easy to get wrong.

**A stray character is a bug you cannot see.** A no-break space is a space
until a parser disagrees. A Cyrillic `a` in an identifier compiles somewhere
and not elsewhere. Zero width characters are not there at all until they are
in a diff nobody can read.

**A repository has a house style it cannot enforce.** Editors substitute
curly quotes and em dashes into files that never asked for them. Deciding
your prose is ASCII is easy; keeping it that way across contributors is not.

**Some characters cost more to feed a model than others** -- and this is
where a tool like this one is most tempted to overclaim. See
[what it costs](#what-it-costs), which reports what was measured rather
than what would sell the tool.

## Install

Not published yet. From a clone:

```bash
cargo install --path .        # or
nix develop                   # the dev shell, with the gate's tooling
```

## The verbs

| | |
|---|---|
| `ctrm check [<path>...]` | report characters outside the set. Exit 1 on a violation |
| `ctrm fix [--check] [<path>...]` | rewrite them. `--check` reports and writes nothing |
| `ctrm stats [--bpe] [<path>...]` | what the files cost now, and after a fix |
| `ctrm explain [<path>]` | the set in force, and the config line that decided it |
| `ctrm sets [--fidelity <f>]` | every declared set and what it holds |

Every verb takes `--format json`, which is a stable contract: its keys and
their meanings do not change under a caller, and the documents are asserted
whole in tests. `check` also takes `--format sarif`, a SARIF 2.1.0 log that
GitHub code scanning can upload, with the same exit code.

Naming no path checks what git tracks. Naming a directory expands to the
tracked files under it. Naming a file reaches it whether git tracks it or
not -- a file you point at is a file you meant.

Exit codes are `0` clean, `1` violation or drift, `2` usage.

## Configuring it

Three optional dotfiles at the root of the repository, each one line per
entry, `#` for a comment.

**`.ctrm`** -- which set applies where. Last matching line wins, the way
`.gitignore` does, so a per-file line placed after a per-type line overrides
it.

```text
* ascii
*.md ascii+typography
docs/**/*.md ascii+typography+emoji @emoji
vendor/** any
```

A path no line matches gets `ascii`. That is the strict default, and it is
deliberate: an unguarded character is the cost this tool exists to find.

**`.ctrm-sets`** -- sets of your own, for characters no preset covers.
Members are written as themselves, as `U+XXXX`, as a range, or as the name
of another set.

```text
house U+2261 U+226A
spec caveman typography house
```

**`.ctrm-map`** -- what a disallowed character is rewritten to. An absent
replacement is an explicit delete, which is the only way anything is ever
removed.

```text
U+2014 --
U+200B
```

Each entry remembers where it came from, and `explain` prints it:

```text
$ ctrm explain docs/a.md
path docs/a.md
set ascii+typography
pattern *.md
origin .ctrm:2
```

## The sets

`ctrm sets` lists them with their members. The shape is small on purpose:
measured across 227 repositories, a typical non-ASCII file needs `ascii`
plus **one** of these.

| set | what it is for |
|---|---|
| `caveman` | the logic and arrow symbols compressed notation uses |
| `typography` | real dashes, curly quotes, ellipsis, no-break space |
| `math` | the mathematics that turns up in prose |
| `legal` | copyright, registered, trade mark |
| `marks` | check, cross, warning, arrow -- in both spellings |
| `box` | box drawing and geometric shapes |
| `emoji` | emoji as single code points |
| `cr` | carriage return, alone |
| `latin1`, `latin-ext`, `cyrillic`, `greek`, `arabic` | coarse blocks |
| `any` | everything, for a file you do not own |

`marks` is the one to look at twice. One name holds both spellings of the
same mark, and which one you get depends on the fidelity in force:

```text
$ ctrm sets | grep marks
marks U+2192 U+26A0 U+2713 U+2717

$ ctrm sets --fidelity emoji | grep marks
marks U+26A0 U+2705 U+274C U+27A1
```

So a rule can say "this directory speaks emoji" once, rather than listing
variants everywhere. The coarse blocks are deliberately last: a block grants
hundreds of characters, which answers "this file is in that script" rather
than "this file needs these characters", and the second question is the one
a budget is asking.

## What it costs

`stats` prints two figures, and the second is a real rewrite rather than an
estimate of one: the map runs against the file's own set and the result is
counted.

```text
$ ctrm stats --bpe notes.md
notes.md outside 3 bytes 26 tokens 6 (o200k) -> 6 (o200k)
```

The figure says how it was measured. `~1458` is the cheap proxy and wears a
tilde; `1951 (o200k)` named the tokenizer that produced it. Neither can be
mistaken for the other at a glance, which is the whole point.

**Rewriting typography saves bytes and often saves no tokens at all.**
Measured here on a document with 60 substituted characters: 1044 bytes ->
972 bytes, and 216 tokens -> 216 tokens. Modern tokenizers have seen em
dashes and curly quotes, and spend about what they spend on ASCII. The
characters that genuinely cost more are the rarer ones -- box drawing,
emoji, symbols -- and today the builtin map does not rewrite those, so
`stats` reports no saving for them either, honestly, rather than an
estimate of one.

Which leaves the claim this tool can actually support today: it makes the
cost **visible**, and it stops the characters **arriving**. A saving figure
that flattered the tool would be the first number to re-measure, so the two
above are printed as found.

## It gates itself

`ctrm check` runs over this repository's own tracked files on every commit,
against the [`.ctrm`](.ctrm) and [`.ctrm-sets`](.ctrm-sets) beside this
file. Every grant the project relies on is one a user could write.

That is not decoration. The first run found a section sign in a Rust doc
comment in a repository whose sources are supposed to be ASCII, and it found
it before any reviewer did.

The specs get a wider set than the prose does, because a document that
*defines* character sets has to print the characters it defines. Granting
them by name beat the two alternatives: `any` would switch the check off for
the files that most need it, and rewriting the evidence into code point
notation would make it unreadable.

## What is not done

`SPEC.md` is the authority and `mth tasks SPEC.md` prints the backlog; this
list goes stale and that one does not.

- **`guard`**, the agent-harness hook adapter, is specified and not built.
- **The `hazard` set** -- zero width characters, bidi controls, tag
  characters -- is specified and not built. Until it is, this tool finds
  confusables only by them being outside a set, not by them being dangerous.
- **Locale letter presets** from CLDR, and **emoji sequence compression**,
  both need data vendored first.
- **No CI**, no release, nothing published. The gate is git hooks today.

## License

MIT -- see [LICENSE](LICENSE). Third-party notices, including the measured
dependency closure, are in
[docs/THIRD-PARTY-NOTICES.md](docs/THIRD-PARTY-NOTICES.md).
