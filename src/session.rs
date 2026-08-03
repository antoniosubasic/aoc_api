//! The endpoints.
//!
//! A [`Session`] holds the session cookie and the one HTTP client built from
//! it, and every request the crate makes goes out through it. Which puzzle a
//! call is about is an argument rather than part of the session, so one
//! session serves a whole event without rebuilding a client per call.
//!
//! Nothing here is throttled or cached; see the crate documentation for why
//! that is the caller's decision.

use crate::{
    error::Error,
    http::{ClientOptions, Request, ReqwestTransport, Response, Transport},
    parse::{self, Hint, Submission},
    puzzle::{BASE_URL, Part, Puzzle, Year},
};
use std::{collections::BTreeMap, fmt, time::Duration};

/// How Advent of Code judged a submitted answer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// The answer was accepted.
    Correct,

    /// The answer was rejected.
    Incorrect {
        /// Which way the answer was wrong, when the site says.
        hint: Option<Hint>,
        /// How long the site asks you to wait before trying again, when it
        /// says.
        wait: Option<Duration>,
    },

    /// The part was already solved, so the site refused to judge the
    /// submission at all.
    ///
    /// The answer was compared against the one the puzzle page shows as
    /// accepted instead, which costs one further request.
    AlreadyComplete {
        /// Whether the submitted answer is the accepted one.
        correct: bool,
    },
}

impl Verdict {
    /// Whether the submitted answer is the right one.
    #[must_use]
    pub const fn is_correct(&self) -> bool {
        matches!(
            self,
            Self::Correct | Self::AlreadyComplete { correct: true }
        )
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Correct => f.write_str("that's the right answer"),
            Self::Incorrect { hint, wait } => {
                f.write_str("that's not the right answer")?;
                if let Some(hint) = hint {
                    write!(f, "; it is {hint}")?;
                }
                if let Some(wait) = wait {
                    write!(f, " (wait {} before trying again)", parse::describe(*wait))?;
                }
                Ok(())
            }
            Self::AlreadyComplete { correct: true } => {
                f.write_str("already solved, with this answer")
            }
            Self::AlreadyComplete { correct: false } => {
                f.write_str("already solved, with a different answer")
            }
        }
    }
}

/// An authenticated conversation with Advent of Code.
///
/// ```no_run
/// use aoc_api::{Part, Puzzle, Session};
///
/// # async fn example() -> Result<(), aoc_api::Error> {
/// let session = Session::new("53616c7465645f5f...", "github.com/my-username/my-repo by me@example.com")?;
/// let puzzle = Puzzle::at(2024, 7)?;
///
/// let input = session.input_text(puzzle).await?;
/// let verdict = session.submit(puzzle, Part::One, "3749").await?;
///
/// println!("{} bytes of input, and {verdict}", input.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Session<T = ReqwestTransport> {
    transport: T,
}

impl Session {
    /// Opens a session authenticated with the given session cookie.
    ///
    /// The cookie is the value of the `session` cookie on `adventofcode.com`
    /// while logged in. It is a credential: treat it like a password.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the cookie cannot be sent in a header
    /// or the HTTP client cannot be built.
    pub fn new(cookie: &str, identification: &str) -> Result<Self, Error> {
        Self::configured(&ClientOptions::new(cookie, identification))
    }

    /// Opens a session with the client configured explicitly.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the options cannot produce a client.
    pub fn configured(options: &ClientOptions) -> Result<Self, Error> {
        Ok(Self {
            transport: ReqwestTransport::new(options)?,
        })
    }
}

impl<T> Session<T> {
    /// Opens a session on a transport of your own, usually
    /// [`FakeTransport`](crate::http::fake::FakeTransport) in a test.
    pub const fn with_transport(transport: T) -> Self {
        Self { transport }
    }

