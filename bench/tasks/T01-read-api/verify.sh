#!/bin/bash
cd "$(dirname "$0")"
test -f ANSWER.md || { echo VERIFY_FAIL; exit 1; }
grep -q "sum_positive\|factorial\|is_palindrome\|max_element" ANSWER.md && echo VERIFY_PASS || { echo VERIFY_FAIL; exit 1; }
