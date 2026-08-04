//! The public API, driven the way a downstream tool drives it.
//!
//! These run through [`FakeTransport`], so nothing here contacts
//! `adventofcode.com` - which is also the point: a tool built on this crate
//! can test itself the same way.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use aoc_api::{
    Error, Part, Puzzle, Session, Verdict,
    http::{Response, fake::FakeTransport},
    session,
};
use std::time::Duration;

const PUZZLE_PAGE: &str = include_str!("fixtures/puzzle-day.html");
const CORRECT: &str = include_str!("fixtures/submit-correct.html");
const TOO_HIGH: &str = include_str!("fixtures/submit-too-high.html");
const COOLDOWN: &str = include_str!("fixtures/submit-cooldown.html");
const LOGGED_OUT: &str = include_str!("fixtures/logged-out.html");

fn puzzle() -> Puzzle {
    Puzzle::at(2020, 1).expect("2020 has a day 1")
}

/// Drives a future to completion on a current-thread runtime, the way a
/// synchronous program consumes this crate.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a current-thread runtime")
        .block_on(future)
}

#[test]
fn a_whole_day_can_be_solved_without_touching_the_network() {
    let transport = FakeTransport::new();
    transport
        .push_body("1721\n979\n366\n299\n675\n1456\n")
        .push_body(TOO_HIGH)
        .push_body(CORRECT);
    let session = Session::with_transport(transport);

    block_on(async {
        let input = session.input_lines(puzzle()).await.expect("the input");
        assert_eq!(input.len(), 6);

        let rejected = session
            .submit(puzzle(), Part::One, "999999")
            .await
            .expect("a judged answer");
        assert!(!rejected.is_correct());

        let accepted = session
            .submit(puzzle(), Part::One, "514579")
            .await
            .expect("a judged answer");
        assert_eq!(accepted, Verdict::Correct);
    });

    assert_eq!(
        session.transport().requested_urls(),
        [
            "https://adventofcode.com/2020/day/1/input",
            "https://adventofcode.com/2020/day/1/answer",
            "https://adventofcode.com/2020/day/1/answer",
        ]
    );
}

/// The same day again, without a `Session` anywhere: a tool that already keeps
/// a transport of its own calls the endpoints directly.
#[test]
fn a_whole_day_can_be_solved_without_a_session_too() {
    let transport = FakeTransport::new();
    transport
        .push_body("1721\n979\n366\n299\n675\n1456\n")
        .push_body(PUZZLE_PAGE)
        .push_body(CORRECT);

    block_on(async {
        let input = session::input_lines(&transport, puzzle())
            .await
            .expect("the input");
        assert_eq!(input.len(), 6);

        let sample = session::sample_lines(&transport, puzzle(), 1)
            .await
            .expect("the first sample");
        assert_eq!(sample[0], "1721");

        let verdict = session::submit(&transport, puzzle(), Part::One, "514579")
            .await
            .expect("a judged answer");
        assert_eq!(verdict, Verdict::Correct);
    });

    assert_eq!(
        transport.requested_urls(),
        [
            "https://adventofcode.com/2020/day/1/input",
            "https://adventofcode.com/2020/day/1",
            "https://adventofcode.com/2020/day/1/answer",
        ]
    );
}

/// The two surfaces are one implementation, so neither can grow a behaviour
/// the other lacks - not a different URL, not a different error.
#[test]
fn a_method_and_its_free_function_make_the_same_request() {
    let through_session = {
        let session = Session::with_transport(FakeTransport::serving(PUZZLE_PAGE));
        let answer = block_on(session.accepted_answer(puzzle(), Part::Two)).expect("an answer");

        (answer, session.transport().requests())
    };

    let directly = {
        let transport = FakeTransport::serving(PUZZLE_PAGE);
        let answer =
            block_on(session::accepted_answer(&transport, puzzle(), Part::Two)).expect("an answer");

        (answer, transport.requests())
    };

    assert_eq!(through_session, directly);
}

#[test]
fn a_cooldown_is_an_error_a_caller_can_branch_on() {
    let session = Session::with_transport(FakeTransport::serving(COOLDOWN));

    let error =
        block_on(session.submit(puzzle(), Part::Two, "241861950")).expect_err("nothing was judged");

    match error {
        Error::Cooldown { wait } => assert_eq!(wait, Duration::from_secs(270)),
        other => panic!("expected a cooldown, got {other}"),
    }
}

#[test]
fn an_expired_cookie_is_an_error_a_caller_can_branch_on() {
    let transport = FakeTransport::new();
    transport.push(Response::new(400, LOGGED_OUT));
    let session = Session::with_transport(transport);

    let error = block_on(session.input_text(puzzle())).expect_err("the cookie was not accepted");

    assert!(matches!(error, Error::Unauthorized), "{error}");
}

#[test]
fn a_coordinate_that_is_not_a_puzzle_never_reaches_the_transport() {
    let session = Session::with_transport(FakeTransport::new());

    assert!(Puzzle::at(2025, 25).is_err());
    assert!(Puzzle::at(2014, 1).is_err());
    assert!(session.transport().requests().is_empty());
}

#[test]
fn samples_and_accepted_answers_come_off_the_puzzle_page() {
    let transport = FakeTransport::new();
    transport.push_body(PUZZLE_PAGE).push_body(PUZZLE_PAGE);
    let session = Session::with_transport(transport);

    block_on(async {
        let samples = session.samples(puzzle()).await.expect("two samples");
        assert_eq!(samples.len(), 2);
        assert!(samples[0].starts_with("1721"));

        let answer = session
            .accepted_answer(puzzle(), Part::Two)
            .await
            .expect("part two is solved on this page");
        assert_eq!(answer, "241861950");
    });
}
