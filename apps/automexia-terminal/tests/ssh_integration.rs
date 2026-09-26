//! Application command parsing and pure integration composition; no SSH server.
use automexia_terminal::{
    automexia::ssh_integration,
    cli::{Cli, CliCommand},
};
use clap::Parser;
fn command(args: &[&str]) -> ssh_integration::Command {
    let parsed = Cli::try_parse_from(args).unwrap();
    match parsed.command.unwrap() {
        CliCommand::SshIntegration(command) => command,
        _ => panic!("SSH integration CLI not routed"),
    }
}
#[test]
fn ssh_library_status_is_explicitly_non_activated() {
    let command = command(&["automexia", "ssh-integration", "status"]);
    for broker in [false, true] {
        let result = ssh_integration::report(&command, broker).unwrap();
        assert_eq!(result["native_broker_enabled"], broker);
        assert_eq!(result["enhanced_execution_enabled"], false);
        assert_eq!(
            result["implemented_runtime_capabilities"],
            serde_json::json!([])
        );
    }
}
#[test]
fn ssh_library_cli_does_not_expose_argv_or_host() {
    let command = command(&[
        "automexia",
        "ssh-integration",
        "inspect",
        "--tty",
        "--",
        "secret-host",
        "printf token=secret",
    ]);
    let report = ssh_integration::report(&command, false)
        .unwrap()
        .to_string();
    assert!(!report.contains("secret-host"));
    assert!(!report.contains("token=secret"));
    assert!(!format!("{command:?}").contains("secret-host"));
    assert!(report.contains("ExplicitCommand"));
}
#[test]
fn ssh_library_cli_candidate_requires_all_declared_conditions() {
    let command = command(&[
        "automexia",
        "ssh-integration",
        "inspect",
        "--tty",
        "--shell",
        "bash",
        "--startup",
        "interactive",
        "--posix-account-shell",
        "--permit-session-files",
        "--assume-compatible-config",
        "--",
        "host",
    ]);
    let report = ssh_integration::report(&command, false).unwrap();
    assert_eq!(report["decision"], "candidate-needs-new-review");
    assert_eq!(report["enhanced_execution_enabled"], false);
}
#[test]
fn ssh_library_cli_native_by_default() {
    let command = command(&["automexia", "ssh-integration", "inspect", "--", "host"]);
    let report = ssh_integration::report(&command, false).unwrap();
    assert_eq!(report["fallback"], "NotInteractive");
}
#[test]
fn ssh_library_cli_cannot_accept_an_apply_or_enable_flag() {
    for flag in ["--apply", "--enable", "--execute", "--force"] {
        assert!(
            Cli::try_parse_from(["automexia", "ssh-integration", "inspect", flag])
                .is_err()
        );
    }
}
#[test]
fn ssh_library_cli_unknown_option_stays_native() {
    let command = command(&[
        "automexia",
        "ssh-integration",
        "inspect",
        "--tty",
        "--",
        "-oRemoteCommand=echo test",
        "host",
    ]);
    let report = ssh_integration::report(&command, false).unwrap();
    assert_eq!(report["fallback"], "UnsupportedOption");
}
