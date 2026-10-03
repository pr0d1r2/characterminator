## What and why

<!-- One decision per commit; the commit bodies carry chosen, rejected and why. -->

## Checklist

- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass (CI runs the full gate)
- [ ] a bug fix adds a `§B` row and the `§V` invariant that catches it, with its test
- [ ] a new rule lands with its runner (test or gate step) in the same commit
- [ ] any token ceiling raise is its own commit, under the V103 cap
- [ ] any new `.ctrm` grant says why in the commit
- [ ] LLM or agent help is disclosed with a `Co-Authored-By:` trailer

See [CONTRIBUTING.md](../CONTRIBUTING.md).
