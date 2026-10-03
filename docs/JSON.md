# The json contract

`--format json` is the stable interface of `ctrm` (`src/render:V11`). The
human output may change between releases; this document may not, except as
the version policy below allows. The rules it states are `src/render:V95`
and `src/render:V122`, and the whole-document tests in
`src/render/json_test.rs` hold the renderer to them.

## The document

One compact json object per run, on stdout, followed by a newline. Pure
ASCII: every non-ASCII character is written as a `\u` escape, so a report
never carries back the characters the tool exists to find.

Every document starts with the same two keys:

| key | type | meaning |
|---|---|---|
| `schema` | number | the contract version, currently `1` |
| `verb` | string | `check`, `fix`, `stats`, `sets` or `explain` |

Every key listed below is always present. An empty list is `[]`; a value
that does not exist is `null`, never a missing key.

`guard` is not covered here: it writes the hook protocol its harness
expects (`src/cli/guard:V35`), not a report. `check --format sarif` writes
a SARIF 2.1.0 log (`src/render:V50`), which has its own schema.

## Units

| field | unit |
|---|---|
| `line` | 1-based line number |
| `column` | 1-based, counted in Unicode **code points**, not bytes and not UTF-16 units |
| `byte` | 0-based byte offset into the file |
| `codepoint` | `U+XXXX`, at least four uppercase hex digits |
| `character` | the character itself, as a json string |

## Enums

| field | values |
|---|---|
| `level` | `allow`, `warn`, `deny`, `forbid` |
| `lint` | a lint name: `outside-set`, a hazard (`bidi-control`, ...) or a pedantic lint (`trailing-whitespace`, ...); `ctrm explain` and the README list them |
| `reason` (skipped) | `not-utf8` (with `byte`, where decoding failed) or `binary` |
| `method` (counts) | `estimate` or `bpe` |

## Sets

`set` names the set a character was judged against, as the rules wrote it:
one set (`ascii`), or a union joined by `+` (`ascii+any`,
`ascii+spec`). A hazard is judged against no set, so its `set` is
`hazard`; the `lint` names which hazard. A pedantic finding fires inside
the set, so its `set` is the set in force and the `lint` says what is wrong.

## Per verb

### `check`

`{schema, verb, violations, skipped}`

A **violation** is:

| key | type | |
|---|---|---|
| `path` | string | as the run names it, relative to the root |
| `line` `column` `byte` | number | see Units |
| `codepoint` `character` | string | |
| `set` `lint` `level` | string | see Sets and Enums |
| `replacement` | string or null | what `fix` writes in place of the span that STARTS at this character; `null` when no rewrite starts here |
| `fixable` | bool | `true` when a `fix` run clears this character, so the next `check` would not report it |

`replacement` and `fixable` are answered by running the fix, not by reading
the map: a file `fix` refuses to rewrite (`src/fix:V5`, `src/fix:V6`) has
`null` and `false` throughout. A character inside a multi-character
rewrite (an emoji sequence) can be `fixable` with a `null` replacement: the
replacement is reported on the span's first character.

A **skipped** entry is `{path, reason}` plus `byte` for `not-utf8`.

### `fix` and `fix --check`

`{schema, verb, rewrites, unmapped, skipped}`

- a **rewrite** is a violation plus `to`, the text written; its
  `replacement` equals `to` and `fixable` is `true`. Positions are in the
  file as it was before the rewrite.
- **unmapped** is what the rewrite leaves, as `check` would report it: a
  violation with `replacement: null` and `fixable: false`. After a bare
  `fix` its position is in the file AS WRITTEN, so it matches the next
  `check`; under `--check` nothing is written, and it is in the file on
  disk.

### `stats`

`{schema, verb, files, total, skipped}`

- a **file** is `{path, outside, bytes, tokens_now, tokens_after}`;
- `total` sums every counted file, with `files` (how many), or is `null`
  when nothing was counted;
- a **count** is `{tokens, method}`.

`stats` is report-only (`src/cli:V7`): it exits 0 even when a file is
skipped as `not-utf8`. The file is named in `skipped` and left out of the
counts; failing a run that only measures would make it unusable as a
dashboard over a tree with one broken file. Gate with `check`, which exits 1
on the same file.

### `sets`

`{schema, verb, sets}`; a set is `{name, ranges}`, a range `{start, end}`
in `U+XXXX`.

### `explain`

`{schema, verb, path, set, rule, effective}`; `rule` is
`{pattern, sets, family, levels, default_level, origin}` or `null` when no
rule matched; `effective` is `{family, default_level, levels}`, each with
the `origin` of the line that set it.

## Errors

A run that fails before it can report -- a usage error, a configuration
that does not parse, a file that cannot be written -- exits 2 and names the
problem on stderr. Under `--format json` it ALSO writes, on stdout:

    {"schema":1,"verb":"check","error":"unknown flag `--stirct`"}

so a script that asked for json parses the failure instead of an empty
stream.

## Exit codes

| code | meaning |
|---|---|
| 0 | clean, or a report-only verb (`stats`, `sets`, `explain`, bare `fix` that left nothing) |
| 1 | `check` found a finding at `deny` or `forbid` (or `warn` under `--strict`), or a file was not UTF-8; `fix --check` found drift or would leave such a finding; bare `fix` left one |
| 2 | usage or configuration error; see Errors |

## Version policy

`schema` rises only when a key is **renamed, retyped or removed**, or its
meaning changes. A key **added** leaves it alone, so a consumer must ignore
keys it does not know. Within one `schema`, every key listed here keeps its
name, type and meaning.
