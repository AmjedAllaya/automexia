//! Inherited, one-shot helper bootstrap carrying the private route authority.

use std::fmt;
use std::io::{self, Read};
use std::path::Path;

use automexia_command_productivity::suggestions::{RouteIdentity, SuggestionCapability};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{HelperSessionBinding, HelperSessionBridge};

const BOOTSTRAP_SCHEMA: u16 = 1;
const BOOTSTRAP_BYTES: usize = 16 * 1024;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "transport", content = "locator")]
pub enum HelperEndpointLocator {
    WindowsPipe(String),
    UnixSocket(String),
}

impl HelperEndpointLocator {
    fn validate(&self) -> bool {
        match self {
            Self::WindowsPipe(value) => {
                value.starts_with(r"\\.\pipe\Automexia.Suggestions.")
                    && value.len() <= 512
                    && !value.chars().any(char::is_control)
            }
            Self::UnixSocket(value) => {
                Path::new(value).is_absolute()
                    && value.len() <= 4096
                    && !value.contains('\0')
                    && !value.chars().any(|character| character.is_control())
            }
        }
    }
}

#[derive(Clone)]
pub struct HelperBootstrap {
    pub schema: u16,
    pub endpoint: HelperEndpointLocator,
    pub binding: HelperSessionBinding,
}

impl HelperBootstrap {
    pub fn validate(&self) -> Result<(), HelperBootstrapError> {
        if self.schema != BOOTSTRAP_SCHEMA || !self.endpoint.validate() {
            return Err(HelperBootstrapError::Invalid);
        }
        HelperSessionBridge::new(self.binding.clone())
            .map(|_| ())
            .map_err(|_| HelperBootstrapError::Invalid)
    }
}

impl fmt::Debug for HelperBootstrap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HelperBootstrap")
            .field("schema", &self.schema)
            .field(
                "transport",
                &match self.endpoint {
                    HelperEndpointLocator::WindowsPipe(_) => "windows-pipe",
                    HelperEndpointLocator::UnixSocket(_) => "unix-socket",
                },
            )
            .field("binding", &self.binding)
            .finish()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HelperBootstrapWire {
    schema: u16,
    endpoint: HelperEndpointLocator,
    route: RouteIdentity,
    capability: SuggestionCapability,
    prompt_generation: u64,
    source_revision: u64,
}

impl Serialize for HelperBootstrap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        HelperBootstrapWire {
            schema: self.schema,
            endpoint: self.endpoint.clone(),
            route: self.binding.route.clone(),
            capability: self.binding.capability,
            prompt_generation: self.binding.prompt_generation,
            source_revision: self.binding.source_revision,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HelperBootstrap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = HelperBootstrapWire::deserialize(deserializer)?;
        Ok(Self {
            schema: wire.schema,
            endpoint: wire.endpoint,
            binding: HelperSessionBinding {
                route: wire.route,
                capability: wire.capability,
                prompt_generation: wire.prompt_generation,
                source_revision: wire.source_revision,
            },
        })
    }
}

#[derive(Debug)]
pub enum HelperBootstrapError {
    Io(io::Error),
    TooLarge,
    Length,
    Json,
    Invalid,
}

impl fmt::Display for HelperBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io(_) => "suggestion helper bootstrap IO failed",
            Self::TooLarge => "suggestion helper bootstrap exceeds its limit",
            Self::Length => "suggestion helper bootstrap length is invalid",
            Self::Json => "suggestion helper bootstrap schema is invalid",
            Self::Invalid => "suggestion helper bootstrap authority is invalid",
        })
    }
}

impl std::error::Error for HelperBootstrapError {}

pub fn encode_helper_bootstrap(
    bootstrap: &HelperBootstrap,
) -> Result<Vec<u8>, HelperBootstrapError> {
    bootstrap.validate()?;
    let payload =
        serde_json::to_vec(bootstrap).map_err(|_| HelperBootstrapError::Json)?;
    if payload.len() > BOOTSTRAP_BYTES {
        return Err(HelperBootstrapError::TooLarge);
    }
    let length =
        u32::try_from(payload.len()).map_err(|_| HelperBootstrapError::TooLarge)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_helper_bootstrap(
    frame: &[u8],
) -> Result<HelperBootstrap, HelperBootstrapError> {
    let prefix: [u8; 4] = frame
        .get(..4)
        .ok_or(HelperBootstrapError::Length)?
        .try_into()
        .map_err(|_| HelperBootstrapError::Length)?;
    let declared = u32::from_le_bytes(prefix) as usize;
    if declared > BOOTSTRAP_BYTES {
        return Err(HelperBootstrapError::TooLarge);
    }
    let payload = frame.get(4..).ok_or(HelperBootstrapError::Length)?;
    if payload.len() != declared {
        return Err(HelperBootstrapError::Length);
    }
    let bootstrap = serde_json::from_slice::<HelperBootstrap>(payload)
        .map_err(|_| HelperBootstrapError::Json)?;
    bootstrap.validate()?;
    Ok(bootstrap)
}

pub fn read_helper_bootstrap(
    reader: &mut impl Read,
) -> Result<HelperBootstrap, HelperBootstrapError> {
    let mut prefix = [0_u8; 4];
    reader
        .read_exact(&mut prefix)
        .map_err(HelperBootstrapError::Io)?;
    let declared = u32::from_le_bytes(prefix) as usize;
    if declared > BOOTSTRAP_BYTES {
        return Err(HelperBootstrapError::TooLarge);
    }
    let mut frame = Vec::with_capacity(4 + declared);
    frame.extend_from_slice(&prefix);
    frame.resize(4 + declared, 0);
    reader
        .read_exact(&mut frame[4..])
        .map_err(HelperBootstrapError::Io)?;
    decode_helper_bootstrap(&frame)
}
