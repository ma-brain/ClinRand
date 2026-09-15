//! Study configuration types matching plan §4 / §5.1.
//!
//! JSON wire format is not internally tagged: `method` is a string sibling of
//! `block` (when present) and `numbering`. Do not change field names without
//! updating the published schema (plan §5.1).

use serde::{Deserialize, Serialize};

/// Study randomization configuration (plan §4).
///
/// Contract-bound field names: they are the JSON schema in plan §5.1 and a
/// published interface. Changing them is a breaking change to configs on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StudyConfigWire", into = "StudyConfigWire")]
pub struct StudyConfig {
    /// Schema identifier, currently `"1.0"`.
    pub schema_version: String,
    /// Study identifier. Synthetic fixtures must match `^(DEMO|TEST|EXAMPLE)-`.
    pub study_id: String,
    /// Protocol version string from the config, not the engine version.
    pub protocol_version: String,
    /// Treatment arms in config order.
    pub arms: Vec<Arm>,
    /// Randomization method. On the wire this is a `method` string plus optional sibling `block`.
    pub method: Method,
    /// Stratification factors in config order (plan §5.4). Do not sort.
    pub strata: Vec<StratificationFactor>,
    /// Allocations generated per stratum combination.
    pub list_length_per_stratum: u32,
    /// Randomization-number assignment (plan §5.6).
    pub numbering: NumberingScheme,
}

/// Treatment arm (plan §4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arm {
    /// Short arm code written to the list.
    pub code: String,
    /// Human-readable label.
    pub label: String,
    /// Integer allocation ratio. No floats.
    pub ratio: u32,
}

/// Randomization method (plan §4).
///
/// JSON: `"method": "stratified_block"` plus sibling `"block"` when the
/// variant requires one. `simple` has no `block`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    /// Unblocked simple randomization.
    Simple,
    /// Permuted-block randomization; requires `block`.
    PermutedBlock { block: BlockScheme },
    /// Stratified permuted-block randomization; requires `block`.
    StratifiedBlock { block: BlockScheme },
}

/// Block size scheme (plan §4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockScheme {
    /// Every block has this size.
    Fixed { size: u32 },
    /// Each block draws a size from `sizes`.
    Variable { sizes: Vec<u32> },
}

/// One stratification factor and its levels (plan §4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StratificationFactor {
    /// Factor name, unique within a config.
    pub name: String,
    /// Levels in config order (plan §5.4). Do not sort.
    pub levels: Vec<String>,
}

/// Randomization-number assignment (plan §4 / §5.6).
///
/// `Global` is listed first so any defaulting lands on it, not
/// `PerStratumRange`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NumberingScheme {
    /// One ascending counter across the whole list.
    Global { start: u32, width: u8 },
    /// Reserved contiguous range per stratum combination.
    PerStratumRange {
        start: u32,
        block_size: u32,
        width: u8,
    },
}

#[derive(Serialize, Deserialize)]
struct StudyConfigWire {
    schema_version: String,
    study_id: String,
    protocol_version: String,
    arms: Vec<Arm>,
    method: MethodName,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    block: Option<BlockScheme>,
    strata: Vec<StratificationFactor>,
    list_length_per_stratum: u32,
    numbering: NumberingScheme,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MethodName {
    Simple,
    PermutedBlock,
    StratifiedBlock,
}

struct ConfigWireError(&'static str);

impl std::fmt::Display for ConfigWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl TryFrom<StudyConfigWire> for StudyConfig {
    type Error = ConfigWireError;

    fn try_from(wire: StudyConfigWire) -> Result<Self, Self::Error> {
        let method = match (wire.method, wire.block) {
            (MethodName::Simple, None) => Method::Simple,
            (MethodName::Simple, Some(_)) => {
                return Err(ConfigWireError(
                    "method \"simple\" must not include a block field",
                ));
            }
            (MethodName::PermutedBlock, Some(block)) => Method::PermutedBlock { block },
            (MethodName::PermutedBlock, None) => {
                return Err(ConfigWireError(
                    "method \"permuted_block\" requires a block field",
                ));
            }
            (MethodName::StratifiedBlock, Some(block)) => Method::StratifiedBlock { block },
            (MethodName::StratifiedBlock, None) => {
                return Err(ConfigWireError(
                    "method \"stratified_block\" requires a block field",
                ));
            }
        };
        Ok(Self {
            schema_version: wire.schema_version,
            study_id: wire.study_id,
            protocol_version: wire.protocol_version,
            arms: wire.arms,
            method,
            strata: wire.strata,
            list_length_per_stratum: wire.list_length_per_stratum,
            numbering: wire.numbering,
        })
    }
}

impl From<StudyConfig> for StudyConfigWire {
    fn from(cfg: StudyConfig) -> Self {
        let (method, block) = match cfg.method {
            Method::Simple => (MethodName::Simple, None),
            Method::PermutedBlock { block } => (MethodName::PermutedBlock, Some(block)),
            Method::StratifiedBlock { block } => (MethodName::StratifiedBlock, Some(block)),
        };
        Self {
            schema_version: cfg.schema_version,
            study_id: cfg.study_id,
            protocol_version: cfg.protocol_version,
            arms: cfg.arms,
            method,
            block,
            strata: cfg.strata,
            list_length_per_stratum: cfg.list_length_per_stratum,
            numbering: cfg.numbering,
        }
    }
}
