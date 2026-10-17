# Einstellungen: Kategorien, Zeilen, Optionen.
settings-cat-graphics = Grafik
settings-cat-audio = Audio
settings-cat-controls = Steuerung
settings-cat-interface = Oberfläche
settings-cat-gameplay = Spielablauf
settings-cat-content = Inhalte
settings-cat-advanced = Erweitert

set-window-mode = Fenstermodus
set-window-mode-desc = Fenster, rahmenlos oder Vollbild. Eine Änderung musst du innerhalb von 15 Sekunden bestätigen.
set-opt-window-windowed = Fenster
set-opt-window-borderless = Rahmenlos
set-opt-window-fullscreen = Vollbild
set-resolution = Fenstergröße
set-resolution-desc = Größe des Fensters in Pixeln. Eine Änderung musst du innerhalb von 15 Sekunden bestätigen.
set-opt-res-1024x640 = 1024 × 640
set-opt-res-1280x720 = 1280 × 720
set-opt-res-1600x900 = 1600 × 900
set-opt-res-1920x1080 = 1920 × 1080
set-opt-res-2560x1440 = 2560 × 1440
set-opt-res-3840x2160 = 3840 × 2160
set-vsync = Vertikale Synchronisation
set-vsync-desc = Passt die Bildrate an deinen Bildschirm an. Verhindert Risse im Bild, kann etwas Verzögerung bringen.
set-texture-filtering = Texturfilterung
set-texture-filtering-desc = Schärfere Texturen im flachen Winkel, geringe Kosten.
set-opt-filter-off = Aus
set-opt-filter-4 = 4×
set-opt-filter-8 = 8×
set-opt-filter-16 = 16×
set-look = Look
set-look-desc = Farbe und Kontrast der 3D-Welt.
set-default-fov = Fahrersicht: Sichtfeld
set-default-fov-desc = Das horizontale Sichtfeld der Fahrersichten in Grad (30 bis 120). 0 behält das eigene des Fahrzeugs. Ein gespeicherter Blickpunkt mit eigenem Sichtfeld hat Vorrang.
set-opt-look-vivid = Lebhaft
set-opt-look-natural = Natürlich
set-opt-look-bevy = Bevy-Standard
set-view-distance = Sichtweite
set-view-distance-desc = Wie weit Szenerie und Kacheln gezeichnet werden.
set-water = Wasser
set-water-desc = Zeichnet Flüsse, Seen und das Meer der Karte. Ausschalten, um den Grund darunter zu sehen.

set-audio-master = Gesamtlautstärke
set-audio-master-desc = Lautstärke des gesamten Simulators.
set-audio-engine = Motorlautstärke
set-audio-engine-desc = Lautstärke von Motor und Antrieb deines Fahrzeugs.
set-audio-ambience = Umgebungslautstärke
set-audio-ambience-desc = Verkehr, Wetter und Stadtgeräusche.
set-audio-interface = Lautstärke der Oberfläche
set-audio-interface-desc = Klicks und Hinweistöne.

set-rebind = Taste oder Knopf ändern
set-rebind-desc = Eigene Tasten, Gamepad-Knöpfe und Lenkrad-Steuerung festlegen.

