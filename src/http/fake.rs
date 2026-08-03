//! A transport that replays canned replies.
//!
//! Every endpoint and every parser in this crate is exercised through this
//! type, so the suite needs neither a network nor a session cookie. It is
//! public because a tool built on `aoc_api` needs the same seam to test
//! itself.

use super::{Request, Response, Transport, TransportError};
use std::{
    collections::VecDeque,
    sync::{Mutex, PoisonError},
};

/// A [`Transport`] that answers from a queue instead of from the network.
///
/// ```
/// use aoc_api::{Puzzle, Session, http::fake::FakeTransport};
///
/// let session = Session::with_transport(FakeTransport::serving("1721\n979\n366\n"));
/// let puzzle = Puzzle::at(2020, 1)?;
///
/// let runtime = tokio::runtime::Builder::new_current_thread().build()?;
/// let input = runtime.block_on(session.input_text(puzzle))?;
///
/// assert_eq!(input, "1721\n979\n366");
/// assert_eq!(
///     session.transport().requested_urls(),
///     ["https://adventofcode.com/2020/day/1/input"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Default)]
pub struct FakeTransport {
    replies: Mutex<VecDeque<Response>>,
    seen: Mutex<Vec<Request>>,
}

impl FakeTransport {
    /// A transport with nothing queued, which fails every request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A transport whose first reply is `200 OK` with `body`.
    #[must_use]
    pub fn serving(body: impl Into<String>) -> Self {
        let transport = Self::new();
        transport.push(Response::ok(body));
        transport
    }

    /// Queues a reply behind whatever is already queued.
    pub fn push(&self, response: Response) -> &Self {
        self.replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(response);
        self
    }

    /// Queues a `200 OK` reply carrying `body`.
    pub fn push_body(&self, body: impl Into<String>) -> &Self {
        self.push(Response::ok(body))
    }

    /// Every request made so far, in order.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The URL of every request made so far, in order.
    #[must_use]
    pub fn requested_urls(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|request| request.url)
            .collect()
    }
}

impl Transport for FakeTransport {
    async fn execute(&self, request: Request) -> Result<Response, TransportError> {
        let reply = self
            .replies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front();

        let url = request.url.clone();
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request);

        reply.ok_or_else(|| TransportError::Request {
            source: "no reply was queued for this request".into(),
            url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn queued_replies_come_back_in_order() {
        let transport = FakeTransport::new();
        transport.push_body("first").push_body("second");

        let first = transport
            .execute(Request::get("https://example.test/1"))
            .await;
        let second = transport
            .execute(Request::get("https://example.test/2"))
            .await;

        assert_eq!(first.expect("first reply").body, "first");
        assert_eq!(second.expect("second reply").body, "second");
        assert_eq!(
            transport.requested_urls(),
            ["https://example.test/1", "https://example.test/2"]
        );
    }

    #[tokio::test]
    async fn a_request_with_nothing_queued_fails_loudly() {
        let transport = FakeTransport::new();

        let error = transport
            .execute(Request::get("https://example.test/"))
            .await
            .expect_err("nothing was queued");

        assert!(error.to_string().contains("https://example.test/"));
    }
}
