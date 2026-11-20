#!/usr/bin/env bash
# Checks that a test client without the GitGuardian signature cannot store a
# machine token, and reads one from the file.
# Usage: machine_store_macos.sh <test client>
set -euo pipefail

client=$1
instance=https://dashboard.gitguardian.com
dir="/Library/Application Support/ggshield"
file=$dir/machine_credential.yaml

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# Usage: store <token> <command that runs the test client>
store() {
    printf '%s' "$1" | "${@:2}" store --instance "$instance" --machine-id machine-1
}

in_system_keychain() {
    security find-generic-password -s ggshield -a "$instance" /Library/Keychains/System.keychain \
        >/dev/null 2>&1
}

[ ! -e "$dir" ] || fail "$dir exists"
trap 'sudo rm -rf "$dir"' EXIT

output=$(store gg_mat_denied sudo "$client") && fail "root store: $output"
jq -e '.error.kind == "machine_store_denied"' <<<"$output" >/dev/null || fail "root store: $output"
output=$(store gg_mat_denied "$client") && fail "user store: $output"
jq -e '.error.kind == "machine_store_denied"' <<<"$output" >/dev/null || fail "user store: $output"
[ ! -e "$file" ] || fail "a denied store wrote $file"
! in_system_keychain || fail "a denied store wrote to the System keychain"

sudo mkdir -m 755 "$dir"
printf 'version: 1\ninstance: %s\nmachine_id: machine-1\ntoken: gg_mat_file\n' "$instance" |
    sudo tee "$file" >/dev/null
sudo chmod 644 "$file"

expected='{"token": "gg_mat_file", "source": "machine_file", "machine_id": "machine-1"}'
output=$("$client" read --instance "$instance")
jq -e --argjson expected "$expected" '.result == $expected' <<<"$output" >/dev/null ||
    fail "user read: $output"

output=$("$client" managed-instance)
jq -e --arg instance "$instance" '.result.instance == $instance' <<<"$output" >/dev/null ||
    fail "managed instance: $output"

echo "machine store: passed"
