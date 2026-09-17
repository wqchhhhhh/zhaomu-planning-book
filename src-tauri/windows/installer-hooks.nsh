!macro NSIS_HOOK_POSTINSTALL
  CreateShortCut "$DESKTOP\朝暮.lnk" "$INSTDIR\planner.exe"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$DESKTOP\朝暮.lnk"
!macroend
