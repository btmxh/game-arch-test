use std::time::Duration;

use super::clock::Clock;

pub trait ClockSync {
    fn sync(&mut self, frequency: f64) {
        if frequency > 0.0 {
            self.sync_impl(frequency)
        }
    }

    fn sync_impl(&mut self, frequency: f64);
}

pub struct OFClockSync<C: Clock> {
    clock: C,
    current_time: f64,
    last_frame_time: f64,
    sleep_error: f64,
}

impl<C: Clock> ClockSync for OFClockSync<C> {
    fn sync_impl(&mut self, frequency: f64) {
        const MIN_LAG: f64 = -1.0 / 30.0;
        self.last_frame_time = self.current_time;
        self.current_time = self.clock.now();

        let excess_time = 1.0 / frequency - (self.current_time - self.last_frame_time);
        let before = self.current_time;
        let sleep_time = (excess_time + self.sleep_error).max(0.0);

        std::thread::sleep(Duration::from_secs_f64(sleep_time));
        self.current_time = self.clock.now();
        let time_slept = self.current_time - before;

        self.sleep_error += excess_time - time_slept;
        self.sleep_error = self.sleep_error.max(MIN_LAG);
    }
}

impl<C: Clock> OFClockSync<C> {
    pub fn new(clock: C) -> Self {
        Self {
            last_frame_time: clock.now(),
            current_time: clock.now(),
            sleep_error: 0.0f64,
            clock,
        }
    }
}

impl<C: Clock + Default> Default for OFClockSync<C> {
    fn default() -> Self {
        Self::new(C::default())
    }
}
