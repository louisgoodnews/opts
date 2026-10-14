# Settings: categories, rows, options. Source language.
settings-cat-graphics = Graphics
settings-cat-audio = Audio
settings-cat-controls = Controls
settings-cat-interface = Interface
settings-cat-gameplay = Gameplay
settings-cat-content = Content
settings-cat-advanced = Advanced

set-window-mode = Window mode
set-window-mode-desc = Windowed, borderless or exclusive fullscreen. You get 15 seconds to confirm a change.
set-opt-window-windowed = Windowed
set-opt-window-borderless = Borderless
set-opt-window-fullscreen = Fullscreen
set-resolution = Window size
set-resolution-desc = Size of the window in pixels. You get 15 seconds to confirm a change.
set-opt-res-1024x640 = 1024 × 640
set-opt-res-1280x720 = 1280 × 720
set-opt-res-1600x900 = 1600 × 900
set-opt-res-1920x1080 = 1920 × 1080
set-opt-res-2560x1440 = 2560 × 1440
set-opt-res-3840x2160 = 3840 × 2160
set-vsync = Vertical sync
set-vsync-desc = Matches the frame rate to your display. Prevents tearing, can add a little delay.
set-texture-filtering = Texture filtering
set-texture-filtering-desc = Sharper textures at an angle, small cost.
set-opt-filter-off = Off
set-opt-filter-4 = 4×
set-opt-filter-8 = 8×
set-opt-filter-16 = 16×
set-look = Look
set-look-desc = Colour and contrast of the 3D world.
set-opt-look-vivid = Vivid
set-opt-look-natural = Natural
set-opt-look-bevy = Bevy default
set-view-distance = View distance
set-view-distance-desc = How far scenery and tiles are drawn.

set-audio-master = Master volume
set-audio-master-desc = Overall volume of the simulator.
set-audio-engine = Engine volume
set-audio-engine-desc = Volume of your vehicle's engine and drivetrain.
set-audio-ambience = Ambience volume
set-audio-ambience-desc = Traffic, weather and city sounds.
set-audio-interface = Interface volume
set-audio-interface-desc = Clicks and notification sounds.

set-rebind = Change a key or button
set-rebind-desc = Choose your own keys, gamepad buttons and wheel controls.

set-theme-mode = Theme
set-theme-mode-desc = Dark is the default. Auto switches by the clock, System follows your desktop.
set-auto-light-from = Light theme from
set-auto-light-from-desc = Time of day the light theme starts in Auto mode (HH:MM).
set-auto-dark-from = Dark theme from
set-auto-dark-from-desc = Time of day the dark theme starts in Auto mode (HH:MM).
set-high-contrast = High contrast
set-high-contrast-desc = Stronger colours and outlines. Overrides the theme and is never switched automatically.
set-accent = Accent colour
set-accent-desc = The colour of selections and primary buttons.
set-theme-file = Theme file
set-theme-file-desc = Name of a theme file in the themes folder of your configuration folder; empty uses the built-in theme.
set-ui-scale = Interface size
set-ui-scale-desc = Scales text and controls, in percent.
set-density = Density
set-density-desc = How much space the interface leaves between elements.
set-reduced-motion = Reduce motion
set-reduced-motion-desc = Transitions and animations become instant.
set-language = Language
set-language-desc = The language of menus and messages. Content from OMSI keeps its own language.
set-language-reload = Reload languages
set-language-reload-desc = Look in the language folder again, so a pack you added or edited shows up without restarting.
set-lang-info = { $percent } % translated, by { $authors }
set-lang-info-unsigned = { $percent } % translated
set-lang-gaps =
    { $count ->
        [one] One text is not translated yet and shows English.
       *[other] { $count } texts are not translated yet and show English.
    }
set-lang-outdated = Made for an older version of the texts.
set-lang-problems =
    { $count ->
        [one] One problem in this pack; the faulty text shows English.
       *[other] { $count } problems in this pack; the faulty texts show English.
    }
set-lang-rtl = Right-to-left writing is not supported yet, so this language cannot be selected.
set-lang-fonts = The bundled font lacks { $scripts }; those characters will be missing.
set-lang-folder = To add a language, put a folder with a pack.json and text files into { $path }.
settings-toast-languages-reloaded =
    { $count ->
        [0] No language packs found.
        [one] One language pack loaded.
       *[other] { $count } language packs loaded.
    }
settings-toast-languages-problems =
    { $count ->
        [one] One problem was found; see the log or the l10n.report command.
       *[other] { $count } problems were found; see the log or the l10n.report command.
    }
