use crate::error::{Error, ErrorKind};
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{self, Display},
    slice,
    str::FromStr,
};

/// Canonical Rust Paths (sans parameters) for cataloguing vulnerable functions.
/// <https://doc.rust-lang.org/reference/paths.html#canonical-paths>
// TODO: find a crate which provides a better type for representing these?
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct FunctionPath(Vec<Identifier>);

impl FunctionPath {
    /// Get the crate name this function is located in
    pub fn crate_name(&self) -> &str {
        self.iter()
            .next()
            .expect("FunctionPath must have 2 or more segments")
            .as_str()
    }

    /// Convert this path into an owned vector of `Identifier`s
    pub fn into_vec(self) -> Vec<Identifier> {
        self.0
    }

    /// Iterate over the segments of this path
    pub fn iter(&self) -> Iter {
        self.0.iter()
    }

    /// Borrow the segments of this path
    pub fn segments(&self) -> &[Identifier] {
        self.0.as_slice()
    }
}

impl Display for FunctionPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut segments = self.iter();

        let crate_name = segments
            .next()
            .expect("FunctionPath must have 2 or more segments");

        write!(f, "{}", crate_name.as_str())?;

        for segment in segments {
            write!(f, "::{}", segment.as_str())?;
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for FunctionPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_str(&String::deserialize(deserializer)?)
            .map_err(|e| D::Error::custom(format!("{}", e)))
    }
}

impl Serialize for FunctionPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for FunctionPath {
    type Err = Error;

    /// Parse a canonical, param-free function path contained in an advisory
    fn from_str(path: &str) -> Result<Self, Error> {
        let mut segments = vec![];

        for segment in path.split("::") {
            segments.push(Identifier::from_str(segment)?);
        }

        if segments.len() >= 2 {
            Ok(FunctionPath(segments))
        } else {
            fail!(
                ErrorKind::Parse,
                "paths must start with the crate name (i.e. minimum two segments): '{}'",
                path
            )
        }
    }
}

/// Iterator over the segments of a `FunctionPath`
pub type Iter<'a> = slice::Iter<'a, Identifier>;

/// Identifiers within paths. Note that the typical Rust path grammar supports
/// multiple types of path segments, however for the purposes of vulnerability
/// advisories we only care about identifiers.
/// <https://doc.rust-lang.org/reference/identifiers.html>
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    /// Borrow this identifier as a `str`
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for Identifier {
    type Err = Error;

    /// Parse an `Identifier` within a `FunctionPath`
    fn from_str(identifier: &str) -> Result<Self, Error> {
        validate_identifier(identifier)?;
        Ok(Identifier(identifier.into()))
    }
}

/// Validate an identifier within a path is valid
fn validate_identifier(identifier: &str) -> Result<(), Error> {
    let mut chars = identifier.chars();

    if let Some(first_char) = chars.next() {
        match first_char {
            'A'...'Z' | 'a'...'z' | '_' => (),
            _ => fail!(
                ErrorKind::Parse,
                "identifier must start with a letter: '{}'",
                identifier
            ),
        }
    } else {
        fail!(ErrorKind::Parse, "empty identifier in function path");
    }

    for c in chars {
        match c {
            'A'...'Z' | 'a'...'z' | '0'...'9' | '_' => (),
            '<' | '>' | '(' | ')' => fail!(
                ErrorKind::Parse,
                "omit parameters when specifying function paths: '{}'",
                identifier
            ),
            _ => fail!(
                ErrorKind::Parse,
                "invalid character in identifier: '{}'",
                identifier
            ),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::FunctionPath;
    use std::str::FromStr;

    const EXAMPLE_PATH_STR: &str = "foo::bar::baz";

    #[test]
    fn crate_name_test() {
        let path = FunctionPath::from_str(EXAMPLE_PATH_STR).unwrap();
        assert_eq!(path.crate_name(), "foo");
    }

    #[test]
    fn display_test() {
        let path = FunctionPath::from_str(EXAMPLE_PATH_STR).unwrap();
        assert_eq!(path.to_string(), EXAMPLE_PATH_STR)
    }

    #[test]
    fn from_str_test() {
        // Valid function paths
        assert!(FunctionPath::from_str("foo::bar").is_ok());
        assert!(FunctionPath::from_str("foo::bar::baz").is_ok());
        assert!(FunctionPath::from_str("foo::Bar::baz").is_ok());
        assert!(FunctionPath::from_str("foo::Bar::_baz").is_ok());
        assert!(FunctionPath::from_str("foo::Bar::_baz_").is_ok());
        assert!(FunctionPath::from_str("f00::B4r::_b4z_").is_ok());

        // Invalid function paths
        assert!(FunctionPath::from_str("minimum_two_components").is_err());
        assert!(FunctionPath::from_str("no-hyphens::foobar").is_err());
        assert!(FunctionPath::from_str("no_leading_digits::0rly").is_err());
    }
}
