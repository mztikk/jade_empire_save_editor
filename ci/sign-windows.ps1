# Sign the staged Windows executables with Authenticode.
#
# Requires a base64 PKCS#12 certificate. The workflow only calls this when the
# certificate secret is configured; see ci/README.md.
param(
    [Parameter(Mandatory = $true)]
    [string] $Dist
)

$ErrorActionPreference = "Stop"
if (-not $env:WINDOWS_CERTIFICATE) { throw "WINDOWS_CERTIFICATE is not set" }
if (-not $env:WINDOWS_CERTIFICATE_PASSWORD) { throw "WINDOWS_CERTIFICATE_PASSWORD is not set" }

$signingDir = Join-Path $env:RUNNER_TEMP "jadeforge-signing"
New-Item -ItemType Directory -Force -Path $signingDir | Out-Null
$certificate = Join-Path $signingDir "certificate.pfx"
[IO.File]::WriteAllBytes(
    $certificate,
    [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE)
)

$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" `
    -Filter signtool.exe -Recurse |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $signtool) { throw "signtool.exe was not found in the Windows SDK" }

try {
    Get-ChildItem -Path $Dist -Recurse -Filter *.exe | ForEach-Object {
        Write-Host "signing $($_.FullName)"
        & $signtool.FullName sign /fd SHA256 /f $certificate `
            /p $env:WINDOWS_CERTIFICATE_PASSWORD `
            /tr http://timestamp.digicert.com /td SHA256 $_.FullName
        if ($LASTEXITCODE -ne 0) { throw "signtool failed for $($_.FullName)" }
    }
} finally {
    Remove-Item -Recurse -Force $signingDir -ErrorAction SilentlyContinue
}
