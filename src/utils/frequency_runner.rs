use std::{collections::VecDeque, num::NonZeroUsize};

use super::clock::{Clock, SteadyClock};

pub struct FrequencyProfiler<C: Clock = SteadyClock> {
    pub clock: C,
    pub num_sample_frames: NonZeroUsize,
    pub times: VecDeque<f64>,
}

impl<C: Clock> FrequencyProfiler<C> {
    pub fn new(clock: C, num_sample_frames: NonZeroUsize) -> Self {
        Self {
            clock,
            num_sample_frames,
            times: VecDeque::new(),
        }
    }

    pub fn update_and_get_frequency(&mut self) -> Option<f64> {
        let cur_time = self.clock.now();
        let freq = if self.times.len() < self.num_sample_frames.get() {
            self.times.front().copied()
        } else {
            self.times.pop_front()
        }
        .map(|t| (self.times.len() as f64 + 1.0) / (cur_time - t));
        self.times.push_back(cur_time);
        freq
    }
}

impl Default for FrequencyProfiler<SteadyClock> {
    fn default() -> Self {
        Self::new(SteadyClock::default(), NonZeroUsize::new(16).unwrap())
    }
}
