use bytes::Bytes;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

/// An arbituary limit to avoid printing gigabytes to the terminal.
const MAX_TEXT_PRINT_LEN: usize = 10_000;

#[derive(Debug, Copy, Clone)]
pub struct BodyTextFmt<'a>(pub &'a Bytes);

impl<'a> Display for BodyTextFmt<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let text = String::from_utf8_lossy(self.0);
        let len = text.len();

        if len < MAX_TEXT_PRINT_LEN {
            return write!(f, "'{text}'");
        }

        let text_start = text.chars().take(MAX_TEXT_PRINT_LEN);
        write!(f, "'")?;
        for c in text_start {
            write!(f, "{c}")?;
        }
        write!(f, "...'")?;

        Ok(())
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_should_display_text_response_as_text() {
        let text_bytes = "Blah blah".into();
        let output = format!("{}", BodyTextFmt(&text_bytes));

        assert_eq!("'Blah blah'", output);
    }

    #[test]
    fn it_should_cutoff_very_long_text() {
        let max_len = MAX_TEXT_PRINT_LEN + 100;
        let text_bytes = (0..max_len).map(|_| "🦊").collect::<String>().into();

        let output = format!("{}", BodyTextFmt(&text_bytes));

        let expected_content = (0..MAX_TEXT_PRINT_LEN).map(|_| "🦊").collect::<String>();
        let expected = format!("'{expected_content}...'");

        assert_eq!(expected, output);
    }
}
