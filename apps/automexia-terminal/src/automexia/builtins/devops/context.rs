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
use crate::automexia::api::SessionFacts;

const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_KUBECONFIG_FILES: usize = 16;
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
    let cwd = view.cwd.as_deref().or(session.cwd.as_deref());
    let home = view.home.as_deref().or(host_home.as_deref());
    let project_context = project_automexia_context(cwd);
    let legacy_context = legacy_automexia_context(home, view.use_process_env);

    let kubernetes = kubernetes_context(home, view.use_process_env)
        .or_else(|| {
            // If WSL has no kubeconfig, host-side configuration is still useful
            // (for example Docker Desktop's kubectl integration).
            if view.wsl.is_some() {
                kubernetes_context(host_home.as_deref(), true)
            } else {
                None
            }
        })
        .or_else(|| {
            project_context
                .as_ref()
                .and_then(kubernetes_from_automexia_json)
                .or_else(|| {
                    legacy_context
                        .as_ref()
                        .and_then(kubernetes_from_automexia_json)
                })
        });

    let docker = docker_context(home, view.use_process_env)
        .or_else(|| {
            if view.wsl.is_some() {
                docker_context(host_home.as_deref(), true)
            } else {
                None
            }
        })
        .or_else(|| {
            project_context
                .as_ref()
                .and_then(docker_from_automexia_json)
        })
        .or_else(|| legacy_context.as_ref().and_then(docker_from_automexia_json));

    let mut clouds = cloud_contexts(
        home,
        project_context.as_ref(),
        legacy_context.as_ref(),
        view.use_process_env,
    );
    if view.wsl.is_some() {
        for host_cloud in cloud_contexts(host_home.as_deref(), None, None, true) {
            if !clouds
                .iter()
                .any(|candidate| candidate.provider == host_cloud.provider)
            {
                clouds.push(host_cloud);
            }
        }
    }

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

    let user = view
        .wsl
        .as_ref()
        .map(|context| context.user.clone())
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
        user,
        environment: environment.map(|value| sanitize_label(&value)),
        production,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn session_view(session: &SessionFacts, host_home: Option<&Path>) -> SessionView {
    #[cfg(target_os = "windows")]
    if let Some(view) = windows_wsl_session_view(session) {
        return view;
    }

    SessionView {
        cwd: session.cwd.clone(),
        home: host_home.map(Path::to_path_buf),
        wsl: None,
        use_process_env: true,
    }
}

/// Parse the common WSL interactive title shape emitted by bash/zsh, e.g.
/// `amjed@DESKTOP-2LR87FN:/mnt/d/work` or `amjed@host: /mnt/d/work`.
/// This observes existing terminal metadata only; Automexia injects no shell
/// command and requires no prompt/plugin installation.
#[cfg(target_os = "windows")]
fn parse_wsl_title(title: &str) -> Option<(String, String)> {
    let title = title.trim();
    let (prefix, path) = if let Some(index) = title.find(": /") {
        (&title[..index], &title[index + 2..])
    } else if let Some(index) = title.find(":/") {
        (&title[..index], &title[index + 1..])
    } else {
        return None;
    };
    let (user, host) = prefix.rsplit_once('@')?;
    let user = user.split_whitespace().last()?.trim();
    if user.is_empty() || host.trim().is_empty() || !path.starts_with('/') {
        return None;
    }
    Some((sanitize_label(user), path.trim().to_string()))
}

