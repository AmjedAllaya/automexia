// Fictional df-style output with fixed-width, right-aligned numeric fields.
pub fn disk_rows() -> Vec<String> {
    [
        ["Filesystem", "Size", "Used", "Avail", "Use%", "Mounted on"],
        [
            "none",
            "5.9G",
            "0",
            "5.9G",
            "0%",
            "/usr/lib/modules/example-kernel",
        ],
        [
            "drivers",
            "238G",
            "230G",
            "8.0G",
            "97%",
            "/usr/lib/wsl/drivers",
        ],
        ["/dev/data", "1007G", "108G", "849G", "12%", "/"],
        [
            "disk one",
            "5.9G",
            "1024M",
            "4.9G",
            "17%",
            "/mnt/archive volume",
        ],
        ["snapfuse", "128K", "128K", "0", "100%", "/snap/example/5"],
    ]
    .into_iter()
    .map(|[device, size, used, avail, percent, mount]| {
        format!("{device:<16} {size:>5} {used:>5} {avail:>5} {percent:>4} {mount}")
    })
    .collect()
}
