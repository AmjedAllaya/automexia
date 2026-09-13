//! Local Git metadata to a reviewed browser destination; never fetch or authenticate.
use super::local_tools::{self, ToolSession};
use crate::cli::{RepoCommand, RepoPage};
use std::io::{self, Write};

const MAX_REMOTE_BYTES: usize = 4096;
const MAX_REMOTE_NAME_BYTES: usize = 128;

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput,
        "unsupported repository remote; use a credential-free GitHub.com or GitLab.com HTTPS or standard Git SSH URL. Local paths, SSH aliases and ambiguous URLs are not opened")
}

fn arguments(remote: &str) -> io::Result<Vec<String>> {
    if remote.is_empty()
        || remote.len() > MAX_REMOTE_NAME_BYTES
        || !remote.starts_with(|c: char| c.is_ascii_alphanumeric())
        || !remote
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'/'))
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            "provide a configured remote name up to 128 ASCII bytes beginning with a letter or number; use --remote to select it explicitly"));
    }
    Ok(["--no-pager", "remote", "get-url", "--", remote]
        .into_iter()
        .map(str::to_owned)
        .collect())
}

pub fn execute(command: &RepoCommand, session: &ToolSession) -> io::Result<()> {
    let args = arguments(&command.remote)?;
    let result = local_tools::run_tool("git", &args, session).map_err(|error|
        io::Error::new(error.kind(), "local Git lookup failed; ensure Git is installed (and Python 3.10+ in Windows-backed WSL), then check the session and permissions. Nothing was installed or authenticated"))?;
    if !result.status.success() {
        return Err(io::Error::other("cannot read the selected Git remote; run inside its repository, check repository ownership and use --remote for a non-origin remote. No configuration was changed"));
    }
    let destination = destination(&result.stdout, command.page)?;
    dispatch(
        &destination,
        command.preview,
        super::desktop_open::open,
        &mut io::stdout().lock(),
    )
}

fn destination(bytes: &[u8], page: RepoPage) -> io::Result<String> {
    if bytes.len() > MAX_REMOTE_BYTES + 2 {
        return Err(invalid());
    }
    let raw = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let raw = match raw.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').unwrap_or(line),
        None => raw,
    };
    if raw.is_empty()
        || raw.len() > MAX_REMOTE_BYTES
        || !raw.is_ascii()
        || raw
            .bytes()
            .any(|c| c.is_ascii_control() || c.is_ascii_whitespace())
        || raw.contains(['%', '\\', '?', '#'])
    {
        return Err(invalid());
    }
    let (host, path) = if let Some(scp) = raw.strip_prefix("git@") {
        let (host, path) = scp.split_once(':').ok_or_else(invalid)?;
        (host.to_ascii_lowercase(), path)
    } else {
        // Keep the original path for validation: URL parsing normalizes dot
        // segments, which would otherwise conceal an ambiguous remote spelling.
        let (_, rest) = raw.split_once("://").ok_or_else(invalid)?;
        let (_, path) = rest.split_once('/').ok_or_else(invalid)?;
        let url = url::Url::parse(raw).map_err(|_| invalid())?;
        let credentials_ok = match url.scheme() {
            "https" => {
                url.username().is_empty()
                    && url.password().is_none()
                    && url.port().is_none()
            }
            "ssh" => {
                url.username() == "git"
                    && url.password().is_none()
                    && matches!(url.port(), None | Some(22))
            }
            _ => false,
        };
        if !credentials_ok || url.query().is_some() || url.fragment().is_some() {
            return Err(invalid());
        }
        (
            url.host_str().ok_or_else(invalid)?.to_ascii_lowercase(),
            path,
        )
    };
    if !matches!(host.as_str(), "github.com" | "gitlab.com") {
        return Err(invalid());
    }
    let path = path.strip_suffix('/').unwrap_or(path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut components = 0;
    for part in path.split('/') {
        components += 1;
        if components > 16
            || part.is_empty()
            || part.len() > 255
            || matches!(part, "." | ".." | "-")
            || !part
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        {
            return Err(invalid());
        }
    }
    if components < 2 || (host == "github.com" && components != 2) {
        return Err(invalid());
    }
    let suffix = match (page, host.as_str()) {
        (RepoPage::Root, _) => "",
        (RepoPage::Issues, "github.com") => "/issues",
        (RepoPage::Issues, _) => "/-/issues",
    };
    Ok(format!("https://{host}/{path}{suffix}"))
}

