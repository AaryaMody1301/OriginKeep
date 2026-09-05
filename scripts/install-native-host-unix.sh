#!/bin/sh
set -eu

HOST_NAME="com.originkeep.host"
CHROMIUM_ID="mplmkmbnahpggimgfihfgieamonbbobh"
FIREFOX_ID="originkeep@aaryamody.local"

if [ "$#" -ne 1 ]; then
  echo "usage: $0 /absolute/path/to/originkeep-native-host" >&2
  exit 2
fi

case "$1" in
  /*) HOST_PATH="$1" ;;
  *) echo "native host path must be absolute" >&2; exit 2 ;;
esac

if [ ! -x "$HOST_PATH" ]; then
  echo "native host is not executable: $HOST_PATH" >&2
  exit 2
fi

escape_json() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

ESCAPED_HOST_PATH=$(escape_json "$HOST_PATH")
CHROMIUM_MANIFEST=$(printf '{"name":"%s","description":"OriginKeep local provenance capture host","path":"%s","type":"stdio","allowed_origins":["chrome-extension://%s/"]}\n' "$HOST_NAME" "$ESCAPED_HOST_PATH" "$CHROMIUM_ID")
FIREFOX_MANIFEST=$(printf '{"name":"%s","description":"OriginKeep local provenance capture host","path":"%s","type":"stdio","allowed_extensions":["%s"]}\n' "$HOST_NAME" "$ESCAPED_HOST_PATH" "$FIREFOX_ID")

write_manifest() {
  directory="$1"
  manifest="$2"
  mkdir -p "$directory"
  printf '%s' "$manifest" > "$directory/$HOST_NAME.json"
  echo "registered $directory/$HOST_NAME.json"
}

case "$(uname -s)" in
  Darwin)
    write_manifest "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts" "$CHROMIUM_MANIFEST"
    write_manifest "$HOME/Library/Application Support/Chromium/NativeMessagingHosts" "$CHROMIUM_MANIFEST"
    write_manifest "$HOME/Library/Application Support/Microsoft Edge/NativeMessagingHosts" "$CHROMIUM_MANIFEST"
    write_manifest "$HOME/Library/Application Support/Mozilla/NativeMessagingHosts" "$FIREFOX_MANIFEST"
    ;;
  Linux)
    write_manifest "$HOME/.config/google-chrome/NativeMessagingHosts" "$CHROMIUM_MANIFEST"
    write_manifest "$HOME/.config/chromium/NativeMessagingHosts" "$CHROMIUM_MANIFEST"
    write_manifest "$HOME/.config/microsoft-edge/NativeMessagingHosts" "$CHROMIUM_MANIFEST"
    write_manifest "$HOME/.mozilla/native-messaging-hosts" "$FIREFOX_MANIFEST"
    ;;
  *)
    echo "unsupported platform for this installer" >&2
    exit 2
    ;;
esac