set-theme-mode = Farbschema
set-theme-mode-desc = Dunkel ist die Voreinstellung. Auto wechselt nach der Uhrzeit, System folgt deinem Desktop.
set-auto-light-from = Helles Schema ab
set-auto-light-from-desc = Uhrzeit, ab der im Auto-Modus das helle Schema gilt (HH:MM).
set-auto-dark-from = Dunkles Schema ab
set-auto-dark-from-desc = Uhrzeit, ab der im Auto-Modus das dunkle Schema gilt (HH:MM).
set-high-contrast = Hoher Kontrast
set-high-contrast-desc = Stärkere Farben und Umrandungen. Überschreibt das Farbschema und wird nie automatisch umgeschaltet.
set-accent = Akzentfarbe
set-accent-desc = Die Farbe von Auswahl und Hauptschaltflächen.
set-theme-file = Themendatei
set-theme-file-desc = Name einer Themendatei im Ordner themes deines Konfigurationsordners; leer verwendet das eingebaute Schema.
set-ui-scale = Oberflächengröße
set-ui-scale-desc = Skaliert Text und Bedienelemente, in Prozent.
set-density = Dichte
set-density-desc = Wie viel Platz die Oberfläche zwischen den Elementen lässt.
set-reduced-motion = Bewegung reduzieren
set-reduced-motion-desc = Übergänge und Animationen erfolgen sofort.
set-language = Sprache
set-language-desc = Die Sprache von Menüs und Meldungen. Inhalte aus OMSI behalten ihre eigene Sprache.
set-language-reload = Sprachen neu laden
set-language-reload-desc = Sucht den Sprachordner erneut, damit ein neues oder geändertes Sprachpaket ohne Neustart erscheint.
set-lang-info = { $percent } % übersetzt, von { $authors }
set-lang-info-unsigned = { $percent } % übersetzt
set-lang-gaps =
    { $count ->
        [one] Ein Text ist noch nicht übersetzt und erscheint auf Englisch.
       *[other] { $count } Texte sind noch nicht übersetzt und erscheinen auf Englisch.
    }
set-lang-outdated = Für eine ältere Fassung der Texte erstellt.
set-lang-problems =
    { $count ->
        [one] Ein Problem in diesem Paket; der fehlerhafte Text erscheint auf Englisch.
       *[other] { $count } Probleme in diesem Paket; die fehlerhaften Texte erscheinen auf Englisch.
    }
set-lang-rtl = Schrift von rechts nach links wird noch nicht unterstützt; diese Sprache lässt sich daher nicht wählen.
set-lang-fonts = Der mitgelieferten Schrift fehlt { $scripts }; diese Zeichen fehlen in der Anzeige.
set-lang-folder = Für eine weitere Sprache einen Ordner mit pack.json und Textdateien in { $path } ablegen.
settings-toast-languages-reloaded =
    { $count ->
        [0] Keine Sprachpakete gefunden.
        [one] Ein Sprachpaket geladen.
       *[other] { $count } Sprachpakete geladen.
    }
settings-toast-languages-problems =
    { $count ->
        [one] Ein Problem gefunden; siehe Protokoll oder den Befehl l10n.report.
       *[other] { $count } Probleme gefunden; siehe Protokoll oder den Befehl l10n.report.
    }
set-units = Einheiten
set-units-desc = Geschwindigkeiten werden in Kilometern oder Meilen pro Stunde angezeigt.
set-opt-units-kmh = km/h
set-opt-units-mph = mph
set-hud-hints = Hinweise anzeigen
set-hud-hints-desc = Kurze Tipps neben Bedienelementen und im Fahrerstand.
set-hud-speed = Geschwindigkeit
set-hud-speed-desc = Die Geschwindigkeitsanzeige zeigen.
set-hud-gear = Gang
set-hud-gear-desc = Den gewählten Gang zeigen.
set-hud-rpm = Motordrehzahl
set-hud-rpm-desc = Die Drehzahl zeigen, wenn die Simulation sie liefert.
set-hud-parking-brake = Feststellbremse
set-hud-parking-brake-desc = Den Zustand der Feststellbremse zeigen.
set-hud-session = Karte und Startpunkt
set-hud-session-desc = Zeigen, wo die Fahrt stattfindet; blendet nach einigen Sekunden aus.
set-hud-clock = Uhr und Wetter
set-hud-clock-desc = Die Uhrzeit der Fahrt und das Wetter zeigen; blendet nach einigen Sekunden aus.
set-hud-camera-label = Kameraname
set-hud-camera-label-desc = Den Namen der Kamera kurz nach einem Wechsel zeigen.

set-traffic = Straßenverkehr
set-traffic-desc = Höchstzahl gleichzeitig fahrender KI-Fahrzeuge (0 für keine). Gilt für die nächste Fahrt.
set-pedestrians = Fußgänger
set-pedestrians-desc = Höchstzahl gleichzeitig laufender Personen (0 für keine). Gilt für die nächste Fahrt.
set-time-scale = Zeitraffer
set-time-scale-desc = Lässt die Uhr der Fahrt schneller als in Echtzeit laufen.

