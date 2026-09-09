#!/bin/bash
cd "$(dirname "$0")"
test -f ANSWER.md || { echo NO_FILE; exit 1; }
grep -qi "id.*u64\|name.*String\|price_cents.*u32\|in_stock.*bool" ANSWER.md && echo VERIFY_PASS || { echo WRONG_ANSWER; exit 1; }
