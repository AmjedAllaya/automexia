//! Host-owned admission for one explicitly authorized semantic surface operation.
//!
//! This synchronous, pure owner has no automatic activation or provider transport.
//! Callers must run decoding outside input/PTY/renderer paths, authenticate the
//! producer, obtain the grant through the host broker, assign a fresh generation
//! on reopen, and revoke on route/capsule replacement or extension disable.
//! A future multi-surface owner must additionally bound aggregate slots/bytes.

use automexia_extension_api::{
    surface::{
        SemanticSurfaceId, SemanticTable, SurfaceBinding, SurfaceEvent, SurfaceTitle,
        SurfaceUpdate, MAX_SURFACE_FRAME_BYTES,
    },
    Capability, CapabilityDecision, Decision, ResourceScope,
};
use automexia_ui_model::semantic_table::{Navigation, TablePresentation};

/// Fixed redacted errors. Never return serde errors containing provider text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    Unauthorized,
    Closed,
    Stale,
    FrameTooLarge,
    InvalidFrame,
    NotReady,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfacePhase {
    Loading,
    Ready,
    Failed(SurfaceTitle),
    Closed,
}

/// Revision and borrowed data are issued together, so input can retain the exact
/// displayed revision without consulting a later snapshot during dispatch.
pub struct PresentedTable<'a> {
    pub revision: u64,
    pub phase: &'a SurfacePhase,
    pub table: &'a TablePresentation,
}

#[derive(Debug)]
pub struct SemanticSurfaceSlot {
    binding: SurfaceBinding,
    surface: SemanticSurfaceId,
    generation: u64,
    revision: u64,
    expires_at_ms: u64,
    last_observed_ms: u64,
    once: bool,
    consumed: bool,
    phase: SurfacePhase,
    snapshot: Option<TablePresentation>,
}

impl SemanticSurfaceSlot {
    /// `binding` and `grant` must come from independent trusted host state, never
    /// from a provider's frame. AllowOnce authorizes one accepted update, not an
    /// unlimited stream. Only AllowSession supports loading/refresh sequences.
    pub fn new(
        binding: SurfaceBinding,
        surface: SemanticSurfaceId,
        generation: u64,
        grant: Option<&CapabilityDecision>,
        now_ms: u64,
    ) -> Result<Self, AdmissionError> {
        let grant = grant.ok_or(AdmissionError::Unauthorized)?;
        if generation == 0
            || grant.capability != Capability::UiOverlay
            || grant.resource != ResourceScope::Session
            || grant.decision == Decision::Deny
            || grant.extension_id != binding.extension
            || grant.session_id != binding.session
            || grant.operation_id != binding.operation
            || grant.capsule_revision != binding.capsule_revision
            || now_ms < grant.decided_at_ms
            || now_ms >= grant.expires_at_ms
        {
            return Err(AdmissionError::Unauthorized);
        }
        Ok(Self {
            binding,
            surface,
            generation,
            revision: 0,
            expires_at_ms: grant.expires_at_ms,
            last_observed_ms: now_ms,
            once: grant.decision == Decision::AllowOnce,
            consumed: false,
            phase: SurfacePhase::Loading,
            snapshot: None,
        })
    }

    fn active(&mut self, now_ms: u64) -> Result<(), AdmissionError> {
        if self.phase == SurfacePhase::Closed {
            return Err(AdmissionError::Closed);
        }
        if now_ms < self.last_observed_ms || now_ms >= self.expires_at_ms {
            self.revoke();
            return Err(AdmissionError::Unauthorized);
        }
        self.last_observed_ms = now_ms;
        Ok(())
    }

    pub fn accept_frame(
        &mut self,
        frame: &[u8],
        now_ms: u64,
    ) -> Result<(), AdmissionError> {
        self.active(now_ms)?;
        if self.once && self.consumed {
            return Err(AdmissionError::Unauthorized);
        }
        // Framing precedes deserialization, including escaped-string scratch space.
        if frame.len() > MAX_SURFACE_FRAME_BYTES {
            return Err(AdmissionError::FrameTooLarge);
        }
        let update: SurfaceUpdate =
            serde_json::from_slice(frame).map_err(|_| AdmissionError::InvalidFrame)?;
        if update.binding() != &self.binding
            || update.surface() != self.surface
            || update.generation() != self.generation
            || update.revision() <= self.revision
        {
            return Err(AdmissionError::Stale);
        }
        // No observable mutation until the entire frame and identity are accepted.
        self.revision = update.revision();
        self.consumed = true;
        match update.into_event() {
            SurfaceEvent::Loading => self.phase = SurfacePhase::Loading,
            SurfaceEvent::Replace(table) => {
                if let Some(presentation) = &mut self.snapshot {
                    presentation.replace(table);
                } else {
                    self.snapshot = Some(TablePresentation::new(table));
                }
                self.phase = SurfacePhase::Ready;
            }
            SurfaceEvent::Failed(message) => self.phase = SurfacePhase::Failed(message),
            SurfaceEvent::Close => self.revoke(),
        }
        Ok(())
    }

    pub fn phase(&mut self, now_ms: u64) -> &SurfacePhase {
        let _ = self.active(now_ms);
        &self.phase
    }

    /// Expiry is checked on reads too; an idle producer cannot keep data visible
    /// after its host authorization expires. The returned snapshot is borrowed.
    pub fn snapshot(&mut self, now_ms: u64) -> Option<&SemanticTable> {
        self.active(now_ms).ok()?;
        self.snapshot.as_ref().map(TablePresentation::table)
    }

    /// Read-only projection of the same admitted snapshot, with no second copy.
    pub fn presentation(&mut self, now_ms: u64) -> Option<PresentedTable<'_>> {
        self.active(now_ms).ok()?;
        Some(PresentedTable {
            revision: self.revision,
            phase: &self.phase,
            table: self.snapshot.as_ref()?,
        })
    }

    pub fn fit_table(
        &mut self,
        columns: usize,
        rows: usize,
        now_ms: u64,
    ) -> Result<(), AdmissionError> {
        self.active(now_ms)?;
        self.snapshot
            .as_mut()
            .ok_or(AdmissionError::NotReady)?
            .fit(columns, rows);
        Ok(())
    }

    /// Input must carry the revision that was presented, never the newest value
    /// read after a delayed event. Selection does not grant resource authority.
    pub fn navigate_table(
        &mut self,
        revision: u64,
        navigation: Navigation,
        now_ms: u64,
    ) -> Result<bool, AdmissionError> {
        self.active(now_ms)?;
        if revision != self.revision {
            return Err(AdmissionError::Stale);
        }
        if self.phase != SurfacePhase::Ready {
            return Err(AdmissionError::NotReady);
        }
        Ok(self
            .snapshot
            .as_mut()
            .ok_or(AdmissionError::NotReady)?
            .navigate(navigation))
    }

    pub fn revoke(&mut self) {
        self.snapshot = None;
        self.phase = SurfacePhase::Closed;
    }
}
