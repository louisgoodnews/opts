//! OPTS CLI toolsuite binary entrypoint.

fn main() -> Result<(), anyhow::Error> {
    let _guard = opts_logging::init_default()?;
    tracing::info!("OPTS CLI toolsuite initialized");
    Ok(())
}
