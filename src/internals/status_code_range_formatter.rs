use http::StatusCode;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::ops::Bound;
use std::ops::RangeBounds;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StatusCodeRangeFormatter<R>(pub R)
where
    R: RangeBounds<StatusCode>;

impl<R> Display for StatusCodeRangeFormatter<R>
where
    R: RangeBounds<StatusCode>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let start = self.0.start_bound();
        let end = self.0.end_bound();

        match start {
            Bound::Included(code) => {
                write!(f, "{}", code.as_u16())?;
            }
            Bound::Excluded(code) => {
                write!(f, "{}=", code.as_u16())?;
            }
            Bound::Unbounded => {}
        };

        write!(f, "..")?;

        match end {
            Bound::Included(code) => {
                write!(f, "={}", code.as_u16())?;
            }
            Bound::Excluded(code) => {
                write!(f, "{}", code.as_u16())?;
            }
            Bound::Unbounded => {}
        };

        Ok(())
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;

    #[test]
    fn it_should_format_range() {
        let output = StatusCodeRangeFormatter(StatusCode::OK..StatusCode::IM_USED).to_string();
        assert_eq!(output, "200..226");
    }

    #[test]
    fn it_should_format_range_with_exclusive_start() {
        let output = StatusCodeRangeFormatter((
            Bound::Excluded(StatusCode::OK),
            Bound::Included(StatusCode::IM_USED),
        ))
        .to_string();
        assert_eq!(output, "200=..=226");
    }

    #[test]
    fn it_should_format_range_inclusive() {
        let output = StatusCodeRangeFormatter(StatusCode::OK..=StatusCode::IM_USED).to_string();
        assert_eq!(output, "200..=226");
    }

    #[test]
    fn it_should_format_range_from() {
        let output = StatusCodeRangeFormatter(StatusCode::OK..).to_string();
        assert_eq!(output, "200..");
    }

    #[test]
    fn it_should_format_range_to() {
        let output = StatusCodeRangeFormatter(..StatusCode::IM_USED).to_string();
        assert_eq!(output, "..226");
    }

    #[test]
    fn it_should_format_range_to_inclusive() {
        let output = StatusCodeRangeFormatter(..=StatusCode::IM_USED).to_string();
        assert_eq!(output, "..=226");
    }

    #[test]
    fn it_should_format_range_full() {
        let output = StatusCodeRangeFormatter(..).to_string();
        assert_eq!(output, "..");
    }
}
