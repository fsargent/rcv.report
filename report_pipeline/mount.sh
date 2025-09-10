#!/bin/sh

set -e

# Create directories
mkdir -p raw-data
mkdir -p preprocessed

echo "Downloading election data..."
aws s3 sync \
    --no-sign-request \
    --exclude ".*" \
    --exclude "*/.*" \
    s3://raw.ranked.vote/ raw-data/

echo "Downloaded election data to raw-data/"


