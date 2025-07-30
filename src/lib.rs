use core::fmt;
use regex::Regex;
use reqwest::header::{HeaderMap, COOKIE};
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::error::Error;

#[cfg(test)]
const SESSION_COOKIE: &str = ""; // add cookie here

async fn send_request(
    cookie: &str,
    method: Method,
    uri: &str,
    content: Option<String>,
) -> Result<String, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, format!("session={}", cookie).parse()?);

    let request = Client::new().request(method, uri).headers(headers);

    let request = match content {
        Some(content) => request.body(content),
        None => request,
    };

    let response = request.send().await?;

    match response.status().is_success() {
        true => Ok(response.text().await?),
        false => Err(format!("request failed: {}", response.status()).into()),
    }
}

pub async fn get_sample_input_text(
    cookie: &str,
    year: u16,
    day: u8,
    nth: u8,
) -> Result<String, Box<dyn Error>> {
    let uri = format!("https://adventofcode.com/{}/day/{}", year, day);
    let response = send_request(cookie, Method::GET, &uri, None).await?;

    let re = Regex::new(r"<pre><code>(?<sample>(.*?\n)*?)<\/code><\/pre>").unwrap();
    let matches = re.captures_iter(&response).collect::<Vec<_>>();

    if matches.len() >= nth as usize {
        Ok(matches[(nth - 1) as usize]
            .name("sample")
            .ok_or("no sample match")?
            .as_str()
            .trim_end_matches("\n")
            .to_string())
    } else {
        Err("sample could not be retrieved".into())
    }
}

pub async fn get_sample_input_lines(
    cookie: &str,
    year: u16,
    day: u8,
    nth: u8,
) -> Result<Vec<String>, Box<dyn Error>> {
    let text = get_sample_input_text(cookie, year, day, nth).await?;
    Ok(text.lines().map(|line| line.to_string()).collect())
}

pub async fn get_input_text(cookie: &str, year: u16, day: u8) -> Result<String, Box<dyn Error>> {
    let uri = format!("https://adventofcode.com/{}/day/{}/input", year, day);
    let response = send_request(cookie, Method::GET, &uri, None).await?;
    Ok(response.trim_end_matches('\n').to_string())
}

pub async fn get_input_lines(
    cookie: &str,
    year: u16,
    day: u8,
) -> Result<Vec<String>, Box<dyn Error>> {
    let text = get_input_text(cookie, year, day).await?;
    Ok(text.lines().map(|line| line.to_string()).collect())
}

pub async fn get_all_stars(cookie: &str) -> Result<HashMap<u16, u8>, Box<dyn Error>> {
    let response: Vec<String> =
        send_request(cookie, Method::GET, "https://adventofcode.com/events", None)
            .await?
            .lines()
            .filter(|line| line.starts_with("<div class=\"eventlist-event\">"))
            .map(|line| line.to_string())
            .collect();

    let stars_map = response
        .iter()
        .map(|line| {
            let year_index = line.find("</a>").unwrap() as i16 - 5;
            let star_index = line.find("</span>").unwrap_or(0) as i16 - 3;

            let year = line[(year_index as usize)..(year_index as usize + 4)]
                .parse::<u16>()
                .unwrap();

            let stars = if star_index < 0 {
                0
            } else {
                line[(star_index as usize)..(star_index as usize + 2)]
                    .trim()
                    .parse::<u8>()
                    .unwrap()
            };

            (year, stars)
        })
        .collect();

    Ok(stars_map)
}

#[deprecated(
    note = "This function is deprecated and will be removed in the future. Please use submit_answer_explicit_error instead."
)]
pub async fn submit_answer(
    cookie: &str,
    year: u16,
    day: u8,
    part: u8,
    answer: &str,
) -> Result<Response, Box<dyn Error>> {
    let uri = format!("https://adventofcode.com/{}/day/{}/answer", year, day);
    let content = format!("level={}&answer={}", part, answer);
    let response = send_request(cookie, Method::POST, &uri, Some(content)).await?;

    if response.contains("That's the right answer!") {
        Ok(Response {
            success: Some(true),
            cooldown: None,
        })
    } else if response.contains("Did you already complete it?")
        || response.contains("Both parts of this puzzle are complete!")
    {
        let day_response_uri = format!("https://adventofcode.com/{}/day/{}", year, day);
        let day_response = send_request(cookie, Method::GET, &day_response_uri, None).await?;

        let re = Regex::new(r"<p>Your puzzle answer was <code>(?<answer>.*?)</code>.</p>").unwrap();
        let matches = re.captures_iter(&day_response).collect::<Vec<_>>();

        if matches.len() >= part as usize {
            let correct_answer = matches[(part - 1) as usize]
                .name("answer")
                .ok_or("answer could not be retrieved")?
                .as_str();

            Ok(Response {
                success: Some(correct_answer == answer),
                cooldown: None,
            })
        } else {
            Err("answer could not be retrieved".into())
        }
    } else if response.contains("You gave an answer too recently") {
        let re = Regex::new(r"You have (?<time>.*?) left to wait").unwrap();
        let capture = re
            .captures(&response)
            .ok_or("cooldown time could not be retrieved")?;

        let time = capture
            .name("time")
            .ok_or("cooldown time could not be retrieved")?
            .as_str();

        Ok(Response {
            success: None,
            cooldown: Some(time.to_string()),
        })
    } else if response.contains("That's not the right answer.")
        || response.contains("before trying again.")
    {
        let re = Regex::new(r"wait (?<time>.*?) before trying again").unwrap();
        let capture = re.captures(&response);

        if let Some(capture) = capture {
            let time = capture
                .name("time")
                .ok_or("cooldown time could not be retrieved")?
                .as_str();

            Ok(Response {
                success: Some(false),
                cooldown: Some(time.to_string()),
            })
        } else {
            Ok(Response {
                success: Some(false),
                cooldown: None,
            })
        }
    } else {
        Err("unknown response".into())
    }
}

