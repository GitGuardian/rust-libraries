#!/usr/bin/env bash
# Signs copies of the test client with a self-signed certificate created for
# this run, and checks how the login keychain and the System keychain treat
# each copy. Needs rcodesign, jq and sudo without a password. It changes the
# keychains of the user who runs it, so it runs in CI only.
# Usage: keychain_macos.sh
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
workspace=$(cd "$here/../../.." && pwd)
built=$workspace/target/debug/gg_credential_store_test_client
service=ggshield
b_instance=https://b.test-client.invalid
p_instance=https://p.test-client.invalid
c_instance=https://c.test-client.invalid
m_instance=https://m.test-client.invalid
machine_dir="/Library/Application Support/ggshield"
system_keychain=/Library/Keychains/System.keychain
case_timeout=20
failures=0

if [ -e "$machine_dir" ]; then
    echo "FAIL: $machine_dir exists, and the machine token cases would replace its token" >&2
    exit 1
fi

work=$(mktemp -d)
cleanup() {
    for instance in "$b_instance" "$p_instance" "$c_instance"; do
        security delete-generic-password -s "$service" -a "$instance" >/dev/null 2>&1 || true
    done
    sudo security delete-generic-password -s "$service" -a "$m_instance" "$system_keychain" \
        >/dev/null 2>&1 || true
    sudo rm -rf "$machine_dir"
    rm -rf "$work"
}
trap cleanup EXIT

cat >"$work/certificate.conf" <<'EOF'
[req]
distinguished_name = subject
x509_extensions = code_signing
prompt = no

[subject]
CN = gg_credential_store test signer

[code_signing]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
EOF
/usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 1 -config "$work/certificate.conf" \
    -keyout "$work/key.pem" -out "$work/certificate.pem" 2>/dev/null
cat "$work/key.pem" "$work/certificate.pem" >"$work/identity.pem"
hash=$(/usr/bin/openssl x509 -in "$work/certificate.pem" -noout -fingerprint -sha1 |
    sed 's/.*=//; s/://g')
export GG_CREDENTIAL_STORE_TEST_SIGNER=$hash
export RUSTFLAGS="--cfg gg_credential_store_test_signer"
export GG_CONFIG_DIR=$work/config GG_CACHE_DIR=$work/cache
unset GGSHIELD_NO_KEYRING

build() {
    local dir=$work/$1 name
    (cd "$workspace" && GG_TEST_CLIENT_BUILD_TAG=$1 cargo build --quiet -p gg_credential_store_test_client)
    mkdir "$dir"
    for name in ggshield ggshield-py; do
        cp "$built" "$dir/$name"
        rcodesign sign --pem-file "$work/identity.pem" --binary-identifier "com.gitguardian.$name" \
            --code-signature-flags runtime "$dir/$name" >/dev/null
        codesign --verify -R "=identifier \"com.gitguardian.$name\" and certificate leaf = H\"$hash\"" \
            "$dir/$name"
    done
}

check() {
    local label=$1 expected=$2 input=$3
    shift 3
    local agents pid status=0 actual verdict dialog=no
    agents=$(pgrep -x SecurityAgent || true)
    printf '%s' "$input" | perl -e 'alarm shift @ARGV; exec @ARGV' "$case_timeout" "$@" \
        >"$work/output" 2>/dev/null &
    pid=$!
    while kill -0 "$pid" 2>/dev/null; do
        if { pgrep -x SecurityAgent || true; } | grep -qvxF -e "${agents:-none}"; then
            dialog=yes
        fi
        sleep 0.2
    done
    wait "$pid" || status=$?
    if [ "$status" = 0 ]; then
        actual="${expected%%=*}=$(jq -r ".result.${expected%%=*}" "$work/output" 2>/dev/null || true)"
    elif [ "$status" = 142 ]; then
        actual=timeout
    else
        actual=$(jq -r '.error.kind // empty' "$work/output" 2>/dev/null || true)
    fi
    if [ -z "$actual" ]; then
        actual="exit $status"
    fi
    verdict=fail
    if [ "$dialog" = yes ]; then
        verdict=dialog
    elif [ "$actual" = "$expected" ]; then
        verdict=pass
    fi
    echo "$label: expected $expected, got $actual, $verdict"
    if [ "$verdict" != pass ]; then
        failures=$((failures + 1))
    fi
}

build v1
build v2
if cmp -s "$work/v1/ggshield" "$work/v2/ggshield"; then
    echo "FAIL: the second build is identical to the first" >&2
    exit 1
fi
mkdir "$work/relocated" "$work/adhoc"
cp "$work/v1/ggshield" "$work/relocated/ggshield"
cp "$built" "$work/adhoc/ggshield"
codesign --force --sign - --identifier com.gitguardian.ggshield.adhoc "$work/adhoc/ggshield"
v1=$work/v1
v2=$work/v2
relocated=$work/relocated
adhoc=$work/adhoc

check B-self-store location=user_os_store token-b "$v1/ggshield" store --instance "$b_instance"
check B-self-read source=user_os_store "" "$v1/ggshield" read --instance "$b_instance"
check B-relocated-read source=user_os_store "" "$relocated/ggshield" read --instance "$b_instance"
check B-upgraded-read os_store_denied "" "$v2/ggshield" read --instance "$b_instance"
check B-py-read os_store_denied "" "$v1/ggshield-py" read --instance "$b_instance"
check B-py-upgraded-read os_store_denied "" "$v2/ggshield-py" read --instance "$b_instance"
check B-adhoc-read os_store_denied "" "$adhoc/ggshield" read --instance "$b_instance"
check B-delete outcome=removed "" "$v1/ggshield" delete --instance "$b_instance"
check B-read-after-delete no_token "" "$v1/ggshield" read --instance "$b_instance"

check P-self-store location=user_os_store token-p "$v1/ggshield-py" store --instance "$p_instance"
check P-ggshield-read os_store_denied "" "$v1/ggshield" read --instance "$p_instance"
check P-delete outcome=removed "" "$v1/ggshield-py" delete --instance "$p_instance"

check C-self-store location=user_file token-c "$adhoc/ggshield" store --instance "$c_instance"
check C-self-read source=user_file "" "$adhoc/ggshield" read --instance "$c_instance"
check C-signed-read source=user_file "" "$v1/ggshield" read --instance "$c_instance"
check C-delete outcome=removed "" "$adhoc/ggshield" delete --instance "$c_instance"

check M-user-store machine_store_denied gg_mat_user \
    "$v1/ggshield" store --instance "$m_instance" --machine-id machine-1
check M-root-store location=machine_os_store gg_mat_root \
    sudo -n "$v1/ggshield" store --instance "$m_instance" --machine-id machine-1
check M-root-read source=machine_os_store "" sudo -n "$v1/ggshield" read --instance "$m_instance"
check M-self-read source=machine_os_store "" "$v1/ggshield" read --instance "$m_instance"
check M-relocated-read source=machine_os_store "" "$relocated/ggshield" read --instance "$m_instance"
check M-upgraded-read source=machine_os_store "" "$v2/ggshield" read --instance "$m_instance"
check M-py-read source=machine_os_store "" "$v1/ggshield-py" read --instance "$m_instance"
check M-adhoc-read machine_token_skipped "" "$adhoc/ggshield" read --instance "$m_instance"
check M-managed-instance "instance=$m_instance" "" "$adhoc/ggshield" managed-instance

if [ "$failures" != 0 ]; then
    echo "keychain cases: $failures failed" >&2
    exit 1
fi
echo "keychain cases: passed"
