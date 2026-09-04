param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[a-p]{32}$')]
  [string]$ExtensionId,

  [Parameter(Mandatory = $true)]
  [string]$HostPath
)

$resolvedHost = (Resolve-Path -LiteralPath $HostPath -ErrorAction Stop).Path
$manifestDirectory = Join-Path $env:LOCALAPPDATA "OriginKeep\native-messaging"
$manifestPath = Join-Path $manifestDirectory "com.originkeep.host.json"
New-Item -ItemType Directory -Force -Path $manifestDirectory | Out-Null

$manifest = @{
  name = "com.originkeep.host"
  description = "OriginKeep local provenance capture host"
  path = $resolvedHost
  type = "stdio"
  allowed_origins = @("chrome-extension://$ExtensionId/")
}

$manifest | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $manifestPath

$registries = @(
  "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.originkeep.host",
  "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.originkeep.host"
)

foreach ($registry in $registries) {
  New-Item -Path $registry -Force | Out-Null
  Set-Item -Path $registry -Value $manifestPath
}

Write-Host "OriginKeep native host registered for Chrome and Edge."
Write-Host "Manifest: $manifestPath"
Write-Host "Host: $resolvedHost"
