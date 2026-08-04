//! Reading Advent of Code's HTML replies.
//!
//! The site has no API: everything it says arrives as a page meant for a
//! browser, and recognising it is the most fragile thing this crate does. It
//! therefore lives here, with no I/O of its own, so every case can be pinned
//! by a saved response body in `tests/fixtures`.
//!
//! Nothing in this module fails on unexpected *extra* markup; it fails when a
//! reply says something it does not recognise at all, which is the signal that
//! the site changed and this module needs updating.

use crate::puzzle::{Part, Year};
use std::{collections::BTreeMap, fmt, time::Duration};

/// A reply this crate could not make sense of.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The puzzle page does not have that many sample blocks.
    #[error("the puzzle page has {found} sample block(s), so there is no sample {requested}")]
    Sample {
        /// Which sample was asked for, counting from one.
        requested: u8,
        /// How many the page actually has.
        found: usize,
    },

    /// The puzzle page does not show an accepted answer for that part.
    #[error("the puzzle page does not show an accepted answer for {part}")]
    AcceptedAnswer {
        /// The part that was asked about.
        part: Part,
    },

    /// The events page listed no events.
    #[error("the events page listed no events")]
    Events,

    /// Advent of Code replied to a submission with something unrecognised.
    #[error("advent of code replied to the submission with something unrecognised: {snippet}")]
    Submission {
        /// The opening of the reply, with its markup stripped.
        snippet: String,
    },
}

/// Which way a rejected answer was wrong, when the site says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hint {
    /// The answer is larger than the right one.
    TooHigh,
    /// The answer is smaller than the right one.
    TooLow,
}

impl fmt::Display for Hint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooHigh => f.write_str("too high"),
            Self::TooLow => f.write_str("too low"),
        }
    }
}

/// What the site said about a submitted answer.
///
/// This is the reply as written, before it means anything: turning it into a
/// [`Verdict`](crate::Verdict) is [`Session::submit`](crate::Session::submit)'s
/// job, because two of these need a second request to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Submission {
    /// `That's the right answer!`
    Correct,

    /// `That's not the right answer.`
    Incorrect {
        /// Which way the answer was wrong, when the site says.
        hint: Option<Hint>,
        /// How long to wait before trying again, when the site says.
        wait: Option<Duration>,
    },

    /// `You gave an answer too recently.` Nothing was judged.
    TooRecent {
        /// How much of the wait is left.
        wait: Duration,
    },

    /// `You don't seem to be solving the right level. Did you already
    /// complete it?`
    AlreadyComplete,

    /// The reply is a page asking whoever sent it to log in.
    LoggedOut,
}

/// Every sample block on a puzzle page, in the order they appear.
///
/// Samples are the `<pre><code>` blocks of the puzzle text. Emphasis markup
/// inside them is dropped and HTML entities are decoded, so what comes back is
/// what the puzzle shows rather than what the page source says.
#[must_use]
pub fn samples(html: &str) -> Vec<String> {
    all_between(html, "<pre><code>", "</code></pre>")
        .into_iter()
        .map(|block| {
            decode_entities(&strip_tags(block))
                .trim_end_matches('\n')
                .to_owned()
        })
        .collect()
}

/// The `nth` sample block on a puzzle page, counting from one.
///
/// # Errors
///
/// Returns [`ParseError::Sample`] if the page has fewer blocks than that.
pub fn sample(html: &str, nth: u8) -> Result<String, ParseError> {
    let samples = samples(html);
    let found = samples.len();

    usize::from(nth)
        .checked_sub(1)
        .and_then(|index| samples.into_iter().nth(index))
        .ok_or(ParseError::Sample {
            requested: nth,
            found,
        })
}

/// Every answer a puzzle page shows as accepted, part one first.
#[must_use]
pub fn accepted_answers(html: &str) -> Vec<String> {
    all_between(html, "Your puzzle answer was <code>", "</code>")
        .into_iter()
        .map(|answer| decode_entities(&strip_tags(answer)))
        .collect()
}

