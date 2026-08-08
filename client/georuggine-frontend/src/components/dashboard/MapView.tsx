import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { MapContainer, TileLayer, Marker, Popup, useMap } from 'react-leaflet';
import { Icon } from 'leaflet';
import 'leaflet/dist/leaflet.css';

// Fix per le icone di Leaflet in React
import markerIcon from 'leaflet/dist/images/marker-icon.png';
import markerShadow from 'leaflet/dist/images/marker-shadow.png';

const customIcon = new Icon({
  iconUrl: markerIcon,
  shadowUrl: markerShadow,
  iconSize: [25, 41],
  iconAnchor: [12, 41],
});

function MapUpdater({ position }: { position: [number, number] }) {
  const map = useMap();
  useEffect(() => {
    map.setView(position, 15);
  }, [position, map]);
  return null;
}

interface MapViewProps {
  position: { lat: number; lon: number } | null;
}

export default function MapView({ position }: MapViewProps) {
  const defaultPos: [number, number] = [45.4642, 9.1900]; // Milano default
  const currentPos: [number, number] = position
    ? [position.lat, position.lon]
    : defaultPos;

  return (
    <motion.div
      className="w-[65%] h-full glass-card overflow-hidden"
      initial={{ opacity: 0, x: 30 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.5, delay: 0.1 }}
    >
      <MapContainer
        center={currentPos}
        zoom={15}
        className="w-full h-full rounded-xl"
        style={{ background: '#111' }}
      >
        <TileLayer
          attribution='&copy; <a href="https://carto.com/">CARTO</a>'
          url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png"
        />
        <Marker position={currentPos} icon={customIcon}>
          <Popup className="dark-popup">
            <div className="text-neutral-900">
              <p className="font-semibold">La tua posizione</p>
              <p className="text-xs">Lat: {currentPos[0].toFixed(6)}</p>
              <p className="text-xs">Lon: {currentPos[1].toFixed(6)}</p>
            </div>
          </Popup>
        </Marker>
        <MapUpdater position={currentPos} />
      </MapContainer>
    </motion.div>
  );
}