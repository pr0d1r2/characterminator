# Contributing

Thank you for looking. This page is the short version; [AGENTS.md](AGENTS.md)
holds the full rules, for humans and agents alike, and `SPEC.md` holds what
must stay true.

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## The quick path: no nix

A Rust toolchain at 1.95 or later is all the code needs.

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

That covers the code. It does not cover the spec checks, the token
ceilings or the dogfood run, and you do not need to run them yourself: CI
runs the full gate on every pull request, on Linux and macOS, and tells you
what failed and how to fix it.

## The full path: with nix

```bash
nix develop        # or: direnv allow
hk check --all     # the whole gate, about 20 s warm
```

Entering the dev shell puts every tool the gate calls on `PATH`, pinned by
`flake.lock`, and installs the `pre-commit` and `pre-push` git hooks. From
then on a commit runs the fast set and a push adds line coverage. Without
the dev shell the hooks refuse rather than pass unexamined; that is
deliberate.

The first entry downloads the toolchain. To use the project's binary cache
rather than build `hk` from source, either be a trusted user of your nix
daemon (`trusted-users` in `nix.conf`) and accept the cache when asked, or
pass the flag once:

```bash
nix develop --accept-flake-config
```

### Installing the tools by hand

If you would rather not use nix, these are the versions the gate pins.
Older ones may read the specs differently.

| tool | version | what it does here |
|---|---|---|
| `hk` | 1.58.1 | runs the gate defined in `hk.pkl` |
| `mth` (crate `microlith`) | 0.7.3 | formats and checks every `SPEC.md` |
| `sherd` | 0.5.3 | checks the spec tree and each node's token budget |
| `itok` | 0.3.1 | checks the per-file token ceilings |
| `cargo-llvm-cov` | 0.8.7 | line coverage, pre-push and CI only |

`flake.nix` and `flake.lock` are the source of truth if this table and they
ever disagree.

## How a change is shaped

- **A bug becomes two things**: a row in the spec's bugs section (`§B`) and
  the invariant (`§V`) that would have caught it, with a test that runs it.
- **A rule and its runner land together.** A rule nothing checks is
  decoration, so a new invariant arrives in the same commit as the test or
  gate step that enforces it.
- **One decision per commit.** Conventional Commits, and the body says what
  was chosen, what was rejected, and why -- written for someone reading the
  history later who cannot ask you.
- **A token ceiling is raised in its own commit**, with the reason. A
  ceiling may not pass its cap (3500 tokens for a spec file, 5500 for a
  node's chain); a node that would is split instead.
- **Never commit to `main`.** A hook refuses it; open a pull request.

## Finding something to do

Every node keeps its own backlog. To list the open and started tasks across
all of them:

```bash
for f in $(find . -name SPEC.md -not -path './target/*' | sort); do
  mth tasks "$f" | grep -E '^task [^:]+: [.~] ' | sed "s|^|$f: |"
done
```

`.` is open and `~` is started. Issues on GitHub are welcome too, and are
the place to ask before starting something large.

## Using an LLM or coding agent

Allowed. Most of this repository was written that way, which
[docs/LLM-DISCLAIMER.md](docs/LLM-DISCLAIMER.md) describes. Two conditions:

- **Disclose it** with a `Co-Authored-By:` trailer naming the model, on
  every commit it helped write.
- **The review bar is the same.** You are the author of what you submit:
  the gate, the commit-body rules and a reviewer's questions apply exactly
  as they would to code you typed.

[AGENTS.md](AGENTS.md) is written for agents and humans both, and is the
file to point your agent at.
