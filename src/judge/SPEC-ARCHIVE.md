# SPEC ARCHIVE

Task rows moved out of `SPEC.md` by `mth archive`. An id is never reused
(`V12`), so a citation to an archived row still resolves -- here.

This is a SINK, not a spec. The citations inside these rows point into
`SPEC.md`, so `mth check` on this file reports every one of them as dangling,
correctly and uselessly. The verb that reads it is `mth tasks`.

## §T TASKS

T65|x|extract the judge from `src/cli` (`checker`, config assembly) ∴ `guard` & `explain` import `src/judge`, ⊥ their parent's privates|V99,`src:V39`
T66|x|ONE walk → read → decode (`files`) for `check`, `fix` & `stats`; guard's hazards-only path → `unruled`; `scan_bytes` returns its text ∴ ⊥ 2nd decode|V71,`src/scan:V8`
