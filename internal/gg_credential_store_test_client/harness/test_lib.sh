#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=SCRIPTDIR/lib.sh
. "$here/lib.sh"

[ "$(new_pids "" "")" = "" ] || { echo "new_pids: empty lists must yield nothing" >&2; exit 1; }
[ "$(new_pids "" "7")" = "7" ] || { echo "new_pids: a pid absent before must be reported" >&2; exit 1; }
[ "$(new_pids "7" "7")" = "" ] || { echo "new_pids: a pid present before must be ignored" >&2; exit 1; }
[ "$(new_pids $'7\n8' $'8\n9')" = "9" ] || { echo "new_pids: only the pid absent before must be reported" >&2; exit 1; }

results=$(mktemp)
RESULTS_FILE=$results
CLIENT_TIMEOUT=5
run_client "true-case" ok 0 /usr/bin/true
run_client "false-case" error 1 /usr/bin/false
run_client "timeout-case" timeout 142 /bin/sleep 30
run_client_with_input "needle" "stdin-case" ok 0 /usr/bin/grep -q needle
report='{"ok":true,"result":{"source":"user_file"}}'
run_client "field-match" "ok:source=user_file" 0 /bin/echo "$report"
run_client "field-mismatch" "ok:source=user_os_store" 0 /bin/echo "$report"
run_client "error-kind" no_token 6 /bin/sh -c 'echo "{\"ok\":false,\"error\":{\"kind\":\"no_token\"}}"; exit 6'

actual_lines=$(wc -l < "$results" | tr -d ' ')
[ "$actual_lines" = 7 ] || { echo "expected 7 lines, got $actual_lines" >&2; cat "$results" >&2; exit 1; }
grep -q $'^field-match\tok:source=user_file\t0\t0\tok:source=user_file\tno\tpass$' "$results" || { echo "bad field-match line" >&2; cat "$results" >&2; exit 1; }
grep -q $'^field-mismatch\tok:source=user_os_store\t0\t0\tok:source=user_file\tno\tfail$' "$results" || { echo "bad field-mismatch line" >&2; cat "$results" >&2; exit 1; }
grep -q $'^error-kind\tno_token\t6\t6\tno_token\tno\tpass$' "$results" || { echo "bad error-kind line" >&2; cat "$results" >&2; exit 1; }
grep -q $'^stdin-case\tok\t0\t0\tok\tno\tpass$' "$results" || { echo "bad stdin-case line" >&2; cat "$results" >&2; exit 1; }
grep -q $'^true-case\tok\t0\t0\tok\tno\tpass$' "$results" || { echo "bad true-case line" >&2; cat "$results" >&2; exit 1; }
grep -q $'^false-case\terror\t1\t1\terror\tno\tpass$' "$results" || { echo "bad false-case line" >&2; cat "$results" >&2; exit 1; }
grep -q $'^timeout-case\ttimeout\t142\t142\ttimeout\tno\tpass$' "$results" || { echo "bad timeout-case line" >&2; cat "$results" >&2; exit 1; }
rm -f "$results"

RESULTS_FILE=""
RESULTS_DIR=$(mktemp -d)
SHARED_RESULTS_BASE=$(mktemp -d)
chmod 555 "$RESULTS_DIR"
run_client "unwritable" ok 0 /usr/bin/true
case "$RESULTS_FILE" in
    "$SHARED_RESULTS_BASE/gg_credential_store-results-$(id -un)/"*) ;;
    *) echo "an unwritable results directory must fall back to a per-user shared one, got $RESULTS_FILE" >&2; exit 1 ;;
esac
chmod 755 "$RESULTS_DIR"
rm -rf "$RESULTS_DIR" "$SHARED_RESULTS_BASE"

RESULTS_FILE=""
RESULTS_DIR=$(mktemp -d)
run_client "first" ok 0 /usr/bin/true
run_client "second" error 1 /usr/bin/false
run_client "third" ok 0 /usr/bin/false
created=$(find "$RESULTS_DIR" -name '*.tsv' | wc -l | tr -d ' ')
[ "$created" = 1 ] || { echo "expected one results file, found $created" >&2; exit 1; }
lines=$(cat "$RESULTS_DIR"/*.tsv | wc -l | tr -d ' ')
[ "$lines" = 4 ] || { echo "expected header plus 3 lines, got $lines" >&2; cat "$RESULTS_DIR"/*.tsv >&2; exit 1; }
if summary > /dev/null; then echo "summary must fail when a row failed" >&2; exit 1; fi
rm -rf "$RESULTS_DIR"
echo "lib.sh self-test passed"
