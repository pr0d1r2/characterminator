# SPEC

## §G GOAL

What a char set IS: builtin presets, membership, union, custom & composed sets.

## §N NAV

rel|path|lens
up|.|-
up|src|the tool: charset presets, rule resolution, scan, fix, lint levels, token facade, render, CLI
self|src/charset|builtin presets, set membership, unions, custom sets, CLDR letters, preset data
sib|src/rules|config files & flags, precedence, origin, rule resolution, fidelity choice
sib|src/scan|read text: positions, UTF-8 validity, binary skip
sib|src/fix|rewriting: map, families, equivalence classes, typography, emoji compression
sib|src/lint|lint names, groups, levels, hazard, pedantic
sib|src/tokens|`itok` facade: counts w/ method label, git-tracked fileset
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- file: `.ctrm-sets` — custom sets, `<name> <member>...`: member = literal char | `U+XXXX` | `U+XXXX-U+YYYY` | set name (≥2 chars, V25) | `<family>:<member>` (V41).
- sets (builtin function presets, V23): `ascii` intrinsic (U+0020–U+007E + `\t` `\n`, `src/rules:V21`) · `caveman` (FORMAT.md symbols `→∴∀∃⊥≠∈∉≤≥§` + measured `⇒∵·×∨∧¬←↔⇔≈∅⊆∪∩↑↓`, R3) · `box` (U+2500–U+257F, U+25A0–U+25FF) · `marks` (labelled members, V41: `✓✗→` @`text`, `✅❌➡` @`emoji`, `⚠` unlabelled) · `math` (`×÷±°²³≤≥≠≈∞µ√∑π`) · `legal` (`©®™`) · `emoji` (single code points: U+1F300–U+1F3FA, U+1F400–U+1FAFF, U+2600–U+27BF; skin tone gap per V42; sequences compress, `src/fix:V31`) · letters ∀ CLDR locale (`pl`, `de`, `fr`, `ja`, …, V30) · `cr` (`\r`) · `typography` (`—–‘’“”…«»` + NBSP + `−`; grant ⇒ `src/fix:V26` map leaves them) · coarse blocks `latin1` (U+0080–U+00FF), `latin-ext` (U+0100–U+017F), `cyrillic`, `greek`, `arabic` · scripts w/ joiners `persian`, `hindi` (`src/lint:V57`) · `hazard` (`src/lint:V34`; ∈ `any` like any set — the FORBID level is what fires, ⊥ the set) · `any`. custom sets via `.ctrm-sets`.

## §R RESEARCH

id|topic|finding|src
R1|Rust corpus need|9 Rust repos (sibling dirs w/ `Cargo.toml`), 702 git-tracked text files: 65.1% pure ASCII (+ `\t` `\n`); `.rs` 77.7%, `.md` 15.0%. non-ASCII files (245): 44.5% need 1 extra char, 71.0% ≤3, 93.5% ≤10|scan 2026-09-12, repos anonymous
R2|Rust preset coverage|of 245 non-ASCII files: 12.7% fully fixed by typography map alone; + `ascii`+1 preset → 90.2%; +2 → 95.1%. `caveman` = 180 of the 1-preset fits|same scan
R3|caveman usage|Rust: 180 fit extended `caveman`, 123 fit FORMAT.md list ∴ misses 32%. fleet: 833 vs 521 ∴ 37%. extras, fleet files containing: `⇒` 222, `·` 168, `∵` 77|R1 scan + R4 scan
R4|fleet extension|227 repos, 73,114 text files: 95.1% pure ASCII. of 3,604 non-ASCII: typography map alone 45.1%; +1 preset → 84.9%; +2 → 89.2%. excl `.nix` (60% of files, 1 repo = 89% of its non-ASCII): 84.1% / 90.0%|fleet scan 2026-09-12, snapshots excluded, repos anonymous
R5|uncovered & pairs|Rust: `Σ` `≡` `⟺` `≪` `⊇` `⋃`, superscripts, CJK (wenyan-style caveman). fleet: `©` `®` `™`, Cyrillic, Arabic, `−` U+2212. top pair both corpora: `caveman`+`box` (Rust 8, fleet 42)|R1 scan + R4 scan
R6|CLDR letters|766 locales in `cldr-misc-full`. `exemplarCharacters` = letters in normal use (pl: 32, 9 non-ASCII); `auxiliary` = loan letters; `punctuation` incl. locale quotes (pl `„”`). Unicode License v3|github.com/unicode-org/cldr-json

## §V INVARIANTS

V3: sets compose by union only (`ascii+latin-ext`). ⊥ subtraction. effective set = union of winning rule's sets.
V22: builtin map & sets ship as data files in same grammar as user files, compiled in via `include_str!`, written in `U+XXXX` form only ∴ ASCII, `.:V13` holds w/ ⊥ grant.
V23: builtin sets = small function presets, sized from R2 & R4: typical file = `ascii` + 1 preset after map. coarse blocks (`latin1`, `latin-ext`, scripts) only for multi-language data (locales), ⊥ recommended for code | docs.
V25: set member ? names another set (name ≥2 chars; 1 char = literal) ∴ user presets = compositions (`spec caveman box marks`). cycle → error naming cycle, exit 2.
V30: language letter presets ∀ CLDR locale, named by locale code (`pl`, `pt-BR`): main `exemplarCharacters` − ASCII, + uppercase forms. data vendored (Unicode License v3, R6), generated into `U+XXXX` data file (V22), ⊥ fetched at runtime. `auxiliary` loan letters ship as `<code>-aux` from same vendoring ∴ 1 generator run, 2 presets.

V41: labelled set member `<family>:<member>`, notation ≡ `src/fix:V28`. unlabelled granted ∀ fidelity; labelled granted ⟺ label = resolved family (`src/rules:V29`). ⊥ family-tree walk here ∵ tree lives in the MAP (`src/fix:V27`) ∴ walking it = this node depending on `src/fix`. family = opaque NAME.
V42: preset range written so ⊥ code point another node WITHHOLDS falls inside (`src/fix:V31`: skin tone, VS, ZWJ, keycap, tag). V3 ⊥ subtraction ∴ gap ! be IN the range. §I = summary, data file SHIPS; test asserts ∀ exclusion.

## §T TASKS

id|status|task|cites
T5|x|ARCHIVED to SPEC-ARCHIVE.md|V3,`.:I.file`
T22|x|ARCHIVED to SPEC-ARCHIVE.md|V22,`.:V13`
T23|x|preset data files per V23, contents from R2 & R4|V22,V23
T25|x|ARCHIVED to SPEC-ARCHIVE.md|V25
T32|.|vendor CLDR exemplars; generator → `<code>` & `<code>-aux` preset data files; license notice|V30,V22
T46|x|labelled members: parse `<family>:<member>`, resolve against a family, `marks` preset data|V41,`src/rules:V29`

## §B BUGS

id|date|cause|fix
B2|2026-09-20|§I stated `emoji` unbroken U+1F300–U+1FAFF; skin tone U+1F3FB–U+1F3FF sit INSIDE & `src/fix:V31` withholds them ∴ data file from §I granted them|V42
