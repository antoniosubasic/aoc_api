[![Version](https://img.shields.io/crates/v/aoc_api)](https://crates.io/crates/aoc_api)
[![Downloads](https://img.shields.io/crates/dv/aoc_api?label=Downloads)](https://crates.io/crates/aoc_api)

a simple [Advent of Code](https://adventofcode.com) API

## Documentation

- [Add Crate](#add-crate)
- [Session initialization](#session-initialization)
- [Features](#features)
    - [Get input](#get-input)
    - [Get sample input](#get-sample-input)
    - [Get achieved stars](#get-achieved-stars)
    - [Submit answer](#submit-answer)

<br><br>

# Add Crate

```bash
cargo add aoc_api
```

```rust
use aoc_api::Session;
```

<br>

# Session initialization

```rust
let client = Session::new("session cookie": String, year: u16, day: u8); // Initializes a new Session instance
```

```rust
let client = Session::new("session cookie": String, input: String, pattern: Regex); // Initializes a new Session instance
```

> <picture>
>   <source media="(prefers-color-scheme: dark)" srcset="https://github.com/Mqxx/GitHub-Markdown/blob/main/blockquotes/badge/dark-theme/info.svg">
>   <img alt="Info" src="https://github.com/Mqxx/GitHub-Markdown/blob/main/blockquotes/badge/dark-theme/Info">
> </picture><br>
> The Regex overload needs to have a regex group named "year" and a group named "day".
> <br> <a href="https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Named_capturing_group">How to name Regex groups</a>
> <br> <a href="https://mmhaskell.com/blog/2023/1/30/advent-of-code-fetching-puzzle-input-using-the-api#authentication">How to obtain session cookie</a>

<br>

# Features

## Get input

```rust
let input_text: Result<String, Box<dyn Error>> = client.get_input_text().await; // Retrieves the input text of the AoC puzzle
let input_lines: Result<Vec<String>, Box<dyn Error>> = client.get_input_lines().await; // Retrieves the input lines of the AoC puzzle
```

<br>

## Get sample input

```rust
let sample_input_text: Result<String, Box<dyn Error>> = client.get_sample_input_text(nth: u8).await; // Retrieves the nth sample input text of the AoC puzzle
let sample_input_lines: Result<Vec<String>, Box<dyn Error>> = client.get_sample_input_lines(nth: u8).await; // Retrieves the nth sample input lines of the AoC puzzle
```

<br>

## Get achieved stars

```rust
let achieved_stars: Result<HashMap<u16, u8>, Box<dyn Error>> = client.get_all_stars().await; // Retrieves each year's number of stars earned (key: year, value: stars)
```

<br>

## Submit answer

```rust
let response: Result<Response, Box<dyn Error>> = client.submit_answer(part: u8, answer: &str).await; // Submits an answer to part 1 or 2 of the AoC puzzle. Returns a response type with a success status and a cooldown period
```

<br><br>

*credits to:*
> [Max](https://github.com/Mqxx) - markdown info icons <br>
> [Monday Morning Haskell](https://mmhaskell.com/) - documentation on how to obtaining session cookie <br>
> [Developer.Mozilla](https://developer.mozilla.org) - documentation on how to name Regex groups