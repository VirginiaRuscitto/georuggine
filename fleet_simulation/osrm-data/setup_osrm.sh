#!/usr/bin/env bash
# ============================================================
# Local OSRM setup via Docker (cross-platform: Linux / macOS / WSL)
# Extract: Turin city (small BBBike extract, ~13 MB)
# Commands verified against: github.com/Project-OSRM/osrm-backend
#
# All downloaded/generated files are stored in a "data" subfolder
# next to this script, keeping the root directory clean.
#
# NOTE: this is a CITY-ONLY extract. Routes going outside the
# Turin bounding box will fail with "NoRoute". For a wider area
# switch PBF_URL to a regional Geofabrik extract instead.
# ============================================================

set -e

# Resolve the directory where this script resides to ensure relative paths work reliably
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="${SCRIPT_DIR}/data"

PBF_URL="https://download.bbbike.org/osm/bbbike/Turin/Turin.osm.pbf"
PBF_FILE="Turin.osm.pbf"
OSRM_BASE="Turin.osrm"
CONTAINER_NAME="osrm"
MIN_SIZE_MB=5

# Create the data subfolder if it doesn't exist yet
if [ ! -d "$DATA_DIR" ]; then
    mkdir -p "$DATA_DIR"
    echo "==> Created data folder: $DATA_DIR"
fi

PBF_PATH="${DATA_DIR}/${PBF_FILE}"

echo "==> Checking Docker..."
if ! docker info > /dev/null 2>&1; then
    echo "ERROR: Docker is not running. Start Docker Desktop / the Docker daemon and try again."
    exit 1
fi

# ------------------------------------------------------------
# 1. Download the OSM extract into data/ (follow redirects with -L)
# ------------------------------------------------------------
if [ -f "$PBF_PATH" ]; then
    echo "==> $PBF_FILE already present, skipping download."
else
    echo "==> Downloading $PBF_FILE ..."
    curl -L -o "$PBF_PATH" "$PBF_URL"
fi

# Verify the downloaded file is a real .pbf and not an HTML error page
size_mb=$(du -m "$PBF_PATH" | cut -f1)
if [ "$size_mb" -lt "$MIN_SIZE_MB" ]; then
    echo "ERROR: downloaded file is too small (${size_mb} MB)."
    echo "It probably contains an error page instead of the .pbf file. First lines:"
    head -c 500 "$PBF_PATH"
    echo ""
    exit 1
fi
echo "==> File downloaded correctly (${size_mb} MB)."

# ------------------------------------------------------------
# 2. osrm-extract - build the road network graph (car profile)
#    Mount the "data" subfolder as /data inside the container.
# ------------------------------------------------------------
echo "==> Running osrm-extract..."
docker run -t -v "${DATA_DIR}:/data" ghcr.io/project-osrm/osrm-backend \
    osrm-extract -p /opt/car.lua /data/"$PBF_FILE"

# ------------------------------------------------------------
# 3. osrm-partition - MLD algorithm (default recommendation)
# ------------------------------------------------------------
echo "==> Running osrm-partition..."
docker run -t -v "${DATA_DIR}:/data" ghcr.io/project-osrm/osrm-backend \
    osrm-partition /data/"$OSRM_BASE"

# ------------------------------------------------------------
# 4. osrm-customize
# ------------------------------------------------------------
echo "==> Running osrm-customize..."
docker run -t -v "${DATA_DIR}:/data" ghcr.io/project-osrm/osrm-backend \
    osrm-customize /data/"$OSRM_BASE"

# ------------------------------------------------------------
# 5. Start the routing server (background, persistent)
# ------------------------------------------------------------
echo "==> Starting OSRM server on port 5000..."

# Remove any previous container with the same name
docker rm -f "$CONTAINER_NAME" > /dev/null 2>&1 || true

docker run -d --restart unless-stopped --name "$CONTAINER_NAME" \
    -p 5000:5000 -v "${DATA_DIR}:/data" ghcr.io/project-osrm/osrm-backend \
    osrm-routed --algorithm mld /data/"$OSRM_BASE"

sleep 3
docker ps --filter "name=$CONTAINER_NAME"

echo ""
echo "==> Setup complete. Files stored in: $DATA_DIR"
echo "==> Quick test:"
echo '    curl "http://localhost:5000/route/v1/driving/7.6528,45.0652;7.6623,45.1064?overview=full"'
echo ""
echo "Useful commands:"
echo "  docker logs $CONTAINER_NAME     -> view server logs"
echo "  docker stop $CONTAINER_NAME     -> stop the server"
echo "  docker start $CONTAINER_NAME    -> restart the server (no reprocessing needed)"