set-units = Units
set-units-desc = Speeds are shown in kilometres or miles per hour.
set-opt-units-kmh = km/h
set-opt-units-mph = mph
set-hud-hints = Show hints
set-hud-hints-desc = Short tips next to controls and in the cab.
set-hud-speed = Speed
set-hud-speed-desc = Show the speed readout.
set-hud-gear = Gear
set-hud-gear-desc = Show the selected gear.
set-hud-rpm = Engine speed
set-hud-rpm-desc = Show the engine speed when the simulation provides it.
set-hud-parking-brake = Parking brake lamp
set-hud-parking-brake-desc = Show the parking brake state.
set-hud-session = Map and entry point
set-hud-session-desc = Show where the drive takes place; it fades after a few seconds.
set-hud-clock = Clock and weather
set-hud-clock-desc = Show the session clock and the weather; it fades after a few seconds.
set-hud-camera-label = Camera name
set-hud-camera-label-desc = Show the name of the camera for a moment after you change it.

set-traffic = Road traffic
set-traffic-desc = Most AI vehicles driving at the same time (0 for none). Applies to the next drive.
set-pedestrians = Pedestrians
set-pedestrians-desc = Most people walking at the same time (0 for none). Applies to the next drive.
set-time-scale = Time scale
set-time-scale-desc = Runs the session clock faster than real time.

set-omsi-path = OMSI 2 folder
set-omsi-path-desc = The folder your OMSI 2 is installed in. Maps and vehicles are read from here, never changed.
set-discovery-interval = Look for new content every
set-discovery-interval-desc = Days between automatic scans of the OMSI folder at start-up.
set-rescan = Scan now
set-rescan-desc = Looks for new or changed maps and vehicles in the background.
set-clear-cache = Clear texture cache
set-clear-cache-desc = Deletes the converted textures; they are made again when needed.
set-recent-map = Last map
set-recent-map-desc = The map you drove last.

set-developer-panel = Developer panel
set-developer-panel-desc = Shows statistics and debug information in the drive view (key F10).
set-log-level = Log detail
set-log-level-desc = How much the game writes to its log file. More detail helps with bug reports.
set-opt-log-error = Errors only
set-opt-log-warn = Warnings
set-opt-log-info = Normal
set-opt-log-debug = Detailed


settings-title = Settings
settings-search-placeholder = Search settings
settings-reset = Reset
settings-reset-disabled = Already at the default value
settings-no-results = No settings match your search. Try another word.
settings-restart-note = Takes effect after a restart.
settings-needs = Needs { $what }, which this computer does not have.
settings-step-down = −
settings-step-up = +
settings-choose = Choose...
settings-path-none = Not set
settings-theme-file-builtin = Built-in
settings-keep-title = Keep these display settings?
settings-keep-body = Going back to the old settings in { $seconds } seconds.
settings-keep = Keep
settings-revert = Go back

