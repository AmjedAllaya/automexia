//! Bounded presentation hints for kubectl/oc table columns. No API parsing,
//! provider access, or health inference from resource names lives here.
use super::{classify_row, OutputClassification, OutputDomain, MAX_ROW_BYTES};
use automexia_extension_api::SemanticSeverity as Severity;

const MAX_COLUMNS: usize = 32;

#[derive(Clone, Copy, Default)]
enum Column {
    #[default]
    Other,
    Name,
    Namespace,
    Ready,
    Status,
    Desired,
    Current,
    Updated,
    Available,
    Completions,
    Suspend,
    Type,
    ExternalIp,
    Targets,
    Condition,
}

#[derive(Clone, Copy)]
enum Kind {
    Pod,
    Ready,
    Replicas,
    Job,
    CronJob,
    Node,
    Storage,
    Namespace,
    Service,
    Event,
    Autoscaler,
    Certificate,
    ApiService,
    Neutral,
}

/// One contiguous output table in one pane snapshot. Prompts, malformed rows,
/// blank lines and unrelated headers end ownership; nothing is shared globally.
#[derive(Default)]
pub struct RowClassifier {
    schema: Option<Schema>,
}

#[derive(Clone, Copy)]
struct Schema {
    columns: [Column; MAX_COLUMNS],
    starts: [usize; MAX_COLUMNS],
    aligned: bool,
    len: usize,
    kind: Kind,
}

impl RowClassifier {
    pub fn reset(&mut self) {
        self.schema = None;
    }

    pub fn classify(&mut self, text: &str) -> Option<OutputClassification> {
        if text.len() > MAX_ROW_BYTES || text.trim().is_empty() {
            self.reset();
            return None;
        }
        // Existing table syntax owns rule recognition; rules do not end a
        // pipe/Markdown table and do not acquire a status themselves.
        if automexia_ui_model::tables::is_rule_line(text.trim()) {
            return None;
        }
        let Some(fields) = Fields::parse(text) else {
            self.reset();
            return classify_row(text);
        };
        if let Some(schema) = Schema::header(&fields) {
            self.schema = Some(schema);
            return (!matches!(schema.kind, Kind::Namespace))
                .then(|| kube(Some(Severity::Info)));
        }
        if let Some(schema) = self.schema {
            // Preserve empty kubectl columns (for example an unbound PVC).
            // Compact producers continue through delimiter fields below.
            if fields.len < schema.len {
                if let Some(aligned) = schema.aligned_fields(text) {
                    if let Some(severity) = schema.row(&aligned, text) {
                        return Some(kube(severity));
                    }
                }
            }
            if let Some(severity) = schema.row(&fields, text) {
                return Some(kube(severity));
            }
            self.reset();
        }
        classify_row(text)
    }
}

fn kube(severity: Option<Severity>) -> OutputClassification {
    OutputClassification {
        domain: OutputDomain::Kubernetes,
        severity,
    }
}

/// kubectl columns use tabs or runs of spaces; single spaces inside a value
/// (cron schedules, event messages, HPA targets) remain part of that column.
/// A compact whitespace-only producer is supported when no column runs exist.
struct Fields<'a> {
    values: [&'a str; MAX_COLUMNS],
    len: usize,
    aligned: bool,
}

impl<'a> Fields<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let text = text.trim().trim_matches('|').trim();
        let mut fields = Self {
            values: [""; MAX_COLUMNS],
            len: 0,
            aligned: text.is_ascii()
                && text.contains("  ")
                && !text.contains(['|', '\t']),
        };
        if text.contains('|') {
            for value in text.split('|') {
                fields.push(value)?;
            }
        } else if text.contains('\t') {
            // Tabs are explicit column separators. Empty fields must keep
            // their positions, just as they do in pipe-delimited tables.
            for value in text.split('\t') {
                fields.push(value)?;
            }
        } else if text.contains("  ") {
            for value in text.split("  ") {
                if !value.trim().is_empty() {
                    fields.push(value)?;
                }
            }
        } else {
            for value in text.split_whitespace() {
                fields.push(value)?;
            }
        }
        (fields.len > 0).then_some(fields)
    }

    fn push(&mut self, value: &'a str) -> Option<()> {
        *self.values.get_mut(self.len)? = value.trim();
        self.len += 1;
        Some(())
    }

    fn has(&self, name: &str) -> bool {
        self.values[..self.len]
            .iter()
            .any(|value| value.eq_ignore_ascii_case(name))
    }
}

