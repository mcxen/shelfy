!macro NSIS_HOOK_PREINSTALL
  ; Attempt to close a running Shelfy instance gracefully before installing.
  ; First try a soft terminate, wait a moment, then force-kill if still running.
  ExecWait '"$SYSDIR\taskkill.exe" /IM shelfy.exe /T' $0
  Sleep 500
  ExecWait '"$SYSDIR\taskkill.exe" /F /IM shelfy.exe /T' $0
!macroend