#[cfg(target_os = "windows")]
fn windows_wsl_session_view(session: &SessionFacts) -> Option<SessionView> {
    let (user, linux_cwd) = parse_wsl_title(&session.title)?;

    // Do not touch WSL UNC provider paths from discovery. Even a direct UNC
    // lookup can inherit unbounded provider/filesystem latency and stall the
    // single bounded extension worker. Shell-published distro/version metadata
    // provides the visible OS identity, /mnt/<drive> paths map directly to the
    // host filesystem for Git/project discovery, and host Docker/Kubernetes/
    // cloud configuration remains the safe fallback.
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
        .unwrap_or_else(|| "WSL".to_string());

    let cwd = windows_path_from_wsl_mount(&linux_cwd).or_else(|| session.cwd.clone());

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
        if part == ".." {
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
    home: Option<&Path>,
    use_process_env: bool,
) -> Option<KubernetesContext> {
    let paths: Vec<PathBuf> =
        match use_process_env.then(|| env::var_os("KUBECONFIG")).flatten() {
            Some(value) => env::split_paths(&value)
                .take(MAX_KUBECONFIG_FILES)
                .collect(),
            None => home
                .map(|home| vec![home.join(".kube").join("config")])
                .unwrap_or_default(),
        };
    if paths.is_empty() {
        return None;
    }

    let documents: Vec<String> = paths
        .iter()
        .filter_map(|path| read_small_text(path))
        .collect();
    let current = documents
        .iter()
        .find_map(|content| yaml_scalar(content, "current-context"))?;
    let namespace = documents
        .iter()
        .find_map(|content| kube_namespace_for_context(content, &current))
        .unwrap_or_else(|| "default".to_string());

    Some(KubernetesContext {
        context: sanitize_label(&current),
        namespace: sanitize_label(&namespace),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn kube_namespace_for_context(content: &str, wanted: &str) -> Option<String> {
    // Kubeconfig context items look like:
    // - context:
    //     namespace: ns
    //   name: ctx
    // We keep the parser deliberately read-only and narrow rather than pulling
    // a YAML dependency into the terminal frontend.
    let mut in_item = false;
    let mut name: Option<String> = None;
    let mut namespace: Option<String> = None;

    let flush =
        |name: &mut Option<String>, namespace: &mut Option<String>| -> Option<String> {
            if name.as_deref() == Some(wanted) {
                return namespace.take();
            }
            *name = None;
            *namespace = None;
            None
        };

    for raw in content.lines() {
        let line = raw.trim();
        if line == "- context:" || line.starts_with("- context:") {
            if in_item {
                if let Some(found) = flush(&mut name, &mut namespace) {
                    return Some(found);
                }
            }
            in_item = true;
            name = None;
            namespace = None;
            continue;
        }
        if !in_item {
            continue;
        }
        if let Some(value) = line.strip_prefix("namespace:") {
            namespace = Some(unquote(value));
        } else if let Some(value) = line.strip_prefix("name:") {
            name = Some(unquote(value));
        } else if line.starts_with("- ") && !line.starts_with("- context:") {
            if let Some(found) = flush(&mut name, &mut namespace) {
                return Some(found);
            }
            in_item = false;
        }
    }
    flush(&mut name, &mut namespace)
}

#[cfg(not(target_arch = "wasm32"))]
fn docker_config_root(home: Option<&Path>, use_process_env: bool) -> Option<PathBuf> {
    use_process_env
        .then(|| env::var_os("DOCKER_CONFIG"))
        .flatten()
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".docker")))
}

#[cfg(not(target_arch = "wasm32"))]
fn docker_context(home: Option<&Path>, use_process_env: bool) -> Option<String> {
    if use_process_env {
        if let Ok(value) = env::var("DOCKER_CONTEXT") {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }

    let root = docker_config_root(home, use_process_env)?;
    let config = root.join("config.json");
    if let Some(value) = read_json(&config) {
        return value
            .get("currentContext")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| Some("default".to_string()));
    }

    // A Docker context store with no explicit currentContext still means the
    // client is using its default context. This is local filesystem detection;
    // it does not contact Docker Desktop or the daemon.
    root.is_dir().then(|| "default".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn cloud_contexts(
    home: Option<&Path>,
    project_context: Option<&Value>,
    legacy_context: Option<&Value>,
    use_process_env: bool,
) -> Vec<CloudContext> {
    let mut contexts: Vec<CloudContext> = [
        aws_context(home, use_process_env),
        azure_context(home, use_process_env),
        gcp_context(home, use_process_env),
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
fn aws_config_path(home: Option<&Path>, use_process_env: bool) -> Option<PathBuf> {
    use_process_env
        .then(|| env::var_os("AWS_CONFIG_FILE"))
        .flatten()
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".aws").join("config")))
}

#[cfg(not(target_arch = "wasm32"))]
fn aws_context(home: Option<&Path>, use_process_env: bool) -> Option<CloudContext> {
    let explicit_profile = use_process_env
        .then(|| {
            env::var("AWS_PROFILE")
                .ok()
                .or_else(|| env::var("AWS_DEFAULT_PROFILE").ok())
        })
        .flatten()
        .filter(|value| !value.trim().is_empty());
    let path = aws_config_path(home, use_process_env);
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
fn azure_config_root(home: Option<&Path>, use_process_env: bool) -> Option<PathBuf> {
    use_process_env
        .then(|| env::var_os("AZURE_CONFIG_DIR"))
        .flatten()
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".azure")))
}

