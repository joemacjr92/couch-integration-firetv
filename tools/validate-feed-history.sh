#!/usr/bin/env bash
set -Eeuo pipefail
usage() { echo "Usage: $0 FEED_ARCH_DIR REPOSITORY_PUBLIC_KEY NEW_VERSION" >&2; exit 64; }
[[ "$#" == 3 ]] || usage
FEED_DIR="$1"; PUBLIC_KEY="$2"; NEW_VERSION="$3"
FEED="$FEED_DIR/feed.json"; SIGNATURE="$FEED_DIR/feed.json.sig"; INDEX="$FEED_DIR/APKINDEX.tar.gz"
[[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "ERROR: invalid release version: $NEW_VERSION" >&2; exit 1; }
for file in "$FEED" "$SIGNATURE" "$INDEX" "$PUBLIC_KEY"; do
  [[ -s "$file" ]] || { echo "ERROR: missing or empty file: $file" >&2; exit 1; }
done
command -v jq >/dev/null; command -v openssl >/dev/null; command -v sha256sum >/dev/null
openssl dgst -sha256 -verify "$PUBLIC_KEY" -signature "$SIGNATURE" "$FEED" >/dev/null || {
  echo "ERROR: existing feed signature is invalid." >&2; exit 1;
}
jq -e '
  .schema == 1 and .channel == "preview" and
  (.sequence | type == "number" and . >= 1 and . == floor and . < 2147483647) and
  .index.path == "APKINDEX.tar.gz" and
  (.index.size | type == "number" and . > 0) and
  (.index.sha256 | test("^[0-9a-f]{64}$")) and
  (.packages | type == "array" and length > 0) and
  all(.packages[];
    .id == "firetv" and .protocol_version == 3 and .min_core_protocol_version == 3 and
    (.version | type == "string" and test("^[0-9]+\\.[0-9]+\\.[0-9]+$")) and
    (.apk == ("couch-integration-firetv-" + .version + "-r0.apk")) and
    (.size | type == "number" and . > 0) and (.sha256 | test("^[0-9a-f]{64}$"))
  )
' "$FEED" >/dev/null || { echo "ERROR: existing feed metadata failed validation." >&2; exit 1; }
[[ "$(stat -c '%s' "$INDEX")" == "$(jq -r '.index.size' "$FEED")" ]]
[[ "$(sha256sum "$INDEX" | awk '{print $1}')" == "$(jq -r '.index.sha256' "$FEED")" ]]
while IFS=$'\t' read -r APK_NAME EXPECTED_SIZE EXPECTED_SHA; do
  APK="$FEED_DIR/$APK_NAME"
  [[ -s "$APK" ]] || { echo "ERROR: historical APK missing: $APK_NAME" >&2; exit 1; }
  [[ "$(stat -c '%s' "$APK")" == "$EXPECTED_SIZE" ]] || { echo "ERROR: historical APK size mismatch: $APK_NAME" >&2; exit 1; }
  [[ "$(sha256sum "$APK" | awk '{print $1}')" == "$EXPECTED_SHA" ]] || { echo "ERROR: historical APK hash mismatch: $APK_NAME" >&2; exit 1; }
done < <(jq -r '.packages[] | [.apk, (.size|tostring), .sha256] | @tsv' "$FEED")
mapfile -t VERSIONS < <(jq -r '.packages[].version' "$FEED" | sort -V)
UNIQUE_COUNT="$(printf '%s\n' "${VERSIONS[@]}" | sort -u | wc -l)"
[[ "$UNIQUE_COUNT" -eq "${#VERSIONS[@]}" ]] || { echo "ERROR: duplicate versions exist in feed history." >&2; exit 1; }
HIGHEST="${VERSIONS[-1]}"
LATEST="$(printf '%s\n' "$HIGHEST" "$NEW_VERSION" | sort -V | tail -n 1)"
[[ "$LATEST" == "$NEW_VERSION" && "$NEW_VERSION" != "$HIGHEST" ]] || {
  echo "ERROR: release $NEW_VERSION must be newer than existing version $HIGHEST." >&2; exit 1;
}
echo "Existing Fire TV feed signature, metadata, APKINDEX, APK history, and version order verified."
