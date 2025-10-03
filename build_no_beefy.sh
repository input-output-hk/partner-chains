#!/bin/bash

# Script to build partner-chains-node without BEEFY features
# This builds from the master branch which doesn't have BEEFY

set -e

echo "🔨 Building partner-chains-node WITHOUT BEEFY features from master branch..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "demo" ]; then
    echo "❌ Error: Please run this script from the project root directory"
    exit 1
fi

# Save current branch
CURRENT_BRANCH=$(git branch --show-current)
echo "📍 Current branch: $CURRENT_BRANCH"

# Switch to master branch
echo "🔄 Switching to master branch..."
git checkout master

# Build the node from master (no BEEFY)
echo "🔨 Building node from master branch (no BEEFY)..."
cargo build --release --bin partner-chains-demo-node

# Copy the binary
cp target/release/partner-chains-demo-node partner-chains-node-no-beefy

echo "✅ Binary created: partner-chains-node-no-beefy"

# Switch back to original branch
echo "🔄 Switching back to $CURRENT_BRANCH branch..."
git checkout $CURRENT_BRANCH

echo "🎯 Ready to test! Use 'partner-chains-node-no-beefy' for non-BEEFY testing"
echo "📊 This binary was built from master branch and should not have BEEFY functionality"