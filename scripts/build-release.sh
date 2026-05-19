#!/usr/bin/env bash
# Build Cerulean Ledger release binary on a disposable EC2 c5.2xlarge.
#
# Usage:
#   ./scripts/build-release.sh              # Build + upload to S3
#   ./scripts/build-release.sh --deploy     # Build + upload + deploy to prod EC2
#
# Cost: ~$0.03 per build (c5.2xlarge spot, ~5 min)
#
# Prerequisites:
#   - AWS CLI configured (aws sts get-caller-identity)
#   - SSH key: ~/.ssh/rust-bc-test.pem

set -euo pipefail

REGION="us-east-1"
SUBNET="subnet-0925d44e76529e2a3"
SG="sg-0e0b542853c5db142"
KEY_NAME="rust-bc-test"
SSH_KEY="$HOME/.ssh/rust-bc-test.pem"
INSTANCE_TYPE="c5.2xlarge"
AMI="ami-0c7217cdde317cfec"  # Amazon Linux 2023 x86_64 us-east-1
S3_BUCKET="ceruleanledger-releases"
VERSION="${VERSION:-$(date +%Y%m%d-%H%M%S)}"
BINARY_NAME="cerulean-node-linux-amd64"
S3_KEY="releases/${VERSION}/${BINARY_NAME}"

# Prod EC2
PROD_HOST="52.91.18.180"
PROD_USER="ec2-user"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

echo "=== Cerulean Ledger Release Build ==="
echo "  Version:  $VERSION"
echo "  Instance: $INSTANCE_TYPE (spot)"
echo "  S3:       s3://$S3_BUCKET/$S3_KEY"
echo ""

# 1. Ensure S3 bucket exists
if ! aws s3 ls "s3://$S3_BUCKET" --region "$REGION" 2>/dev/null; then
    echo "Creating S3 bucket: $S3_BUCKET"
    aws s3 mb "s3://$S3_BUCKET" --region "$REGION"
fi

# 2. Create source tarball
echo "=== Packaging source ==="
TAR="/tmp/cerulean-src-${VERSION}.tar.gz"
tar czf "$TAR" \
    --exclude=target --exclude=.git --exclude=node_modules \
    --exclude=block-explorer-vite --exclude=cerulean-voto \
    --exclude=dist --exclude='*.pdf' .
echo "  Source: $(du -h "$TAR" | cut -f1)"

# 3. Launch spot instance
echo "=== Launching build instance ==="
INSTANCE_ID=$(aws ec2 run-instances \
    --region "$REGION" \
    --image-id "$AMI" \
    --instance-type "$INSTANCE_TYPE" \
    --key-name "$KEY_NAME" \
    --subnet-id "$SUBNET" \
    --security-group-ids "$SG" \
    --associate-public-ip-address \
    --instance-market-options '{"MarketType":"spot","SpotOptions":{"SpotInstanceType":"one-time"}}' \
    --block-device-mappings '[{"DeviceName":"/dev/xvda","Ebs":{"VolumeSize":30,"VolumeType":"gp3"}}]' \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=cerulean-build-${VERSION}}]" \
    --query 'Instances[0].InstanceId' --output text)

echo "  Instance: $INSTANCE_ID"
echo "  Waiting for running state..."

aws ec2 wait instance-running --region "$REGION" --instance-ids "$INSTANCE_ID"

BUILD_HOST=$(aws ec2 describe-instances --region "$REGION" \
    --instance-ids "$INSTANCE_ID" \
    --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)

echo "  IP: $BUILD_HOST"
echo "  Waiting for SSH..."

# Wait for SSH (max 60s)
for i in $(seq 1 12); do
    if ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "ec2-user@$BUILD_HOST" "echo ok" 2>/dev/null; then
        break
    fi
    sleep 5
done

# 4. Setup build environment
echo "=== Setting up build environment ==="
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "ubuntu@$BUILD_HOST" "
    sudo apt-get update && sudo apt-get install -y gcc g++ make clang libclang-dev llvm-dev libssl-dev protobuf-compiler pkg-config perl 2>/dev/null
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly-2025-05-01
"

# 5. Upload source
echo "=== Uploading source ==="
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$TAR" "ubuntu@$BUILD_HOST:~/src.tar.gz"

# 6. Build
echo "=== Compiling (this takes ~5 min on c5.2xlarge) ==="
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "ubuntu@$BUILD_HOST" "
    source \$HOME/.cargo/env
    mkdir -p ~/build && cd ~/build
    tar xzf ~/src.tar.gz
    cargo build --release --bin rust-bc 2>&1 | tail -5
    ls -lh target/release/rust-bc
    file target/release/rust-bc
"

# 7. Download binary
echo "=== Downloading binary ==="
mkdir -p "$REPO_ROOT/dist"
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
    "ubuntu@$BUILD_HOST:~/build/target/release/rust-bc" \
    "$REPO_ROOT/dist/$BINARY_NAME"

chmod +x "$REPO_ROOT/dist/$BINARY_NAME"
SIZE=$(du -h "$REPO_ROOT/dist/$BINARY_NAME" | cut -f1)
echo "  Binary: $REPO_ROOT/dist/$BINARY_NAME ($SIZE)"

# 8. Upload to S3
echo "=== Uploading to S3 ==="
aws s3 cp "$REPO_ROOT/dist/$BINARY_NAME" "s3://$S3_BUCKET/$S3_KEY" --region "$REGION"
echo "  s3://$S3_BUCKET/$S3_KEY"

# 9. Terminate build instance
echo "=== Terminating build instance ==="
aws ec2 terminate-instances --region "$REGION" --instance-ids "$INSTANCE_ID" > /dev/null
echo "  $INSTANCE_ID terminated"

# Cleanup
rm -f "$TAR"

echo ""
echo "=== Build complete ==="
echo "  Binary: dist/$BINARY_NAME ($SIZE)"
echo "  S3:     s3://$S3_BUCKET/$S3_KEY"
echo "  Cost:   ~\$0.03"

# 10. Optional deploy to prod
if [[ "${1:-}" == "--deploy" ]]; then
    echo ""
    echo "=== Deploying to production ($PROD_HOST) ==="

    scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
        "$REPO_ROOT/dist/$BINARY_NAME" "$PROD_USER@$PROD_HOST:~/cerulean-node-new"

    ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$PROD_USER@$PROD_HOST" "
        chmod +x ~/cerulean-node-new
        cd ~/rust-bc
        docker compose -f docker-compose.sandbox.yml down
        # Replace binary in the image by rebuilding with prebuilt binary
        docker compose -f docker-compose.sandbox.yml up -d
    "

    echo "=== Deploy complete ==="
fi
