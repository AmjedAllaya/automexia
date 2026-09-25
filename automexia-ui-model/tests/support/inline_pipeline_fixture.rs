// Fictional, producer-independent records. Safe for native terminal replay.
pub fn pipeline_rows() -> Vec<String> {
    [
        [
            "RESOURCE ID",
            "IMAGE",
            "COMMAND",
            "CREATED",
            "STATUS",
            "ENDPOINTS",
            "NAMES",
        ],
        [
            "resource-a001",
            "registry/service:v1",
            "\"runner --serve --port=8080\"",
            "3 weeks ago",
            "Up 6 seconds",
            "0.0.0.0:8080->80/tcp, 0.0.0.0:8443->443/tcp, 127.0.0.1:43665->6443/tcp",
            "example-control",
        ],
        [
            "resource-b002",
            "registry/service:v1",
            "\"runner --serve --port=8080\"",
            "3 weeks ago",
            "Up 6 seconds",
            "",
            "example-worker",
        ],
    ]
    .into_iter()
    .map(|r| {
        format!(
            "{:<16}{:<28}{:<34}{:<18}{:<24}{:<116}{}",
            r[0], r[1], r[2], r[3], r[4], r[5], r[6]
        )
    })
    .collect()
}

pub const PIPELINE_STARTS: [usize; 7] = [0, 16, 44, 78, 96, 120, 236];
