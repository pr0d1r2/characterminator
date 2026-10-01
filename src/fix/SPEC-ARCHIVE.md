# SPEC ARCHIVE

Task rows moved out of `SPEC.md` by `mth archive`. An id is never reused
(`V12`), so a citation to an archived row still resolves -- here.

This is a SINK, not a spec. The citations inside these rows point into
`SPEC.md`, so `mth check` on this file reports every one of them as dangling,
correctly and uselessly. The verb that reads it is `mth tasks`.

## §T TASKS

T10|x|transliteration map + `fix` & `fix --check`; property tests: idempotency, untouched bytes|V4,V5,V6
T26|x|builtin map: typography defaults|V26,`src/charset:V22`
T29|x|family lines: parse, tree validation, fallback path|V27
T30|x|class lines: parse, family-labelled members, resolution along fallback path|V28,V27
