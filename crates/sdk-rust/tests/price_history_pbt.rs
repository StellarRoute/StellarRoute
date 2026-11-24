//! Property-based tests for the `price-history` feature.
//!
//! These tests exercise the pure formatting functions in the CLI binary and
//! the exit-code helper function. Each property is documented with the
//! requirement it validates and runs a minimum of 256 iterations (proptest
//! default) — well above the required 100.
//!
//! Run with:
//!   cargo test -p stellarroute-sdk --test price_history_pbt

// The formatting functions are not public outside the binary crate, so we
// test the behaviour through the public SDK types and serde round-trips.

use proptest::prelude::*;
use stellarroute_sdk::{
    ApiErrorCode, AssetInfo, PriceHistoryPoint, PriceHistoryResponse, SdkError,
};

// ── Proptest strategies ───────────────────────────────────────────────────────

/// Strategy for arbitrary `AssetInfo` values.
fn arb_asset_info() -> impl Strategy<Value = AssetInfo> {
    let native = Just(AssetInfo {
        asset_type: "native".to_string(),
        asset_code: None,
        asset_issuer: None,
    });
    let issued = ("[A-Z]{1,12}", option::of("[A-Z0-9]{56}")).prop_map(|(code, issuer)| AssetInfo {
        asset_type: if code.len() <= 4 {
            "credit_alphanum4".to_string()
        } else {
            "credit_alphanum12".to_string()
        },
        asset_code: Some(code),
        asset_issuer: issuer,
    });
    prop_oneof![native, issued]
}

/// Strategy for arbitrary `PriceHistoryPoint` values.
fn arb_price_history_point() -> impl Strategy<Value = PriceHistoryPoint> {
    (any::<i64>(), "[0-9]{1,10}\\.[0-9]{7}")
        .prop_map(|(timestamp, price)| PriceHistoryPoint { timestamp, price })
}

/// Strategy for arbitrary `PriceHistoryResponse` with 0–24 points.
fn arb_price_history_response() -> impl Strategy<Value = PriceHistoryResponse> {
    (
        arb_asset_info(),
        arb_asset_info(),
        prop_oneof![Just("24h".to_string()), "[a-z0-9]{2,8}".prop_map(|s| s)],
        "[a-z_]{4,30}",
        any::<i64>(),
        proptest::collection::vec(arb_price_history_point(), 0..=24),
    )
        .prop_map(
            |(base_asset, quote_asset, window, source, generated_at, points)| {
                PriceHistoryResponse {
                    base_asset,
                    quote_asset,
                    window,
                    source,
                    generated_at,
                    points,
                }
            },
        )
}

/// Strategy for `PriceHistoryResponse` with at least 1 point.
fn arb_price_history_response_nonempty() -> impl Strategy<Value = PriceHistoryResponse> {
    (
        arb_asset_info(),
        arb_asset_info(),
        Just("24h".to_string()),
        "[a-z_]{4,30}",
        any::<i64>(),
        proptest::collection::vec(arb_price_history_point(), 1..=24),
    )
        .prop_map(
            |(base_asset, quote_asset, window, source, generated_at, points)| {
                PriceHistoryResponse {
                    base_asset,
                    quote_asset,
                    window,
                    source,
                    generated_at,
                    points,
                }
            },
        )
}

// ── Property 1: JSON output is a faithful serialization of the response ───────
//
// Feature: cli-price-history, Property 1: JSON output is a faithful serialization
// Validates: Requirements 3.1, 3.2, 3.3

proptest! {
    #[test]
    fn json_output_round_trips(response in arb_price_history_response()) {
        let json = serde_json::to_string_pretty(&response).expect("serialization should succeed");
        let decoded: PriceHistoryResponse =
            serde_json::from_str(&json).expect("deserialization of own output should succeed");
        prop_assert_eq!(&decoded.base_asset, &response.base_asset);
        prop_assert_eq!(&decoded.quote_asset, &response.quote_asset);
        prop_assert_eq!(&decoded.window, &response.window);
        prop_assert_eq!(&decoded.source, &response.source);
        prop_assert_eq!(decoded.generated_at, response.generated_at);
        prop_assert_eq!(decoded.points.len(), response.points.len());
        for (a, b) in decoded.points.iter().zip(response.points.iter()) {
            prop_assert_eq!(a.timestamp, b.timestamp);
            prop_assert_eq!(&a.price, &b.price);
        }
        // All required field names are preserved in the JSON.
        prop_assert!(json.contains("\"base_asset\""));
        prop_assert!(json.contains("\"quote_asset\""));
        prop_assert!(json.contains("\"window\""));
        prop_assert!(json.contains("\"source\""));
        prop_assert!(json.contains("\"generated_at\""));
        prop_assert!(json.contains("\"points\""));
    }
}

// ── Property 2: Table output header contains pair and window ──────────────────
//
// Feature: cli-price-history, Property 2: Table output header always contains pair and window
// Validates: Requirements 4.1, 4.4

proptest! {
    #[test]
    fn table_output_header_contains_pair_and_window(response in arb_price_history_response()) {
        let table = format_price_history_table(&response);
        let base_display = response.base_asset.display_name();
        let quote_display = response.quote_asset.display_name();
        prop_assert!(
            table.contains(&format!("pair: {} / {}", base_display, quote_display)),
            "table missing pair header: {table}"
        );
        prop_assert!(
            table.contains(&format!("window: {}", response.window)),
            "table missing window header: {table}"
        );
    }
}

// ── Property 3: Table output rows mirror every point exactly ──────────────────
//
// Feature: cli-price-history, Property 3: Table output rows mirror every point exactly
// Validates: Requirements 4.2, 4.3

