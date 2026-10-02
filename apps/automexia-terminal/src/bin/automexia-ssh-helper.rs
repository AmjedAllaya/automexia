//! Optional native helper uploaded only by an explicit +ssh invocation.
fn main() {
    match automexia_terminal::automexia::ssh_helper::run(
        std::env::args_os().skip(1).collect(),
    ) {
        Ok(Some(status)) => {
            automexia_terminal::automexia::ssh_wrapper::exit_with_status(status)
        }
        Ok(None) => {}
        Err(_) => {
            eprintln!("Automexia SSH helper could not complete this request.");
            std::process::exit(70);
        }
    }
}
