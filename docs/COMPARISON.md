# How ctrm compares

Three tools a reader is likely to reach for first, what each is built for,
and where `ctrm` differs. Every claim about another tool is taken from its
own README as of 2026-10-03; if one is out of date, an issue or a pull
request correcting it is welcome.

## The short version

- To **block Trojan Source and invisible characters** in a JavaScript
  project, with an ESLint rule, `anti-trojan-source` is built for exactly
  that.
- To **require plain ASCII or a declared encoding per file type**, as part
  of checking indentation and line endings, `editorconfig-checker`'s
  `charset = ascii` does it.
- To **say which characters each path may hold**, rewrite the rest, and
  know what that saves in tokens, that is what `ctrm` is for.

## anti-trojan-source

An npm package with a CLI, a library and an ESLint plugin
([lirantal/anti-trojan-source](https://github.com/lirantal/anti-trojan-source)).
It reports every Unicode Format (Cf) and Control (Cc) character except tab,
line feed and carriage return, plus an explicit list of variation
selectors and invisible letters; `--extended` adds a curated set of ASCII
lookalikes. It reads files by glob or path, and also from standard input,
and has a JSON output mode.

Where `ctrm` differs:

- **An allowlist, not a blocklist.** `ctrm` reports everything outside the
  set a path is granted, so a stray accented letter in an ASCII file is a finding too.
  Its hazards (bidi controls, tag characters, stray byte order marks,
  control and default-ignorable characters) are reported on top of that,
  whatever the grant.
- **Per path.** `.ctrm` grants sets by glob, last match wins, and `ctrm
  explain <path>` names the line that decided.
- **It fixes.** `ctrm fix` rewrites through a map and deletes hazards.
- **Agents.** `ctrm guard` is a Claude Code hook that blocks smuggling
  characters in a file before it is read and in tool output.
- **SARIF** for GitHub code scanning, beside json.

Where anti-trojan-source fits better: a JavaScript toolchain where ESLint
is the gate, and input piped on standard input, which `ctrm` does not
read.

## editorconfig-checker

Checks files against their `.editorconfig`
([editorconfig-checker/editorconfig-checker](https://github.com/editorconfig-checker/editorconfig-checker)):
indentation, line endings, trailing whitespace, final newline, line length
and `charset`. With `charset = ascii` on a glob, files there must be plain
ASCII; other values check the encoding (`utf-8`, `utf-8-bom`, `utf-16be`,
`utf-16le`, `latin1`).

Where `ctrm` differs: the choice is wider than "ASCII or not". A path can
be granted `ascii+typography`, a locale's letters such as `pl`, or a set
you declare, and `fix` rewrites what falls outside instead of only
reporting it. `ctrm`'s opt-in pedantic lints cover line endings, trailing
whitespace and the final newline too, but not indentation or line length.

Where editorconfig-checker fits better: a repository that already keeps
an `.editorconfig` and wants one checker for all of it.

## A grep one-liner

```bash
git grep -nP '[^\x00-\x7F]'
```

finds every non-ASCII byte in tracked files, with no install. It has no
notion of a grant, so every exception becomes a pathspec; it prints the
line, not the code point, so an invisible character is invisible in its
own report; and it fixes nothing.

## When not to use ctrm

- Input on **standard input**: not read. `ctrm check -` is refused; name
  the files.
- **Outside a git repository**, a bare `ctrm check` is refused, because the
  tracked files are what it checks; name the paths instead.
- **Windows** is not tested.
- `guard` speaks **Claude Code's hook format** only.
- **Token savings** come from rewriting characters a tokenizer splits.
  Rewriting typography saves bytes and often no tokens at all; `ctrm stats`
  says what a fix would save before you run one ([What it costs](../README.md#what-it-costs)).
