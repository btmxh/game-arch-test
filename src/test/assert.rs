use std::{borrow::Cow, fmt::Debug};

use super::result::{Comparison, TestError, TestResult};

pub fn assert_equals<T: PartialEq + Debug>(
    found: &T,
    expected: &T,
    msg: impl Into<Cow<'static, str>>,
) -> TestResult {
    if found == expected {
        Ok(())
    } else {
        Err(TestError::AssertCompareError {
            found: format!("{found:?}"),
            expected: format!("{expected:?}"),
            comparison: Comparison::Equals,
            compare_error: None,
            custom_msg: msg.into(),
        })
    }
}

pub fn assert_not_equals<T: PartialEq + Debug>(
    found: &T,
    expected: &T,
    msg: impl Into<Cow<'static, str>>,
) -> TestResult {
    if found != expected {
        Ok(())
    } else {
        Err(TestError::AssertCompareError {
            found: format!("{found:?}"),
            expected: format!("{expected:?}"),
            comparison: Comparison::NotEquals,
            compare_error: None,
            custom_msg: msg.into(),
        })
    }
}

pub fn assert_less_than<T: PartialOrd + Debug>(
    found: &T,
    expected: &T,
    msg: impl Into<Cow<'static, str>>,
) -> TestResult {
    if found < expected {
        Ok(())
    } else {
        Err(TestError::AssertCompareError {
            found: format!("{found:?}"),
            expected: format!("{expected:?}"),
            comparison: Comparison::Equals,
            compare_error: None,
            custom_msg: msg.into(),
        })
    }
}

pub fn assert_greater_than<T: PartialOrd + Debug>(
    found: &T,
    expected: &T,
    msg: impl Into<Cow<'static, str>>,
) -> TestResult {
    if found > expected {
        Ok(())
    } else {
        Err(TestError::AssertCompareError {
            found: format!("{found:?}"),
            expected: format!("{expected:?}"),
            comparison: Comparison::Greater,
            compare_error: None,
            custom_msg: msg.into(),
        })
    }
}

pub fn assert_less_equals<T: PartialOrd + Debug>(
    found: &T,
    expected: &T,
    msg: impl Into<Cow<'static, str>>,
) -> TestResult {
    if found <= expected {
        Ok(())
    } else {
        Err(TestError::AssertCompareError {
            found: format!("{found:?}"),
            expected: format!("{expected:?}"),
            comparison: Comparison::LessEquals,
            compare_error: None,
            custom_msg: msg.into(),
        })
    }
}

pub fn assert_greater_equals<T: PartialOrd + Debug>(
    found: &T,
    expected: &T,
    msg: impl Into<Cow<'static, str>>,
) -> TestResult {
    if found >= expected {
        Ok(())
    } else {
        Err(TestError::AssertCompareError {
            found: format!("{found:?}"),
            expected: format!("{expected:?}"),
            comparison: Comparison::GreaterEquals,
            compare_error: None,
            custom_msg: msg.into(),
        })
    }
}
