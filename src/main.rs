//! OPTS application entrypoint: a thin wrapper, the start-up sequence lives in `opts_game::run`
//! (the same code path as `cargo run -p opts-game`).

fn main() {
    opts_game::run();
}
