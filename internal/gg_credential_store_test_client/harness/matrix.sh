#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=SCRIPTDIR/lib.sh
. "$here/lib.sh"

BIN=$here/bin
REQUIREMENTS=$workspace_dir/public/gg_credential_store/src/os_store/macos/requirements
BUILT=$workspace_dir/target/release/gg_credential_store_test_client
SERVICE=ggshield
B_INSTANCE=https://b.test-client.invalid
P_INSTANCE=https://p.test-client.invalid
C_INSTANCE=https://c.test-client.invalid

usage() {
    cat <<USAGE
usage: matrix.sh PHASE

phases, in order:
  build    build and sign the test clients into harness/bin (needs GG_SIGN_IDENTITY, "-" for ad hoc)
  user     the user token cases, run as the logged-in user in the GUI session
  cleanup  remove the test items from the login keychain
USAGE
}

build_client() {
    (cd "$workspace_dir" && GG_TEST_CLIENT_BUILD_TAG=$1 cargo build --release -p gg_credential_store_test_client)
}

install_launchers() {
    local dir=$1 name
    mkdir -p "$dir"
    for name in ggshield ggshield-py; do
        cp "$BUILT" "$dir/$name"
        "$here/sign.sh" "$dir/$name" "$GG_SIGN_IDENTITY" "com.gitguardian.$name"
        if [ "$GG_SIGN_IDENTITY" != "-" ]; then
            codesign --verify -R "$REQUIREMENTS/$name.txt" "$dir/$name"
        fi
    done
}

phase_build() {
    : "${GG_SIGN_IDENTITY:?set GG_SIGN_IDENTITY to the codesign identity name, or - for ad hoc}"
    rm -rf "$BIN"
    build_client v1
    install_launchers "$BIN/v1"
    mkdir -p "$BIN/relocated" "$BIN/adhoc"
    cp "$BIN/v1/ggshield" "$BIN/relocated/ggshield"
    cp "$BUILT" "$BIN/adhoc/ggshield"
    "$here/sign.sh" "$BIN/adhoc/ggshield" - com.gitguardian.ggshield.adhoc
    build_client v2
    install_launchers "$BIN/v2"
    if cmp -s "$BIN/v1/ggshield" "$BIN/v2/ggshield"; then
        echo "the rebuilt test client is identical to the first build" >&2
        exit 1
    fi
    echo "built: $BIN/v1 $BIN/v2 $BIN/relocated $BIN/adhoc"
}

phase_user() {
    local v1=$BIN/v1 v2=$BIN/v2 relocated=$BIN/relocated adhoc=$BIN/adhoc state
    state=$(mktemp -d)
    export GG_CONFIG_DIR=$state/config GG_CACHE_DIR=$state/cache
    unset GGSHIELD_NO_KEYRING

    run_client_with_input "token-b" "B-self-store" "ok:location=user_os_store" 0 "$v1/ggshield" store --instance "$B_INSTANCE"
    run_client "B-self-read" "ok:source=user_os_store" 0 "$v1/ggshield" read --instance "$B_INSTANCE"
    run_client "B-relocated-read" "ok:source=user_os_store" 0 "$relocated/ggshield" read --instance "$B_INSTANCE"
    run_client "B-upgraded-read" "ok:source=user_os_store" 0 "$v2/ggshield" read --instance "$B_INSTANCE"
    run_client "B-py-read" "ok:source=user_os_store" 0 "$v1/ggshield-py" read --instance "$B_INSTANCE"
    run_client "B-py-upgraded-read" "ok:source=user_os_store" 0 "$v2/ggshield-py" read --instance "$B_INSTANCE"
    run_client "B-adhoc-read" os_store_denied 2 "$adhoc/ggshield" read --instance "$B_INSTANCE"
    if [ -n "${GG_KEYCHAIN_PASSWORD:-}" ]; then
        security lock-keychain
        run_client "B-locked-read" os_store_locked 4 "$v1/ggshield" read --instance "$B_INSTANCE"
        security unlock-keychain -p "$GG_KEYCHAIN_PASSWORD"
    fi
    run_client "B-delete" ok 0 "$v1/ggshield" delete --instance "$B_INSTANCE"
    run_client "B-read-after-delete" no_token 6 "$v1/ggshield" read --instance "$B_INSTANCE"

    run_client_with_input "token-p" "P-self-store" "ok:location=user_os_store" 0 "$v1/ggshield-py" store --instance "$P_INSTANCE"
    run_client "P-ggshield-read" "ok:source=user_os_store" 0 "$v1/ggshield" read --instance "$P_INSTANCE"
    run_client "P-delete" ok 0 "$v1/ggshield-py" delete --instance "$P_INSTANCE"

    run_client_with_input "token-c" "C-self-store" "ok:location=user_file" 0 "$adhoc/ggshield" store --instance "$C_INSTANCE"
    run_client "C-self-read" "ok:source=user_file" 0 "$adhoc/ggshield" read --instance "$C_INSTANCE"
    run_client "C-signed-read" "ok:source=user_file" 0 "$v1/ggshield" read --instance "$C_INSTANCE"
    run_client "C-delete" ok 0 "$adhoc/ggshield" delete --instance "$C_INSTANCE"

    rm -rf "$state"
    summary
}

phase_cleanup() {
    local instance
    for instance in "$B_INSTANCE" "$P_INSTANCE" "$C_INSTANCE"; do
        security delete-generic-password -s "$SERVICE" -a "$instance" > /dev/null 2>&1 || true
    done
    echo "removed the test items of $SERVICE for $B_INSTANCE, $P_INSTANCE and $C_INSTANCE"
}

case "${1:-}" in
    build) phase_build ;;
    user) phase_user ;;
    cleanup) phase_cleanup ;;
    *) usage; exit 2 ;;
esac
