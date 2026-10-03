# SPEC ARCHIVE

Task rows moved out of `SPEC.md` by `mth archive`. An id is never reused
(`V12`), so a citation to an archived row still resolves -- here.

This is a SINK, not a spec. The citations inside these rows point into
`SPEC.md`, so `mth check` on this file reports every one of them as dangling,
correctly and uselessly. The verb that reads it is `mth tasks`.

## §T TASKS

T32|x|vendor CLDR exemplars; generator → `<code>` & `<code>-aux` preset data files; license notice. done: generator + 30 Latin/Cyrillic/Greek locales (EU official − `en`, + `cy is nb ru sr tr uk`). open: CJK & remaining locales (option A: sets of 1000s)|V30,`src/charset:V22`,V59
T60|x|1A: generator: ranges, escapes, parent & script aliases; ∀ `cldr-misc-full` locale incl CJK; lazy locale parse; R row w/ size & time delta; flip T32 → x|V61,V59,V30
