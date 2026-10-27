# kilna-plugin-cut installer for Windows:
#   irm https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "lacodda/kilna-plugin-cut"

# The tag comes from the /releases/latest redirect rather than the REST API:
# unauthenticated API calls are capped at 60 per hour per IP, and an installer
# that fails because someone else on the same address ran it is no installer.
# $env:KILNA_PLUGIN_CUT_VERSION pins a specific release.
$tag = $env:KILNA_PLUGIN_CUT_VERSION
if (-not $tag) {
    $request = [Net.HttpWebRequest]::Create("https://github.com/$repo/releases/latest")
    $request.AllowAutoRedirect = $false
    $request.UserAgent = "kilna-plugin-cut-installer"
    try {
        $response = $request.GetResponse()
        $tag = ($response.Headers["Location"] -split "/")[-1]
        $response.Close()
    } catch {
        throw "Cannot resolve the latest release of ${repo}: $($_.Exception.Message)"
    }
}
if (-not $tag -or $tag -notmatch '^v\d') {
    throw "Cannot resolve the latest release of $repo - set `$env:KILNA_PLUGIN_CUT_VERSION to a tag like v0.1.0"
}

# kilna looks for plugins in the plugins folder beside its workspace.
$dir = if ($env:KILNA_PLUGIN_CUT_DIR) { $env:KILNA_PLUGIN_CUT_DIR } else { Join-Path $env:APPDATA "com.lacodda.kilna\plugins" }
$name = "kilna-plugin-cut-$tag-x86_64-pc-windows-msvc"
$url = "https://github.com/$repo/releases/download/$tag/$name.zip"
$tmp = Join-Path ([IO.Path]::GetTempPath()) "kilna-plugin-cut-install-$([guid]::NewGuid())"
New-Item -ItemType Directory -Force $tmp | Out-Null

try {
    Write-Host "Downloading $url"
    Invoke-WebRequest $url -OutFile (Join-Path $tmp "plugin.zip")
    Expand-Archive (Join-Path $tmp "plugin.zip") -DestinationPath $tmp -Force
    $exe = Get-ChildItem $tmp -Recurse -Filter "kilna-plugin-cut.exe" | Select-Object -First 1
    if (-not $exe) { throw "The release archive holds no kilna-plugin-cut.exe" }
    New-Item -ItemType Directory -Force $dir | Out-Null
    Copy-Item $exe.FullName $dir -Force
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "Installed kilna-plugin-cut $tag to $dir\kilna-plugin-cut.exe"

# The settings stay as the person wrote them: an example is written only when
# there is none, and never over one.
$settings = Join-Path $dir "kilna-plugin-cut.json"
if (-not (Test-Path $settings)) {
    $example = @'
{
  "channel": "",
  "call": "FULL VERSION",
  "cover_seconds": 0.5
}
'@
    [IO.File]::WriteAllText($settings, $example, (New-Object Text.UTF8Encoding $false))
    Write-Host "Wrote $settings - put your channel's name in it."
}

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "Note: ffmpeg is not on your PATH; the plugin needs it (winget install Gyan.FFmpeg)."
}
Write-Host "In kilna, 'Cut the short' now stands on a short's Cuts tab."
