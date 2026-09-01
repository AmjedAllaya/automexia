use std::process::Command;

fn allowed() {
    let _ = Command::new("ssh").arg("example.invalid");
}

fn non_literal_fixture_alias_is_safe() {
    let fixture = "<redacted>";
    let secret = fixture;
    assert_eq!(secret, fixture);
}
