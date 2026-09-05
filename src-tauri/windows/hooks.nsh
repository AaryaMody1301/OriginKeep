!define ORIGINKEEP_EXTENSION_ID "mplmkmbnahpggimgfihfgieamonbbobh"
!define ORIGINKEEP_NATIVE_HOST "com.originkeep.host"

!macro NSIS_HOOK_POSTINSTALL
  FileOpen $0 "$INSTDIR\com.originkeep.host.json" w
  FileWrite $0 '{"name":"${ORIGINKEEP_NATIVE_HOST}","description":"OriginKeep local provenance capture host","path":"originkeep-native-host.exe","type":"stdio","allowed_origins":["chrome-extension://${ORIGINKEEP_EXTENSION_ID}/"]}'
  FileClose $0

  WriteRegStr HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}" "" "$INSTDIR\com.originkeep.host.json"
  WriteRegStr HKCU "Software\Google\Chrome\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}" "" "$INSTDIR\com.originkeep.host.json"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}"
  DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\${ORIGINKEEP_NATIVE_HOST}"
  Delete "$INSTDIR\com.originkeep.host.json"
!macroend
