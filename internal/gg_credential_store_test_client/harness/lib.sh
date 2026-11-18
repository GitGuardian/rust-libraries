#!/usr/bin/env bash
# shellcheck disable=SC2034 # The variables are read by the scripts that source this file.

harness_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
workspace_dir=$(cd "$harness_dir/../../.." && pwd)
RESULTS_DIR=${RESULTS_DIR:-$harness_dir/results}
RESULTS_FILE=${RESULTS_FILE:-}
SHARED_RESULTS_BASE=${SHARED_RESULTS_BASE:-/Users/Shared}
CLIENT_TIMEOUT=${CLIENT_TIMEOUT:-20}

ensure_results_file() {
    if [ -z "$RESULTS_FILE" ]; then
        if ! mkdir -p "$RESULTS_DIR" 2>/dev/null || [ ! -w "$RESULTS_DIR" ]; then
            RESULTS_DIR=$SHARED_RESULTS_BASE/gg_credential_store-results-$(id -un)
            mkdir -p "$RESULTS_DIR"
        fi
        RESULTS_FILE="$RESULTS_DIR/$(date +%Y%m%d-%H%M%S)-$(id -un).tsv"
        printf 'label\texpected\texpected_exit\tactual_exit\tactual_kind\tsecurity_agent_seen\tverdict\n' > "$RESULTS_FILE"
    fi
}

security_agent_pids() {
    pgrep -x SecurityAgent 2>/dev/null || true
}

new_pids() {
    local before=$1 after=$2 pid
    for pid in $after; do
        case " $(echo "$before" | tr '\n' ' ') " in
            *" $pid "*) ;;
            *) echo "$pid" ;;
        esac
    done
}

security_agent_appeared() {
    local before=$1
    [ -n "$(new_pids "$before" "$(security_agent_pids)")" ]
}

json_field() {
    plutil -extract "$2" raw -o - "$1" 2>/dev/null || echo error
}

client_kind() {
    local exit_code=$1 output=$2 expected=$3
    if [ "$exit_code" = 0 ]; then
        case "$expected" in
            ok:*=*)
                local field=${expected#ok:}
                field=${field%%=*}
                echo "ok:$field=$(json_field "$output" "result.$field")"
                ;;
            *) echo ok ;;
        esac
        return
    fi
    if [ "$exit_code" = 142 ]; then
        echo timeout
        return
    fi
    json_field "$output" error.kind
}

matches_any() {
    local actual=$1 expected=$2 candidate
    local IFS='|'
    for candidate in $expected; do
        [ "$actual" = "$candidate" ] && return 0
    done
    return 1
}

run_client() {
    local label=$1 expected=$2 expected_exit=$3
    shift 3
    local output exit_code=0 seen=no kind verdict agents_before
    ensure_results_file
    output=$(mktemp)
    agents_before=$(security_agent_pids)
    ( perl -e 'alarm shift @ARGV; exec @ARGV' "$CLIENT_TIMEOUT" "$@"; exit $? ) > "$output" 2>/dev/null < "${CLIENT_STDIN:-/dev/null}" &
    local pid=$!
    while kill -0 "$pid" 2>/dev/null; do
        if security_agent_appeared "$agents_before"; then
            seen=yes
        fi
        sleep 0.2
    done
    wait "$pid" || exit_code=$?
    kind=$(client_kind "$exit_code" "$output" "$expected")
    verdict=fail
    if matches_any "$exit_code" "$expected_exit" && matches_any "$kind" "$expected"; then
        verdict=pass
    fi
    if [ "$seen" = yes ]; then
        verdict=dialog
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$label" "$expected" "$expected_exit" "$exit_code" "$kind" "$seen" "$verdict" >> "$RESULTS_FILE"
    echo "$label: expected $expected/$expected_exit, got $kind/$exit_code, dialog=$seen, $verdict"
    rm -f "$output"
}

run_client_with_input() {
    local input=$1
    shift
    local file
    file=$(mktemp)
    printf '%s' "$input" > "$file"
    CLIENT_STDIN=$file run_client "$@"
    rm -f "$file"
}

summary() {
    ensure_results_file
    local file=$RESULTS_FILE
    echo
    echo "results: $file"
    column -t -s $'\t' "$file"
    if grep -q $'\tfail$' "$file" || grep -q $'\tdialog$' "$file"; then
        echo "matrix: failed"
        return 1
    fi
    echo "matrix: passed"
}
