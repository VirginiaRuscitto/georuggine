# ============================================================
# Local OSRM setup via Docker (Windows / PowerShell, no WSL needed)
# Extract: Turin city (small BBBike extract, ~13 MB)
# Commands verified against: github.com/Project-OSRM/osrm-backend
#
# All downloaded/generated files are stored in a "data" subfolder
# next to this script, keeping osrm-data\ clean.
#
# NOTE: this is a CITY-ONLY extract. Routes going outside the
# Turin bounding box will fail with "NoRoute". For a wider area
# switch PbfUrl to a regional Geofabrik extract instead.
# ============================================================

# Do NOT use $ErrorActionPreference = "Stop" here: Docker prints harmless
# warnings to stderr (e.g. "No blkio throttle..." on WSL2 backends), and
# PowerShell would otherwise treat those as fatal terminating errors.
# We check success explicitly via $LASTEXITCODE after each docker call instead.
$ErrorActionPreference = "Continue"

$DataDir   = Join-Path $PSScriptRoot "data"
$PbfUrl    = "https://download.bbbike.org/osm/bbbike/Turin/Turin.osm.pbf"
$PbfFile   = "Turin.osm.pbf"
$OsrmBase  = "Turin.osrm"
$ContainerName = "osrm"
$MinSizeMB = 5

# Create the data subfolder if it doesn't exist yet
if (-not (Test-Path $DataDir)) {
    New-Item -ItemType Directory -Path $DataDir | Out-Null
    Write-Host "==> Created data folder: $DataDir" -ForegroundColor Cyan
}

$PbfPath = Join-Path $DataDir $PbfFile

Write-Host "==> Checking Docker..." -ForegroundColor Cyan
docker info *>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Docker is not running. Start Docker Desktop and try again." -ForegroundColor Red
    exit 1
}

# ------------------------------------------------------------
# 1. Download the OSM extract into data\ (follow redirects with -L)
# ------------------------------------------------------------
if (Test-Path $PbfPath) {
    Write-Host "==> $PbfFile already present, skipping download." -ForegroundColor Cyan
} else {
    Write-Host "==> Downloading $PbfFile ..." -ForegroundColor Cyan
    curl.exe -L -o $PbfPath $PbfUrl
}

# Verify the downloaded file is a real .pbf and not an HTML error page
$sizeMB = (Get-Item $PbfPath).Length / 1MB
if ($sizeMB -lt $MinSizeMB) {
    Write-Host "ERROR: downloaded file is too small ($([math]::Round($sizeMB,2)) MB)." -ForegroundColor Red
    Write-Host "It probably contains an error page instead of the .pbf file. First lines:" -ForegroundColor Yellow
    Get-Content $PbfPath -TotalCount 5
    exit 1
}
Write-Host "==> File downloaded correctly ($([math]::Round($sizeMB,1)) MB)." -ForegroundColor Green

# ------------------------------------------------------------
# 2. osrm-extract - build the road network graph (car profile)
#    Mount the "data" subfolder as /data inside the container.
# ------------------------------------------------------------
Write-Host "==> Running osrm-extract..." -ForegroundColor Cyan
docker run -t -v "${DataDir}:/data" ghcr.io/project-osrm/osrm-backend `
    osrm-extract -p /opt/car.lua /data/$PbfFile
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR in osrm-extract" -ForegroundColor Red; exit 1 }

# ------------------------------------------------------------
# 3. osrm-partition - MLD algorithm (default recommendation)
# ------------------------------------------------------------
Write-Host "==> Running osrm-partition..." -ForegroundColor Cyan
docker run -t -v "${DataDir}:/data" ghcr.io/project-osrm/osrm-backend `
    osrm-partition /data/$OsrmBase
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR in osrm-partition" -ForegroundColor Red; exit 1 }

# ------------------------------------------------------------
# 4. osrm-customize
# ------------------------------------------------------------
Write-Host "==> Running osrm-customize..." -ForegroundColor Cyan
docker run -t -v "${DataDir}:/data" ghcr.io/project-osrm/osrm-backend `
    osrm-customize /data/$OsrmBase
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR in osrm-customize" -ForegroundColor Red; exit 1 }

# ------------------------------------------------------------
# 5. Start the routing server (background, persistent)
# ------------------------------------------------------------
Write-Host "==> Starting OSRM server on port 5000..." -ForegroundColor Cyan

docker rm -f $ContainerName *>$null

docker run -d --restart unless-stopped --name $ContainerName `
    -p 5000:5000 -v "${DataDir}:/data" ghcr.io/project-osrm/osrm-backend `
    osrm-routed --algorithm mld /data/$OsrmBase

Start-Sleep -Seconds 3
docker ps --filter "name=$ContainerName"

Write-Host ""
Write-Host "==> Setup complete. Files stored in: $DataDir" -ForegroundColor Green
Write-Host "==> Quick test (note: http, not https):" -ForegroundColor Green
Write-Host '    Invoke-WebRequest -Uri "http://localhost:5000/route/v1/driving/7.6528,45.0652;7.6623,45.1064?overview=full" | Select-Object -ExpandProperty Content' -ForegroundColor Yellow
Write-Host ""
Write-Host "Useful commands:" -ForegroundColor Cyan
Write-Host "  docker logs $ContainerName     -> view server logs"
Write-Host "  docker stop $ContainerName     -> stop the server"
Write-Host "  docker start $ContainerName    -> restart the server (no reprocessing needed)"
