# Scripted RepoPact demo (see docs/demo.md). Safe to run; uses a temp directory.
$ErrorActionPreference = "Continue"
$here = Split-Path -Parent $PSScriptRoot
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("repopact-demo-" + [System.Guid]::NewGuid())
function Step($t) { Write-Host "`n$ $t" -ForegroundColor Cyan }

try {
    Step "python scripts/init_repo.py --target $tmp/demo"
    python "$here/scripts/init_repo.py" --target "$tmp/demo"

    Set-Location "$tmp/demo"
    Step "python scripts/validate_repo.py"
    python scripts/validate_repo.py

    Step 'python scripts/new.py work-item "Demo work"'
    python scripts/new.py work-item "Demo work"

    Step "python scripts/validate_repo.py   # active item with a pending criterion is fine"
    python scripts/validate_repo.py

    Step "mark the criterion satisfied WITHOUT evidence, then validate (expect failure)"
    $item = (Get-ChildItem work/active -Directory | Select-Object -First 1).FullName
    $manifest = Join-Path $item "work-item.json"
    $d = Get-Content $manifest -Raw | ConvertFrom-Json
    $d.acceptance_criteria[0].state = "satisfied"
    $d | ConvertTo-Json -Depth 10 | Set-Content $manifest
    python scripts/validate_repo.py
    if ($LASTEXITCODE -ne 0) { Write-Host "`nValidator rejected it — completion requires proof." -ForegroundColor Yellow }
}
finally {
    Set-Location $here
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
