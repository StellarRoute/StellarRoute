//! Response compression helpers for quote payloads.

use axum::{
    body::Body,
    http::{
        header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, VARY},
        HeaderValue, Response,
    },
};
use flate2::{write::GzEncoder, Compression};
use serde::Serialize;
use std::io::Write;

use crate::error::{ApiError, Result};

const MIN_COMPRESS_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseEncoding {
    Brotli,
    Gzip,
    Identity,
}

impl ResponseEncoding {
    fn label(self) -> &'static str {
        match self {
            Self::Brotli => "br",
            Self::Gzip => "gzip",
            Self::Identity => "identity",
        }
    }

    fn header_value(self) -> Option<&'static str> {
        match self {
            Self::Brotli => Some("br"),
            Self::Gzip => Some("gzip"),
            Self::Identity => None,
        }
    }
}

pub fn json_response<T: Serialize>(
    value: &T,
    accept_encoding: Option<&HeaderValue>,
) -> Result<Response<Body>> {
    let json = serde_json::to_vec(value).map_err(|err| {
        ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
            "failed to serialize JSON response: {err}"
        )))
    })?;

    let original_len = json.len();
    let encoding = choose_encoding(accept_encoding, original_len);
    let body = match encoding {
        ResponseEncoding::Brotli => brotli_compress(&json)?,
        ResponseEncoding::Gzip => gzip_compress(&json)?,
        ResponseEncoding::Identity => json,
    };

    crate::metrics::record_quote_response_bytes(encoding.label(), original_len, body.len());

    let mut builder = Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, body.len().to_string())
        .header(VARY, ACCEPT_ENCODING.as_str());

    if let Some(value) = encoding.header_value() {
        builder = builder.header(CONTENT_ENCODING, value);
    }

    builder.body(Body::from(body)).map_err(|err| {
        ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
            "failed to build JSON response: {err}"
        )))
    })
}

fn choose_encoding(accept_encoding: Option<&HeaderValue>, body_len: usize) -> ResponseEncoding {
    if body_len < MIN_COMPRESS_BYTES {
        return ResponseEncoding::Identity;
    }

    let Some(header) = accept_encoding.and_then(|value| value.to_str().ok()) else {
        return ResponseEncoding::Identity;
    };

    let br = coding_quality(header, "br");
    let gzip = coding_quality(header, "gzip");

    if br > 0.0 && br >= gzip {
        ResponseEncoding::Brotli
    } else if gzip > 0.0 {
        ResponseEncoding::Gzip
    } else {
        ResponseEncoding::Identity
    }
}

fn coding_quality(header: &str, coding: &str) -> f32 {
    header
        .split(',')
        .filter_map(|item| {
            let (token, q) = parse_coding(item)?;
            if token.eq_ignore_ascii_case(coding) || token == "*" {
                Some(q)
            } else {
                None
            }
        })
        .fold(0.0, f32::max)
}

fn parse_coding(item: &str) -> Option<(&str, f32)> {
    let mut parts = item.trim().split(';');
    let token = parts.next().unwrap_or("").trim();
    if token.is_empty() {
        return None;
    }

    let mut quality = 1.0;
    for param in parts {
        let param = param.trim();
        if let Some(value) = param.strip_prefix("q=") {
            quality = value.parse::<f32>().unwrap_or(0.0).clamp(0.0, 1.0);
        }
    }

    Some((token, quality))
}

fn gzip_compress(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).map_err(|err| {
        ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
            "failed to gzip response: {err}"
        )))
    })?;
    encoder.finish().map_err(|err| {
        ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
            "failed to finish gzip response: {err}"
        )))
    })
}

fn brotli_compress(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut output, 4096, 5, 22);
        writer.write_all(bytes).map_err(|err| {
            ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
                "failed to brotli-compress response: {err}"
            )))
        })?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::CONTENT_ENCODING;
    use serde_json::json;

    #[test]
    fn small_payload_uses_identity_even_when_compression_is_accepted() {
        let response = json_response(
            &json!({"ok": true}),
            Some(&HeaderValue::from_static("br, gzip")),
        )
        .expect("response");

        assert!(response.headers().get(CONTENT_ENCODING).is_none());
    }

    #[test]
    fn brotli_is_preferred_for_large_payloads() {
        let response = json_response(
            &json!({"data": "x".repeat(MIN_COMPRESS_BYTES + 1)}),
            Some(&HeaderValue::from_static("gzip, br")),
        )
        .expect("response");

        assert_eq!(
            response.headers().get(CONTENT_ENCODING),
            Some(&HeaderValue::from_static("br"))
        );
    }

    #[test]
    fn gzip_is_used_when_brotli_is_not_accepted() {
        let response = json_response(
            &json!({"data": "x".repeat(MIN_COMPRESS_BYTES + 1)}),
            Some(&HeaderValue::from_static("gzip")),
        )
        .expect("response");

        assert_eq!(
            response.headers().get(CONTENT_ENCODING),
            Some(&HeaderValue::from_static("gzip"))
        );
    }

    #[test]
    fn q_zero_disables_a_coding() {
        assert_eq!(
            choose_encoding(
                Some(&HeaderValue::from_static("br;q=0, gzip")),
                MIN_COMPRESS_BYTES + 1
            ),
            ResponseEncoding::Gzip
        );
    }

    #[test]
    fn explicit_quality_weights_are_respected() {
        assert_eq!(
            choose_encoding(
                Some(&HeaderValue::from_static("br;q=0.2, gzip;q=0.9")),
                MIN_COMPRESS_BYTES + 1
            ),
            ResponseEncoding::Gzip
        );
    }
}
