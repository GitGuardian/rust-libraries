#!/usr/bin/env bash
set -euo pipefail

usage="usage: sign.sh BINARY IDENTITY IDENTIFIER (IDENTITY: a codesign identity, - for ad hoc, or a PEM file for rcodesign)"
binary=${1:?$usage}
identity=${2:?$usage}
identifier=${3:?$usage}

if [ "$identity" = "-" ]; then
    codesign --force --sign - --identifier "$identifier" "$binary"
elif [[ "$identity" == *.pem ]]; then
    rcodesign sign --pem-file "$identity" --binary-identifier "$identifier" \
        --code-signature-flags runtime "$binary"
else
    args=(--force --sign "$identity" --identifier "$identifier" --options runtime)
    if [ "${GG_SIGN_TIMESTAMP:-0}" = 1 ]; then
        args+=(--timestamp)
    fi
    codesign "${args[@]}" "$binary"
fi

signed_as=$(codesign --display --verbose "$binary" 2>&1 | sed -n 's/^Identifier=//p')
if [ "$signed_as" != "$identifier" ]; then
    echo "FAIL: $binary is signed as '$signed_as', not $identifier" >&2
    exit 1
fi
echo "signed $binary as $identifier"
