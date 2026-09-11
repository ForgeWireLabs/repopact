; WI062: narrowest possible NSIS hook to add a Desktop shortcut.
;
; Tauri's generated installer already creates the Start-menu shortcut and
; the uninstall registry entry on its own; this hook only adds the Desktop
; shortcut Jeremy explicitly expected, using Tauri's own installer
; variables ($INSTDIR, ${MAINBINARYNAME}) rather than any hard-coded
; development or Cargo target path, so it stays correct across machines
; and across per-user/per-machine install modes.
;
; See Tauri's NsisConfig.installerHooks documentation for the supported
; macro names (NSIS_HOOK_PREINSTALL/POSTINSTALL/PRE/POSTUNINSTALL).

!macro NSIS_HOOK_POSTINSTALL
  CreateShortcut "$DESKTOP\RepoPact Workbench.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$DESKTOP\RepoPact Workbench.lnk"
!macroend
