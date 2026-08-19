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
use std::{fmt, sync::Arc, time::Duration};

const PUZZLE_PAGE: &str = include_str!("fixtures/puzzle-day.html");
const EVENTS: &str = include_str!("fixtures/events.html");
const ALREADY_COMPLETE: &str = include_str!("fixtures/submit-already-complete.html");
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
/// a transport of its own calls the endpoints directly. Same replies, same
/// answers, same wrong answer in the middle - so the day reads the same on
/// either surface.
#[test]
fn a_whole_day_can_be_solved_without_a_session_too() {
    let transport = FakeTransport::new();
    transport
        .push_body("1721\n979\n366\n299\n675\n1456\n")
        .push_body(TOO_HIGH)
        .push_body(CORRECT);

    block_on(async {
        let input = session::input_lines(&transport, puzzle())
            .await
            .expect("the input");
        assert_eq!(input.len(), 6);

        let rejected = session::submit(&transport, puzzle(), Part::One, "999999")
            .await
            .expect("a judged answer");
        assert!(!rejected.is_correct());

        let accepted = session::submit(&transport, puzzle(), Part::One, "514579")
            .await
            .expect("a judged answer");
        assert_eq!(accepted, Verdict::Correct);
    });

    assert_eq!(
        transport.requested_urls(),
        [
            "https://adventofcode.com/2020/day/1/input",
            "https://adventofcode.com/2020/day/1/answer",
            "https://adventofcode.com/2020/day/1/answer",
        ]
    );
}

/// Drives one endpoint through both surfaces, on transports serving the same
/// replies, and asserts they agree on what came back and on what went out.
fn both_ways<T: fmt::Debug>(
    bodies: &[&str],
    through_session: impl AsyncFnOnce(&Session<FakeTransport>) -> T,
    directly: impl AsyncFnOnce(&FakeTransport) -> T,
) {
    let (queued, same) = (FakeTransport::new(), FakeTransport::new());
    for body in bodies {
        queued.push_body(*body);
        same.push_body(*body);
    }

    let session = Session::with_transport(queued);
    let from_method = block_on(through_session(&session));
    let from_function = block_on(directly(&same));

    // `Error` is not `PartialEq`, and the point is that the two are
    // indistinguishable, so compare what they print.
    assert_eq!(format!("{from_method:?}"), format!("{from_function:?}"));
    assert_eq!(session.transport().requests(), same.requests());
}

/// The two surfaces are one implementation, so neither can grow a behaviour
/// the other lacks - not a different URL, not a different reply, not a
/// different error. Every endpoint is checked, because prose alone would not
/// stop a method from quietly growing a retry of its own.
#[test]
fn every_method_makes_the_same_request_as_its_free_function() {
    let input = "1721\n979\n366\n";

    both_ways(
        &[input],
        async |session| session.input_text(puzzle()).await,
        async |transport| session::input_text(transport, puzzle()).await,
    );
    both_ways(
        &[input],
        async |session| session.input_lines(puzzle()).await,
        async |transport| session::input_lines(transport, puzzle()).await,
    );
    both_ways(
        &[PUZZLE_PAGE],
        async |session| session.samples(puzzle()).await,
        async |transport| session::samples(transport, puzzle()).await,
    );
    both_ways(
        &[PUZZLE_PAGE],
        async |session| session.sample_text(puzzle(), 1).await,
        async |transport| session::sample_text(transport, puzzle(), 1).await,
    );
    both_ways(
        &[PUZZLE_PAGE],
        async |session| session.sample_lines(puzzle(), 1).await,
        async |transport| session::sample_lines(transport, puzzle(), 1).await,
    );
    both_ways(
        &[EVENTS],
        async |session| session.stars().await,
        async |transport| session::stars(transport).await,
    );
    both_ways(
        &[PUZZLE_PAGE],
        async |session| session.accepted_answer(puzzle(), Part::Two).await,
        async |transport| session::accepted_answer(transport, puzzle(), Part::Two).await,
    );
    both_ways(
        &[CORRECT],
        async |session| session.submit(puzzle(), Part::One, "514579").await,
        async |transport| session::submit(transport, puzzle(), Part::One, "514579").await,
    );

    // The one call that makes two requests: both surfaces must make both.
    both_ways(
        &[ALREADY_COMPLETE, PUZZLE_PAGE],
        async |session| session.submit(puzzle(), Part::Two, "241861950").await,
        async |transport| session::submit(transport, puzzle(), Part::Two, "241861950").await,
    );
}

/// The reason the free functions exist: a tool that keeps one transport in a
/// type of its own, shared between tasks, passes it as it holds it.
#[test]
fn a_transport_behind_a_pointer_drives_the_endpoints_as_it_is() {
    struct Tool {
        transport: Arc<FakeTransport>,
    }

    let tool = Tool {
        transport: Arc::new(FakeTransport::serving("1721\n979\n366\n")),
    };

    let input = block_on(session::input_text(&tool.transport, puzzle())).expect("the input");

    assert_eq!(input, "1721\n979\n366");
    assert_eq!(
        tool.transport.requested_urls(),
        ["https://adventofcode.com/2020/day/1/input"]
    );
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
