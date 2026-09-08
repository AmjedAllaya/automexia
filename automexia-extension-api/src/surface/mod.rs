//! Bounded, capability-free semantic surface data. Not a renderer or a transport.
//!
//! Producers must select safe display fields, excluding credentials and raw
//! authentication objects. Display data can still be confidential: this local
//! UI contract is not a release format or permission to log, persist or publish it.
//! Validation rejects hostile syntax, not confidential information in ordinary text.
//! Hosts must bound encoded frames before decoding, authenticate the producer and
//! independently authorize every surface operation. Data cannot grant authority.

mod bounds;

use crate::{
    ContractError, ContractVersion, ExtensionId, OperationId, SemanticSeverity, SessionId,
};
use bounds::{Charged, Cost, List};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeSet, fmt, num::NonZeroU64};

pub const MAX_SURFACE_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_TABLE_COLUMNS: usize = 64;
pub const MAX_TABLE_ROWS: usize = 20_000;
pub const MAX_TABLE_CELLS: usize = 320_000;
pub const MAX_TABLE_TEXT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CELL_TEXT_BYTES: usize = 1024;
pub const MAX_SURFACE_TITLE_BYTES: usize = 128;
pub const MAX_SURFACE_ID_BYTES: usize = 64;

macro_rules! handle {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(NonZeroU64);
        impl $name {
            pub fn new(value: u64) -> Result<Self, ContractError> {
                NonZeroU64::new(value)
                    .map(Self)
                    .ok_or(ContractError::InvalidValue("surface handle"))
            }
            pub fn get(self) -> u64 {
                self.0.get()
            }
        }
    };
}
handle!(SemanticSurfaceId);
handle!(RowId);
handle!(ResourceHandle);

