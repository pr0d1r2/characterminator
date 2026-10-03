# Built by an LLM, deliberately and in the open

This repository -- code, spec, tests and prose -- was written by
[Claude Code](https://claude.com/claude-code) running Anthropic's **Claude
Opus 5** and, from later in the work, **Claude Opus 5.5**. Nearly every
commit carries a `Co-Authored-By:` trailer naming the model that wrote
it, `Claude Opus 5` or `Claude Opus 5.5`, so `git log` shows which. A
human owns every decision, reviews every diff, and is accountable for what
ships.

That is the disclaimer. The rest of this file is why it is stated as a
design note rather than as an apology, and what a reader can check for
themselves.

## Why say it at all

Two reasons, and only the first is obvious.

A model writes plausible code, and plausible is not correct. A reader who
does not know how a repository was produced cannot calibrate how hard to
look at it. Saying so is the minimum.

The second is specific to a tool like this one. `ctrm` exists to make a
claim about a file mechanical -- that every character in it was granted by
a rule somebody wrote down -- so that "our prose is ASCII" becomes a
statement about a file that was parsed rather than one that was skimmed. A
repository built by a model, arguing that claims should be checked rather
than asserted, has to hold itself to that first. Everything below is an
attempt to make the provenance checkable instead of merely disclosed.

## The method is spec-driven development

[`SPEC.md`](../SPEC.md) is the law rather than a description written
afterwards, and there is one at the root and one per node and sub-node
under `src/`. Each holds the invariants that must stay true, the tasks that
remain, and the bugs found so far paired with the rule that now catches
each one.

A rule and its runner land in the **same commit**. A rule with no runner
gates nothing, and this repository has already caught itself writing one:
`V44` fixed the pin policy and `T48` -- the gate step that checks it --
arrived beside it rather than later.

## What the gate proves, and what it does not

Every commit runs `fmt`, `clippy -D warnings`, the test suite, this tool on
its own tree, the spec format checker on every spec file, a token
ceiling per spec, and the federation checker. `hk.pkl` defines all of it,
and each step is a command a human can paste into a shell -- so a verdict
never rests on the runner's own logic.

What that does **not** prove is that the design is right. A gate catches a
regression against a rule somebody already thought of. It has nothing to
say about a rule nobody wrote.

## Three defects this method actually caught

Stated concretely, because "the gate works" is the kind of claim this
document exists to distrust.

**A section sign in a Rust doc comment.** The tool's first run over its own
tree found a non-ASCII character in a source file, in a repository whose
constraints say sources are ASCII. Nothing else would have found it: it
renders fine, compiles fine, and reads as deliberate.

**A preset that granted what another rule forbade.** The spec's summary
listed the emoji block as one unbroken range, while a different node
withholds the skin tone modifiers that sit **inside** that range. A data
file written from the summary granted them. A test caught it, the spec was
amended, and a new invariant now says a preset's ranges must be written
around what another node withholds.

**Two spellings of one thing.** An invariant fixes a single format for
naming where a config line came from, precisely so an error message and a
report can never disagree. They had: one said `.ctrm:2` and the other said
`.ctrm line 2`. Nothing failed, because nothing compared them -- it
surfaced only when a new verb printed one beside the other.

All three are the same shape: something that looked right from every angle
except the one nobody was standing at.

## What to check for yourself

- `git log` -- the reasoning is in the commit bodies, including what was
  **rejected**. That is where this project keeps its closed options.
- [`SPEC.md`](../SPEC.md) and the one beside each node under `src/` -- what must hold, and
  every bug paired with the invariant that now catches it.
- [`.spec-records`](../.spec-records) -- decisions that must survive a
  later edit, enforced by the gate.
- `hk check --all` -- run the gate yourself. It needs no network.
- The tests. 347 of them, and the ones worth reading are the tests that
  prove a guard is **selective**: not only that it catches the bad case,
  but that it still accepts every real shape.

## What a model is bad at, in this codebase specifically

Worth naming, because it tells a reviewer where to look.

**Numbers that flatter the work.** The measured claim that rewriting
typography saves tokens turned out to be false under a modern tokenizer --
it saves bytes. That correction is in the README because the first draft of
this project's own pitch did not make it.

**Plausible restatement.** A model will happily write a second
implementation of a rule that already exists elsewhere, and it will be
subtly different. The federation layout and the rule that no node calls a
sibling exist partly to make that mistake visible.

**Confident prose about work that is not done.** Every claim in the README
about a verb was run before it was written, and the ones that are not done
are in a list that says so.
