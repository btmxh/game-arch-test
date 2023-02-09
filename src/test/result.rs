use std::borrow::Cow;

pub type TestResult = anyhow::Result<(), TestError>;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Comparison {
    Less,
    Greater,
    LessEquals,
    GreaterEquals,
    Equals,
    NotEquals,
}

#[derive(Debug)]
pub enum TestError {
    ChildFailedError(Vec<Cow<'static, str>>),
    AssertCompareError {
        found: String,
        expected: String,
        custom_msg: Cow<'static, str>,
        comparison: Comparison,
        compare_error: Option<String>,
    },
    AssertError {
        result: bool,
        custom_msg: Option<Cow<'static, str>>,
    },
    GenericError(anyhow::Error),
}

impl From<anyhow::Error> for TestError {
    fn from(value: anyhow::Error) -> Self {
        Self::GenericError(value)
    }
}
