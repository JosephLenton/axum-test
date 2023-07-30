#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum ExpectedState {
    Success,
    Failure,
    None,
}

impl From<Option<bool>> for ExpectedState {
    fn from(maybe_success: Option<bool>) -> Self {
        match maybe_success {
            None => Self::None,
            Some(true) => Self::Success,
            Some(false) => Self::Failure,
        }
    }
}
