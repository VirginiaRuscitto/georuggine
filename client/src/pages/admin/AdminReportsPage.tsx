import { useState, useEffect, useMemo, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Search,
  BarChart3,
  Activity,
  Navigation,
  Clock,
  User,
  CircleAlert,
  CircleCheck,
} from 'lucide-react';
import { MapContainer, TileLayer, Marker, Popup, Polyline, useMap } from 'react-leaflet';
import { Icon } from 'leaflet';
import 'leaflet/dist/leaflet.css';
import markerIcon from 'leaflet/dist/images/marker-icon.png';
import markerShadow from 'leaflet/dist/images/marker-shadow.png';
import Navbar from '../../components/layout/Navbar';
import AnimatedBackground from '../../components/ui/AnimatedBackground';
import GlassCard from '../../components/ui/GlassCard';
import { api } from '../../lib/api';

// ----------------------------------------------------------------
// Tipi
// ----------------------------------------------------------------

interface UserItem {
  id: number;
  username: string;
  name?: string;
  surname?: string;
  state: string;
  is_admin: boolean;
}

type Period = 'day' | 'week' | 'month';

interface Position {
  lat: number;
  lon: number;
  recorded_at: string;
}

interface RouteReport {
  user_id: number;
  period: Period;
  segments: Position[][]; // un array di posizioni per ogni sessione di movimento
  avg_speed_kmh: number;
  movement_duration_secs: number;
  pause_duration_secs: number;
}

// Colori usati per distinguere visivamente le sessioni di movimento sulla mappa
const SEGMENT_COLORS = ['#e5e5e5', '#38bdf8', '#f472b6', '#a3e635', '#fb923c', '#c084fc'];

const MAP_API_KEY = import.meta.env.VITE_MAP_API_KEY;

// ----------------------------------------------------------------
// Icone marker Leaflet
// ----------------------------------------------------------------

const baseIcon = new Icon({
  iconUrl: markerIcon,
  shadowUrl: markerShadow,
  iconSize: [25, 41],
  iconAnchor: [12, 41],
});

const startSvg = `
<svg xmlns="http://www.w3.org/2000/svg" width="32" height="42" viewBox="0 0 25 41">
  <path fill="#22c55e" stroke="#ffffff" stroke-width="1.5"
    d="M12.5 0C5.6 0 0 5.6 0 12.5c0 9.4 12.5 28.5 12.5 28.5S25 21.9 25 12.5C25 5.6 19.4 0 12.5 0z"/>
  <circle cx="12.5" cy="12.5" r="5" fill="#ffffff"/>
</svg>`;

const endSvg = `
<svg xmlns="http://www.w3.org/2000/svg" width="32" height="42" viewBox="0 0 25 41">
  <path fill="#f59e0b" stroke="#ffffff" stroke-width="1.5"
    d="M12.5 0C5.6 0 0 5.6 0 12.5c0 9.4 12.5 28.5 12.5 28.5S25 21.9 25 12.5C25 5.6 19.4 0 12.5 0z"/>
  <circle cx="12.5" cy="12.5" r="5" fill="#ffffff"/>
</svg>`;

const startIcon = new Icon({
  iconUrl: 'data:image/svg+xml;charset=UTF-8,' + encodeURIComponent(startSvg),
  iconSize: [25, 41],
  iconAnchor: [12, 41],
});

const endIcon = new Icon({
  iconUrl: 'data:image/svg+xml;charset=UTF-8,' + encodeURIComponent(endSvg),
  iconSize: [25, 41],
  iconAnchor: [12, 41],
});

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

const DEFAULT_POS: [number, number] = [45.4642, 9.19];

