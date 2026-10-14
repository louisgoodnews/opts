# ADR-0002: Clean-Room Reverse Engineering Policy

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0002 |
| **Status** | ✅ Accepted |
| **Date** | 2026-10-04 |
| **Deciders** | Project Lead |
| **Supersedes** | None |
| **Revised** | 2026-10-14 — Added reference to Content Issue Registry as formal discovery log for undocumented keywords and asset edge cases. |

---

## ⚠️ Legal Disclaimer

> **This document is not legal advice.** It describes a technical and procedural policy adopted by the OPTS project to minimize legal risk. It does not guarantee legal safety. Before accepting external code contributions, publishing reverse-engineered specifications, or distributing any binaries, the project maintainers **must** consult a qualified intellectual property attorney in their jurisdiction. Laws regarding reverse engineering differ significantly between the United States, the European Union, and other regions.

---

## 📌 Context

OPTS aims to load and simulate assets created for **OMSI 2 (Omnibus Simulator 2)**, a proprietary commercial product developed by MR Software and published by Aerosoft. OMSI 2's file formats (`.o3d`, `.sco`, `.sli`, `.bus`, `.hof`) are **undocumented, proprietary, and closed-source**.

To achieve the project's vision, we must reverse-engineer these formats. This creates significant legal risk:

1. **Copyright Infringement:** Copying or deriving code from OMSI's binaries (`OMSI.exe`, DLLs, or any part of its source code) would constitute copyright infringement.
2. **Licensing Contamination:** Many community reverse-engineering tools are licensed under the **GPL** (e.g., the Blender O3D import/export plugin). Copying code from GPL projects into OPTS (which is MIT/Apache-2.0) would create a license incompatibility.
3. **End-User License Agreement (EULA) Violations:** OMSI 2's EULA likely prohibits reverse engineering. While EULA terms are not always enforceable (particularly where reverse engineering is protected by fair use for interoperability), they add legal uncertainty.
4. **Contributor Contamination:** If a contributor who has seen OMSI's proprietary code contributes to OPTS, the entire project could be tainted by that knowledge.

Without a strict, documented policy, OPTS risks legal action from MR Software, Aerosoft, or the authors of GPL-licensed community tools.

---

## 🎯 Decision

OPTS will follow a **strict clean-room reverse engineering policy** with the following principles:

1. **We will never decompile, disassemble, or inspect the OMSI 2 executable or any of its proprietary binaries.**
2. **We will document file formats based only on black-box observation and public knowledge.**
3. **We will treat every community reverse-engineering project as a "read-only" reference, never as a source of code.**
4. **We will implement parsers from our own specifications, written in our own words and syntax.**
5. **We will never distribute OMSI 2's copyrighted assets.**
6. **We will document the provenance of every fact in our format specifications.**

---

## 📜 Policy in Detail

### 1. ✅ Permitted Activities

| Activity | Description |
| :--- | :--- |
| **Black-Box Observation** | Opening OMSI asset files in a hex editor and observing their byte structure. |
| **Behavioral Testing** | Placing a custom asset in OMSI and observing what happens in the running game. |
| **Reading Public Documentation** | Reading community wikis, forum posts, and blog articles describing OMSI formats. |
| **Reading Public Specs** | Referencing spec documents (like the 010 Editor template) that describe the binary layout of `.o3d`. |
| **Reading GPL/Open Source Code for Education** | Reading the *source code* of projects like `Blender-O3D-IO` to understand how the format is structured, **but never copying code or its structure directly.** |
| **Creating Test Fixtures** | Creating our own tiny `.o3d`, `.sco`, and `.sli` files using third-party tools or hex editors, for testing our parsers. |

### 2. ❌ Forbidden Activities

