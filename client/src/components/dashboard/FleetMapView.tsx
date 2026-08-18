import { useEffect } from 'react';
import { motion } from 'framer-motion';
import { MapContainer, TileLayer, Marker, Popup, Polyline, useMap } from 'react-leaflet';
import { DivIcon, latLngBounds } from 'leaflet';
import 'leaflet/dist/leaflet.css';
import GlassCard from '../ui/GlassCard';

export interface FleetUserTrack {
  id: number;
  label: string;
  color: string;
  trajectory: { lat: number; lon: number }[];
  state: 'disconnected' | 'stopped' | 'moving';
}

const DEFAULT_CENTER: [number, number] = [45.0703, 7.6869]; // Torino default

function userIcon(color: string) {
  return new DivIcon({
    className: '',
    html: `<div style="
      width: 16px; height: 16px; border-radius: 9999px;
      background:${color}; border: 2px solid rgba(255,255,255,0.85);
      box-shadow: 0 0 0 3px rgba(0,0,0,0.35);
    "></div>`,
    iconSize: [16, 16],
    iconAnchor: [8, 8],
  });
}

function FitAllBounds({ tracks }: { tracks: FleetUserTrack[] }) {
  const map = useMap();

  useEffect(() => {
    const allPoints = tracks.flatMap((t) => t.trajectory.map((p) => [p.lat, p.lon] as [number, number]));
    if (allPoints.length === 0) return;

    if (allPoints.length === 1) {
      map.setView(allPoints[0], 14);
      return;
    }

    const bounds = latLngBounds(allPoints);
    map.fitBounds(bounds, { padding: [50, 50], maxZoom: 15 });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tracks.map((t) => `${t.id}:${t.trajectory.length}`).join('|'), map]);

  return null;
}

interface FleetMapViewProps {
  tracks: FleetUserTrack[];
  loading?: boolean;
}

export default function FleetMapView({ tracks, loading }: FleetMapViewProps) {
  return (
    <GlassCard variant="subtle" noEnter className="w-full h-full overflow-hidden !p-0 relative">
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4 }}
        className="w-full h-full"
      >
        <MapContainer
          center={DEFAULT_CENTER}
          zoom={13}
          className="w-full h-full rounded-xl"
          style={{ background: '#111' }}
        >
          <TileLayer
            attribution='&copy; <a href="https://carto.com/">CARTO</a>'
            url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png"
          />

          {tracks.map((track) =>
            track.trajectory.length > 1 ? (
              <Polyline
                key={`line-${track.id}`}
                positions={track.trajectory.map((p) => [p.lat, p.lon])}
                pathOptions={{ color: track.color, weight: 4, opacity: 0.7 }}
              />
            ) : null
          )}

          {tracks.map((track) => {
            const last = track.trajectory[track.trajectory.length - 1];
            if (!last) return null;
            return (
              <Marker key={`marker-${track.id}`} position={[last.lat, last.lon]} icon={userIcon(track.color)}>
                <Popup className="dark-popup">
                  <div className="text-neutral-900">
                    <p className="font-semibold">{track.label}</p>
                    <p className="text-xs capitalize">{track.state}</p>
                  </div>
                </Popup>
              </Marker>
            );
          })}

          <FitAllBounds tracks={tracks} />
        </MapContainer>

        {/* Legenda */}
        {tracks.length > 0 && (
          <div className="absolute bottom-4 left-4 z-[1000] flex flex-col gap-1.5 bg-black/50 backdrop-blur-sm rounded-lg px-3 py-2 border border-white/10">
            {tracks.map((track) => (
              <div key={`legend-${track.id}`} className="flex items-center gap-2 text-xs text-white/90">
                <span
                  className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                  style={{ background: track.color }}
                />
                <span>{track.label}</span>
              </div>
            ))}
          </div>
        )}

        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/20 z-[999]">
            <div className="w-6 h-6 border-2 border-white/20 border-t-white rounded-full animate-spin" />
          </div>
        )}

        {!loading && tracks.length === 0 && (
          <div className="absolute inset-0 flex items-center justify-center z-[999]">
            <p className="text-sm text-muted">Nessun percorso disponibile al momento</p>
          </div>
        )}
      </motion.div>
    </GlassCard>
  );
}