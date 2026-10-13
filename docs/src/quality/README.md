# Content Quality

| Page | Content |
| :--- | :--- |
| [Field report 2026-10-13](field-report-2026-10-13.md) | Nineteen items reported by the project lead after hands-on testing, each traced to a milestone (R-01 … R-19). |
| [Content issue registry](content-issue-registry.md) | Splines, scenery objects, meshes, vehicle scripts, vehicles and textures that log a warning or an error, with counts, context and status. Generated; the full list is the [CSV](content-issue-registry.csv). |

## How the registry is built

```
# 1. Run the game for a minute on every map (fresh logs of the current build):
for m in maps/*; do opts-game --map-path "$m" --vehicle-path <any bus> --autostart; done
# 2. Run every vehicle's scripts offline (1,218 scripted vehicles, 60 frames each):
CORPUS_REPORT=corpus.tsv cargo run --release -p opts-sim --example vehicle_scripts -- <OMSI dir>
# 3. Build the registry:
python3 scripts/content_issues.py --since <time of the sweep, UTC> --corpus corpus.tsv
```

`--since` drops older log lines, so entries of an earlier build do not pollute the list. Commit the regenerated files; a diff shows what was fixed (entry gone) and what regressed (new entry). The *Status* column of the markdown stays `open` for every entry the generator writes; resolved entries disappear instead of being closed by hand.

Reading the entries: *Subject* is the content file (paths relative to the OMSI folder), *Message* the log message with numbers replaced by `N`, *Context* the vehicles or maps seen in. Entries of an `(script engine)` subject are host callbacks the scripts call that OPTS does not implement yet ([M-0057](../roadmap/phase-4/M-0057.md), [M-0058](../roadmap/phase-4/M-0058.md)); `stack underflow` entries would be VM bugs (there are none since 2026-10-12).
