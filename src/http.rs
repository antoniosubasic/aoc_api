//! The HTTP transport.
//!
//! Everything that leaves this crate goes through [`Transport`]. The real
//! implementation, [`ReqwestTransport`], is the only place a client is
//! constructed and the only place in the crate that knows `reqwest` exists;
//! [`fake::FakeTransport`] replays canned replies so endpoints and parsing can
//! be exercised without a network or a session cookie.

pub mod fake;

use reqwest::{
    Client,
    header::{COOKIE, HeaderMap, HeaderValue},
};
use std::{error::Error, fmt, future::Future, time::Duration};

/// How long a request may take before it is abandoned.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Anything that stops a request from producing a reply.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// The session cookie cannot be sent in a header.
    #[error("the session cookie is empty or holds characters an http header cannot carry")]
    Cookie,

    /// The HTTP client could not be built.
    #[error("the http client could not be built")]
    Client {
        /// What went wrong underneath.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    /// The request could not be completed.
    #[error("the request to {url} failed")]
    Request {
        /// The URL that was being requested.
        url: String,
        /// What went wrong underneath.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}

/// The HTTP method of a [`Request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// Reads a page.
    Get,
    /// Submits a form.
    Post,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => f.write_str("GET"),
            Self::Post => f.write_str("POST"),
        }
    }
}

/// A request this crate makes to Advent of Code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The method.
    pub method: Method,
    /// The absolute URL.
    pub url: String,
    /// Fields sent as `application/x-www-form-urlencoded`; empty for a GET.
    pub form: Vec<(String, String)>,
}

impl Request {
    /// A request that reads `url`.
    #[must_use]
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            url: url.into(),
            form: Vec::new(),
        }
    }

    /// A request that posts a form to `url`.
    #[must_use]
    pub fn post_form(url: impl Into<String>, form: Vec<(String, String)>) -> Self {
        Self {
            method: Method::Post,
            url: url.into(),
            form,
        }
    }
}

/// A reply from Advent of Code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The HTTP status code.
    pub status: u16,
    /// The body, decoded as text.
    pub body: String,
}

impl Response {
    /// A reply with the given status and body.
    #[must_use]
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    /// A `200 OK` reply carrying `body`.
    #[must_use]
    pub fn ok(body: impl Into<String>) -> Self {
        Self::new(200, body)
    }

    /// Whether the status is in the `2xx` range.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

/// Sends requests to Advent of Code.
///
/// This is the seam the rest of the crate is built on: no endpoint and no
/// parser knows how a reply was obtained, so every one of them can be driven
/// from [`fake::FakeTransport`] in a test.
pub trait Transport {
    /// Sends `request` and returns the reply.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError`] if the request could not be completed. A
    /// reply with an unsuccessful status is a [`Response`], not an error;
    /// interpreting the status is the caller's job.
    fn execute(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send;
}

/// Everything the one HTTP client needs to know.
#[derive(Clone)]
pub struct ClientOptions {
    cookie: String,
    timeout: Duration,
}

impl ClientOptions {
    /// Options authenticating with the given session cookie.
    ///
    /// The cookie is the value of the `session` cookie on `adventofcode.com`.
    /// It is a credential: it is marked sensitive so it stays out of any
    /// header dump, and it is never printed by [`Debug`].
    #[must_use]
    pub fn new(cookie: impl Into<String>) -> Self {
        Self {
            cookie: cookie.into(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Overrides how long a request may take, from [`DEFAULT_TIMEOUT`].
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl fmt::Debug for ClientOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientOptions")
            .field("cookie", &"<redacted>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// The real transport, backed by `adventofcode.com`.
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    client: Client,
}

impl ReqwestTransport {
    /// Builds the one client this crate uses.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Cookie`] if the cookie cannot be sent in a
    /// header, or [`TransportError::Client`] if the client itself cannot be
    /// built.
    pub fn new(options: &ClientOptions) -> Result<Self, TransportError> {
        Self::from_builder(Client::builder(), options)
    }

    /// The single place a client is constructed.
    ///
    /// Taking the builder as an argument lets a test disable proxy discovery
    /// without changing anything about the headers under test.
    fn from_builder(
        builder: reqwest::ClientBuilder,
        options: &ClientOptions,
    ) -> Result<Self, TransportError> {
        let client = builder
            .default_headers(headers(options)?)
            .timeout(options.timeout)
            .build()
            .map_err(|source| TransportError::Client {
                source: Box::new(source),
            })?;

        Ok(Self { client })
    }

    /// Turns one of this crate's requests into a `reqwest` one.
    fn build(&self, request: &Request) -> Result<reqwest::Request, TransportError> {
        let builder = match request.method {
            Method::Get => self.client.get(&request.url),
            Method::Post => self.client.post(&request.url).form(&request.form),
        };

        builder.build().map_err(|source| TransportError::Request {
            url: request.url.clone(),
            source: Box::new(source),
        })
    }
}

impl Transport for ReqwestTransport {
    async fn execute(&self, request: Request) -> Result<Response, TransportError> {
        let failed = |source: reqwest::Error| TransportError::Request {
            url: request.url.clone(),
            source: Box::new(source),
        };

        let response = self
            .client
            .execute(self.build(&request)?)
            .await
            .map_err(failed)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(failed)?;

        Ok(Response { status, body })
    }
}

/// The headers every request carries.
fn headers(options: &ClientOptions) -> Result<HeaderMap, TransportError> {
    let mut headers = HeaderMap::new();

    let mut cookie = HeaderValue::from_str(&format!("session={}", options.cookie))
        .map_err(|_| TransportError::Cookie)?;
    cookie.set_sensitive(true);
    headers.insert(COOKIE, cookie);

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read as _, Write as _},
        net::TcpListener,
        thread,
    };

    /// Serves one canned reply on the loopback interface and hands back the
    /// bytes the client sent. Nothing here leaves the machine.
    fn serve_once() -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
        let url = format!(
            "http://{}/2024/day/1/input",
            listener.local_addr().expect("the port just bound")
        );

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("one connection");
            let mut buffer = [0_u8; 8192];
            let read = stream.read(&mut buffer).expect("the request head");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
                )
                .expect("the canned reply");

            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });

