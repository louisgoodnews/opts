# Wizard, master-detail pattern and loading screen (English, the source language).

md-search-label = Search
md-search = Search { $count } entries…
md-section-recent = Recent
md-section-favourites = Favourites
md-section-all = All
md-no-results = No results for “{ $query }”
md-clear-filter = Clear filter
md-star-add = Add to favourites (Ctrl+D)
md-star-remove = Remove from favourites (Ctrl+D)

# --- Plan a drive ---
wiz-title = Plan a drive
wiz-step-where = Where
wiz-step-what = What
wiz-step-when = When
wiz-step-review = Review
wiz-back = Back
wiz-next = Next
wiz-skip = Skip to review
wiz-drive = Drive
wiz-change = Change
wiz-open-settings = Open settings
wiz-reason-no-map = Choose a map first
wiz-reason-no-start = Choose a start point first
wiz-reason-no-vehicle = Choose a vehicle first
wiz-save-failed = Your choices could not be saved to { $path }. The drive starts anyway.
wiz-date = { $day } { $month } { $year }

wiz-where-search = Search { $count } maps…
wiz-where-empty = No maps found. Check the OMSI folder or rescan.
wiz-where-start = Start point
wiz-where-no-starts = This map has no named start points.
wiz-where-description = Description from the map, in its original language
wiz-where-remembered-missing = Your last map is no longer installed.
wiz-fact-years = Set in { $years }
wiz-fact-tiles =
    { $count ->
        [one] { $count } tile
       *[other] { $count } tiles
    }
wiz-fact-starts =
    { $count ->
        [one] { $count } start point
       *[other] { $count } start points
    }
wiz-fact-depot = Depot: { $depot }

wiz-what-search = Search { $count } vehicles…
wiz-what-none = Drive without a vehicle
wiz-what-none-sub = Map only, free camera
wiz-what-none-body = Start on the map with a free camera and no instruments. The simulation runs. Spawn a vehicle whenever you like from the pause menu, under Vehicle.
wiz-what-empty = No vehicles found. Check the OMSI folder or rescan.
wiz-class-road = Road vehicle
wiz-class-rail = Rail vehicle
wiz-class-aircraft = Aircraft
wiz-fact-mass = Empty weight: { $tonnes } t
wiz-what-livery = Livery
wiz-what-livery-default = Default
wiz-what-livery-loading = Reading the liveries…
wiz-what-livery-none = This vehicle offers no other liveries.
wiz-what-configure = Configure…
wiz-what-no-options = This vehicle has no equipment options.
wiz-what-options-changed =
    { $count ->
        [one] 1 option changed
       *[other] { $count } options changed
    }
wiz-what-remembered-missing = Your last vehicle is no longer installed.
wiz-options-title = Equipment
wiz-options-reset = Reset
wiz-options-done = Done

wiz-when-quick = Time of day
wiz-when-exact = Date and time
wiz-when-weather = Weather
wiz-when-now = Now
wiz-when-morning = Morning
wiz-when-noon = Noon
wiz-when-evening = Evening
wiz-when-night = Night
wiz-when-day = Day
wiz-when-month = Month
wiz-when-year = Year
wiz-when-hour = Hour
wiz-when-minute = Minute
wiz-step-less = Decrease
wiz-step-more = Increase
wiz-when-era-hint = This map is set in { $from }–{ $to }.
wiz-month-1 = January
wiz-month-2 = February
wiz-month-3 = March
wiz-month-4 = April
wiz-month-5 = May
wiz-month-6 = June
wiz-month-7 = July
wiz-month-8 = August
wiz-month-9 = September
wiz-month-10 = October
wiz-month-11 = November
wiz-month-12 = December
wiz-weather-clear = Clear day
wiz-weather-clear-effect = Clear sky, full sunlight
wiz-weather-overcast = Overcast
wiz-weather-overcast-effect = Cloud cover, softer light
wiz-weather-rain = Rain
wiz-weather-rain-effect = Dimmer, diffuse light; rain itself is not shown yet
wiz-weather-snow = Snow
wiz-weather-snow-effect = Dimmer, diffuse light; snowfall itself is not shown yet
wiz-weather-fog = Fog
wiz-weather-fog-effect = Dimmer, diffuse light; fog itself is not shown yet
wiz-weather-live = Live weather

wiz-review-title = Ready to drive?

wiz-option-vis_wimpel_invisible = Pennant on the front
wiz-option-wimpel_visibility = Flag mount
wiz-option-vis_grill_invisible = Grille cover
wiz-option-ident_visible = Operator identification

# --- Loading screen ---
load-title = Preparing your drive
load-tile-map = MAP
wiz-no-vehicle = No vehicle
load-no-vehicle-sub = Free camera
load-tile-vehicle = VEHICLE
load-tile-when = TIME AND WEATHER
load-stage-catalogue = Reading your OMSI content (first start only)…
load-stage-resolve = Finding your map and vehicle…
load-stage-map = Loading the map: { $done } of { $total } tiles
load-stage-light = Setting up light and weather…
load-stage-vehicle = Preparing the vehicle…
load-stage-vehicle-meshes = Preparing the vehicle: { $left } protected models left
load-stage-start = Getting ready to drive…
load-stage-ready = Ready
load-stage-failed = Loading stopped
load-cancel = Cancel
load-cancel-title = Cancel loading?
load-cancel-body = You go back to Plan a drive. Nothing is lost; your choices stay.
load-cancel-keep = Keep loading
load-cancel-yes = Cancel loading
load-ready-problems =
    { $count ->
        [one] Ready with 1 problem
       *[other] Ready with { $count } problems
    }
load-failed = Couldn't load the drive
load-details = Details
load-continue = Continue
load-retry = Retry
load-choose-another = Choose another
load-back = Back
load-problem-map = Couldn't load the map { $name }
load-problem-vehicle = Couldn't load the vehicle { $name }
load-problem-no-model = The vehicle { $name } has no model file in your OMSI folder
load-problem-tiles = { $count } of { $total } map tiles are missing
load-problem-objects = { $count } scenery objects could not be built
load-problem-files = { $count } files could not be read
load-cause-map-unreadable = The map folder or its main file could not be read.
load-cause-no-tiles = None of the map's tiles could be read.
load-cause-no-vehicle-file = A vehicle from the map's own lists is used instead.
load-cause-vehicle-unreadable = The model file is missing or unreadable.
load-controls-title = CONTROLS
load-controls-accelerate = Accelerate
load-controls-brake = Brake
load-controls-steer = Steer
load-controls-parking = Parking brake
load-controls-gears = Drive, neutral, reverse
load-controls-cameras = Camera views
load-controls-pause = Pause
load-controls-menu = Menu
load-minor-toast =
    { $count ->
        [one] Loaded with 1 minor problem
       *[other] Loaded with { $count } minor problems
    }
load-minor-title = Minor loading problems
load-close = Close
