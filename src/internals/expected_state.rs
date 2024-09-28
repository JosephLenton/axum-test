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

#[cfg(test)]
mod test_from {
    use super::*;

    #[test]
    fn it_should_turn_none_to_none() {
        let output = ExpectedState::from(None);
        assert_eq!(output, ExpectedState::None);
    }

    #[test]
    fn it_should_turn_true_to_success() {
        let output = ExpectedState::from(Some(true));
        assert_eq!(output, ExpectedState::Success);
    }

    #[test]
    fn it_should_turn_false_to_failure() {
        let output = ExpectedState::from(Some(false));
        assert_eq!(output, ExpectedState::Failure);
    }
}
