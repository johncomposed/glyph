use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, string::String, vec::Vec};

#[cfg(feature = "compression")]
use std::io::Write;

use crate::error::{BakeError, BakeErrorCode};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BakeDescriptorV0 {
    pub format_version: u8,
    pub font_face_index: u32,
    /// Static variation instance to bake. Omitted or empty means the `fvar` default instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variation: Option<VariationRequestV0>,
}

/// Caller-authored variation instance: user-space axis values keyed by four-byte `fvar` tag.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariationRequestV0 {
    pub axes: BTreeMap<String, f32>,
}

/// The fixed variation instance a baked font represents. Present only for a variable source.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariationV0 {
    /// Clamped user-space value of every `fvar` axis, keyed by tag.
    pub axes: BTreeMap<String, f32>,
    /// Normalized F2Dot14 bit patterns in `fvar` axis order; HarfRust and Skrifa consume these directly.
    pub coordinates: Vec<i16>,
}

impl BakeDescriptorV0 {
    pub fn new(font_face_index: u32) -> Self {
        Self {
            format_version: 0,
            font_face_index,
            variation: None,
        }
    }

    pub fn with_variation(mut self, axes: impl IntoIterator<Item = (String, f32)>) -> Self {
        self.variation = Some(VariationRequestV0 {
            axes: axes.into_iter().collect(),
        });
        self
    }

    pub(crate) fn validate(self) -> Result<Self, BakeError> {
        if self.format_version != 0 {
            return Err(BakeError::new(
                BakeErrorCode::InvalidDescriptor,
                format!(
                    "unsupported font bake format version {}",
                    self.format_version
                ),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BakeResultV0 {
    pub artifacts: Vec<BakeArtifactV0>,
    pub report: BakeReportV0,
    pub warnings: Vec<BakeWarning>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BakeArtifactV0 {
    pub role: String,
    pub id: String,
    #[serde(skip)]
    pub bytes: Vec<u8>,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BakeWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontMetricsV0 {
    pub glyph_count: u16,
    pub glyph_id_width: u8,
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub underline_position: i16,
    pub underline_thickness: i16,
    pub strikeout_position: i16,
    pub strikeout_size: i16,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceV0 {
    pub source_fingerprint: String,
    pub font_face_index: u32,
    pub baker_version: String,
    pub harfrust_version: String,
    pub harfbuzz_reference_version: String,
    pub unicode_version: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TablePayloadReport {
    pub tag: String,
    pub raw_bytes: usize,
    pub padded_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapingPayloadReportV0 {
    pub format: String,
    pub sfnt_directory_bytes: usize,
    pub tables: Vec<TablePayloadReport>,
    pub extents_bytes: usize,
    pub extents_availability_bytes: usize,
    pub total_raw_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gzip_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brotli_bytes: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BakeReportV0 {
    pub source: SourcePayloadReport,
    pub shared: SharedPayloadReport,
    pub rasters: Vec<serde_json::Value>,
    pub containers: Vec<ContainerPayloadReport>,
    pub transport: Vec<TransportPayloadReport>,
}

#[derive(Debug, Serialize)]
pub struct SourcePayloadReport {
    pub bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct SharedPayloadReport {
    pub shaping: ShapingPayloadReportV0,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPayloadReport {
    pub artifact_id: String,
    pub role: String,
    pub json_bytes: usize,
    pub padding_bytes: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportPayloadReport {
    pub artifact_id: String,
    pub format: String,
    pub bytes: usize,
}

pub(crate) fn compressed_lengths(bytes: &[u8]) -> Result<Option<(usize, usize)>, BakeError> {
    #[cfg(not(feature = "compression"))]
    {
        let _ = bytes;
        Ok(None)
    }

    #[cfg(feature = "compression")]
    {
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        gzip.write_all(bytes).map_err(compression_error)?;
        let gzip_len = gzip.finish().map_err(compression_error)?.len();

        let mut brotli = Vec::new();
        {
            let mut writer = brotli::CompressorWriter::new(&mut brotli, 4096, 11, 22);
            writer.write_all(bytes).map_err(compression_error)?;
        }
        Ok(Some((gzip_len, brotli.len())))
    }
}

#[cfg(feature = "compression")]
fn compression_error(error: std::io::Error) -> BakeError {
    BakeError::new(
        BakeErrorCode::SerializationFailed,
        format!("failed to measure transport compression: {error}"),
    )
}