pub async fn submit_answer_explicit_error(
    cookie: &str,
    year: u16,
    day: u8,
    part: u8,
    answer: &str,
) -> Result<bool, SubmitAnswerError> {
    let uri = format!("https://adventofcode.com/{}/day/{}/answer", year, day);
    let content = format!("level={}&answer={}", part, answer);
    let response = send_request(cookie, Method::POST, &uri, Some(content))
        .await
        .map_err(|e| SubmitAnswerError::Other(e.to_string()))?;

    if response.contains("That's the right answer!") {
        Ok(true)
    } else if response.contains("Did you already complete it?")
        || response.contains("Both parts of this puzzle are complete!")
    {
        let day_response_uri = format!("https://adventofcode.com/{}/day/{}", year, day);
        let day_response = send_request(cookie, Method::GET, &day_response_uri, None)
            .await
            .map_err(|e| SubmitAnswerError::Other(e.to_string()))?;

        let re = Regex::new(r"<p>Your puzzle answer was <code>(?<answer>.*?)</code>.</p>").unwrap();
        let matches = re.captures_iter(&day_response).collect::<Vec<_>>();

        if matches.len() >= part as usize {
            let correct_answer = matches[(part - 1) as usize]
                .name("answer")
                .ok_or_else(|| {
                    SubmitAnswerError::Other("answer could not be retrieved".to_string())
                })?
                .as_str();
            Ok(correct_answer == answer)
        } else {
            Err(SubmitAnswerError::Other(
                "answer could not be retrieved".to_string(),
            ))
        }
    } else if response.contains("You gave an answer too recently") {
        let re = Regex::new(r"You have (?<time>.*?) left to wait").unwrap();
        let capture = re.captures(&response).ok_or_else(|| {
            SubmitAnswerError::Other("cooldown time could not be retrieved".to_string())
        })?;

        let time = capture
            .name("time")
            .ok_or_else(|| {
                SubmitAnswerError::Other("cooldown time could not be retrieved".to_string())
            })?
            .as_str();

        Err(SubmitAnswerError::Cooldown(time.to_string()))
    } else if response.contains("That's not the right answer.")
        || response.contains("before trying again.")
    {
        let re = Regex::new(r"wait (?<time>.*?) before trying again").unwrap();
        let capture = re.captures(&response);

        if let Some(capture) = capture {
            let time = capture
                .name("time")
                .ok_or_else(|| {
                    SubmitAnswerError::Other("cooldown time could not be retrieved".to_string())
                })?
                .as_str();
            Err(SubmitAnswerError::Cooldown(time.to_string()))
        } else {
            Ok(false)
        }
    } else {
        Err(SubmitAnswerError::Unknown("unknown response".to_string()))
    }
}

pub struct Session {
    cookie: String,
    year: u16,
    day: u8,
}

impl Session {
    pub fn new(cookie: String, year: u16, day: u8) -> Self {
        Self { cookie, year, day }
    }

    pub fn from_pattern(cookie: String, input: String, pattern: Regex) -> Result<Self, String> {
        let captures = pattern.captures(&input).ok_or("no regex match")?;

        let year = captures
            .name("year")
            .ok_or("no year match")?
            .as_str()
            .parse::<u16>()
            .map_err(|e| e.to_string())?;
        let day = captures
            .name("day")
            .ok_or("no day match")?
            .as_str()
            .parse::<u8>()
            .map_err(|e| e.to_string())?;

        Ok(Self::new(cookie, year, day))
    }

    pub async fn get_sample_input_text(&self, nth: u8) -> Result<String, Box<dyn Error>> {
        get_sample_input_text(&self.cookie, self.year, self.day, nth).await
    }

    pub async fn get_sample_input_lines(&self, nth: u8) -> Result<Vec<String>, Box<dyn Error>> {
        get_sample_input_lines(&self.cookie, self.year, self.day, nth).await
    }

    pub async fn get_input_text(&self) -> Result<String, Box<dyn Error>> {
        get_input_text(&self.cookie, self.year, self.day).await
    }

