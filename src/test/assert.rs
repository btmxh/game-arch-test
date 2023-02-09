use std::fmt::Debug;

use super::result::{TestError, TestResult};

pub fn assert_equals<T: PartialEq + Debug>(found: &T, expected: &T) -> TestResult {
    if found != expected {
        Err(TestError::AssertError {
            found: format!("{found:?}"),
            expected: format!("{expected:?}"),
            error: None,
        })
    } else {
        Ok(())
    }
}