/// The answer a puzzle page shows as accepted for `part`.
///
/// # Errors
///
/// Returns [`ParseError::AcceptedAnswer`] if the page shows no answer for that
/// part, which is what a page for an unsolved part looks like.
pub fn accepted_answer(html: &str, part: Part) -> Result<String, ParseError> {
    accepted_answers(html)
        .into_iter()
        .nth(part.index())
        .ok_or(ParseError::AcceptedAnswer { part })
}

/// How many stars each event on the events page has been awarded.
///
/// An event with no stars yet is present with a count of zero.
///
/// # Errors
///
/// Returns [`ParseError::Events`] if the page lists no events at all, which is
/// what the page looks like when the session cookie is not accepted.
pub fn stars(html: &str) -> Result<BTreeMap<Year, u8>, ParseError> {
    let mut stars = BTreeMap::new();

    for entry in html.split("eventlist-event").skip(1) {
        let entry = entry.split_once("</div>").map_or(entry, |(head, _)| head);

        let Some(year) = event_year(entry) else {
            continue;
        };

        let count = between(entry, "star-count\">", "</span>")
            .and_then(|count| count.trim().trim_end_matches('*').trim().parse().ok())
            .unwrap_or(0);

        stars.insert(year, count);
    }

    if stars.is_empty() {
        Err(ParseError::Events)
    } else {
        Ok(stars)
    }
}

/// Which event one entry of the events list is for.
fn event_year(entry: &str) -> Option<Year> {
    [between(entry, "href=\"/", "\""), between(entry, "[", "]")]
        .into_iter()
        .flatten()
        .find_map(year)
}

/// A year written out, if it is one.
fn year(text: &str) -> Option<Year> {
    Year::new(text.trim().parse().ok()?).ok()
}

/// What the site said in reply to a submitted answer.
///
/// # Errors
///
/// Returns [`ParseError::Submission`] if the reply is none of the ones this
/// crate knows, which means the site's wording changed.
pub fn submission(html: &str) -> Result<Submission, ParseError> {
    // Matching happens on a lowercased copy so a capitalisation change cannot
    // turn a known reply into an unrecognised one. Every needle below is
    // lowercase for that reason, and every extracted substring comes from the
    // same copy, so the offsets always line up.
    let reply = html.to_lowercase();

    if reply.contains("that's the right answer") {
        return Ok(Submission::Correct);
    }

    if reply.contains("you gave an answer too recently") {
        return Ok(Submission::TooRecent {
            wait: wait_before(&reply, " left to wait").unwrap_or_default(),
        });
    }

    if reply.contains("that's not the right answer") {
        let hint = if reply.contains("your answer is too high") {
            Some(Hint::TooHigh)
        } else if reply.contains("your answer is too low") {
            Some(Hint::TooLow)
        } else {
            None
        };

        return Ok(Submission::Incorrect {
            hint,
            wait: wait_before(&reply, " before trying again"),
        });
    }

    if reply.contains("did you already complete it")
        || reply.contains("both parts of this puzzle are complete")
    {
        return Ok(Submission::AlreadyComplete);
    }

    if reply.contains("/auth/login") || reply.contains("please log in") {
        return Ok(Submission::LoggedOut);
    }

    Err(ParseError::Submission {
        snippet: snippet(html),
    })
}

/// Whether a reply is the site asking whoever sent it to log in.
///
/// The site answers an unauthenticated request in several ways - a short
/// refusal on the input endpoint, a whole page with a log-in link elsewhere -
/// and this recognises all of them.
#[must_use]
pub fn is_logged_out(html: &str) -> bool {
    let reply = html.to_lowercase();

    reply.contains("please log in") || reply.contains("/auth/login")
}

