; Faderust installer (NSIS). Built by the release workflow:
;   makensis -DVERSION=0.7.1 -DEXE=faderust.exe -DOUTFILE=Faderust_0.7.1_x64-setup.exe \
;            -DRESOURCES=<folder with the Stream Deck plugin files> installer\faderust.nsi
; Per user (no administrator rights), follows the Windows display language
; (English or German). `/S` installs silently, `/UPDATE` starts Faderust
; afterwards (the in-app updater passes both).

Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !error "pass -DVERSION=x.y.z"
!endif
!ifndef EXE
  !error "pass -DEXE=<path to faderust.exe>"
!endif
!ifndef OUTFILE
  !define OUTFILE "Faderust_${VERSION}_x64-setup.exe"
!endif

!define APP "Faderust"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}"
!define RUNKEY "Software\Microsoft\Windows\CurrentVersion\Run"

Name "${APP}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\${APP}"
InstallDirRegKey HKCU "Software\${APP}" "InstallDir"
RequestExecutionLevel user
BrandingText "${APP} ${VERSION}"
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "FileDescription" "${APP} setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "MIT"

!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "nsis-header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "nsis-sidebar.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "nsis-sidebar.bmp"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\faderust.exe"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; The first language is the fallback; NSIS picks the Windows language when
; it is one of these.
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "German"

; Faderust holds its files while it runs (also in the tray).
!macro StopFaderust
  nsExec::Exec 'taskkill /F /IM faderust.exe'
  Pop $0
  Sleep 700
!macroend

Section "Faderust" SecMain
  !insertmacro StopFaderust
  SetOutPath "$INSTDIR"
  File "/oname=faderust.exe" "${EXE}"
  !ifdef RESOURCES
    SetOutPath "$INSTDIR\resources"
    File /r "${RESOURCES}\*.*"
  !endif
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\faderust.exe"

  WriteRegStr HKCU "Software\${APP}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "Niklas970"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\faderust.exe"
  WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" 1
SectionEnd

Function .onInstSuccess
  ${GetParameters} $R0
  ${GetOptions} $R0 "/UPDATE" $R1
  ${IfNot} ${Errors}
    ; After an in-app update: back up, in the tray like before.
    Exec '"$INSTDIR\faderust.exe" --minimized'
  ${EndIf}
FunctionEnd

Section "Uninstall"
  !insertmacro StopFaderust
  Delete "$SMPROGRAMS\${APP}.lnk"
  DeleteRegValue HKCU "${RUNKEY}" "faderust"
  DeleteRegKey HKCU "${UNINSTKEY}"
  DeleteRegKey HKCU "Software\${APP}"
  Delete "$INSTDIR\faderust.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /r "$INSTDIR\resources"
  RMDir "$INSTDIR"
SectionEnd
