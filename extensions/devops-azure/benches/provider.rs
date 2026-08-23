use automexia_devops_azure::parse_public_accounts;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

fn maximum_public_accounts() -> Vec<u8> {
    let mut accounts = Vec::with_capacity(128);
    for index in 0..128_u32 {
        accounts.push(format!(
            r#"{{"cloudName":"AzureCloud","id":"00000000-0000-0000-0000-{index:012}","isDefault":false,"name":"Subscription {index}","state":"Enabled","tenantId":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","user":{{"name":"benchmark@example.invalid","type":"user"}}}}"#
        ));
    }
    format!("[{}]", accounts.join(",")).into_bytes()
}

fn benchmark_public_accounts(criterion: &mut Criterion) {
    let input = maximum_public_accounts();
    criterion.bench_function("azure_public_accounts_128", |bencher| {
        bencher.iter(|| {
            let parsed = parse_public_accounts(black_box(&input)).expect("valid fixture");
            black_box(parsed);
        });
    });
}

criterion_group!(benches, benchmark_public_accounts);
criterion_main!(benches);
