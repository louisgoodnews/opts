# Engine Callbacks `(M.V.*)`: Inferred Signatures

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Inferred from call sites (2026-10-14); nothing verified against OMSI |
| **Applies to** | `.osc` scripts of vehicles ([`.osc`](osc.md)) |
| **Used by** | [Bus script integration strategy](../../../architecture/bus-script-integration.md), [vehicle scripts route](../../../architecture/vehicle-scripts-route.md) |
| **Evidence tags** | **[C]** counted over the installation, ❓ inferred |

A script calls an engine service with `(M.V.Name)`: the arguments are on the stacks before the call, the results after it. The shipped lists `program/callbacklist_*.txt` name 25 vehicle callbacks and 17 script-texture callbacks and give **no argument lists**. This page records what the scripts themselves reveal. A wrong arity is not a harmless mistake: an unimplemented callback that leaves its arguments on the stack misaligns every following operation of the script, so signatures matter even for services OPTS will only stub.

## Method

Over 4,913 `.osc` files (vehicles and add-ons) with **64,792** `(M.V.*)` calls of **39** distinct names [C] (`STDrawPixel`, `STCopyColor` and `STGetA` are in the lists but never called):

1. At every call site the contiguous run of pure pushes before the call was counted, then a stack was simulated to see how often an assumed argument count underflows. For every callback the assumed arity underflows in 0-2 % of the sites and one more argument in 12-50 %: a clean knee.
2. The first consumer after the call gives the result type (a `(S.$.x)` store or a `$` operator: string; a float store, an arithmetic operator or an `{if}`: float).
3. Cross-checks with shipped data: `.hof` files, `Fonts/*.oft` font names, `Money/*.cfg`, `TicketPacks/*.otp` and the `[scripttexture]` blocks of `model.cfg`.
4. Plausibility from the code around the call (names of the variables the result is stored in, values compared with).

**Confidence** rates the signature (arity, order, types), not what the host data means. *high* = arity knee and result type clear, consistent across many vehicle families; *medium* = signature clear, parts of the semantics open; *low* = at least one argument unexplained. The analysis scripts are not part of the repository; the numbers can be reproduced by a token scan with a conservative stack-depth analysis.

## Conventions

Floats and strings live on separate stacks, so only the order *within* a stack matters. Float arguments are listed bottom to top (the last is pushed last). `idx` = zero-based index, `-1` = not found where noted.

## The callbacks

Uses over all scripts; Buses = player `.bus` files that can reach the callback (1,492 `.bus` definitions in total).

