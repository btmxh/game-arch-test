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
    /// Whether or not to enable `test` mode.
    ///
    /// The `test` mode disable the `content` scene and uses the `test` scene
    /// as its replacement. It is used to test the program in a similar fashion
    /// to how unit tests work, optionally allowing the user to visually see the
    /// process (by default, the window is not hidden), being the main testing
    /// mechanism (the program still has some sanity `#[test]` unit tests, and
    /// they can simple by run in the traditional way of doing a `cargo test`).
    ///
    /// In CI contexts, one should also enable the `--headless` and
    /// `--auto-run-tests` flags.
    ///
    /// tl;dr: enable this to test the program
    #[arg(long)]
    pub test: bool,
    /// Whether or not to hide the window. Hiding the window will also come with a
    /// side effect of disabling all rendering calls (jobs executed by
    /// `execute_draw_event` and `execute_draw_sync` will still be executed).
    #[arg(long)]
    pub headless: bool,
    /// Whether or not to automatically run all tests on program launch (if `test`
    /// mode is enabled, via the flag `--test`). This can be helpful when the
    /// user is unable to manually run the tests, i.e. when the flag `--headless`
    /// is enabled in CI contexts.
    #[arg(long)]
    pub auto_run_tests: bool,
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
