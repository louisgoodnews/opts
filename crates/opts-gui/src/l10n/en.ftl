# English: the source language. Keys are lower-case words joined by dashes.
# Subset of Fluent: messages, { $variables }, { other-message } references and plural selectors.

widget-clear = Clear
widget-combobox-placeholder = Select option...
widget-not-available-yet = Feature not available yet!
widget-not-available-yet-reason = { widget-not-available-yet } ({ $milestone })
widget-items =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }

menu-shutdown = Shutdown
menu-load-last-game = Load Last Game
menu-start-new-game = Start New Game
menu-settings = Settings
menu-credits = Credits
menu-theme-light = Light
menu-theme-dark = Dark
menu-title = OPTS
menu-subtitle = Open Public Transport Simulator
menu-card-title = Clean-Room Simulation Engine
menu-card-no-map = No map loaded · Select Start New Game to begin.
menu-card-map-preview = Map Preview: { $path }
menu-card-storage = Storage: SQLite (WAL) · UI: Material Design 3 Glassmorphism
menu-footer = OPTS v0.1.0-alpha · Clean-Room Architecture
