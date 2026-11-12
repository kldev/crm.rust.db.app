#!/bin/sh
# One-shot init (compose service rustfs-init): creates the attachments bucket through the plain S3 API, SigV4-signed by
# curl. Idempotent - 409 "already exists" is success - and patient: retries while RustFS is still starting.
set -eu

: "${S3_ENDPOINT:?}" "${S3_ACCESS_KEY:?}" "${S3_SECRET_KEY:?}" "${S3_BUCKET:?}" "${S3_REGION:=us-east-1}"
sign="aws:amz:${S3_REGION}:s3"

for attempt in $(seq 1 60); do
  code=$(curl -s -o /tmp/out -w '%{http_code}' -X PUT \
    --aws-sigv4 "$sign" --user "${S3_ACCESS_KEY}:${S3_SECRET_KEY}" \
    "${S3_ENDPOINT}/${S3_BUCKET}" || true)
  case "$code" in
    200) echo "Bucket ${S3_BUCKET} created (or already ours)"; break ;;
    409) echo "Bucket ${S3_BUCKET} already exists"; break ;;
    000|502|503) echo "S3 not ready yet (HTTP $code, attempt $attempt)"; sleep 2 ;;
    *) echo "CreateBucket failed with HTTP $code"; cat /tmp/out; exit 1 ;;
  esac
  [ "$attempt" -eq 60 ] && { echo "S3 did not become ready"; exit 1; }
done

# The bucket stays private: the app streams files to signed-in agents itself, nothing is served from S3 directly.
code=$(curl -s -o /tmp/out -w '%{http_code}' --head \
  --aws-sigv4 "$sign" --user "${S3_ACCESS_KEY}:${S3_SECRET_KEY}" "${S3_ENDPOINT}/${S3_BUCKET}")
[ "$code" = 200 ] || { echo "Bucket ${S3_BUCKET} is not reachable with these keys (HTTP $code)"; exit 1; }
echo "Bucket ${S3_BUCKET} ready"
