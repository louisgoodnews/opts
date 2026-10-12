# Settings Registry

Milestone [M-0063](../roadmap/phase-9/M-0063.md). This page settles the API that
[expandability §5](../roadmap/phase-9/expandability.md#5-settings-registry-declarative) sketches.
Code: `crates/opts-game/src/settings_registry/`, the page in `settings_screen/`.

**Goal:** a feature adds a setting with *one descriptor*; the settings page, persistence,
validation, *Reset*, search, the control protocol (`settings.*`) and `docs/src/reference/settings.md`
follow without further edits. A test fixture proves it.

## 1. The descriptor

```rust
pub struct SettingDescriptor {
    pub key: &'static str,            // "graphics.vsync"; legacy keys keep their stored name
    pub kind: SettingKind,
    pub default: &'static str,        // the stored text of the default value
    pub category: Category,           // Graphics, Audio, Controls, Interface, Gameplay, Content, Advanced
    pub order: u16,                   // inside the category
    pub label_key: &'static str,      // message keys (en + de); the description key is label_key + "-desc"
    pub description_key: &'static str,
    pub keywords: &'static [&'static str],  // extra search words (English), German words live in the message
    pub apply: Apply,                 // Live | OnRestart | WithRevert(secs)
    pub available: Available,         // Always | Capability(name) | NotYet(&'static NotYetPlace)
    pub since: &'static str,          // "0.1"
    pub migrate_from: &'static [&'static str], // older key names, read when `key` is absent
}

pub enum SettingKind {
    Bool,
    Enum(&'static [EnumOption]),      // EnumOption { value: &str (stored), label_key: &str }
    Int { min: i64, max: i64, step: i64 },
    Float { min: f64, max: f64, step: f64 },
    Text,                             // free text (validated by `max_len`)
    Path,                             // a file or folder; empty = none
    Time,                             // "HH:MM"
    Binding,                          // an input action binding (Controls page)
    Action,                           // a button, no stored value (Rescan now, Clear cache)
}
```

`Available::NotYet(place)` references a registered `opts_gui::not_yet::NotYetPlace`; the row is
shown with the red "Feature not available yet!" and a disabled control, and the place appears in
the generated list `reference/not-available-yet.md`. `Capability(name)` consults the capability
resource (`SettingsCapabilities`, filled by the features: audio device, gamepad, ...); an absent
capability shows the row disabled with a reason.

## 2. Registration

```rust
app.register_setting(SettingDescriptor { .. });      // trait SettingsAppExt
app.register_settings(&[..]);                         // a slice
```

`SettingsRegistryPlugin` (added by `OptsGamePlugin`) owns the `SettingsRegistry` resource.
`opts-game` registers the built-in settings (`settings_registry/builtin.rs`); other crates register
theirs in their own plugin. Registering a key twice, or a key that is another's `migrate_from`, is
an error caught by a test.

## 3. Storage

`settings.db` keeps its table `app_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)`; the
descriptor is the schema, values are **strings in the form `SettingKind::to_text` writes** (`true`,
the enum's stored value, a decimal, `HH:MM`, a path). Existing keys keep their exact stored form
(for example `theme_mode = dark`, `ui_scale = 100`).

- `GameSettings` keeps its typed fields for the existing consumers. Keys of those fields are
  *legacy-backed*: the registry reads and writes them through `GameSettings::raw` / `set_raw`,
  which use the same parser as loading, so a value can never differ between the field and the key.
- Every other key lives in `GameSettings::extra: BTreeMap<String, String>`, loaded from and saved
  to the same table. Unknown keys (from a newer version or another feature) are kept, not dropped.
- **Baseline diff saving is unchanged:** `save()` writes only the keys whose text differs from
  what this process loaded or saved last, so several instances do not overwrite each other.
- Absent keys give the default; an unreadable or out-of-range stored value gives the default too
  (validation on load), and the file is not rewritten until the user changes the key.
- **Migration:** when `key` is absent and a `migrate_from` key is present, the old value is
  validated and used (and written under the new key on the next change). The old row stays, so
  an older build keeps working. The M-0061 migration `is_dark_theme` to `theme_mode` is kept as
  it is (legacy flag, read-only).

## 4. Access and notification

```rust
registry.get(&settings, "graphics.vsync") -> SettingValue     // never fails: default if absent
settings.get_bool("graphics.vsync") / get_int / get_float / get_str     // typed, default aware
registry.set(&mut settings, key, text) -> Result<SettingChange, SettingError>  // validates
registry.reset(&mut settings, key) -> Result<SettingChange, SettingError>
```

Systems change settings through `SettingsApi` (a `SystemParam` bundling the registry, the
`GameSettings`, the message writer and the save): it validates, stores, saves, and writes the
message `SettingChanged { key, old, new, source }`. Systems that apply a value listen for the
message (`apply` systems: window mode, language, logging level, ...); legacy consumers keep
working through change detection on `GameSettings`. The control protocol and the page use the
same API, so every route gives the same validation and the same effect.

`source` is `Page`, `Protocol`, `Reset`, `Revert` or `Load`.

## 5. Apply policies

| Policy | Behaviour |
| :--- | :--- |
| `Live` | applied at once and saved |
| `OnRestart` | saved at once; the row shows "Takes effect after a restart" |
| `WithRevert(secs)` | applied at once, then a modal "Keep these display settings?" counts down `secs` seconds (15 for display settings); *Keep* saves, *Revert* or the timeout restores the old value. Nobody confirming means revert. The value is persisted only after *Keep*. The apply and the restore go through the `DisplayBackend` trait, so a test uses a fake display. |

## 6. The page

`spawn_settings_page(commands, parent, category)`: a left list of categories (only those with at
least one visible row; Audio shows its `NotYet` rows), a right pane of rows (label, control by kind,
one-line description, *Reset*), a search field above both. Search matches label, description,
keywords and key across categories (case-insensitive words; every word must match) in the active
language; with a query the right pane lists the matches grouped by category. Deep link
`screen:settings/<category>`. The control by kind: Bool: switch; Enum up to 4 options: radio group,
more: combobox; Int/Float with a short range: stepper buttons plus value (a slider where the
component library has one); Time: two-digit text field; Path: text, *Choose...* through the
folder-chooser trait; Action: button. Every element has a `UiId` of the form
`settings/<category>/<key>/<role>`.

## 7. Protocol and reference

`settings.list [category]`, `settings.get <key>`, `settings.set <key> <value>`, `settings.reset <key>`
(see [control protocol](control-protocol.md)). `docs/src/reference/settings.md` is generated from
the registry by the test `reference_page_is_up_to_date`
(`UPDATE_SETTINGS_DOC=1 cargo test -p opts-game settings_registry`), checked like the roadmap copy.

## 8. Decisions

- **Strings as the stored form**, descriptor as schema: no schema migration of the database, old files load unchanged.
- **Legacy fields stay** (no flag-day): the registry bridges them through the loader's own parser.
- **`extra` map** instead of a typed field per setting: a new setting needs no struct change.
- **Revert is a page concern with a backend trait**, not part of persistence.
