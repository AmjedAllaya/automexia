#![no_main]

use automexia_devops::actions::{preview_native_alias_import, NativeAliasSource};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let sources = [
        NativeAliasSource::Powershell,
        NativeAliasSource::Bash,
        NativeAliasSource::Zsh,
        NativeAliasSource::Fish,
        NativeAliasSource::Cmd,
        NativeAliasSource::Git,
    ];
    for source in sources {
        if let Ok(preview) = preview_native_alias_import(source, data) {
            assert!(preview.importable_count() <= preview.entries.len());
            assert!(preview.rejected_count() <= preview.entries.len());
            assert_eq!(
                preview.importable_count() + preview.rejected_count(),
                preview.entries.len()
            );
        }
    }
});
