mod context;
mod model;
mod semantics;

use crate::automexia::api::{Capability, ExtensionManifest};

pub use context::detect;
pub use model::{CloudContext, DevOpsSnapshot};
pub use semantics::classify_row_text;

pub const ID: &str = "automexia.devops";
pub const VERSION: &str = "0.3.13";

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Automexia DevOps",
    description: "Native DevOps context HUD plus semantic error, warning, success and log highlighting.",
    version: VERSION,
    // This is part of the default Automexia experience. /market can disable it
    // without introducing shell commands or modifying the user's prompt.
    default_enabled: true,
    capabilities: &[
        Capability::FilesystemRead,
        Capability::EnvironmentRead,
        Capability::TerminalOutputRead,
        Capability::UiOverlay,
    ],
};
