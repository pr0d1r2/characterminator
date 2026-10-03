# Security

`ctrm` exists to stop characters that read one way to a reviewer and
another way to a model. A way around that is a security bug, and this file
says how to report one.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting on this repository: the
**Security** tab, then **Report a vulnerability**
(<https://github.com/pr0d1r2/characterminator/security/advisories/new>).
The report stays private between you and the maintainer until a fix is
published.

Please do not open a public issue or pull request for a vulnerability
before it is fixed. A minimal reproducer -- the bytes of a file, a hook
payload, or a workflow -- is the most useful thing a report can carry.

## In scope

- **Hazard detection bypasses.** A bidi control, an invisible character,
  tag smuggling or another hazard class (`src/lint:V34`) that `ctrm check`
  does not report, under any configuration.
- **Guard bypasses.** A `Read`, web fetch, web search, shell or MCP output
  holding a hazard that `ctrm guard` passes, or an input that makes the
  guard exit `2` (which Claude Code reads as "block every call") or crash
  where it should decide (`src/cli/guard`).
- **The sequence exemptions.** A joiner, tag or variation selector that
  escapes the hazard lints through the emoji sequence exemption
  (`src/lint:V63`) or the script joiner exemption for Persian and Hindi
  (`src/lint:V57`) when it is not part of what those rules exempt.
- **`fix` writing a disallowed character.** A rewrite that leaves, or
  introduces, a character outside the set the file is granted, or a hazard,
  while reporting success.
- **The GitHub Action's template-injection surface.** Any input or path
  that reaches a shell as code rather than as data (`action.yml`, root
  `SPEC.md` V52).

Out of scope: findings that are working as specified -- a character the
configuration grants, a hazard reported at `warn` because a `.ctrm` said
so -- and the cost of pathological inputs already documented in
[docs/MEMORY.md](docs/MEMORY.md).

## Supported versions

Only the latest `0.x` release receives fixes. Before `1.0`, a fix lands in
the next release rather than as a backport.

## What to expect

This is a small project maintained on a best-effort basis. There is no
service-level agreement: a report will be read, acknowledged when it is,
and fixed as soon as the maintainer can. A confirmed bypass is fixed
together with a rule in the spec that catches it from then on, and the
reporter is credited in the advisory unless they ask not to be.
