//! One-shot browser search planning. No filesystem, terminal or network state.
use crate::cli::SearchCommand;
use std::io::{self, Write};

pub(crate) struct SearchProvider {
    pub id: &'static str,
    pub description: &'static str,
    endpoint: &'static str,
    query_key: &'static str,
}

pub(crate) const SEARCH_PROVIDERS: &[SearchProvider] = &[
    SearchProvider {
        id: "google",
        description: "Google web search",
        endpoint: "https://www.google.com/search",
        query_key: "q",
    },
    SearchProvider {
        id: "github",
        description: "GitHub repositories",
        endpoint: "https://github.com/search?type=repositories",
        query_key: "q",
    },
    SearchProvider {
        id: "youtube",
        description: "YouTube videos",
        endpoint: "https://www.youtube.com/results",
        query_key: "search_query",
    },
    SearchProvider {
        id: "ddg",
        description: "DuckDuckGo web search",
        endpoint: "https://duckduckgo.com/",
        query_key: "q",
    },
];

pub(crate) const DOC_SITES: &[(&str, &str)] = &[
    ("kubernetes", "kubernetes.io/docs"),
    ("docker", "docs.docker.com"),
    ("rust", "doc.rust-lang.org"),
    ("python", "docs.python.org/3"),
    ("git", "git-scm.com/docs"),
    ("terraform", "developer.hashicorp.com/terraform"),
];

pub fn provider_help() -> String {
    SEARCH_PROVIDERS
        .iter()
        .map(|p| format!("{}: {}", p.id, p.description))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn docs_help() -> String {
    format!("Official documentation via Google's site filter: {}. Terms are sent to Google only when opening; --print-url is offline.", DOC_SITES.iter().map(|(id, _)| *id).collect::<Vec<_>>().join(", "))
}

fn search_url(source: &str, arguments: &[String]) -> io::Result<String> {
    let provider = SEARCH_PROVIDERS
        .iter()
        .find(|p| p.id == source)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown search source; use amx search --help",
            )
        })?;
    let query = super::google::validated_query(arguments)?;
    let mut url = url::Url::parse(provider.endpoint).expect("fixed search endpoint");
    url.query_pairs_mut()
        .append_pair(provider.query_key, &query);
    Ok(url.into())
}

fn docs_url(tool: &str, arguments: &[String]) -> io::Result<String> {
    let (_, site) = DOC_SITES
        .iter()
        .find(|(id, _)| *id == tool)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown documentation tool; use amx docs --help",
            )
        })?;
    let query = super::google::validated_query(arguments)?;
    let mut url = url::Url::parse("https://www.google.com/search")
        .expect("fixed docs search endpoint");
    // Keep the official-site constraint separate from user-controlled query syntax.
    url.query_pairs_mut()
        .append_pair("as_sitesearch", site)
        .append_pair("q", &query);
    Ok(url.into())
}

fn run(
    command: &SearchCommand,
    docs: bool,
    output: &mut impl Write,
    open: impl FnOnce(&str) -> io::Result<()>,
) -> io::Result<()> {
    let url = if docs {
        docs_url(&command.source, &command.search.query)?
    } else {
        search_url(&command.source, &command.search.query)?
    };
    if command.search.print_url {
        writeln!(output, "{url}")
    } else {
        open(&url).map_err(|_| io::Error::other("could not open search; check the default browser or use --print-url before the search terms"))?;
        writeln!(
            output,
            "{} sent to your default browser.",
            if docs {
                "Official documentation search (via Google)"
            } else {
                "Search"
            }
        )
    }
}

