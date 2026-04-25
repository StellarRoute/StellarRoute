use stellarroute_api::models::response::QuoteResponse;

#[test]
fn test_quote_response_conformance() {
    let json = r#"{
        "base_asset": {
            "asset_type": "native",
            "asset_code": null,
            "asset_issuer": null
        },
        "quote_asset": {
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC",
            "asset_issuer": "GBBD67SIWK6V6I7SGPW76BGSYDBBCOOT6YF7KOCUT5NJSWJRXFNY6X3K"
        },
        "amount": "100.0000000",
        "price": "1.0000000",
        "total": "100.0000000",
        "quote_type": "sell",
        "path": [
            {
                "from_asset": {
                    "asset_type": "native",
                    "asset_code": null,
                    "asset_issuer": null
                },
                "to_asset": {
                    "asset_type": "credit_alphanum4",
                    "asset_code": "USDC",
                    "asset_issuer": "GBBD67SIWK6V6I7SGPW76BGSYDBBCOOT6YF7KOCUT5NJSWJRXFNY6X3K"
                },
                "price": "1.0000000",
                "source": "sdex"
            }
        ],
        "timestamp": 1700000000000,
        "expires_at": 1700000030000,
        "ttl_seconds": 30,
        "price_impact": "0.01"
    }"#;

    let response: QuoteResponse =
        serde_json::from_str(json).expect("Failed to deserialize reference quote response");

    // Validate required fields
    assert_eq!(response.base_asset.display_name(), "XLM");
    assert_eq!(response.quote_asset.display_name(), "USDC");
    assert_eq!(response.amount, "100.0000000");
    assert_eq!(response.path.len(), 1);
    assert_eq!(response.timestamp, 1700000000000);
    assert!(response.price_impact.is_some());
}

#[test]
fn test_quote_response_minimal_conformance() {
    let json = r#"{
        "base_asset": {"asset_type": "native"},
        "quote_asset": {"asset_type": "native"},
        "amount": "1",
        "price": "1",
        "total": "1",
        "quote_type": "sell",
        "path": [],
        "timestamp": 1700000000000
    }"#;

    let response: QuoteResponse =
        serde_json::from_str(json).expect("Failed to deserialize minimal quote response");
    assert_eq!(response.amount, "1");
}

#[test]
fn test_quote_response_missing_fields_fail() {
    let json = r#"{
        "amount": "1"
    }"#;

    let result: Result<QuoteResponse, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "Should have failed due to missing required fields"
    );
}
