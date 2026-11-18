#!/usr/bin/env bash
# Stores a machine token as root and checks it as the calling user.
# Usage: machine_store_linux.sh <test client>
set -euo pipefail

client=$1
instance=https://dashboard.gitguardian.com
dir=/etc/xdg/ggshield
file=$dir/machine_credential.yaml

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# Usage: store <token> <command that runs the test client>
store() {
    printf '%s' "$1" | "${@:2}" store --instance "$instance" --machine-id machine-1
}

trap 'sudo rm -rf "$dir"' EXIT
sudo rm -rf "$dir"

output=$(store gg_mat_user "$client") && fail "a user stored a machine token: $output"
jq -e '.error.kind == "machine_store_denied"' <<<"$output" >/dev/null || fail "user store: $output"
[ ! -e "$file" ] || fail "a user store wrote $file"

output=$(store gg_mat_root sudo "$client")
jq -e '.result.location == "machine_file"' <<<"$output" >/dev/null || fail "root store: $output"
[ "$(stat -c '%U %a' "$dir")" = "root 755" ] || fail "$dir: $(stat -c '%U %a' "$dir")"
[ "$(stat -c '%U %a' "$file")" = "root 644" ] || fail "$file: $(stat -c '%U %a' "$file")"

expected='{"token": "gg_mat_root", "source": "machine_file", "machine_id": "machine-1"}'
output=$("$client" read --instance "$instance")
jq -e --argjson expected "$expected" '.result == $expected' <<<"$output" >/dev/null ||
    fail "user read: $output"

output=$("$client" managed-instance)
jq -e --arg instance "$instance" '.result.instance == $instance' <<<"$output" >/dev/null ||
    fail "managed instance: $output"

output=$(store gg_mat_user "$client") && fail "a user replaced the machine token: $output"
jq -e '.error.kind == "machine_store_denied"' <<<"$output" >/dev/null || fail "user overwrite: $output"
output=$("$client" read --instance "$instance")
jq -e '.result.token == "gg_mat_root"' <<<"$output" >/dev/null || fail "read after overwrite: $output"

echo "machine store: passed"
