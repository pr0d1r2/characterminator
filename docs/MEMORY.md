# Memory

How much RAM `ctrm` uses, what makes it grow, and why it is built the way it
is. Every figure here was measured, not estimated; the method is at the end.

## The short version

- **A normal run uses about 8-15 MB.** Checking or counting this repository
  (about 100 tracked files) peaks at 8 MB.
- **Memory grows with the number of findings, not with the size of the
  files.** An 11 MB file with nothing to report peaks at 14 MB. A 9 MB file
  where nearly every character is a finding peaks at 268 MB.
- **The cases that use hundreds of MB are pathological**: millions of
  findings in one run. That happens when a file is judged against a set it
  was never meant to meet -- for example a whole book of non-Latin text under
  the default `ascii`.

## Measured

Release build, macOS arm64. "Peak" is the maximum resident set size: the
most RAM the process held at once.

| run | input | findings | time | peak |
|---|---|---|---|---|
| `ctrm check` on this repository | ~100 files | 0 | 0.04 s | 8 MB |
| `ctrm stats` on this repository | ~100 files | -- | 0.05 s | 8 MB |
| `ctrm check` | 11 MB, plain ASCII | 0 | 0.10 s | 14 MB |
| `ctrm check --pedantic` | 11 MB, plain ASCII | 0 | 0.12 s | 14 MB |
| `ctrm check --rule '* any'` | 9 MB, mixed scripts | 0 | 0.23 s | 12 MB |
| `ctrm check` | 9 MB, mixed scripts | 2.06 M | 0.56 s | 268 MB |
| `ctrm check --format json` | same | 2.06 M | 1.43 s | 503 MB |
| `ctrm check --format sarif` | same | 2.06 M | 0.90 s | 651 MB |
| `ctrm fix --check` | same | 0.69 M rewrites + 1.37 M left | 0.69 s | 385 MB |
| `ctrm fix --check --format json` | same | same | 0.90 s | 579 MB |

The mixed file is 5.5 million characters drawn at random from ASCII letters,
spaces, `e` with an acute accent, an em dash and a CJK character, so about
three in eight characters fall outside `ascii`.

## Where the memory goes

For a run with findings, three things dominate:

1. **The findings themselves.** Each one records the character, its line,
   column and byte offset, the lint that fired and its level: about 64 bytes.
   Two million findings are about 130 MB.
2. **The report text.** The whole report is built in memory and printed in
   one write. A human line such as `mixed.txt:1:3 U+2014 ascii` is about 30
   bytes; a json or SARIF entry is several times that, which is why those
   formats peak higher than human output for the same findings.
3. **The file being read**, plus the compiled-in character sets, maps and
   tables -- the 8-15 MB floor every run pays.

`fix --check` holds two lists: every rewrite it would make, and everything
the rewritten text would still be reported for, judged the same way `check`
judges it. Both are held the way `check` holds its findings -- the path
once per file, each row moved rather than copied -- so on the mixed file it
peaks at 385 MB, about 1.45 times `check`. It used to copy the path, the
set and every row on the way to the report, and peaked at over 900 MB;
`src/fix/SPEC.md` R18 records the before and after.

## Why the report is built before it is printed

So that a run that fails part way prints nothing rather than a report that
looks complete. A gate that streamed its findings and then stopped on an
unreadable file would leave half a report on screen, and a reader -- or a
script -- could not tell it from a whole one.

Piping into `head` does not reduce memory for the same reason: the report
exists in full before the first line is written. `ctrm check | head` still
exits with the right verdict (`src/cli:V47`).

## The guard reads at most 16 MiB

`ctrm guard` runs once per `Read` an agent makes, inside the harness, so a
multi-GB file read whole would put GBs in a hook. It judges at most the
first 16 MiB, cut back to the last line break inside that, and says so: a
hazard in the prefix still denies the read; otherwise the read goes ahead
with a note that the rest was not judged. It never denies for size alone
and never passes a capped file in silence (`src/cli/guard:V102`). `ctrm
check` has no cap and judges the whole file.

The cap bounds the file's bytes, and with them the findings, so it also
bounds the pathological case above: a guard read costs at most what
`check` costs on a 16 MiB file.

## Keeping it small

- **Grant the sets a file actually needs.** Memory follows findings, and
  findings follow the gap between a file and its grant. A Chinese document
  under `ascii` is two million findings; under `ascii+zh` it is the handful
  of characters that are really out of place.
- **Prefer human output for very large reports.** It is the most compact of
  the three formats.
- **Check large generated or vendored files with `any`**, or leave them out
  of the paths you name. `any` still reports hazards, which is the part
  worth keeping.

## What changed, and what is left

The first measurements of the 2.06-million-finding case were far worse:
753 MB for human output, about 2 GB for json and 2.8 GB for SARIF. Every
finding carried its own copy of the path and the set name, and the path was
re-escaped character by character for every finding. Those copies are gone;
a path is held and escaped once per file. The research row
`src/render:R17` records the before and after.

What remains is the price of building the whole report first. Halving it
again would mean streaming, with a per-file buffer so a failure never cuts a
file's findings in half. That is possible future work, and only worth doing
for inputs like the ones in the table above.

## Method

Each figure is the "maximum resident set size" reported by
`/usr/bin/time -l` for one run of the release binary, with output sent to
`/dev/null`. Times are wall clock from the same run, on a machine that was
otherwise lightly loaded, so treat them as approximate.
