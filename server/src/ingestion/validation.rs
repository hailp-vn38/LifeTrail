use axum::http::HeaderMap;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

pub(super) struct ValidatedBatch {
    pub batch_id: Uuid,
    pub hash: Vec<u8>,
    pub byte_length: u64,
    pub records: Vec<GpsRecord>,
}

impl ValidatedBatch {
    pub fn record_count(&self) -> u64 {
        self.records.len() as u64
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GpsRecord {
    pub ts_ms: i64,
    pub lat: f64,
    pub lon: f64,
    pub alt_m: Option<f64>,
    pub speed_mps: Option<f64>,
    pub course_deg: Option<f64>,
    pub fix_quality: i64,
    pub satellites: i64,
    pub hdop: Option<f64>,
}

pub(super) struct Metadata {
    batch_id: Uuid,
    hash: String,
    byte_length: u64,
    record_count: u64,
}

pub(super) struct ValidationError {
    line: Option<usize>,
    field: Option<&'static str>,
    reason: &'static str,
}

impl ValidationError {
    fn batch(reason: &'static str) -> Self {
        Self {
            line: None,
            field: None,
            reason,
        }
    }

    fn record(line: usize, field: &'static str, reason: &'static str) -> Self {
        Self {
            line: Some(line),
            field: Some(field),
            reason,
        }
    }

    pub fn details(&self) -> Option<Value> {
        let mut details = serde_json::Map::new();
        if let Some(line) = self.line {
            details.insert("line".to_owned(), json!(line));
        }
        if let Some(field) = self.field {
            details.insert("field".to_owned(), json!(field));
        }
        details.insert("reason".to_owned(), json!(self.reason));
        Some(Value::Object(details))
    }
}

pub(super) fn parse_metadata(headers: &HeaderMap) -> Result<Metadata, ValidationError> {
    required_ndjson(headers)?;
    let batch_id = header(headers, "x-lifetrail-batch-id")?;
    let parsed_batch_id =
        Uuid::parse_str(batch_id).map_err(|_| ValidationError::batch("invalid_batch_id"))?;
    if parsed_batch_id.get_version_num() != 4
        || parsed_batch_id.hyphenated().to_string() != batch_id
    {
        return Err(ValidationError::batch("invalid_batch_id"));
    }
    if header(headers, "x-lifetrail-schema")? != "gps/1" {
        return Err(ValidationError::batch("unsupported_schema"));
    }
    let hash = header(headers, "x-lifetrail-content-sha256")?;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ValidationError::batch("invalid_sha256"));
    }
    Ok(Metadata {
        batch_id: parsed_batch_id,
        hash: hash.to_owned(),
        byte_length: unsigned_header(headers, "x-lifetrail-byte-length")?,
        record_count: unsigned_header(headers, "x-lifetrail-record-count")?,
    })
}

pub(super) fn validate(
    metadata: Metadata,
    bytes: Vec<u8>,
) -> Result<ValidatedBatch, ValidationError> {
    if bytes.len() as u64 != metadata.byte_length {
        return Err(ValidationError::batch("byte_length_mismatch"));
    }
    let actual_hash = format!("{:x}", Sha256::digest(&bytes));
    if actual_hash != metadata.hash {
        return Err(ValidationError::batch("sha256_mismatch"));
    }
    if bytes.is_empty() || bytes.last() != Some(&b'\n') || bytes.contains(&b'\r') {
        return Err(ValidationError::batch("invalid_ndjson_framing"));
    }
    let mut records = Vec::new();
    let mut previous_ts_ms = None;
    for (index, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        let line_number = index + 1;
        if line.is_empty() {
            return Err(ValidationError::record(line_number, "body", "blank_line"));
        }
        let record: GpsRecord = serde_json::from_slice(line)
            .map_err(|_| ValidationError::record(line_number, "body", "invalid_record"))?;
        validate_record(&record, line_number, previous_ts_ms)?;
        previous_ts_ms = Some(record.ts_ms);
        records.push(record);
    }
    if records.len() as u64 != metadata.record_count {
        return Err(ValidationError::batch("record_count_mismatch"));
    }
    Ok(ValidatedBatch {
        batch_id: metadata.batch_id,
        hash: Sha256::digest(&bytes).to_vec(),
        byte_length: metadata.byte_length,
        records,
    })
}

fn required_ndjson(headers: &HeaderMap) -> Result<(), ValidationError> {
    let value = headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ValidationError::batch("missing_content_type"))?;
    if value != "application/x-ndjson" {
        return Err(ValidationError::batch("invalid_content_type"));
    }
    Ok(())
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, ValidationError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ValidationError::batch("missing_required_header"))
}

fn unsigned_header(headers: &HeaderMap, name: &str) -> Result<u64, ValidationError> {
    let value = header(headers, name)?;
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ValidationError::batch("invalid_header_value"));
    }
    value
        .parse()
        .map_err(|_| ValidationError::batch("invalid_header_value"))
}

fn validate_record(
    record: &GpsRecord,
    line: usize,
    previous_ts_ms: Option<i64>,
) -> Result<(), ValidationError> {
    if record.ts_ms <= 0 {
        return Err(ValidationError::record(line, "ts_ms", "must_be_positive"));
    }
    if previous_ts_ms.is_some_and(|previous| record.ts_ms <= previous) {
        return Err(ValidationError::record(
            line,
            "ts_ms",
            "not_strictly_increasing",
        ));
    }
    range(line, "lat", record.lat, -90.0, 90.0)?;
    range(line, "lon", record.lon, -180.0, 180.0)?;
    if record.fix_quality < 0 {
        return Err(ValidationError::record(
            line,
            "fix_quality",
            "must_be_non_negative",
        ));
    }
    if record.satellites < 0 {
        return Err(ValidationError::record(
            line,
            "satellites",
            "must_be_non_negative",
        ));
    }
    optional_non_negative(line, "speed_mps", record.speed_mps)?;
    optional_non_negative(line, "hdop", record.hdop)?;
    if let Some(course_deg) = record.course_deg
        && (!course_deg.is_finite() || !(0.0..360.0).contains(&course_deg))
    {
        return Err(ValidationError::record(line, "course_deg", "out_of_range"));
    }
    if let Some(alt_m) = record.alt_m
        && !alt_m.is_finite()
    {
        return Err(ValidationError::record(line, "alt_m", "not_finite"));
    }
    Ok(())
}

fn range(
    line: usize,
    field: &'static str,
    value: f64,
    minimum: f64,
    maximum: f64,
) -> Result<(), ValidationError> {
    if !value.is_finite() || !(minimum..=maximum).contains(&value) {
        return Err(ValidationError::record(line, field, "out_of_range"));
    }
    Ok(())
}

fn optional_non_negative(
    line: usize,
    field: &'static str,
    value: Option<f64>,
) -> Result<(), ValidationError> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(ValidationError::record(line, field, "must_be_non_negative"));
    }
    Ok(())
}
