# SPEC ARCHIVE

Task rows moved out of `SPEC.md` by `mth archive`. An id is never reused
(`V12`), so a citation to an archived row still resolves -- here.

This is a SINK, not a spec. The citations inside these rows point into
`SPEC.md`, so `mth check` on this file reports every one of them as dangling,
correctly and uselessly. The verb that reads it is `mth tasks`.

## §T TASKS

T6|x|`.ctrm` parse & rule resolution, last match wins|V1,V2
T18|x|one line parser per kind (rules, map, sets); flag twins feed same parser|V18,`.:I.flag`
T19|x|property test: file ≡ `--no-files` + flag sequence, ∀ kinds|V18
T20|x|config assembly: precedence chain, origin per entry, `explain` prints origin|V19,V20
T21|x|zero-file mode; `ascii` as intrinsic constant|V21,V1
T24|x|rule resolution: `ascii` implicit base|V24
T31|x|fidelity: `@<family>` in rules, `--fidelity`, family-aware presets|V29,V24
T42|x|own glob matcher, zero-dep: `?` · `*` (⊥ cross `/`) · `**` · gitignore surface (⊥ `/` ⇒ `**/` prefix · leading `/` anchors · trailing `/` ⇒ dir). feeds `resolve`'s matcher parameter|V2
T49|x|discover `.ctrm-sets` & `.ctrm-map` at the run root, into the V19 chain|V45,V19
