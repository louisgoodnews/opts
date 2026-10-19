# Assistent, Master-Detail-Muster und Ladebildschirm (Deutsch).

md-search-label = Suche
md-search = { $count } Einträge durchsuchen…
md-section-recent = Zuletzt
md-section-favourites = Favoriten
md-section-all = Alle
md-no-results = Keine Treffer für „{ $query }“
md-clear-filter = Filter löschen
md-star-add = Zu Favoriten hinzufügen (Strg+D)
md-star-remove = Aus Favoriten entfernen (Strg+D)

# --- Fahrt planen ---
wiz-title = Fahrt planen
wiz-step-where = Wo
wiz-step-what = Was
wiz-step-when = Wann
wiz-step-review = Prüfen
wiz-back = Zurück
wiz-next = Weiter
wiz-skip = Zur Übersicht
wiz-drive = Losfahren
wiz-change = Ändern
wiz-open-settings = Einstellungen öffnen
wiz-reason-no-map = Wähle zuerst eine Karte
wiz-reason-no-start = Wähle zuerst einen Startpunkt
wiz-reason-no-vehicle = Wähle zuerst ein Fahrzeug
wiz-save-failed = Deine Auswahl konnte nicht nach { $path } gespeichert werden. Die Fahrt startet trotzdem.
wiz-date = { $day }. { $month } { $year }

wiz-where-search = { $count } Karten durchsuchen…
wiz-where-empty = Keine Karten gefunden. Prüfe den OMSI-Ordner oder suche neu.
wiz-where-start = Startpunkt
map-config-open = Kartenkonfiguration
wiz-where-date = Datum
wiz-where-no-starts = Diese Karte hat keine benannten Startpunkte.
wiz-where-description = Beschreibung der Karte, in ihrer Originalsprache
wiz-where-remembered-missing = Deine letzte Karte ist nicht mehr installiert.
wiz-fact-years = Spielt { $years }
wiz-fact-tiles =
    { $count ->
        [one] { $count } Kachel
       *[other] { $count } Kacheln
    }
wiz-fact-starts =
    { $count ->
        [one] { $count } Startpunkt
       *[other] { $count } Startpunkte
    }
wiz-fact-depot = Betriebshof: { $depot }

wiz-what-search = { $count } Fahrzeuge durchsuchen…
wiz-what-none = Ohne Fahrzeug fahren
wiz-what-none-sub = Nur die Karte, freie Kamera
wiz-what-none-body = Starten Sie auf der Karte mit freier Kamera und ohne Instrumente. Die Simulation läuft. Ein Fahrzeug können Sie jederzeit im Pause-Menü unter Fahrzeug setzen.
wiz-what-empty = Keine Fahrzeuge gefunden. Prüfe den OMSI-Ordner oder suche neu.
wiz-class-road = Straßenfahrzeug
wiz-class-rail = Schienenfahrzeug
wiz-class-aircraft = Luftfahrzeug
wiz-fact-mass = Leergewicht: { $tonnes } t
wiz-what-livery = Lackierung
wiz-what-livery-default = Standard
wiz-what-livery-loading = Lackierungen werden gelesen…
wiz-what-livery-none = Dieses Fahrzeug hat keine weiteren Lackierungen.
wiz-what-configure = Konfigurieren…
wiz-what-no-options = Dieses Fahrzeug hat keine Ausstattungsoptionen.
wiz-what-options-changed =
    { $count ->
        [one] 1 Option geändert
       *[other] { $count } Optionen geändert
    }
wiz-what-remembered-missing = Dein letztes Fahrzeug ist nicht mehr installiert.
wiz-options-title = Ausstattung
wiz-options-reset = Zurücksetzen
wiz-options-done = Fertig

wiz-when-quick = Tageszeit
wiz-when-exact = Uhrzeit
wiz-when-weather = Wetter
wiz-when-now = Jetzt
wiz-when-morning = Morgen
wiz-when-noon = Mittag
wiz-when-evening = Abend
wiz-when-night = Nacht
wiz-when-day = Tag
wiz-when-month = Monat
wiz-when-year = Jahr
wiz-when-hour = Stunde
wiz-when-minute = Minute
wiz-step-less = Verringern
wiz-step-more = Erhöhen
wiz-when-era-hint = Diese Karte spielt { $from }–{ $to }.
wiz-when-source-map = Vorgabe der Karte
wiz-when-source-now = Jetzt
wiz-month-1 = Januar
wiz-month-2 = Februar
wiz-month-3 = März
wiz-month-4 = April
wiz-month-5 = Mai
wiz-month-6 = Juni
wiz-month-7 = Juli
wiz-month-8 = August
wiz-month-9 = September
wiz-month-10 = Oktober
wiz-month-11 = November
wiz-month-12 = Dezember
wiz-weather-clear = Klarer Tag
wiz-weather-clear-effect = Klarer Himmel, volles Sonnenlicht
wiz-weather-overcast = Bedeckt
wiz-weather-overcast-effect = Wolkendecke, weicheres Licht
wiz-weather-rain = Regen
wiz-weather-rain-effect = Regen, dunkleres diffuses Licht
wiz-weather-snow = Schnee
wiz-weather-snow-effect = Schneefall, dunkleres diffuses Licht, Schnee auf dem Boden
wiz-weather-fog = Nebel
wiz-weather-fog-effect = Nebel, dunkleres diffuses Licht
wiz-weather-fx-clouds = Einzelne Wolken
wiz-weather-fx-overcast = Bedeckt
wiz-weather-fx-clear = Klarer Himmel
wiz-weather-fx-rain = Regen
wiz-weather-fx-snow = Schnee
wiz-weather-fx-fog = Nebel, { $m } m Sicht
wiz-weather-fx-wind = Wind: { $speed } m/s
wiz-weather-fx-line = { $temp } °C – { $effects }
wiz-weather-live = Live-Wetter

