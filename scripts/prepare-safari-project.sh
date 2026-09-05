#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Safari Web Extension packaging requires macOS with Xcode." >&2
  exit 1
fi

npm run prepare:browsers
rm -rf safari/generated
mkdir -p safari/generated

xcrun safari-web-extension-packager browser-packages/chromium \
  --project-location safari/generated \
  --app-name "OriginKeep Companion" \
  --bundle-identifier "com.originkeep.safari" \
  --swift

echo
echo "Safari project generated under safari/generated."
echo "Replace the generated SafariWebExtensionHandler.swift with safari/SafariWebExtensionHandler.swift."
echo "Then build/test in Xcode. Public distribution requires Apple signing/notarization."