/// A wait the site phrased in words, as a [`Duration`].
///
/// Handles both the compact form the cooldown uses (`4m 30s`) and the prose
/// form a rejected answer uses (`one minute`, `5 minutes`). Returns [`None`]
/// if there is no wait in the text at all.
#[must_use]
pub fn duration(text: &str) -> Option<Duration> {
    let mut total = Duration::ZERO;
    let mut pending: Option<u32> = None;
    let mut found = false;

    for word in text.split_whitespace() {
        let word = word
            .trim_matches(|character: char| !character.is_ascii_alphanumeric())
            .to_ascii_lowercase();

        if let Some(count) = count(&word) {
            pending = Some(count);
        } else if let Some(unit) = unit(&word) {
            if let Some(count) = pending.take() {
                total = total.saturating_add(unit.saturating_mul(count));
                found = true;
            }
        } else if let Some(wait) = compact(&word) {
            total = total.saturating_add(wait);
            found = true;
        }
    }

    found.then_some(total)
}

/// Renders a wait the way the site phrases it, for an error message.
#[must_use]
pub fn describe(wait: Duration) -> String {
    let seconds = wait.as_secs();
    let (hours, minutes, seconds) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}s"));
    }

    parts.join(" ")
}

/// The wait written immediately before `marker`.
///
/// Anchoring on the end of the phrase rather than its beginning keeps this
/// working whichever way the sentence leading up to it is worded - the site
/// has several, and they all end the same way.
fn wait_before(reply: &str, marker: &str) -> Option<Duration> {
    let head = reply.get(..reply.find(marker)?)?;
    let words: Vec<&str> = head.split_whitespace().collect();

    duration(&words[words.len().saturating_sub(6)..].join(" "))
}

/// A number, written as digits or as one of the words the site uses.
fn count(word: &str) -> Option<u32> {
    if !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit()) {
        // A wait longer than a century is not one; saturating keeps a silly
        // number from reading as no number at all.
        return Some(word.parse().unwrap_or(u32::MAX));
    }

    Some(match word {
        "one" | "a" | "an" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return None,
    })
}

/// A unit of time, written out.
fn unit(word: &str) -> Option<Duration> {
    Some(match word {
        "second" | "seconds" => Duration::from_secs(1),
        "minute" | "minutes" => Duration::from_secs(60),
        "hour" | "hours" => Duration::from_secs(3600),
        "day" | "days" => Duration::from_secs(86_400),
        _ => return None,
    })
}

/// A count and its unit written together, as in `4m` or `30s`.
fn compact(word: &str) -> Option<Duration> {
    let split = word.find(|character: char| !character.is_ascii_digit())?;
    let (digits, unit) = word.split_at(split);
    let count = count(digits)?;

    let unit = match unit {
        "s" => Duration::from_secs(1),
        "m" => Duration::from_secs(60),
        "h" => Duration::from_secs(3600),
        "d" => Duration::from_secs(86_400),
        _ => return None,
    };

    Some(unit.saturating_mul(count))
}

/// The opening of a reply, with markup stripped, for an error message.
fn snippet(html: &str) -> String {
    let text = decode_entities(&strip_tags(html));
    let mut snippet: String = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if snippet.chars().count() > 160 {
        snippet = snippet.chars().take(157).collect::<String>() + "...";
    }

    snippet
}

/// The text between the first `start` and the next `end` after it.
fn between<'a>(haystack: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let after = haystack.get(haystack.find(start)? + start.len()..)?;
    after.get(..after.find(end)?)
}

/// The text between every `start` and the next `end` after it.
fn all_between<'a>(haystack: &'a str, start: &str, end: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut rest = haystack;

    while let Some(index) = rest.find(start) {
        let after = &rest[index + start.len()..];
        let Some(stop) = after.find(end) else { break };

        found.push(&after[..stop]);
        rest = &after[stop + end.len()..];
    }

    found
}

/// Drops every tag, keeping the text between them.
fn strip_tags(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut depth = 0_usize;

    for character in html.chars() {
        match character {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => text.push(character),
            _ => {}
        }
    }

    text
}