impl Schema {
    fn header(fields: &Fields<'_>) -> Option<Self> {
        let has = |name| fields.has(name);
        let name = has("NAME");
        let kind = if name && has("READY") && has("STATUS") {
            Kind::Pod
        } else if name && has("DESIRED") && has("CURRENT") && has("READY") {
            Kind::Replicas
        } else if name && has("READY") && has("AGE") {
            Kind::Ready
        } else if name && has("COMPLETIONS") && has("DURATION") {
            Kind::Job
        } else if name && has("SCHEDULE") && has("SUSPEND") && has("ACTIVE") {
            Kind::CronJob
        } else if name && has("STATUS") && has("ROLES") && has("VERSION") {
            Kind::Node
        } else if name
            && has("STATUS")
            && (has("VOLUME") || has("RECLAIM POLICY") || has("RECLAIMPOLICY"))
        {
            Kind::Storage
        } else if name && has("TYPE") && has("CLUSTER-IP") && has("EXTERNAL-IP") {
            Kind::Service
        } else if has("TYPE")
            && has("REASON")
            && has("MESSAGE")
            && (has("OBJECT") || has("LAST SEEN"))
        {
            Kind::Event
        } else if name && has("TARGETS") && has("MINPODS") && has("MAXPODS") {
            Kind::Autoscaler
        } else if name && has("SIGNERNAME") && has("CONDITION") {
            Kind::Certificate
        } else if name && has("SERVICE") && has("AVAILABLE") {
            Kind::ApiService
        }
        // NAME STATUS AGE alone is ambiguous. Only recognized namespace states
        // or a namespace/ prefix may claim a data row below.
        else if name && has("STATUS") && has("AGE") && fields.len <= 4 {
            Kind::Namespace
        } else if name
            && has("AGE")
            && (has("ENDPOINTS")
                || has("POD-SELECTOR")
                || has("HOSTS") && has("ADDRESS")
                || has("PROVISIONER") && has("RECLAIMPOLICY"))
        {
            Kind::Neutral
        } else {
            return None;
        };
        let mut columns = [Column::Other; MAX_COLUMNS];
        for (column, name) in columns.iter_mut().zip(&fields.values[..fields.len]) {
            *column = match *name {
                "NAME" => Column::Name,
                "NAMESPACE" => Column::Namespace,
                "READY" => Column::Ready,
                "STATUS" => Column::Status,
                "DESIRED" => Column::Desired,
                "CURRENT" => Column::Current,
                "UP-TO-DATE" => Column::Updated,
                "AVAILABLE" => Column::Available,
                "COMPLETIONS" => Column::Completions,
                "SUSPEND" => Column::Suspend,
                "TYPE" => Column::Type,
                "EXTERNAL-IP" => Column::ExternalIp,
                "TARGETS" => Column::Targets,
                "CONDITION" => Column::Condition,
                _ => Column::Other,
            };
        }
        // Human-facing default kubectl headers are uppercase. Avoid treating
        // incidental lowercase prose as a schema with no recognized columns.
        if !columns
            .iter()
            .any(|c| matches!(c, Column::Name | Column::Type))
        {
            return None;
        }
        let mut starts = [0; MAX_COLUMNS];
        // Every field was borrowed from the same source by Fields::parse.
        let origin = fields.values[0].as_ptr() as usize;
        for (start, value) in starts.iter_mut().zip(&fields.values[..fields.len]) {
            *start = (value.as_ptr() as usize).checked_sub(origin)?;
        }
        Some(Self {
            columns,
            starts,
            aligned: fields.aligned,
            len: fields.len,
            kind,
        })
    }

