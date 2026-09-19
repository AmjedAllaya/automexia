//! Explicit one-shot Google search; no terminal state, config, logging or HTTP client.
use crate::cli::GoogleCommand;
use std::io::{self, Write};

const MAX_QUERY_BYTES: usize = 4096;
const MAX_QUERY_ARGUMENTS: usize = 256;

pub(super) fn validated_query(arguments: &[String]) -> io::Result<String> {
    let invalid = || {
        io::Error::new(io::ErrorKind::InvalidInput,
        "provide search terms: amx google <terms>, amx search <source> <terms> or amx docs <tool> <terms> (up to 256 arguments and 4096 UTF-8 bytes; no control characters)")
    };
    if arguments.is_empty() || arguments.len() > MAX_QUERY_ARGUMENTS {
        return Err(invalid());
    }
    let mut bytes = arguments.len() - 1;
    for argument in arguments {
        bytes = bytes.saturating_add(argument.len());
        if bytes > MAX_QUERY_BYTES || argument.chars().any(char::is_control) {
            return Err(invalid());
        }
    }
    let query = arguments.join(" ");
    if query.trim().is_empty() {
        return Err(invalid());
    }
    Ok(query)
}

fn query_url(arguments: &[String]) -> io::Result<String> {
    let query = validated_query(arguments)?;
    let mut url =
        url::Url::parse("https://www.google.com/search").expect("fixed Google origin");
    url.query_pairs_mut().append_pair("q", &query);
    Ok(url.into())
}

fn run(
    command: &GoogleCommand,
    output: &mut impl Write,
    open: impl FnOnce(&str) -> io::Result<()>,
) -> io::Result<()> {
    let url = query_url(&command.query)?;
    if command.print_url {
        writeln!(output, "{url}")
    } else {
        open(&url).map_err(|_| io::Error::other(
            "could not open Google search; check your default browser or use amx google --print-url <search terms>"))?;
        writeln!(output, "Google search sent to your default browser.")
    }
}

pub fn execute(command: &GoogleCommand) -> io::Result<()> {
    run(command, &mut io::stdout().lock(), super::desktop_open::open)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    fn command(args: &[&str], preview: bool) -> GoogleCommand {
        GoogleCommand {
            query: args.iter().map(|s| (*s).into()).collect(),
            print_url: preview,
        }
    }

    #[test]
    fn google_command_encodes_one_exact_query_without_changing_origin() {
        let url = query_url(
            &command(&["rust & café", "+#%", "\"exact phrase\"", "🦀"], false).query,
        )
        .unwrap();
        assert_eq!(url, "https://www.google.com/search?q=rust+%26+caf%C3%A9+%2B%23%25+%22exact+phrase%22+%F0%9F%A6%80");
        for value in [
            "https://example.invalid/?q=other#x",
            "$(touch marker); & run",
            "e\u{301} 中文",
            "-site:example.invalid",
            "a  b",
        ] {
            let encoded = query_url(&command(&[value], false).query).unwrap();
            let parsed = url::Url::parse(&encoded).unwrap();
            assert_eq!(
                parsed.origin().ascii_serialization(),
                "https://www.google.com"
            );
            assert_eq!(parsed.path(), "/search");
            assert!(parsed.fragment().is_none());
            assert_eq!(
                parsed.query_pairs().collect::<Vec<_>>(),
                [("q".into(), value.into())]
            );
        }
    }

    #[test]
    fn google_command_limits_empty_controls_and_no_unintended_side_effects() {
        for value in ["", "  ", "\n", "test\0value", "test\u{1b}value"] {
            let mut output = Vec::new();
            assert!(run(&command(&[value], false), &mut output, |_| panic!(
                "invalid query opened browser"
            ))
            .is_err());
            assert!(output.is_empty());
        }
        for bytes in [MAX_QUERY_BYTES - 1, MAX_QUERY_BYTES, MAX_QUERY_BYTES + 1] {
            assert_eq!(
                query_url(&["a".repeat(bytes)]).is_ok(),
                bytes <= MAX_QUERY_BYTES
            );
        }
        for count in [
            0,
            1,
            MAX_QUERY_ARGUMENTS - 1,
            MAX_QUERY_ARGUMENTS,
            MAX_QUERY_ARGUMENTS + 1,
        ] {
            assert_eq!(
                query_url(&vec!["a".into(); count]).is_ok(),
                count > 0 && count <= MAX_QUERY_ARGUMENTS
            );
        }
        assert!(query_url(&["a".repeat(MAX_QUERY_BYTES - 1), "b".into()]).is_err());
    }

    #[test]
    fn google_command_dispatch_preview_failure_and_debug_are_private() {
        let mut output = Vec::new();
        run(&command(&["fixture query"], true), &mut output, |_| {
            panic!("preview opened browser")
        })
        .unwrap();
        assert_eq!(output, b"https://www.google.com/search?q=fixture+query\n");
        output.clear();
        let mut calls = 0;
        run(&command(&["fixture query"], false), &mut output, |url| {
            calls += 1;
            assert_eq!(url, "https://www.google.com/search?q=fixture+query");
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert!(!String::from_utf8(output.clone())
            .unwrap()
            .contains("fixture query"));
        output.clear();
        let error = run(&command(&["fixture query"], false), &mut output, |_| {
            Err(io::Error::other("private launcher detail"))
        })
        .unwrap_err();
        assert!(!error.to_string().contains("private launcher detail"));
        assert!(output.is_empty());
        let parsed = crate::cli::Cli::try_parse_from([
            "automexia",
            "google",
            "secret-fixture",
            "--literal",
        ])
        .unwrap();
        assert!(!format!("{parsed:?}").contains("secret-fixture"));
        let Some(crate::cli::CliCommand::Google(parsed)) = parsed.command else {
            panic!("wrong command");
        };
        assert_eq!(parsed.query, ["secret-fixture", "--literal"]);
    }

    #[test]
    #[ignore = "explicit correctness-checked Google URL construction benchmark"]
    fn google_command_benchmark_checked_encoding() {
        let arguments = command(&["rust & café", "🦀"], false).query;
        let mut times = Vec::new();
        for _ in 0..100 {
            let begin = std::time::Instant::now();
            for _ in 0..1000 {
                assert_eq!(
                    query_url(&arguments).unwrap(),
                    "https://www.google.com/search?q=rust+%26+caf%C3%A9+%F0%9F%A6%80"
                );
            }
            times.push(begin.elapsed().as_micros());
        }
        times.sort_unstable();
        println!("1000 checked Google URL encodings: median={}us p95={}us; no browser/network timing", times[50], times[95]);
    }
}
