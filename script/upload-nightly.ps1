[CmdletBinding()]
Param(
    [Parameter()][string]$Architecture,
    [Parameter()][string]$ReleaseTag
)

$ErrorActionPreference = "Stop"
. "$PSScriptRoot\lib\workspace.ps1"

if (-not $ReleaseTag) {
    $ReleaseTag = $env:GITHUB_REF_NAME
}

$artifacts = Get-ChildItem -Path "./release-artifacts/*" -File -ErrorAction SilentlyContinue

foreach ($file in $artifacts) {
    Write-Host "Uploading $($file.Name) to GitHub release $ReleaseTag"
    gh release upload $ReleaseTag $file.FullName --clobber
}
