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
| `graphics.hdr` | HDR rendering | on / off | `true` | at once | available | Renders the 3D view in high dynamic range and tones it down at the end, which keeps bright lights from clipping. Turn it off to save about 50 MB of video memory. |
| `graphics.default_fov` | Driver's view: field of view | whole number 0 to 120 | `0` | at once | available | The horizontal field of view of the driver's views, in degrees (30 to 120). 0 keeps each vehicle's own. A viewpoint you saved with its own field of view still wins. |
| `graphics.view_distance` | View distance | whole number 1 to 8 | `4` | at once | not available yet (M-0029) | How far scenery and tiles are drawn. |
| `graphics.water` | Water | on / off | `true` | after restart | available | Draws rivers, lakes and the sea of the map. Turn it off to see the ground below. |
| `graphics.weather_fog` | Weather: fog | on / off | `true` | at once | available | Draws the fog of the weather (distance haze and a fogged sky). Off shows the weather without fog. |
| `graphics.weather_clouds` | Weather: clouds | on / off | `true` | at once | available | Draws the cloud layer of the weather. Off keeps the sky and the dimmer light but hides the clouds. |
| `graphics.weather_precipitation` | Weather: rain and snow | `off`, `low`, `medium`, `high` | `medium` | at once | available | How many rain streaks or snowflakes are drawn around the camera at most (1500, 3000 or 6000). Light precipitation uses fewer. Off draws none. |
| `graphics.weather_wet_ground` | Weather: wet and snowy ground | on / off | `true` | at once | available | Darkens and smooths the ground and roads when it is wet, adds puddles and a snow cover. Off keeps them dry. |
| `graphics.light_quality_mode` | Lighting quality mode | `automatic`, `custom` | `automatic` | at once | available | Automatic: OPTS picks the lighting level from the measured frame time. Custom: you choose every effect yourself and nothing changes by itself. Switching to custom keeps the level reached as the starting values; switching back to automatic starts again at the fullest level. |
| `graphics.light_shafts` | Sun rays | `off`, `low`, `medium`, `high`, `ultra` | `medium` | at once | available | Light shafts through the air (volumetric fog). The quality is the number of steps per ray: low 8, medium 16, high 32, ultra 64. Off draws none and costs nothing. |
| `graphics.light_night_shafts` | Rays at night | on / off | `true` | at once | available | Also draws light shafts at night, lit by the moon. Costs more at night because of the many lamps. |
| `graphics.light_lantern_cones` | Cones of street lamps | on / off | `true` | at once | available | Visible cones of light below street lamps and other lanterns. |
| `graphics.light_headlight_cones` | Cones of headlights | on / off | `true` | at once | available | Visible cones of light in front of vehicle headlights. |
| `graphics.light_cone_cap` | Most light cones | whole number 0 to 96 | `64` | at once | available | How many light cones are drawn at most at the same time, from 0 to 96. The nearest ones are kept. |
| `graphics.light_cone_depth_fade` | Soft cone edges | on / off | `true` | at once | available | Cones fade out where they meet walls and the ground instead of cutting through them. Needs an extra depth pass and costs frame time. |
| `graphics.mirror_count` | Mirrors | whole number 0 to 2 | `2` | at once | available | How many mirrors show a live rear view, 0 to 2. Each one renders the scene again. |
| `graphics.mirror_rate` | Mirror update rate | `1`, `2`, `4` | `2` | at once | available | How often a mirror renders: every frame, every 2nd or every 4th frame. Less often is cheaper. |
| `graphics.glass_reflections` | Sky in glass | on / off | `true` | at once | available | Windows and other glass reflect the sky. |
| `graphics.bloom` | Bloom | on / off | `false` | at once | available | A soft glow around very bright lights. |
| `graphics.light_governor_target_fps` | Governor target frame rate | whole number 30 to 240 | `60` | at once | available | The frame rate the automatic lighting quality tries to keep. Without vsync it is the deadline; with vsync the deadline is your display's refresh interval, or this rate if that is lower. Frames that miss it by a wide margin make the quality step down; with no missed frames for a while it steps up again. |

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
| `controls.vehicle.battery` | Battery switch | input binding | `E` | at once | available | Batterietrennschalter |
| `controls.vehicle.engine_start` | Start engine | input binding | `M` | at once | available | Motor starten |
| `controls.vehicle.lights` | Headlights | input binding | `L` | at once | available | Scheinwerfer |
| `controls.vehicle.parking_light` | Parking lights | input binding | `Shift+L` | at once | available | Standlicht |
| `controls.vehicle.high_beam` | High beam | input binding | `F` | at once | available | Fernlicht |
| `controls.vehicle.fog_rear` | Rear fog light | input binding | `Ctrl+F` | at once | available | Nebelschlussleuchte |
| `controls.vehicle.indicator_left` | Indicator left | input binding | `Numpad7` | at once | available | Blinker links |
| `controls.vehicle.indicator_right` | Indicator right | input binding | `Numpad9` | at once | available | Blinker rechts |
| `controls.vehicle.indicator_off` | Indicators off | input binding | `NumpadDecimal` | at once | available | Blinker aus |
| `controls.vehicle.hazard` | Hazard lights | input binding | `B` | at once | available | Warnblinker |
| `controls.vehicle.horn` | Horn | input binding | `H` | at once | available | Hupe |
| `controls.vehicle.wiper` | Wipers | input binding | `none` | at once | available | Scheibenwischer |
| `controls.vehicle.door_1` | Door 1 | input binding | `NumpadDivide` | at once | available | Tür 1 |
| `controls.vehicle.door_2` | Door 2 | input binding | `NumpadMultiply` | at once | available | Tür 2 |
| `controls.vehicle.door_3` | Door 3 | input binding | `Numpad0` | at once | available | Tür 3 |
| `controls.vehicle.door_4` | Door 4 | input binding | `Shift+NumpadDivide` | at once | available | Tür 4 |
| `controls.vehicle.interior_driver` | Driver's light | input binding | `6` | at once | available | Fahrerlicht |
| `controls.vehicle.interior_lower_right` | Interior light, lower right | input binding | `7` | at once | available | Innenlicht unten rechts |
| `controls.vehicle.interior_upper` | Interior light, upper deck | input binding | `8` | at once | available | Innenlicht Oberdeck |
| `controls.vehicle.interior_lower` | Interior light, lower deck | input binding | `9` | at once | available | Innenlicht Unterdeck |
| `controls.vehicle.microphone` | Microphone | input binding | `Q` | at once | available | Mikrofon |
| `controls.vehicle.ticket` | Hand over ticket | input binding | `T` | at once | available | Fahrschein ausgeben |
| `controls.vehicle.ticket_change` | Ticket with change | input binding | `Ctrl+T` | at once | available | Fahrschein mit Rückgeld |
| `controls.vehicle.ticket_take` | Collect money | input binding | `Shift+T` | at once | available | Geld einsammeln |

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
| `ui_font` | Interface font | text | - | at once | available | A font family for all text; empty uses Noto Sans. Every font installed on this computer is listed, and so are the files you put in the fonts folder of your configuration folder (.ttf, .otf; after a restart). |
| `ui_font_size` | Text size | whole number 12 to 24 | `14` | at once | available | The size of body text, 12 to 24 px; every other text size follows it, none goes below 12 px. |
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
| `hud.minimap` | Mini map | on / off | `true` | at once | available | Show the map of the roads around you in the bottom right corner (F7). |
| `hud.minimap_zoom` | Mini map scale | `100`, `200`, `500`, `1000` | `200` | at once | available | How far the mini map looks around you. The + and - keys change it while driving. |
| `hud.minimap_zoom_mode` | Mini map zoom | `speed`, `fixed` | `speed` | at once | available | With speed zoom the map shows a small area when you are slow or stopped and the scale above when you are fast (Ctrl+F7). Fixed keeps the scale. |
| `hud.minimap_orientation` | Mini map direction | `heading`, `north` | `heading` | at once | available | Heading up turns the map with you; north up keeps north at the top (Shift+F7). |

