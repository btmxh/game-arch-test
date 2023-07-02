use std::fmt::Debug;

pub trait ResultExt<T> {
    fn log_error(self) -> Option<T>;
    fn log_warn(self) -> Option<T>;
    fn log_info(self) -> Option<T>;
    fn log_debug(self) -> Option<T>;
}

impl<T, E: Debug> ResultExt<T> for Result<T, E> {
    fn log_error(self) -> Option<T> {
        self.map_err(|e| tracing::error!("{:?}", e)).ok()
    }

    fn log_warn(self) -> Option<T> {
        self.map_err(|e| tracing::warn!("{:?}", e)).ok()
    }

    fn log_info(self) -> Option<T> {
        self.map_err(|e| tracing::info!("{:?}", e)).ok()
    }

    fn log_debug(self) -> Option<T> {
        self.map_err(|e| tracing::debug!("{:?}", e)).ok()
    }
}