| Activity | Description |
| :--- | :--- |
| **Decompilation** | Using tools like IDA Pro, Ghidra, or dnSpy on `OMSI.exe` or any OMSI DLL. |
| **Disassembly** | Inspecting the machine code of OMSI's binaries. |
| **Source Code Leakage** | Using any leaked OMSI source code, in whole or in part. |
| **Code Copying** | Copying code from `Blender-O3D-IO`, `O3DParse`, `O3DView`, or any other community project into OPTS. |
| **Structural Copying** | Translating code from another language (C#, Python, GDScript) into Rust line-by-line. |
| **Asset Distribution** | Distributing OMSI `.o3d`, `.sco`, `.sli`, `.dds`, or map files with OPTS. |
| **EULA Violations** | Any activity that would breach the OMSI 2 EULA if challenged in court. |

### 3. 🔒 The "Two-Phase" Documentation Process

To create a defensible clean-room implementation, we separate reverse engineering into two distinct phases, performed by potentially different contributors:

**Phase A — Specification Writing (Discovery)**
- A contributor studies the format using only **permitted activities** (hex editors, black-box tests, public documentation).
- They write a specification in `docs/src/formats/` describing the format's structure.
- **Crucially:** They do **not** write any implementation code.
- Every claim in the spec must cite its source (e.g., "observed in file X", "confirmed by community post Y").

**Phase B — Implementation (Translation)**
- A contributor reads only the specification (not the original OMSI files, not community source code).
- They write the Rust parser based on the spec alone.
- If the spec is ambiguous, they submit a request to update the spec rather than "figuring it out" by looking at forbidden sources.

For a solo contributor, this means **separating your discovery work from your implementation work into distinct sessions or commits**, with the spec committed *before* implementation begins. This establishes a documented trail proving the spec preceded the code.

When undocumented keywords, asset edge cases, or parser failures are discovered during black-box observation, they must be recorded in the **Content Issue Registry** (`docs/src/quality/content-issue-registry.csv` and [`docs/src/quality/README.md`](../quality/README.md)) as a formal discovery log. Each logged defect links the observed anomaly in third-party assets to its corresponding specification entry (`Q-NNN`), maintaining a clear provenance trail before any parser implementation or fix is written.

### 4. 🚧 The GPL Contamination Problem

Many community reverse-engineering tools are licensed under **GPL v2 or v3**. GPL is a "copyleft" license: any derivative work must also be GPL. Since OPTS is licensed under MIT/Apache-2.0, we **cannot**:

- Copy code from GPL projects.
- Translate GPL code into Rust.
- Closely paraphrase GPL code.

**What we *can* do:**
- Read GPL code to *understand* a format.
- Learn the *general concept* of how a format is structured.
- Then close the GPL source and write our own implementation from scratch, based on our own spec.

This is a delicate distinction. If you ever find yourself thinking, "Let me just look at how `Blender-O3D-IO` did this bit and convert it," **stop**. That's structural copying. Instead, update the spec with what you learned conceptually, and implement from the spec.

### 5. 📝 Provenance Logging

Every specification file must include a "Provenance" section at the bottom, listing:

```markdown
## Provenance

- Byte structure observed in hex editor (2024-MM-DD)
- Vertex layout confirmed by community post at [URL] (2024-MM-DD)
- Concept of chunk-based structure inspired by reading `Blender-O3D-IO` source (no code copied)
- Test fixture created with hex editor (2024-MM-DD)
```

This creates a documented chain of custody for every fact in the spec.

### 6. 🚫 Contribution Gatekeeping

To prevent contaminated contributions:

- The `CONTRIBUTING.md` file must require contributors to affirm they have **not** decompiled OMSI or copied code from GPL projects.
- Pull requests touching `opts-core` parser code must include a note confirming compliance with this ADR.
- If a contributor admits to having decompiled OMSI or worked on OMSI's proprietary code, their contributions to `opts-core` must be rejected or rewritten by another contributor.

---

## ⚖️ Legal Basis (Informational)

This policy is informed by several landmark cases, though outcomes depend heavily on jurisdiction:

- **Sega Enterprises v. Accolade (1992, US):** Established that reverse engineering for interoperability can be fair use.
- **Sony Computer Entertainment v. Connectix (2000, US):** Reinforced that reverse engineering to create a compatible product is fair use.
- **Google v. Oracle (2021, US):** Addressed reimplementation of APIs as fair use, though the ruling was narrow.
- **EU Software Directive (2009/24/EC):** Article 6 permits decompilation for interoperability under strict conditions, but Article 5(3) permits observation and testing.

**These cases do not guarantee protection.** They provide a foundation for a defensible position, but the only way to eliminate legal risk is to obtain a license from MR Software or to not reverse-engineer the format at all.

---

## 🌊 Consequences

### ✅ Positive
- **Legal Defensibility:** We have a documented, good-faith process that demonstrates respect for intellectual property.
- **Contributor Safety:** Contributors are protected by a clear policy that tells them exactly what is and isn't permitted.
- **License Compatibility:** OPTS remains MIT/Apache-2.0 without GPL contamination.
- **Trust:** Users and community members can trust that OPTS is a legitimate, ethical project.

### ⚠️ Negative
- **Slower Development:** The two-phase process is slower than just reading and translating community code.
- **Technical Friction:** Reverse-engineering from black-box observation alone is difficult and error-prone.
- **Incompleteness:** Some format details may be impossible to determine without decompilation, meaning we may not be able to support every OMSI feature.
- **Contribution Friction:** The gatekeeping rules may discourage some contributors.

### 🔄 Neutral
- **Legal Ambiguity Remains:** Even with this policy, a determined litigant could still pursue action. We accept this residual risk as inherent to the project's mission.

---

## 🔗 References
- [Sega Enterprises Ltd. v. Accolade, Inc., 977 F.2d 1510 (9th Cir. 1992)](https://en.wikipedia.org/wiki/Sega_v._Accolade)
- [Sony Computer Entertainment, Inc. v. Connectix Corp., 203 F.3d 596 (9th Cir. 2000)](https://en.wikipedia.org/wiki/Sony_Computer_Entertainment,_Inc._v._Connectix_Corp.)
- [Google LLC v. Oracle America, Inc., 593 U.S. ___ (2021)](https://en.wikipedia.org/wiki/Google_LLC_v._Oracle_America,_Inc.)
- [EU Directive 2009/24/EC on the legal protection of computer programs](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A32009L0024)
- [Blender-O3D-IO-Public (GPL)](https://github.com/space928/Blender-O3D-IO-Public)
- [OPTS Glossary](../glossary.md)
