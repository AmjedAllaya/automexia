use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::{env, fs};

use super::model::DevOpsSnapshot;
#[cfg(not(target_arch = "wasm32"))]
use super::model::{CloudContext, KubernetesContext, WslContext};
use automexia_extension_api::SessionFacts;

const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_LABEL_CHARS: usize = 96;

pub fn detect(session: &SessionFacts) -> DevOpsSnapshot {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = session;
        return DevOpsSnapshot::default();
    }

    #[cfg(not(target_arch = "wasm32"))]
    detect_native(session)
}

/// Attach isolated guest discovery using the same domain classification owner.
#[cfg(not(target_arch = "wasm32"))]
pub fn attach_kubernetes_context(
    snapshot: &mut DevOpsSnapshot,
    context: Option<KubernetesContext>,
) {
    snapshot.production |= context.as_ref().is_some_and(|context| {
        looks_production(&context.context) || looks_production(&context.namespace)
    });
    snapshot.kubernetes = context;
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
struct SessionView {
    cwd: Option<PathBuf>,
    home: Option<PathBuf>,
    wsl: Option<WslContext>,

    /// The Automexia process environment belongs to the host process. Once a
    /// nested WSL shell is active it is not the Linux child's environment, so
    /// WSL discovery intentionally relies on local config files instead.
    use_process_env: bool,
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_native(session: &SessionFacts) -> DevOpsSnapshot {
    let host_home = dirs::home_dir();
    let view = session_view(session, host_home.as_deref());
    let cwd = view.cwd.as_deref();
    let home = view.home.as_deref();
    let project_context = project_automexia_context(cwd);
    let legacy_context = legacy_automexia_context(home, view.use_process_env);

    // Passive status discovery reads only bounded public local files and
    // session metadata. It never invokes WSL, a shell, or a provider CLI.
    let kubernetes = if view.wsl.is_some()
        || (cfg!(target_os = "windows") && crate::kubernetes::is_wsl_candidate(session))
    {
        // The application isolates guest-backed reads in a bounded helper.
        // Never substitute the host cluster while that result is unavailable.
        None
    } else {
        kubernetes_context(session, home).or_else(|| {
            project_context
                .as_ref()
                .and_then(kubernetes_from_automexia_json)
                .or_else(|| {
                    legacy_context
                        .as_ref()
                        .and_then(kubernetes_from_automexia_json)
                })
        })
    };

    let docker = docker_context(home, cwd, view.use_process_env)
        .or_else(|| {
            project_context
                .as_ref()
                .and_then(docker_from_automexia_json)
        })
        .or_else(|| legacy_context.as_ref().and_then(docker_from_automexia_json));

    let clouds = cloud_contexts(
        home,
        cwd,
        project_context.as_ref(),
        legacy_context.as_ref(),
        view.use_process_env,
    );

    let terraform = terraform_workspace(cwd, view.use_process_env)
        .or_else(|| {
            project_context
                .as_ref()
                .and_then(terraform_from_automexia_json)
        })
        .or_else(|| {
            legacy_context
                .as_ref()
                .and_then(terraform_from_automexia_json)
        });

    let git_branch = git_branch(cwd)
        .or_else(|| project_context.as_ref().and_then(git_from_automexia_json))
        .or_else(|| legacy_context.as_ref().and_then(git_from_automexia_json));

    let user = session
        .shell_integration
        .then(|| session.shell_user.clone())
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| view.wsl.as_ref().map(|context| context.user.clone()))
        .or_else(|| {
            view.use_process_env
                .then(|| env::var("USERNAME").ok().or_else(|| env::var("USER").ok()))
                .flatten()
        })
        .map(|value| sanitize_label(&value));

    let environment = view
        .use_process_env
        .then(inherited_environment)
        .flatten()
        .or_else(|| {
            project_context
                .as_ref()
                .and_then(environment_from_automexia_json)
        })
        .or_else(|| {
            legacy_context
                .as_ref()
                .and_then(environment_from_automexia_json)
        });

    let project = project_context
        .as_ref()
        .and_then(project_from_automexia_json)
        .or_else(|| {
            legacy_context
                .as_ref()
                .and_then(project_from_automexia_json)
        });

    let production = kubernetes
        .iter()
        .flat_map(|context| [&context.context, &context.namespace])
        .chain(docker.iter())
        .chain(terraform.iter())
        .chain(environment.iter())
        .chain(project.iter())
        .chain(
            clouds
                .iter()
                .flat_map(|cloud| [&cloud.profile, &cloud.region]),
        )
        .any(|value| looks_production(value));

    DevOpsSnapshot {
        wsl: view.wsl,
        kubernetes,
        docker: docker.map(|value| sanitize_label(&value)),
        clouds,
        terraform: terraform.map(|value| sanitize_label(&value)),
        git_branch: git_branch.map(|value| sanitize_label(&value)),
        environment: environment.map(|value| sanitize_label(&value)),
        user,
        production,
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn session_view(session: &SessionFacts, host_home: Option<&Path>) -> SessionView {
    #[cfg(target_os = "windows")]
    {
        if let Some(view) = windows_wsl_session_view(session) {
            return view;
        }
        if crate::kubernetes::is_wsl_candidate(session) {
            // Partial guest identity cannot authorize host home/provider reads.
            return SessionView {
                cwd: None,
                home: None,
                wsl: None,
                use_process_env: false,
            };
        }
    }
    let home = native_session_home(session, host_home);
    // Inherited provider variables belong to the application's initial user.
    // A published different/cleared home must not borrow that user's context.
    let use_process_env = !session.environment.contains_key("HOME")
        || (home.is_some() && home.as_deref() == host_home);
    SessionView {
        cwd: session
            .cwd
            .clone()
            .filter(|path| crate::kubernetes::bounded_local_path(path)),
        home,
        wsl: None,
        use_process_env,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_session_home(
    session: &SessionFacts,
    host_home: Option<&Path>,
) -> Option<PathBuf> {
    #[cfg(windows)]
    if crate::kubernetes::windows_location_hints(session) {
        let value = |name: &str| {
            session
                .environment
                .get(name)
                .filter(|value| !value.is_empty())
        };
        // The Windows user home is separate from kubectl's first-existing-config
        // search. An unset exported HOME is normal in PowerShell.
        let home = value("USERPROFILE")
            .cloned()
            .or_else(|| {
                value("HOMEDRIVE")
                    .zip(value("HOMEPATH"))
                    .map(|(drive, path)| format!("{drive}{path}"))
            })
            .or_else(|| value("HOME").cloned());
        return home
            .map(PathBuf::from)
            .filter(|path| crate::kubernetes::bounded_local_path(path));
    }
    session
        .environment
        .get("HOME")
        .map(PathBuf::from)
        .or_else(|| host_home.map(Path::to_path_buf))
        .filter(|path| crate::kubernetes::bounded_local_path(path))
}

#[cfg(target_os = "windows")]
fn windows_wsl_session_view(session: &SessionFacts) -> Option<SessionView> {
    if !session.shell_integration {
        return None;
    }
    // `WSL_DISTRO_NAME` is published by the Bash/Zsh integration. Requiring it
    // avoids treating Git Bash or another POSIX-looking Windows shell as WSL.
    let distro = session
        .distro
        .as_ref()
        .filter(|value| !value.trim().is_empty())?;
    if session.shell_name.as_deref().is_some_and(|shell| {
        shell.eq_ignore_ascii_case("PowerShell")
            || shell.eq_ignore_ascii_case("CMD")
            || shell.eq_ignore_ascii_case("Command Prompt")
    }) {
        return None;
    }
    let user = sanitize_label(
        session
            .shell_user
            .as_deref()
            .filter(|value| !value.trim().is_empty())?,
    );
    let linux_cwd = session.cwd.as_ref()?.to_string_lossy().into_owned();
    if !linux_cwd.starts_with('/') {
        return None;
    }

    // Do not touch WSL UNC provider paths from discovery. Even a direct UNC
    // lookup can inherit unbounded provider/filesystem latency and stall the
    // single bounded extension worker. Shell-published distro/version metadata
    // provides the visible OS identity, /mnt/<drive> paths map directly to the
    // host filesystem for Git/project discovery. Unmapped guest locations and
    // unavailable provider context never fall back to the host's identity.
    let distro_label = session
        .os_version
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            session
                .distro
                .as_ref()
                .filter(|value| !value.trim().is_empty())
        })
        .cloned()
        .unwrap_or_else(|| distro.clone());

    let cwd = windows_path_from_wsl_mount(&linux_cwd);

    Some(SessionView {
        cwd,
        home: None,
        wsl: Some(WslContext {
            distro: distro_label,
            user,
        }),
        use_process_env: false,
    })
}

#[cfg(target_os = "windows")]
fn windows_path_from_wsl_mount(linux_path: &str) -> Option<PathBuf> {
    let rest = linux_path.strip_prefix("/mnt/")?;
    let mut parts = rest.split('/');
    let drive = parts.next()?;
    if drive.len() != 1 || !drive.as_bytes()[0].is_ascii_alphabetic() {
        return None;
    }
    let letter = (drive.as_bytes()[0] as char).to_ascii_uppercase();
    let mut out = PathBuf::from(format!("{letter}:\\"));
    for part in parts.filter(|part| !part.is_empty() && *part != ".") {
        if part == ".."
            || part.contains(['\\', ':'])
            || part.chars().any(char::is_control)
        {
            return None;
        }
        out.push(part);
    }
    Some(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn inherited_environment() -> Option<String> {
    ["AUTOMEXIA_ENV", "ENVIRONMENT", "APP_ENV", "NODE_ENV"]
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
}

#[cfg(not(target_arch = "wasm32"))]
fn kubernetes_context(
    session: &SessionFacts,
    home: Option<&Path>,
) -> Option<KubernetesContext> {
    let paths = crate::kubernetes::local_paths(session, home);
    crate::kubernetes::from_files(&paths)
}

// Provider overrides follow the shell's working directory, never the GUI's
// launch directory. Reuse the local-only path authority shared with kubeconfig.
#[cfg(not(target_arch = "wasm32"))]
fn provider_path(path: PathBuf, cwd: Option<&Path>) -> Option<PathBuf> {
    if path.as_os_str().is_empty() {
        return None;
    }
    let resolved = if path.is_absolute() {
        path
    } else {
        // Reject Windows drive-relative and rooted-but-drive-less paths: their
        // meaning depends on hidden process state instead of the session cwd.
        if path.has_root()
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::Prefix(_)))
        {
            return None;
        }
        cwd?.join(path)
    };
    crate::kubernetes::bounded_local_path(&resolved).then_some(resolved)
}

#[cfg(not(target_arch = "wasm32"))]
fn docker_config_root(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<PathBuf> {
    let path = use_process_env
        .then(|| env::var_os("DOCKER_CONFIG"))
        .flatten()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".docker")))?;
    provider_path(path, cwd)
}

#[cfg(not(target_arch = "wasm32"))]
fn docker_context(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<String> {
    if use_process_env {
        if let Ok(value) = env::var("DOCKER_CONTEXT") {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
        // The endpoint selects Docker's virtual default context. Its value is
        // neither a display label nor an instruction to contact the daemon.
        if env::var_os("DOCKER_HOST").is_some_and(|value| !value.is_empty()) {
            return Some("default".to_string());
        }
    }

    let root = docker_config_root(home, cwd, use_process_env)?;
    let config = root.join("config.json");
    if let Some(value) = read_json(&config) {
        return value
            .get("currentContext")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| Some("default".to_string()));
    }

    // A local context store is evidence of a configured client, not daemon
    // reachability. Do not probe Docker Desktop or a provider endpoint.
    root.is_dir().then(|| "default".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn cloud_contexts(
    home: Option<&Path>,
    cwd: Option<&Path>,
    project_context: Option<&Value>,
    legacy_context: Option<&Value>,
    use_process_env: bool,
) -> Vec<CloudContext> {
    let mut contexts: Vec<CloudContext> = [
        aws_context(home, cwd, use_process_env),
        azure_context(home, cwd, use_process_env),
        gcp_context(home, cwd, use_process_env),
    ]
    .into_iter()
    .flatten()
    .collect();

    if let Some(context) = project_context.or(legacy_context) {
        if let Some(selected) = cloud_from_automexia_json(context) {
            contexts.retain(|candidate| candidate.provider != selected.provider);
            contexts.insert(0, selected);
        }
    }
    contexts
}

#[cfg(not(target_arch = "wasm32"))]
fn aws_config_path(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<PathBuf> {
    let path = use_process_env
        .then(|| env::var_os("AWS_CONFIG_FILE"))
        .flatten()
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".aws").join("config")))?;
    provider_path(path, cwd)
}

#[cfg(not(target_arch = "wasm32"))]
fn aws_context(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<CloudContext> {
    let explicit_profile = use_process_env
        .then(|| {
            env::var("AWS_PROFILE")
                .ok()
                .or_else(|| env::var("AWS_DEFAULT_PROFILE").ok())
        })
        .flatten()
        .filter(|value| !value.trim().is_empty());
    let path = aws_config_path(home, cwd, use_process_env);
    if explicit_profile.is_none() && path.as_ref().is_none_or(|path| !path.is_file()) {
        return None;
    }

    let profile = explicit_profile.unwrap_or_else(|| "default".to_string());
    let region = use_process_env
        .then(|| {
            env::var("AWS_REGION")
                .ok()
                .or_else(|| env::var("AWS_DEFAULT_REGION").ok())
        })
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| path.as_deref().and_then(|path| aws_region(path, &profile)))
        .unwrap_or_default();

    Some(CloudContext {
        provider: "AWS",
        profile: sanitize_label(&profile),
        region: sanitize_label(&region),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn azure_config_root(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<PathBuf> {
    let path = use_process_env
        .then(|| env::var_os("AZURE_CONFIG_DIR"))
        .flatten()
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".azure")))?;
    provider_path(path, cwd)
}

#[cfg(not(target_arch = "wasm32"))]
fn azure_context(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<CloudContext> {
    let root = azure_config_root(home, cwd, use_process_env)?;
    let cloud = use_process_env
        .then(|| env::var("AZURE_CLOUD_NAME").ok())
        .flatten()
        .or_else(|| ini_value(&root.join("config"), "cloud", "name"))
        .unwrap_or_else(|| "AzureCloud".to_string());
    if cloud.is_empty() || cloud.len() > 512 || cloud.chars().any(char::is_control) {
        return None;
    }
    let text = read_small_text(&root.join("azureProfile.json"))?;
    // Azure CLI writes this public cache as UTF-8 with an optional BOM.
    let value: Value =
        serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text)).ok()?;
    let subscriptions = value.get("subscriptions")?.as_array()?;
    let mut defaults = subscriptions.iter().filter(|item| {
        item.get("isDefault").and_then(Value::as_bool) == Some(true)
            && item
                .get("environmentName")
                .and_then(Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(&cloud))
    });
    let selected = defaults.next()?;
    // Azure CLI has no selected subscription when zero or multiple cached
    // defaults match the active cloud. Never invent one from array order or
    // unrelated SDK/deployment environment variables.
    if defaults.next().is_some() {
        return None;
    }
    let profile = ["displayName", "name", "id"].into_iter().find_map(|key| {
        selected
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
    })?;
    Some(CloudContext {
        provider: "Azure",
        profile: sanitize_label(profile),
        region: String::new(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn gcloud_config_root(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<PathBuf> {
    if use_process_env {
        if let Some(root) = env::var_os("CLOUDSDK_CONFIG") {
            return provider_path(PathBuf::from(root), cwd);
        }
        #[cfg(target_os = "windows")]
        if let Some(appdata) = env::var_os("APPDATA") {
            if appdata.is_empty() {
                return None;
            }
            return provider_path(PathBuf::from(appdata).join("gcloud"), cwd);
        }
    }
    provider_path(home?.join(".config").join("gcloud"), cwd)
}

#[cfg(not(target_arch = "wasm32"))]
fn valid_gcloud_configuration(name: &str) -> bool {
    name.len() <= 512
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        })
}

#[cfg(not(target_arch = "wasm32"))]
fn gcp_context(
    home: Option<&Path>,
    cwd: Option<&Path>,
    use_process_env: bool,
) -> Option<CloudContext> {
    let root = gcloud_config_root(home, cwd, use_process_env)?;
    let override_name = if use_process_env {
        match env::var("CLOUDSDK_ACTIVE_CONFIG_NAME") {
            Ok(name) => Some(name),
            Err(env::VarError::NotPresent) => None,
            Err(env::VarError::NotUnicode(_)) => return None,
        }
    } else {
        None
    };
    let active = if let Some(name) = &override_name {
        name.clone()
    } else {
        let path = root.join("active_config");
        match read_small_text(&path) {
            Some(value) => {
                let name = value.trim();
                if name.is_empty() {
                    "default".to_string()
                } else {
                    name.to_string()
                }
            }
            None if fs::metadata(path)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
            {
                "default".to_string()
            }
            None => return None,
        }
    };
    if active != "NONE" && !valid_gcloud_configuration(&active) {
        return None;
    }
    let content = if active == "NONE" {
        None
    } else {
        let path = provider_path(
            root.join("configurations").join(format!("config_{active}")),
            cwd,
        )?;
        read_small_text(&path)
    };
    // An explicitly named configuration must exist. NONE is gcloud's documented
    // no-file configuration and still allows independent property overrides.
    if override_name.is_some() && active != "NONE" && content.is_none() {
        return None;
    }
    let project = use_process_env
        .then(|| env::var("CLOUDSDK_CORE_PROJECT").ok())
        .flatten()
        .or_else(|| {
            content
                .as_deref()
                .and_then(|text| ini_value_from_text(text, "core", "project"))
        })
        .or_else(|| content.as_ref().map(|_| active.clone()))?;
    if project.trim().is_empty() {
        return None;
    }
    let region = use_process_env
        .then(|| env::var("CLOUDSDK_COMPUTE_REGION").ok())
        .flatten()
        .or_else(|| {
            content
                .as_deref()
                .and_then(|text| ini_value_from_text(text, "compute", "region"))
        })
        .unwrap_or_default();
    Some(CloudContext {
        provider: "GCP",
        profile: sanitize_label(&project),
        region: sanitize_label(&region),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn valid_terraform_workspace(name: &str) -> bool {
    // Terraform requires a nonempty name unchanged by Go's url.PathEscape.
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._~$&+:=@".contains(&byte))
}

#[cfg(not(target_arch = "wasm32"))]
fn terraform_workspace(cwd: Option<&Path>, use_process_env: bool) -> Option<String> {
    if use_process_env {
        if let Ok(value) = env::var("TF_WORKSPACE") {
            if !value.is_empty() {
                return valid_terraform_workspace(&value).then_some(value);
            }
        }
    }
    let selected = use_process_env
        .then(|| env::var_os("TF_DATA_DIR"))
        .flatten()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| cwd.map(|cwd| cwd.join(".terraform")))?;
    let root = provider_path(selected, cwd)?;
    // A selected initialized data directory is concrete local project evidence.
    // Do not show a Terraform badge in every unrelated working directory.
    if !root.is_dir() {
        return None;
    }
    let path = root.join("environment");
    match read_small_text(&path) {
        Some(value) => {
            let name = value.trim();
            if name.is_empty() {
                Some("default".to_string())
            } else {
                valid_terraform_workspace(name).then(|| name.to_string())
            }
        }
        None if fs::metadata(path)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
        {
            Some("default".to_string())
        }
        None => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn git_branch(cwd: Option<&Path>) -> Option<String> {
    let mut cursor = cwd?.to_path_buf();
    loop {
        let dot_git = cursor.join(".git");
        let head_path = if dot_git.is_dir() {
            dot_git.join("HEAD")
        } else if dot_git.is_file() {
            let pointer = read_small_text(&dot_git)?;
            let git_dir = pointer.trim().strip_prefix("gitdir:")?.trim();
            let git_dir = PathBuf::from(git_dir);
            if git_dir.is_absolute() {
                git_dir.join("HEAD")
            } else {
                cursor.join(git_dir).join("HEAD")
            }
        } else {
            if !cursor.pop() {
                return None;
            }
            continue;
        };

        let head = read_small_text(&head_path)?;
        let head = head.trim();
        if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
            return Some(branch.to_string());
        }
        if !head.is_empty() {
            return Some(head.chars().take(8).collect());
        }
        return None;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn project_automexia_context(cwd: Option<&Path>) -> Option<Value> {
    let mut cursor = cwd?.to_path_buf();
    loop {
        let candidate = cursor.join(".automexia-context.json");
        if candidate.is_file() {
            return read_json(&candidate);
        }
        if !cursor.pop() {
            return None;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn legacy_automexia_context(home: Option<&Path>, use_process_env: bool) -> Option<Value> {
    let root = use_process_env
        .then(|| {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("APPDATA").map(PathBuf::from))
        })
        .flatten()
        .or_else(|| home.map(|home| home.join(".config")))?;
    read_json(&root.join("automexia").join("context.json"))
}

#[cfg(not(target_arch = "wasm32"))]
fn kubernetes_from_automexia_json(value: &Value) -> Option<KubernetesContext> {
    let nested = value
        .get("kubernetes")
        .or_else(|| value.get("kube"))
        .unwrap_or(value);
    let context = json_string(
        nested,
        &["kubernetesContext", "kubernetes_context", "context"],
    )?;
    let namespace =
        json_string(nested, &["namespace"]).unwrap_or_else(|| "default".to_string());
    Some(KubernetesContext {
        context: sanitize_label(&context),
        namespace: sanitize_label(&namespace),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn docker_from_automexia_json(value: &Value) -> Option<String> {
    let nested = value.get("docker").unwrap_or(value);
    json_string(nested, &["dockerContext", "docker_context", "context"])
}

#[cfg(not(target_arch = "wasm32"))]
fn terraform_from_automexia_json(value: &Value) -> Option<String> {
    let nested = value.get("terraform").unwrap_or(value);
    json_string(
        nested,
        &["terraformWorkspace", "terraform_workspace", "workspace"],
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn git_from_automexia_json(value: &Value) -> Option<String> {
    let nested = value.get("git").unwrap_or(value);
    json_string(nested, &["branch", "gitBranch", "git_branch"])
}

#[cfg(not(target_arch = "wasm32"))]
fn environment_from_automexia_json(value: &Value) -> Option<String> {
    json_string(value, &["environment", "env"])
}

#[cfg(not(target_arch = "wasm32"))]
fn project_from_automexia_json(value: &Value) -> Option<String> {
    json_string(value, &["project", "projectName", "project_name"])
}

#[cfg(not(target_arch = "wasm32"))]
fn cloud_from_automexia_json(value: &Value) -> Option<CloudContext> {
    let nested = value.get("cloud").unwrap_or(value);
    let provider_raw =
        json_string(nested, &["cloudProvider", "cloud_provider", "provider"])?;
    let profile = json_string(
        nested,
        &["cloudProfile", "cloud_profile", "profile", "account"],
    )
    .unwrap_or_default();
    let region = json_string(nested, &["cloudRegion", "cloud_region", "region"])
        .unwrap_or_default();
    if provider_raw.is_empty() && profile.is_empty() {
        return None;
    }
    let provider = match provider_raw.to_ascii_lowercase().as_str() {
        "aws" => "AWS",
        "azure" => "Azure",
        "gcp" | "google" => "GCP",
        _ => "Cloud",
    };
    Some(CloudContext {
        provider,
        profile: sanitize_label(&profile),
        region: sanitize_label(&region),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn json_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(result) = value.get(*key).and_then(Value::as_str) {
            if !result.trim().is_empty() {
                return Some(result.to_string());
            }
        }
        if let Some(result) = value
            .get("snapshot")
            .and_then(|snapshot| snapshot.get(*key))
            .and_then(Value::as_str)
        {
            if !result.trim().is_empty() {
                return Some(result.to_string());
            }
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn aws_region(path: &Path, profile: &str) -> Option<String> {
    let section = if profile == "default" {
        "default".to_string()
    } else {
        format!("profile {profile}")
    };
    ini_value(path, &section, "region")
}

#[cfg(not(target_arch = "wasm32"))]
fn ini_value(path: &Path, wanted_section: &str, wanted_key: &str) -> Option<String> {
    let content = read_small_text(path)?;
    ini_value_from_text(&content, wanted_section, wanted_key)
}

#[cfg(not(target_arch = "wasm32"))]
fn ini_value_from_text(
    content: &str,
    wanted_section: &str,
    wanted_key: &str,
) -> Option<String> {
    let mut section = String::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        if section != wanted_section || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == wanted_key {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn read_json(path: &Path) -> Option<Value> {
    let text = read_small_text(path)?;
    serde_json::from_str(&text).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_small_text(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_CONFIG_BYTES {
        return None;
    }

    let file = fs::File::open(path).ok()?;
    let mut reader = file.take(MAX_CONFIG_BYTES + 1);
    let mut text = String::with_capacity(metadata.len() as usize);
    reader.read_to_string(&mut text).ok()?;
    (text.len() as u64 <= MAX_CONFIG_BYTES).then_some(text)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sanitize_label(input: &str) -> String {
    let mut out = String::with_capacity(input.len().min(MAX_LABEL_CHARS));
    let mut count = 0usize;
    for character in input.chars() {
        if count >= MAX_LABEL_CHARS {
            break;
        }
        if character.is_control()
            || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            continue;
        }
        out.push(character);
        count += 1;
    }
    out.trim().to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn looks_production(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| {
            matches!(
                token.to_ascii_lowercase().as_str(),
                "prod" | "production" | "live"
            )
        })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::kubernetes::namespace_for_context as kube_namespace_for_context;

    #[test]
    fn provider_paths_require_bounded_local_session_authority() {
        let temporary = tempfile::tempdir().unwrap();
        let cwd = temporary.path();
        assert_eq!(
            provider_path("relative".into(), Some(cwd)),
            Some(cwd.join("relative"))
        );
        assert_eq!(provider_path("relative".into(), None), None);
        for value in ["", "bad\npath", &"x".repeat(4097)] {
            assert_eq!(provider_path(value.into(), Some(cwd)), None);
        }
        #[cfg(windows)]
        for value in [
            r"\\fixture.invalid\share\config",
            r"\\?\UNC\fixture.invalid\share\config",
            r"\\.\pipe\fixture",
            r"C:relative",
            r"\rooted",
        ] {
            assert_eq!(provider_path(value.into(), Some(cwd)), None);
        }
        #[cfg(not(windows))]
        assert_eq!(
            provider_path("//fixture.invalid/config".into(), Some(cwd)),
            None
        );
    }

    #[test]
    fn kube_namespace_is_independent_of_yaml_mapping_order() {
        // Both forms are valid kubeconfig; hand-written files often put name first.
        for document in [
            "contexts:\n- context:\n    namespace: sandbox\n  name: fixture\n",
            "contexts:\n- name: fixture\n  context:\n    namespace: sandbox\n",
            "contexts: [{name: fixture, context: {namespace: sandbox}}]\n",
            r#"{"contexts":[{"name":"fixture","context":{"namespace":"sandbox"}}]}"#,
        ] {
            assert_eq!(
                kube_namespace_for_context(document, "fixture").as_deref(),
                Some("sandbox"),
            );
        }
    }

    #[test]
    fn kube_namespace_never_borrows_fields_from_another_section() {
        let document = "contexts:\n- context: {}\n  name: fixture\nusers:\n- name: fixture\n  user:\n    namespace: not-a-context\n";
        assert_eq!(
            kube_namespace_for_context(document, "fixture").as_deref(),
            Some("default"),
        );
        assert_eq!(kube_namespace_for_context(document, "absent"), None);
    }

    #[test]
    fn kube_namespace_decodes_quoted_labels_without_yaml_comments() {
        let document = "contexts:\n- context:\n    namespace: \"sandbox\" # local selection\n  name: 'fixture'\n";
        assert_eq!(
            kube_namespace_for_context(document, "fixture").as_deref(),
            Some("sandbox"),
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn wsl_view_uses_explicit_metadata_instead_of_mutable_title_text() {
        let session = SessionFacts {
            session_id: 1,
            cwd: Some(PathBuf::from("/mnt/d/work tree")),
            title: "misleading native title D:/ignored".to_string(),
            distro: Some("Ubuntu-24.04".to_string()),
            os_version: Some("24.04".to_string()),
            shell_name: Some("bash".to_string()),
            shell_user: Some("alice".to_string()),
            shell_path: Some("/usr/bin/bash".to_string()),
            shell_integration: true,
            shell_pid: 42,
            environment: Default::default(),
        };
        let view = windows_wsl_session_view(&session).expect("explicit WSL view");
        assert_eq!(view.cwd.as_deref(), Some(Path::new(r"D:\work tree")));
        assert_eq!(view.wsl.expect("WSL identity").user, "alice");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn native_powershell_drive_title_is_not_wsl() {
        let title = "alice@devbox: D:/projects/automexia";
        let session = SessionFacts {
            session_id: 1,
            cwd: Some(PathBuf::from(r"D:\projects\automexia")),
            title: title.to_string(),
            // A nested WSL process may have left these terminal-scoped user
            // variables behind. The native drive title must still win.
            distro: Some("Ubuntu-24.04".to_string()),
            os_version: Some("24.04".to_string()),
            shell_name: Some("PowerShell".to_string()),
            shell_user: Some("alice".to_string()),
            shell_path: Some(
                r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe".to_string(),
            ),
            shell_integration: true,
            shell_pid: 42,
            environment: Default::default(),
        };
        assert!(windows_wsl_session_view(&session).is_none());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn native_command_prompt_never_inherits_a_stale_wsl_badge() {
        let session = SessionFacts {
            session_id: 1,
            cwd: Some(PathBuf::from(r"D:\projects\automexia")),
            title: "CMD - D:/fixture/automexia".to_string(),
            distro: Some("Ubuntu-24.04".to_string()),
            os_version: Some("24.04".to_string()),
            shell_name: Some("CMD".to_string()),
            shell_user: Some("alice".to_string()),
            shell_path: Some(r"C:\Windows\System32\cmd.exe".to_string()),
            shell_integration: true,
            shell_pid: 42,
            environment: Default::default(),
        };
        assert!(windows_wsl_session_view(&session).is_none());
    }

    #[test]
    fn passive_status_discovery_has_no_process_shell_or_provider_launch_authority() {
        let source = include_str!("context.rs")
            .split("#[cfg(all(test")
            .next()
            .expect("production source prefix");
        for forbidden in [
            "std::process",
            "Command::new",
            ".spawn(",
            "WSL_CONTEXT_PROBE_SCRIPT",
            "wsl_live_contexts",
            "--exec\", \"sh\", \"-c",
        ] {
            assert!(
                !source.contains(forbidden),
                "passive status discovery regained launch authority: {forbidden}"
            );
        }
    }
    #[test]
    fn existing_docker_config_root_implies_default_context() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home = std::env::temp_dir().join(format!(
            "automexia-devops-docker-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(home.join(".docker")).unwrap();
        assert_eq!(
            docker_context(Some(&home), None, false).as_deref(),
            Some("default")
        );
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn production_detection_uses_real_tokens() {
        assert!(looks_production("prod"));
        assert!(looks_production("cluster/prod-eu"));
        assert!(looks_production("payments_production"));
        assert!(looks_production("live.westeurope"));
        assert!(!looks_production("product-catalog"));
        assert!(!looks_production("productive"));
        assert!(!looks_production("staging"));
    }

    #[test]
    fn nested_project_context_fields_are_supported() {
        let value: Value = serde_json::from_str(
            r#"{
                "project":"payments",
                "environment":"staging",
                "git":{"branch":"main"},
                "kubernetes":{"context":"aks-staging","namespace":"payments"},
                "docker":{"context":"desktop-linux"},
                "cloud":{"provider":"azure","profile":"Development Subscription","region":"westeurope"},
                "terraform":{"workspace":"staging"}
            }"#,
        )
        .unwrap();
        assert_eq!(
            docker_from_automexia_json(&value).as_deref(),
            Some("desktop-linux")
        );
        assert_eq!(
            terraform_from_automexia_json(&value).as_deref(),
            Some("staging")
        );
        assert_eq!(git_from_automexia_json(&value).as_deref(), Some("main"));
        assert_eq!(
            project_from_automexia_json(&value).as_deref(),
            Some("payments")
        );
        assert_eq!(
            environment_from_automexia_json(&value).as_deref(),
            Some("staging")
        );
        assert_eq!(
            kubernetes_from_automexia_json(&value).unwrap().namespace,
            "payments"
        );
        assert_eq!(cloud_from_automexia_json(&value).unwrap().provider, "Azure");
    }

    #[test]
    fn label_sanitizer_strips_controls_and_limits_length() {
        assert_eq!(sanitize_label("  dev\nops\t  "), "devops");
        assert_eq!(sanitize_label("safe\u{202e}label\u{2069}"), "safelabel");
        let long = "x".repeat(MAX_LABEL_CHARS + 20);
        assert_eq!(sanitize_label(&long).chars().count(), MAX_LABEL_CHARS);
    }
}
