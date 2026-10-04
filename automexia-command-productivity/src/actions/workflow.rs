//! Explicitly reviewed shell workflows. This model owns no I/O or execution.
use super::ShellKind;
use serde::{Deserialize, Serialize};

pub const WORKFLOW_VERSION: u8 = 1;
pub const MAX_WORKFLOW_STEPS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStep {
    pub command: String,
    pub shell: ShellKind,
    #[serde(default)]
    pub completion: WorkflowCompletion,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
}

const fn default_timeout() -> u32 {
    300
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowCompletion {
    #[default]
    Success,
    /// Explicitly reviewed transition into an integrated child shell, such as SSH.
    NewShell,
    /// Pause after this command, for human inspection or interactive setup.
    Pause,
}

pub fn validate_workflow_steps(steps: &[WorkflowStep]) -> Result<(), &'static str> {
    if steps.is_empty() || steps.len() > MAX_WORKFLOW_STEPS {
        return Err("A workflow needs between 1 and 32 steps.");
    }
    for step in steps {
        if step.command.trim().is_empty()
            || step.command.len() > super::MAX_STRING_BYTES
            || step.command.chars().any(|c| {
                c.is_control()
                    || matches!(c,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}' |
                '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return Err("Each step needs one command line without control characters (up to 4096 bytes).");
        }
        if !(1..=3600).contains(&step.timeout_seconds) {
            return Err("Step timeout must be between 1 and 3600 seconds.");
        }
    }
    Ok(())
}

/// Content-free observation from the existing shell integration owner. It is a
/// sequencing hint for previously approved text, never authentication or consent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowObservation {
    pub generation: u64,
    pub scope: u64,
    pub input_revision: u64,
    pub prompt: Option<u64>,
    pub completed: Option<(u64, i32)>,
    pub shell: Option<ShellKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowEffect {
    Submit(usize),
    Wait,
    Paused(&'static str),
    Finished,
}

#[derive(Clone, Debug)]
pub struct WorkflowRun {
    steps: Vec<WorkflowStep>,
    next: usize,
    submitted: bool,
    baseline: WorkflowObservation,
    deadline_ms: u64,
    paused: Option<&'static str>,
}

impl WorkflowRun {
    pub fn new(
        steps: Vec<WorkflowStep>,
        initial: WorkflowObservation,
        now_ms: u64,
    ) -> Result<Self, &'static str> {
        validate_workflow_steps(&steps)?;
        if initial.prompt.is_none() || initial.shell != Some(steps[0].shell) {
            return Err(
                "Start from an empty integrated prompt in the workflow's first shell.",
            );
        }
        Ok(Self {
            steps,
            next: 0,
            submitted: false,
            baseline: initial,
            deadline_ms: now_ms,
            paused: None,
        })
    }

    pub fn steps(&self) -> &[WorkflowStep] {
        &self.steps
    }
    pub fn position(&self) -> usize {
        self.next
    }
    pub fn pause(&mut self, reason: &'static str) {
        self.paused = Some(reason);
    }

    /// Resume never repeats a submitted command. An in-flight command must first
    /// reach its reviewed completion condition; users can cancel and edit instead.
    pub fn resume(
        &mut self,
        observed: WorkflowObservation,
        now_ms: u64,
    ) -> Result<(), &'static str> {
        let transition = self.submitted
            && self
                .steps
                .get(self.next)
                .is_some_and(|s| s.completion == WorkflowCompletion::NewShell);
        if !transition
            && (observed.generation != self.baseline.generation
                || observed.scope != self.baseline.scope)
        {
            return Err("The terminal changed. Cancel and review the workflow again.");
        }
        if observed.prompt.is_none() {
            return Err("Wait for an empty integrated prompt.");
        }
        self.baseline.input_revision = observed.input_revision;
        self.deadline_ms = now_ms.saturating_add(
            self.steps
                .get(self.next)
                .map_or(0, |s| u64::from(s.timeout_seconds) * 1000),
        );
        self.paused = None;
        Ok(())
    }

    pub fn poll(&mut self, observed: WorkflowObservation, now_ms: u64) -> WorkflowEffect {
        if let Some(reason) = self.paused {
            return WorkflowEffect::Paused(reason);
        }
        if self.next >= self.steps.len() {
            return WorkflowEffect::Finished;
        }
        let transition = self.submitted
            && self.steps[self.next].completion == WorkflowCompletion::NewShell
            && observed.scope != self.baseline.scope
            && observed.generation == self.baseline.generation.saturating_add(1);
        if observed.generation != self.baseline.generation && !transition {
            return self.stop("The terminal was reset. Review the workflow again.");
        }
        if observed.input_revision != self.baseline.input_revision {
            return self.stop(
                "Keyboard or pasted input paused the workflow. Resume when ready.",
            );
        }
        if self.submitted {
            let step = &self.steps[self.next];
            if now_ms >= self.deadline_ms {
                return self.stop("Step timed out. No further commands were sent.");
            }
            if step.completion == WorkflowCompletion::NewShell {
                if observed.scope == self.baseline.scope
                    && observed.prompt.is_some()
                    && observed.prompt != self.baseline.prompt
                    && observed
                        .completed
                        .is_some_and(|(id, _)| Some(id) == self.baseline.prompt)
                {
                    return self.stop(
                        "The command returned without entering a new integrated shell.",
                    );
                }
                // The approved transition may replace the advisory shell metadata.
                // Never send commands to unknown shells or to password/MFA input.
                if observed.scope == self.baseline.scope
                    || observed.prompt.is_none()
                    || observed.shell.is_none()
                {
                    return WorkflowEffect::Wait;
                }
            } else {
                if observed.scope != self.baseline.scope {
                    return self
                        .stop("Unexpected shell change. Review the workflow again.");
                }
                let Some(prompt) = observed.prompt else {
                    return WorkflowEffect::Wait;
                };
                if Some(prompt) == self.baseline.prompt {
                    return WorkflowEffect::Wait;
                }
                let Some((source, status)) = observed.completed else {
                    return self
                        .stop("The shell did not report this command's exit status.");
                };
                if Some(source) != self.baseline.prompt {
                    return self.stop(
                        "Command tracking changed. No further commands were sent.",
                    );
                }
                if status != 0 {
                    return self
                        .stop("Command failed. Cancel or review the remaining steps.");
                }
            }
            let pause = self.steps[self.next].completion == WorkflowCompletion::Pause;
            self.next += 1;
            self.submitted = false;
            self.baseline = observed;
            if pause {
                return self.stop("Step completed. Resume to continue.");
            }
        }
        if self.next >= self.steps.len() {
            return WorkflowEffect::Finished;
        }
        if observed.scope != self.baseline.scope
            || observed.prompt != self.baseline.prompt
        {
            return self.stop("The prompt changed before submission. Review again.");
        }
        if observed.prompt.is_none()
            || observed.shell != Some(self.steps[self.next].shell)
        {
            return self.stop("This step needs a different integrated shell.");
        }
        self.submitted = true;
        self.deadline_ms = now_ms
            .saturating_add(u64::from(self.steps[self.next].timeout_seconds) * 1000);
        WorkflowEffect::Submit(self.next)
    }

    fn stop(&mut self, reason: &'static str) -> WorkflowEffect {
        self.paused = Some(reason);
        WorkflowEffect::Paused(reason)
    }
}