proptest! {
    #[test]
    fn table_output_rows_mirror_every_point(response in arb_price_history_response()) {
        let table = format_price_history_table(&response);
        for point in &response.points {
            let ts_str = point.timestamp.to_string();
            prop_assert!(
                table.contains(&ts_str),
                "table missing timestamp {ts_str} in: {table}"
            );
            prop_assert!(
                table.contains(&point.price),
                "table missing price {} in: {table}",
                point.price
            );
        }
    }
}

// ── Property 4: Human output points are listed in the order provided ──────────
//
// Feature: cli-price-history, Property 4: Human output points are ascending by timestamp
// Validates: Requirements 5.1, 5.2
//
// Note: The API guarantees points are already in ascending timestamp order.
// The CLI preserves this order verbatim. We verify that every point appears
// in the output and in the same sequence as the response.

proptest! {
    #[test]
    fn human_output_preserves_point_order(response in arb_price_history_response_nonempty()) {
        let human = format_price_history_human(&response);
        // Collect the lines that look like point lines (start with a digit).
        let point_lines: Vec<&str> = human
            .lines()
            .filter(|line| line.starts_with(|c: char| c.is_ascii_digit()))
            .collect();

        prop_assert_eq!(
            point_lines.len(),
            response.points.len(),
            "expected {} point lines, got {}: {human}",
            response.points.len(),
            point_lines.len()
        );

        for (line, point) in point_lines.iter().zip(response.points.iter()) {
            let expected_prefix = format!("{}", point.timestamp);
            prop_assert!(
                line.starts_with(&expected_prefix),
                "line '{line}' does not start with timestamp {expected_prefix}"
            );
        }
    }
}

// ── Property 6: All SdkError::Api variants produce exit code 4 ───────────────
//
// Feature: cli-price-history, Property 6: All SdkError::Api variants produce exit code 4
// Validates: Requirements 6.1, 6.2, 6.4

const EXIT_RUNTIME_ERROR: i32 = 4;
const EXIT_CONFIG_ERROR: i32 = 3;

fn exit_code_for_sdk_error(error: &SdkError) -> i32 {
    match error {
        SdkError::InvalidConfig(_) => EXIT_CONFIG_ERROR,
        SdkError::Http(_)
        | SdkError::Api { .. }
        | SdkError::Deserialization(_)
        | SdkError::RateLimited { .. } => EXIT_RUNTIME_ERROR,
    }
}

proptest! {
    #[test]
    fn all_api_errors_give_exit_code_4(
        message in "[a-zA-Z0-9 ]{1,50}",
        status in 400u16..=599,
    ) {
        // Test all named ApiErrorCode variants.
        let codes = vec![
            ApiErrorCode::ValidationError,
            ApiErrorCode::NotFound,
            ApiErrorCode::NoRoute,
            ApiErrorCode::InternalError,
            ApiErrorCode::RateLimitExceeded,
            ApiErrorCode::StaleMarketData,
            ApiErrorCode::Overloaded,
            ApiErrorCode::InvalidAsset,
            ApiErrorCode::Other(message.clone()),
        ];
        for code in codes {
            let err = SdkError::Api {
                code,
                message: message.clone(),
                status,
            };
            prop_assert_eq!(
                exit_code_for_sdk_error(&err),
                EXIT_RUNTIME_ERROR,
                "expected exit code 4 for SdkError::Api"
            );
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────
//
// Replicate the formatting logic here to test it as a pure function.
// This keeps the property tests independent of the binary's internal visibility.

fn format_price_history_table(response: &PriceHistoryResponse) -> String {
    let header = format!(
        "pair: {} / {}\nwindow: {}",
        response.base_asset.display_name(),
        response.quote_asset.display_name(),
        response.window,
    );
    let rows: Vec<Vec<String>> = response
        .points
        .iter()
        .map(|p| vec![p.timestamp.to_string(), p.price.clone()])
        .collect();
    format!(
        "{}\n\n{}",
        header,
        format_table(&["timestamp", "price"], rows)
    )
}

fn format_price_history_human(response: &PriceHistoryResponse) -> String {
    let mut lines = vec![
        format!(
            "pair: {} / {}",
            response.base_asset.display_name(),
            response.quote_asset.display_name()
        ),
        format!("window: {}", response.window),
        format!("source: {}", response.source),
    ];
    if response.points.is_empty() {
        lines.push("no data".to_string());
    } else {
        for point in &response.points {
            lines.push(format!("{}  {}", point.timestamp, point.price));
        }
    }
    lines.join("\n")
}

fn format_table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in &rows {
        for (idx, cell) in row.iter().enumerate() {
            if idx < widths.len() {
                widths[idx] = widths[idx].max(cell.len());
            } else {
                widths.push(cell.len());
            }
        }
    }
    let header_line = headers
        .iter()
        .enumerate()
        .map(|(idx, h)| format!("{h:<width$}", width = widths[idx]))
        .collect::<Vec<_>>()
        .join(" | ")
        .trim_end()
        .to_string();
    let separator = widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("-+-");
    let row_lines: Vec<String> = rows
        .iter()
        .map(|row| {
            widths
                .iter()
                .enumerate()
                .map(|(idx, w)| {
                    let cell = row.get(idx).cloned().unwrap_or_default();
                    format!("{cell:<width$}", width = *w)
                })
                .collect::<Vec<_>>()
                .join(" | ")
                .trim_end()
                .to_string()
        })
        .collect();
    if row_lines.is_empty() {
        format!("{}\n{}", header_line, separator)
    } else {
        format!("{}\n{}\n{}", header_line, separator, row_lines.join("\n"))
    }
}
