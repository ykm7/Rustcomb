use std::str::FromStr;

use regex::Regex;

use crate::MyErrors;

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum SearchMode {
    #[clap(name = "literal", help = "Match exact literal strings")]
    Literal,
    #[clap(name = "regex", help = "Use regular expression patterns")]
    Regex,
}

impl FromStr for SearchMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "literal" => Ok(SearchMode::Literal),
            "regex" => Ok(SearchMode::Regex),
            _ => Err(
                "Not an expected conversion string. Required to be either [literal or regex]"
                    .to_string(),
            ),
        }
    }
}
/**
 * Use a single initialised re pattern to save it being created on each call (STAR_PATTERN)
 *
 * re.Replace() returns a COW (Copy on Write) type.
 * So a new string is only created when its modified.
 * Can be used whether using the Borrowed or the Owned
 */
pub fn clean_up_regex(
    pattern: Option<&str>,
    mode: SearchMode,
) -> Result<Option<regex::Regex>, MyErrors> {
    pattern
        .map(|pat| {
            let s = match mode {
                SearchMode::Literal => regex::escape(pat),
                SearchMode::Regex => pat.to_string(),
            };
            Regex::new(&s).map_err(MyErrors::Regex)
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{SearchMode, clean_up_regex};

    #[test]
    fn test_none() {
        let pattern = None;
        let result = clean_up_regex(pattern, SearchMode::Literal).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_literal() {
        let pattern = Some("Hello[ ]World");
        let result = clean_up_regex(pattern, SearchMode::Literal)
            .unwrap()
            .unwrap();
        assert_eq!(result.as_str(), "Hello\\[ \\]World");
    }

    #[test]
    fn test_regex() {
        let pattern = Some("Hello[ ]World");
        let result = clean_up_regex(pattern, SearchMode::Regex).unwrap().unwrap();
        assert_eq!(result.as_str(), "Hello[ ]World");
    }
}
