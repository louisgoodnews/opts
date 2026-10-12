<!-- Generated from the settings registry by the test `reference_page_is_up_to_date`; regenerate with `UPDATE_SETTINGS_DOC=1 cargo test -p opts-game settings_registry`. -->

# Settings Reference

Every setting is declared once as a `SettingDescriptor` ([registry](../architecture/settings-registry.md)); this page, the settings screen, the saved file and the control-protocol commands `settings.list`, `settings.get`, `settings.set` and `settings.reset` all come from it. Values are stored as text in `settings.db`; *Applies* says when a change takes effect.

## ⟦settings-cat-graphics⟧

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `graphics.window_mode` | ⟦set-window-mode⟧ | `windowed`, `borderless`, `fullscreen` | `windowed` | at once, 15 s to confirm | available | ⟦set-window-mode-desc⟧ |
| `graphics.resolution` | ⟦set-resolution⟧ | `1024x640`, `1280x720`, `1600x900`, `1920x1080`, `2560x1440`, `3840x2160` | `1920x1080` | at once, 15 s to confirm | available | ⟦set-resolution-desc⟧ |
| `graphics.vsync` | ⟦set-vsync⟧ | on / off | `false` | at once | available | ⟦set-vsync-desc⟧ |
| `texture_filtering` | ⟦set-texture-filtering⟧ | `off`, `4`, `8`, `16` | `16` | at once | available | ⟦set-texture-filtering-desc⟧ |
| `look` | ⟦set-look⟧ | `vivid`, `natural`, `bevy` | `vivid` | at once | available | ⟦set-look-desc⟧ |
| `graphics.view_distance` | ⟦set-view-distance⟧ | whole number 1 to 8 | `4` | at once | not available yet (M-0029) | ⟦set-view-distance-desc⟧ |

## ⟦settings-cat-audio⟧

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `audio.master` | ⟦set-audio-master⟧ | whole number 0 to 100 | `80` | at once | not available yet (M-0027) | ⟦set-audio-master-desc⟧ |
| `audio.engine` | ⟦set-audio-engine⟧ | whole number 0 to 100 | `80` | at once | not available yet (M-0027) | ⟦set-audio-engine-desc⟧ |
| `audio.ambience` | ⟦set-audio-ambience⟧ | whole number 0 to 100 | `60` | at once | not available yet (M-0027) | ⟦set-audio-ambience-desc⟧ |
| `audio.interface` | ⟦set-audio-interface⟧ | whole number 0 to 100 | `60` | at once | not available yet (M-0027) | ⟦set-audio-interface-desc⟧ |

## ⟦settings-cat-controls⟧

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `controls.rebind` | ⟦set-rebind⟧ | action (no stored value) | - | at once | not available yet (M-0035) | ⟦set-rebind-desc⟧ |

## ⟦settings-cat-interface⟧

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `theme_mode` | ⟦set-theme-mode⟧ | `dark`, `light`, `system`, `auto` | `dark` | at once | available | ⟦set-theme-mode-desc⟧ |
| `auto_light_from` | ⟦set-auto-light-from⟧ | time `HH:MM` | `07:00` | at once | available | ⟦set-auto-light-from-desc⟧ |
| `auto_dark_from` | ⟦set-auto-dark-from⟧ | time `HH:MM` | `19:00` | at once | available | ⟦set-auto-dark-from-desc⟧ |
| `high_contrast` | ⟦set-high-contrast⟧ | on / off | `false` | at once | available | ⟦set-high-contrast-desc⟧ |
| `active_theme` | ⟦set-accent⟧ | `Default`, `Amber`, `Violet`, `Emerald`, `Teal` | `Default` | at once | available | ⟦set-accent-desc⟧ |
| `theme_file` | ⟦set-theme-file⟧ | text | - | at once | available | ⟦set-theme-file-desc⟧ |
| `ui_scale` | ⟦set-ui-scale⟧ | whole number 75 to 200 | `100` | at once | available | ⟦set-ui-scale-desc⟧ |
| `density` | ⟦set-density⟧ | `comfortable`, `compact`, `dense` | `comfortable` | at once | available | ⟦set-density-desc⟧ |
| `reduced_motion` | ⟦set-reduced-motion⟧ | on / off | `false` | at once | available | ⟦set-reduced-motion-desc⟧ |
| `interface.language` | ⟦set-language⟧ | `en`, `de` | `en` | at once | available | ⟦set-language-desc⟧ |
| `interface.units` | ⟦set-units⟧ | `kmh`, `mph` | `kmh` | at once | available | ⟦set-units-desc⟧ |
| `interface.hud_hints` | ⟦set-hud-hints⟧ | on / off | `true` | at once | available | ⟦set-hud-hints-desc⟧ |

## ⟦settings-cat-gameplay⟧

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `gameplay.traffic` | ⟦set-traffic⟧ | whole number 0 to 200 | `30` | at once | available | ⟦set-traffic-desc⟧ |
| `gameplay.pedestrians` | ⟦set-pedestrians⟧ | whole number 0 to 200 | `40` | at once | available | ⟦set-pedestrians-desc⟧ |
| `gameplay.time_scale` | ⟦set-time-scale⟧ | number 0 to 60 | `1` | at once | not available yet (M-0064) | ⟦set-time-scale-desc⟧ |

## ⟦settings-cat-content⟧

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `omsi_path` | ⟦set-omsi-path⟧ | path | - | at once | available | ⟦set-omsi-path-desc⟧ |
| `discovery_interval_days` | ⟦set-discovery-interval⟧ | whole number 1 to 90 | `1` | at once | available | ⟦set-discovery-interval-desc⟧ |
| `content.rescan` | ⟦set-rescan⟧ | action (no stored value) | - | at once | available | ⟦set-rescan-desc⟧ |
| `content.clear_cache` | ⟦set-clear-cache⟧ | action (no stored value) | - | at once | available | ⟦set-clear-cache-desc⟧ |
| `recent_map_path` | ⟦set-recent-map⟧ | path | - | at once | internal | ⟦set-recent-map-desc⟧ |

## ⟦settings-cat-advanced⟧

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `advanced.developer_panel` | ⟦set-developer-panel⟧ | on / off | `false` | at once | available | ⟦set-developer-panel-desc⟧ |
| `advanced.log_level` | ⟦set-log-level⟧ | `error`, `warn`, `info`, `debug` | `info` | at once | available | ⟦set-log-level-desc⟧ |
