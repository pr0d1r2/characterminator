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
T46|x|labelled members: parse `<family>:<member>`, resolve against a family, `marks` preset data|V41,`src/rules:V29`
