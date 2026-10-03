# SPEC

## §G GOAL

What a char set IS: builtin presets, membership, union, custom & composed sets.

## §F FEDERATION

dir|owns|⊥owns|tokens
locale|CLDR letter presets: vendored locale data, generator, lazy lookup|set algebra, other presets, hazard & emoji data|-

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
sib|src/judge|findings from rules, sets, lints, scan & fix; config assembly
sib|src/render|human & json output, stable json contract
sib|src/cli|arg dispatch, verbs, exit codes, `guard` hook adapter

## §I INTERFACES

- file: `.ctrm-sets` — custom sets, `<name> <member>...`: member = literal char | `U+XXXX` | `U+XXXX-U+YYYY` | set name (≥2 chars, V25) | `<family>:<member>` (V41).
- sets (builtin function presets, V23): `ascii` intrinsic (U+0020–U+007E + `\t` `\n`, `src/rules:V21`) · `caveman` (FORMAT.md symbols `→∴∀∃⊥≠∈∉≤≥§` + measured `⇒∵·×∨∧¬←↔⇔≈∅⊆∪∩↑↓`, R3, + `≡Σ⟺⊇`, R19) · `box` (U+2500–U+257F, U+25A0–U+25FF) · `marks` (labelled members, V41: `✓✗→` @`text`, `✅❌➡` @`emoji`, `⚠` unlabelled) · `math` (`×÷±°²³≤≥≠≈∞µ√∑π`) · `legal` (`©®™`) · `emoji` (single code points: U+1F300–U+1F3FA, U+1F400–U+1FAFF, U+2600–U+27BF; skin tone gap per V42; sequences compress, `src/fix/emoji:V31`) · letters per CLDR locale (`pl`, `de`, `fr`, …, + `<code>-aux`; ∀ 766 `cldr-misc-full` locales + default-content codes, all-ASCII ones = `ascii`; `src/charset/locale:V59`, `src/charset/locale:V61`) · `cr` (`\r`) · `typography` (`—–‘’“”…«»` + NBSP + `−` + low-9 & reversed-9 quotes U+201A U+201B U+201E U+201F; grant ⇒ `src/fix:V26` map leaves them) · coarse blocks `latin1` (U+0080–U+00FF), `latin-ext` (U+0100–U+017F), `cyrillic`, `greek`, `arabic` · scripts w/ joiners `persian`, `hindi` (`src/lint/hazard:V57`) · `hazard` (`src/lint/hazard:V34`; ∈ `any` like any set — the FORBID level is what fires, ⊥ the set) = union of `hazard-bidi` `hazard-tag` `hazard-bom` `hazard-control` `hazard-invisible`, 1 per class (`src/lint/hazard:V49`), each a nameable set · `any`. custom sets via `.ctrm-sets`.

## §R RESEARCH

id|topic|finding|src
R1|Rust corpus need|9 Rust repos (sibling dirs w/ `Cargo.toml`), 702 git-tracked text files: 65.1% pure ASCII (+ `\t` `\n`); `.rs` 77.7%, `.md` 15.0%. non-ASCII files (245): 44.5% need 1 extra char, 71.0% ≤3, 93.5% ≤10|scan 2026-09-12, repos anonymous
R2|Rust preset coverage|of 245 non-ASCII files: 12.7% fully fixed by typography map alone; + `ascii`+1 preset → 90.2%; +2 → 95.1%. `caveman` = 180 of the 1-preset fits|same scan
R3|caveman usage|Rust: 180 fit extended `caveman`, 123 fit FORMAT.md list ∴ misses 32%. fleet: 833 vs 521 ∴ 37%. extras, fleet files containing: `⇒` 222, `·` 168, `∵` 77|R1 scan + R4 scan
R4|fleet extension|227 repos, 73,114 text files: 95.1% pure ASCII. of 3,604 non-ASCII: typography map alone 45.1%; +1 preset → 84.9%; +2 → 89.2%. excl `.nix` (60% of files, 1 repo = 89% of its non-ASCII): 84.1% / 90.0%|fleet scan 2026-09-12, snapshots excluded, repos anonymous
R5|uncovered & pairs|Rust: `Σ` `≡` `⟺` `≪` `⊇` `⋃`, superscripts, CJK (wenyan-style caveman). fleet: `©` `®` `™`, Cyrillic, Arabic, `−` U+2212. top pair both corpora: `caveman`+`box` (Rust 8, fleet 42)|R1 scan + R4 scan
R16|perf hotspots (V86, `src/rules:V87`, `src/charset/locale:V88`)|`contains`: 10 MB text of `zh` letters under `* zh` 0.970 → 0.084 s (`* any` 0.069); 10 MB ASCII under `* zh` 0.188 → 0.096. glob: 10k paths × 400 `dX/**/*qN*.txt` rules 0.862 → 0.415 s; `*a*a*a*a*a*a*b` vs 80×`a` 6.358 → 0.006 s. locale index: `adopt_all` release 35.6 → 6.3 ms, debug 188 → 51 ms; `ctrm sets` 0.052 → 0.023 s, output byte-identical|2026-10-03, macOS arm64, release build, min cpu of 5–7 runs, generated fixtures, loop timer
R19|wave 1 re-baseline: preset fit (root T28 gate)|13 sibling Rust repos (public `itok` `microlith` `pklith` `rekall` `sherd` `xenolith` + 7 private), 1,670 files, 638 non-ASCII. map alone 13.0%; +1 preset → 87.1% (`caveman` 469 of 473); +2 → 89.7%; 10.3% fit no 3 ∴ V23 shape holds, but ∀ preset misses R5's `≡` (6 repos) `Σ` (5) `⟺` `⊇` (4 each). rest: Polish letters (locale sets, V23) & CJK. 0 hazards; map saves ~0 tok (3,500,946 → 3,500,249, estimate)|2026-10-03, `ctrm` @`bd38f2d`, `--no-files` `* ascii`, private repos anonymous

