#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <version>" >&2
    exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

awk -v heading="## [$1]" '
    /^## \[/ {
        if (found) exit
        if ($0 == heading || index($0, heading " ") == 1) {
            found = 1
            next
        }
    }
    found {
        notes = notes $0 ORS
        if ($0 ~ /[^[:space:]]/) content = 1
    }
    END {
        if (!found || !content) {
            print "Missing or empty changelog entry: " heading > "/dev/stderr"
            exit 1
        }
        sub(/^\n+/, "", notes)
        sub(/\n+$/, "", notes)
        print notes
    }
' "$ROOT_DIR/CHANGELOG.md"
