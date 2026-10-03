# Questions and troubleshooting

Each answer was checked against the 0.1.0 binary. Exit codes: `0` clean,
`1` a finding (or drift, for `fix --check`), `2` a usage or configuration
error; `guard` never exits `2`.

## Why did `check` skip a file I can see?

A bare `ctrm check` checks the files **git tracks**. An untracked file is
not part of the repository yet, so it is not judged. Name it to check it
anyway: a file you point at is a file you meant.

```text
$ ctrm check               # untracked.md is not tracked: exit 0
$ ctrm check untracked.md
untracked.md:1:4 U+00E9 ascii
```

## "is not inside a git work tree"

```text
ctrm: /tmp/scratch: is not inside a git work tree, so there is no tracked fileset -- name the files to check
```

A bare run needs git to know which files are yours. Outside a repository,
name the files or directories: `ctrm check notes.md docs/`.

## Why is the set in a subdirectory the same as at the root?

Every verb finds the repository root (the nearest `.git` above the working
directory) and reads `.ctrm`, `.ctrm-sets` and `.ctrm-map` from there.
Paths are matched and shown relative to that root. `-C <dir>` names the
root instead.

## Which line decided a file's set?

```text
$ ctrm explain docs/notes.md
```

It prints the set in force, the `.ctrm` line that won (the last matching
one), and where it was written. A path no line matches gets `ascii`.

## "skipped, binary" and "invalid UTF-8"

A file with a NUL byte anywhere is binary: it is named and skipped, and
does not fail the run. A file that is not valid UTF-8 cannot be judged,
so it is named with where decoding stopped, and the run exits 1:

```text
bin.dat: skipped, binary
bad.txt: invalid UTF-8 at 1:3 (byte 2)
```

## Why did `fix` leave a character?

`fix` rewrites only what the map has an entry for: dashes, curly quotes,
the ellipsis, unusual spaces, emoji sequences and more. A letter such as
`U+00E9` has no ASCII spelling that keeps its meaning, so it is left and
listed:

```text
left: left.md:1:4 U+00E9 ascii
rewrote 0, 1 left (no map entry): grant a set in .ctrm or edit by hand
```

Grant it (`*.md ascii+en-aux`, or the locale set for the language), add
a line to `.ctrm-map`, or edit the text. `ctrm sets --containing U+00E9`
lists every set that holds it.

## Why can I not grant a hazard, or lower its level?

Bidi controls, tag characters, stray byte order marks, control and
default-ignorable characters hide or reorder text, so they are always
`forbid`, whatever the grant, `any` included. A line that tries to lower
one is refused rather than silently ignored:

```text
ctrm: --rule '* any !hazard=allow': hazard lints are forbid and cannot be lowered
```

Joiners inside official emoji sequences, a byte order mark at byte 0, and
the joiners Persian and Hindi spell with are exempt, exactly and only
there.

## Why did `init` grant `any` to all my Markdown?

`init` drafts one line per file type. When files of one type need sets
that no small cover reaches, such as Markdown in several scripts, the
draft says so in a comment and grants `any`. Hazards still fire under
`any`. Split the line by directory by hand (`docs/ja/*.md ja`); a draft
that does this is planned.

## Why did my token count not drop?

Modern tokenizers already know em dashes and curly quotes, so rewriting
typography saves bytes and often no tokens. `ctrm stats` shows the count
now and after a fix, labelled with the method that counted it, before you
change anything. See [What it costs](../README.md#what-it-costs).

## Can it read standard input?

Not in 0.1. `ctrm check -` is refused as an unknown flag. A file whose
name starts with a dash is named after `--`: `ctrm check -- -odd.md`.

## The guard hook printed an error and the tool call went ahead

`guard` treats a broken hook as broken, not as a verdict: input that is
not hook JSON is named on stderr and the hook exits `1`, which Claude Code
shows and does not treat as a block. It never exits `2`, which Claude Code
would read as a block, so a stray word in a hook command cannot brick a
session.

## Colour?

`ctrm` never prints colour. `--no-color` and `NO_COLOR` are accepted and
change nothing, so wrappers that pass them to every tool keep working.
