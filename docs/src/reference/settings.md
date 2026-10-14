<!-- Generated from the settings registry by the test `reference_page_is_up_to_date`; regenerate with `UPDATE_SETTINGS_DOC=1 cargo test -p opts-game settings_registry`. -->

# Settings Reference

Every setting is declared once as a `SettingDescriptor` ([registry](../architecture/settings-registry.md)); this page, the settings screen, the saved file and the control-protocol commands `settings.list`, `settings.get`, `settings.set` and `settings.reset` all come from it. Values are stored as text in `settings.db`; *Applies* says when a change takes effect.

## Graphics

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `graphics.window_mode` | Window mode | `windowed`, `borderless`, `fullscreen` | `windowed` | at once, 15 s to confirm | available | Windowed, borderless or exclusive fullscreen. You get 15 seconds to confirm a change. |
| `graphics.resolution` | Window size | `1024x640`, `1280x720`, `1600x900`, `1920x1080`, `2560x1440`, `3840x2160` | `1920x1080` | at once, 15 s to confirm | available | Size of the window in pixels. You get 15 seconds to confirm a change. |
| `graphics.vsync` | Vertical sync | on / off | `false` | at once | available | Matches the frame rate to your display. Prevents tearing, can add a little delay. |
| `texture_filtering` | Texture filtering | `off`, `4`, `8`, `16` | `16` | at once | available | Sharper textures at an angle, small cost. |
| `look` | Look | `vivid`, `natural`, `bevy` | `vivid` | at once | available | Colour and contrast of the 3D world. |
| `graphics.view_distance` | View distance | whole number 1 to 8 | `4` | at once | not available yet (M-0029) | How far scenery and tiles are drawn. |

## Audio

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `audio.master` | Master volume | whole number 0 to 100 | `80` | at once | not available yet (M-0027) | Overall volume of the simulator. |
| `audio.engine` | Engine volume | whole number 0 to 100 | `80` | at once | not available yet (M-0027) | Volume of your vehicle's engine and drivetrain. |
| `audio.ambience` | Ambience volume | whole number 0 to 100 | `60` | at once | not available yet (M-0027) | Traffic, weather and city sounds. |
| `audio.interface` | Interface volume | whole number 0 to 100 | `60` | at once | not available yet (M-0027) | Clicks and notification sounds. |

## Controls

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `controls.rebind` | Change a key or button | action (no stored value) | - | at once | not available yet (M-0035) | Choose your own keys, gamepad buttons and wheel controls. |

## Interface

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `theme_mode` | Theme | `dark`, `light`, `system`, `auto` | `dark` | at once | available | Dark is the default. Auto switches by the clock, System follows your desktop. |
| `auto_light_from` | Light theme from | time `HH:MM` | `07:00` | at once | available | Time of day the light theme starts in Auto mode (HH:MM). |
| `auto_dark_from` | Dark theme from | time `HH:MM` | `19:00` | at once | available | Time of day the dark theme starts in Auto mode (HH:MM). |
| `high_contrast` | High contrast | on / off | `false` | at once | available | Stronger colours and outlines. Overrides the theme and is never switched automatically. |
| `active_theme` | Accent colour | `Default`, `Amber`, `Violet`, `Emerald`, `Teal` | `Default` | at once | available | The colour of selections and primary buttons. |
| `theme_file` | Theme file | text | - | at once | available | Name of a theme file in the themes folder of your configuration folder; empty uses the built-in theme. |
| `ui_scale` | Interface size | whole number 75 to 200 | `100` | at once | available | Scales text and controls, in percent. |
| `density` | Density | `comfortable`, `compact`, `dense` | `comfortable` | at once | available | How much space the interface leaves between elements. |
| `reduced_motion` | Reduce motion | on / off | `false` | at once | available | Transitions and animations become instant. |
| `interface.language` | Language | language tag (`en`, `de`, or an installed pack such as `fr`; see `l10n.languages`) | `en` | at once | available | The language of menus and messages. Content from OMSI keeps its own language. |
| `interface.language_reload` | Reload languages | action (no stored value) | - | at once | available | Look in the language folder again, so a pack you added or edited shows up without restarting. |
| `interface.units` | Units | `kmh`, `mph` | `kmh` | at once | available | Speeds are shown in kilometres or miles per hour. |
| `interface.hud_hints` | Show hints | on / off | `true` | at once | available | Short tips next to controls and in the cab. |
| `hud.speed` | Speed | on / off | `true` | at once | available | Show the speed readout. |
| `hud.gear` | Gear | on / off | `true` | at once | available | Show the selected gear. |
| `hud.rpm` | Engine speed | on / off | `true` | at once | available | Show the engine speed when the simulation provides it. |
| `hud.parking_brake` | Parking brake lamp | on / off | `true` | at once | available | Show the parking brake state. |
| `hud.session_info` | Map and entry point | on / off | `true` | at once | available | Show where the drive takes place; it fades after a few seconds. |
| `hud.clock` | Clock and weather | on / off | `true` | at once | available | Show the session clock and the weather; it fades after a few seconds. |
| `hud.camera_label` | Camera name | on / off | `true` | at once | available | Show the name of the camera for a moment after you change it. |

