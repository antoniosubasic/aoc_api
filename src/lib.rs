//! A typed client for [Advent of Code](https://adventofcode.com).
//!
//! It downloads puzzle inputs and samples, reads how many stars an account has
//! earned, and submits answers. The site has no API, so every reply is a page
//! meant for a browser; recognising them is [`parse`]'s job and nothing else's.
//!
//! ```no_run
//! use aoc_api::{Part, Puzzle, Session, Verdict};
//!
//! # async fn example() -> Result<(), aoc_api::Error> {
//! let session = Session::new("53616c7465645f5f...")?;
//! let puzzle = Puzzle::at(2024, 7)?;
//!
//! let input = session.input_text(puzzle).await?;
//!
//! match session.submit(puzzle, Part::One, "3749").await? {
//!     Verdict::Correct => println!("gold star"),
//!     verdict => println!("{verdict}"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Shape
//!
//! * [`Session`] holds the cookie and the one HTTP client built from it.
//!   Which puzzle a call is about is an argument, so one session serves a
//!   whole event.
//! * [`Puzzle`], [`Year`], [`Day`] and [`Part`] are validated newtypes, so an
//!   out-of-range coordinate cannot become a request.
//! * [`http::Transport`] is the seam everything external sits behind.
//!   [`http::fake::FakeTransport`] replays canned replies, which is how this
//!   crate's own tests run without a network or a cookie - and how a tool
//!   built on it can do the same.
//! * [`Error`] is the union of the typed errors each module owns.
//!
//! # Runtime
//!
//! The API is asynchronous and brings no runtime of its own: it runs on
//! whichever executor the caller already has, including a current-thread
//! `tokio` runtime driven to completion, which is how a synchronous program
//! should use it.
//!
//! # Automation etiquette
//!
//! This crate follows the Advent of Code [automation guidelines]. Two of them
//! are settled here, and two are deliberately left to you:
//!
//! * **Identification.** Every request carries a `User-Agent` naming this
//!   crate's repository, version and maintainer - not you, the caller. There
//!   is one place a client is built, [`http::ReqwestTransport::new`], and the
//!   header is baked into its default headers, so no request can go out
//!   without it.
//! * **No hidden traffic.** A request happens when you call a method, and
//!   never otherwise. Nothing polls, retries or prefetches.
//! * **Throttling is yours.** This crate does not sleep between requests,
//!   because a library cannot know how a program is being driven and a hidden
//!   delay in someone else's process is a poor surprise. Space out your calls;
//!   [`aoc-runtime`](https://crates.io/crates/aoc-runtime) is one example of
//!   doing it, with a persisted minimum gap.
//! * **Caching is yours.** Inputs are personal, permanent and unchanging, so
//!   download one once and keep it. This crate returns the body and forgets
//!   it.
//!
//! [automation guidelines]: https://www.reddit.com/r/adventofcode/wiki/faqs/automation

pub mod error;
pub mod http;
pub mod parse;
pub mod puzzle;
pub mod session;

pub use error::Error;
pub use parse::Hint;
pub use puzzle::{Day, Part, Puzzle, PuzzleError, Year};
pub use session::{Session, Verdict};
