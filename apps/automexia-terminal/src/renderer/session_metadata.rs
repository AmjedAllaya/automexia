#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MetadataReadiness {
    Pending,
    #[default]
    Complete,
    Unavailable,
}

#[derive(Default)]
pub(crate) struct ShellMetadataState {
    readiness: MetadataReadiness,
}

impl ShellMetadataState {
    pub(crate) fn readiness(&self) -> MetadataReadiness {
        self.readiness
    }
}
