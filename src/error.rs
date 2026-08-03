//! The error every fallible call in this crate returns.
//!
//! Each module owns a typed error describing what can go wrong inside it -
//! [`TransportError`], [`ParseError`], [`PuzzleError`] - and [`Error`] is the
//! union, plus the cases that only exist once a reply is read in context.
//! Sources are kept intact, so walking `source` gets to whatever really
//! happened.

use crate::{
    http::TransportError,
    parse::{self, ParseError},
    puzzle::{Puzzle, PuzzleError},
};
use std::time::Duration;

/// Anything that can stop a request to Advent of Code from producing an
/// answer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The request never completed.
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// The reply could not be interpreted.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// The coordinates do not name a puzzle.
    #[error(transparent)]
    Puzzle(#[from] PuzzleError),

    /// The session cookie is missing, expired or not accepted.
    #[error("advent of code did not accept the session cookie; it is missing, expired or invalid")]
    Unauthorized,

    /// The puzzle has not unlocked yet.
    #[error("{puzzle} has not unlocked yet")]
    Locked {
        /// The puzzle that is still locked.
        puzzle: Puzzle,
    },

    /// An answer was submitted too recently for this one to be judged.
    ///
    /// Nothing was judged: the answer still has to be submitted again once the
    /// wait is over.
    #[error("an answer was submitted too recently; {} left to wait", parse::describe(*wait))]
    Cooldown {
        /// How much of the wait the site says is left. Zero if the site did
        /// not say.
        wait: Duration,
    },

    /// Advent of Code replied with a status this crate does not expect.
    #[error("advent of code replied with http status {status}")]
    Status {
        /// The status code.
        status: u16,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cooldown_says_how_long_is_left() {
        let error = Error::Cooldown {
            wait: Duration::from_secs(270),
        };

        assert_eq!(
            error.to_string(),
            "an answer was submitted too recently; 4m 30s left to wait"
        );
    }

    #[test]
    fn a_locked_puzzle_names_itself() {
        let error = Error::Locked {
            puzzle: Puzzle::at(2024, 7).expect("2024 has a day 7"),
        };

        assert_eq!(error.to_string(), "2024 day 7 has not unlocked yet");
    }

    #[test]
    fn a_transport_failure_keeps_its_cause() {
        let error = Error::from(TransportError::Request {
            url: "https://adventofcode.com/2024/day/7/input".to_owned(),
            source: "connection reset".into(),
        });

        // `transparent` forwards both the message and the cause, so the URL
        // that failed and the reason it failed both survive.
        assert!(error.to_string().contains("/2024/day/7/input"), "{error}");
        assert_eq!(
            std::error::Error::source(&error).map(ToString::to_string),
            Some("connection reset".to_owned())
        );
    }
}
