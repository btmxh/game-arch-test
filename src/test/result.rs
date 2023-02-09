use std::borrow::Cow;

pub type TestResult = anyhow::Result<(), TestError>;

#[derive(Debug)]
pub enum TestError {
    ChildFailedError(Vec<Cow<'static, str>>),
    AssertError {
        found: String,
        expected: String,
        error: Option<String>,
    },
    GenericError(anyhow::Error),
}

impl From<anyhow::Error> for TestError {
    fn from(value: anyhow::Error) -> Self {
        Self::GenericError(value)
    }
}