function formatDuration(secs: number): string {
  if (!Number.isFinite(secs) || secs < 0) return '0:00:00';
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function formatSpeed(kmh: number): string {
  if (!Number.isFinite(kmh)) return '0.0 km/h';
  return `${kmh.toFixed(1)} km/h`;
}

function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString('it-IT', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

// ----------------------------------------------------------------
// MapFitter
// ----------------------------------------------------------------

function MapFitter({ points }: { points: [number, number][] }) {
  const map = useMap();
  useEffect(() => {
    if (points.length === 0) return;
    if (points.length === 1) {
      map.setView(points[0], 14);
      return;
    }
    map.fitBounds(points, { padding: [40, 40] });
  }, [points, map]);
  return null;
}

// ----------------------------------------------------------------
// Pagina principale
// ----------------------------------------------------------------

export default function AdminReportsPage() {
  const [users, setUsers] = useState<UserItem[]>([]);
  const [usersLoading, setUsersLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [showUserDropdown, setShowUserDropdown] = useState(false);
  const [selectedUser, setSelectedUser] = useState<UserItem | null>(null);
  const [period, setPeriod] = useState<Period>('day');
  const [report, setReport] = useState<RouteReport | null>(null);
  const [reportLoading, setReportLoading] = useState(false);
  const [reportError, setReportError] = useState('');
  const [reportSuccess, setReportSuccess] = useState('');
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetchUsers();
  }, []);

  useEffect(() => {
    const onClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowUserDropdown(false);
      }
    };
    document.addEventListener('mousedown', onClickOutside);
    return () => document.removeEventListener('mousedown', onClickOutside);
  }, []);

  const fetchUsers = async () => {
    try {
      // Il report ha senso solo per i camionisti: chiediamo al backend solo
      // i non-admin invece di scaricarli tutti e filtrare lato client.
      const res = await api.get('/api/users?is_admin=false');
      setUsers(res.data || []);
    } catch (e) {
      console.error('Errore fetch users:', e);
    } finally {
      setUsersLoading(false);
    }
  };

  const filteredUsers = useMemo(() => {
    const term = search.toLowerCase().trim();
    if (!term) return users;
    return users.filter((u) => {
      const displayName = `${u.name ?? ''} ${u.surname ?? ''} ${u.username ?? ''}`.toLowerCase();
      return displayName.includes(term);
    });
  }, [users, search]);

  const handleSelectUser = (u: UserItem) => {
    setSelectedUser(u);
    setSearch(u.name && u.surname ? `${u.name} ${u.surname}` : u.username);
    setShowUserDropdown(false);
    setReport(null);
    setReportError('');
  };

  const handleGenerate = async () => {
    if (!selectedUser) {
      setReportError('Seleziona un utente prima di generare il report');
      return;
    }
    setReportLoading(true);
    setReportError('');
    setReportSuccess('');
    try {
      const res = await api.get('/api/report', {
        params: { user_id: selectedUser.id, period },
      });
      setReport(res.data);
      setReportSuccess('Report generato con successo');
      setTimeout(() => setReportSuccess(''), 3000);
    } catch (e: any) {
      console.error('Errore fetch report:', e);
      setReport(null);
      setReportError(
        e.response?.data?.error || 'Impossibile generare il report. Riprova più tardi',
      );
    } finally {
      setReportLoading(false);
    }
  };

  // Un array di coordinate per ogni sessione di movimento: ogni sessione viene
  // disegnata come una Polyline separata, così non si crea mai una linea che
  // "teletrasporta" da una sessione all'altra (es. tratte in città diverse).
  const segmentPoints: [number, number][][] = useMemo(() => {
    if (!report) return [];
    return report.segments
      .filter((seg) => seg.length > 0)
      .map((seg) => seg.map((p) => [p.lat, p.lon] as [number, number]));
  }, [report]);

  // Tutti i punti insieme, usati solo per calcolare i bounds della mappa
  const allPoints: [number, number][] = useMemo(() => segmentPoints.flat(), [segmentPoints]);

  // Primo punto in assoluto (per il marker di partenza)
  const firstPosition: Position | null = useMemo(() => {
    if (!report) return null;
    const firstSeg = report.segments.find((s) => s.length > 0);
    return firstSeg ? firstSeg[0] : null;
  }, [report]);

  // Ultimo punto in assoluto (per il marker di arrivo/ultima posizione)
  const lastPosition: Position | null = useMemo(() => {
    if (!report) return null;
    const lastSeg = [...report.segments].reverse().find((s) => s.length > 0);
    return lastSeg ? lastSeg[lastSeg.length - 1] : null;
  }, [report]);

  const mapCenter: [number, number] =
    allPoints.length > 0 ? allPoints[0] : DEFAULT_POS;

  return (
    <div className="min-h-screen relative">
      <AnimatedBackground />
      <Navbar />

      <div className="relative z-10 pt-24 pb-12 px-6 max-w-7xl mx-auto">
        <motion.div
          className="mb-8"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          <h1 className="text-3xl font-bold mb-2">Report Utenti</h1>
          <p className="text-muted">
            Analizza tragitto, velocità e tempi di attività di un singolo utente
          </p>
        </motion.div>

        <div className="grid grid-cols-1 lg:grid-cols-[380px_1fr] gap-6 items-start">
          {/* Colonna sinistra: form */}
          <GlassCard variant="hover" delay={0.1}>
            <div className="flex items-center gap-3 mb-6">
              <div className="w-9 h-9 rounded-lg bg-white/5 border border-white/10 flex items-center justify-center">
                <BarChart3 size={18} className="text-neutral-300" />
              </div>
              <h2 className="text-lg font-semibold">Richiedi report</h2>
            </div>

            <div className="space-y-5">
              {/* Dropdown utenti con ricerca */}
              <div ref={dropdownRef} className="relative">
                <label className="text-xs text-muted mb-1 block">Utente</label>
                <div className="relative">
                  <Search
                    size={16}
                    className="absolute left-3 top-1/2 -translate-y-1/2 text-muted pointer-events-none"
                  />
                  <input
                    type="text"
                    placeholder="Cerca utente..."
                    value={search}
                    onChange={(e) => {
                      setSearch(e.target.value);
                      setShowUserDropdown(true);
                      if (selectedUser) {
                        const display = selectedUser.name && selectedUser.surname
                          ? `${selectedUser.name} ${selectedUser.surname}`
                          : selectedUser.username;
                        if (e.target.value !== display) {
                          setSelectedUser(null);
                        }
                      }
                    }}
                    onFocus={() => setShowUserDropdown(true)}
                    className="glass-input w-full !py-2.5 !pl-10 !pr-4 !text-sm"
                  />
                </div>

                <AnimatePresence>
                  {showUserDropdown && (
                    <motion.div
                      className="absolute left-0 right-0 top-full mt-2 max-h-56 overflow-y-auto bg-[#1a1a2e] border border-white/10 rounded-xl p-2 z-[9999] shadow-2xl"
                      initial={{ opacity: 0, y: -8, scale: 0.95 }}
                      animate={{ opacity: 1, y: 0, scale: 1 }}
                      exit={{ opacity: 0, y: -8, scale: 0.95 }}
                      transition={{ duration: 0.15 }}
                    >
                      {usersLoading ? (
                        <p className="text-xs text-muted text-center py-3">Caricamento...</p>
                      ) : filteredUsers.length === 0 ? (
                        <p className="text-xs text-muted text-center py-3">Nessun utente</p>
                      ) : (
                        filteredUsers.map((u) => (
                          <button
                            key={u.id}
                            type="button"
                            onClick={() => handleSelectUser(u)}
                            className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors flex items-center gap-2 ${
                              selectedUser?.id === u.id
                                ? 'bg-white/10 text-white'
                                : 'hover:bg-white/[0.05] text-neutral-200'
                            }`}
                          >
                            <User size={14} className="text-muted flex-shrink-0" />
                            <span className="truncate">
                              {u.name && u.surname ? `${u.name} ${u.surname}` : u.username}
                            </span>
                          </button>
                        ))
                      )}
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>

              {/* Granularità */}
              <div>
                <label className="text-xs text-muted mb-2 block">Granularità</label>
                <div className="grid grid-cols-3 gap-2">
                  <PeriodChip
                    label="Giorno"
                    active={period === 'day'}
                    onClick={() => setPeriod('day')}
                  />
                  <PeriodChip
                    label="Settimana"
                    active={period === 'week'}
                    onClick={() => setPeriod('week')}
                  />
                  <PeriodChip
                    label="Mese"
                    active={period === 'month'}
                    onClick={() => setPeriod('month')}
                  />
                </div>
              </div>

              {/* Messaggi */}
              <AnimatePresence>
                {reportError && (
                  <motion.div
                    className="flex items-center gap-2 text-sm text-danger bg-danger/10 border border-danger/20 rounded-lg px-3 py-2"
                    initial={{ opacity: 0, y: -5 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                  >
                    <CircleAlert size={14} />
                    {reportError}
                  </motion.div>
                )}
                {reportSuccess && (
                  <motion.div
                    className="flex items-center gap-2 text-sm text-success bg-success/10 border border-success/20 rounded-lg px-3 py-2"
                    initial={{ opacity: 0, y: -5 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                  >
                    <CircleCheck size={14} />
                    {reportSuccess}
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Bottone */}
              <motion.button
                type="button"
                onClick={handleGenerate}
                disabled={!selectedUser || reportLoading}
                className="btn-primary w-full flex items-center justify-center gap-2 disabled:opacity-50"
                whileHover={{ scale: selectedUser ? 1.01 : 1 }}
                whileTap={{ scale: selectedUser ? 0.99 : 1 }}
              >
                {reportLoading ? (
                  'Generazione...'
                ) : (
                  <>
                    <BarChart3 size={16} />
                    Genera report
                  </>
                )}
              </motion.button>
            </div>
          </GlassCard>

          {/* Colonna destra: risultati */}
          <GlassCard variant="hover" delay={0.2}>
            <div className="flex items-center justify-center mb-6">
              <h2 className="text-base font-semibold text-muted tracking-wide">Report data</h2>
            </div>

            {/* Metriche */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-6">
              <MetricCard
                icon={<Activity size={18} />}
                label="Velocità media"
                value={report ? formatSpeed(report.avg_speed_kmh) : '—'}
                color="text-emerald-400"
                bg="bg-emerald-400/10"
                border="border-emerald-400/20"
              />
              <MetricCard
                icon={<Navigation size={18} />}
                label="Tempo in movimento"
                value={report ? formatDuration(report.movement_duration_secs) : '—'}
                color="text-blue-400"
                bg="bg-blue-400/10"
                border="border-blue-400/20"
              />
              <MetricCard
                icon={<Clock size={18} />}
                label="Tempo in pausa"
                value={report ? formatDuration(report.pause_duration_secs) : '—'}
                color="text-amber-400"
                bg="bg-amber-400/10"
                border="border-amber-400/20"
              />
            </div>

            {/* Mappa */}
            <div className="rounded-xl overflow-hidden border border-white/[0.06]" style={{ height: 420 }}>
              <MapContainer
                center={mapCenter}
                zoom={6}
                className="w-full h-full"
                style={{ background: '#111' }}
                key={report ? `${report.user_id}-${report.period}` : 'empty'}
              >
            <TileLayer
                      attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/attributions">CARTO</a>'
                      url={`https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png?key=${MAP_API_KEY}`}
            />

                {/* Una Polyline per ogni sessione di movimento: niente linee che
                    collegano tratte lontane appartenenti a sessioni diverse */}
                {segmentPoints.map((pts, i) =>
                  pts.length >= 2 ? (
                    <Polyline
                      key={`seg-${i}`}
                      positions={pts}
                      pathOptions={{
                        color: SEGMENT_COLORS[i % SEGMENT_COLORS.length],
                        weight: 3,
                        opacity: 0.85,
                      }}
                    />
                  ) : null
                )}

                {firstPosition && (
                  <Marker position={[firstPosition.lat, firstPosition.lon]} icon={startIcon}>
                    <Popup>
                      <div className="text-neutral-900">
                        <p className="font-semibold">Partenza</p>
                        <p className="text-xs">{formatTimestamp(firstPosition.recorded_at)}</p>
                        <p className="text-xs">
                          Lat: {firstPosition.lat.toFixed(5)} · Lon:{' '}
                          {firstPosition.lon.toFixed(5)}
                        </p>
                      </div>
                    </Popup>
                  </Marker>
                )}

                {lastPosition && lastPosition !== firstPosition && (
                  <Marker position={[lastPosition.lat, lastPosition.lon]} icon={endIcon}>
                    <Popup>
                      <div className="text-neutral-900">
                        <p className="font-semibold">Ultima posizione</p>
                        <p className="text-xs">{formatTimestamp(lastPosition.recorded_at)}</p>
                        <p className="text-xs">
                          Lat: {lastPosition.lat.toFixed(5)} · Lon: {lastPosition.lon.toFixed(5)}
                        </p>
                      </div>
                    </Popup>
                  </Marker>
                )}

                {allPoints.length === 0 && (
                  <Marker position={DEFAULT_POS} icon={baseIcon}>
                    <Popup>
                      <div className="text-neutral-900">
                        <p className="text-xs">
                          {report
                            ? 'Nessun dato nel periodo selezionato'
                            : 'Seleziona un utente e genera un report'}
                        </p>
                      </div>
                    </Popup>
                  </Marker>
                )}

                <MapFitter points={allPoints} />
              </MapContainer>
            </div>

            {/* Legenda */}
            <div className="flex items-center gap-5 mt-4 text-xs text-muted">
              <span className="flex items-center gap-2">
                <span className="w-3 h-3 rounded-full bg-emerald-400" />
                Partenza
              </span>
              <span className="flex items-center gap-2">
                <span className="w-3 h-3 rounded-full bg-amber-400" />
                Ultima posizione
              </span>
              <span className="flex items-center gap-2">
                <span className="w-6 h-0.5 bg-neutral-300" />
                Tragitto
              </span>
              {report && segmentPoints.length > 1 && (
                <span className="text-muted">
                  {segmentPoints.length} sessioni di movimento nel periodo (colori diversi)
                </span>
              )}
            </div>
          </GlassCard>
        </div>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Sotto-componenti
// ----------------------------------------------------------------

function PeriodChip({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <motion.button
      type="button"
      onClick={onClick}
      className={`px-3 py-2 rounded-lg text-sm border transition-colors ${
        active
          ? 'bg-white/10 border-white/20 text-white'
          : 'bg-white/[0.03] border-white/[0.06] text-muted hover:text-white hover:bg-white/[0.05]'
      }`}
      whileTap={{ scale: 0.97 }}
    >
      {label}
    </motion.button>
  );
}

function MetricCard({
  icon,
  label,
  value,
  color,
  bg,
  border,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  color: string;
  bg: string;
  border: string;
}) {
  return (
    <div className={`p-4 rounded-xl ${bg} border ${border}`}>
      <div className={`w-9 h-9 rounded-lg flex items-center justify-center mb-3 ${color}`}>
        {icon}
      </div>
      <p className="text-xs text-muted mb-1">{label}</p>
      <p className="text-xl font-semibold">{value}</p>
    </div>
  );
}