fn dispatch(
    destination: &str,
    preview: bool,
    open: impl FnOnce(&str) -> io::Result<()>,
    output: &mut impl Write,
) -> io::Result<()> {
    if preview {
        let plan = serde_json::json!({"action":"browse-repository", "destination":destination, "execution":"preview-only"});
        writeln!(output, "{plan}")
    } else {
        open(destination).map_err(|_| io::Error::other("repository browser handoff failed; check your desktop browser registration. No fetch, push or authentication was requested"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amx_repo_protocols_and_issue_routes_keep_exact_repository_identity() {
        for remote in [
            "git@github.com:example-org/fixture.git",
            "https://github.com/example-org/fixture.git",
            "ssh://git@github.com/example-org/fixture",
            "ssh://git@github.com:22/example-org/fixture.git/",
            "https://github.com:443/example-org/fixture\r\n",
        ] {
            assert_eq!(
                destination(remote.as_bytes(), RepoPage::Root).unwrap(),
                "https://github.com/example-org/fixture"
            );
            assert_eq!(
                destination(remote.as_bytes(), RepoPage::Issues).unwrap(),
                "https://github.com/example-org/fixture/issues"
            );
        }
        assert_eq!(
            destination(
                b"https://gitlab.com/example/subgroup/fixture.git\n",
                RepoPage::Issues
            )
            .unwrap(),
            "https://gitlab.com/example/subgroup/fixture/-/issues"
        );
    }

    #[test]
    fn amx_repo_dns_case_is_insensitive_but_repository_case_is_preserved() {
        // SSH is not a special URL scheme: its parser does not fold DNS case.
        for (host, canonical, suffix) in [
            ("GitHub.COM", "github.com", "/issues"),
            ("GitLab.COM", "gitlab.com", "/-/issues"),
        ] {
            for remote in [
                format!("ssh://git@{host}:22/Example-Org/Fixture-Repo.git"),
                format!("git@{host}:Example-Org/Fixture-Repo.git"),
                format!("https://{host}/Example-Org/Fixture-Repo.git"),
            ] {
                assert_eq!(
                    destination(remote.as_bytes(), RepoPage::Root).unwrap(),
                    format!("https://{canonical}/Example-Org/Fixture-Repo")
                );
                assert_eq!(
                    destination(remote.as_bytes(), RepoPage::Issues).unwrap(),
                    format!("https://{canonical}/Example-Org/Fixture-Repo{suffix}")
                );
            }
        }
    }

    #[test]
    fn amx_repo_credentials_hosts_and_ambiguous_paths_are_never_opened() {
        for remote in [
            "",
            "fixture",
            "../fixture.git",
            "file:///fixture",
            "http://github.com/a/b",
            "https://alice@github.com/a/b",
            "https://alice:fixture@github.com/a/b",
            "ssh://alice@github.com/a/b",
            "ssh://git:fixture@github.com/a/b",
            "https://github.com.example.invalid/a/b",
            "ssh://git@GitHub.COM.example.invalid/a/b",
            "ssh://git@GitLab.COM./a/b",
            "ssh://git@GITHUBCOM/a/b",
            "https://example.invalid/a/b",
            "git@alias:a/b",
            "git@github.com:/a/b",
            "https://github.com/a/b?query",
            "https://github.com/a/b#fragment",
            "https://github.com/a/../b",
            "https://github.com/a/./b",
            "https://github.com/a//b",
            "https://github.com/a/%62",
            "https://github.com/a/b\nother",
            "https://github.com/a/b\r",
            "https://github.com/a/b ",
            "https://github.com/a/b\u{202e}",
            "https://github.com/a",
            "https://github.com/a/b/c",
            "https://github.com:444/a/b",
            "ssh://git@github.com:23/a/b",
            "https://gitlab.com/a/-/b",
            "https://github.com/a/.git",
            "https://github.com/a\\b",
            "git@github.com:a:b/c",
        ] {
            let error = destination(remote.as_bytes(), RepoPage::Root).unwrap_err();
            assert!(!error.to_string().contains("alice"));
        }
        assert!(destination(&[0xff], RepoPage::Root).is_err());
    }

    #[test]
    fn amx_repo_argument_and_output_limits_are_enforced() {
        assert_eq!(
            arguments("upstream/work").unwrap(),
            ["--no-pager", "remote", "get-url", "--", "upstream/work"]
        );
        for name in ["", "--push", "bad\nname", "$(fixture)", &"x".repeat(129)] {
            assert!(arguments(name).is_err());
        }
        assert!(arguments(&"x".repeat(128)).is_ok());
        for size in [254, 255, 256] {
            let remote = format!("https://github.com/example/{}", "x".repeat(size));
            assert_eq!(
                destination(remote.as_bytes(), RepoPage::Root).is_ok(),
                size <= 255
            );
        }
        for count in [1, 2, 16, 17] {
            let remote = format!("https://gitlab.com/{}", vec!["group"; count].join("/"));
            assert_eq!(
                destination(remote.as_bytes(), RepoPage::Root).is_ok(),
                (2..=16).contains(&count)
            );
        }
        assert!(destination(&vec![b'x'; MAX_REMOTE_BYTES + 3], RepoPage::Root).is_err());
        let prefix = format!(
            "https://gitlab.com/{}/",
            vec!["x".repeat(255); 15].join("/")
        );
        for size in [MAX_REMOTE_BYTES - 1, MAX_REMOTE_BYTES, MAX_REMOTE_BYTES + 1] {
            let remote = format!("{prefix}{}", "y".repeat(size - prefix.len()));
            assert_eq!(
                destination(remote.as_bytes(), RepoPage::Root).is_ok(),
                size <= MAX_REMOTE_BYTES
            );
        }
    }

    #[test]
    fn amx_repo_preview_and_errors_do_not_launch_or_disclose_input() {
        let mut output = Vec::new();
        dispatch(
            "https://github.com/example/fixture",
            true,
            |_| panic!("preview launched browser"),
            &mut output,
        )
        .unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(plan["execution"], "preview-only");
        output.clear();
        let error = dispatch(
            "https://github.com/example/fixture",
            false,
            |_| Err(io::Error::other("private fixture")),
            &mut output,
        )
        .unwrap_err();
        assert!(!error.to_string().contains("private fixture"));
        assert!(output.is_empty());
    }

    #[test]
    fn amx_repo_cli_defaults_and_remote_debug_are_stable() {
        use clap::Parser;
        let cli =
            crate::cli::Cli::try_parse_from(["automexia", "repo", "--preview"]).unwrap();
        let Some(crate::cli::CliCommand::Repo(mut command)) = cli.command else {
            panic!("wrong command")
        };
        assert_eq!(command.page, RepoPage::Root);
        assert_eq!(command.remote, "origin");
        command.remote = "private-fixture".into();
        assert!(!format!("{command:?}").contains("private-fixture"));
    }

    #[test]
    #[ignore = "explicit correctness-checked repository URL microbenchmark"]
    fn amx_repo_benchmark_checked_remote_parsing() {
        let mut samples = Vec::with_capacity(100);
        for _ in 0..100 {
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                assert_eq!(
                    destination(
                        b"ssh://git@GitHub.COM/example/fixture.git\n",
                        RepoPage::Issues
                    )
                    .unwrap(),
                    "https://github.com/example/fixture/issues"
                );
            }
            samples.push(start.elapsed().as_micros());
        }
        samples.sort_unstable();
        println!("amx-repo 1000 checked remotes: median={}us p95={}us; excludes Git, filesystem, WSL and browser latency", samples[50], samples[95]);
    }
}