    fn aligned_fields<'a>(&self, text: &'a str) -> Option<Fields<'a>> {
        let text = text.trim();
        if !self.aligned || !text.is_ascii() {
            return None;
        }
        let mut fields = Fields {
            values: [""; MAX_COLUMNS],
            len: self.len,
            aligned: false,
        };
        for index in 0..self.len {
            let start = self.starts[index].min(text.len());
            let end = if index + 1 < self.len {
                self.starts[index + 1].min(text.len())
            } else {
                text.len()
            };
            // Never split a data token merely to fit a header's old geometry.
            if start > 0
                && start < text.len()
                && !text.as_bytes()[start - 1].is_ascii_whitespace()
                && !text.as_bytes()[start].is_ascii_whitespace()
            {
                return None;
            }
            fields.values[index] = text.get(start..end)?.trim();
        }
        Some(fields)
    }

    fn row(&self, fields: &Fields<'_>, text: &str) -> Option<Option<Severity>> {
        if fields.len < self.len {
            return None;
        }
        let get = |wanted: fn(Column) -> bool| {
            self.columns[..self.len]
                .iter()
                .position(|column| wanted(*column))
                .map(|index| fields.values[index])
        };
        let name = get(|c| matches!(c, Column::Name));
        if name.is_some_and(|value| !resource_name(value))
            || get(|c| matches!(c, Column::Namespace))
                .is_some_and(|value| !resource_name(value))
        {
            return None;
        }
        let status = get(|c| matches!(c, Column::Status));
        let ready = get(|c| matches!(c, Column::Ready));
        if matches!(self.kind, Kind::Namespace)
            && !matches!(status, Some("Active" | "Terminating"))
            && !name.is_some_and(|name| name.starts_with("namespace/"))
        {
            return None;
        }
        // A valid resource row with malformed/new status data retains domain
        // ownership, so generic words in its name cannot recolor it.
        let severity = (|| match self.kind {
            Kind::Pod => super::workloads::classify_kubernetes(text).flatten(),
            Kind::Ready => {
                let (ready, desired) = counts(ready?)?;
                let mut severity = readiness(ready, desired);
                for value in [
                    get(|c| matches!(c, Column::Updated)),
                    get(|c| matches!(c, Column::Available)),
                ]
                .into_iter()
                .flatten()
                {
                    if value.parse::<u32>().ok() != Some(desired) {
                        severity = Severity::Warning;
                    }
                }
                Some(severity)
            }
            Kind::Replicas => {
                let desired =
                    get(|c| matches!(c, Column::Desired))?.parse::<u32>().ok()?;
                let mut severity = readiness(ready?.parse().ok()?, desired);
                for value in [
                    get(|c| matches!(c, Column::Current)),
                    get(|c| matches!(c, Column::Updated)),
                    get(|c| matches!(c, Column::Available)),
                ]
                .into_iter()
                .flatten()
                {
                    if value.parse::<u32>().ok() != Some(desired) {
                        severity = Severity::Warning;
                    }
                }
                Some(severity)
            }
            Kind::Job => {
                if let Some(status) = status {
                    job_status(status)
                } else {
                    let (done, total) =
                        counts(get(|c| matches!(c, Column::Completions))?)?;
                    Some(if total > 0 && done >= total {
                        Severity::Info
                    } else {
                        Severity::Warning
                    })
                }
            }
            Kind::CronJob => match get(|c| matches!(c, Column::Suspend))? {
                "True" | "true" => Some(Severity::Warning),
                "False" | "false" => Some(Severity::Info),
                _ => None,
            },
            Kind::Node => match status? {
                "Ready" => Some(Severity::Success),
                "NotReady"
                | "Ready,SchedulingDisabled"
                | "NotReady,SchedulingDisabled"
                | "Unknown" => Some(Severity::Warning),
                _ => None,
            },
            Kind::Storage => match status? {
                "Bound" => Some(Severity::Success),
                "Available" => Some(Severity::Info),
                "Pending" | "Released" | "Terminating" => Some(Severity::Warning),
                "Lost" | "Failed" => Some(Severity::Error),
                _ => None,
            },
            Kind::Namespace => match status? {
                "Active" => Some(Severity::Success),
                "Terminating" => Some(Severity::Warning),
                _ => None,
            },
            Kind::Service => Some(
                if get(|c| matches!(c, Column::Type)) == Some("LoadBalancer")
                    && get(|c| matches!(c, Column::ExternalIp)) == Some("<pending>")
                {
                    Severity::Warning
                } else {
                    Severity::Info
                },
            ),
            Kind::Event => match get(|c| matches!(c, Column::Type))? {
                "Warning" => Some(Severity::Warning),
                "Normal" => Some(Severity::Info),
                _ => None,
            },
            Kind::Autoscaler => Some(
                if get(|c| matches!(c, Column::Targets))?.contains("<unknown>") {
                    Severity::Warning
                } else {
                    Severity::Info
                },
            ),
            Kind::Certificate => certificate(get(|c| matches!(c, Column::Condition))?),
            // API services append a diagnostic reason, e.g.
            // "False (MissingEndpoints)". The condition still owns severity.
            Kind::ApiService => match get(|c| matches!(c, Column::Available))?
                .split_whitespace()
                .next()?
            {
                "True" => Some(Severity::Success),
                "False" => Some(Severity::Error),
                "Unknown" => Some(Severity::Warning),
                _ => None,
            },
            Kind::Neutral => Some(Severity::Info),
        })();
        Some(severity)
    }
}

