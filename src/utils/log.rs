use std::io;

use anyhow::Context;
use tracing::subscriber::set_global_default;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, prelude::__tracing_subscriber_SubscriberExt, EnvFilter};

use crate::utils::args::args;

pub type LogGuard = Option<WorkerGuard>;

pub fn init_log() -> anyhow::Result<LogGuard> {
    let collector = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(args().log_level.into()))
        .with(fmt::Layer::new().with_writer(io::stdout));

    LogTracer::init()?;
    if let Some(log_file) = args().log_file.as_ref() {
        let appender = tracing_appender::rolling::never(".", log_file);
        let (nonblocking, guard) = tracing_appender::non_blocking(appender);
        let collector = collector.with(fmt::Layer::new().with_ansi(false).with_writer(nonblocking));
        set_global_default(collector).map(|_| Some(guard))
    } else {
        set_global_default(collector).map(|_| None)
    }
    .context("unable to set global logger")
}
