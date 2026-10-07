# 🧭 Coordinate Systems and Conversions

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Draft — Mesh space specified; World/tile space blocked on Q-M01 |
| **Last Updated** | 2026-10-05 |
| **Relevant Crates** | `opts-core`, `opts-bevy` |

---

## 📖 Overview

This document defines the coordinate frames used across the OMSI data formats and specifies how they map into the OPTS engine's native engine (Bevy).

In OMSI, meshes and world assets operate across distinct coordinate boundaries:
1. **Mesh Space (`.o3d`, `.x`):** Left-handed, Y-up (DirectX native).
2. **World / Tile Space (`.map`):** Tile grid in metres. Blocker: tile origin and world axis signs remain open under [Q-M01](../formats/map/tiles.md#open-questions).
3. **Engine Space (Bevy):** Right-handed, Y-up (OpenGL / Vulkan convention).

---

## 📐 OMSI Mesh Space (Resolved)

Established conclusively through in-engine mutation testing (tests T01 and T02 in OMSI 2.3.004):

| Axis | Vector | Direction | Meaning | Evidence |
| :---: | :---: | :--- | :--- | :--- |
| **+X** | `(1, 0, 0)` | Right | Lateral axis | [M: T01] |
| **+Y** | `(0, 1, 0)` | **Up** | Vertical elevation | [M: T01] |
| **+Z** | `(0, 0, 1)` | **Forward** | Longitudinal / depth axis | [M: T01] |

- **Handedness:** **Left-handed** (with index finger along +Y and thumb along +X, middle finger points into the screen along +Z).
- **Units:** Metres ($1.0 = 1\text{ m}$).
- **Triangle Winding:** Counter-clockwise (CCW) when viewing front faces; backfaces are culled.
- **UV Coordinates:** Origin `(0.0, 0.0)` is top-left of the texture; $U$ increases to the right, $V$ increases downwards.

---

## 📐 Bevy Engine Space

Bevy uses the standard right-handed, Y-up 3D convention:

| Axis | Vector | Direction |
| :---: | :---: | :--- |
| **+X** | `(1, 0, 0)` | Right |
| **+Y** | `(0, 1, 0)` | Up |
| **+Z** | `(0, 0, 1)` | Backwards (towards viewer) |
| **-Z** | `(0, 0, -1)` | Forward (into screen) |

---

## 🔄 Coordinate Conversion: Mesh Space to Bevy

Converting an OMSI `.o3d` or `.x` mesh into a Bevy mesh requires converting from **left-handed Y-up** to **right-handed Y-up**:

$$\begin{bmatrix} X_{\text{bevy}} \\ Y_{\text{bevy}} \\ Z_{\text{bevy}} \end{bmatrix} = \begin{bmatrix} 1 & 0 & 0 \\ 0 & 1 & 0 \\ 0 & 0 & -1 \end{bmatrix} \begin{bmatrix} X_{\text{omsi}} \\ Y_{\text{omsi}} \\ Z_{\text{omsi}} \end{bmatrix}$$

### 1. Positions and Normals
- Negate the $Z$ component of vertex positions: $Z_{\text{bevy}} = -Z_{\text{omsi}}$.
- Negate the $Z$ component of vertex normals: $N_{z,\text{bevy}} = -N_{z,\text{omsi}}$.

### 2. Triangle Winding
Negating an odd number of spatial axes inverts the triangle winding order (clockwise $\leftrightarrow$ counter-clockwise). To preserve front-face orientation in Bevy's CCW rasterizer, each triangle's indices must be reversed:
$$\text{Face}(i_0, i_1, i_2) \longrightarrow \text{Face}(i_0, i_2, i_1)$$

### 3. UV Mapping
Bevy and standard modern graphics pipelines expect $V=0.0$ at the bottom or top depending on the texture asset format. For direct texture compatibility with DirectX bitmaps and DDS textures, $V$ is typically retained as stored or inverted ($V_{\text{bevy}} = 1.0 - V_{\text{omsi}}$) based on the image format loader.

---

## 📐 OMSI Vehicle & Entity Space (Resolved)

Established from `.bus` manifests, passenger path networks (`paths.cfg`), and model animations (`model.cfg`):

| Axis | Direction | Meaning |
| :---: | :--- | :--- |
| **+X** | Right | Lateral axis |
| **+Y** | Forward | Longitudinal axis |
| **+Z** | Up | Vertical elevation |

- **Handedness:** Right-handed ($+X \times +Y = +Z$).
- **Units:** Metres ($1.0 = 1\text{ m}$).

### 🔄 Coordinate Conversion: Entity Space to Bevy

Converting an OMSI vehicle/entity position into Bevy requires remapping the longitudinal and vertical axes:

$$\begin{bmatrix} X_{\text{bevy}} \\ Y_{\text{bevy}} \\ Z_{\text{bevy}} \end{bmatrix} = \begin{bmatrix} 1 & 0 & 0 \\ 0 & 0 & 1 \\ 0 & -1 & 0 \end{bmatrix} \begin{bmatrix} X_{\text{omsi}} \\ Y_{\text{omsi}} \\ Z_{\text{omsi}} \end{bmatrix}$$

Inverse transformation:

$$\begin{bmatrix} X_{\text{omsi}} \\ Y_{\text{omsi}} \\ Z_{\text{omsi}} \end{bmatrix} = \begin{bmatrix} 1 & 0 & 0 \\ 0 & 0 & -1 \\ 0 & 1 & 0 \end{bmatrix} \begin{bmatrix} X_{\text{bevy}} \\ Y_{\text{bevy}} \\ Z_{\text{bevy}} \end{bmatrix}$$

### 🔄 Orientation and Euler Angles

OMSI Euler angles (yaw, pitch, roll in degrees) map into Bevy quaternion space:
- **Yaw ($\psi$):** Turn around OMSI vertical axis $+Z$ $\longrightarrow$ Bevy rotation around $+Y$.
- **Pitch ($\theta$):** Nose up/down around OMSI lateral axis $+X$ $\longrightarrow$ Bevy rotation around $+X$.
- **Roll ($\phi$):** Lateral banking around OMSI longitudinal axis $+Y$ $\longrightarrow$ Bevy rotation around $-Z$.

Implemented and verified in `opts_bevy::coords` (`omsi_to_bevy_pos`, `bevy_to_omsi_pos`, `omsi_euler_to_bevy_quat`, `bevy_quat_to_omsi_euler`, `OmsiTransform`, and `OptsCoordsPlugin`).

---

## 🚧 Blocker: World and Tile Coordinates (Q-M01)

> ⚠️ **Documented Blocker:** The mutation kit `opts-mutation-kit` only included fixtures for `.o3d` and `.x` mesh formats. World tile layout and spline coordinate frame questions ([`tiles.md`](../formats/map/tiles.md) Q-M01, Q-M02, Q-M03) could not be resolved during this pass.

Until a map-side mutation kit is run:
- Tile origin (corner vs. centre) remains open.
- Exact world mapping of tile coordinates `tile_<x>_<y>.map` remains provisional.
- Boundaries at asset load vs. simulation step for world coordinates will be finalized once Q-M01 is resolved.

---

## 📝 Provenance

- **In-Engine Mutation Testing (2026-10-05):** Resolved via test T01 (colored axes gizmo with 3m red on +X, 2m green on +Y, 1m blue on +Z) and test T02 (UV quadrant quad) rendered in OMSI 2.3.004.
- **Clean-room confirmation:** Confirmed without inspecting OMSI binaries or reading proprietary source code.
