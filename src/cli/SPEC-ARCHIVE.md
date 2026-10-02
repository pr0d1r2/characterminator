# SPEC ARCHIVE

Task rows moved out of `SPEC.md` by `mth archive`. An id is never reused
(`V12`), so a citation to an archived row still resolves -- here.

This is a SINK, not a spec. The citations inside these rows point into
`SPEC.md`, so `mth check` on this file reports every one of them as dangling,
correctly and uselessly. The verb that reads it is `mth tasks`.

## §T TASKS

T8|x|`check` verb + fileset via `itok::walk::tracked`|V7,`src/tokens:V9`,`.:I.cmd`
T11|x|`stats` verb w/ `itok` counts now vs after fix|`src/tokens:V10`
T12|x|`explain` & `sets` verbs|`src/rules:V2`,V7
T34|x|`explain --as` renderers (args, lines, prompt) + round-trip property test|V32,`src/rules:V18`
T44|x|`[dir]` arg: accept a dir in argv, expansion per `src/tokens:V43`|V7,`src/tokens:V43`
T50|x|`fix` & `fix --check` verbs: builtin map + discovered `.ctrm-map`, write only on explicit call|V7,`src/fix:V4`,`src/rules:V45`
T55|x|wire the flag twins (`.:I.flag`) into every verb: `--rule` `--map` `--set` `--*-file` `--no-files` `--no-builtin-map` `--no-builtin-sets` `--strict` `--pedantic`; same parser & origin as the dotfiles|`src/rules:V18`,`src/rules:V19`,`src/lint:V36`
