#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WslContext {
    pub distro: String,
    pub user: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubernetesContext {
    pub context: String,
    pub namespace: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CloudContext {
    pub provider: &'static str,
    pub profile: String,
    pub region: String,
}

/// Immutable-ish UI model produced by the DevOps discovery worker.
///
/// A snapshot represents configured/local context, not a guarantee that a
/// remote daemon, cluster or cloud API is currently reachable/authenticated.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DevOpsSnapshot {
    pub wsl: Option<WslContext>,
    pub kubernetes: Option<KubernetesContext>,
    pub docker: Option<String>,
    pub clouds: Vec<CloudContext>,
    pub terraform: Option<String>,
    pub git_branch: Option<String>,
    pub user: Option<String>,
    pub environment: Option<String>,
    pub production: bool,
}
