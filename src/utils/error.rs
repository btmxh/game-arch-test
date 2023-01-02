use std::fmt::Display;

pub trait IntoAnyhowError {
    fn into_anyhow_error(self) -> anyhow::Error;
}

impl<T> IntoAnyhowError for super::mpsc::SendError<T> {
    fn into_anyhow_error(self) -> anyhow::Error {
        anyhow::Error::msg("mpsc::SendError(...)")
    }
}

pub trait ResultExt<T> {
    fn log_error(self) -> Option<T>;
    fn log_warn(self) -> Option<T>;
}

impl<T, E: Display> ResultExt<T> for Result<T, E> {
    fn log_error(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::error!("{}", err);
                None
            }
        }
    }

    fn log_warn(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!("{}", err);
                None
            }
        }
    }
}