macro_rules! public_text {
    ($name:ident, $max:expr) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(Box<str>);
        impl $name {
            pub fn new(value: impl AsRef<str>) -> Result<Self, ContractError> {
                let value = value.as_ref();
                bounds::display_text(value, $max)?;
                Ok(Self(value.into()))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<display-data>)"))
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                bounds::text::<D, $max>(d).map(Self)
            }
        }
    };
}
public_text!(CellText, MAX_CELL_TEXT_BYTES);
public_text!(SurfaceTitle, MAX_SURFACE_TITLE_BYTES);

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(Box<str>);
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<identifier>)"))
            }
        }
        impl $name {
            pub fn new(value: impl AsRef<str>) -> Result<Self, ContractError> {
                let value = value.as_ref();
                if value.is_empty()
                    || value.len() > MAX_SURFACE_ID_BYTES
                    || !value.bytes().all(|c| {
                        c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-')
                    })
                {
                    return Err(ContractError::InvalidValue("surface identifier"));
                }
                Ok(Self(value.into()))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let value = bounds::text::<D, MAX_SURFACE_ID_BYTES>(d)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}
identifier!(TableSchemaId);
identifier!(ColumnId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverflowPolicy {
    Ellipsis,
    HorizontalScroll,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResponsivePolicy {
    Always,
    Hide,
    Details,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataKind {
    Text,
    Identifier,
    Status,
    Percentage,
    Bytes,
    Duration,
    Timestamp,
    Boolean,
    Number,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RowIdentityPolicy {
    StableHandle,
}

/// Widths are terminal cells. This contract never accepts zero-width columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnWidths {
    min: u16,
    preferred: u16,
    max: Option<u16>,
}
impl ColumnWidths {
    pub fn new(
        min: u16,
        preferred: u16,
        max: Option<u16>,
    ) -> Result<Self, ContractError> {
        if min == 0 || preferred < min || max.is_some_and(|max| max < preferred) {
            return Err(ContractError::InvalidValue("column widths"));
        }
        Ok(Self {
            min,
            preferred,
            max,
        })
    }
    pub fn min(self) -> u16 {
        self.min
    }
    pub fn preferred(self) -> u16 {
        self.preferred
    }
    pub fn max(self) -> Option<u16> {
        self.max
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ColumnWire")]
pub struct ColumnSchema {
    id: ColumnId,
    title: SurfaceTitle,
    min_width: u16,
    preferred_width: u16,
    max_width: Option<u16>,
    priority: u8,
    alignment: Alignment,
    overflow: OverflowPolicy,
    responsive: ResponsivePolicy,
    data_kind: DataKind,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ColumnWire {
    id: ColumnId,
    title: SurfaceTitle,
    min_width: u16,
    preferred_width: u16,
    max_width: Option<u16>,
    priority: u8,
    alignment: Alignment,
    overflow: OverflowPolicy,
    responsive: ResponsivePolicy,
    data_kind: DataKind,
}
impl TryFrom<ColumnWire> for ColumnSchema {
    type Error = ContractError;
    fn try_from(w: ColumnWire) -> Result<Self, Self::Error> {
        let widths = ColumnWidths::new(w.min_width, w.preferred_width, w.max_width)?;
        Ok(Self::new(w.id, w.title, w.data_kind)?.with_layout(
            widths,
            w.priority,
            w.alignment,
            w.overflow,
            w.responsive,
        ))
    }
}
impl ColumnSchema {
    pub fn new(
        id: ColumnId,
        title: SurfaceTitle,
        data_kind: DataKind,
    ) -> Result<Self, ContractError> {
        if title.as_str().trim().is_empty() {
            return Err(ContractError::Empty("column title"));
        }
        Ok(Self {
            id,
            title,
            min_width: 1,
            preferred_width: 16,
            max_width: None,
            priority: 0,
            alignment: Alignment::Left,
            overflow: OverflowPolicy::Ellipsis,
            responsive: ResponsivePolicy::Always,
            data_kind,
        })
    }
    pub fn with_layout(
        mut self,
        widths: ColumnWidths,
        priority: u8,
        alignment: Alignment,
        overflow: OverflowPolicy,
        responsive: ResponsivePolicy,
    ) -> Self {
        self.min_width = widths.min;
        self.preferred_width = widths.preferred;
        self.max_width = widths.max;
        self.priority = priority;
        self.alignment = alignment;
        self.overflow = overflow;
        self.responsive = responsive;
        self
    }
    pub fn id(&self) -> &ColumnId {
        &self.id
    }
    pub fn title(&self) -> &str {
        self.title.as_str()
    }
    pub fn widths(&self) -> ColumnWidths {
        ColumnWidths {
            min: self.min_width,
            preferred: self.preferred_width,
            max: self.max_width,
        }
    }
    pub fn priority(&self) -> u8 {
        self.priority
    }
    pub fn alignment(&self) -> Alignment {
        self.alignment
    }
    pub fn overflow(&self) -> OverflowPolicy {
        self.overflow
    }
    pub fn responsive(&self) -> ResponsivePolicy {
        self.responsive
    }
    pub fn data_kind(&self) -> DataKind {
        self.data_kind
    }
}
impl Charged for ColumnSchema {
    fn cost(&self) -> Cost {
        Cost {
            cells: 0,
            text: self.title.0.len() + self.id.0.len(),
        }
    }
}
type Columns = List<
    ColumnSchema,
    MAX_TABLE_COLUMNS,
    0,
    { MAX_TABLE_COLUMNS * (MAX_SURFACE_TITLE_BYTES + MAX_SURFACE_ID_BYTES) },
>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SchemaWire")]
pub struct TableSchema {
    id: TableSchemaId,
    columns: Columns,
    row_identity: RowIdentityPolicy,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaWire {
    id: TableSchemaId,
    columns: Columns,
    row_identity: RowIdentityPolicy,
}
impl TryFrom<SchemaWire> for TableSchema {
    type Error = ContractError;
    fn try_from(w: SchemaWire) -> Result<Self, Self::Error> {
        let result = Self {
            id: w.id,
            columns: w.columns,
            row_identity: w.row_identity,
        };
        result.validate()?;
        Ok(result)
    }
}
impl TableSchema {
    pub fn new(
        id: TableSchemaId,
        columns: Vec<ColumnSchema>,
    ) -> Result<Self, ContractError> {
        let result = Self {
            id,
            columns: Columns::new(columns)?,
            row_identity: RowIdentityPolicy::StableHandle,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), ContractError> {
        if self.columns().is_empty() {
            return Err(ContractError::Empty("table columns"));
        }
        let mut ids = BTreeSet::new();
        for c in self.columns() {
            if !ids.insert(c.id()) {
                return Err(ContractError::InvalidValue("duplicate column identifier"));
            }
        }
        Ok(())
    }
    pub fn id(&self) -> &TableSchemaId {
        &self.id
    }
    pub fn columns(&self) -> &[ColumnSchema] {
        self.columns.as_slice()
    }
    pub fn row_identity(&self) -> RowIdentityPolicy {
        self.row_identity
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct BasisPoints(u16);
impl BasisPoints {
    pub fn new(value: u16) -> Result<Self, ContractError> {
        Self::try_from(value)
    }
    pub fn get(self) -> u16 {
        self.0
    }
}
impl TryFrom<u16> for BasisPoints {
    type Error = ContractError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value > 10_000 {
            Err(ContractError::InvalidValue("percentage basis points"))
        } else {
            Ok(Self(value))
        }
    }
}
impl From<BasisPoints> for u16 {
    fn from(value: BasisPoints) -> Self {
        value.0
    }
}

/// Exact coefficient * 10^-scale; no binary floating-point rounding or NaN.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DecimalWire")]
pub struct Decimal {
    coefficient: i64,
    scale: u8,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecimalWire {
    coefficient: i64,
    scale: u8,
}
impl Decimal {
    pub fn new(coefficient: i64, scale: u8) -> Result<Self, ContractError> {
        if scale > 9 {
            return Err(ContractError::InvalidValue("decimal scale"));
        }
        Ok(Self { coefficient, scale })
    }
    pub fn coefficient(self) -> i64 {
        self.coefficient
    }
    pub fn scale(self) -> u8 {
        self.scale
    }
}
impl TryFrom<DecimalWire> for Decimal {
    type Error = ContractError;
    fn try_from(w: DecimalWire) -> Result<Self, Self::Error> {
        Self::new(w.coefficient, w.scale)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NumberValue {
    Signed(i64),
    Unsigned(u64),
    Decimal(Decimal),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusValue {
    pub label: CellText,
    pub severity: SemanticSeverity,
}

/// Externally tagged variants decode directly without an unbounded buffered map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CellValue {
    Missing,
    Text(CellText),
    Identifier(CellText),
    Status(StatusValue),
    Percentage(BasisPoints),
    Bytes(u64),
    Duration(u64),
    Timestamp(i64),
    Boolean(bool),
    Number(NumberValue),
}
impl CellValue {
    pub fn data_kind(&self) -> Option<DataKind> {
        Some(match self {
            Self::Missing => return None,
            Self::Text(_) => DataKind::Text,
            Self::Identifier(_) => DataKind::Identifier,
            Self::Status(_) => DataKind::Status,
            Self::Percentage(_) => DataKind::Percentage,
            Self::Bytes(_) => DataKind::Bytes,
            Self::Duration(_) => DataKind::Duration,
            Self::Timestamp(_) => DataKind::Timestamp,
            Self::Boolean(_) => DataKind::Boolean,
            Self::Number(_) => DataKind::Number,
        })
    }
}
impl Charged for CellValue {
    fn cost(&self) -> Cost {
        let text = match self {
            Self::Text(t) | Self::Identifier(t) => t.0.len(),
            Self::Status(s) => s.label.0.len(),
            _ => 0,
        };
        Cost { cells: 1, text }
    }
}
type Cells = List<
    CellValue,
    MAX_TABLE_COLUMNS,
    MAX_TABLE_COLUMNS,
    { MAX_TABLE_COLUMNS * MAX_CELL_TEXT_BYTES },
>;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableRow {
    id: RowId,
    resource: Option<ResourceHandle>,
    cells: Cells,
}
impl TableRow {
    pub fn new(
        id: RowId,
        resource: Option<ResourceHandle>,
        cells: Vec<CellValue>,
    ) -> Result<Self, ContractError> {
        Ok(Self {
            id,
            resource,
            cells: Cells::new(cells)?,
        })
    }
    pub fn id(&self) -> RowId {
        self.id
    }
    pub fn resource(&self) -> Option<ResourceHandle> {
        self.resource
    }
    pub fn cells(&self) -> &[CellValue] {
        self.cells.as_slice()
    }
}
impl Charged for TableRow {
    fn cost(&self) -> Cost {
        self.cells.cost()
    }
}
type Rows = List<TableRow, MAX_TABLE_ROWS, MAX_TABLE_CELLS, MAX_TABLE_TEXT_BYTES>;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TableWire")]
pub struct SemanticTable {
    schema: TableSchema,
    rows: Rows,
}
impl fmt::Debug for SemanticTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Diagnostics must not traverse thousands of rows or reveal local data.
        f.debug_struct("SemanticTable")
            .field("columns", &self.schema.columns().len())
            .field("rows", &self.rows().len())
            .finish_non_exhaustive()
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TableWire {
    schema: TableSchema,
    rows: Rows,
}
impl TryFrom<TableWire> for SemanticTable {
    type Error = ContractError;
    fn try_from(w: TableWire) -> Result<Self, Self::Error> {
        let result = Self {
            schema: w.schema,
            rows: w.rows,
        };
        result.validate()?;
        Ok(result)
    }
}
impl SemanticTable {
    pub fn new(schema: TableSchema, rows: Vec<TableRow>) -> Result<Self, ContractError> {
        let result = Self {
            schema,
            rows: Rows::new(rows)?,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), ContractError> {
        let mut ids = BTreeSet::new();
        for row in self.rows() {
            if !ids.insert(row.id()) {
                return Err(ContractError::InvalidValue("duplicate row identifier"));
            }
            if row.cells().len() != self.schema.columns().len() {
                return Err(ContractError::InvalidValue("table row shape"));
            }
            for (cell, column) in row.cells().iter().zip(self.schema.columns()) {
                if cell
                    .data_kind()
                    .is_some_and(|kind| kind != column.data_kind())
                {
                    return Err(ContractError::InvalidValue("table cell kind"));
                }
            }
        }
        Ok(())
    }
    pub fn schema(&self) -> &TableSchema {
        &self.schema
    }
    pub fn rows(&self) -> &[TableRow] {
        self.rows.as_slice()
    }
    pub fn cell_count(&self) -> usize {
        self.rows.cost().cells
    }
    pub fn text_bytes(&self) -> usize {
        self.rows.cost().text
    }
}

/// Untrusted identity claims; compare with the host's independent route/grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceBinding {
    pub extension: ExtensionId,
    pub session: SessionId,
    pub capsule_revision: u64,
    pub operation: OperationId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceEvent {
    Loading,
    Replace(SemanticTable),
    Failed(SurfaceTitle),
    Close,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceUpdate {
    version: ContractVersion,
    binding: SurfaceBinding,
    surface: SemanticSurfaceId,
    generation: NonZeroU64,
    revision: NonZeroU64,
    event: SurfaceEvent,
}
impl SurfaceUpdate {
    pub fn new(
        binding: SurfaceBinding,
        surface: SemanticSurfaceId,
        generation: u64,
        revision: u64,
        event: SurfaceEvent,
    ) -> Result<Self, ContractError> {
        Ok(Self {
            version: ContractVersion::CURRENT,
            binding,
            surface,
            generation: NonZeroU64::new(generation)
                .ok_or(ContractError::InvalidValue("surface generation"))?,
            revision: NonZeroU64::new(revision)
                .ok_or(ContractError::InvalidValue("surface revision"))?,
            event,
        })
    }
    pub fn binding(&self) -> &SurfaceBinding {
        &self.binding
    }
    pub fn surface(&self) -> SemanticSurfaceId {
        self.surface
    }
    pub fn generation(&self) -> u64 {
        self.generation.get()
    }
    pub fn revision(&self) -> u64 {
        self.revision.get()
    }
    pub fn into_event(self) -> SurfaceEvent {
        self.event
    }
}
