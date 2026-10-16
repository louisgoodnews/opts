# The `.o3d` scrambler's generator: derivation report (2026-10-16)

| Metadata | Value |
| :--- | :--- |
| **Question** | MI-0017 / MI-0044 / Q-012: keyed `.o3d` meshes could only be decoded heuristically (about 24 % wrong, 3 % rejected). The generator's state chain was unknown (o3d.md Addendum 6). |
| **Result** | The generator is known and reproduced: `o3d::exact::decode` returns the original positions of 6,415 of 6,417 plain/keyed twins, the normals of 6,410 and the texture coordinates of 6,017 (the rest are plain copies with other texture coordinates). Rules: [o3d.md Addendum 14](../formats/binary/o3d.md#-addendum-14--2026-10-16-the-scramblers-generator-exact-decoding). |
| **Branch** | `research/scramble-chain` |
| **Clean room** | Black-box measurement on the installed files only. No OMSI binary or DLL inspected, no GPL source read, nothing of the installation modified, no OMSI asset committed. |
| **Tools** | `scramble_state_dump` (per-vertex evidence of every twin), `exact_twin_eval` (verification), `map_object_census` (`ok:exact`); the analysis scripts of this report are not kept (one-off, in the session scratchpad). |

## 1. Evidence per vertex

For every keyed mesh that has an unscrambled copy of the same model (6,417 files, 2.15 M vertices) the plain copy gives the truth of each vertex: the position permutation (one of the four the scrambler uses), the texture jitter `(du, dv)` and the signed permutation of the normal. Addendum 4 had established that each vertex is governed by a state `q` in `0..8000`: the permutation is an interval of `q`, the u-jitter reveals `q mod 100` when `q mod 5 = 0` and the v-jitter `2q mod 100` when `q mod 3 = 0`. A *consistency test* says whether a given `q` fits a vertex's evidence; chance level is about 10 %.

## 2. The form `q = n·s`

Files whose vertex count `n` is a multiple of 25 only ever show the u-jitter codes 25, 50 and 75; multiples of 20 only 20, 40, 60, 80. So `q mod 100` is always a multiple of `gcd(n, 100)`: `q = n·s mod 8000` for an integer state `s` at **every** vertex, not only at vertex 0. Linear congruential generators seeded with `K·n` (Delphi, glibc, MSVC, Borland, MINSTD, 1–3 draws per vertex, low or high bits) fit nothing.

## 3. The step multiplier is `n`

Two files holding the same model under different keys share every data-dependent quantity; only the seed differs. For a step `sᵢ₊₁ = a·sᵢ + c + g(data)`, their states differ by `aⁱ·(K_A − K_B)` at every vertex. Scanning `a` over the whole residue range with the evidence of 80 vertices leaves `a = n` in group after group (n = 8, 11, 18, 31, 32, 48, 64, 96, 108, 366). Hence `sᵢ₊₁ = n·sᵢ + gᵢ`, i.e. the next state is the current output `q` plus a data term.

## 4. The data term

Intersecting the candidate states of same-model files gives `g` exactly for 73 first vertices; adjacent vertices whose evidence alone pins both states give 1,450 more from the whole corpus (reliability checked against a third key: 7 of 7 confirmed, 0 contradicted). Facts: `g` is a byte (one sample is exactly 255), a function of the position only (files sharing a first-vertex position but differing in UV or normal never diverge: 25 of 25; a differing position diverges in 97 %), of all three coordinates, and zero whenever a coordinate's fractional part is tiny — including large coordinates such as (−214.05, 7.08, −15.02). No scaled rounding of a coordinate, product, sum, length, polynomial fit, float bit pattern or byte/word polynomial hash matched more than the zero baseline. `⌊600·|frac(x)·frac(y)·frac(z)|⌋ mod 256` matches all 1,450; the scale is pinned (599.5 and 600.5 already lose 50–70 samples); the byte mask is confirmed on 48 samples above 255.

## 5. Whole chains, boundaries, normals

Simulating every twin from its header: 99.3 % of the vertex observations consistent. The remaining breaks were all at exact class boundaries: `q = 0` is swap x/y, `q = 1000` swap x/z, `q = 3000` identity, `q = 7000` identity, `q = 7001` swap y/z (thousands of observations each) — the classes are intervals of `q`, not of `(q + 1000) mod 8000` as Addendum 4 wrote them (same thing except at `q = 7000`). With the boundaries 99.88 %; the rest are plain copies with other texture coordinates (jitter codes such as 38 or 29 that the scrambler never produces).

The normal: the stored slots are negated by residues of `q` (x when `q mod 4 = 0`, y when `q mod 6 = 0`, z when `q mod 7 = 0`; 100 % within each permutation class once decomposed per slot) and permuted as a function of `q` with the boundaries at 600 and 4500 read off the exact values.

## 6. Key `0`

Every keyed mesh of `Lemmental_Neuenbreid` has key `0` (615 objects, 17,175 placements). These files start at `s₀ = 0` (955 of 957 twins) and the first step works, but accumulation does not: at every break no state fits the accumulating chain, while the two solvable cases show the state equal to the previous vertex's term alone. The memoryless chain `sᵢ₊₁ = g(pᵢ)` makes 934 of 957 key-0 twins fully consistent (100.0 % of the vertex observations); the 23 others break only after vertex 10 or carry a flagged header.

## 7. What changed in OPTS

| Where | Change |
| :--- | :--- |
| `opts-core::formats::o3d::exact` | `generator`, `decision`, `data_term`, `decode`, `scramble` (the inverse, for tests); `O3d::exact_decoded`; `VertexLayout` reads an exactly decoded mesh like a plain file |
| `opts-bevy::model` | keyed meshes are decoded exactly; `CacheStats::exact_meshes`; scene-trace stage `exact` (the plain-twin lookup and the heuristic decoder stayed as fallbacks until the follow-up below) |
| `map_object_census` | class `ok:exact` |
| docs | o3d.md Addendum 14, Q-012 resolved; registry MI-0049, MI-0017 and MI-0044 closed; scenery-parsing.md |

## 8. Results

| Map | Before (heuristic) | After (exact) |
| :--- | :--- | :--- |
| Lemmental_Neuenbreid | 95 objects / 3,023 placements undecodable; 53 decoded meshes / 1,144 placements on the wrong axis; 169 meshes without a floor axis | 615 objects / 17,175 placements exact; 0 undecodable; 0 on the wrong axis |
| Grundorf | 77 objects undecodable, 10 meshes on the wrong axis | 0, 0 |
| Waldhofen | 419 objects undecodable, 57 meshes on the wrong axis | 0, 0 |
| Aachen Linie 33 | 1 undecodable | 0 |

Live on Neuenbreid: the house that lay on its side at the start stands, `DDR_House_small_01` is a complete house, the sign pole near the start carries its plate, and the lamp post has its head; the scene trace shows no `rejected`, `decode-cache` or `twin` stage any more.

## 9. Open

- Two key-0 twins with a flagged header, and the normals of 5 twins (zero components whose sign is ambiguous) are not reproduced.

## 10. Follow-up (2026-10-16, branch `cleanup/retire-heuristic-decoder`)

- The heuristic decoder, its on-disk cache, the background decode pool, the plain-twin substitution, the incident files and the vehicle prewarm stage were retired ([o3d.md Addendum 15](../formats/binary/o3d.md#-addendum-15--2026-10-16-the-heuristic-decoder-retired)). A keyed file the rule does not cover is skipped with a `WARN` naming the path.
- The jitter's arithmetic was pinned down: `fl32(c²)/10000` in float32 (not `(c/100)²`), which makes 87 % of the jittered texture coordinates bit-exact (3,759 of 6,417 twins in every coordinate, 1,372 before; 6,019 within 1e-5); the rest differ by one float32 ulp because the exporter's float32 addition is not injective (Addendum 15).
