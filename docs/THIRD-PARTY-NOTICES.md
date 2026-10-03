# Third-party notices

`characterminator` has **three direct dependencies**, ships **one vendored
file**, and compiles in **data generated from the Unicode Character
Database**. This document says what each is, why it is there, and what you
owe whom if you distribute a binary.

Every figure below is the *runtime* closure -- `cargo tree -e normal`,
measured rather than asserted. Dev-dependencies are excluded: they are not
distributed in anything you run.

## Direct dependency: `itok`

[`itok`](https://github.com/pr0d1r2/itok) counts tokens and lists the
git-tracked files. MIT licensed.

It is here because of a rule rather than a convenience. A saving this tool
reports has to be the same number `itok` reports elsewhere, and two
counters drift the moment either one changes. So there is no second counter
in this crate, `src/tokens` is `itok`'s only call site, and the blast
radius of the dependency is one module.

Taken as `default-features = false, features = ["bpe"]`:

- **`bpe` is kept** because it *is* the real-tokenizer tier. Without it the
  only available count is bytes/4, and `stats --bpe` would have to claim a
  measurement it could not make.
- **`session` is dropped.** It reads agent transcripts, which this crate
  never does. Dropping it removes six packages -- `serde_core`,
  `serde_derive`, `syn`, `quote`, `proc-macro2` and `unicode-ident` -- from
  the closure.

That last one matters more than its size. `unicode-ident` carries
`(MIT OR Apache-2.0) AND Unicode-3.0`, where the choice applies to the
first half only, so the Unicode obligation would hold however you took the
rest. It is the only non-choice licence anywhere near this tree, and this
build does not carry it.

## Direct dependencies: `unicode-normalization` and `unicode-security`

The four data-bound pedantic lints read published Unicode tables: normalization
for `not-nfc` and `nfkc-compat`, UTS #39 for `mixed-script` (the resolved script
set) and `confusable` (the skeleton). Both crates are from the `unicode-rs`
project, MIT OR Apache-2.0, and `src/lint/pedantic/ucd.rs` is their only call
site.

They are here because the alternative is a second private copy of a
published table, which drifts from Unicode the day Unicode moves. Vendoring
the UCD files and generating tables, as `hazard.ctrm-sets` does, was
rejected for these four: decomposition and confusable data are far larger
than a range list, and the composition algorithm is code, not data.

Both are taken with `default-features = false` (`unicode-normalization`
drops `std`). They add four packages: themselves, `unicode-script` (beneath
`unicode-security`) and `tinyvec`. Their tables are generated from the
Unicode Character Database -- version 17.0.0 for normalization and script,
16.0.0 for the UTS #39 confusables -- so the Unicode License below applies
to that data as well.

## The closure: 26 packages

Three direct, twenty-three transitive. Twenty sit beneath `tiktoken-rs`,
which is what a real tokenizer costs; the Unicode tables cost four.

| package | version | licence |
|---|---|---|
| `aho-corasick` | 1.1.5 | Unlicense OR MIT |
| `anyhow` | 1.0.104 | MIT OR Apache-2.0 |
| `base64` | 0.21.7 | MIT OR Apache-2.0 |
| `bit-set` | 0.5.3 | MIT/Apache-2.0 |
| `bit-vec` | 0.6.3 | MIT/Apache-2.0 |
| `bstr` | 1.13.1 | MIT OR Apache-2.0 |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 |
| `fancy-regex` | 0.13.0 | MIT |
| `itok` | 0.3.1 | MIT |
| `lazy_static` | 1.5.0 | MIT OR Apache-2.0 |
| `libc` | 0.2.189 | MIT OR Apache-2.0 |
| `lock_api` | 0.4.14 | MIT OR Apache-2.0 |
| `memchr` | 2.8.3 | Unlicense OR MIT |
| `parking_lot` | 0.12.5 | MIT OR Apache-2.0 |
| `parking_lot_core` | 0.9.12 | MIT OR Apache-2.0 |
| `regex` | 1.13.1 | MIT OR Apache-2.0 |
| `regex-automata` | 0.4.18 | MIT OR Apache-2.0 |
| `regex-syntax` | 0.8.11 | MIT OR Apache-2.0 |
| `rustc-hash` | 1.1.0 | Apache-2.0/MIT |
| `scopeguard` | 1.2.0 | MIT OR Apache-2.0 |
| `smallvec` | 1.16.1 | MIT OR Apache-2.0 |
| `tiktoken-rs` | 0.6.0 | MIT |
| `tinyvec` | 1.13.3 | Zlib OR Apache-2.0 OR MIT |
| `unicode-normalization` | 0.1.25 | MIT OR Apache-2.0 |
| `unicode-script` | 0.5.8 | MIT OR Apache-2.0 |
| `unicode-security` | 0.1.2 | MIT/Apache-2.0 |

The versions are what `Cargo.lock` resolves today. The lock file is tracked
and is the authority; this table is a readable copy of it, regenerated
rather than edited.

Six licence expressions appear, all permissive:

- `MIT OR Apache-2.0` (16 packages)
- `MIT` (3)
- `Unlicense OR MIT` (2)
- `MIT/Apache-2.0` (3) -- older syntax, same meaning
- `Apache-2.0/MIT` (1) -- likewise
- `Zlib OR Apache-2.0 OR MIT` (1)

Where an expression offers a choice, `characterminator` is distributed
under MIT and takes the MIT option.

`Cargo.lock` carries three packages this table does not: `bitflags`,
`redox_syscall` and `windows-link`. They resolve only on platforms other
than the one measured here, and they are permissive on the same terms.

