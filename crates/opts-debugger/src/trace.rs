//! Scene-trace reader (ISS-096 stage 4, the `trace` tool).
//!
//! Will read a `scene-trace-*.jsonl` file offline and filter its records by text, by tile, or
//! count them. The route to the record types is open: through `opts-bevy`, or a Bevy-free split
//! of the record schema (ISS-096 decision 2).
