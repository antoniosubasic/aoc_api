//! Validated puzzle coordinates.
//!
//! [`Year`], [`Day`] and [`Part`] have private fields and fallible
//! constructors, so an out-of-range value cannot become a request. [`Puzzle`]
//! checks the pairing on top of that: from [`Year::FIRST_SHORT`] on, the event
//! stops after [`Day::LAST_SHORT`], so day 25 of 2025 is not a puzzle even
//! though both halves are individually plausible.

use std::fmt;

/// The site every request in this crate is made against.
pub const BASE_URL: &str = "https://adventofcode.com";

/// A coordinate that does not name a puzzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PuzzleError {
    /// The year is before the first event.
    #[error("advent of code started in {first}, so there is no {year} event", first = Year::FIRST)]
    Year {
        /// The rejected year.
        year: u16,
    },

    /// The day is outside the range any event has ever had.
    #[error(
        "advent of code publishes puzzles on days {first} to {last}, so there is no day {day}",
        first = Day::FIRST,
        last = Day::LAST_FULL
    )]
    Day {
        /// The rejected day.
        day: u8,
    },

    /// Both halves are valid, but that event never published that day.
    #[error("the {year} event stops after day {last}, so there is no day {day}")]
    Pairing {
        /// The event.
        year: Year,
        /// The rejected day.
        day: Day,
        /// The last day the event published.
        last: Day,
    },

    /// The part is neither one nor two.
    #[error("a puzzle has two parts, so there is no part {part}")]
    Part {
        /// The rejected part.
        part: u8,
    },
}

/// A validated Advent of Code year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Year(u16);

impl Year {
    /// The first year Advent of Code ran.
    pub const FIRST: Self = Self(2015);

    /// The first shortened event. From 2025 on, Advent of Code publishes 12
    /// puzzles instead of 25.
    pub const FIRST_SHORT: Self = Self(2025);

    /// Creates a year, rejecting anything before [`Year::FIRST`].
    ///
    /// # Errors
    ///
    /// Returns [`PuzzleError::Year`] for a year that never had an event.
    pub const fn new(year: u16) -> Result<Self, PuzzleError> {
        if year >= Self::FIRST.0 {
            Ok(Self(year))
        } else {
            Err(PuzzleError::Year { year })
        }
    }

    /// The last day this event publishes a puzzle on.
    #[must_use]
    pub const fn last_day(self) -> Day {
        if self.0 >= Self::FIRST_SHORT.0 {
            Day::LAST_SHORT
        } else {
            Day::LAST_FULL
        }
    }

    /// Whether this event has a puzzle on the given day.
    #[must_use]
    pub const fn has_day(self, day: Day) -> bool {
        day.0 <= self.last_day().0
    }

    /// The underlying value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A validated Advent of Code day, always within `1..=25`.
///
/// The upper bound is the widest range any event has had; which days a
/// *particular* event has is [`Year::has_day`], enforced by [`Puzzle::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(u8);

impl Day {
    /// The first puzzle day.
    pub const FIRST: Self = Self(1);

    /// The last puzzle day of events up to and including 2024.
    pub const LAST_FULL: Self = Self(25);

    /// The last puzzle day from [`Year::FIRST_SHORT`] on.
    pub const LAST_SHORT: Self = Self(12);

    /// Creates a day, rejecting anything outside `1..=25`.
    ///
    /// # Errors
    ///
    /// Returns [`PuzzleError::Day`] for a day no event has ever published.
    pub const fn new(day: u8) -> Result<Self, PuzzleError> {
        if day >= Self::FIRST.0 && day <= Self::LAST_FULL.0 {
            Ok(Self(day))
        } else {
            Err(PuzzleError::Day { day })
        }
    }

    /// The underlying value.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Which half of a puzzle an answer belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    /// The first part.
    One,
    /// The second part, unlocked by solving the first.
    Two,
}

impl Part {
    /// The part number as understood by the Advent of Code API.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }

    /// The part's position among a puzzle's answers, counting from zero.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }

    /// Creates a part from the number the API uses.
    ///
    /// # Errors
    ///
    /// Returns [`PuzzleError::Part`] for anything other than `1` or `2`.
    pub const fn from_number(part: u8) -> Result<Self, PuzzleError> {
        match part {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            part => Err(PuzzleError::Part { part }),
        }
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "part {}", self.number())
    }
}

/// A specific puzzle: one day of one year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Puzzle {
    /// The event year.
    pub year: Year,
    /// The day within the event.
    pub day: Day,
}

impl Puzzle {
    /// Creates a puzzle, rejecting a day the event never published.
    ///
    /// Both coordinates are valid on their own, but not every pairing is: from
    /// [`Year::FIRST_SHORT`] on, the event stops after [`Day::LAST_SHORT`].
    ///
    /// # Errors
    ///
    /// Returns [`PuzzleError::Pairing`] if that event has no such day.
    pub const fn new(year: Year, day: Day) -> Result<Self, PuzzleError> {
        if year.has_day(day) {
            Ok(Self { year, day })
        } else {
            Err(PuzzleError::Pairing {
                year,
                day,
                last: year.last_day(),
            })
        }
    }

