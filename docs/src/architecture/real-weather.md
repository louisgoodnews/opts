# Real Weather: Provider Survey (M-0116, partial)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0116](../roadmap/phase-11/M-0116.md) |
| **Status** | 🟡 Partial: survey written from search results, 2026-11-14. Mapping, policy, procedural fallback and the decision are not done. |
| **Related** | [Weather](weather.md), [Session environment](session-environment.md), [ADR-0002](../adr/0002-clean-room-policy.md) |

**Evidence tags.** **[S]** seen in a search-result excerpt of the provider's own page (not a full read); **[T]** claimed by a third-party page only; ❓ not verified. No page was read in full in this pass, so every licence statement below needs a full read before a decision. Nothing here is legal advice.

---

## 1. Provider survey

| Provider (class) | Coverage | Licence and attribution | Commercial use | Rate limits and cost | Auth and offline use | Evidence |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Open-Meteo** (community aggregator of open national models, with a historical archive) | Global, from open model and station data | Data under CC BY 4.0. Attribution required; the credit line suggested by a third party is "Weather data by Open-Meteo.com (CC BY 4.0)" [T]. | The free API is limited to **non-commercial** use in the terms [S]. Paid subscriptions carry the CC BY 4.0 data terms [S]. The homepage's "free for commercial use" wording conflicts with the terms; the terms win. | Free-tier caps in the terms: under 10,000 calls per day, 5,000 per hour, 600 per minute [S]. The homepage and third parties quote about 10,000 per day only [T]. Paid tiers: ❓ not read. | No key for the free tier [T]. Responses are cacheable only as far as the terms allow: ❓ not read. | [open-meteo.com/en/terms](https://open-meteo.com/en/terms), [/en/licence](https://open-meteo.com/en/licence), [/en/pricing](https://open-meteo.com/en/pricing) |
| **Deutscher Wetterdienst** (public met service, open data, Germany) | Germany, stations and radar; CDC climate archive | CC BY 4.0; the DWD must be named as source. The CDC terms PDF is dated May 2024 [S]. Warnings data is a separate case: a third-party reading says official warnings lose the source label when altered [T]. | Covered by CC BY 4.0, so commercial use is allowed with attribution [S]. Warnings: ❓. | No rate limit stated for bulk open data [T]; ❓ for the warnings feed. | No key [T]. | [opendata.dwd.de CDC terms (PDF)](https://opendata.dwd.de:443/climate_environment/CDC/Terms_of_use.pdf), [dwd.de/copyright](https://www.dwd.de/EN/service/copyright/copyright_node.html) |
| **Met Office DataPoint** (public met service, UK) | UK and global forecast and observations | Open Government Licence; the data is free to use, copy and adapt under the OGL [S]. Attribution wording on DataPoint itself: not found. Other Met Office licences ask for "© Crown copyright [year], Met Office" or "Powered by Met Office data" [T]. | Allowed under the OGL, which the search result describes as worldwide, royalty-free and perpetual [S]. Endorsement claims need written consent [S]. | Free while within the fair-use limits; the numeric limits were not in the excerpt ❓. The policy may change at any time, with continued use as acceptance [S]. | Registration required [T]. | [DataPoint terms](https://www.metoffice.gov.uk/services/data/datapoint/terms-and-conditions---datapoint), [registration terms](https://www.metoffice.gov.uk/policies/datapoint-registration) |
| **OpenWeatherMap** (commercial API) | Global | Terms not read ❓. | Terms not read ❓. | 1,000 calls per day free under pay-per-call [S]. The overage rate differs between pages: 0.0012 GBP per call on the detailed page, 0.0015 USD on an older page [S]. A new subscription defaults to a 2,000 calls per day cap [S]. Fixed plans start at 30 GBP per month [S]. The One Call 2.5 API is deprecated [S]. | API key required. A card may be needed even for the free tier [T]. | [openweathermap.org/price](https://openweathermap.org/price), [full-price](https://openweathermap.org/full-price.md) |
| **Meteostat** (community and station aggregator, historical) | Global stations, historical | Aggregated dataset under CC BY 4.0; credit to Meteostat and its providers, example naming DWD [S]. | No non-commercial clause found [T]. The library's License object says commercial use depends on each underlying source [S]. | No official figure found. Quotas come from the RapidAPI plan for the JSON API [T]. | API key via RapidAPI for the JSON API [T]; bulk data and the Python library are separate [T]. | [dev.meteostat.net/license](https://dev.meteostat.net/license), [dev.meteostat.net/terms](https://dev.meteostat.net/terms) |
| **MET Norway** (public met service, Norway) | Global forecast | Terms of service exist ❓ not read. | ❓ | ❓ | ❓ | Listed in search results, [api.met.no terms](https://api.met.no/doc/TermsOfService); not surveyed further. |

**Provider classes covered.** Public services (DWD, Met Office; MET Norway listed only), commercial API (OpenWeatherMap), community and historical (Open-Meteo, Meteostat). The milestone asks for at least three classes; this pass covers all three, but the commercial class rests on pricing pages, not terms.

**Gaps that block the decision.**

1. Full terms text for every provider (the table is built from excerpts).
2. OpenWeatherMap terms and the price conflict (GBP versus USD).
3. Whether Open-Meteo's non-commercial clause applies to OPTS, which is a commercial product if it is sold. This is a licensing question for the project lead.
4. Caching rules for offline use, which none of the excerpts answered.
5. Met Office DataPoint attribution wording and current fair-use numbers.

---

## 2. Not done (remaining M-0116 items)

- Observation-to-preset mapping for 10 sample situations (needs the preset list in [weather.md §2](weather.md)).
- Offline, privacy and caching policy; the consent UI sketch.
- Procedural fallback design.
- Recommendation, ADR proposal, and this page's go/no-go. The survey above names no recommended provider.

---

## 3. Date and method

Checked 2026-11-14 with web search excerpts only. Re-check each link before any decision; provider terms change (see the milestone risk table).
