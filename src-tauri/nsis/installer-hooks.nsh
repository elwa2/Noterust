; NoteRust NSIS hooks.
;
; Tauri's `bundle.fileAssociations` already writes the NoteRust.txt ProgID and
; points .txt/.log/.md/.csv at it. These hooks add the per-user registration
; that makes NoteRust appear in Explorer's "Open with" picker and in
; Settings > Default apps, and remove those keys again on uninstall.

!macro NSIS_HOOK_POSTINSTALL
  ; Windows 11 app picker / "Open with" entry.
  WriteRegStr HKCU "Software\Classes\Applications\NoteRust.exe" "FriendlyAppName" "NoteRust"
  WriteRegStr HKCU "Software\Classes\Applications\NoteRust.exe\DefaultIcon" "" '"$INSTDIR\NoteRust.exe",0'
  WriteRegStr HKCU "Software\Classes\Applications\NoteRust.exe\shell\open\command" "" '"$INSTDIR\NoteRust.exe" "%1"'

  WriteRegStr HKCU "Software\Classes\Applications\NoteRust.exe\SupportedTypes" ".txt" ""
  WriteRegStr HKCU "Software\Classes\Applications\NoteRust.exe\SupportedTypes" ".log" ""
  WriteRegStr HKCU "Software\Classes\Applications\NoteRust.exe\SupportedTypes" ".md" ""
  WriteRegStr HKCU "Software\Classes\Applications\NoteRust.exe\SupportedTypes" ".csv" ""

  ; Settings > Default apps registration.
  WriteRegStr HKCU "Software\NoteRust\Capabilities" "ApplicationName" "NoteRust"
  WriteRegStr HKCU "Software\NoteRust\Capabilities" "ApplicationDescription" "NoteRust text and Markdown editor"
  WriteRegStr HKCU "Software\NoteRust\Capabilities" "ApplicationIcon" '"$INSTDIR\NoteRust.exe",0'
  WriteRegStr HKCU "Software\NoteRust\Capabilities\FileAssociations" ".txt" "NoteRust.txt"
  WriteRegStr HKCU "Software\NoteRust\Capabilities\FileAssociations" ".log" "NoteRust.txt"
  WriteRegStr HKCU "Software\NoteRust\Capabilities\FileAssociations" ".md" "NoteRust.txt"
  WriteRegStr HKCU "Software\NoteRust\Capabilities\FileAssociations" ".csv" "NoteRust.txt"
  WriteRegStr HKCU "Software\RegisteredApplications" "NoteRust" "Software\NoteRust\Capabilities"

  ; The picker reads both keys.
  WriteRegStr HKCU "Software\Classes\.txt\OpenWithProgids" "NoteRust.txt" ""
  WriteRegStr HKCU "Software\Classes\.log\OpenWithProgids" "NoteRust.txt" ""
  WriteRegStr HKCU "Software\Classes\.md\OpenWithProgids" "NoteRust.txt" ""
  WriteRegStr HKCU "Software\Classes\.csv\OpenWithProgids" "NoteRust.txt" ""
  WriteRegStr HKCU "Software\Classes\.txt\OpenWithList" "NoteRust.exe" ""
  WriteRegStr HKCU "Software\Classes\.log\OpenWithList" "NoteRust.exe" ""
  WriteRegStr HKCU "Software\Classes\.md\OpenWithList" "NoteRust.exe" ""
  WriteRegStr HKCU "Software\Classes\.csv\OpenWithList" "NoteRust.exe" ""

  ; Right-click verb on every file type.
  WriteRegStr HKCU "Software\Classes\*\shell\NoteRust" "" "Open with NoteRust"
  WriteRegStr HKCU "Software\Classes\*\shell\NoteRust" "Icon" '"$INSTDIR\NoteRust.exe",0'
  WriteRegStr HKCU "Software\Classes\*\shell\NoteRust\command" "" '"$INSTDIR\NoteRust.exe" "%1"'

  ; Ask once on a fresh install (not when the in-app updater re-runs the setup).
  ${If} $UpdateMode <> 1
    WriteRegDWORD HKCU "Software\NoteRust" "ShowDefaultAppsPrompt" 1
  ${EndIf}

  ; Refresh Explorer so the new handler is listed immediately.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, i 0, i 0)'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegValue HKCU "Software\RegisteredApplications" "NoteRust"
  DeleteRegKey HKCU "Software\NoteRust\Capabilities"
  DeleteRegValue HKCU "Software\NoteRust" "ShowDefaultAppsPrompt"
  DeleteRegValue HKCU "Software\NoteRust" "OpenWithDisabled"
  DeleteRegKey /ifempty HKCU "Software\NoteRust"

  DeleteRegKey HKCU "Software\Classes\Applications\NoteRust.exe"
  DeleteRegKey HKCU "Software\Classes\*\shell\NoteRust"

  DeleteRegValue HKCU "Software\Classes\.txt\OpenWithProgids" "NoteRust.txt"
  DeleteRegValue HKCU "Software\Classes\.log\OpenWithProgids" "NoteRust.txt"
  DeleteRegValue HKCU "Software\Classes\.md\OpenWithProgids" "NoteRust.txt"
  DeleteRegValue HKCU "Software\Classes\.csv\OpenWithProgids" "NoteRust.txt"
  DeleteRegValue HKCU "Software\Classes\.txt\OpenWithList" "NoteRust.exe"
  DeleteRegValue HKCU "Software\Classes\.log\OpenWithList" "NoteRust.exe"
  DeleteRegValue HKCU "Software\Classes\.md\OpenWithList" "NoteRust.exe"
  DeleteRegValue HKCU "Software\Classes\.csv\OpenWithList" "NoteRust.exe"

  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, i 0, i 0)'
!macroend
