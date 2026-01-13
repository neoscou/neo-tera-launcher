# Modern NSIS Installer for Neolithic TERA
# Requires NSIS 3.08+ with Modern UI 2

!include "MUI2.nsh"
!include "FileFunc.nsh"

# Installer settings
Name "Neolithic TERA"
OutFile "Output\NeolithicTERA-Setup.exe"
InstallDir "$PROGRAMFILES64"
InstallDirRegKey HKLM "Software\NeolithicTERA" "Install_Dir"
RequestExecutionLevel admin

# Ensure directory page doesn't auto-append folder name
!define MUI_PAGE_CUSTOMFUNCTION_SHOW DirectoryShow
DirText "Choose the folder where you want to install Neolithic TERA. The files will be installed directly into the folder you select."

# Modern UI Configuration
!define MUI_ABORTWARNING
!define MUI_ICON "..\teralaunch\src-tauri\icons\logo.ico"
!define MUI_UNICON "..\teralaunch\src-tauri\icons\logo.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
# !define MUI_WELCOMEFINISHPAGE_BITMAP "wizard.bmp"
# !define MUI_UNWELCOMEFINISHPAGE_BITMAP "wizard.bmp"

# Welcome page
!define MUI_WELCOMEPAGE_TITLE "Welcome to Neolithic TERA Setup"
!define MUI_WELCOMEPAGE_TEXT "This wizard will guide you through the installation of Neolithic TERA.$\r$\n$\r$\nClick Next to continue."
!insertmacro MUI_PAGE_WELCOME

# License page (optional - can be removed)
# !insertmacro MUI_PAGE_LICENSE "license.txt"

# Directory page
!define MUI_DIRECTORYPAGE_TEXT_TOP "Choose the folder in which to install Neolithic TERA."
!insertmacro MUI_PAGE_DIRECTORY

# Installing page
!insertmacro MUI_PAGE_INSTFILES

# Finish page
!define MUI_FINISHPAGE_RUN "$INSTDIR\Neolithic TERA Launcher.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Neolithic TERA"
!define MUI_FINISHPAGE_TITLE "Installation Complete"
!define MUI_FINISHPAGE_TEXT "Neolithic TERA has been installed on your computer.$\r$\n$\r$\nClick Finish to close this wizard."
!insertmacro MUI_PAGE_FINISH

# Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

# Language
!insertmacro MUI_LANGUAGE "English"

# Custom function to prevent auto-appending folder name
Function DirectoryShow
  # This ensures the directory chosen is used exactly as-is
FunctionEnd

# Version Info
VIProductVersion "1.1.0.0"
VIAddVersionKey "ProductName" "Neolithic TERA"
VIAddVersionKey "CompanyName" "Neolithic TERA"
VIAddVersionKey "FileDescription" "Neolithic TERA Installer"
VIAddVersionKey "FileVersion" "1.1.0"
VIAddVersionKey "ProductVersion" "1.1.0"
VIAddVersionKey "LegalCopyright" "© 2026 Neolithic TERA"

# Installer Section
Section "Neolithic TERA" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  
  DetailPrint "Installing launcher..."
  File "SourceFiles\Neolithic TERA Launcher.exe"
  File "SourceFiles\file_cache.json"
  
  DetailPrint "Installing Binaries..."
  SetOutPath "$INSTDIR\Binaries"
  File /r "SourceFiles\Binaries\*.*"
  
  DetailPrint "Installing Engine..."
  SetOutPath "$INSTDIR\Engine"
  File /r "SourceFiles\Engine\*.*"
  
  SetOutPath "$INSTDIR"
  
  # Write uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  
  # Create shortcuts
  CreateDirectory "$SMPROGRAMS\Neolithic TERA"
  CreateShortcut "$SMPROGRAMS\Neolithic TERA\Neolithic TERA.lnk" "$INSTDIR\Neolithic TERA Launcher.exe"
  CreateShortcut "$SMPROGRAMS\Neolithic TERA\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\Neolithic TERA.lnk" "$INSTDIR\Neolithic TERA Launcher.exe"
  
  # Registry entries
  WriteRegStr HKLM "Software\NeolithicTERA" "Install_Dir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "DisplayName" "Neolithic TERA"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "DisplayIcon" "$INSTDIR\Neolithic TERA Launcher.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "Publisher" "Neolithic TERA"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "DisplayVersion" "1.1.0"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "NoRepair" 1
  
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA" "EstimatedSize" "$0"
SectionEnd

# Uninstaller Section
Section "Uninstall"
  # Only remove files that WE installed - never use wildcard deletion!
  
  # Remove launcher files
  Delete "$INSTDIR\Neolithic TERA Launcher.exe"
  Delete "$INSTDIR\file_cache.json"
  Delete "$INSTDIR\Uninstall.exe"
  
  # Remove config files created by launcher
  Delete "$INSTDIR\tera_config.ini"
  Delete "$INSTDIR\debug.log"
  Delete "$INSTDIR\hash-file.json"
  
  # Remove only the folders WE created
  RMDir /r "$INSTDIR\Binaries"
  RMDir /r "$INSTDIR\Engine"
  RMDir /r "$INSTDIR\S1Game"
  
  # Remove shortcuts
  Delete "$SMPROGRAMS\Neolithic TERA\*.*"
  RMDir "$SMPROGRAMS\Neolithic TERA"
  Delete "$DESKTOP\Neolithic TERA.lnk"
  
  # Remove registry keys
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NeolithicTERA"
  DeleteRegKey HKLM "Software\NeolithicTERA"
  
  # Only remove the installation directory if it's empty (safety check)
  # This will fail silently if there are other files in the directory
  RMDir "$INSTDIR"
SectionEnd
