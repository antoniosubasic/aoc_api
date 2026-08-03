# aoc_api

[![CI](https://github.com/antoniosubasic/aoc_api/actions/workflows/ci.yml/badge.svg)](https://github.com/antoniosubasic/aoc_api/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/aoc_api.svg)](https://crates.io/crates/aoc_api)
[![downloads](https://img.shields.io/crates/d/aoc_api.svg)](https://crates.io/crates/aoc_api)
[![license](https://img.shields.io/crates/l/aoc_api.svg)](LICENSE)

A typed client for [Advent of Code](https://adventofcode.com): downloads puzzle
inputs and samples, reads how many stars an account has earned, and submits
answers — identifying itself as the
[automation guidelines](https://www.reddit.com/r/adventofcode/wiki/faqs/automation)
ask. There is also a [C# version](https://github.com/antoniosubasic/AoC.API).

```console
$ cargo add aoc_api
```

```rust
use aoc_api::{Part, Puzzle, Session, Verdict};

let session = Session::new("53616c7465645f5f...", "github.com/my-username/my-repo by me@example.com")?;
let puzzle = Puzzle::at(2024, 7)?;

let input = session.input_text(puzzle).await?;

match session.submit(puzzle, Part::One, "3749").await? {
    Verdict::Correct => println!("gold star"),
    verdict => println!("{verdict}"),
}
```

The cookie is the value of the `session` cookie on `adventofcode.com` while
logged in ([how to find it](https://mmhaskell.com/blog/2023/1/30/advent-of-code-fetching-puzzle-input-using-the-api#authentication)).
It is a credential: treat it like a password. This crate marks it sensitive so
it stays out of header dumps, and never prints it in `Debug` output.

## What it does

| Call | Returns |
| --- | --- |
| `session.input_text(puzzle)` | the puzzle's personal input, without its trailing newline |
| `session.input_lines(puzzle)` | the same, as `Vec<String>` |
| `session.samples(puzzle)` | every sample block on the puzzle page |
| `session.sample_text(puzzle, nth)` | the `nth` sample block, counting from one |
| `session.sample_lines(puzzle, nth)` | the same, as `Vec<String>` |
| `session.stars()` | `BTreeMap<Year, u8>` — stars earned per event |
| `session.accepted_answer(puzzle, part)` | the answer the puzzle page shows as accepted |
| `session.submit(puzzle, part, answer)` | a [`Verdict`](#verdicts-and-errors) |

One session serves a whole event: it holds the cookie and the single HTTP
client built from it, and which puzzle a call is about is an argument.

### Validated coordinates

`Puzzle`, `Year`, `Day` and `Part` are newtypes with private fields and
fallible constructors, so an out-of-range coordinate cannot become a request.
The *pairing* is validated too — events up to 2024 run 25 puzzles, and from
2025 on they run 12:

```rust
Puzzle::at(2024, 25)?; // fine
Puzzle::at(2025, 25);  // Err: the 2025 event stops after day 12
Puzzle::at(1066, 1);   // Err: advent of code started in 2015
```

### Verdicts and errors

A rejected answer is a `Verdict`, not an error — being wrong is a normal
outcome:

| `Verdict` | Meaning |
| --- | --- |
| `Correct` | accepted |
| `Incorrect { hint, wait }` | rejected; `hint` is `TooHigh`/`TooLow` when the site says so, `wait` is how long it asks you to wait |
| `AlreadyComplete { correct }` | the part was already solved, so the site refused to judge; the answer was compared against the accepted one on the puzzle page instead |

`verdict.is_correct()` collapses that to a `bool` when that is all you need.

Everything that stops a call from producing an answer is an `Error` variant you
can branch on: `Transport` (the request failed), `Unauthorized` (the cookie is
missing, expired or invalid), `Locked` (the puzzle has not unlocked yet),
`Cooldown { wait: Duration }` (an answer was submitted too recently, so nothing
was judged), `Parse` (the reply was not one this crate recognises) and `Status`
(anything else the site returned). Each module owns its own error type and
`Error` is the union, with `source` chains intact.

### Testing without a network

`http::Transport` is the seam everything external sits behind, and
`http::fake::FakeTransport` replays canned replies. This crate's own tests run
entirely through it — no network, no session cookie — and it is public so a
tool built on this crate can do the same:

```rust
use aoc_api::{Puzzle, Session, http::fake::FakeTransport};

let session = Session::with_transport(FakeTransport::serving("1721\n979\n366\n"));
let input = session.input_text(Puzzle::at(2020, 1)?).await?;

assert_eq!(input, "1721\n979\n366");
assert_eq!(
    session.transport().requested_urls(),
    ["https://adventofcode.com/2020/day/1/input"]
);
```

## Automation etiquette

This crate follows the Advent of Code
[automation guidelines](https://www.reddit.com/r/adventofcode/wiki/faqs/automation).
Two of them are settled here; two are deliberately left to you, and this
section says which is which so you can be accurate about your own tool.

- **Every request identifies you.** You provide your identification when opening
  a `Session`, which is built into the HTTP client's default headers so no
  call site can omit it. The [automation guidelines](https://www.reddit.com/r/adventofcode/wiki/faqs/automation)
  ask for `github.com/your-repo by you@example.com` or similar. Since this is
  a library, the tool built upon it is the one doing the work, so it is the
  one that must identify itself.
- **Nothing happens that you did not ask for.** A request is made when you call
  a method and at no other time: nothing polls, retries, prefetches or runs on
  a schedule. The one call that can make two requests is `submit`, and only
  when the site says the part is already solved, in which case it reads the
  puzzle page to compare your answer against the accepted one.
- **Throttling is yours.** This crate does not sleep between requests. A
  library cannot know how a program is being driven, and a hidden delay inside
  someone else's process is a poor surprise — a caller that already paces
  itself would end up paying twice. Space your calls out; five seconds between
  them is a sensible floor.
  [`aoc-runtime`](https://github.com/antoniosubasic/aoc-runtime) is a worked
  example: it holds a minimum gap persisted to its state directory, so the gap
  survives across separate invocations rather than only within one run.
- **Caching is yours.** Puzzle inputs are personal, permanent and unchanging,
  so download one once and keep it on disk. This crate hands you the body and
  forgets it; it never re-downloads on its own, and it never downloads anything
  you did not ask for.

Please do not work around the first two.

## Design notes

Decisions made during the rewrite, and why:

- **Async, with no runtime of its own.** The API is asynchronous because
  `reqwest` is, but `tokio` is a dev-dependency only: the crate spawns nothing
  and needs no runtime features, so it runs on whichever executor you already
  have — including a current-thread runtime driven to completion, which is how
  a synchronous program should use it.
- **One session, no free functions.** The free functions rebuilt a client per
  call and duplicated `Session` exactly. `Session` is now the only API: it is
  where the cookie and the identified client live, and where they are reused.
- **A cooldown is a `Duration`.** The remaining wait used to be handed back as
  the site's own prose. It is now parsed into a `Duration`, in both shapes the
  site uses (`4m 30s` and `one minute`), so a caller can actually wait for it.
- **`rustls`, not the platform TLS stack.** `reqwest` is trimmed to the
  features this crate uses, which takes `openssl` out of the dependency tree
  and the advisories that follow it along with it.
- **No `regex`.** The site's replies are read by a small, dependency-free
  parser in [`src/parse.rs`](src/parse.rs), pinned by saved response bodies in
  `tests/fixtures` — correct, wrong, too high, too low, cooldown, already
  complete and logged out. It also drops emphasis markup and decodes entities,
  so a sample comes back as the puzzle shows it rather than as page source.

## Migrating from 3.x

Version 4 is a rewrite. Every removed item and its replacement:

| 3.x | 4.x |
| --- | --- |
| `Session::new(cookie, year, day)` | `Session::new(&cookie, "identification")?` plus `Puzzle::at(year, day)?` |
| `Session::from_pattern(cookie, input, pattern)` | recover the year and day yourself, then `Puzzle::at(year, day)?` |
| `get_input_text(&cookie, year, day).await?` | `session.input_text(puzzle).await?` |
| `get_input_lines(&cookie, year, day).await?` | `session.input_lines(puzzle).await?` |
| `get_sample_input_text(&cookie, year, day, nth).await?` | `session.sample_text(puzzle, nth).await?` |
| `get_sample_input_lines(&cookie, year, day, nth).await?` | `session.sample_lines(puzzle, nth).await?` |
| `get_all_stars(&cookie).await?` → `HashMap<u16, u8>` | `session.stars().await?` → `BTreeMap<Year, u8>` |
| `submit_answer(...)` *(deprecated)* → `Response { success, cooldown }` | `session.submit(puzzle, part, answer).await?` → `Verdict` |
| `submit_answer_explicit_error(...)` → `Result<bool, SubmitAnswerError>` | `session.submit(puzzle, part, answer).await?`; `verdict.is_correct()` is the old `bool` |
| `Response` | removed — `Verdict` says what happened |
| `SubmitAnswerError::Cooldown(String)` | `Error::Cooldown { wait: Duration }` |
| `SubmitAnswerError::Unknown(String)` | `Error::Parse(ParseError::Submission { snippet })` |
| `SubmitAnswerError::Other(String)` | `Error::Transport(_)`, `Error::Unauthorized`, `Error::Locked { .. }` or `Error::Status { .. }`, depending on what actually went wrong |
| `Box<dyn Error>` | `aoc_api::Error` |

A whole call site, before and after:

```rust
// 3.x
let input = aoc_api::get_input_text(&cookie, 2024, 7).await?;
match aoc_api::submit_answer_explicit_error(&cookie, 2024, 7, 1, "3749").await {
    Ok(true) => println!("correct"),
    Ok(false) => println!("wrong"),
    Err(SubmitAnswerError::Cooldown(wait)) => println!("wait {wait}"),
    Err(error) => return Err(error.into()),
}

// 4.x
let session = Session::new(&cookie, "github.com/my-username/my-repo by me@example.com")?;
let puzzle = Puzzle::at(2024, 7)?;

let input = session.input_text(puzzle).await?;
match session.submit(puzzle, Part::One, "3749").await {
    Ok(verdict) if verdict.is_correct() => println!("correct"),
    Ok(verdict) => println!("{verdict}"),
    Err(Error::Cooldown { wait }) => println!("wait {wait:?}"),
    Err(error) => return Err(error),
}
```

Note that a rejected answer the site asked you to wait after used to arrive as
`Err(Cooldown)`, losing the fact that it had been judged at all. It is now
`Ok(Verdict::Incorrect { wait: Some(_), .. })`, and `Error::Cooldown` means
only what the site means by it: nothing was judged, so the answer still has to
be submitted again.

## Development

```console
$ cargo test                      # unit, integration and doc tests; no network
$ cargo clippy --all-targets
$ cargo fmt --all --check
$ cargo doc --no-deps             # CI runs this with RUSTDOCFLAGS=-D warnings
$ cargo deny check                # licence and advisory audit, see deny.toml
```

CI sets `RUSTFLAGS: -D warnings`, so any warning fails the build there, and it
also builds against the MSRV declared as `rust-version` in `Cargo.toml` (1.88).
No test contacts `adventofcode.com`: endpoints and parsing run through
`FakeTransport`, and the handful of tests that exercise the real client talk to
a loopback listener the test itself stands up.

Releases are handled by [release-plz](https://release-plz.dev): merging the
generated release pull request publishes to crates.io and tags `vX.Y.Z`.

## License

[GPL-3.0](LICENSE)