## §V INVARIANTS

V3: sets compose by union only (`ascii+latin-ext`). ⊥ subtraction. effective set = union of winning rule's sets.
V22: builtin map & sets ship as data files in same grammar as user files, compiled in via `include_str!`, written in `U+XXXX` form only ∴ ASCII, `.:V13` holds w/ ⊥ grant.
V23: builtin sets = small function presets, sized from R2 & R4: typical file = `ascii` + 1 preset after map. coarse blocks (`latin1`, `latin-ext`, scripts) only for multi-language data (locales), ⊥ recommended for code | docs.
V25: set member ? names another set (name ≥2 chars; 1 char = literal) ∴ user presets = compositions (`spec caveman box marks`). cycle → error naming cycle, exit 2.
V41: labelled set member `<family>:<member>`, notation ≡ `src/fix:V28`. unlabelled granted ∀ fidelity; labelled granted ⟺ label = resolved family (`src/rules:V29`). ⊥ family-tree walk here ∵ tree lives in the MAP (`src/fix:V27`) ∴ walking it = this node depending on `src/fix`. family = opaque NAME.
V42: preset range written so ⊥ code point another node WITHHOLDS falls inside (`src/fix/emoji:V31`: skin tone, VS, ZWJ, keycap, tag). V3 ⊥ subtraction ∴ gap ! be IN the range. §I = summary, data file SHIPS; test asserts ∀ exclusion.
V77: ∀ generator here & in `locale` (`locale/cldr-letters.sh`, `emoji-sequences.sh`): `fetch <dir>` creates `<dir>` (`mkdir -p`) before writing; failed download → curl's message (`-sS`), ⊥ bare exit code. ∀ tool a step needs (`jq`, `curl`, `sha256sum`, `xargs`) ∈ dev shell. test reads both scripts.
V86: `CharSet::contains` = binary search (`partition_point`) over canonical `ranges` (sorted, merged, `range::normalize`); ASCII point → walk of leading ranges only ∴ O(log n), ⊥ linear scan (`zh` = 1,716 ranges). `ranges` stays `pub` (`render`, `lint` read it; other nodes' tests build literals) ∴ canonical form guaranteed by `new`/`union`; hand-built set ! be canonical. runner: `set_test.rs` diffs search vs linear walk, generated + ∀ shipped set.
V97: ∀ GENERATED data file reproducible by a NAMED command: `locale/locales.ctrm-sets` ← `locale/cldr-letters.sh generate <dir>` · `emoji-sequences.txt` ← `emoji-sequences.sh data <dir>` (each after its `fetch <dir>`, online ∴ ⊥ gated: gate offline, `.:C`) · `src/fix/emoji/emoji-seq.ctrm-map` ← `emoji-sequences.sh map`, OFFLINE (reads vendored `emoji-sequences.txt`) ∴ hk step `emoji-seq-map` regenerates & diffs ∀ run. EXCEPTION, stated ⊥ hidden: `hazard.ctrm-sets` has ⊥ committed generator; members = the awk in its header over 3 UCD files (sha256 recorded, files ⊥ vendored), `\t` `\n` `\r` cut from Cc by hand ∴ reproducible by hand only, til T62.
V100: `U+XXXX`(`+U+XXXX`)* decoded by ONE fn, `code_points` (hex digits only, ⊥ sign); set members, map tokens (`src/fix`) & the emoji list (`src/lint/hazard:V63`) all call it ∴ ⊥ 2 grammars of one spelling. runner: `every_vendored_emoji_sequence_decodes` & `src/fix` `codepoint` tests.

## §T TASKS

id|status|task|cites
T5|x|ARCHIVED to SPEC-ARCHIVE.md|V3,`.:I.file`
T22|x|ARCHIVED to SPEC-ARCHIVE.md|V22,`.:V13`
T23|x|ARCHIVED to SPEC-ARCHIVE.md|V22,V23
T25|x|ARCHIVED to SPEC-ARCHIVE.md|V25
T46|x|ARCHIVED to SPEC-ARCHIVE.md|V41,`src/rules:V29`
T62|.|commit `hazard-sets.sh` (`fetch <dir>` + `generate <dir>`, V77 shape) emitting `hazard.ctrm-sets` from the header's awk & hashes ∴ V97's exception closes|V97,V77,`src/lint/hazard:V34`

## §B BUGS

id|date|cause|fix
B2|2026-09-20|§I stated `emoji` unbroken U+1F300–U+1FAFF; skin tone U+1F3FB–U+1F3FF sit INSIDE & `src/fix/emoji:V31` withholds them ∴ data file from §I granted them|V42
B36|2026-10-02|`cldr-letters.sh fetch <newdir>` ⊥ `mkdir -p` & `curl -s` ∴ exit 23, ⊥ message; `curl` ∉ dev shell|V77