| Callback | Uses | Buses | Float args | String args | Result | Conf. | Host data that would serve it |
| :--- | ---: | ---: | :--- | :--- | :--- | :--- | :--- |
| `GetFontIndex` | 6154 | 936 | – | name:str | idx:f | high | Fonts/*.oft registry: matches the [newfont] {name} line (269 of 326 distinct literal names resolve to a file in Fonts/; "default" is an engine fallback) |
| `GetTerminusString` | 4813 | 1055 | termIdx, col | – | str | high | .hof terminus table [addterminus*] (row index, string column 0..25) |
| `TextLength` | 4287 | 934 | font | text:str | width:f | high | Font metrics of the .oft (char advance table) |
| `GetTTBusstopIndex` | 3959 | 949 | – | – | idx:f | high | Active timetable / schedule: current stop position (0-based) |
| `GiveChangeCoin` | 3486 | 981 | coinIdx | – | – | high | Cash desk: spawn a coin / bill from the currency of the vehicle (Money/&lt;cur&gt;/&lt;cur&gt;.cfg: [coin] then [bill]) |
| `GetTTBusstopName` | 3292 | 479 | stopNo | – | str | high | Timetable stop name (map / schedule file), compared with hof idents |
| `GetBusstopString` | 2933 | 1007 | busstopIdx, col | – | str | high | .hof [addbusstop*] row, string column 0..8 |
| `NrSpecRandom` | 2688 | 905 | n | – | r:f | high | Seeded generator, keyed by n |
| `STTextOut` | 2576 | 928 | tex, x, y, font, a, b | text:str | – | low | Canvas + font |
| `GetDepotStringGlobal` | 2478 | 1042 | slot | – | str | high | .hof [global_strings] slot (0 announcement dir, 1 roller-blind dir, 2 side-sign dir, 3-5 numeric strings) |
| `GetTerminusIndex` | 2007 | 1010 | code | – | idx:f | high | .hof terminus table: code column -> row index |
| `GetTTBusstopCount` | 1986 | 347 | – | – | n:f | high | Active timetable |
| `GetTerminusCode` | 1970 | 1036 | termIdx | – | code:f | high | .hof terminus table: row -> code column |
| `GetTTTerminusIndex` | 1534 | 260 | – | – | termIdx:f | high | Active timetable destination resolved to hof terminus row |
| `GetRouteBusstopIdent` | 1518 | 918 | routeIdx, stopNo | – | ident:str | high | .hof [infosystem_busstop_list] (ident of the n-th stop) |
| `STUnlock` | 1481 | 936 | tex | – | – | high | CPU canvas of tex |
| `STLock` | 1465 | 936 | tex | – | – | high | CPU canvas of tex |
| `GetBusstopIndex` | 1421 | 1002 | – | ident:str | idx:f | high | .hof bus-stop table (Ident column -> row) |
| `GetTTLineString` | 1346 | 295 | – | – | str | high | Active timetable line string (e.g. "109", "91.06A") |
| `STSetColor` | 1310 | 928 | tex, A, R, G, B | – | – | medium | Canvas state (current draw colour) |
| `GetRouteIndex` | 1129 | 944 | line*100+course | – | routeIdx:f | high | .hof [infosystem_trip] list: route code (e.g. 13001) -> trip index |
| `STDrawRect` | 1097 | 904 | tex, x1, y1, x2, y2 | – | – | high | Canvas |
| `GetBusstopCount` | 1051 | 918 | routeIdx | – | n:f | high | .hof [infosystem_busstop_list] count of a route |
| `STFilter` | 1045 | 928 | tex | – | – | high | CPU canvas of tex |
| `STNewTex` | 980 | 936 | tex | – | – | high | [scripttexture] canvas of the part (declared size) |
| `GetTTDelay` | 828 | 879 | – | – | sec:f | high | Simulation clock minus timetable at the current stop |
| `GetTicketValue` | 755 | 939 | ticketIdx | – | price:f | high | TicketPacks/&lt;pack&gt;/*.otp [ticket] {value} |
| `GetTTBusstopDep` | 731 | 864 | stopNo | – | sec:f | high | Timetable departure time at stop |
| `GetTicketName` | 726 | 924 | ticketIdx | – | str | high | TicketPacks/*.otp [ticket] name or {displaystring} |
| `GetHumanCountOnSeat` | 699 | 110 | seatIdx | – | n:f | high | Passenger system: occupant of seat (passengercabin [passpos] order; chain-wide on articulated buses) |
| `GetHumanCountOnPathLink` | 607 | 757 | linkIdx | – | n:f | high | Passenger system: humans currently on a pathlink/pathpoint of the cabin graph |
| `STReadPixel` | 586 | 889 | tex, x, y | – | – | medium | Canvas |
| `STGetR` | 482 | 889 | tex | – | R:f | high | Canvas (last pixel read) |
| `GetTTBusstopArr` | 400 | 657 | stopNo | – | sec:f | high | Timetable arrival time at stop |
| `GetRouteTerminusIndex` | 323 | 944 | routeIdx | – | termIdx:f | high | .hof [infosystem_trip] line 3 (terminus code) resolved to terminus row |
| `STLoadTex` | 240 | 886 | tex | file:str | – | medium | Texture folder of the vehicle (paths start with ..\..\Anzeigen\ = Vehicles/Anzeigen) + canvas |
| `GetHeightAbovePoint` | 201 | 840 | x, y, z | – | h:f | medium | Ground / kerb height query in vehicle coordinates |
| `STGetG` | 104 | 284 | tex | – | G:f | high | Canvas (last pixel read) |
| `STGetB` | 104 | 284 | tex | – | B:f | high | Canvas (last pixel read) |

Notes that matter for implementation:

* **Timetable group** (`GetTT*`): no arguments except the stop number; they read the *active* timetable of the vehicle. Times are seconds of the day (scripts divide by 3600 and take `% 24`). `GetTTDelay` is positive when late (below -30 s the scripts pick "too early" textures, 240-3600 s "too late").
* **`.hof` group** (`GetTerminusString`, `GetBusstopString`, `GetRouteIndex`, `GetDepotStringGlobal`, …): `GetRouteIndex` takes `line × 100 + course` (route code `13001` = line 130, course 01); `GetTerminusString` takes `terminus index` then the string column (0-25); `GetBusstopString` takes the stop index then the column (0-8); `GetDepotStringGlobal` slot 0 is the map/hof name the scripts compare with literals, slots 3-5 are numeric strings fed to `$StrToFloat`. Layout of the data: [`.hof`](hof.md).
* **How a selection reaches the displays (verified on the MAN IBIS of the SD200 and NG272 and on the Hamburg flip-dot matrix of the HH109, M-0099).** A driver keys line, course and destination on the IBIS (`ibis_setmode_linie_kurs`, digits, `ibis_eingabe`; `ibis_setmode_route`; `ibis_setmode_ziel`): the line is entered as `line × 100 + suffix`, the route as the course, the destination as the terminus code, and the script then calls `GetRouteIndex(line × 100 + course)`, `GetRouteBusstopIdent`, `GetBusstopIndex`, `GetBusstopString`, `GetRouteTerminusIndex`, `GetTerminusCode` and `GetTerminusString(row, 0)` and writes its string variables (`IBIS_terminus_name`, `act_busstop`, `Haltestelle`, ...). The destination blind and matrix are not run by the IBIS: they take a *scheduled target* from the host, the float input `AI_Target_Index` (the terminus **row**), the strings `SetLineTo` (line text) and `Ident`, and the trigger `ai_scheduled_settarget`; they write `target_index_int` (the row they show) when they have moved. A display that is switching on or winding ignores the trigger (the MAN matrix and the Hamburg matrix test that their animation timer is zero), so the host offers the target again until `target_index_int` equals the row.
* **Timetable times (M-0099).** `GetTTBusstopArr(i)` and `GetTTBusstopDep(i)` give the scheduled times of stop `i` of the followed run in seconds after midnight (the profile's manual times, the rest interpolated; [timetable files](../timetable.md)); 0 beyond the stops and without a run. `GetTTBusstopName` is the stop's name in `Busstops.cfg` (the hof's stop idents spell it the same way), `GetTTTerminusIndex` the depot terminus row named by the trip's destination text, `GetTTDelay` the clock minus the arrival at the current stop.
* **`GiveChangeCoin`**: the index runs over the coins, then the bills, of the currency file of the vehicle (`Money/&lt;cur&gt;.cfg`); the same index is used by the different cash-desk triggers of a vehicle for the same coin.
* **`NrSpecRandom n`**: stable per vehicle and argument (a matrix display calls it every frame with the pixel number to choose dead pixels); returns a value in `[0, 1]`.
* **Passenger counts**: seat indices fit the `[passpos]` count of the cabin in 25 of 28 vehicles; path-link indices exceed the per-file `[pathlink]` count in 18 of 79, so they may be chain-wide or path-point indices ❓.

## The script-texture API (`ST*`, `GetFontIndex`, `TextLength`)

Scripts draw destination boards, IBIS and matrix displays into CPU canvases that the model binds to meshes.

* **Texture index.** Always the first float argument; it is the position of the `[scripttexture]` block (width, height) in the `model.cfg` of the part that runs the script (MAN NL/NG declares 2, Krüger buses 7). A mesh binds a canvas with `[useScriptTexture] n` or with `\S:n` as the name of a `[matl_transmap]` ❓ (two add-on families use model.cfg variants that were not matched).
* **Normal order.** `STNewTex` → `STLock` → draw (`STDrawRect`, `STTextOut`, `STReadPixel` + `STGetR/G/B`) → `STUnlock` → `STFilter` (in 888 of 1,045 `STFilter` uses it follows `STUnlock`). Per-frame refreshes lock and unlock without `STNewTex`. `STLoadTex` runs while the canvas is unlocked or just before `STLock`.
* **Colours** are 0-255 per channel with an alpha value, never a palette index; the argument order of `STSetColor` is `tex, A, R, G, B` (from one LED macro) ❓. Palettes are a script technique: a bitmap is loaded into a canvas and read back pixel by pixel.
* **Coordinates.** `STDrawRect` takes two corners (`x1 y1 x1+3 y1+3` draws a 4×4 LED); `STTextOut` takes `tex, x, y, font, a, b` and the text; `TextLength` centres text (`320 font text TextLength - 2 /`).
* **Fonts.** `GetFontIndex` takes the `[newfont]` name of an `.oft` (not the file name): 269 of 326 distinct literal names resolve to a file in `Fonts/`; 2,498 further call sites build the name at run time; `default` is an engine fallback. The result is a float handle for `TextLength` and `STTextOut`.
* **Texture files.** `STLoadTex` paths are relative to the vehicle's texture folder (many start with `..\..\Anzeigen\`, i.e. `Vehicles/Anzeigen/`).

## Least certain signatures and the experiment that settles each

| Callback | What is open | Experiment (needs OMSI or a recorded display state) |
| :--- | :--- | :--- |
| `STTextOut` args 5, 6 (`a`, `b`) | Pairs seen: `0 0` 1,414×, `1 0` 350×, `2 0` 292×, `2 <var>` 390×. Letter spacing, alignment, clip width? | Draw a fixed string on a 256×32 canvas for `a` in 0-3 and `b` in {0, 3, 10, 100, -1}; scan rows with `STReadPixel`/`STGetR` and compare the first and last lit column with `TextLength` |
| `STSetColor` | Argument order and alpha semantics from one macro | Draw a 4×4 rectangle with distinct values (A=17, R=90, G=140, B=200), read it back; repeat over an existing colour (blend or overwrite) and with text |
| `GetHeightAbovePoint` | Frame, sign, which surface counts, the no-hit value | Park beside a known 0.15 m kerb; log results for z in {-1, 0, 0.15, 0.5, 2} at points inside, near and far from the kerb, with and without kneeling |
| `STLoadTex` | Whether it consumes the file name or returns a status; scaling; base path | Compare string-stack depth around the call; load an existing, a missing and a differently sized file and read the corners |
| `GetHumanCountOnPathLink` / `OnSeat` | Index space (per segment or chain-wide), count or flag | Put one passenger on a known link or seat of the rear segment of an articulated bus; loop indices 0-199; repeat with two passengers on one link |

Runners-up: the coin index to coin mapping of `GiveChangeCoin`, which of an `.otp` ticket's strings `GetTicketName` returns, whether `NrSpecRandom` is stable per vehicle over frames.

## Related

[`.osc` language](osc.md) · [predefined variables](predefined-variables.md) · [`.hof`](hof.md) · [model.cfg](model_cfg.md) · [script VM](../../../architecture/script-vm.md) · [vehicle scripts route](../../../architecture/vehicle-scripts-route.md)

**Clean-room attestation (ADR-0002):** no OMSI binaries were inspected; all statements come from shipped text files and are inferences, not facts about the engine.
