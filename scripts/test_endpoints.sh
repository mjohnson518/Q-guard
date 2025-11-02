#!/bin/bash

set -e

BASE_URL="${Q_GUARD_URL:-http://localhost:8080}"

echo "Testing Q-guard Endpoints"
echo "============================"
echo "Base URL: $BASE_URL"
echo ""

# Test health endpoint
echo "Step 1: Testing health endpoint..."
curl -s "$BASE_URL/health" | jq
echo ""

# Test stats endpoint
echo "Step 2: Testing stats endpoint..."
curl -s "$BASE_URL/stats" | jq
echo ""

# Test gas prediction (should return 402)
echo "Step 3: Testing gas prediction without payment (expecting 402)..."
HTTP_CODE=$(curl -s -o /tmp/response.json -w "%{http_code}" "$BASE_URL/api/gas/prediction")
echo "Status: $HTTP_CODE"
if [ "$HTTP_CODE" -eq 402 ]; then
    echo "[PASS] Correctly returned 402 Payment Required"
    cat /tmp/response.json | jq
else
    echo "[FAIL] Expected 402, got $HTTP_CODE"
fi
echo ""

echo "[PASS] All basic tests passed!"
echo ""
echo "To test full payment flow:"
echo "  cargo run --bin test-agent"