## Gameplay

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `gameplay.traffic` | Road traffic | whole number 0 to 200 | `30` | at once | available | Most AI vehicles driving at the same time (0 for none). Applies to the next drive. |
| `gameplay.pedestrians` | Pedestrians | whole number 0 to 200 | `40` | at once | available | Most people walking at the same time (0 for none). Applies to the next drive. |
| `gameplay.time_scale` | Time scale | number 0 to 60 | `1` | at once | not available yet (M-0064) | Runs the session clock faster than real time. |
| `gameplay.autosave_minutes` | Autosave | whole number 0 to 60 | `0` | at once | available | Saves the drive to the autosave slot every this many minutes of play. 0 turns it off. |

## Content

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `omsi_path` | OMSI 2 folder | path | - | after restart | available | The folder your OMSI 2 is installed in. Maps and vehicles are read from here, never changed. |
| `discovery_interval_days` | Look for new content every | whole number 1 to 90 | `1` | at once | available | Days between automatic scans of the OMSI folder at start-up. |
| `content.rescan` | Scan now | action (no stored value) | - | at once | available | Looks for new or changed maps and vehicles in the background. |
| `content.clear_cache` | Clear texture cache | action (no stored value) | - | at once | available | Deletes the converted textures; they are made again when needed. |
| `recent_map_path` | Last map | path | - | at once | internal | The map you drove last. |

## Advanced

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `advanced.developer_panel` | Developer panel | on / off | `false` | at once | available | Shows statistics and debug information in the drive view (key F10). |
| `advanced.log_level` | Log detail | `error`, `warn`, `info`, `debug` | `info` | at once | available | How much the game writes to its log file. More detail helps with bug reports. |
| `advanced.hardware_redetect` | Detect hardware again | action (no stored value) | - | at once | available | Looks at your computer again and stores the result in a file on this computer. Nothing is sent anywhere. |
| `advanced.hardware_delete` | Delete stored hardware profile | action (no stored value) | - | at once | available | Removes the file with the detected hardware from this computer. |
| `advanced.hardware_allow_reports` | Allow including the hardware profile in reports | on / off | `false` | at once | available | When on, a bug report you save also lists the hardware shown below. You can read it in the report file before you share it. Off by default. |
| `advanced.trace_scene` | Scene trace | `off`, `summary`, `full` | `off` | at once | available | Developer tool: records which mesh file is placed where (tile, position, decode quality) in a file next to the logs. Summary keeps counts and problems, Full keeps everything. Takes effect for the next map load. Costs nothing when off. |
| `advanced.trace_overlay` | Scene trace overlay | on / off | `false` | at once | available | Developer tool: draws a box around every traced object near the camera, coloured by decode quality (red = failed, orange = unreliable, yellow = decoded). Needs the scene trace. |
