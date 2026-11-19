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
M_INSTANCE=https://m.test-client.invalid
MACHINE_DIR="/Library/Application Support/ggshield"
SYSTEM_KEYCHAIN=/Library/Keychains/System.keychain

usage() {
    cat <<USAGE
usage: matrix.sh PHASE

phases, in order:
  build    build and sign the test clients into harness/bin (needs GG_SIGN_IDENTITY, "-" for ad hoc)
  user     the user token cases, run as the logged-in user in the GUI session
  machine  the machine token cases, run as the logged-in user, with sudo for root
  cleanup  remove the test items from the login keychain and the machine token
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
        if [ -n "${GG_CREDENTIAL_STORE_TEST_SIGNER:-}" ]; then
            codesign --verify -R "=identifier \"com.gitguardian.$name\" and certificate leaf = H\"$GG_CREDENTIAL_STORE_TEST_SIGNER\"" "$dir/$name"
        elif [ "$GG_SIGN_IDENTITY" != "-" ]; then
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
    local other=("ok:source=user_os_store" 0)
    if [ -n "${GG_CREDENTIAL_STORE_TEST_SIGNER:-}" ]; then
        other=(os_store_denied 2)
    fi
    state=$(mktemp -d)
    export GG_CONFIG_DIR=$state/config GG_CACHE_DIR=$state/cache
    unset GGSHIELD_NO_KEYRING

    run_client_with_input "token-b" "B-self-store" "ok:location=user_os_store" 0 "$v1/ggshield" store --instance "$B_INSTANCE"
    run_client "B-self-read" "ok:source=user_os_store" 0 "$v1/ggshield" read --instance "$B_INSTANCE"
    run_client "B-relocated-read" "ok:source=user_os_store" 0 "$relocated/ggshield" read --instance "$B_INSTANCE"
    run_client "B-upgraded-read" "${other[@]}" "$v2/ggshield" read --instance "$B_INSTANCE"
    run_client "B-py-read" "${other[@]}" "$v1/ggshield-py" read --instance "$B_INSTANCE"
    run_client "B-py-upgraded-read" "${other[@]}" "$v2/ggshield-py" read --instance "$B_INSTANCE"
    run_client "B-adhoc-read" os_store_denied 2 "$adhoc/ggshield" read --instance "$B_INSTANCE"
    if [ -n "${GG_KEYCHAIN_PASSWORD:-}" ]; then
        security lock-keychain
        run_client "B-locked-read" os_store_locked 4 "$v1/ggshield" read --instance "$B_INSTANCE"
        security unlock-keychain -p "$GG_KEYCHAIN_PASSWORD"
    fi
    run_client "B-delete" "ok:outcome=removed" 0 "$v1/ggshield" delete --instance "$B_INSTANCE"
    run_client "B-read-after-delete" no_token 6 "$v1/ggshield" read --instance "$B_INSTANCE"

    run_client_with_input "token-p" "P-self-store" "ok:location=user_os_store" 0 "$v1/ggshield-py" store --instance "$P_INSTANCE"
    run_client "P-ggshield-read" "${other[@]}" "$v1/ggshield" read --instance "$P_INSTANCE"
    run_client "P-delete" "ok:outcome=removed" 0 "$v1/ggshield-py" delete --instance "$P_INSTANCE"

    run_client_with_input "token-c" "C-self-store" "ok:location=user_file" 0 "$adhoc/ggshield" store --instance "$C_INSTANCE"
    run_client "C-self-read" "ok:source=user_file" 0 "$adhoc/ggshield" read --instance "$C_INSTANCE"
    run_client "C-signed-read" "ok:source=user_file" 0 "$v1/ggshield" read --instance "$C_INSTANCE"
    run_client "C-delete" "ok:outcome=removed" 0 "$adhoc/ggshield" delete --instance "$C_INSTANCE"

    rm -rf "$state"
    summary
}

phase_machine() {
    local v1=$BIN/v1 v2=$BIN/v2 relocated=$BIN/relocated adhoc=$BIN/adhoc state
    if [ -e "$MACHINE_DIR" ]; then
        echo "$MACHINE_DIR exists; the machine cases would replace its machine token" >&2
        exit 1
    fi
    sudo -v
    state=$(mktemp -d)
    export GG_CONFIG_DIR=$state/config GG_CACHE_DIR=$state/cache
    unset GGSHIELD_NO_KEYRING

    run_client_with_input "gg_mat_user" "M-user-store" machine_store_denied 7 "$v1/ggshield" store --instance "$M_INSTANCE" --machine-id machine-1
    run_client_with_input "gg_mat_root" "M-root-store" "ok:location=machine_os_store" 0 sudo -n "$v1/ggshield" store --instance "$M_INSTANCE" --machine-id machine-1
    run_client "M-root-read" "ok:source=machine_os_store" 0 sudo -n "$v1/ggshield" read --instance "$M_INSTANCE"
    run_client "M-self-read" "ok:source=machine_os_store" 0 "$v1/ggshield" read --instance "$M_INSTANCE"
    run_client "M-relocated-read" "ok:source=machine_os_store" 0 "$relocated/ggshield" read --instance "$M_INSTANCE"
    run_client "M-upgraded-read" "ok:source=machine_os_store" 0 "$v2/ggshield" read --instance "$M_INSTANCE"
    run_client "M-py-read" "ok:source=machine_os_store" 0 "$v1/ggshield-py" read --instance "$M_INSTANCE"
    run_client "M-adhoc-read" no_token 6 "$adhoc/ggshield" read --instance "$M_INSTANCE"
    run_client "M-managed-instance" "ok:instance=$M_INSTANCE" 0 "$adhoc/ggshield" managed-instance

    remove_machine_token
    rm -rf "$state"
    summary
}

remove_machine_token() {
    sudo security delete-generic-password -s "$SERVICE" -a "$M_INSTANCE" "$SYSTEM_KEYCHAIN" > /dev/null 2>&1 || true
    sudo rm -f "$MACHINE_DIR/machine_credential.yaml"
    sudo rmdir "$MACHINE_DIR" 2> /dev/null || true
}

phase_cleanup() {
    local instance
    for instance in "$B_INSTANCE" "$P_INSTANCE" "$C_INSTANCE"; do
        security delete-generic-password -s "$SERVICE" -a "$instance" > /dev/null 2>&1 || true
    done
    echo "removed the test items of $SERVICE for $B_INSTANCE, $P_INSTANCE and $C_INSTANCE"
    if [ "$(sudo -n cat "$MACHINE_DIR/machine_credential.yaml" 2> /dev/null | sed -n 's/^instance: //p')" = "$M_INSTANCE" ]; then
        remove_machine_token
        echo "removed the machine token of $M_INSTANCE"
    fi
}

case "${1:-}" in
    build) phase_build ;;
    user) phase_user ;;
    machine) phase_machine ;;
    cleanup) phase_cleanup ;;
    *) usage; exit 2 ;;
esac
