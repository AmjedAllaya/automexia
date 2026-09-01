use std::process::Command;

fn forbidden() {
    let _ = Command::new("sh").arg("-c");
}