    pub async fn get_input_lines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        get_input_lines(&self.cookie, self.year, self.day).await
    }

    pub async fn get_all_stars(&self) -> Result<HashMap<u16, u8>, Box<dyn Error>> {
        get_all_stars(&self.cookie).await
    }

    #[deprecated(
        note = "This function is deprecated and will be removed in the future. Please use submit_answer_explicit_error instead."
    )]
    pub async fn submit_answer(&self, part: u8, answer: &str) -> Result<Response, Box<dyn Error>> {
        submit_answer(&self.cookie, self.year, self.day, part, answer).await
    }

    pub async fn submit_answer_explicit_error(
        &self,
        part: u8,
        answer: &str,
    ) -> Result<bool, SubmitAnswerError> {
        submit_answer_explicit_error(&self.cookie, self.year, self.day, part, answer).await
    }
}

#[deprecated(note = "This struct is deprecated and will be removed in the future.")]
pub struct Response {
    pub success: Option<bool>,
    pub cooldown: Option<String>,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let success_str = self.success.map_or(String::new(), |s| s.to_string());
        let cooldown_str = self
            .cooldown
            .as_ref()
            .map_or(String::new(), |c| format!("on cooldown: {c}"));
        let seperator = if self.success.is_some() && self.cooldown.is_some() {
            "\n"
        } else {
            ""
        };
        write!(f, "{success_str}{seperator}{cooldown_str}")
    }
}

#[derive(Debug)]
pub enum SubmitAnswerError {
    Cooldown(String),
    Unknown(String),
    Other(String),
}

impl fmt::Display for SubmitAnswerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubmitAnswerError::Cooldown(msg) => write!(f, "on cooldown: {}", msg),
            SubmitAnswerError::Unknown(msg) => write!(f, "unknown error: {}", msg),
            SubmitAnswerError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for SubmitAnswerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let cookie = String::from("cookie");
        let year = 2020;
        let day = 1;

        let session = Session::new(cookie.clone(), year, day);

        assert_eq!(session.cookie, cookie);
        assert_eq!(session.year, year);
        assert_eq!(session.day, day);
    }

    #[test]
    fn test_session_from_pattern() {
        let cookie = String::from("cookie");
        let input = String::from("2020/day01");
        let pattern = Regex::new(r"(?<year>\d{4})/day(?<day>\d+)").unwrap();

        let session = Session::from_pattern(cookie.clone(), input, pattern).unwrap();

        assert_eq!(session.cookie, cookie);
        assert_eq!(session.year, 2020);
        assert_eq!(session.day, 1);
    }

    #[test]
    fn test_session_from_pattern_no_match() {
        let cookie = String::from("cookie");
        let input = String::from("something");
        let pattern = Regex::new(r"(?<year>\d{4})/day(?<day>\d+)").unwrap();

        let session = Session::from_pattern(cookie, input, pattern);
        assert!(session.is_err());
    }

    #[tokio::test]
    async fn test_session_get_sample_input() {
        let cookie = String::from(SESSION_COOKIE);
        let year = 2020;
        let day = 1;

        let session = Session::new(cookie, year, day);

        let sample_input_text = session.get_sample_input_text(1).await.unwrap();
        assert_eq!(sample_input_text, "1721\n979\n366\n299\n675\n1456");

        let sample_input_lines = session.get_sample_input_lines(1).await.unwrap();
        assert_eq!(
            sample_input_lines,
            vec!["1721", "979", "366", "299", "675", "1456"]
        );
    }

    #[tokio::test]
    async fn test_session_get_input() {
        let cookie = String::from(SESSION_COOKIE);
        let year = 2020;
        let day = 1;

        let session = Session::new(cookie, year, day);

        let input_text = session.get_input_text().await;
        assert!(input_text.is_ok());

        let input_lines = session.get_input_lines().await;
        assert!(input_lines.is_ok());
    }

    #[tokio::test]
    async fn test_session_get_all_stars() {
        let cookie = String::from(SESSION_COOKIE);
        let year = 2020;
        let day = 1;

        let session = Session::new(cookie, year, day);

        let stars = session.get_all_stars().await;
        assert!(stars.is_ok());
    }

    #[tokio::test]
    async fn test_session_submit_answer() {
        let cookie = String::from(SESSION_COOKIE);
        let year = 2020;
        let day = 1;

        let session = Session::new(cookie, year, day);

        let response = session.submit_answer(1, "test").await.unwrap();
        assert_eq!(response.success, Some(false));

        let response = session.submit_answer(2, "261342720").await.unwrap();
        assert_eq!(response.success, Some(true));
    }

    #[tokio::test]
    async fn test_session_submit_answer_explicit_error() {
        let cookie = String::from(SESSION_COOKIE);
        let year = 2020;
        let day = 1;

        let session = Session::new(cookie, year, day);

        let response = session
            .submit_answer_explicit_error(1, "test")
            .await
            .unwrap();
        assert_eq!(response, false);

        let response = session
            .submit_answer_explicit_error(2, "261342720")
            .await
            .unwrap();
        assert_eq!(response, true);
    }
}
