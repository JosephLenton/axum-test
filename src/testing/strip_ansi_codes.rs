use regex::Regex;

pub fn strip_ansi_codes<S>(s: S) -> String
where
    S: AsRef<str>,
{
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s.as_ref(), "").to_string()
}
