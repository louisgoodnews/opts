//! Saves overlay and save or load toast keys, and load error texts.

use opts_shell_keys_help::keys;

keys! {
    /// Saves overlay: title when saving.
    SAVES_TITLE_SAVE = "saves-title-save",
    /// Saves overlay: title when loading.
    SAVES_TITLE_LOAD = "saves-title-load",
    /// Saves overlay: what a save holds and what it does not.
    SAVES_NOTE = "saves-note",
    /// Saves overlay: name field label.
    SAVES_NAME_LABEL = "saves-name-label",
    /// Saves overlay: name field placeholder.
    SAVES_NAME_PLACEHOLDER = "saves-name-placeholder",
    /// Saves overlay: the name is empty.
    SAVES_NAME_EMPTY = "saves-name-empty",
    /// Saves overlay: save button.
    SAVES_SAVE = "saves-save",
    /// Saves overlay: apply a new name.
    SAVES_RENAME_APPLY = "saves-rename-apply",
    /// Saves overlay: no saves yet.
    SAVES_EMPTY = "saves-empty",
    /// Saves overlay: second line of a row (variables `map`, `vehicles`, `played`, `date`).
    SAVES_ROW_DETAIL = "saves-row-detail",
    /// Saves overlay: a save without a vehicle.
    SAVES_NO_VEHICLE = "saves-no-vehicle",
    /// Saves overlay: the main file was damaged, the backup is shown.
    SAVES_FROM_BACKUP = "saves-from-backup",
    /// Saves overlay: a file that cannot be read (variable `reason`).
    SAVES_UNREADABLE = "saves-unreadable",
    /// Saves overlay: the map is not installed (variable `name`).
    SAVES_WARN_MISSING_MAP = "saves-warn-missing-map",
    /// Saves overlay: a vehicle is not installed (variable `name`).
    SAVES_WARN_MISSING_VEHICLE = "saves-warn-missing-vehicle",
    /// Saves overlay: content differs from the save (variable `name`).
    SAVES_WARN_CHANGED = "saves-warn-changed",
    /// Saves overlay: load button.
    SAVES_LOAD = "saves-load",
    /// Saves overlay: overwrite button.
    SAVES_OVERWRITE = "saves-overwrite",
    /// Saves overlay: rename button.
    SAVES_RENAME = "saves-rename",
    /// Saves overlay: delete button.
    SAVES_DELETE = "saves-delete",
    /// Overwrite question: title.
    SAVES_OVERWRITE_TITLE = "saves-overwrite-title",
    /// Overwrite question: consequence (variable `name`).
    SAVES_OVERWRITE_BODY = "saves-overwrite-body",
    /// Delete question: title.
    SAVES_DELETE_TITLE = "saves-delete-title",
    /// Delete question: consequence (variable `name`).
    SAVES_DELETE_BODY = "saves-delete-body",
    /// Load question: title.
    SAVES_LOAD_TITLE = "saves-load-title",
    /// Load question: consequence (variable `name`).
    SAVES_LOAD_BODY = "saves-load-body",
    /// Toast: saved (variable `name`).
    SAVE_SAVED_TOAST = "save-saved-toast",
    /// Toast: autosaved (variable `name`).
    SAVE_AUTOSAVED_TOAST = "save-autosaved-toast",
    /// Toast: saving failed (variable `error`).
    SAVE_FAILED_TOAST = "save-failed-toast",
    /// Toast: loading failed (variable `error`).
    LOAD_FAILED_TOAST = "load-failed-toast",
    /// Toast: loaded (variable `name`).
    SAVE_LOADED_TOAST = "save-loaded-toast",
    /// Toast: loaded with problems (variable `count`).
    SAVE_LOADED_WARNINGS = "save-loaded-warnings",
    /// Name of the quick save.
    SAVE_QUICK_NAME = "save-quick-name",
    /// Name of the autosave.
    SAVE_AUTO_NAME = "save-auto-name",
    /// Load error: what happened.
    LOAD_ERROR_TITLE = "load-error-title",
    /// Load error: damaged file (variables `name`, `error`).
    LOAD_ERR_CORRUPT = "load-err-corrupt",
    /// Load error: made by a newer version (variables `name`, `found`).
    LOAD_ERR_NEWER = "load-err-newer",
    /// Load error: the map is missing (variables `name`, `map`).
    LOAD_ERR_MISSING_MAP = "load-err-missing-map",
    /// Load error: no vehicle can be rebuilt (variables `name`, `vehicles`).
    LOAD_ERR_MISSING_VEHICLES = "load-err-missing-vehicles",
    /// Load error: anything else (variables `name`, `error`).
    LOAD_ERR_OTHER = "load-err-other",
    /// Load error: pick another save.
    LOAD_CHOOSE_ANOTHER = "load-choose-another",
    /// Load error: close.
    LOAD_BACK = "load-back",
}
