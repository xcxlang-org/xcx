param (
    [string]$ProjectRoot,
    [string]$TempDir,
    [int]$ParentPid = 0
)

if ($ParentPid -gt 0) {
    Wait-Process -Id $ParentPid -ErrorAction SilentlyContinue
}
else {
    Start-Sleep -Seconds 3
}
cmd.exe /c rmdir /s /q "$ProjectRoot\test_output"
cmd.exe /c del /s /q "$ProjectRoot\*.db" "$ProjectRoot\*.db-journal" "$ProjectRoot\*.db-wal" "$ProjectRoot\*.db-shm" >$null 2>&1
cmd.exe /c del /s /q "$TempDir\*.db" "$TempDir\*.db-journal" "$TempDir\*.db-wal" "$TempDir\*.db-shm" >$null 2>&1