settings-info-theme-now = Right now the { $theme } theme is shown.
settings-info-catalogue = Catalogue: { $maps } maps, { $vehicles } vehicles, { $scenery } scenery objects, { $splines } splines and { $other } other entries (trailers and AI vehicles, fonts, humans and more).
settings-info-catalogue-total = { $entities } entries in total, built from { $files } files. A file is one file on disk; an entry is one thing made of one or more files, such as a map or a vehicle. Maps and vehicles are counted as on the Home screen.
settings-info-last-scan = Last scan: { $when }.
settings-info-never-scanned = The catalogue has not been scanned yet.
settings-info-cache = Texture cache: { $files } files, { $size } MB.
settings-info-locations = Where your data lives
settings-info-settings-file = Your settings are saved in: { $path }
settings-info-reference-file = Fixed starting values (read only): { $path }
settings-info-catalogue-file = The catalogue of your OMSI content: { $path }
settings-info-themes-folder = Your theme files go in: { $path }
settings-info-cache-folder = Converted textures are cached in: { $path }
settings-info-last-map = The map you drove last: { $path }
settings-keys-title = Keys and buttons in the menus
settings-key-navigate = Move between controls: arrow keys or D-pad.
settings-key-regions = Jump to the next or previous area: Tab or Shift+Tab.
settings-key-confirm = Use the selected control: Enter or Space, A on a gamepad.
settings-key-back = Go back or close: Escape, Backspace, B on a gamepad.
settings-key-search = Search: Ctrl+F or /, Y on a gamepad.
settings-key-tabs = Next or previous tab: ] and [, Ctrl+Tab, shoulder buttons on a gamepad.
settings-key-scroll = Scroll: Page Up, Page Down, Home and End.
settings-key-menu = Open the menu: Start on a gamepad.
settings-key-developer = Developer panel in a drive: F10 (when switched on in Advanced).
settings-toast-rescan-started = Looking for new content in the background.
settings-toast-no-omsi = No OMSI 2 folder is set, so there is nothing to scan.
settings-toast-cache-cleared = The texture cache was cleared.
settings-toast-omsi-restart = OMSI folder saved. Restart OPTS to load its content.
settings-toast-omsi-invalid = That folder does not look like an OMSI 2 installation (it needs the folders maps and Vehicles).

settings-back = Back
set-autosave = Autosave
set-autosave-desc = Saves the drive to the autosave slot every this many minutes of play. 0 turns it off.


# Hardware (M-0081)
set-hw-redetect = Detect hardware again
set-hw-redetect-desc = Looks at your computer again and stores the result in a file on this computer. Nothing is sent anywhere.
set-hw-delete = Delete stored hardware profile
set-hw-delete-desc = Removes the file with the detected hardware from this computer.
set-hw-reports = Allow including the hardware profile in reports
set-hw-reports-desc = When on, a bug report you save also lists the hardware shown below. You can read it in the report file before you share it. Off by default.
settings-info-hw-title = Your hardware
settings-info-hw-privacy = OPTS looks at your computer only to choose good starting values. The result stays on this computer. It holds no serial numbers, names or addresses, and nothing is sent anywhere. The full list: run opts-tools hardware report.
settings-info-hw-detecting = Detecting your hardware ...
settings-info-hw-stored = A copy is stored in: { $path }
settings-info-hw-not-stored = No copy is stored on this computer.
settings-info-hw-os = System: { $os }
settings-info-hw-cpu = Processor: { $vendor }, { $physical } cores, { $logical } threads
settings-info-hw-memory = Memory: { $available } of { $total } GB free
settings-info-hw-gpu = Graphics: { $vendor }, { $class }, { $vram } GB video memory, { $backend }
settings-info-hw-display = Screen: { $width } × { $height }, { $count } connected
settings-info-hw-storage = Content drive: { $class }, { $free } GB free
settings-info-hw-recs = Starting values chosen for this computer: { $radius } tiles around you, texture filtering { $filter }, look { $look }, interface size { $scale } %
settings-info-hw-dlss = DLSS: { $dlss }
settings-toast-hw-detecting = Detecting your hardware. The page updates when it is done.
settings-toast-hw-deleted = The stored hardware profile was deleted.
settings-toast-hw-nothing-stored = No hardware profile was stored.
hw-reason-not_probed = not detected yet
hw-reason-not_on_this_platform = not available on this system
hw-reason-needs_renderer = known once the game window is open
hw-reason-unavailable = not available on this machine
hw-reason-unreadable = could not be read
hw-reason-unparsed = not understood
hw-reason-timeout = skipped, it took too long
hw-reason-not_requested = not measured
hw-reason-cancelled = cancelled
hw-dlss-yes = possible
hw-dlss-unknown = not known yet
hw-dlss-no-not_nvidia = not possible, it needs an NVIDIA graphics card
hw-dlss-no-os_unsupported = not possible on this operating system
hw-dlss-no-backend_unsupported = not possible with this graphics interface
hw-dlss-no-no_ray_tracing = not possible, the card has no ray tracing
hw-dlss-no-no_runtime = not possible, the DLSS files are missing

scan-never = never
scan-chip = { $status } · Last scan: { $when }
scan-level-good = Up to date
scan-level-recommended = Scan recommended
scan-level-needed = Scan needed
scan-why-recent = Scanned recently and the OMSI folder has not changed since. Nothing to do.
scan-why-older = The last scan is older than your automatic scan interval ({ $days } day(s)). Press "Scan now" to pick up new content.
scan-why-changed = The OMSI folder was modified after the last scan, so there is probably new or removed content. Press "Scan now".
scan-why-running = A scan is running in the background. The numbers below update when it is done.
scan-why-never = The OMSI folder has not been scanned yet. Press "Scan now".
scan-why-failed = The last scan failed. Check the OMSI folder above, then press "Scan now".
scan-why-empty = The last scan found no maps or vehicles. Check the OMSI folder above, then press "Scan now".
scan-why-folder-missing = The OMSI folder is not set or cannot be found. Choose it above, then press "Scan now".
