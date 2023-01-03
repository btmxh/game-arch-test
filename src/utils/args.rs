use std::mem::MaybeUninit;

use clap::Parser;
use tracing::Level;

/// A Rust rhythm game architecture test
#[derive(Parser, Debug)]
pub struct Args {
    /// Whether or not to enable OpenGL debug callback
    #[arg(long)]
    pub gl_disable_debug_callback: bool,
    /// Index to select OpenGL config, if not provided, the system will
    /// automatically choose the most suitable config
    #[arg(long)]
    pub gl_config_index: Option<usize>,
    /// Whether or not to select OpenGL config with sRGB capabilities
    #[arg(long)]
    pub gl_disable_srgb: bool,
    #[arg(long, default_value_t = Level::TRACE)]
    pub log_level: Level,
    #[arg(long)]
    pub log_file: Option<String>,
}

static mut STATIC_ARGS: MaybeUninit<Args> = MaybeUninit::uninit();

pub fn parse_args() {
    let args = Args::parse();
    unsafe { STATIC_ARGS = MaybeUninit::new(args) };
}

pub fn args() -> &'static Args {
    unsafe { STATIC_ARGS.assume_init_ref() }
}
