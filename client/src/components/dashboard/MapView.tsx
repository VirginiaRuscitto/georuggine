import { useEffect } from 'react';
import { motion } from 'framer-motion';
import { MapContainer, TileLayer, Marker, Popup, Polyline, useMap } from 'react-leaflet';
import { Icon, latLngBounds } from 'leaflet';
import 'leaflet/dist/leaflet.css';
import GlassCard from '../ui/GlassCard';

// Fix per le icone di Leaflet in React
import markerIcon from 'leaflet/dist/images/marker-icon.png';
import markerShadow from 'leaflet/dist/images/marker-shadow.png';

const MAP_API_KEY = import.meta.env.VITE_MAP_API_KEY;

const customIcon = new Icon({
  iconUrl: markerIcon,
  shadowUrl: markerShadow,
  iconSize: [25, 41],
  iconAnchor: [12, 41],
});

interface TrajectoryPoint {
  lat: number;
  lon: number;
  recorded_at?: string;
}

function MapUpdater({
  position,
  trajectory,
}: {
  position: [number, number];
  trajectory: TrajectoryPoint[];
}) {
  const map = useMap();

  useEffect(() => {
    if (trajectory.length > 1) {
      const bounds = latLngBounds(trajectory.map((p) => [p.lat, p.lon] as [number, number]));
      bounds.extend(position);
      map.fitBounds(bounds, { padding: [40, 40], maxZoom: 16 });
    } else {
      map.setView(position, 15);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [position[0], position[1], trajectory.length, map]);

  return null;
}

interface MapViewProps {
  position: { lat: number; lon: number } | null;
  trajectory?: TrajectoryPoint[];
}

export default function MapView({ position, trajectory = [] }: MapViewProps) {
  const defaultPos: [number, number] = [45.4642, 9.1900]; // Milano default
  const hasPosition = position !== null;
  const currentPos: [number, number] = hasPosition
    ? [position.lat, position.lon]
    : defaultPos;

  const polylinePositions: [number, number][] = trajectory.map((p) => [p.lat, p.lon]);

  return (
    <GlassCard
      variant="subtle"
      noEnter
      className="w-[65%] h-full overflow-hidden !p-0"
    >
      <motion.div
        initial={{ opacity: 0, x: 16 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 0.45, delay: 0.05 }}
        className="w-full h-full"
      >
        <MapContainer
          center={currentPos}
          zoom={15}
          className="w-full h-full rounded-xl"
          style={{ background: '#111' }}
        >
        <TileLayer
          attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/attributions">CARTO</a>'
          url={`https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png?key=${MAP_API_KEY}`}
        />

          {polylinePositions.length > 1 && (
            <Polyline
              positions={polylinePositions}
              pathOptions={{ color: '#38bdf8', weight: 4, opacity: 0.75 }}
            />
          )}

          {hasPosition && (
            <Marker position={currentPos} icon={customIcon}>
              <Popup className="dark-popup">
                <div className="text-neutral-900">
                  <p className="font-semibold">La tua posizione</p>
                  <p className="text-xs">Lat: {currentPos[0].toFixed(6)}</p>
                  <p className="text-xs">Lon: {currentPos[1].toFixed(6)}</p>
                </div>
              </Popup>
            </Marker>
          )}

          <MapUpdater position={currentPos} trajectory={trajectory} />
        </MapContainer>
      </motion.div>
    </GlassCard>
  );
}