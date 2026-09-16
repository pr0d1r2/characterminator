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
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- file: `.characterminator` ? — line-based, `<glob|path> <set>[+<set>...] [@<family>] [!<level>] [!<lint|group>=<level>]`, `#` comment. zero-dep parse (`.context-limits` shape).

## §V INVARIANTS

V1: path w/ no matching rule → `ascii`. strict default; extended set = explicit grant. ≠ `itok`'s opt-in `.context-limits`: here an unguarded char IS the cost.
V2: rule resolution: later matching line wins (gitignore semantics); per-type glob & per-file path share one grammar ∴ per-file line placed after per-type line overrides it. `explain` ! print winner.
V17: locale separation: extended sets granted to data paths (`locales/**`, `*.po`, `config/locales/*.yml`); code stays `ascii`. non-ASCII string literal in code file → hint: move to locale file ?.
V18: ∀ data-file line kind → flag twin: `--rule` ≡ rules line, `--map` ≡ map line, `--set` ≡ sets line. flag value = exactly 1 line, same parser ∴ ∀ file F: `--no-files` + 1 flag per line of F ≡ F, property-tested.
V19: precedence, low → high: builtin → discovered dotfiles → `--*-file` (argv order) → inline flags (argv order). later wins: rule per V2, map entry per char, set per name.
V20: ∀ effective rule, map entry, set → origin (`<file>:<line>` | `argv[<n>]` | `builtin:<line>`). `explain` ! print it. export via `explain --as` (`src/cli:V32`).
V21: zero-file run: `--no-files --no-builtin-map --no-builtin-sets` → config from argv only. `ascii` intrinsic (code, ⊥ data) ∴ V1 holds w/ ⊥ file.
V24: `ascii` = implicit base ∀ rule: effective set = `ascii` ∪ named sets ∴ `*.md caveman` ≡ `*.md ascii+caveman`. explicit `ascii+` stays legal.
V29: fidelity = family name, default `text`; per rule `@<family>` suffix (`docs/** marks @emoji`); last matching rule naming one wins. `--fidelity <f>` ≡ `--rule '* @<f>'` (V19 order). presets w/ classes grant only resolved family's members ∴ other families compress into it; `src/fix:V6` holds. mix → grant variants explicitly.

## §T TASKS

id|status|task|cites
T6|.|`.characterminator` parse & rule resolution, last match wins|V1,V2
T15|.|locale hint on non-ASCII literal in code file ?|V17
T18|.|one line parser per kind (rules, map, sets); flag twins feed same parser|V18,`.:I.flag`
T19|.|property test: file ≡ `--no-files` + flag sequence, ∀ kinds|V18
T20|.|config assembly: precedence chain, origin per entry, `explain` prints origin|V19,V20
T21|.|zero-file mode; `ascii` as intrinsic constant|V21,V1
T24|.|rule resolution: `ascii` implicit base|V24
T31|.|fidelity: `@<family>` in rules, `--fidelity`, family-aware presets|V29,V24
