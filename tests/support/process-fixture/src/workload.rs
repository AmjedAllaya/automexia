use std::{
    env, fs,
    io::{self, Read, Write},
    path::Path,
    process::{self, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub enum Workload {
    Wait,
    StdoutFlood,
    StderrFlood,
    UploadEcho,
    NativePersistentHelper,
    NativePersistentParent,
    Descendant,
    DescendantNoPipes,
    PromptStall,
    PromptOverflow,
    PromptOutput,
}

impl Workload {
    pub fn parse(mode: &str) -> Option<Self> {
        match mode {
            "wait" => Some(Self::Wait),
            "stdout-flood" => Some(Self::StdoutFlood),
            "stderr-flood" => Some(Self::StderrFlood),
            "upload-echo" => Some(Self::UploadEcho),
            "native-persistent-helper" => Some(Self::NativePersistentHelper),
            "native-persistent-parent" => Some(Self::NativePersistentParent),
            "descendant" => Some(Self::Descendant),
            "descendant-no-pipes" => Some(Self::DescendantNoPipes),
            "prompt-stall" => Some(Self::PromptStall),
            "prompt-overflow" => Some(Self::PromptOverflow),
            "prompt-output" => Some(Self::PromptOutput),
            _ => None,
        }
    }

    pub fn run(self, child_command: fn(&str) -> Command) {
        match self {
            Self::Wait => {
                let ready = env::var_os("AMX_PROCESS_READY").unwrap();
                fs::write(ready, process::id().to_string()).unwrap();
                thread::park_timeout(Duration::from_secs(30));
            }
            Self::StdoutFlood => {
                io::stdout().write_all(&vec![b'x'; 1024 * 1024]).unwrap();
            }
            Self::StderrFlood => {
                io::stderr().write_all(&vec![b'x'; 1024 * 1024]).unwrap();
            }
            Self::UploadEcho => {
                let mut input = Vec::new();
                io::stdin().take(65537).read_to_end(&mut input).unwrap();
                assert!(input.len() <= 65536);
                io::stdout().write_all(&input).unwrap();
                io::stdout().flush().unwrap();
                process::exit(7);
            }
            Self::NativePersistentHelper => {
                fs::write(
                    env::var_os("AMX_PROCESS_READY").unwrap(),
                    process::id().to_string(),
                )
                .unwrap();
                let release = env::var_os("AMX_PROCESS_BACKGROUND_RELEASE").unwrap();
                if release_received(Path::new(&release)) {
                    fs::write(env::var_os("AMX_PROCESS_RETIRED").unwrap(), b"retired")
                        .unwrap();
                }
            }
            Self::NativePersistentParent => {
                let _child = child_command("native-persistent-helper")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap();
                let release = env::var_os("AMX_PROCESS_RELEASE").unwrap();
                assert!(
                    release_received(Path::new(&release)),
                    "native leader release timed out"
                );
                process::exit(7);
            }
            Self::Descendant | Self::DescendantNoPipes => {
                let ready = env::var_os("AMX_PROCESS_READY").unwrap();
                let release = env::var_os("AMX_PROCESS_RELEASE").unwrap();
                let mut command = child_command("wait");
                command.env("AMX_PROCESS_READY", &ready);
                if matches!(self, Self::DescendantNoPipes) {
                    command.stdout(Stdio::null()).stderr(Stdio::null());
                }
                let _child = command.spawn().unwrap();
                assert!(
                    release_received(Path::new(&release)),
                    "descendant readiness timed out"
                );
                // Exit while the descendant is still running. The supervising
                // parent must retire the whole native group.
                process::exit(0);
            }
            Self::PromptStall => thread::sleep(Duration::from_secs(30)),
            Self::PromptOverflow => io::stdout().write_all(&[b'x'; 4097]).unwrap(),
            Self::PromptOutput => io::stdout().write_all(b"fixture-response").unwrap(),
        }
    }
}

fn release_received(path: &Path) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.is_file() {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
    true
}