        (url, handle)
    }

    fn transport(options: &ClientOptions) -> ReqwestTransport {
        // `no_proxy` keeps an ambient `HTTP_PROXY` from redirecting the
        // loopback request; the headers are built exactly as in production.
        ReqwestTransport::from_builder(Client::builder().no_proxy(), options)
            .expect("a client with valid options")
    }

    #[tokio::test]
    async fn every_request_carries_the_session_cookie() {
        let (url, server) = serve_once();

        let response = transport(&ClientOptions::new("53616c7465645f5f"))
            .execute(Request::get(url))
            .await
            .expect("the loopback server replies");
        let sent = server.join().expect("the server thread");

        assert!(
            sent.contains("cookie: session=53616c7465645f5f"),
            "the cookie is missing from:\n{sent}"
        );
        assert_eq!(response.body, "hello");
    }

    #[tokio::test]
    async fn a_posted_answer_is_form_encoded() {
        let (url, server) = serve_once();

        transport(&ClientOptions::new("cookie"))
            .execute(Request::post_form(
                url,
                vec![
                    ("level".to_owned(), "1".to_owned()),
                    ("answer".to_owned(), "1 + 1".to_owned()),
                ],
            ))
            .await
            .expect("the loopback server replies");
        let sent = server.join().expect("the server thread");

        assert!(sent.starts_with("POST /2024/day/1/input"), "{sent}");
        assert!(
            sent.contains("content-type: application/x-www-form-urlencoded"),
            "{sent}"
        );
        assert!(sent.ends_with("level=1&answer=1+%2B+1"), "{sent}");
    }

    #[test]
    fn a_cookie_that_cannot_be_sent_is_rejected_before_any_request() {
        assert!(matches!(
            ReqwestTransport::new(&ClientOptions::new("bad\ncookie")),
            Err(TransportError::Cookie)
        ));
    }

    #[test]
    fn the_cookie_never_appears_in_debug_output() {
        let options = ClientOptions::new("53616c7465645f5f");
        let transport = ReqwestTransport::new(&options).expect("valid options");

        assert!(!format!("{options:?}").contains("53616c7465645f5f"));
        assert!(!format!("{transport:?}").contains("53616c7465645f5f"));
    }

    #[test]
    fn a_response_knows_whether_it_succeeded() {
        assert!(Response::ok("body").is_success());
        assert!(!Response::new(400, "body").is_success());
        assert!(!Response::new(404, "body").is_success());
    }
}
