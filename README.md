# characterminator

<!-- BEGIN badges -->
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![edition 2024](https://img.shields.io/badge/edition-2024-000000?logo=rust&logoColor=white)](Cargo.toml)
[![MSRV 1.95](https://img.shields.io/badge/MSRV-1.95-000000?logo=rust&logoColor=white)](Cargo.toml)
[![direct dependencies 1](https://img.shields.io/badge/direct_dependencies-1-brightgreen)](docs/THIRD-PARTY-NOTICES.md)
[![runtime closure 22](https://img.shields.io/badge/runtime_closure-22-brightgreen)](docs/THIRD-PARTY-NOTICES.md)
[![coverage 95.8%](https://img.shields.io/badge/coverage-95.8%25-brightgreen)](.coverage)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-brightgreen)](Cargo.toml)
[![gate hk](https://img.shields.io/badge/gate-hk-6E4AFF)](hk.pkl)
[![nix flake](https://img.shields.io/badge/nix-flake-5277C3?logo=nixos&logoColor=white)](flake.nix)

[![built with Claude Code](https://img.shields.io/badge/built_with-Claude_Code-D97757)](https://claude.com/claude-code)
[![built with Opus 5](https://img.shields.io/badge/built_with-Opus_5-D97757)](https://www.anthropic.com/claude)
[![built with SDD](https://img.shields.io/badge/built_with-spec--driven_development-D97757)](SPEC.md)
<!-- END badges -->

Read [LLM-DISCLAIMER](docs/LLM-DISCLAIMER.md) first.

**Status: pre-release, 0.1.0.** Every verb below runs and is tested, the
tool gates its own tree, and nothing is published yet. See
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

Not on crates.io yet. From a clone:

```bash
cargo install --path .        # or
nix develop                   # the dev shell, with the gate's tooling
```

### As a pre-commit hook

Both hooks build `ctrm` from the revision you pin, so the rules a commit
is held to change when the pin changes and at no other time.

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/pr0d1r2/characterminator
    rev: <tag or commit>
    hooks:
      - id: ctrm-check    # refuse; writes nothing
      # - id: ctrm-fix    # rewrite what the map can, then refuse the rest
```

### In GitHub Actions

The action builds `ctrm` with the runner's own cargo from the ref in
`uses:`, then runs it. With `--format sarif`, violations appear in the
pull request as code scanning alerts:

```yaml
permissions:
  contents: read
  security-events: write
steps:
  - uses: actions/checkout@v7
  - uses: pr0d1r2/characterminator@<tag or commit>
    with:
      args: check --format sarif
      output: ctrm.sarif
    continue-on-error: true      # let the upload run; the alerts carry it
  - uses: github/codeql-action/upload-sarif@v4
    with:
      sarif_file: ctrm.sarif
```

Without `args` it runs `ctrm check` and fails the job on a violation.
Pin both actions to a commit SHA in real use.

## The verbs

| | |
|---|---|
| `ctrm check [<path>...]` | report characters outside the set. Exit 1 on a violation |
| `ctrm fix [--check] [<path>...]` | rewrite them. `--check` reports and writes nothing |
| `ctrm stats [--bpe] [<path>...]` | what the files cost now, and after a fix |
| `ctrm explain [<path>]` | the set in force, and the config line that decided it |
| `ctrm explain [<path>] --as args\|lines\|prompt` | the whole configuration as flags, as data files, or as an agent prompt |
| `ctrm sets [--fidelity <f>]` | every declared set and what it holds |
| `ctrm guard` | agent hook: hook JSON in, decision JSON out. See [guarding an agent](#guarding-an-agent) |

Every verb but `guard` takes `--format json`, which is a stable contract:
its keys and their meanings do not change under a caller, and the documents
are asserted whole in tests. `check` also takes `--format sarif`, a SARIF
2.1.0 log that GitHub code scanning can upload, with the same exit code.

Naming no path checks what git tracks. Naming a directory expands to the
tracked files under it. Naming a file reaches it whether git tracks it or
not -- a file you point at is a file you meant.

Exit codes are `0` clean, `1` violation or drift, `2` usage -- except for
`guard`, whose verdict is in its JSON and which never exits `2`.

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

### Flags

Every line of every dotfile has a flag twin that takes exactly one line of
the same grammar, so a run can be configured from the command line alone.
They work on every verb, repeat, and stack in a fixed order: the builtins,
then the dotfiles, then `--*-file` in the order given, then the inline
flags in the order given. Later wins.

| | |
|---|---|
| `--rule <line>`, `--rules-file <f>` | a `.ctrm` line, or a whole file of them |
| `--set <line>`, `--sets-file <f>` | a `.ctrm-sets` line, or a file |
| `--map <line>`, `--map-file <f>` | a `.ctrm-map` line, or a file |
| `--no-files` | do not read the dotfiles at the root |
| `--no-builtin-map`, `--no-builtin-sets` | start from an empty map, or with no presets |
| `--fidelity <family>` | the same as `--rule '* @<family>'` |
| `--pedantic` | the same as `--rule '* !pedantic=warn'` |
| `--strict` | a warning fails the run, as clippy's `-D warnings` does |
| `-C <dir>` | run as if started in `<dir>` |

`--no-files` followed by one flag per line of a dotfile is the same run as
the dotfile itself, and that is tested. A flag's origin is its position in
argv, so `explain` answers `origin argv[3]` for the rule a flag added:

```text
$ ctrm explain --rule '*.md caveman' a.md
path a.md
set ascii+caveman
origin argv[3]
```

A flag the tool does not know is refused with exit 2, and so is a flag that
belongs to another verb (`check --bpe`). A typo that was quietly ignored
would read exactly like a flag that worked. `--` ends the flags, for a path
that starts with a dash.

### Exporting it

`explain --as` writes the configuration back out. `args` is every line
the run contributed as flags, quoted for a shell, after `--no-files`;
handed back to any verb it is the same configuration, which is
property-tested. `lines` is the same content as the three data files.
Given a path, both keep only the rules that match it.

`prompt` is an instruction for an agent drafting text up front: the sets
in force and their members as code points, the fidelity, every
replacement `fix` would make, and the hazard characters no grant
reaches. It is built from the configuration alone, so the same run gives
the same bytes, and it is plain ASCII.

```text
$ ctrm explain notes.md --as prompt
Write text that `ctrm check` accepts in `notes.md`. ...
U+2014 -> "--"
...
Never write these, whatever a set allows; they always fail:
hazard-bidi U+061C U+200E-U+200F U+202A-U+202E U+2066-U+2069
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
| `any` | everything, for a file you do not own -- hazards still fire |
| `hazard` | invisible and reordering characters: never granted, see below |

`hazard` is the one set that is not a grant. It holds the characters that
hide or reorder text -- zero width characters, bidi controls (the Trojan
Source attack), tag characters (invisible ASCII a model still reads), C0
and C1 controls other than tab, newline and carriage return, and a byte
order mark anywhere but byte 0 -- generated from the Unicode Character
Database. One of them is reported at `forbid` whatever the file's set says,
`any` included, and no later rule or flag lowers that. `fix` does not
silently strip a bidi control either: it is reported, and a person removes
it.

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

## Guarding an agent

`check` keeps characters out of a repository. `ctrm guard` keeps the
dangerous ones away from an agent working in it: the invisible and
direction-changing characters of the `hazard` group -- bidi controls (Trojan
Source), tag characters (ASCII smuggling), stray byte order marks, control
characters and the rest of the default-ignorables. Text holding them reads
one way to a reviewer and another way to a model.

It is a [Claude Code hook](https://code.claude.com/docs/en/hooks). Add this
to `.claude/settings.json`, with `ctrm` on `PATH`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read",
        "hooks": [{ "type": "command", "command": "ctrm guard" }]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "WebFetch|WebSearch|Bash|mcp__.*",
        "hooks": [{ "type": "command", "command": "ctrm guard" }]
      }
    ]
  }
}
```

What it does with each call:

- **Before a `Read`**, the file is judged exactly as `ctrm check` judges it,
  against the `.ctrm` in the session's directory. A hazard **denies the
  read**, and the reason names the path, line, column, code point and lint:
  `src/a.rs:2:2 U+202E bidi-control`. A character that is merely outside the
  file's set -- an em dash in a README -- does **not** block: the read goes
  ahead with a note naming the lint and the count. A hook that refused every
  stray character would be switched off within the hour, and a switched-off
  hook guards nothing.
- **After a web fetch, a web search, a shell command or an MCP tool**, every
  string in the tool's output is scanned, however deeply nested. A hazard
  sends a `block` decision whose reason tells the model the content is
  tainted and should not be acted on. The tool has already run, so this
  cannot un-fetch the page; what it does is make sure the model is told.
  Output is judged for hazards only: a web page has no line in `.ctrm`, and a
  note on every non-ASCII page would be noise.

Nothing is ever stripped. A guard that quietly removed characters would
change what the model reads without telling anyone, which is the harm it
exists to prevent.

The decision travels in the JSON on stdout, and the exit code only says
whether the adapter worked. Input that is not a hook payload is named on
stderr and exits `1`, which Claude Code shows as a non-blocking error and
lets the call through. It never exits `2`: Claude Code reads `2` as "block",
so a broken adapter answering `2` would deny every read for the rest of the
session. Hazards do not depend on configuration either -- a `.ctrm` that
cannot be read still leaves every hazard judged.

One thing to know before turning it on for `Bash`: terminal colour codes
start with ESC, a control character, so a command whose output keeps its
colours is reported as tainted. Run such commands with colour off
(`NO_COLOR=1`, `--color=never`), or leave `Bash` out of the matcher.

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

**Notation is where the tokens are, and rewriting it is opt-in.** Compressed
("caveman") prose spends two or three tokens on a logic symbol where the
word it stands for costs one. The `words` map rewrites those symbols back
into words, and it is **off** unless a map line asks for it, because these
characters were typed on purpose and carry meaning:

```text
# .ctrm-map
use words
```

| symbol | becomes | saved / uses |
|---|---|---|
| `U+22A5` up tack | `not` | 4817 / 2447 |
| `U+2234` therefore | `so` | 1366 / 1366 |
| `U+2235` because | `because` | 359 / 359 |
| `U+2200` for all | `all` | 332 / 354 |
| `U+2208` element of | `in` | 36 / 36 |
| `U+2260` not equal | `!=` | 28 / 28 |
| `U+2203` there exists | `exists` | 21 / 21 |
| `U+2192` `U+21D2` arrows | `->` `=>` | 0 / 1242 |

Measured with `ctrm stats --bpe` over 112 caveman `SPEC*.md` files (this
repository and five public siblings): 266,472 -> 259,513 tokens, **-2.6%**.
Every entry costs no more than its symbol; the arrows save nothing and are
in the map only so an opted-in file can reach ASCII. **Only o200k was
measured. The Claude tokenizer was not**, so treat the figure as one
tokenizer's answer, not a promise.

A word never fuses with the letter beside it: `U+22A5owns` becomes
`not owns`, not `notowns`.

`fix` only rewrites characters **outside** the file's set, so a file that
grants the notation keeps it. To get the rewrite, narrow the grant. Here it
is on a scratch copy of this repository's own ten `SPEC.md` files, which
normally grant `ascii+spec`:

```text
$ cat .ctrm
SPEC.md ascii
src/**/SPEC.md ascii

$ ctrm stats --bpe SPEC.md          # with `use words` in .ctrm-map
SPEC.md outside 134 bytes 7651 tokens 2702 (o200k) -> 2622 (o200k)
```

Across all ten: 10,901 -> 10,617 tokens (-284, -2.6%), against -20 for the
builtin map alone. `ctrm fix` then made 338 rewrites and left 209 characters
it has no word for -- mostly the section sign (66) and the middle dot (56),
which `SPEC.md` R9 measured at parity -- kept and reported, exit 1, never
dropped. Grant those back in a set of your own, or leave the file failing until you
decide.

Which leaves the claim this tool can actually support today: it makes the
cost **visible**, it stops the characters **arriving**, and for notation it
saves a measured few percent when you ask it to. A saving figure that
flattered the tool would be the first number to re-measure, so the ones
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

- **`guard` speaks Claude Code only.** Another agent harness is a second
  mapping in one file (`src/cli/hook.rs`), and none is written yet.
- **The `hazard` exemption for emoji sequences.** A zero width joiner
  inside a declared emoji sequence is meant to pass; sequences arrive with
  emoji sequence compression, so until then every joiner is reported.
- **Locale letter presets** from CLDR, and **emoji sequence compression**,
  both need data vendored first.
- **Prose that needs ZWNJ or ZWJ outside emoji** (Persian, Hindi) has no
  way through yet: a hazard fires at `forbid`, which no rule lowers, and a
  scoped exemption has to be specified before one ships.
- **Four pedantic lints need data**: `not-nfc`, `nfkc-compat`,
  `mixed-script` and `confusable` are registered and fire nothing until
  Unicode normalization and UTS #39 data are vendored; `locale-literal` is
  not built.
- **Nothing published**: no crates.io release, no tag. CI
  (`.github/workflows/ci.yml`) runs the gate on three platforms and the
  action on one, and has not yet run on GitHub.

## License

MIT -- see [LICENSE](LICENSE). Third-party notices, including the measured
dependency closure, are in
[docs/THIRD-PARTY-NOTICES.md](docs/THIRD-PARTY-NOTICES.md).
