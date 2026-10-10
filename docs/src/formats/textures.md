# Texture Formats — `.dds`, `.bmp`, `.tga`, `.png`, `.jpg`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Image containers (public formats) |
| **File Extensions** | `.dds` (99,815), `.bmp` (45,575), `.jpg` (22,482 + 15 `.jpeg`), `.tga` (19,717), `.png` (14,752) |
| **Parser Crate** | `opts-core` (decoding delegated to image crates; see [Implementation Notes](#implementation-notes)) |
| **Parser Module** | `opts_core::formats::texture` |
| **Encoding** | Binary, little-endian (PNG: big-endian chunks) |
| **Status** | 🟡 Draft — container usage corpus-verified; rendering semantics (alpha modes, mip policy) unverified |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

OMSI textures are **ordinary image files** in five common container formats. OMSI/DirectX 9 loads them; there is no OMSI-specific image container. This page documents **which variants of each container the installed content actually uses**, how textures are found, and the traps (wrong extensions, alpha-only maps, non-power-of-two sizes, paletted bitmaps) a loader must handle. It does not re-specify the public formats (Microsoft DDS, BMP, Truevision TGA, PNG, JPEG).

Evidence tags: **[C]** counted over the whole install (header scan of 202,000 image files), **[S]** statistics over a random sample, **[H]** inspected in individual files, ❓ unverified. No mutation test in OMSI has been run.

| Extension | Files | Typical role |
| :--- | ---: | :--- |
| `.dds` | 99,815 | Main format for vehicles, buildings and map ground masks (compressed, optional mipmaps) |
| `.bmp` | 45,575 | Older content, UI images, bump maps, map overview images |
| `.jpg` / `.jpeg` | 22,497 | Photographic textures, repaint sources |
| `.tga` | 19,717 | Textures with alpha (trees, humans, vehicle parts) |
| `.png` | 14,752 | Newer add-ons, sources |

### Where textures are referenced

| Reference | Written in | Example |
| :--- | :--- | :--- |
| Material texture name | [`.o3d`](binary/o3d.md) material chunk (`name_len` + name), [`.x`](binary/x.md) `TextureFilename` | `Tree_Medium_02.tga`, `reg.tga` |
| `[matl]`, `[matl_nightmap]`, `[matl_lightmap]`, `[matl_envmap]`, `[matl_bumpmap]`, `[texture]`, `[CTCTexture]` | [`.sco`](text/sco.md), [`.sli`](text/sli.md), model configs | `reeperbahn.dds` |
| Ground texture | `[groundtex]` in [`global.cfg`](map/global-cfg.md) | `texture\gras.bmp` |
| Texture property | `<texture>.<ext>.cfg` ([texture-cfg](text/texture-cfg.md)) | `str_asphdrk.bmp.cfg` |

---

## 🔤 General Structure

### Finding a texture

Names are Windows paths (case-insensitive, `\` or `/`), with sub-folders allowed. For a scenery object at `Sceneryobjects/<Pack>/model/x.o3d` the texture is looked for in `Sceneryobjects/<Pack>/texture/` and in the global `Texture/` folder; vehicles use their own `Texture` folders (see [`model_cfg`](text/vehicles/model_cfg.md)). In a 1,500-mesh sample of scenery objects, **4,864 material texture names** resolved as follows [S]:

| Result | Names | Share |
| :--- | ---: | ---: |
| File exists under exactly that name | 3,838 | 79 % |
| **Only a `.dds` with the same base name exists** | **888** | 18 % |
| Not found in the pack's `texture` folder nor `Texture/` | 138 | 3 % |

The 888 substitutions are `.jpg`→`.dds` (362), `.bmp`→`.dds` (291), `.png`/`.PNG`→`.dds` (214), `.tga`→`.dds` (15), `.psd`→`.dds` (2). **Mutation Kit 2 experimentally verified this behavior in OMSI 2 (T204):**
- When a `.bmp` texture is requested and missing on disk, OMSI **automatically and silently falls back to a `.dds`** of the same base name.
- OMSI **does not fall back to `.tga` or `.png`**; missing requests for `.bmp` when only `.tga` or `.png` exist cause OMSI to emit `Warning: Did not find texture file "..."!` in `logfile.txt`.
- A compatible reader should try: exact file name $\to$ same base name with `.dds`. Other extensions are not part of OMSI's automatic runtime fallback chain.

### Extension does not equal content

**1,170 files** carry an extension that does not match their content [C]:

| Extension | Actual content | Files |
| :--- | :--- | ---: |
| `.dds` | BMP (`BM` header) | 924 |
| `.dds` | PNG / JPEG / empty | 24 / 6 / 22 |
| `.bmp` | DDS / JPEG / PNG / empty | 68 / 57 / 30 / 5 |
| `.png` | JPEG / DDS / BMP | 7 / 2 / 1 |
| `.jpg` | DDS / PNG / BMP / empty | 10 / 5 / 5 / 5 |

These textures are used by shipped vehicles (892 mismatching `.dds` are under `Vehicles/`), so OMSI's loader must **identify the format from the content** (consistent with D3DX's file loader, ❓). A loader must sniff magic bytes (`DDS `, `BM`, `\x89PNG`, `FF D8`) before trusting the extension. 3 `.dds` and a few other files have unrecognised content (`00 00 0A 00 …`) and 32 are zero-byte files.

---

## 📚 Field Reference

### `.dds` (99,815 files; 980 are not DDS, see above)

Header: magic `DDS ` + 124-byte `DDS_HEADER` (public layout). Corpus usage of the header fields [C]:

**Pixel formats**

| Format | Files | Notes |
| :--- | ---: | :--- |
| `DXT1` (BC1) | 50,065 | Opaque or 1-bit alpha |
| **`A8`** (pixel-format flags `0x2` = alpha-only, 8 bit, `amask 0xFF`) | **20,908** | 20,886 of these are **map ground-layer masks** (below); the rest are small alpha-only textures (`nv_alpha.dds`, `*_trans.dds`) |
| `DXT5` (BC3) | 17,003 | Alpha textures (smooth alpha) |
| `DXT3` (BC2) | 9,705 | |
| `A8B8G8R8` (masks `ff/ff00/ff0000/ff000000` as R/G/B/A) | 409 | Uncompressed 32-bit; RGBA byte order |
| `A8R8G8B8` (masks `ff0000/ff00/ff/ff000000`) | 358 | Uncompressed 32-bit; BGRA byte order |
| `R8G8B8` 24-bit | 323 | |
| `X8R8G8B8` 32-bit, no alpha | 28 | |
| `DXT2` | 14 | Premultiplied-alpha BC2 |
| `DX10` extension header | 9 | Needs the 20-byte `DDS_HEADER_DXT10`; not loadable by D3D9 ❓ |
| 16-bit (`R5G6B5`, `A4R4G4B4`, `A1R5G5B5`, `L8A8`) | 11 | |
| `ATI2` (BC5) | 1 | Two-channel (normal map) |
| luminance (`flags 0x20`) | 1 | |

**Header flags and caps**

| `dwFlags` | Files | Meaning |
| :--- | ---: | :--- |
| `0xA1007` | 46,983 | caps\|h\|w\|format\|**mipmap count**\|linearsize |
| `0x81007` | 27,109 | as above, **no mipmaps** |
| `0x1007` | 21,483 | no size, no mipmaps (the A8 masks) |
| `0x21007` | 2,541 | mipmap count, no size field |
| `0x2100F` / `0x100F` | 570 / 149 | with pitch |

`dwCaps`: `0x1000` (texture only) 44,957; `0x401008` (complex+mipmap+texture) 32,194; `0x1002` (alpha+texture) 20,584. **No cube maps and no volume textures** occur. [C]

**Mipmaps.** The `MIPMAPCOUNT` flag is absent in **48,741** files (49 %): single level, so the engine/D3DX must generate mips or sample the single level. With the flag: 16,906 have a count of 1, **22,854 a full chain**, and **10,589 a partial chain** (more than one but fewer levels than the full chain). A loader must read the stored count and not assume a complete chain. [C]

**Dimensions.** Mostly square powers of two: 512×512 (28,900), 1024×1024 (19,413), 256×256 (15,145), 2048×2048 (4,857), 128×128 (3,744), 1024×512 (2,159), 512×256 (1,948), 64×64 (1,789), 4096×4096 (832). **11,545 files (12 %) are not power-of-two** (e.g. 164×132 for `laststn.osn_*.dds` screenshots). [C]

#### Map ground masks (`maps/<Map>/texture/map/tile_<x>_<y>.map.<N>.dds`)

20,886 `.dds` files in map `texture/map/` folders are **alpha-only (A8) images from 64×64 to 1024×1024**, 65,664 bytes for 256×256 (128 header + 65,536). `<N>` is a small number (1–7 seen); for Berlin-Spandau there are 8 `[groundtex]` entries and `<N>` runs 1…7, i.e. **mask `N` is the blend weight of ground texture `N`** (texture 0 being the base layer without a mask) ❓ — inferred from counts only ([Q-TX09](#-open-questions)). Sizes: 512² (11,434), 256² (6,256), 128² (1,818), 64² (1,194), 1024² (202). The matching light map is `tile_<x>_<y>.map.LM.bmp` ([tiles](map/tiles.md)).

### `.bmp` (45,575 files; 160 have another content type)

Standard Windows bitmap (`BM`). All images are **stored bottom-up** (no negative heights) [C].

| Bits | Compression | Files |
| ---: | :--- | ---: |
| 24 | none | 35,710 |
| 32 | none (`BI_RGB`) | 7,481 |
| 8 (paletted) | none | 1,876 |
| 32 | `BI_BITFIELDS` | 132 |
| 16 | none / bitfields | 119 / 47 |
| 4 | none / RLE4 | 45 / 2 |
| 1 | none | 3 |

DIB header sizes: 40 (`BITMAPINFOHEADER`) 43,262; **124** (`V5`) 1,100; **108** (`V4`) 1,006; 56 (`V3`) 47 — loaders must accept V4/V5 headers, not just 40-byte ones. 8 files have a header size field that disagrees with the real file size (tolerate). **5,155 (11 %) are non-power-of-two** (e.g. 800×480 ×571, 300×400 ×273). Common sizes: 256×256 (26,982), 512×512 (4,449), 1024×1024 (2,159), 128×128 (887). [C]

Observed roles: ordinary diffuse textures (24-bit), **textures with alpha stored as 32-bit BMP** (e.g. `tile_<x>_<y>.map.roadmap.bmp`), **8-bit bump/height maps** (`water_bump.bmp`, with `[matl_bumpmap]`), and map overview images (`*.roadmap.bmp`, `whole.roadmap.bmp`, some of which are 4-bit). Alpha in 32-bit `BI_RGB` BMPs is not guaranteed to be meaningful ([Q-TX06](#-open-questions)).

### `.tga` (19,717 files)

| Image type | Bits | Files | Notes |
| :--- | ---: | ---: | :--- |
| 10 (RLE true-colour) | 32 | 9,927 | |
| 2 (uncompressed true-colour) | 32 | 5,886 | |
| 10 | 24 | 2,425 | |
| 2 | 24 | 1,351 | |
| 10 | 16 | 21 | |
| 9 (RLE colour-mapped) / 1 / 3 | 8 | 35 / 8 / 2 | Palette/grey |
| other first-byte values (83, 78, 255, …) | | ~53 | **Not TGA images** despite the extension |

- **RLE is the majority** (63 %): a loader must implement RLE decoding.
- **Origin**: bottom-left in 19,622 files; **top-left (descriptor bit 5) in 21**. Both orientations must be honoured when mapping to UV space ([Q-TX07](#-open-questions)).
- Alpha: 8 alpha bits declared in 14,520 files (the 32-bit ones); some declare 1 or 4 alpha bits.
- Sizes: 2048×2048 (2,301), 512×512 (2,098), 1024×1024 (1,910), 256×256 (1,049), and **non-power-of-two** sizes in the hundreds (801×641 ×966, 321×254 ×560). [C]

### `.png` and `.jpg`

- **PNG** (14,752): 8-bit RGBA (colour type 6) 8,245; 8-bit RGB 4,092; 8-bit palette 1,838; 4-bit palette 493; 16-bit RGBA 20; 1-bit grey/palette 51; 10 are not PNG files. [C]
- **JPEG** (22,497): 22,470 are JFIF/EXIF (`FF D8`); 27 are other content. No alpha; typically photographic textures and **repaint sources** that ship alongside a compressed `.dds`.

---

## 🧪 Examples

A minimal valid **A8 mask** DDS header (self-written, 256×256, as used for map ground masks): magic `DDS `, `dwSize = 124`, `dwFlags = 0x1007`, `dwHeight = 256`, `dwWidth = 256`, `dwPitchOrLinearSize = 0`, `dwMipMapCount = 0`, pixel format `dwSize = 32`, `dwFlags = 2`, `dwRGBBitCount = 8`, `dwABitMask = 0xFF`, `dwCaps = 0x1002`; then 65,536 bytes of alpha. Hex of the pixel-format block: `20000000 02000000 00000000 08000000 00000000 00000000 00000000 ff000000`.

Content sniffing (descriptive):

```
bytes 0..4 == "DDS "          -> DDS
bytes 0..2 == "BM"            -> BMP
bytes 0..8 == 89 50 4E 47 0D 0A 1A 0A -> PNG
bytes 0..2 == FF D8           -> JPEG
else                          -> TGA only if byte 2 in {1,2,3,9,10,11} and plausible header; otherwise unknown
```

---

## Implementation Notes

- Decode DDS (BC1/BC2/BC3/BC5, uncompressed 16/24/32, A8) and keep block-compressed data compressed for the GPU where possible; fall back to CPU decoding.
- Treat DDS BC formats as **top-left origin, row 0 = top**; BMP and most TGA are **bottom-up** and must be flipped, or the V coordinate flipped consistently with the mesh UV convention ([`o3d.md` Q-005](binary/o3d.md#-open-questions)).
- Honour stored mip counts; generate mips when absent (49 % of DDS).
- Accept non-power-of-two sizes (≈ 12 % of DDS/BMP, many TGA).
- Never fail on an extension mismatch; sniff first.
- Resolve names with the fallback chain above and keep a "texture missing" placeholder.

---

## ❓ Open Questions

| ID | Question | Priority | Status / Finding |
| :--- | :--- | :--- | :--- |
| Q-TX01 | Exact name-resolution rule: extension fallback order. | 🟢 Low | ✅ **Resolved** by Mutation Kit 2 (T204): automatic extension fallback is strictly limited to `.dds`. When `.bmp` is missing, `.dds` is loaded silently. OMSI emits warnings and does not fall back to `.tga` or `.png`. |
| Q-TX02 | Does OMSI sniff content or fix the loader by extension (the 924 BMP-in-`.dds` files)? | 🟡 Medium | |
| Q-TX03 | Alpha handling: how `[matl_alpha]` modes map to DXT1 1-bit alpha vs DXT3/5 vs A8 vs 32-bit BMP/TGA alpha; which textures are alpha-tested vs blended. | 🟢 Low | ✅ Confirmed by Mutation Kit 2 (T205): Mode 1 = alpha testing / cutout (1-bit threshold), Mode 2 = alpha blending (smooth transparency). |
| Q-TX04 | Non-power-of-two textures and missing mip chains: does OMSI rescale, pad, or rely on hardware NPOT? | 🟡 Medium |
| Q-TX05 | Colour space: are textures treated as sRGB or linear? | 🟡 Medium |
| Q-TX06 | Is the 4th channel of 32-bit `BI_RGB` BMPs ever alpha? | 🟡 Medium |
| Q-TX07 | Interaction of image origin (BMP/TGA bottom-up) with the mesh UV V-convention (see `o3d.md` Q-005). The T02 test in the mutation kit answers it for `.bmp`. | 🔴 High |
| Q-TX08 | The 9 DX10-header DDS files and the BC5 (`ATI2`) file: loaded or ignored? | 🟢 Low |
| Q-TX09 | Mapping of `tile_<x>_<y>.map.<N>.dds` masks to `[groundtex]` indices and their blend maths. | 🟡 Medium — implemented as `layer N = mask N` blended over the layers below (`opts_bevy::ground`); see [tiles.md](map/tiles.md#-ground-painting-texture-masks--2026-10-10). Orientation (row 0 = north) unverified in-game. |
| Q-TX10 | Which PNG/JPEG/PSD references are actually loaded vs only authoring sources? | 🟢 Low |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (texture page, alpha-channel page, Blender import/export page).

- **Alpha**: only DDS and TGA carry transparency; BMP and JPEG do not; PNG is discouraged because of occasional display errors (📚). ✅ consistent with the corpus (14,752 PNG files exist but many are authoring sources). The wiki's alpha-editing page is a tool how-to (a DXT/BMP manipulation utility that edits the alpha channel as a separate image) and states no format rules.
- **Size**: powers of two are the rule but textures need not be square (`64×2048`, `1×1` are given as examples); keep to 256–1024 px for 32-bit memory reasons (📚). ⚠️ ✅ 12 % of corpus DDS and 11 % of BMP are non-power-of-two, so the rule is a recommendation.
- **Lookup** 📚: first the `texture` folder beside the object/spline, then `OMSI 2\Texture\`; sub-folders and parent folders may be referenced. Seasonal variants live in sub-folders `WinterSnowfall`, `WinterSnow`, `Spring`, `Summer`, `Fall`, `Winter` and must have the **same name and extension** as the base texture ✅ (`WinterSnow` folders seen). The `.dds` fallback found in the corpus ([Q-TX01](#-open-questions)) is **not** mentioned on the wiki.
- **Materials** 📚: one material per texture; the Blender workflow exports normals, UVs and materials; `.x` files can be converted to `.o3d` with a converter tool.
- Surface properties come from `*.cfg` / `*.surf` files ([texture-cfg](text/texture-cfg.md), [community cross-check](community-crosscheck.md)).

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `11-textur`, `21-bildbearbeitung-mit-alphakanal`, `5-blender-import-export-von-3d-modellen`.
- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04. Header scan (first 160 bytes) of 202,356 files with the extensions above.
- **Files inspected** (relative to the install): `maps/Berlin-Spandau/texture/map/tile_2380_11287.map.1.dds` (A8 mask header decoded byte-wise), `Drivers/nopic.bmp`, `Humans/ADDON_Bad_Huegelsdorf/Mann_1/texture/Mann_1/CBoy0001-M3-Body-D_V10.tga`, `maps/Rheinhausen/texture/water_bump.bmp`, `maps/Berlin-Spandau/laststn.osn_0_0.dds`.
- **Techniques:** header decode and tally per container; content-sniffing census against extensions (all image files); name-resolution experiment: 1,500 random `.o3d` files under `Sceneryobjects/…/model/`, every material texture name looked up in the pack's `texture` folder and the global `Texture` folder with exact name and with alternative extensions; map-mask count compared with the number of `[groundtex]` entries in `global.cfg`.
- **Not done:** no decoding of pixel data, no run-time tests in OMSI.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.o3d`](binary/o3d.md), [`.x`](binary/x.md) — texture names in materials
- [`.sco`](text/sco.md) — `[matl*]` keywords; [`sco-sli-audit`](text/sco-sli-audit.md)
- [Texture Property Files](text/texture-cfg.md), [Map Tiles](map/tiles.md), [`global.cfg`](map/global-cfg.md)
- [.wav — Sound Files](wav.md)