#[cfg(not(target_arch = "wasm32"))]
fn azure_context(home: Option<&Path>, use_process_env: bool) -> Option<CloudContext> {
    if let Some(subscription) = use_process_env
        .then(|| {
            env::var("AZURE_SUBSCRIPTION")
                .ok()
                .or_else(|| env::var("AZURE_SUBSCRIPTION_ID").ok())
        })
        .flatten()
        .filter(|value| !value.trim().is_empty())
    {
        return Some(CloudContext {
            provider: "Azure",
            profile: sanitize_label(&subscription),
            region: String::new(),
        });
    }

    let root = azure_config_root(home, use_process_env)?;
    let value = read_json(&root.join("azureProfile.json"))?;
    let subscriptions = value.get("subscriptions")?.as_array()?;
    let selected = subscriptions
        .iter()
        .find(|item| item.get("isDefault").and_then(Value::as_bool) == Some(true))
        .or_else(|| subscriptions.first())?;
    let profile = ["displayName", "name", "id"]
        .into_iter()
        .find_map(|key| selected.get(key).and_then(Value::as_str))?;
    Some(CloudContext {
        provider: "Azure",
        profile: sanitize_label(profile),
        region: String::new(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn gcloud_config_root(home: Option<&Path>, use_process_env: bool) -> Option<PathBuf> {
    if use_process_env {
        if let Some(root) = env::var_os("CLOUDSDK_CONFIG") {
            return Some(PathBuf::from(root));
        }
        #[cfg(target_os = "windows")]
        if let Some(appdata) = env::var_os("APPDATA") {
            return Some(PathBuf::from(appdata).join("gcloud"));
        }
    }
    home.map(|home| home.join(".config").join("gcloud"))
}

#[cfg(not(target_arch = "wasm32"))]
fn gcp_context(home: Option<&Path>, use_process_env: bool) -> Option<CloudContext> {
    if let Some(project) = use_process_env
        .then(|| {
            env::var("GOOGLE_CLOUD_PROJECT")
                .ok()
                .or_else(|| env::var("CLOUDSDK_CORE_PROJECT").ok())
        })
        .flatten()
        .filter(|value| !value.trim().is_empty())
    {
        return Some(CloudContext {
            provider: "GCP",
            profile: sanitize_label(&project),
            region: use_process_env
                .then(|| env::var("CLOUDSDK_COMPUTE_REGION").ok())
                .flatten()
                .map(|value| sanitize_label(&value))
                .unwrap_or_default(),
        });
    }

    let root = gcloud_config_root(home, use_process_env)?;
    let active = read_small_text(&root.join("active_config"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default".to_string());
    let config = root.join("configurations").join(format!("config_{active}"));
    if !config.is_file() {
        return None;
    }
    let project = ini_value(&config, "core", "project").unwrap_or(active);
    let region = ini_value(&config, "compute", "region").unwrap_or_default();
    Some(CloudContext {
        provider: "GCP",
        profile: sanitize_label(&project),
        region: sanitize_label(&region),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn terraform_workspace(cwd: Option<&Path>, use_process_env: bool) -> Option<String> {
    if use_process_env {
        if let Ok(value) = env::var("TF_WORKSPACE") {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }
    read_small_text(&cwd?.join(".terraform").join("environment"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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
fn yaml_scalar(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|raw| {
        let line = raw.trim();
        let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
        let value = unquote(value);
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
        .to_string()
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
        if character.is_control() {
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

    #[cfg(target_os = "windows")]
    #[test]
    fn wsl_title_parser_accepts_common_bash_shapes() {
        assert_eq!(
            parse_wsl_title("amjed@DESKTOP-2LR87FN:/mnt/d/work"),
            Some(("amjed".to_string(), "/mnt/d/work".to_string()))
        );
        assert_eq!(
            parse_wsl_title("amjed@DESKTOP-2LR87FN: /home/amjed/project"),
            Some(("amjed".to_string(), "/home/amjed/project".to_string()))
        );
        assert_eq!(parse_wsl_title("Windows PowerShell"), None);
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
            docker_context(Some(&home), false).as_deref(),
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
        let long = "x".repeat(MAX_LABEL_CHARS + 20);
        assert_eq!(sanitize_label(&long).chars().count(), MAX_LABEL_CHARS);
    }
}
