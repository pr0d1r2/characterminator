# Third-party notices

`characterminator` has **one direct dependency** and ships **one vendored
file**. This document says what each is, why it is there, and what you owe
whom if you distribute a binary.

Every figure below is the *runtime* closure -- `cargo tree -e normal`,
measured rather than asserted. Dev-dependencies are excluded: they are not
distributed in anything you run.

## The one direct dependency: `itok`

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

## The closure: 22 packages

One direct, twenty-one transitive, nearly all of them beneath
`tiktoken-rs`, which is what a real tokenizer costs.

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

The versions are what `Cargo.lock` resolves today. The lock file is tracked
and is the authority; this table is a readable copy of it, regenerated
rather than edited.

Five licence expressions appear, all permissive:

- `MIT OR Apache-2.0` (14 packages)
- `MIT` (3)
- `Unlicense OR MIT` (2)
- `MIT/Apache-2.0` (2) -- older syntax, same meaning
- `Apache-2.0/MIT` (1) -- likewise

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