wiz-review-title = Bereit zur Fahrt?

wiz-option-vis_wimpel_invisible = Wimpel vorn
wiz-option-wimpel_visibility = Fahnenhalterung
wiz-option-vis_grill_invisible = Grillabdeckung
wiz-option-ident_visible = Betreiberkennzeichnung

# --- Ladebildschirm ---
load-title = Deine Fahrt wird vorbereitet
load-tile-map = KARTE
wiz-no-vehicle = Kein Fahrzeug
load-no-vehicle-sub = Freie Kamera
load-tile-vehicle = FAHRZEUG
load-tile-when = ZEIT UND WETTER
load-stage-catalogue = Dein OMSI-Inhalt wird gelesen (nur beim ersten Start)…
load-stage-resolve = Karte und Fahrzeug werden gesucht…
load-stage-map = Karte wird geladen: { $done } von { $total } Kacheln
load-stage-light = Licht und Wetter werden eingerichtet…
load-stage-vehicle = Fahrzeug wird vorbereitet…
load-stage-start = Fahrt wird startklar gemacht…
load-stage-ready = Bereit
load-stage-failed = Laden abgebrochen
load-cancel = Abbrechen
load-cancel-title = Laden abbrechen?
load-cancel-body = Du kehrst zur Fahrtplanung zurück. Nichts geht verloren; deine Auswahl bleibt.
load-cancel-keep = Weiter laden
load-cancel-yes = Laden abbrechen
load-ready-problems =
    { $count ->
        [one] Bereit mit 1 Problem
       *[other] Bereit mit { $count } Problemen
    }
load-failed = Die Fahrt konnte nicht geladen werden
load-details = Details
load-continue = Weiter
load-retry = Erneut versuchen
load-choose-another = Andere Auswahl
load-back = Zurück
load-problem-map = Die Karte { $name } konnte nicht geladen werden
load-problem-vehicle = Das Fahrzeug { $name } konnte nicht geladen werden
load-problem-no-model = Das Fahrzeug { $name } hat keine Modelldatei in deinem OMSI-Ordner
load-problem-tiles = { $count } von { $total } Kartenkacheln fehlen
load-problem-objects = { $count } Szenerieobjekte konnten nicht gebaut werden
load-problem-files = { $count } Dateien konnten nicht gelesen werden
load-cause-map-unreadable = Der Kartenordner oder seine Hauptdatei konnte nicht gelesen werden.
load-cause-no-tiles = Keine der Kacheln der Karte konnte gelesen werden.
load-cause-no-vehicle-file = Stattdessen wird ein Fahrzeug aus den Listen der Karte verwendet.
load-cause-vehicle-unreadable = Die Modelldatei fehlt oder ist nicht lesbar.
load-controls-title = STEUERUNG
load-controls-accelerate = Beschleunigen
load-controls-brake = Bremsen
load-controls-steer = Lenken
load-controls-parking = Feststellbremse
load-controls-gears = Fahren, Neutral, Rückwärts
load-controls-cameras = Kameraansichten
load-controls-pause = Pause
load-controls-menu = Menü
load-minor-toast =
    { $count ->
        [one] Mit 1 kleinen Problem geladen
       *[other] Mit { $count } kleinen Problemen geladen
    }
load-minor-title = Kleine Ladeprobleme
load-close = Schließen
wiz-configs-title = Konfigurationen
wiz-configs-name = Name
wiz-configs-save = Als Konfiguration speichern
wiz-configs-empty = Noch keine Konfigurationen. Speichere Fahrzeug, Lackierung und Optionen, um sie später in einem Schritt zu laden.
wiz-configs-favourite = Merken
wiz-configs-unfavourite = Nicht mehr merken
wiz-configs-duplicate = Duplizieren
wiz-configs-delete = Löschen
wiz-configs-delete-confirm = Löschen? Nochmal drücken
wiz-configs-copy-suffix = (Kopie)
wiz-configs-saved = Konfiguration „{ $name }“ gespeichert.
wiz-configs-loaded = Konfiguration „{ $name }“ geladen.
wiz-configs-loaded-partly =
    { $count ->
        [one] Konfiguration „{ $name }“ geladen; 1 Teil ist nicht mehr verfügbar und wurde ausgelassen.
       *[other] Konfiguration „{ $name }“ geladen; { $count } Teile sind nicht mehr verfügbar und wurden ausgelassen.
    }
wiz-configs-load-failed = Konfiguration „{ $name }“ kann nicht geladen werden: Das Fahrzeug ist nicht installiert.
wiz-configs-save-failed = Die Konfigurationen konnten nicht gespeichert werden: { $reason }
wiz-configs-file-problem = Die Konfigurationsdatei ist nicht nutzbar: { $reason }
wiz-configs-vehicle-missing = Das Fahrzeug ist nicht installiert.
wiz-configs-repaint-missing = Die Lackierung „{ $name }“ wird nicht mehr angeboten; die Standardlackierung wird verwendet.
wiz-configs-option-left-out = Die Option „{ $name }“ wird nicht mehr akzeptiert und ausgelassen.
