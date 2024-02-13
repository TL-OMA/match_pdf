; Script generated by the Inno Setup Script Wizard.
; SEE THE DOCUMENTATION FOR DETAILS ON CREATING INNO SETUP SCRIPT FILES!

#define MyAppName "MatchPDF"
#define MyAppVersion "1.0.1"
#define MyAppPublisher "MatchPDF"
#define MyAppURL "http://www.matchpdf.com/"
#define MyAppExeName "match_pdf.exe"
#define MyDateTimeString GetDateTimeString('yyyy-mm-dd_hh-nn-ss', '', '')

[Setup]
; NOTE: The value of AppId uniquely identifies this application. Do not use the same AppId value in installers for other applications.
; (To generate a new GUID, click Tools | Generate GUID inside the IDE.)
AppId={{755B9C4E-AE3D-4832-970A-69AC4E350DBE}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
;AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=C:\repos\match_pdf\licenseFiles\MatchPDFLicense.txt
InfoBeforeFile=C:\repos\match_pdf\installerFiles\beforeInstallation.txt
InfoAfterFile=C:\repos\match_pdf\installerFiles\afterInstallation.txt
; Uncomment the following line to run in non administrative install mode (install for current user only.)
;PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=C:\MatchPDF Installers\{#MyDateTimeString}
OutputBaseFilename=MatchPDF_{#MyAppVersion}_Installer
SetupIconFile=C:\temp\installerFiles\matchPDF.ico
Compression=lzma
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "C:\repos\match_pdf\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\repos\match_pdf\target\release\pdfium.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\repos\match_pdf\licenseFiles\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\repos\match_pdf\licenseFiles\MatchPDFLicense.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\repos\match_pdf\licenseFiles\NOTICE"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\repos\match_pdf\licenseFiles\Readme.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\temp\installerFiles\UserGuide.pdf"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\temp\installerFiles\example1.pdf"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\temp\installerFiles\example2.pdf"; DestDir: "{app}"; Flags: ignoreversion
; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

