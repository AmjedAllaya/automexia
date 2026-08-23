//! Pure, bounded keybinding compilation and resolution.
//!
//! This private crate deliberately owns no filesystem, process, PTY, window,
//! renderer, clipboard, environment, credential, or network authority.

mod action;
mod binding;
mod compiler;
mod fixture;
mod parser;
mod profile;
mod registry;
mod sequence;
mod trigger;

pub use action::{
    action_schemas, resolve_action, ActionCapability, ActionId, ActionInvocation,
    ActionSchema, ParameterKind, SupportLevel,
};
pub use binding::{
    BindingOperation, BindingOrigin, BindingPolicy, BindingScope, BindingSpec,
    Consumption, ModeFlags, ModePredicate,
};
pub use compiler::{
    compile, compile_with_options, CompileError, CompileErrorCode, CompileOptions,
    CompileReport,
};
pub use fixture::{
    ghostty_1_3_linux_bindings, ghostty_1_3_linux_provenance,
    parse_ghostty_keybind_fixture, FixtureGenerator, FixtureOutput, FixtureOutputs,
    FixtureProvenance, FixtureUpstream, GHOSTTY_1_3_ACTIONS, GHOSTTY_1_3_LINUX_KEYBINDS,
    GHOSTTY_1_3_LINUX_PROVENANCE,
};
pub use parser::{parse_binding_line, parse_binding_lines, ParseError, ParsedLine};
pub use profile::{
    adapt_for_windows, bundled_profile, PlatformFamily, ProfileId, ProfileResolution,
    GHOSTTY_1_3_COMMIT, GHOSTTY_1_3_PATCH, GHOSTTY_1_3_TAG,
};
pub use registry::{CompiledBinding, CompiledRegistry, RegistryStats};
pub use sequence::{
    ActionDamage, ActionOutcome, ActionStatus, CancellationReason, SequenceResolution,
    SurfaceBindingState, TableActivation,
};
pub use trigger::{KeyAtom, Modifiers, NamedKey, NormalizedKeyEvent, Trigger};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_BINDINGS: usize = 4_096;
pub const MAX_DIAGNOSTICS: usize = 256;
pub const MAX_SEQUENCE_CHORDS: usize = 8;
pub const MAX_CHAIN_ACTIONS: usize = 32;
pub const MAX_TABLE_DEPTH: usize = 32;
pub const MAX_IDENTIFIER_BYTES: usize = 128;
pub const MAX_PARAMETER_BYTES: usize = 4 * 1024;
pub const MAX_PENDING_BYTES: usize = 4 * 1024;
pub const MAX_FIXTURE_BYTES: usize = 256 * 1024;