/// Decodes the handful of entities the puzzle text uses.
fn decode_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(index) = rest.find('&') {
        decoded.push_str(&rest[..index]);
        let entity = &rest[index..];

        if let Some((character, end)) = entity
            .find(';')
            .filter(|end| *end <= 8)
            .and_then(|end| decode(&entity[..=end]).map(|character| (character, end)))
        {
            decoded.push_str(character);
            rest = &entity[end + 1..];
        } else {
            decoded.push('&');
            rest = &entity[1..];
        }
    }

    decoded.push_str(rest);
    decoded
}

/// One entity, if it is one this crate knows.
fn decode(entity: &str) -> Option<&'static str> {
    Some(match entity {
        "&amp;" => "&",
        "&lt;" => "<",
        "&gt;" => ">",
        "&quot;" => "\"",
        "&apos;" | "&#39;" => "'",
        "&nbsp;" => " ",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORRECT: &str = include_str!("../tests/fixtures/submit-correct.html");
    const TOO_HIGH: &str = include_str!("../tests/fixtures/submit-too-high.html");
    const TOO_LOW: &str = include_str!("../tests/fixtures/submit-too-low.html");
    const WRONG: &str = include_str!("../tests/fixtures/submit-wrong.html");
    const COOLDOWN: &str = include_str!("../tests/fixtures/submit-cooldown.html");
    const COMPLETED: &str = include_str!("../tests/fixtures/submit-already-complete.html");
    const LOGGED_OUT: &str = include_str!("../tests/fixtures/logged-out.html");
    const PUZZLE: &str = include_str!("../tests/fixtures/puzzle-day.html");
    const EVENTS: &str = include_str!("../tests/fixtures/events.html");

    #[test]
    fn an_accepted_answer_is_recognised() {
        assert_eq!(submission(CORRECT), Ok(Submission::Correct));
    }

    #[test]
    fn a_rejected_answer_carries_the_direction_it_was_wrong_in() {
        assert_eq!(
            submission(TOO_HIGH),
            Ok(Submission::Incorrect {
                hint: Some(Hint::TooHigh),
                wait: Some(Duration::from_secs(60)),
            })
        );
        assert_eq!(
            submission(TOO_LOW),
            Ok(Submission::Incorrect {
                hint: Some(Hint::TooLow),
                wait: Some(Duration::from_secs(300)),
            })
        );
    }

    #[test]
    fn a_rejected_answer_without_a_hint_is_still_a_rejection() {
        assert_eq!(
            submission(WRONG),
            Ok(Submission::Incorrect {
                hint: None,
                wait: Some(Duration::from_secs(60)),
            })
        );
    }

    #[test]
    fn a_submission_on_cooldown_reports_the_remaining_wait() {
        assert_eq!(
            submission(COOLDOWN),
            Ok(Submission::TooRecent {
                wait: Duration::from_secs(4 * 60 + 30),
            })
        );
    }

    #[test]
    fn a_part_that_is_already_solved_is_recognised() {
        assert_eq!(submission(COMPLETED), Ok(Submission::AlreadyComplete));
    }

    #[test]
    fn a_reply_that_asks_for_a_login_is_recognised() {
        assert_eq!(submission(LOGGED_OUT), Ok(Submission::LoggedOut));
        assert!(is_logged_out(LOGGED_OUT));
        assert!(!is_logged_out(PUZZLE));
    }

    #[test]
    fn an_unrecognised_reply_reports_what_the_site_said() {
        let error = submission("<main><article><p>Ho ho ho.</p></article></main>")
            .expect_err("nothing in there is a known reply");

        assert!(error.to_string().contains("Ho ho ho."), "{error}");
    }

    #[test]
    fn an_unrecognised_reply_does_not_repeat_a_whole_page_back() {
        let error = submission(&format!("<p>{}</p>", "lorem ipsum ".repeat(100)))
            .expect_err("nothing in there is a known reply");

        let ParseError::Submission { snippet } = error else {
            panic!("expected an unrecognised submission")
        };
        assert_eq!(snippet.chars().count(), 160);
    }

    #[test]
    fn samples_are_read_without_their_markup() {
        let samples = samples(PUZZLE);

        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0], "1721\n979\n366\n299\n675\n1456");
        assert_eq!(
            sample(PUZZLE, 1).as_deref(),
            Ok("1721\n979\n366\n299\n675\n1456")
        );
    }

    #[test]
    fn emphasis_and_entities_inside_a_sample_are_resolved() {
        assert_eq!(sample(PUZZLE, 2).as_deref(), Ok("3 & 4 <target>\n7 > 5"));
    }

    #[test]
    fn asking_for_a_sample_a_page_does_not_have_says_how_many_there_are() {
        assert_eq!(
            sample(PUZZLE, 3),
            Err(ParseError::Sample {
                requested: 3,
                found: 2
            })
        );
        assert_eq!(
            sample(PUZZLE, 0),
            Err(ParseError::Sample {
                requested: 0,
                found: 2
            })
        );
    }

    #[test]
    fn a_solved_page_shows_both_accepted_answers() {
        assert_eq!(accepted_answers(PUZZLE), ["514579", "241861950"]);
        assert_eq!(accepted_answer(PUZZLE, Part::One).as_deref(), Ok("514579"));
        assert_eq!(
            accepted_answer(PUZZLE, Part::Two).as_deref(),
            Ok("241861950")
        );
    }

    #[test]
    fn a_page_without_an_accepted_answer_says_which_part_is_missing() {
        assert_eq!(
            accepted_answer("<p>nothing solved here</p>", Part::One),
            Err(ParseError::AcceptedAnswer { part: Part::One })
        );
    }

    #[test]
    fn the_events_page_lists_every_event_and_its_stars() {
        let stars = stars(EVENTS).expect("the fixture lists events");

        assert_eq!(stars.get(&Year::new(2024).expect("valid")), Some(&50));
        assert_eq!(stars.get(&Year::new(2023).expect("valid")), Some(&9));
        assert_eq!(stars.get(&Year::new(2022).expect("valid")), Some(&0));
        assert_eq!(stars.len(), 4);
    }

    #[test]
    fn the_running_event_counts_even_though_it_links_to_the_front_page() {
        let stars = stars(EVENTS).expect("the fixture lists events");

        assert_eq!(stars.get(&Year::new(2025).expect("valid")), Some(&19));
        assert_eq!(
            stars.values().map(|count| u32::from(*count)).sum::<u32>(),
            78
        );
    }

    #[test]
    fn an_events_page_with_no_events_is_an_error_rather_than_an_empty_map() {
        assert_eq!(
            stars("<main><p>nothing here</p></main>"),
            Err(ParseError::Events)
        );
    }

    #[test]
    fn waits_are_read_in_both_of_the_shapes_the_site_uses() {
        assert_eq!(duration("4m 30s"), Some(Duration::from_secs(270)));
        assert_eq!(duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(duration("1h 2m 3s"), Some(Duration::from_secs(3723)));
        assert_eq!(duration("one minute"), Some(Duration::from_secs(60)));
        assert_eq!(duration("5 minutes"), Some(Duration::from_secs(300)));
        assert_eq!(duration("2 hours"), Some(Duration::from_secs(7200)));
        assert_eq!(duration("ten seconds"), Some(Duration::from_secs(10)));
        assert_eq!(duration("shortly"), None);
    }

    #[test]
    fn waits_are_rendered_the_way_the_site_phrases_them() {
        assert_eq!(describe(Duration::from_secs(270)), "4m 30s");
        assert_eq!(describe(Duration::from_secs(60)), "1m");
        assert_eq!(describe(Duration::from_secs(0)), "0s");
        assert_eq!(describe(Duration::from_secs(3723)), "1h 2m 3s");
    }
}
