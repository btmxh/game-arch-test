use anyhow::Context;

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
};

use self::{
    close::Close, error::Error, freq_profile::FreqProfile, update_delay_test::UpdateDelayTest,
    vsync::VSync,
};

pub mod close;
pub mod error;
pub mod freq_profile;
pub mod update_delay_test;
pub mod vsync;

pub struct Utility {
    vsync: VSync,
    freq_profile: FreqProfile,
    update_delay_test: UpdateDelayTest,
    close: Close,
    error: Error,
}

impl Utility {
    pub fn new(
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            vsync: VSync::new(executor, main_ctx).context("unable to initialize VSync scene")?,
            freq_profile: FreqProfile::new(executor, main_ctx)
                .context("unable to initialize frequency profiling scene")?,
            update_delay_test: UpdateDelayTest::new(executor, main_ctx)
                .context("unable to initialize update delay test scene")?,
            close: Close::new(executor, main_ctx).context("unable to initialize close scene")?,
            error: Error::new(executor, main_ctx).context("unable to initialize error scene")?,
        })
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        Ok(self.vsync.handle_event(executor, main_ctx, event)?
            || self.freq_profile.handle_event(executor, main_ctx, event)?
            || self
                .update_delay_test
                .handle_event(executor, main_ctx, event)?
            || self.close.handle_event(executor, main_ctx, event)?
            || self.error.handle_event(executor, main_ctx, event)?)
    }
}