## The vendored file: `pkl/Config.pkl`

[`pkl/Config.pkl`](../pkl/Config.pkl) is the configuration schema of
[`hk`](https://github.com/jdx/hk), the gate runner, vendored here rather
than fetched from a `package://` URL when the gate is evaluated.

The reason is the gate's own rule: it runs with **no network**. A schema
fetched at evaluation time would make every commit depend on a host being
reachable, which is a dependency nobody declared and nobody can audit.

It is not forked, not generated and not edited. Any change to it is a
re-sync from upstream. `hk` is MIT licensed.

## Generated data: the Unicode Character Database

Some of what the binary enforces is Unicode's data rather than ours, and it
is compiled in, so it travels inside every copy of the binary.

| file | generated from | Unicode version |
|---|---|---|
| [`src/charset/emoji-sequences.txt`](../src/charset/emoji-sequences.txt), and [`src/fix/emoji/emoji-seq.ctrm-map`](../src/fix/emoji/emoji-seq.ctrm-map) built from it | Unicode emoji `emoji-sequences.txt` (presentation, keycap, flag and tag sequences) and `emoji-zwj-sequences.txt` (ZWJ sequences): the five kinds `presentation`, `keycap`, `flag`, `tag` and `zwj` | emoji 18.0 |
| [`src/charset/hazard.ctrm-sets`](../src/charset/hazard.ctrm-sets) | `DerivedCoreProperties.txt` (`Default_Ignorable_Code_Point`), `PropList.txt` (`Bidi_Control`), `extracted/DerivedGeneralCategory.txt` (`Cc`) | 18.0.0 |
| [`src/charset/locale/locales.ctrm-sets`](../src/charset/locale/locales.ctrm-sets) | CLDR `cldr-json` tag 48.2.3, every locale's `cldr-misc-full/main/<code>/characters.json` (exemplar characters), `cldr-core` `availableLocales.json`, `defaultContent.json`, `parentLocales.json` and `likelySubtags.json` (the locale list and parent chain), with `UnicodeData.txt` for uppercase forms | CLDR 48, UCD 18.0.0 |

The emoji sequence file is the presentation, keycap, flag, tag and ZWJ
sequences of those two files, one per line in `U+XXXX` notation, written
by `src/charset/emoji-sequences.sh`; its header carries both files' dates
and SHA-256 digests. The map is generated from it by the same script, plus
a seven-row table written by hand and cited there (UTS #51 section 2.6).

The hazard file is the code point ranges of those properties, rewritten
into this tool's own `U+XXXX` set grammar; its header names the upstream
files, their dates and SHA-256 digests, and the command that produced each
line. The locale file is generated the same way, by
`src/charset/locale/cldr-letters.sh`, and its header carries the tag,
each core input's SHA-256, and one SHA-256 over the digests of all 766
locale files. No upstream file is copied into the repository whole.
CLDR is distributed under the same Unicode License v3 as the UCD, so the
text below covers both.

The Unicode Character Database is distributed under the Unicode License
v3, reproduced below as Unicode publishes it at
<https://www.unicode.org/license.txt>, except that the copyright sign is
written `(c)`: this file is held to plain ASCII by the tool's own check.

```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright (c) 1991-2026 Unicode, Inc.

NOTICE TO USER: Carefully read the following legal agreement. BY
DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

Permission is hereby granted, free of charge, to any person obtaining a
copy of data files and any associated documentation (the "Data Files") or
software and any associated documentation (the "Software") to deal in the
Data Files or Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, and/or sell
copies of the Data Files or Software, and to permit persons to whom the
Data Files or Software are furnished to do so, provided that either (a)
this copyright and permission notice appear with all copies of the Data
Files or Software, or (b) this copyright and permission notice appear in
associated Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
THIRD PARTY RIGHTS.

IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in these Data Files or Software without prior written
authorization of the copyright holder.
```

## Not dependencies: the tools in the gate

Four sibling tools appear in the dev shell and in the gate, and are **not**
linked into this crate. A consumer installing the binary never sees them.

- [`microlith`](https://github.com/pr0d1r2/microlith) (`mth`) -- owns the
  `SPEC.md` format and checks all ten spec files.
- [`itok`](https://github.com/pr0d1r2/itok) -- also called as a binary, to
  enforce the per-spec token ceilings. The same tool as the dependency
  above, in a second role.
- [`sherd`](https://github.com/pr0d1r2/sherd) -- checks the node
  federation: that every declared node resolves, that sibling lenses stay
  disjoint, and that no `NAV` section is stale.
- [`hk`](https://github.com/jdx/hk) -- the gate runner.

All four are MIT licensed. They arrive as pinned flake inputs, at tags
rather than branches, so what the gate enforces cannot move without a diff
to review.

## Reproducing these numbers

Nothing here is hand-maintained, and you should not trust it because it is
written down:

```bash
cargo tree -e normal                    # the closure in the table above
cargo tree -e normal --no-default-features
cargo deny check licenses               # if you have cargo-deny
```

## Trademarks

Nominative use only; no affiliation or endorsement is implied.

- **Rust** and **Cargo** are trademarks of the Rust Foundation.
- **GitHub** is a trademark of GitHub, Inc.
- **NixOS** and **Nix** are trademarks of the NixOS Foundation.
- **Linux** is a registered trademark of Linus Torvalds.
- **Claude** and **Anthropic** are trademarks of Anthropic PBC.
- **Unicode** is a registered trademark of Unicode, Inc.

## `characterminator` itself

Everything in this repository that is not covered above is licensed under
the MIT License -- see [`LICENSE`](../LICENSE).