fn resource_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-._/".contains(&b))
}

fn counts(value: &str) -> Option<(u32, u32)> {
    let (ready, desired) = value.split_once('/')?;
    if ready.is_empty()
        || desired.is_empty()
        || !ready
            .bytes()
            .chain(desired.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((ready.parse().ok()?, desired.parse().ok()?))
}

/// `kubectl get all` resource prefixes remain useful when the header has
/// scrolled out of view. Bare ambiguous numeric rows require a header.
pub(super) fn named_resource(text: &str) -> Option<OutputClassification> {
    let fields = Fields::parse(text)?;
    let index =
        (0..fields.len.min(2)).find(|index| fields.values[*index].contains('/'))?;
    let (kind, name) = fields.values[index].split_once('/')?;
    if !resource_name(name) {
        return None;
    }
    let kind = kind.split('.').next()?;
    let header = match kind {
        "deployment" | "deployments" => "NAME  READY  UP-TO-DATE  AVAILABLE  AGE",
        "statefulset" | "statefulsets" => "NAME  READY  AGE",
        "replicaset" | "replicationcontroller" => "NAME  DESIRED  CURRENT  READY  AGE",
        "daemonset" => {
            "NAME  DESIRED  CURRENT  READY  UP-TO-DATE  AVAILABLE  NODE SELECTOR  AGE"
        }
        "job" => {
            if fields
                .values
                .get(index + 1)
                .is_some_and(|value| value.contains('/'))
            {
                "NAME  COMPLETIONS  DURATION  AGE"
            } else {
                "NAME  STATUS  COMPLETIONS  DURATION  AGE"
            }
        }
        "namespace" => "NAME  STATUS  AGE",
        "node" => "NAME  STATUS  ROLES  AGE  VERSION",
        "persistentvolumeclaim" => {
            "NAME  STATUS  VOLUME  CAPACITY  ACCESS MODES  STORAGECLASS  AGE"
        }
        "service" => "NAME  TYPE  CLUSTER-IP  EXTERNAL-IP  PORT(S)  AGE",
        _ => return None,
    };
    let mut row = Fields {
        values: [""; MAX_COLUMNS],
        len: fields.len - index,
        aligned: false,
    };
    row.values[..row.len].copy_from_slice(&fields.values[index..fields.len]);
    let schema = Schema::header(&Fields::parse(header)?)?;
    schema.row(&row, text).map(kube)
}

fn readiness(ready: u32, desired: u32) -> Severity {
    if ready != desired {
        Severity::Warning
    } else if desired == 0 {
        Severity::Info
    } else {
        Severity::Success
    }
}

fn job_status(value: &str) -> Option<Severity> {
    match value {
        "Complete" | "Completed" | "Succeeded" => Some(Severity::Info),
        "Failed" | "FailureTarget" => Some(Severity::Error),
        "Running" | "Suspended" | "Terminating" | "Unknown" | "SuccessCriteriaMet" => {
            Some(Severity::Warning)
        }
        _ => None,
    }
}

fn certificate(value: &str) -> Option<Severity> {
    let mut approved = false;
    let mut issued = false;
    for part in value.split(',') {
        match part.trim() {
            "Denied" | "Failed" => return Some(Severity::Error),
            "Approved" => approved = true,
            "Issued" => issued = true,
            "Pending" => return Some(Severity::Warning),
            _ => return None,
        }
    }
    Some(if approved && issued {
        Severity::Success
    } else {
        Severity::Info
    })
}
