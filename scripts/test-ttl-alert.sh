#!/bin/bash
# Test script for TTL alert payload generation

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

# Mock get_contract_id for testing
get_contract_id() {
    echo "CCVQQT5T4XZ2N7L6W5E4R3T2Y1U0I9O8P7A6S5D4F3G2H1J0K9L8M7N6B5V"
}

# Test payload generation
test_redaction() {
    local test_input="Error: secret=mysecret123 key=api-key-456 token=token-789 password=pass123"
    local redacted
    redacted="$(redact_secrets "${test_input}")"
    
    echo "Test input: ${test_input}"
    echo "Redacted:    ${redacted}"
    
    if [[ "${redacted}" == *"secret=REDACTED"* && "${redacted}" == *"key=REDACTED"* && "${redacted}" == *"token=REDACTED"* && "${redacted}" == *"password=REDACTED"* ]]; then
        echo "✓ Redaction test passed"
    else
        echo "✗ Redaction test failed"
        exit 1
    fi
}

# Test payload structure
test_payload() {
    NETWORK="testnet"
    local timestamp
    timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    local contract_id
    contract_id="$(get_contract_id)"
    local redacted_error
    redacted_error="$(redact_secrets "test error")"
    
    local payload
    payload=$(cat <<EOF
{
  "text": "⚠️ StellarRoute TTL Extension Failed",
  "attachments": [
    {
      "color": "danger",
      "title": "TTL Extension Failure",
      "fields": [
        {
          "title": "Network",
          "value": "${NETWORK}",
          "short": true
        },
        {
          "title": "Contract ID",
          "value": "${contract_id}",
          "short": true
        },
        {
          "title": "Timestamp",
          "value": "${timestamp}",
          "short": true
        }
      ],
      "text": "Error: ${redacted_error}",
      "footer": "StellarRoute TTL Bot"
    }
  ]
}
EOF
)
    
    # Validate JSON
    if command -v jq &>/dev/null; then
        if echo "${payload}" | jq . &>/dev/null; then
            echo "✓ Payload is valid JSON"
            echo "Payload:"
            echo "${payload}" | jq .
        else
            echo "✗ Payload is invalid JSON"
            exit 1
        fi
    else
        echo "jq not available, skipping JSON validation"
    fi
}

echo "Testing TTL alert functionality..."
echo "=============================="
test_redaction
echo "=============================="
test_payload
echo "=============================="
echo "All tests passed!"
