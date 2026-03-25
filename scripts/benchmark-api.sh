#!/usr/bin/env bash
set -euo pipefail

API="http://localhost:3001"

echo "PredicTerm API Benchmark"
echo "========================"
echo ""

endpoints=(
    "/health"
    "/api/v1/stats/summary"
    "/api/v1/calibration?bucket_width=10"
    "/api/v1/maker-taker?bucket_width=10"
    "/api/v1/temporal?granularity=quarterly"
    "/api/v1/categories"
    "/api/v1/yes-no"
    "/api/v1/cohorts"
    "/api/v1/markets?limit=50"
)

for ep in "${endpoints[@]}"; do
    printf "%-45s " "$ep"

    result=$(curl -s -o /dev/null -w "%{http_code} %{time_total}" "${API}${ep}" 2>/dev/null || echo "000 0.000")
    code=$(echo "$result" | awk '{print $1}')
    time_s=$(echo "$result" | awk '{print $2}')
    time_ms=$(echo "$time_s" | awk '{printf "%.0f", $1 * 1000}')

    if [ "$code" = "200" ]; then
        if [ "$time_ms" -lt 500 ]; then
            echo "${time_ms}ms  [OK]"
        else
            echo "${time_ms}ms  [SLOW - over 500ms target]"
        fi
    else
        echo "HTTP ${code}  [FAIL]"
    fi
done

echo ""
echo "Target: all endpoints < 500ms"
