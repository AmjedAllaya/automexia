//! User-configurable presentation of ordinary terminal output.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Presentation {
    pub inline_tables: bool,
    pub output_highlighting: bool,
    pub command_timestamps: bool,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            inline_tables: true,
            output_highlighting: true,
            command_timestamps: true,
        }
    }
}
