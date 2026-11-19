#!/usr/bin/env bash
# Checks the user token store against gnome-keyring and Python keyring, and the
# fallback to the config file without a session bus and with a locked keyring.
# Usage: dbus-run-session -- secret_service_linux.sh <test client>
set -euo pipefail

client=$1
instance=https://dashboard.gitguardian.com
config=$(mktemp -d)
runtime=$(mktemp -d)
trap 'rm -rf "$config" "$runtime"' EXIT
export GG_CONFIG_DIR=$config
export PYTHON_KEYRING_BACKEND=keyring.backends.SecretService.Keyring

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# Usage: check <label> <jq filter> <output>
check() {
    jq -e "$2" <<<"$3" >/dev/null || fail "$1: $3"
}

keyring_get() {
    python3 -c 'import sys, keyring; print(keyring.get_password("ggshield", sys.argv[1]))' "$instance"
}

keyring_set() {
    python3 -c 'import sys, keyring; keyring.set_password("ggshield", sys.argv[1], sys.argv[2])' \
        "$instance" "$1"
}

item_count() {
    python3 - "$instance" <<'EOF'
import sys
import secretstorage

collection = secretstorage.get_default_collection(secretstorage.dbus_init())
print(len(list(collection.search_items({"service": "ggshield", "username": sys.argv[1]}))))
EOF
}

output=$(printf gg_no_bus |
    env -u DBUS_SESSION_BUS_ADDRESS XDG_RUNTIME_DIR="$runtime" "$client" store --instance "$instance")
check "store without a session bus" '.result.location == "user_file" and .result.reason == "unavailable"' "$output"
rm -f "$config/auth_config.yaml"

printf test | gnome-keyring-daemon --unlock --components=secrets >/dev/null

output=$(printf gg_from_crate | "$client" store --instance "$instance")
check "store" '.result.location == "user_os_store"' "$output"
[ "$(keyring_get)" = gg_from_crate ] || fail "Python keyring read: $(keyring_get)"

keyring_set gg_from_keyring
output=$("$client" read --instance "$instance")
check "read after Python keyring" '.result.token == "gg_from_keyring" and .result.source == "user_os_store"' "$output"
[ "$(item_count)" = 1 ] || fail "items after both writes: $(item_count)"

output=$("$client" delete --instance "$instance")
check "delete" '.result.deleted' "$output"
[ "$(keyring_get)" = None ] || fail "Python keyring read after delete: $(keyring_get)"

dbus-send --session --print-reply --dest=org.freedesktop.secrets /org/freedesktop/secrets \
    org.freedesktop.Secret.Service.Lock array:objpath:/org/freedesktop/secrets/collection/login >/dev/null
output=$(printf gg_locked | "$client" store --instance "$instance")
check "store with a locked keyring" '.result.location == "user_file" and .result.reason == "locked"' "$output"

echo "secret service: passed"
