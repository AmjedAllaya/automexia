//! Package-owned persistent CP5 helper.
//!
//! The process accepts no arguments or environment authority. Its stdin starts
//! with one inherited bootstrap frame and then carries bounded shell requests;
//! stdout carries only restricted shell response records.

use std::io::{BufReader, Write};
use std::process::ExitCode;

use automexia_terminal::automexia::suggestions::{
    connect_helper_endpoint, read_helper_bootstrap, run_helper_records,
    HelperSessionBridge,
};

fn main() -> ExitCode {
    if std::env::args_os().len() != 1 {
        return ExitCode::from(64);
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = BufReader::new(stdin.lock());
    let mut output = stdout.lock();
    let result = (|| {
        let bootstrap = read_helper_bootstrap(&mut input).map_err(|_| ())?;
        let mut bridge = HelperSessionBridge::new(bootstrap.binding).map_err(|_| ())?;
        let mut endpoint =
            connect_helper_endpoint(&bootstrap.endpoint).map_err(|_| ())?;
        run_helper_records(&mut input, &mut output, &mut bridge, &mut endpoint)
            .map_err(|_| ())?;
        output.flush().map_err(|_| ())
    })();
    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(70)
    }
}
