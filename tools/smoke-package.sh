#!/bin/sh
# Exercise the production Couch integration installer against a disposable
# ARMv7 APK and repository. This never uses a release key or a device.
set -eu

usage() {
    echo "Usage: $0 FIRETV_BINARY COUCH_CONFD SDK_ROOT PLUGIN_MANIFEST OUTPUT_DIR" >&2
    exit 64
}
[ "$#" = 5 ] || usage

binary=$1
confd=$2
sdk=$3
manifest=$4
output=$5

for path in "$binary" "$confd" "$sdk" "$manifest"; do
    case "$path" in /*) ;; *) echo "paths must be absolute: $path" >&2; exit 64;; esac
done
[ -x "$binary" ] || { echo "Fire TV binary is not executable: $binary" >&2; exit 1; }
[ -x "$confd" ] || { echo "couch-confd is not executable: $confd" >&2; exit 1; }
[ -f "$sdk/tools/integrations/build-apk.sh" ] || { echo "pinned SDK package tools are missing" >&2; exit 1; }
[ -f "$sdk/tools/integrations/build-repository.sh" ] || { echo "pinned SDK repository builder is missing" >&2; exit 1; }
[ -f "$manifest" ] || { echo "plugin manifest is missing: $manifest" >&2; exit 1; }
[ ! -e "$output" ] || { echo "output directory must not already exist: $output" >&2; exit 1; }

mkdir -p "$output"

docker run --rm --platform linux/arm/v7 \
    -v "$binary:/work/input/couch-plugin-firetv:ro" \
    -v "$confd:/work/input/couch-confd:ro" \
    -v "$manifest:/work/input/plugin.json:ro" \
    -v "$sdk:/work/sdk:ro" \
    -v "$output:/out:rw" \
    alpine:3.24 sh -eu -c '
        apk add --no-cache alpine-sdk binutils jq busybox-extras >/dev/null

        # Generate a key that exists only inside this disposable container.
        abuild-keygen -a -n >/dev/null
        key=$(find /root/.abuild -type f -name "*.rsa" ! -name "*.rsa.pub" -print -quit)
        test -n "$key"
        test -f "$key.pub"
        cp "$key.pub" /etc/apk/keys/

        version=$(jq -er ".version" /work/input/plugin.json)
        mkdir -p /out/packages

        apk_file="/out/packages/couch-integration-firetv-$version-r0.apk"
        sh /work/sdk/tools/integrations/build-apk.sh \
            firetv "$version" /work/input/couch-plugin-firetv \
            /work/input/plugin.json "$key" /out/packages
        test -s "$apk_file"

        # Verify the APK signature independently before passing it to Couch.
        apk --keys-dir /root/.abuild verify "$apk_file"

        # Signed sideload admission through the same couch-confd CLI used on
        # devices. The integration host is started for its real Hello handshake.
        /work/input/couch-confd integrations \
            --root /tmp/firetv-sideload \
            --keys-dir /root/.abuild \
            install-sideload "$apk_file" >/out/sideload.log 2>&1
        /work/input/couch-confd integrations --root /tmp/firetv-sideload list \
            | grep -Fx "firetv $version"

        # Build and serve a signed local repository, then install through the
        # repository path rather than treating a sideload as equivalent.
        sh /work/sdk/tools/integrations/build-repository.sh \
            "$key" /out/packages /out/repository
        busybox httpd -f -p 18080 -h /out/repository/armv7 \
            >/out/httpd.log 2>&1 &
        server=$!
        trap "kill $server 2>/dev/null || true" EXIT HUP INT TERM

        ready=0
        attempt=0
        while [ "$attempt" -lt 30 ]; do
            if wget -q -O /dev/null http://127.0.0.1:18080/APKINDEX.tar.gz; then
                ready=1
                break
            fi
            attempt=$((attempt + 1))
            sleep 1
        done
        test "$ready" = 1

        /work/input/couch-confd integrations \
            --root /tmp/firetv-repository \
            --keys-dir /root/.abuild \
            install-repository couch-integration-firetv \
            --repository http://127.0.0.1:18080 >/out/repository-install.log 2>&1
        /work/input/couch-confd integrations --root /tmp/firetv-repository list \
            | grep -Fx "firetv $version"

        # An empty trust directory must reject the otherwise valid signed APK.
        mkdir -m 700 /tmp/untrusted-keys
        if /work/input/couch-confd integrations \
            --root /tmp/firetv-untrusted \
            --keys-dir /tmp/untrusted-keys \
            install-sideload "$apk_file" >/out/untrusted.log 2>&1; then
            echo "untrusted APK was unexpectedly admitted" >&2
            exit 1
        fi
        test ! -e /tmp/firetv-untrusted/state/firetv

        # Appending bytes invalidates the signed APK; it must not be admitted.
        cp "$apk_file" /tmp/firetv-tampered.apk
        printf "\001" >>/tmp/firetv-tampered.apk
        if /work/input/couch-confd integrations \
            --root /tmp/firetv-tampered \
            --keys-dir /root/.abuild \
            install-sideload /tmp/firetv-tampered.apk >/out/tampered.log 2>&1; then
            echo "tampered APK was unexpectedly admitted" >&2
            exit 1
        fi
        test ! -e /tmp/firetv-tampered/state/firetv

        sha256sum "$apk_file" /out/repository/armv7/APKINDEX.tar.gz \
            > /out/checksums.txt
        printf "%s\n" "signed sideload: passed" \
            "signed repository install: passed" \
            "untrusted signature rejection: passed" \
            "tampered APK rejection: passed" > /out/receipt.txt
    '

echo "Fire TV package admission smoke test passed: $output"