set-omsi-path = OMSI-2-Ordner
set-omsi-path-desc = Der Ordner, in dem dein OMSI 2 installiert ist. Karten und Fahrzeuge werden von hier gelesen, nie verändert.
set-discovery-interval = Neue Inhalte suchen alle
set-discovery-interval-desc = Tage zwischen automatischen Suchläufen im OMSI-Ordner beim Start.
set-rescan = Jetzt suchen
set-rescan-desc = Sucht im Hintergrund nach neuen oder geänderten Karten und Fahrzeugen.
set-recent-map = Letzte Karte
set-recent-map-desc = Die Karte, die du zuletzt gefahren bist.

set-developer-panel = Entwicklerfeld
set-developer-panel-desc = Zeigt Statistiken und Fehlersuche-Informationen in der Fahransicht (Taste F10).
set-log-level = Protokolldetails
set-log-level-desc = Wie viel das Spiel in seine Protokolldatei schreibt. Mehr Details helfen bei Fehlerberichten.
set-opt-log-error = Nur Fehler
set-opt-log-warn = Warnungen
set-opt-log-info = Normal
set-opt-log-debug = Ausführlich
set-trace-scene = Szenen-Trace
set-trace-scene-desc = Entwicklerwerkzeug: protokolliert in einer Datei neben den Logs, welche Mesh-Datei wo steht (Kachel, Position, Dekodierqualität). Zusammenfassung hält Zähler und Probleme fest, Alles hält jedes Objekt fest. Wirkt ab dem nächsten Laden einer Karte. Kostet nichts, solange es aus ist.
set-opt-trace-off = Aus
set-opt-trace-summary = Zusammenfassung
set-opt-trace-full = Alles
set-trace-overlay = Szenen-Trace-Überlagerung
set-trace-overlay-desc = Entwicklerwerkzeug: zeichnet um jedes verfolgte Objekt in Kameranähe einen Kasten, eingefärbt nach Dekodierqualität (rot = fehlgeschlagen, orange = unsicher, gelb = dekodiert). Braucht den Szenen-Trace.

settings-title = Einstellungen
settings-search-placeholder = Einstellungen durchsuchen
settings-reset = Zurücksetzen
settings-reset-disabled = Schon auf dem Standardwert
settings-no-results = Keine Einstellung passt zu deiner Suche. Versuche ein anderes Wort.
settings-restart-note = Wirkt nach einem Neustart.
settings-needs = Braucht { $what }, was dieser Computer nicht hat.
settings-step-down = −
settings-step-up = +
settings-choose = Auswählen...
settings-path-none = Nicht festgelegt
settings-theme-file-builtin = Eingebaut
settings-keep-title = Diese Anzeigeeinstellungen behalten?
settings-keep-body = Zurück zu den alten Einstellungen in { $seconds } Sekunden.
settings-keep = Behalten
settings-revert = Zurück

