# Working here

For agents and humans. Read this before changing anything; read `SPEC.md`
for what must hold and what to build next.

## The spec is the law

`SPEC.md` at the root, plus one in every node directory under `src/`. It is
not a description written afterwards: it holds the invariants that must stay
true, the tasks that remain, and a record of every bug found so far paired
with the rule that now catches it.

Fifteen files, not one -- the root, the `src` hub, nine nodes, and four
sub-nodes (`guard` and `explain` under `src/cli`, `emoji` and `words`
under `src/fix`); `find src -name SPEC.md` lists them -- because a
session should load the chain it needs -- root plus hub plus the node
it is working in, plus the parent for a sub-node -- rather than
everything. Each node owns its own rules and cites its siblings by
name.

`mth tasks SPEC.md` prints the backlog in id order. A row carrying `~` is
work somebody started. Only `/spec` edits the spec; `/build` flips a status
cell and nothing else.

Three conventions that are easy to break by accident:

- **Ids are never reused.** A gap costs nothing; a reused id silently
  redirects every citation that pointed at the old meaning.
- **Rows stay in id order**, and a suffixed id rides its base.
- **A rule and its runner land in the same commit.** A rule nothing checks
  is decoration, which is why `V44` arrived with `T48` rather than alone.

## The gate

`hk.pkl` defines every op once, and each one is a command you can paste into
a shell. Entering the dev shell -- `direnv allow`, or `nix develop` --
installs the hooks.

```bash
hk check --all        # everything
cargo test            # the fast inner loop
```

The steps, and what each is for:

| step | what it refuses |
|---|---|
| `fmt`, `clippy`, `test` | the usual, with clippy at `-D warnings` |
| `ctrm` | this tool, run on this repository's own tree |
| `mth`, `mth-check` | every `SPEC.md` is well formed and still correct, and no closed option recorded in a `.spec-records` (the root's, or one beside a node's spec) has gone missing |
| `context-limits` | no spec is over the token ceiling it declared |
| `sherd-check`, `sherd-sync`, `sherd-budget` | the node tree resolves, `NAV` is not stale, and no node chain is over its ceiling |
| `flake-tags` | every flake input pins a `vX.Y.Z` tag, not a branch (V44) |
| `hook-guard` | a `flake.nix` shellHook that installs a hook before both of its guards (V40) |
| `emoji-seq-map` | an emoji sequence map that is not what its generator writes from the vendored data (`src/charset:V97`) |
| `coverage-badge` | the README coverage badge says what `.coverage` claims, copied rather than typed |
| `coverage` | line coverage under the floor, or a `.coverage` claim this run does not reproduce (V46); `hk check` and pre-push, not pre-commit |
| `no-commit-to-branch` | a commit to `main`; pre-commit only |

The cargo steps are chained on purpose. Cargo locks the target directory, so
two cargo jobs launched in parallel do not run in parallel -- the second
blocks on the lock, which reads as a hang.

**A missing tool fails the step rather than skipping it.** `mth`, `itok` and
`sherd` each check they are on `PATH` and exit 1 with an instruction if they
are not. Nothing else checks what they check, so a skip would read exactly
like a pass. That is the failure mode this repository keeps finding in its
own gates, and it is worth naming because the fix always looks like extra
noise until the day it does not.

## The clippy configuration is strict, deliberately

`unwrap_used`, `expect_used`, `panic`, `indexing_slicing` and
`arithmetic_side_effects` are **denied**, including in tests. So are
`too_many_lines` at 15 and `too_many_arguments` at 4.

That last pair is the one that will bite you. A function that grows past
fifteen lines is asking to be split, and a call taking five arguments is
usually a struct that has not been written yet. Both are cheap to fix and
the fix is nearly always better; resist the urge to reach for an `#[allow]`.

In tests, `assert!(x.is_ok())` followed by `unwrap_or_default()` is the
idiom that replaces `expect`.

## Dogfooding is not optional

The `ctrm` gate step runs this tool over this tree. If your change makes the
tool reject a file that should be fine, the answer is one of:

1. the character does not belong there -- fix the file;
2. it belongs there -- **grant it in `.ctrm`, and say why in the commit**,
   because a grant is a decision;
3. the tool is wrong -- which is a bug, and a bug gets a row in the
   spec's BUGS section, paired with the invariant that now catches it.

Never widen a grant to make a gate go green without writing down why. The
whole repository is an argument that a rule should be checkable, and
`* any` at the top of `.ctrm` would end that argument quietly.

## Commits

Conventional Commits. One commit per decision, and the body carries what was
chosen, what was **rejected**, and why -- written for someone auditing this
later who cannot ask you.

The repository keeps its rejected options in commit messages rather than in
the spec, which is what `.spec-records` says in its own header. A closed
option that becomes load-bearing gets a line in that file, and then `mth
check --records` fails if a later edit quietly drops it.

Never commit to `main`; a hook refuses it.

## The tools in the gate are siblings, not dependencies

`mth`, `itok`, `sherd` and `hk` arrive as pinned flake inputs and are called
as binaries. They are not linked into this crate and a consumer building
from crates.io never sees them.

Three direct dependencies, each behind one facade node (`src:C`) and
justified in `docs/THIRD-PARTY-NOTICES.md`. `itok`, for token counting. This crate deliberately
does not write a second counter -- a saving reported here has to be the same
number `itok` reports elsewhere, and two counters drift the moment either
changes. `src/tokens` is its only call site, so the blast radius is one
module.
`unicode-normalization` and `unicode-security` carry the Unicode tables the
pedantic lints read; `src/lint/pedantic/ucd.rs` is their only call site.

Flake inputs pin **tags**, bumped in their own reviewed commit (V44).
Following a branch is rejected: what the gate enforces would move with no
diff to read.
