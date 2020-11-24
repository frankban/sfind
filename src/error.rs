use std::fmt;

use crate::sf;

/// A generic application error with a message.
#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error {
            message: err.to_string(),
        }
    }
}

impl From<sf::Error> for Error {
    fn from(err: sf::Error) -> Error {
        Error {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error {
            message: String::from("bad wolf"),
        };
        assert_eq!(err.to_string(), "bad wolf");
    }

    #[test]
    fn error_from_serde_json_error() {
        let serde_err = serde_json::from_str::<i32>(":").unwrap_err();
        let err = Error::from(serde_err);
        assert_eq!(err.message, "expected value at line 1 column 1");
    }

    #[test]
    fn error_from_sf_error() {
        let err = Error::from(sf::Error::Message(String::from("bad wolf")));
        assert_eq!(err.message, "bad wolf");
    }
}
