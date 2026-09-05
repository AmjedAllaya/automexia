use automexia_devops_aws::{parse_public_config, MAX_PROFILES};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn maximum_public_config() -> Vec<u8> {
    let mut config = String::new();
    for index in 0..MAX_PROFILES {
        config.push_str(&format!(
            "[sso-session session-{index}]\nsso_region=eu-west-3\nsso_start_url=https://example.awsapps.com/start\n\
             [profile profile-{index}]\nregion=eu-west-1\nsso_session=session-{index}\nsso_account_id={index:012}\nsso_role_name=Developer\n"
        ));
    }
    config.into_bytes()
}

fn benchmark_public_config(criterion: &mut Criterion) {
    let config = maximum_public_config();
    criterion.bench_function(
        "aws_public_config_128_profiles_and_sso_sessions",
        |bencher| {
            bencher.iter(|| parse_public_config(black_box(&config)).unwrap());
        },
    );
}

criterion_group!(benches, benchmark_public_config);
criterion_main!(benches);