pub fn execute(command: &SearchCommand, docs: bool) -> io::Result<()> {
    run(
        command,
        docs,
        &mut io::stdout().lock(),
        super::desktop_open::open,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amx_browser_search_preserves_exact_provider_query_and_repository_scope() {
        let query = vec!["rust & café".into(), "🦀 +#%".into()];
        for (source, expected) in [
            ("google", "https://www.google.com/search?q=rust+%26+caf%C3%A9+%F0%9F%A6%80+%2B%23%25"),
            ("github", "https://github.com/search?type=repositories&q=rust+%26+caf%C3%A9+%F0%9F%A6%80+%2B%23%25"),
            ("youtube", "https://www.youtube.com/results?search_query=rust+%26+caf%C3%A9+%F0%9F%A6%80+%2B%23%25"),
            ("ddg", "https://duckduckgo.com/?q=rust+%26+caf%C3%A9+%F0%9F%A6%80+%2B%23%25"),
        ] {
            assert_eq!(search_url(source, &query).unwrap(), expected);
        }
    }

    #[test]
    fn amx_browser_search_rejects_unknown_sources_without_fallback() {
        for source in [
            "",
            "missing",
            "https://example.invalid",
            "--help",
            "github\n",
        ] {
            assert!(search_url(source, &["test".into()]).is_err());
        }
    }

    fn command(source: &str, query: Vec<String>, print_url: bool) -> SearchCommand {
        SearchCommand {
            source: source.into(),
            search: crate::cli::GoogleCommand { query, print_url },
        }
    }

    #[test]
    fn amx_browser_search_docs_site_is_separate_from_user_operators() {
        assert_eq!(docs_url("kubernetes", &["deployment".into()]).unwrap(), "https://www.google.com/search?as_sitesearch=kubernetes.io%2Fdocs&q=deployment");
        for (tool, site) in DOC_SITES {
            let url = url::Url::parse(
                &docs_url(
                    tool,
                    &["site:example.invalid OR &as_sitesearch=other#x".into()],
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(url.host_str(), Some("www.google.com"));
            assert_eq!(
                url.query_pairs()
                    .filter(|(key, _)| key == "as_sitesearch")
                    .collect::<Vec<_>>(),
                [("as_sitesearch".into(), (*site).into())]
            );
            assert!(url.fragment().is_none());
        }
        assert!(docs_url("missing", &["test".into()]).is_err());
    }

    #[test]
    fn amx_browser_search_every_route_enforces_limits_before_dispatch() {
        // Exercise the public route, not just the shared validator: adding a
        // provider must not accidentally bypass input or preview policy.
        for (docs, source) in SEARCH_PROVIDERS
            .iter()
            .map(|p| (false, p.id))
            .chain(DOC_SITES.iter().map(|(id, _)| (true, *id)))
        {
            for query in [
                vec![],
                vec![" ".into()],
                vec!["a\n".into()],
                vec!["a\0b".into()],
                vec!["\u{1b}]52;payload".into()],
                vec!["a".repeat(4097)],
                vec!["a".into(); 257],
                vec!["a".repeat(4095), "b".into()],
            ] {
                let mut out = Vec::new();
                assert!(
                    run(&command(source, query, false), docs, &mut out, |_| panic!(
                        "invalid query dispatched"
                    ))
                    .is_err()
                );
                assert!(out.is_empty());
            }
            for query in [
                vec!["a".repeat(4095)],
                vec!["é".repeat(2048)],
                vec!["a".into(); 256],
                vec!["$(touch fixture); & run".into()],
            ] {
                let mut out = Vec::new();
                run(&command(source, query, true), docs, &mut out, |_| {
                    panic!("preview dispatched")
                })
                .unwrap();
                assert!(!out.is_empty());
            }
        }
    }

    #[test]
    fn amx_browser_search_dispatch_failure_debug_and_cli_roundtrip() {
        use clap::Parser;
        for family in ["search", "docs"] {
            let source = if family == "docs" {
                "kubernetes"
            } else {
                "github"
            };
            let parsed = crate::cli::Cli::try_parse_from([
                "automexia",
                family,
                source,
                "--print-url",
                "fixture-private",
                "--literal",
            ])
            .unwrap();
            assert!(!format!("{parsed:?}").contains("fixture-private"));
            let Some(
                crate::cli::CliCommand::Search(parsed)
                | crate::cli::CliCommand::Docs(parsed),
            ) = parsed.command
            else {
                panic!("wrong route");
            };
            assert_eq!(parsed.source, source);
            assert_eq!(parsed.search.query, ["fixture-private", "--literal"]);
            assert!(parsed.search.print_url);
            let mut out = Vec::new();
            let mut calls = 0;
            run(
                &command(source, vec!["fixture-private".into()], false),
                family == "docs",
                &mut out,
                |target| {
                    calls += 1;
                    assert!(target.starts_with("https://"));
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(calls, 1);
            assert!(!String::from_utf8(out.clone())
                .unwrap()
                .contains("fixture-private"));
            out.clear();
            let error = run(
                &command(source, vec!["fixture-private".into()], false),
                family == "docs",
                &mut out,
                |_| Err(io::Error::other("private handler detail")),
            )
            .unwrap_err();
            assert!(!error.to_string().contains("private handler detail"));
            assert!(out.is_empty());
        }
    }

    #[test]
    #[ignore = "explicit correctness-checked browser routing benchmark"]
    fn amx_browser_search_benchmark_checked_routing() {
        let query = vec!["rust & café".into()];
        let mut times = Vec::new();
        for _ in 0..100 {
            let begin = std::time::Instant::now();
            for _ in 0..1000 {
                assert_eq!(
                    search_url("github", &query).unwrap(),
                    "https://github.com/search?type=repositories&q=rust+%26+caf%C3%A9"
                );
                assert_eq!(docs_url("kubernetes", &query).unwrap(), "https://www.google.com/search?as_sitesearch=kubernetes.io%2Fdocs&q=rust+%26+caf%C3%A9");
            }
            times.push(begin.elapsed().as_micros());
        }
        times.sort_unstable();
        println!("1000 checked search+docs pairs: median={}us p95={}us; URL planning only, no browser/network timing", times[50], times[95]);
    }
}
