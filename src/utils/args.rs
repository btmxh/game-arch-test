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
    /// Log level, use this to turn off unnecessary log messages
    #[arg(long, default_value_t = Level::TRACE)]
    pub log_level: Level,
    /// Log file, can be relative or absolute path
    #[arg(long, default_value = "amk.log")]
    pub log_file: Option<String>,
    /// Whether or not to block the event loop on certain events like
    /// `RedrawRequested` or `Resize`. This should be turned on or off
    /// accordingly for better performance and in order to get intended
    /// behavior.
    #[arg(long, action = clap::ArgAction::Set, default_value_t = default_block_event_loop())]
    pub block_event_loop: bool,
    /// Whether or not to throttle while handling Resize events.
    ///
    /// This should be used on platforms with the flag `block_event_loop`
    /// set to false (X11, etc.). Otherwise, all Resize events would then
    /// be handled, making the draw thread lags back.
    ///
    /// On platforms with the flag `block_event_loop`, enabling this will
    /// make the resizing process somewhat laggy and introduce rendering
    /// artifacts (only when resize).
    #[arg(long, action = clap::ArgAction::Set, default_value_t = !default_block_event_loop())]
    pub throttle_resize: bool,
}

static mut STATIC_ARGS: MaybeUninit<Args> = MaybeUninit::uninit();

pub fn parse_args() {
    let args = Args::parse();
    unsafe { STATIC_ARGS = MaybeUninit::new(args) };
}

pub fn args() -> &'static Args {
    unsafe { STATIC_ARGS.assume_init_ref() }
}

fn default_block_event_loop() -> bool {
    // TODO: inspect winit source code and add more OSes
    cfg!(windows)
}
