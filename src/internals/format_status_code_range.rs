use http::StatusCode;
use std::fmt::Write;
use std::ops::Bound;
use std::ops::RangeBounds;

pub fn format_status_code_range<R>(range: R) -> String
where
    R: RangeBounds<StatusCode>,
{
    let mut output = String::new();

    let start = range.start_bound();
    let end = range.end_bound();

    match start {
        Bound::Included(code) | Bound::Excluded(code) => {
            write!(output, "{}", code.as_u16()).expect("Failed to build debug string");
        }
        Bound::Unbounded => {}
    };

    write!(output, "..").expect("Failed to build debug string");

    match end {
        Bound::Included(code) => {
            write!(output, "={}", code.as_u16()).expect("Failed to build debug string");
        }
        Bound::Excluded(code) => {
            write!(output, "{}", code.as_u16()).expect("Failed to build debug string");
        }
        Bound::Unbounded => {}
    };

    output
}

#[cfg(test)]
mod test_format_status_code_range {
    use super::*;

    #[test]
    fn it_should_format_range() {
        let output = format_status_code_range(StatusCode::OK..StatusCode::IM_USED);
        assert_eq!(output, "200..226");
    }

    #[test]
    fn it_should_format_range_inclusive() {
        let output = format_status_code_range(StatusCode::OK..=StatusCode::IM_USED);
        assert_eq!(output, "200..=226");
    }

    #[test]
    fn it_should_format_range_from() {
        let output = format_status_code_range(StatusCode::OK..);
        assert_eq!(output, "200..");
    }

    #[test]
    fn it_should_format_range_to() {
        let output = format_status_code_range(..StatusCode::IM_USED);
        assert_eq!(output, "..226");
    }

    #[test]
    fn it_should_format_range_to_inclusive() {
        let output = format_status_code_range(..=StatusCode::IM_USED);
        assert_eq!(output, "..=226");
    }

    #[test]
    fn it_should_format_range_full() {
        let output = format_status_code_range(..);
        assert_eq!(output, "..");
    }
}