settings-info-theme-now = Gerade wird das Farbschema „{ $theme }“ angezeigt.
settings-info-catalogue = Katalog: { $maps } Karten, { $vehicles } Fahrzeuge, { $scenery } Szenerieobjekte, { $splines } Splines und { $other } weitere Einträge (Anhänger und KI-Fahrzeuge, Schriften, Menschen und mehr).
settings-info-catalogue-total = Insgesamt { $entities } Einträge, aufgebaut aus { $files } Dateien. Eine Datei ist eine Datei auf der Festplatte; ein Eintrag ist eine Sache aus einer oder mehreren Dateien, etwa eine Karte oder ein Fahrzeug. Karten und Fahrzeuge werden gezählt wie auf der Startseite.
settings-info-last-scan = Letzte Suche: { $when }.
settings-info-never-scanned = Der Katalog wurde noch nicht durchsucht.
settings-info-catalogue-pending = Katalog: wird berechnet…
settings-info-catalogue-total-pending = Einträge und Dateien: wird berechnet…
settings-info-last-scan-pending = Letzte Suche: wird berechnet…
settings-info-locations = Wo deine Daten liegen
settings-info-settings-file = Deine Einstellungen werden gespeichert in: { $path }
settings-info-reference-file = Feste Ausgangswerte (nur lesbar): { $path }
settings-info-catalogue-file = Der Katalog deiner OMSI-Inhalte: { $path }
settings-info-themes-folder = Deine Themendateien gehören in: { $path }
settings-info-last-map = Die Karte, die du zuletzt gefahren bist: { $path }
settings-keys-title = Tasten und Knöpfe in den Menüs
settings-key-navigate = Zwischen Bedienelementen wechseln: Pfeiltasten oder Steuerkreuz.
settings-key-regions = Zum nächsten oder vorigen Bereich springen: Tab oder Umschalt+Tab.
settings-key-confirm = Gewähltes Element benutzen: Eingabe oder Leertaste, A am Gamepad.
settings-key-back = Zurück oder schließen: Escape, Rücktaste, B am Gamepad.
settings-key-search = Suchen: Strg+F oder /, Y am Gamepad.
settings-key-tabs = Nächster oder vorheriger Reiter: ] und [, Strg+Tab, Schultertasten am Gamepad.
settings-key-scroll = Scrollen: Bild auf, Bild ab, Pos1 und Ende.
settings-key-menu = Menü öffnen: Start am Gamepad.
settings-key-developer = Entwicklerfeld in einer Fahrt: F10 (wenn unter Erweitert eingeschaltet).
settings-toast-rescan-started = Neue Inhalte werden im Hintergrund gesucht.
settings-toast-no-omsi = Es ist kein OMSI-2-Ordner festgelegt, also gibt es nichts zu durchsuchen.
settings-toast-omsi-restart = OMSI-Ordner gespeichert. Starte OPTS neu, um seine Inhalte zu laden.
settings-toast-omsi-invalid = Dieser Ordner sieht nicht nach einer OMSI-2-Installation aus (er braucht die Ordner maps und Vehicles).

settings-back = Zurück
set-autosave = Automatisch speichern
set-autosave-desc = Speichert die Fahrt alle so viele Spielminuten im Slot für die automatische Speicherung. 0 schaltet es aus.


# Hardware (M-0081)
set-hw-redetect = Hardware neu erkennen
set-hw-redetect-desc = Prüft deinen Computer erneut und speichert das Ergebnis in einer Datei auf diesem Computer. Nichts wird irgendwohin gesendet.
set-hw-delete = Gespeichertes Hardware-Profil löschen
set-hw-delete-desc = Entfernt die Datei mit der erkannten Hardware von diesem Computer.
set-hw-reports = Hardware-Profil in Berichten erlauben
set-hw-reports-desc = Wenn an, enthält ein gespeicherter Fehlerbericht auch die unten gezeigte Hardware. Du kannst sie in der Berichtsdatei lesen, bevor du sie teilst. Standardmäßig aus.
settings-info-hw-title = Deine Hardware
settings-info-hw-privacy = OPTS prüft deinen Computer nur, um gute Startwerte zu wählen. Das Ergebnis bleibt auf diesem Computer. Es enthält keine Seriennummern, Namen oder Adressen, und nichts wird irgendwohin gesendet. Die vollständige Liste: opts-tools hardware report.
settings-info-hw-detecting = Hardware wird erkannt ...
settings-info-hw-stored = Eine Kopie liegt in: { $path }
settings-info-hw-not-stored = Auf diesem Computer ist keine Kopie gespeichert.
settings-info-hw-os = System: { $os }
settings-info-hw-cpu = Prozessor: { $vendor }, { $physical } Kerne, { $logical } Threads
settings-info-hw-memory = Speicher: { $available } von { $total } GB frei
settings-info-hw-gpu = Grafik: { $vendor }, { $class }, { $vram } GB Grafikspeicher, { $backend }
settings-info-hw-display = Bildschirm: { $width } × { $height }, { $count } angeschlossen
settings-info-hw-storage = Datenträger mit den Inhalten: { $class }, { $free } GB frei
settings-info-hw-recs = Für diesen Computer gewählte Startwerte: { $radius } Kacheln um dich, Texturfilter { $filter }, Look { $look }, Oberflächengröße { $scale } %
settings-info-hw-dlss = DLSS: { $dlss }
settings-toast-hw-detecting = Hardware wird erkannt. Die Seite aktualisiert sich danach.
settings-toast-hw-deleted = Das gespeicherte Hardware-Profil wurde gelöscht.
settings-toast-hw-nothing-stored = Es war kein Hardware-Profil gespeichert.
hw-reason-not_probed = noch nicht erkannt
hw-reason-not_on_this_platform = auf diesem System nicht verfügbar
hw-reason-needs_renderer = bekannt, sobald das Spielfenster offen ist
hw-reason-unavailable = auf diesem Rechner nicht verfügbar
hw-reason-unreadable = konnte nicht gelesen werden
hw-reason-unparsed = nicht verstanden
hw-reason-timeout = übersprungen, es dauerte zu lange
hw-reason-not_requested = nicht gemessen
hw-reason-cancelled = abgebrochen
hw-dlss-yes = möglich
hw-dlss-unknown = noch nicht bekannt
hw-dlss-no-not_nvidia = nicht möglich, es braucht eine NVIDIA-Grafikkarte
hw-dlss-no-os_unsupported = auf diesem Betriebssystem nicht möglich
hw-dlss-no-backend_unsupported = mit dieser Grafikschnittstelle nicht möglich
hw-dlss-no-no_ray_tracing = nicht möglich, die Karte hat kein Raytracing
hw-dlss-no-no_runtime = nicht möglich, die DLSS-Dateien fehlen

scan-never = nie
scan-chip = { $status } · Letzte Suche: { $when }
scan-chip-pending = Wird geprüft…
scan-why-pending = Der OMSI-Ordner und der Katalog werden im Hintergrund geprüft. Das dauert einen Moment.
scan-level-good = Aktuell
scan-level-recommended = Suche empfohlen
scan-level-needed = Suche nötig
scan-why-recent = Vor Kurzem durchsucht, und der OMSI-Ordner hat sich seitdem nicht geändert. Nichts zu tun.
scan-why-older = Die letzte Suche ist älter als dein automatisches Suchintervall ({ $days } Tag(e)). Mit „Jetzt suchen“ werden neue Inhalte gefunden.
scan-why-changed = Der OMSI-Ordner wurde nach der letzten Suche verändert, vermutlich gibt es neue oder entfernte Inhalte. Drücke „Jetzt suchen“.
scan-why-running = Im Hintergrund läuft eine Suche. Die Zahlen unten aktualisieren sich danach.
scan-why-never = Der OMSI-Ordner wurde noch nicht durchsucht. Drücke „Jetzt suchen“.
scan-why-failed = Die letzte Suche ist fehlgeschlagen. Prüfe oben den OMSI-Ordner und drücke dann „Jetzt suchen“.
scan-why-empty = Die letzte Suche hat keine Karten oder Fahrzeuge gefunden. Prüfe oben den OMSI-Ordner und drücke dann „Jetzt suchen“.
scan-why-folder-missing = Der OMSI-Ordner ist nicht festgelegt oder nicht auffindbar. Wähle ihn oben aus und drücke dann „Jetzt suchen“.
set-hud-minimap = Minikarte
set-hud-minimap-desc = Die Karte der Straßen um dich unten rechts zeigen (F7).
set-hud-minimap-zoom = Maßstab der Minikarte
set-hud-minimap-zoom-desc = Wie weit die Minikarte um dich herum reicht. Die Tasten + und - ändern ihn während der Fahrt.
set-opt-minimap-zoom-100 = 100 Meter
set-opt-minimap-zoom-200 = 200 Meter
set-opt-minimap-zoom-500 = 500 Meter
set-opt-minimap-zoom-1000 = 1 Kilometer
set-hud-minimap-zoom-mode = Zoom der Minikarte
set-hud-minimap-zoom-mode-desc = Mit Geschwindigkeitszoom zeigt die Karte langsam oder im Stand einen kleinen Ausschnitt und schnell den eingestellten Maßstab (Strg+F7). „Fest“ behält den Maßstab.
set-opt-minimap-zoom-mode-speed = Der Geschwindigkeit folgen
set-opt-minimap-zoom-mode-fixed = Fest
set-hud-minimap-orientation = Richtung der Minikarte
set-hud-minimap-orientation-desc = „Fahrtrichtung oben“ dreht die Karte mit; „Norden oben“ hält Norden oben (Umschalt+F7).
set-opt-minimap-heading = Fahrtrichtung oben
set-opt-minimap-north = Norden oben
