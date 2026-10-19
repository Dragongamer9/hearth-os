#!/bin/bash
# Uploads an ISO (and its .sha256) to Hearth's Cloudflare R2 bucket under a fixed name, so the website's
# Download buttons always get the newest one, and records the version in latest.json.
#   tools/r2-upload.sh <iso file> <name in the bucket, e.g. hearth-os-live.iso> <version>
# Needs the repository secrets R2_ACCOUNT_ID, R2_ACCESS_KEY_ID and R2_SECRET_ACCESS_KEY (in the environment).
# Public address: https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/<name>
set -euo pipefail
file="$1"; name="$2"; version="$3"
BUCKET="${R2_BUCKET:-hearth-os}"
: "${R2_ACCOUNT_ID:?}" "${R2_ACCESS_KEY_ID:?}" "${R2_SECRET_ACCESS_KEY:?}"
export AWS_ACCESS_KEY_ID="$R2_ACCESS_KEY_ID" AWS_SECRET_ACCESS_KEY="$R2_SECRET_ACCESS_KEY" AWS_DEFAULT_REGION=auto
# R2 doesn't take the newer AWS CLI's default upload checksums on multipart uploads.
export AWS_REQUEST_CHECKSUM_CALCULATION=when_required AWS_RESPONSE_CHECKSUM_VALIDATION=when_required
s3() { aws s3 --endpoint-url "https://$R2_ACCOUNT_ID.r2.cloudflarestorage.com" "$@"; }
command -v aws >/dev/null || { pip install --quiet awscli || pip install --quiet --break-system-packages awscli; }

[ -f "$file.sha256" ] || (cd "$(dirname "$file")" && sha256sum "$(basename "$file")" > "$(basename "$file").sha256")
# The checksum file names the file the way people download it.
sed "s#  .*#  $name#" "$file.sha256" > /tmp/r2-sum
aws configure set default.s3.multipart_chunksize 64MB
s3 cp "$file" "s3://$BUCKET/$name" --content-type application/x-iso9660-image --no-progress
s3 cp /tmp/r2-sum "s3://$BUCKET/$name.sha256" --content-type text/plain --no-progress

# latest.json: which version each download is (the website can show it).
s3 cp "s3://$BUCKET/latest.json" /tmp/latest.json --no-progress 2>/dev/null || echo '{}' > /tmp/latest.json
python3 - "$name" "$version" "$(stat -c %s "$file")" <<'EOF'
import json, sys, time
name, version, size = sys.argv[1], sys.argv[2], int(sys.argv[3])
try: d = json.load(open('/tmp/latest.json'))
except Exception: d = {}
d[name] = {'version': version, 'size': size, 'date': time.strftime('%Y-%m-%d')}
json.dump(d, open('/tmp/latest.json', 'w'), indent=1)
EOF
s3 cp /tmp/latest.json "s3://$BUCKET/latest.json" --content-type application/json --cache-control "max-age=300" --no-progress
echo "Uploaded: https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/$name"