## Gameplay

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `gameplay.traffic` | Road traffic | whole number 0 to 200 | `100` | at once | available | Most AI vehicles driving at the same time (0 for none). Applies to the next drive. |
| `gameplay.pedestrians` | Pedestrians | whole number 0 to 200 | `40` | at once | available | Most people walking at the same time (0 for none). Applies to the next drive. |
| `gameplay.time_scale` | Time scale | number 0 to 60 | `1` | at once | not available yet (M-0064) | Runs the session clock faster than real time. |
| `gameplay.autosave_minutes` | Autosave | whole number 0 to 60 | `0` | at once | available | Saves the drive to the autosave slot every this many minutes of play. 0 turns it off. |
| `gameplay.load_all_tiles` | Load all tiles on start | on / off | `false` | at once | available | Loads every tile of the map while the drive loads and keeps them all in memory. Loading takes longer and uses more memory, so large maps may need a strong machine. Applies to the next drive; switching it during a drive loads the rest in the background. |
| `gameplay.tile_radius` | Tile radius | whole number 0 to 16 | `0` | at once | available | Tiles kept loaded around the one you are on: 1 is 3 x 3, 2 is 5 x 5. 0 chooses automatically for this computer. Unused while all tiles are loaded. |
| `camera.door_overlay` | Door camera | on / off | `false` | at once | available | Shows the view from a door in the corner of the screen while the door opens or when you press F4 (F4 again: next door, then off). |

## Content

| Key | Name | Values | Default | Applies | Status | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `omsi_path` | OMSI 2 folder | path | - | after restart | available | The folder your OMSI 2 is installed in. Maps and vehicles are read from here, never changed. |
| `discovery_interval_days` | Look for new content every | whole number 1 to 90 | `1` | at once | available | Days between automatic scans of the OMSI folder at start-up. |
| `content.rescan` | Scan now | action (no stored value) | - | at once | available | Looks for new or changed maps and vehicles in the background. |
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
| `advanced.streaming_budget_ms` | Tile streaming budget | whole number 0 to 50 | `8` | at once | available | Milliseconds per frame that loading new map tiles may use while driving. A small value keeps the frame rate steady and fills in a tile over a few frames; a large one loads faster but can stutter; 0 builds one whole tile per frame as before. |
