#!/usr/bin/env bash
# Signs the test client with a self-signed certificate created for this run, and
# runs the user and machine token cases of the harness with it. Needs rcodesign
# and sudo.
# Usage: keychain_macos.sh
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
harness=$here/../harness
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

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
export GG_SIGN_IDENTITY=$work/identity.pem GG_CREDENTIAL_STORE_TEST_SIGNER=$hash
export RUSTFLAGS="--cfg gg_credential_store_test_signer"

"$harness/matrix.sh" build
status=0
"$harness/matrix.sh" user || status=$?
"$harness/matrix.sh" machine || status=$?
"$harness/matrix.sh" cleanup
exit "$status"
