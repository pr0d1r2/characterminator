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
  a tag character, a stray byte order mark or a control character that
  `ctrm check` does not report, under any configuration. Every hazard is
  reported at `forbid`, and no `.ctrm` line or flag lowers it.
- **Guard bypasses.** `ctrm guard` decides in two tiers. It **blocks** a
  tag character (U+E0000-U+E007F) or a bidi embedding, override or isolate
  (U+202A-U+202E, U+2066-U+2069): a `Read` holding one is denied, and tool
  output holding one is flagged as tainted. Every other hazard -- the
  direction marks, soft hyphens, zero-width characters, a stray byte order
  mark, control characters -- it lets through with a note to the model. In
  scope: a `Read`, web fetch, web search, shell, grep, subagent or MCP output
  holding a blocked character that the guard passes, a note-tier hazard it
  passes without a note, or an input that makes the guard exit `2` (which
  Claude Code reads as "block every call") or crash where it should decide.
  A note-tier hazard passing with its note is the design, not a bypass.
- **The sequence exemptions.** A joiner, tag or variation selector that
  escapes the hazard lints through the emoji sequence exemption or the
  script joiner exemption for Persian and Hindi where that exemption does
  not apply: outside a listed emoji sequence, or in a file whose rule does
  not name the shipped `persian` or `hindi` set.
- **`fix` writing a disallowed character.** A rewrite that leaves, or
  introduces, a character outside the set the file is granted, or a hazard,
  while reporting success.
- **The GitHub Action's template-injection surface.** Any input or path
  that reaches a shell as code rather than as data (`action.yml`).

Out of scope: findings that are working as specified -- a character the
configuration grants, a note-tier hazard the guard notes rather than
blocks -- and the cost of pathological inputs already documented in
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