    /// The transport every request goes out through.
    pub const fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Session<T> {
    /// Downloads a puzzle's personal input, without its trailing newline.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unauthorized`] if the cookie was not accepted,
    /// [`Error::Locked`] if the puzzle has not unlocked yet, or
    /// [`Error::Transport`] if the request failed.
    pub async fn input_text(&self, puzzle: Puzzle) -> Result<String, Error> {
        let body = self.get(puzzle.input_url(), Some(puzzle)).await?;

        Ok(body.trim_end_matches('\n').to_owned())
    }

    /// Downloads a puzzle's personal input as lines.
    ///
    /// # Errors
    ///
    /// As [`Session::input_text`].
    pub async fn input_lines(&self, puzzle: Puzzle) -> Result<Vec<String>, Error> {
        Ok(self
            .input_text(puzzle)
            .await?
            .lines()
            .map(ToOwned::to_owned)
            .collect())
    }

    /// Every sample block on a puzzle's page, in the order they appear.
    ///
    /// # Errors
    ///
    /// As [`Session::input_text`]. An unsolved puzzle still has samples, but a
    /// locked one has no page at all.
    pub async fn samples(&self, puzzle: Puzzle) -> Result<Vec<String>, Error> {
        Ok(parse::samples(&self.page(puzzle).await?))
    }

    /// The `nth` sample block on a puzzle's page, counting from one.
    ///
    /// # Errors
    ///
    /// As [`Session::samples`], plus [`Error::Parse`] if the page has fewer
    /// sample blocks than that.
    pub async fn sample_text(&self, puzzle: Puzzle, nth: u8) -> Result<String, Error> {
        Ok(parse::sample(&self.page(puzzle).await?, nth)?)
    }

    /// The `nth` sample block on a puzzle's page, as lines.
    ///
    /// # Errors
    ///
    /// As [`Session::sample_text`].
    pub async fn sample_lines(&self, puzzle: Puzzle, nth: u8) -> Result<Vec<String>, Error> {
        Ok(self
            .sample_text(puzzle, nth)
            .await?
            .lines()
            .map(ToOwned::to_owned)
            .collect())
    }

    /// How many stars this account has earned in each event.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unauthorized`] if the cookie was not accepted,
    /// [`Error::Parse`] if the page listed no events, or [`Error::Transport`]
    /// if the request failed.
    pub async fn stars(&self) -> Result<BTreeMap<Year, u8>, Error> {
        let body = self.get(format!("{BASE_URL}/events"), None).await?;

        Ok(parse::stars(&body)?)
    }

    /// The answer a puzzle's page shows as accepted for `part`.
    ///
    /// # Errors
    ///
    /// As [`Session::samples`], plus [`Error::Parse`] if that part is not
    /// solved yet.
    pub async fn accepted_answer(&self, puzzle: Puzzle, part: Part) -> Result<String, Error> {
        Ok(parse::accepted_answer(&self.page(puzzle).await?, part)?)
    }

    /// Submits an answer and reports how the site judged it.
    ///
    /// A rejected answer is a [`Verdict`], not an error. If the part turns out
    /// to be solved already the site refuses to judge anything, so the answer
    /// is compared against the accepted one on the puzzle page, which costs a
    /// second request.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Cooldown`] if an answer was submitted too recently for
    /// this one to be judged, [`Error::Unauthorized`] if the cookie was not
    /// accepted, [`Error::Locked`] if the puzzle has not unlocked yet,
    /// [`Error::Parse`] if the reply was not one this crate knows, or
    /// [`Error::Transport`] if the request failed.
    pub async fn submit(&self, puzzle: Puzzle, part: Part, answer: &str) -> Result<Verdict, Error> {
        let request = Request::post_form(
            puzzle.answer_url(),
            vec![
                ("level".to_owned(), part.number().to_string()),
                ("answer".to_owned(), answer.to_owned()),
            ],
        );

        let body = self.send(request, Some(puzzle)).await?;

        match parse::submission(&body)? {
            Submission::Correct => Ok(Verdict::Correct),
            Submission::Incorrect { hint, wait } => Ok(Verdict::Incorrect { hint, wait }),
            Submission::TooRecent { wait } => Err(Error::Cooldown { wait }),
            Submission::LoggedOut => Err(Error::Unauthorized),
            Submission::AlreadyComplete => {
                let accepted = self.accepted_answer(puzzle, part).await?;

                Ok(Verdict::AlreadyComplete {
                    correct: accepted.trim() == answer.trim(),
                })
            }
        }
    }

    /// A puzzle's page.
    async fn page(&self, puzzle: Puzzle) -> Result<String, Error> {
        self.get(puzzle.url(), Some(puzzle)).await
    }

    /// Reads a URL.
    async fn get(&self, url: String, puzzle: Option<Puzzle>) -> Result<String, Error> {
        self.send(Request::get(url), puzzle).await
    }

    /// Sends a request and returns the body, once the reply is known to be one
    /// worth reading.
    async fn send(&self, request: Request, puzzle: Option<Puzzle>) -> Result<String, Error> {
        let response = self.transport.execute(request).await?;
        check(&response, puzzle)?;

        Ok(response.body)
    }
}

/// Turns a reply that is not worth reading into the reason it is not.
///
/// The site distinguishes very little by status code, so the body has the last
/// word - and it has it first, because a rejected cookie is not always a
/// rejected request. The input endpoint refuses with a `400`, while the puzzle
/// and events pages answer `200` with a perfectly ordinary page that happens
/// to offer a log-in link. Both mean the same thing, and saying so beats
/// failing later on a page that turned out to have no puzzle in it.
fn check(response: &Response, puzzle: Option<Puzzle>) -> Result<(), Error> {
    if parse::is_logged_out(&response.body) {
        return Err(Error::Unauthorized);
    }

    if response.is_success() {
        return Ok(());
    }

    match (response.status, puzzle) {
        (401 | 403, _) => Err(Error::Unauthorized),
        (404, Some(puzzle)) => Err(Error::Locked { puzzle }),
        (status, _) => Err(Error::Status { status }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{http::fake::FakeTransport, parse::ParseError};

    const PUZZLE: &str = include_str!("../tests/fixtures/puzzle-day.html");
    const EVENTS: &str = include_str!("../tests/fixtures/events.html");
    const EVENTS_LOGGED_OUT: &str = include_str!("../tests/fixtures/events-logged-out.html");
    const CORRECT: &str = include_str!("../tests/fixtures/submit-correct.html");
    const WRONG: &str = include_str!("../tests/fixtures/submit-wrong.html");
    const COOLDOWN: &str = include_str!("../tests/fixtures/submit-cooldown.html");
    const COMPLETED: &str = include_str!("../tests/fixtures/submit-already-complete.html");
    const LOGGED_OUT: &str = include_str!("../tests/fixtures/logged-out.html");

    fn puzzle() -> Puzzle {
        Puzzle::at(2020, 1).expect("2020 has a day 1")
    }

    fn session(transport: FakeTransport) -> Session<FakeTransport> {
        Session::with_transport(transport)
    }

    #[tokio::test]
    async fn an_input_is_fetched_from_the_input_endpoint_without_its_trailing_newline() {
        let session = session(FakeTransport::serving("1721\n979\n366\n"));

        let input = session
            .input_text(puzzle())
            .await
            .expect("the fake replies");

        assert_eq!(input, "1721\n979\n366");
        assert_eq!(
            session.transport().requested_urls(),
            ["https://adventofcode.com/2020/day/1/input"]
        );
    }

    #[tokio::test]
    async fn an_input_can_be_read_as_lines() {
        let session = session(FakeTransport::serving("1721\n979\n366\n"));

        let lines = session
            .input_lines(puzzle())
            .await
            .expect("the fake replies");

        assert_eq!(lines, ["1721", "979", "366"]);
    }

    #[tokio::test]
    async fn samples_come_from_the_puzzle_page() {
        let session = session(FakeTransport::serving(PUZZLE));

        let sample = session
            .sample_lines(puzzle(), 1)
            .await
            .expect("the fake replies");

        assert_eq!(sample, ["1721", "979", "366", "299", "675", "1456"]);
        assert_eq!(
            session.transport().requested_urls(),
            ["https://adventofcode.com/2020/day/1"]
        );
    }

    #[tokio::test]
    async fn stars_are_read_from_the_events_page() {
        let session = session(FakeTransport::serving(EVENTS));

        let stars = session.stars().await.expect("the fake replies");

        assert_eq!(stars.values().copied().collect::<Vec<_>>(), [0, 9, 50]);
        assert_eq!(
            session.transport().requested_urls(),
            ["https://adventofcode.com/events"]
        );
    }

    #[tokio::test]
    async fn an_accepted_answer_is_posted_to_the_answer_endpoint() {
        let session = session(FakeTransport::serving(CORRECT));

        let verdict = session
            .submit(puzzle(), Part::Two, "241861950")
            .await
            .expect("the fake replies");

        assert_eq!(verdict, Verdict::Correct);
        assert!(verdict.is_correct());

        let request = session
            .transport()
            .requests()
            .pop()
            .expect("exactly one request");
        assert_eq!(request.url, "https://adventofcode.com/2020/day/1/answer");
        assert_eq!(
            request.form,
            [
                ("level".to_owned(), "2".to_owned()),
                ("answer".to_owned(), "241861950".to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn a_rejected_answer_is_a_verdict_rather_than_an_error() {
        let session = session(FakeTransport::serving(WRONG));

        let verdict = session
            .submit(puzzle(), Part::One, "0")
            .await
            .expect("the fake replies");

        assert_eq!(
            verdict,
            Verdict::Incorrect {
                hint: None,
                wait: Some(Duration::from_secs(60)),
            }
        );
        assert!(!verdict.is_correct());
    }

    #[tokio::test]
    async fn a_submission_on_cooldown_reports_the_remaining_wait() {
        let session = session(FakeTransport::serving(COOLDOWN));

        let error = session
            .submit(puzzle(), Part::One, "514579")
            .await
            .expect_err("nothing was judged");

        assert!(matches!(
            error,
            Error::Cooldown { wait } if wait == Duration::from_secs(270)
        ));
    }

    #[tokio::test]
    async fn an_answer_to_a_solved_part_is_checked_against_the_puzzle_page() {
        let transport = FakeTransport::new();
        transport.push_body(COMPLETED).push_body(PUZZLE);
        let session = session(transport);

        let verdict = session
            .submit(puzzle(), Part::One, "514579")
            .await
            .expect("the fake replies twice");

        assert_eq!(verdict, Verdict::AlreadyComplete { correct: true });
        assert_eq!(
            session.transport().requested_urls(),
            [
                "https://adventofcode.com/2020/day/1/answer",
                "https://adventofcode.com/2020/day/1",
            ]
        );
    }

    #[tokio::test]
    async fn a_different_answer_to_a_solved_part_is_still_wrong() {
        let transport = FakeTransport::new();
        transport.push_body(COMPLETED).push_body(PUZZLE);

        let verdict = session(transport)
            .submit(puzzle(), Part::One, "1")
            .await
            .expect("the fake replies twice");

        assert_eq!(verdict, Verdict::AlreadyComplete { correct: false });
        assert!(!verdict.is_correct());
    }

    #[tokio::test]
    async fn a_reply_asking_for_a_login_is_an_expired_cookie_whatever_its_status() {
        let transport = FakeTransport::new();
        transport.push(Response::new(400, LOGGED_OUT));

        let error = session(transport)
            .input_text(puzzle())
            .await
            .expect_err("the cookie was not accepted");

        assert!(matches!(error, Error::Unauthorized), "{error}");
    }

    #[tokio::test]
    async fn a_page_offering_a_login_is_an_expired_cookie_even_at_status_200() {
        let session = session(FakeTransport::serving(EVENTS_LOGGED_OUT));

        let error = session
            .stars()
            .await
            .expect_err("the page has no stars because nobody is logged in");

        assert!(matches!(error, Error::Unauthorized), "{error}");
    }

    #[tokio::test]
    async fn a_puzzle_that_has_not_unlocked_says_so() {
        let transport = FakeTransport::new();
        transport.push(Response::new(
            404,
            "Please don't repeatedly request this endpoint before it unlocks!",
        ));

        let error = session(transport)
            .input_text(puzzle())
            .await
            .expect_err("the puzzle is locked");

        assert!(matches!(error, Error::Locked { puzzle } if puzzle == self::puzzle()));
    }

    #[tokio::test]
    async fn an_unexpected_status_is_reported_as_itself() {
        let transport = FakeTransport::new();
        transport.push(Response::new(500, "<h1>Internal Server Error</h1>"));

        let error = session(transport)
            .stars()
            .await
            .expect_err("the site is unwell");

        assert!(matches!(error, Error::Status { status: 500 }), "{error}");
    }

    #[tokio::test]
    async fn a_reply_the_parser_does_not_know_is_an_error_rather_than_a_guess() {
        let session = session(FakeTransport::serving("<p>Ho ho ho.</p>"));

        let error = session
            .submit(puzzle(), Part::One, "514579")
            .await
            .expect_err("the reply is unrecognised");

        assert!(
            matches!(error, Error::Parse(ParseError::Submission { .. })),
            "{error}"
        );
    }

    #[tokio::test]
    async fn a_transport_failure_is_reported_rather_than_retried() {
        let session = session(FakeTransport::new());

        let error = session
            .input_text(puzzle())
            .await
            .expect_err("nothing was queued");

        assert!(matches!(error, Error::Transport(_)), "{error}");
        assert_eq!(session.transport().requests().len(), 1);
    }

    #[test]
    fn a_verdict_reads_like_the_site_phrases_it() {
        assert_eq!(Verdict::Correct.to_string(), "that's the right answer");
        assert_eq!(
            Verdict::Incorrect {
                hint: Some(Hint::TooHigh),
                wait: Some(Duration::from_secs(60)),
            }
            .to_string(),
            "that's not the right answer; it is too high (wait 1m before trying again)"
        );
        assert_eq!(
            Verdict::AlreadyComplete { correct: true }.to_string(),
            "already solved, with this answer"
        );
    }
}