    /// Creates a puzzle from raw numbers, validating both and their pairing.
    ///
    /// ```
    /// use aoc_api::Puzzle;
    ///
    /// assert!(Puzzle::at(2024, 25).is_ok());
    /// assert!(Puzzle::at(2025, 25).is_err()); // the 2025 event stops at day 12
    /// assert!(Puzzle::at(1066, 1).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the [`PuzzleError`] describing whichever coordinate is wrong.
    pub const fn at(year: u16, day: u8) -> Result<Self, PuzzleError> {
        let year = match Year::new(year) {
            Ok(year) => year,
            Err(error) => return Err(error),
        };
        let day = match Day::new(day) {
            Ok(day) => day,
            Err(error) => return Err(error),
        };

        Self::new(year, day)
    }

    /// The canonical puzzle URL.
    #[must_use]
    pub fn url(self) -> String {
        format!("{BASE_URL}/{}/day/{}", self.year, self.day)
    }

    /// The URL the puzzle's personal input is downloaded from.
    #[must_use]
    pub fn input_url(self) -> String {
        format!("{}/input", self.url())
    }

    /// The URL answers are submitted to.
    #[must_use]
    pub fn answer_url(self) -> String {
        format!("{}/answer", self.url())
    }
}

impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} day {}", self.year, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn year(year: u16) -> Year {
        Year::new(year).expect("valid year")
    }

    fn day(day: u8) -> Day {
        Day::new(day).expect("valid day")
    }

    #[test]
    fn a_year_before_the_first_event_is_not_a_year() {
        assert_eq!(Year::new(2014), Err(PuzzleError::Year { year: 2014 }));
        assert_eq!(Year::new(2015).map(Year::get), Ok(2015));
        assert_eq!(Year::new(2024).map(Year::get), Ok(2024));
    }

    #[test]
    fn only_puzzle_days_are_days() {
        assert_eq!(Day::new(0), Err(PuzzleError::Day { day: 0 }));
        assert_eq!(Day::new(26), Err(PuzzleError::Day { day: 26 }));
        assert_eq!(Day::new(1).map(Day::get), Ok(1));
        assert_eq!(Day::new(25).map(Day::get), Ok(25));
    }

    #[test]
    fn events_from_2025_on_are_half_as_long() {
        assert_eq!(year(2015).last_day().get(), 25);
        assert_eq!(year(2024).last_day().get(), 25);
        assert_eq!(year(2025).last_day().get(), 12);
        assert_eq!(year(2030).last_day().get(), 12);

        assert!(year(2024).has_day(day(25)));
        assert!(year(2025).has_day(day(12)));
        assert!(!year(2025).has_day(day(13)));
    }

    #[test]
    fn a_day_its_event_never_published_is_not_a_puzzle() {
        assert!(Puzzle::new(year(2024), day(25)).is_ok());
        assert!(Puzzle::new(year(2025), day(12)).is_ok());
        assert_eq!(
            Puzzle::new(year(2025), day(25)),
            Err(PuzzleError::Pairing {
                year: year(2025),
                day: day(25),
                last: Day::LAST_SHORT,
            })
        );
    }

    #[test]
    fn raw_numbers_are_validated_before_they_can_become_a_request() {
        assert!(Puzzle::at(2024, 7).is_ok());
        assert_eq!(Puzzle::at(1066, 7), Err(PuzzleError::Year { year: 1066 }));
        assert_eq!(Puzzle::at(2024, 47), Err(PuzzleError::Day { day: 47 }));
        assert!(matches!(
            Puzzle::at(2025, 20),
            Err(PuzzleError::Pairing { .. })
        ));
    }

    #[test]
    fn a_puzzle_knows_its_three_urls() {
        let puzzle = Puzzle::at(2024, 5).expect("2024 has a day 5");

        assert_eq!(puzzle.url(), "https://adventofcode.com/2024/day/5");
        assert_eq!(
            puzzle.input_url(),
            "https://adventofcode.com/2024/day/5/input"
        );
        assert_eq!(
            puzzle.answer_url(),
            "https://adventofcode.com/2024/day/5/answer"
        );
    }

    #[test]
    fn parts_map_to_api_numbers() {
        assert_eq!(Part::One.number(), 1);
        assert_eq!(Part::Two.number(), 2);
        assert_eq!(Part::from_number(1), Ok(Part::One));
        assert_eq!(Part::from_number(2), Ok(Part::Two));
        assert_eq!(Part::from_number(3), Err(PuzzleError::Part { part: 3 }));
        assert_eq!(Part::Two.to_string(), "part 2");
    }

    #[test]
    fn coordinate_errors_say_which_coordinate_is_wrong() {
        assert!(
            PuzzleError::Year { year: 1066 }
                .to_string()
                .contains("no 1066 event")
        );
        assert!(
            PuzzleError::Day { day: 47 }
                .to_string()
                .contains("days 1 to 25")
        );
        assert!(
            PuzzleError::Pairing {
                year: year(2025),
                day: day(20),
                last: Day::LAST_SHORT,
            }
            .to_string()
            .contains("2025 event stops after day 12")
        );
    }
}
