use crate::testing::strip_ansi_codes;
use pretty_assertions::assert_str_eq;

pub fn assert_error_message<E, O>(expected: E, output: O)
where
    E: AsRef<str>,
    O: AsRef<str>,
{
    let output_str = strip_ansi_codes(output);
    assert_str_eq!(expected.as_ref(), output_str);
}
