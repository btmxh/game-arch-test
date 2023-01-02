use tracing::{subscriber::set_global_default, Level};
use tracing_subscriber::FmtSubscriber;

pub fn init_log() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_ansi(true)
        .finish();

    set_global_default(subscriber)?;
    Ok(())
}
