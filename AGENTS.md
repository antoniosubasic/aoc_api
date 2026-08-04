# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.

## What this is

`aoc_api` is a library crate: a typed client for adventofcode.com that
downloads inputs and samples, reads stars and submits answers. There is no
binary. Rust edition 2024, MSRV 1.88 (`rust-version` in `Cargo.toml`, built by
a dedicated CI job).

## Commands

```console
$ cargo test                         # unit, integration and doc tests; no network
$ cargo test --lib                   # the per-module `mod tests` only
$ cargo test --test api              # the public-API integration suite
$ cargo test --doc                   # doc examples (the `FakeTransport` ones really run)
$ cargo test a_cooldown              # one test, by substring of its name
$ cargo clippy --all-targets
$ cargo fmt --all --check
$ RUSTDOCFLAGS=-D warnings cargo doc --no-deps
$ cargo deny check                   # licence and advisory audit
```

CI sets `RUSTFLAGS: -D warnings`, so anything that warns locally fails there.

## Architecture

Five modules, layered so the fragile part has no I/O and the I/O part has no
parsing:

- **`puzzle`** — `Year`, `Day`, `Part`, `Puzzle`: newtypes with private fields
  and `const` fallible constructors. They also own URL construction
  (`url()`, `input_url()`, `answer_url()`) and `BASE_URL`. The *pairing* is
  validated, not just each half: events through 2024 run 25 days, 2025 onwards
  run 12 (`Year::FIRST_SHORT` / `Day::LAST_SHORT`). An invalid coordinate can
  therefore never reach the transport.
- **`http`** — the `Transport` trait, which is the seam everything external
  sits behind, plus `Request`/`Response` (plain data, no `reqwest` types) and
  `ClientOptions`. `ReqwestTransport` is the **only** place a client is built
  and the only place in the crate that knows `reqwest` exists; `fn headers` is
  the only place the `User-Agent` identification and the session cookie are
  set. There are deliberately no per-call-site headers — that would be a way
  for a request to go out unidentified.
- **`http::fake`** — `FakeTransport`, a queue of canned `Response`s that also
  records every `Request`. Public on purpose: downstream tools test against it
  too. It is how this crate's own suite runs with no network and no cookie.
- **`parse`** — every reply the site sends is a browser page, and reading it is
  the most fragile thing here, so it lives in one module with no I/O at all:
  free functions over `&str`, dependency-free (`between`/`all_between`/
  `strip_tags`/`decode_entities`), **no `regex`**. `Submission` is what the
  reply literally said; turning it into a `Verdict` is `session`'s job because
  two cases need a second request.
- **`session`** — the endpoints. Each one is a free function over a
  `&impl Transport` (`input_text`, `samples`, `stars`, `submit`, …), and
  `Session<T = ReqwestTransport>` is a holder for the transport whose methods
  delegate to them one line at a time. Both surfaces are public and neither
  may grow behaviour the other lacks — put logic in the function, never in the
  method. Which puzzle a call is about is an argument, so a session serves a
  whole event. `fn check` decides what a reply means: it asks
  `parse::is_logged_out` **before** looking at the status, because a rejected
  cookie arrives as a `400` from the input endpoint but as an ordinary `200`
  page with a log-in link from the puzzle and events pages.
- **`error`** — `Error` is the union of the per-module typed errors
  (`TransportError`, `ParseError`, `PuzzleError`) plus the cases that only
  exist once a reply is read in context (`Unauthorized`, `Locked`, `Cooldown`,
  `Status`). `#[error(transparent)]` and `#[source]` keep the chain walkable.

Two invariants that the code is shaped around, both from the Advent of Code
[automation guidelines](https://www.reddit.com/r/adventofcode/wiki/faqs/automation):
**every request carries the caller's identification** (enforced by there being
one constructor), and **no request happens that the caller did not ask for** —
nothing polls, retries, prefetches or sleeps. The single exception is `submit`,
which reads the puzzle page when the site says the part is already solved.
Throttling and caching are explicitly the caller's job; do not add either here.
A rejected answer is a `Verdict`, not an `Error` — being wrong is a normal
outcome.

## Working in this codebase

- **Nothing may contact `adventofcode.com`, in tests or in doc examples.** Test
  against `FakeTransport`. The only tests that open a socket are in `http`'s
  own `mod tests`, against a loopback `TcpListener` the test stands up itself.
  Doc examples that would need a real session are `no_run`; the ones on
  `FakeTransport` execute for real.
- **Parser changes are pinned by fixtures.** Every reply shape has a saved body
  in `tests/fixtures` (correct, wrong, too high, too low, cooldown, already
  complete, logged out, a puzzle page, an events page). If the site starts
  saying something new, save the body as a fixture and add the case — do not
  loosen a matcher until it guesses. An unrecognised reply is
  `ParseError::Submission`, deliberately.
- **`unwrap`, `expect` and `panic!` are `deny` in library code** and allowed
  only in tests (`clippy.toml`). `unsafe_code` is `forbid`. `missing_docs` is
  `warn`, which `-D warnings` makes fatal in CI: every public item, field and
  variant needs a doc comment, and every fallible function needs an `# Errors`
  section.
- **Keep `openssl` and `native-tls` out.** `deny.toml` bans them by name and
  `reqwest` is pinned to `rustls` with a minimal feature set. Adding a
  dependency that drags either back in fails `cargo deny`.
- **The crate brings no async runtime.** `tokio` is a dev-dependency only.
  Never spawn, never sleep, never require a runtime feature.
- **Public enums are `#[non_exhaustive]`.** Adding a variant is not a breaking
  change; matching on one downstream forces a wildcard arm.
- Test names are full sentences describing the behaviour
  (`a_rejected_answer_is_a_verdict_rather_than_an_error`). Comments explain
  *why*, not what. Error messages are lowercase and phrased as the site phrases
  things.

## Releases

Handled by [release-plz](https://release-plz.dev) from conventional commits —
**never edit `version` in `Cargo.toml` or write a changelog by hand.** (The
manifest lagging behind what the README documents is normal: the release PR
raises it.) Prefixes that matter, per `release-plz.toml`: `feat`, `fix`,
`perf`, `refactor`, `docs`, `chore`, `bump`, `build(deps)`; `!` marks a
breaking change and is protected in the changelog. `test`, `style` and `ci` are
skipped. Merging the generated `chore: release vX.Y.Z` pull request publishes
to crates.io and tags `vX.Y.Z`; nothing is committed to `main` directly.
