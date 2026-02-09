#!/bin/bash

# Xcode Cloud post-clone script
# This script runs after the repository is cloned but before the build starts
# It installs XcodeGen and generates the Xcode project from project.yml

set -e

echo "=== Installing XcodeGen ==="
brew install xcodegen

echo "=== Generating Xcode Project ==="
cd "$CI_PRIMARY_REPOSITORY_PATH/ios_dashboard"
xcodegen generate

echo "=== Project generated successfully ==="
ls -la *.xcodeproj
