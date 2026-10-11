# Projects, Workflow and Packaging

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification |
| **Relates to** | [Hot reload](../architecture/hot-reload.md), [control protocol](../architecture/control-protocol.md), roadmap M-0031…M-0033 (modding) |

## 1. Projects and the overlay

An editor **project** is a folder with a manifest `project.opts.toml` and an *overlay* that mirrors the OMSI layout:

```
MyMap.opts/
  project.opts.toml            name, kind (map | vehicle | pack), base install, dependencies, author, licence
  overlay/                     ← the only place the editor writes
    maps/MyMap/…               exact OMSI relative paths
    Vehicles/MyBus/…
    Splines/…  Sceneryobjects/…  Texture/…
  .opts-editor/                journal, backups, thumbnails, caches, workspace presets (not exported)
  README.md  CREDITS.md        authoring notes, credits (exported)
```

* **Reading** resolves a path in the overlay first, then in the base installation (`OmsiInstall` with an overlay root, the catalogue extended with the overlay). Editing an installed file *copies it into the overlay on first save* (copy-on-write) and from then on shadows the original. The original is never touched (P1).
* **Kinds.** *Map* projects own a map folder and the content built for it; *vehicle* projects own a vehicle folder; *pack* projects own loose content (objects, profiles, textures). A project can depend on other packs (listed in the manifest with name, version, source); dependencies are resolved from the installation or from sibling projects.
* **Starting points.** *New empty*, *New from installed content* (copy a map/vehicle as the basis for a variant — with the clean-room caveat for protected files, [README §6](README.md#6-protected-content-and-clean-room-rules)), *Edit installed in place* (a project with an empty overlay whose first save creates overlay copies).
* **Multiple installs.** The base installation is chosen per project (the path remembered by the startup dialog, [application startup](../architecture/application-startup.md)); a project records which content it needs, so opening it on a machine with a different install reports precisely what is missing.

## 2. Working loop

1. **Edit** in the viewport/inspector/source view. Every change is a command; validation runs live.
2. **Save** (`Ctrl+S`): writes the overlay files atomically; the running preview and any *Play from here* session pick them up through the hot-reload pipeline (debounce 250 ms by default).
3. **Play from here** (`F5`): launches the game with `--project <dir> --map <name> --entry-at <x,y,z> [--vehicle …] [--time …]`. The editor stays open; closing the game returns the camera to where the player was (the game reports its position through the control protocol).
4. **Review** the diff (`File → Review changes`): per file and per element, original vs overlay, with *Revert element/file*. For binary files a summary (terrain: tiles and heights changed; meshes: new/changed).
5. **Package** when ready.

## 3. Installing and exporting

| Action | What it does | Safety |
| :--- | :--- | :--- |
| **Export package** | Builds a folder or `.zip` with the overlay files (OMSI-relative layout), `package.opts.toml` (name, version, author, licence, dependencies, file list with hashes) and `CREDITS.md`. Unzipping into the OMSI root installs it the way OMSI add-ons are installed. | Blocked by validation *Errors*; third-party files require the redistribution confirmation ([PKG-001](validation.md)); never overwrites files of the base installation (PKG-003). |
| **Install into OMSI** | Copies the package into a chosen OMSI folder (or into a *test installation* the user designates) and optionally rebuilds the OPTS catalogue. | **Explicit action, off by default.** Shows every file to be written and every file it would overwrite; records an *install receipt* (`Installed/<name>.opts-receipt`) with hashes and the previous content of overwritten files so *Uninstall* restores them. The editor refuses to run against a path that does not look like an OMSI root. |
| **Open in OMSI** | Starts OMSI (user-configured command) on the installed package for the acceptance check. | Informational; OPTS does not drive OMSI. |
| **Export for review** | Zips only the *changed* files with an HTML diff report. | For sharing work in progress. |

## 4. Versioning, collaboration and merging

* Text files keep a stable element order and line-oriented diffs, so projects work in Git: `.opts-editor/` is ignored, large binaries (`.dds`, `.o3d`, `.wav`) are suited for LFS (a generated `.gitattributes` suggestion).
* **Semantic merge.** `opts merge base ours theirs` merges tiles by element ID: objects/splines added or edited on both sides merge; the same element edited differently conflicts and is resolved in a three-way view; ID collisions between independently added elements are resolved by renumbering (ID allocation can reserve ranges per author: `[NextIDCode]` blocks of 10,000 per contributor, recorded in the project manifest).
* **Locks.** The editor takes an advisory lock file per document when opened for writing (`.opts-editor/locks/`) and warns when two editors work on one project; hot reload picks up external edits.
* **Change log.** Optional per-project `CHANGES.md` generated from the journal (what was added/changed since the last export).

## 5. Credits, licences and third-party content

* The project manifest holds author, licence (SPDX id or free text), and credits. Importing a file records its **origin** (installed pack and author from its `.sco`/`.cfg` comments, URL, or "own work"). The *Dependencies* view lists everything the project references from outside, grouped by pack, with the author when known.
* Exports embed only files marked *own work* or confirmed redistributable; everything else stays a dependency. The confirmation text is stored in the package manifest.
* Protected (scrambled) content is always a dependency and never embedded.

## 6. Control protocol and automation

The editor exposes its operations through `editor.*` methods so that scripts, AI agents and tests drive it like a user ([ADR-0006](../adr/0006-control-protocol.md)):

| Method | Effect |
| :--- | :--- |
| `editor.open project=…` / `editor.close` | open/close a project |
| `editor.documents` / `editor.elements document=… [kind=…]` | list documents and elements with ids |
| `editor.select ids=[…]` / `editor.selection` | selection |
| `editor.run command=… args={…}` | run a named command (the same registry the menus use) |
| `editor.set element=… field=… value=…` | set a property through the schema |
| `editor.undo` / `editor.redo` / `editor.save [all]` | history and persistence |
| `editor.validate [scope=…]` | run validation, return findings |
| `editor.play from=x,y,z` | start *Play from here* |
| `editor.export kind=package` | build a package |
| `screenshot`, `camera.*` | existing methods work in the editor viewport |

Everything the editor does is therefore testable headlessly: a script opens a fixture project, places objects, saves, runs the game loader and compares outputs.

## 7. Settings

Stored in the existing settings database: workspace presets, key bindings, grid/snap defaults, autosave interval (default 2 min to the journal), backup count, which OMSI version the user targets (hints for `_2` syntax), language, units shown (metric only), recent projects, and the *Install into OMSI* test target.
