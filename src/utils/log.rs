use std::{fs::File, sync::Arc};

use anyhow::Context;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

use crate::utils::{args::args, error::ResultExt};

pub fn init_log() -> anyhow::Result<()> {
    let stdout = tracing_subscriber::fmt::layer().pretty();
    let log_file = args()
        .log_file
        .as_ref()
        .map(File::create)
        .and_then(|f| f.context("unable to create/open log file").log_warn());
    match log_file {
        Some(f) => {
            tracing_subscriber::registry()
                .with(
                    stdout
                        .with_writer(Arc::new(f))
                        .with_filter(LevelFilter::from_level(args().log_level)),
                )
                .init();
            tracing::info!(
                "Logging to stdout and log file '{}'",
                args().log_file.as_ref().unwrap()
            )
        }
        None => {
            tracing_subscriber::registry()
                .with(stdout.with_filter(LevelFilter::from_level(args().log_level)))
                .init();
            tracing::info!("Logging to stdout only");
        }
    };
    Ok(())
}
