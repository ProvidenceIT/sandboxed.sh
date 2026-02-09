#!/bin/bash

# Xcode Cloud post-clone script
# This script runs after the repository is cloned but before the build starts
# It installs XcodeGen and generates the Xcode project from project.yml

set -e

echo "=== Installing XcodeGen ==="
if command -v xcodegen >/dev/null 2>&1; then
  echo "XcodeGen already installed: $(xcodegen --version)"
else
  # Avoid Homebrew auto-update (which can fail on ghcr.io in Xcode Cloud).
  export HOMEBREW_NO_AUTO_UPDATE=1
  export HOMEBREW_NO_ENV_HINTS=1
  export HOMEBREW_NO_INSTALL_FROM_API=1
  brew install xcodegen
fi

echo "=== Generating Xcode Project ==="
cd "$CI_PRIMARY_REPOSITORY_PATH/ios_dashboard"
xcodegen generate

echo "=== Project generated successfully ==="
ls -la *.xcodeproj
