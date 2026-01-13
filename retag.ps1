$packageJson = Get-Content -Path "$PSScriptRoot\package.json" | ConvertFrom-Json
$version = "v$($packageJson.version)"

Write-Host "Retagging $version..." -ForegroundColor Cyan
Write-Host ""
Write-Host "Remember to commit & push any changes you want included in the release." -ForegroundColor Yellow
Write-Host ""
Read-Host "Press Enter to continue or Ctrl+C to cancel"

git tag -d $version
git push origin --delete $version
git tag $version
git push origin $version

Write-Host "Done! Tag $version has been recreated and pushed." -ForegroundColor Green
