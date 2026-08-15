use std::{fs, hint::black_box, path::Path};

use automexia_devops_ssh::{scan_inventory, GrantKind, InventoryGrant, InventoryLimits};
use criterion::{criterion_group, criterion_main, Criterion};

fn secure_file(path: &Path, contents: &str) {
    fs::write(path, contents).expect("benchmark fixture write");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .expect("benchmark fixture permissions");
    }
}

fn inventory_benchmark(criterion: &mut Criterion) {
    let root = tempfile::tempdir().expect("benchmark root");
    let config = root.path().join("config");
    let mut contents = String::with_capacity(500_000);
    for index in 0..10_000 {
        contents.push_str(&format!(
            "Host node-{index}\n HostName node-{index}.example\n User deploy\n"
        ));
    }
    secure_file(&config, &contents);
    let grant = InventoryGrant::new("benchmark", root.path(), [&config], GrantKind::User)
        .expect("benchmark grant");

    criterion.bench_function("openssh_inventory_10000_aliases", |bencher| {
        let mut generation = 0;
        bencher.iter(|| {
            generation += 1;
            black_box(
                scan_inventory(
                    std::slice::from_ref(&grant),
                    InventoryLimits::default(),
                    generation,
                )
                .expect("bounded benchmark scan"),
            )
        });
    });
}

criterion_group!(benches, inventory_benchmark);
criterion_main!(benches);
