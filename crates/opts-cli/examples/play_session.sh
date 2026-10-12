#!/usr/bin/env bash
# An end-to-end session through the control protocol, the way an AI agent plays OPTS without a human:
# Home -> plan a drive (wizard) -> loading -> drive 100 m -> scroll and change settings -> pause -> leave.
#
#   opts-game --omsi-path "/path/to/OMSI 2" --cli-port 44201 &
#   crates/opts-cli/examples/play_session.sh
#
# Needs `opts-cli` on PATH (or OPTS_CLI) and `jq`. Every step waits for a condition (`ui.wait_for`)
# instead of sleeping, and reads the answer back. The ids are the stable ones of the screens; list
# what is on screen with `ui.screen` (dialogs and buttons) and `ui.tree` (everything).
set -euo pipefail
# OPTS_CLI_PORT selects another port than the default 44201 (the `--cli-port 0` port is in the game's log).
CLI=("${OPTS_CLI:-$(command -v opts-cli || echo ./target/debug/opts-cli)}" ${OPTS_CLI_PORT:+--port "$OPTS_CLI_PORT"})
MAP="${MAP:-berlin-spandau}"          # entry id or title from `ui.scrollables` (wizard.where.list)
VEHICLE="${VEHICLE:-MAN SD200 - SD77}"
c() { "${CLI[@]}" "$@"; }
j() { "${CLI[@]}" --json "$@" | jq -r "$JQ"; }

c ping >/dev/null || { echo "no OPTS control server on port ${OPTS_CLI_PORT:-44201}" >&2; exit 2; }
c version | grep protocol_version

echo "== Home"
# A fresh configuration scans the OMSI folder first (about 8 s) and Home says "43 maps" when it is done.
# Answering the first-run question before that can leave the wizard's lists empty (an app limitation,
# see the architecture page), so wait until Home shows the counts.
for _ in $(seq 1 120); do
  "${CLI[@]}" --json ui.tree --visible_only true | jq -e '[.. | objects | .label? // empty | select(test("[0-9]+ maps"))] | length > 0' >/dev/null && break
  sleep 1
done
JQ='.result.screen + " / " + .result.app_state' j ui.screen
# The first-run dialog (a fresh configuration) is a dialog like any other: answer it by id.
if [ "$(JQ='.result.dialogs[0].tag // ""' j ui.screen)" = "first-run" ]; then
  c ui.click dialog.first-run.pick >/dev/null     # leads straight into the wizard
  c ui.wait_for screen --value drive >/dev/null
else
  c ui.click home.new-drive >/dev/null
  c ui.wait_for screen --value drive >/dev/null
fi

echo "== Plan a drive (Where, What, When, Review)"
c ui.wait_for visible wizard.where.list >/dev/null
c ui.wait_for items_above wizard.where.list --number 0 --timeout_ms 20000 >/dev/null
c ui.scroll --target wizard.where.list --item "$MAP" >/dev/null   # reveals the entry in the windowed list
c ui.select wizard.where.list "$MAP"
c ui.click wizard.footer.next >/dev/null
c ui.wait_for visible wizard.what.list >/dev/null
c ui.wait_for items_above wizard.what.list --number 0 >/dev/null
c ui.select wizard.what.list "$VEHICLE"
c ui.click wizard.footer.next >/dev/null                  # What -> When
c ui.click wizard.footer.next >/dev/null                  # When -> Review
c ui.wait_for visible wizard.footer.drive >/dev/null
JQ='.result.screen' j ui.screen
c ui.click wizard.footer.drive >/dev/null

echo "== Loading"
JQ='"stage " + (.result.loading.stage // "-")' j ui.screen
c --timeout-ms 130000 ui.wait_for hud_ready --timeout_ms 120000 >/dev/null
c snapshot --screenshot true >/dev/null

echo "== Drive 100 m"
c drive.set gear drive >/dev/null
c drive.set parking_brake false >/dev/null
c drive.set throttle 0.7 >/dev/null
c --timeout-ms 100000 ui.wait_for distance_above --number 100 --timeout_ms 90000 >/dev/null
JQ='"speed " + (.result.speed_kmh|tostring) + " km/h, driven " + (.result.distance_m|tostring) + " m"' j drive.state
c drive.set throttle 0 >/dev/null
c drive.set brake 1 >/dev/null
c --timeout-ms 60000 ui.wait_for speed_below --number 1 --timeout_ms 40000 >/dev/null
c drive.release >/dev/null

echo "== Pause and settings"
c ui.key Escape >/dev/null                                 # the pause overlay (the same gate as the keyboard)
c ui.wait_for visible pause.settings >/dev/null
c ui.click pause.settings >/dev/null
c ui.wait_for visible settings/search >/dev/null
VIEW="$("${CLI[@]}" --json ui.scrollables --visible_only true | jq -r '[.result.scrollables[] | select(.max[1] > 0)][0].id')"
c ui.scroll --target "$VIEW" --to bottom >/dev/null
JQ='"scrolled to " + (.result.scrollables[] | select(.id == "'"$VIEW"'") | .offset[1]|tostring)' j ui.scrollables
c ui.click settings/graphics/texture_filtering/option-8 >/dev/null
JQ='"texture_filtering = " + .result.value' j settings.get texture_filtering
c ui.click settings-overlay.close >/dev/null

echo "== Leave"
c ui.wait_for visible pause.leave >/dev/null               # the pause overlay is back under the settings
c ui.click pause.leave >/dev/null
c ui.wait_for dialog --value pause.leave >/dev/null        # the confirmation (its tag)
c ui.click dialog.pause.leave.home >/dev/null
c ui.wait_for app_state --value MainMenu >/dev/null
JQ='.result.screen' j ui.screen
