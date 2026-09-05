!define ORIGINKEEP_EXTENSION_ID "mplmkmbnahpggimgfihfgieamonbbobh"
!define ORIGINKEEP_FIREFOX_ID "originkeep@aaryamody1301.github.io"
!define ORIGINKEEP_NATIVE_HOST "com.originkeep.host"

!macro NSIS_HOOK_POSTINSTALL
  Delete "$INSTDIR\com.originkeep.host.json"

  FileOpen $0 "$INSTDIR\com.originkeep.host.chromium.json" w
  FileWrite $0 '{"name":"${ORIGINKEEP_NATIVE_HOST}","description":"OriginKeep local provenance capture host","path":"originkeep-native-host.exe","type":"stdio","allowed_origins":["chrome-extension://${ORIGINKEEP_EXTENSION_ID}/"]}'
  FileClose $0

  FileOpen $0 "$INSTDIR\com.originkeep.host.firefox.json" w
  FileWrite $0 '{"name":"${ORIGINKEEP_NATIVE_HOST}","description":"OriginKeep local provenance capture host","path":"originkeep-native-host.exe","type":"stdio","allowed_extensions":["${ORIGINKEEP_FIREFOX_ID}"]}'
  FileClose $0

  WriteRegStr HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}" "" "$INSTDIR\com.originkeep.host.chromium.json"
  WriteRegStr HKCU "Software\Google\Chrome\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}" "" "$INSTDIR\com.originkeep.host.chromium.json"
  WriteRegStr HKCU "Software\Mozilla\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}" "" "$INSTDIR\com.originkeep.host.firefox.json"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}"
  DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}"
  DeleteRegKey HKCU "Software\Mozilla\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}"
  Delete "$INSTDIR\com.originkeep.host.json"
  Delete "$INSTDIR\com.originkeep.host.chromium.json"
  Delete "$INSTDIR\com.originkeep.host.firefox.json"
!macroend
