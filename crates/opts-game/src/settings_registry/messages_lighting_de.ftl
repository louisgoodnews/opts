# Lichtqualität (Abschnitt der Grafikseite, der Automatikmodus und die Zeile der Umgebungsseite).

set-light-section = Lichtqualität
set-light-section-note = Lichtstrahlen, Lichtkegel, Spiegel, Spiegelungen und Bloom. Im Automatikmodus senkt OPTS diese Stufe für Stufe, wenn die Bildrate sinkt, und erhöht sie wieder, wenn Spielraum da ist.
set-light-auto-note = Automatisch, Stufe { $level } ({ $name }): Die Regler unten zeigen, was gerade gilt. Wähle Benutzerdefiniert, um sie selbst einzustellen.
set-light-auto-disabled = Das entscheidet der Automatikmodus. Stelle den Modus der Lichtqualität auf Benutzerdefiniert, um es zu ändern.
set-light-level-0 = voll
set-light-level-1 = weniger Kegel
set-light-level-2 = nur Scheinwerfer
set-light-level-3 = Kegel aus
set-light-level-4 = nur Sonnenstrahlen
set-light-level-5 = Volumetrik aus

set-light-mode = Modus der Lichtqualität
set-light-mode-desc = Automatisch: OPTS wählt die Lichtstufe anhand der gemessenen Bildzeit. Benutzerdefiniert: Du wählst jeden Effekt selbst, nichts ändert sich von allein. Beim Wechsel zu Benutzerdefiniert bleibt die erreichte Stufe als Ausgangswert; zurück zu Automatisch beginnt wieder bei der vollsten Stufe.
set-opt-lightmode-automatic = Automatisch
set-opt-lightmode-custom = Benutzerdefiniert

set-light-shafts = Sonnenstrahlen
set-light-shafts-desc = Lichtstrahlen in der Luft (volumetrischer Nebel). Die Qualität ist die Zahl der Schritte je Strahl: niedrig 8, mittel 16, hoch 32, Ultra 64. Aus zeichnet keine und kostet nichts.
set-opt-shafts-off = Aus
set-opt-shafts-low = Niedrig (8)
set-opt-shafts-medium = Mittel (16)
set-opt-shafts-high = Hoch (32)
set-opt-shafts-ultra = Ultra (64)

set-light-night-shafts = Strahlen bei Nacht
set-light-night-shafts-desc = Zeichnet Lichtstrahlen auch nachts, beleuchtet vom Mond. Kostet nachts mehr wegen der vielen Lampen.
set-light-lantern-cones = Kegel von Straßenlaternen
set-light-lantern-cones-desc = Sichtbare Lichtkegel unter Straßenlaternen und anderen Leuchten.
set-light-headlight-cones = Kegel von Scheinwerfern
set-light-headlight-cones-desc = Sichtbare Lichtkegel vor den Scheinwerfern von Fahrzeugen.
set-light-cone-cap = Höchstzahl Lichtkegel
set-light-cone-cap-desc = Wie viele Lichtkegel höchstens gleichzeitig gezeichnet werden, von 0 bis 96. Die nächsten bleiben erhalten.
set-light-cone-depth-fade = Weiche Kegelränder
set-light-cone-depth-fade-desc = Kegel blenden dort aus, wo sie auf Wände und Boden treffen, statt sie zu durchschneiden. Braucht einen zusätzlichen Tiefendurchgang und kostet Bildzeit.
set-mirror-count = Spiegel
set-mirror-count-desc = Wie viele Spiegel einen Live-Blick nach hinten zeigen, 0 bis 2. Jeder zeichnet die Szene noch einmal.
set-mirror-rate = Aktualisierung der Spiegel
set-mirror-rate-desc = Wie oft ein Spiegel gezeichnet wird: jedes Bild, jedes 2. oder jedes 4. Bild. Seltener ist günstiger.
set-opt-mirror-rate-1 = Jedes Bild
set-opt-mirror-rate-2 = Jedes 2. Bild
set-opt-mirror-rate-4 = Jedes 4. Bild
set-glass-reflections = Himmel im Glas
set-glass-reflections-desc = Fenster und anderes Glas spiegeln den Himmel.
set-bloom = Bloom
set-bloom-desc = Ein weicher Schein um sehr helle Lichter.

env-light-quality = Lichtqualität
env-light-automatic = Automatisch (Stufe { $level }: { $name })
env-light-custom = Benutzerdefiniert
