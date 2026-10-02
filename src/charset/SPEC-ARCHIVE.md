# SPEC ARCHIVE

Task rows moved out of `SPEC.md` by `mth archive`. An id is never reused
(`V12`), so a citation to an archived row still resolves -- here.

This is a SINK, not a spec. The citations inside these rows point into
`SPEC.md`, so `mth check` on this file reports every one of them as dangling,
correctly and uselessly. The verb that reads it is `mth tasks`.

## §T TASKS

T5|x|charset model: builtin sets, union compose, custom ranges|V3,`.:I.file`
T22|x|builtin map & sets as `U+XXXX` data files via `include_str!`|V22,`.:V13`
T23|x|preset data files per V23, contents from R2 & R4|V22,V23
T25|x|set composition & cycle detection|V25
T32|x|vendor CLDR exemplars; generator → `<code>` & `<code>-aux` preset data files; license notice. done: generator + 30 Latin/Cyrillic/Greek locales (EU official − `en`, + `cy is nb ru sr tr uk`). open: CJK & remaining locales (option A: sets of 1000s)|V30,V22,V59
T46|x|labelled members: parse `<family>:<member>`, resolve against a family, `marks` preset data|V41,`src/rules:V29`
T60|x|1A: generator: ranges, escapes, parent & script aliases; ∀ `cldr-misc-full` locale incl CJK; lazy locale parse; R row w/ size & time delta; flip T32 → x|V61,V59,V30
