# SPEC

## §G GOAL

Which set applies WHERE: config files & flags, precedence, origin, fidelity choice.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/judge|findings from rules, sets, lints, scan & fix; config assembly
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- file: `.ctrm` — line-based, `<glob|path> <set>[+<set>...] [@<family>] [!<level>] [!<lint|group>=<level>]`, `#` comment. zero-dep parse (`.context-limits` shape).

## §V INVARIANTS

V1: path w/ no matching rule → `ascii`. strict default; extended set = explicit grant. ≠ `itok`'s opt-in `.context-limits`: here an unguarded char IS the cost.
V2: rule resolution: later matching line wins (gitignore semantics); per-type glob & per-file path share one grammar ∴ per-file line placed after per-type line overrides it. `explain` ! print winner.
V17: locale separation: extended sets granted to data paths (`locales/**`, `*.po`, `config/locales/*.yml`); code stays `ascii`. non-ASCII string literal in code file → the `locale-literal` pedantic lint (`src/lint/pedantic:V37`), ⊥ a mechanism of its own. PENDING `src/lint/pedantic:T58`: ⊥ registered ∴ this clause ⊥ enforced today.
V18: ∀ data-file line kind → flag twin: `--rule` ≡ rules line, `--map` ≡ map line, `--set` ≡ sets line. flag value = exactly 1 line, same parser ∴ ∀ file F: `--no-files` + 1 flag per line of F ≡ F, property-tested.
V19: precedence, low → high: builtin → discovered dotfiles → `--*-file` (argv order) → inline flags (argv order). later wins: rule per V2, map entry per char, set per name.
V20: ∀ effective rule, map entry, set → origin (`<file>:<line>` | `argv[<n>]` | `builtin:<line>`). `explain` ! print it: grant winner + effective family + ∀ effective level, each w/ the origin of the line that set it, ⊥ only the winner's (B28). export via `explain --as` (`src/cli/explain:V32`).
V21: zero-file run: `--no-files --no-builtin-map --no-builtin-sets` → config from argv only. `ascii` intrinsic (code, ⊥ data) ∴ V1 holds w/ ⊥ file.
V24: `ascii` = implicit base ∀ rule: effective set = `ascii` ∪ named sets ∴ `*.md caveman` ≡ `*.md ascii+caveman`. explicit `ascii+` stays legal.
V29: fidelity = family name, default `text`; per rule `@<family>` suffix (`docs/** marks @emoji`); last matching rule naming one wins. `--fidelity <f>` ≡ `--rule '* @<f>'` (V19 order). presets w/ classes grant only resolved family's members ∴ other families compress into it; `src/fix:V6` holds. mix → grant variants explicitly. family ∉ map tree (`src/fix:V27`) → config error, exit 2 ∀ verb (`src/cli:V74`) ∵ a typo resolved to unlabelled members only (B30). `sets` lists at the whole-repo family ∴ both spellings ≡.
V56: a rule naming ⊥ set (`docs/** !warn`) moves levels ONLY, ⊥ competes for the grant: the grant = last matching rule that NAMES a set, as fidelity = last that names a family (V29). `explain` winner = that rule. a level line that won the grant REJECTED ∵ it narrowed the path back to `ascii` & a line written to RELAX a lint made more chars fail (B5).

V45: DISCOVERED dotfile = `.ctrm`, `.ctrm-sets`, `.ctrm-map` @ the RUN ROOT only: `-C <dir>` | else the git work tree holding cwd | else cwd (`src/cli:V115`). ⊥ per-dir & ⊥ nearest-ancestor `.ctrm` ∵ a law that ? sit in any ancestor is one ⊥ reader resolves by looking; the work-tree top is 1 place. `--*-file <f>` resolves as a path arg (`src/cli:V115`); `-C` repeated → last wins (V19).
V75: rule line ! name ≥ 1 of set | `@<family>` | `!<level>`. pattern only (`a.md`) → parse error at its origin, exit 2 ∵ a line that grants & levels ⊥ reads like a rule that worked.
V87: glob match (V2) = iterative, last-star backtrack: `*` over chars, `**` over segments ∴ O(pattern × path), ⊥ recursion, ⊥ alloc; surface rules = virtual `**` segments, ⊥ rebuilt string. runner: `glob_test.rs` diffs vs old recursive matcher on generated cases.

## §T TASKS

id|status|task|cites
T6|x|ARCHIVED to SPEC-ARCHIVE.md|V1,V2
T15|x|ARCHIVED to SPEC-ARCHIVE.md|V17,`src/lint/pedantic:V37`
T18|x|ARCHIVED to SPEC-ARCHIVE.md|V18,`.:I.flag`
T19|x|ARCHIVED to SPEC-ARCHIVE.md|V18
T20|x|ARCHIVED to SPEC-ARCHIVE.md|V19,V20
T21|x|ARCHIVED to SPEC-ARCHIVE.md|V21,V1
T24|x|ARCHIVED to SPEC-ARCHIVE.md|V24
T31|x|ARCHIVED to SPEC-ARCHIVE.md|V29,V24
T42|x|ARCHIVED to SPEC-ARCHIVE.md|V2
T49|x|ARCHIVED to SPEC-ARCHIVE.md|V45,V19

## §B BUGS

id|date|cause|fix
B5|2026-10-01|level-only rule (`*.bat !crlf=allow`) won the grant ∴ narrowed path to `ascii`; the exemption line made CR a violation. via T39 fixtures|V56
B28|2026-10-02|`explain` printed only the grant winner ∴ `--fidelity emoji` & a level-only `!warn` that `check` applied read as "family none" / "rule none". via release review|V20
B30|2026-10-02|`sets` read `--fidelity` alone ∴ ignored `--rule '* @emoji'`; an undeclared family (`emjoi`) passed as a name. via release review|V29
