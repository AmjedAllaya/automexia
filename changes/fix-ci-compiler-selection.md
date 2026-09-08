Select the repository-pinned Rust compiler consistently in CI and release
workflows without changing runner-global defaults. Quality cache generations
now follow the selected compiler; mutation tests reject compiler and cache
drift, ambiguous selectors and malformed pin files. Release permissions,
explicit nightly checks and the disabled automatic local pre-push gate are
unchanged.
