import { useEffect, useMemo, useRef, useState } from 'react';
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
import {
  MapContainer,
  TileLayer,
  Marker,
  Popup,
  Polyline,
  useMap,
} from 'react-leaflet';
import { Icon } from 'leaflet';
import 'leaflet/dist/leaflet.css';

import Navbar from '../../components/layout/Navbar';
import AnimatedBackground from '../../components/ui/AnimatedBackground';
import GlassCard from '../../components/ui/GlassCard';
import { api } from '../../lib/api';

interface UserItem {
  id: number;
  name: string;
  surname: string;
  email: string;
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
  segments: Position[][];
  avg_speed_kmh: number;
  movement_duration_secs: number;
  pause_duration_secs: number;
  last_known_position: Position | null;
}

interface UsersResponse {
  users: UserItem[];
  has_next_page: boolean;
}

const DEFAULT_POS: [number, number] = [45.4642, 9.19];

const SEGMENT_COLORS = [
  '#e5e5e5',
  '#38bdf8',
  '#f472b6',
  '#a3e635',
  '#fb923c',
  '#c084fc',
];

const MAP_API_KEY = import.meta.env.VITE_MAP_API_KEY;

const startSvg = `
<svg xmlns="http://www.w3.org/2000/svg" width="32" height="42" viewBox="0 0 25 41">
  <path
    fill="#22c55e"
    stroke="#ffffff"
    stroke-width="1.5"
    d="M12.5 0C5.6 0 0 5.6 0 12.5c0 9.4 12.5 28.5 12.5 28.5S25 21.9 25 12.5C25 5.6 19.4 0 12.5 0z"
  />
  <circle cx="12.5" cy="12.5" r="5" fill="#ffffff"/>
</svg>
`;

const endSvg = `
<svg xmlns="http://www.w3.org/2000/svg" width="32" height="42" viewBox="0 0 25 41">
  <path
    fill="#f59e0b"
    stroke="#ffffff"
    stroke-width="1.5"
    d="M12.5 0C5.6 0 0 5.6 0 12.5c0 9.4 12.5 28.5 12.5 28.5S25 21.9 25 12.5C25 5.6 19.4 0 12.5 0z"
  />
  <circle cx="12.5" cy="12.5" r="5" fill="#ffffff"/>
</svg>
`;

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

// NOTA: qui manca volutamente l'icona "baseIcon" (marker di default usato
// quando non c'è nessun dato nel periodo). Lasciato così com'era, come
// richiesto: è il "pallino di default che non si mostra".

function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return '0:00:00';
  }
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);
  return `${hours}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}

function formatSpeed(kmh: number): string {
  if (!Number.isFinite(kmh)) {
    return '0.0 km/h';
  }
  return `${kmh.toFixed(1)} km/h`;
}

function formatTimestamp(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleString('it-IT', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function MapFitter({ points }: { points: [number, number][] }) {
  const map = useMap();

  useEffect(() => {
    if (points.length === 0) {
      return;
    }
    if (points.length === 1) {
      map.setView(points[0], 14);
      return;
    }
    map.fitBounds(points, { padding: [40, 40] });
  }, [points, map]);

  return null;
}

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
  const successTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    fetchUsers();
  }, []);

  useEffect(() => {
    return () => {
      if (successTimeoutRef.current) {
        clearTimeout(successTimeoutRef.current);
      }
    };
  }, []);

  async function fetchUsers() {
    setUsersLoading(true);
    try {
      const response = await api.get<UsersResponse>('/api/users', {
        params: {
          is_admin: false,
          limit: 100,
          offset: 0,
        },
      });
      setUsers(response.data?.users ?? []);
    } catch (error) {
      console.error('Errore fetch users:', error);
      setUsers([]);
    } finally {
      setUsersLoading(false);
    }
  }

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowUserDropdown(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const filteredUsers = useMemo(() => {
    const term = search.toLowerCase().trim();
    if (!term) {
      return users;
    }
    return users.filter((user) => {
      const displayName = `${user.name ?? ''} ${user.surname ?? ''} ${user.email ?? ''}`.toLowerCase();
      return displayName.includes(term);
    });
  }, [users, search]);

  function getUserDisplayName(user: UserItem): string {
    const fullName = `${user.name ?? ''} ${user.surname ?? ''}`.trim();
    return fullName || user.email;
  }

  function handleSelectUser(user: UserItem) {
    setSelectedUser(user);
    setSearch(getUserDisplayName(user));
    setShowUserDropdown(false);
    setReport(null);
    setReportError('');
    setReportSuccess('');
  }

  function handleSearchChange(value: string) {
    setSearch(value);
    setShowUserDropdown(true);
    setReportError('');
    setReportSuccess('');

    if (selectedUser) {
      const displayName = getUserDisplayName(selectedUser);
      if (value !== displayName) {
        setSelectedUser(null);
        setReport(null);
      }
    }
  }

  async function handleGenerate() {
    if (!selectedUser) {
      setReportError('Seleziona un utente prima di generare il report');
      return;
    }

    setReportLoading(true);
    setReportError('');
    setReportSuccess('');

    try {
      const response = await api.get<RouteReport>('/api/report', {
        params: {
          user_id: selectedUser.id,
          period,
        },
      });

      setReport(response.data);
      setReportSuccess('Report generato con successo');

      if (successTimeoutRef.current) {
        clearTimeout(successTimeoutRef.current);
      }
      successTimeoutRef.current = setTimeout(() => setReportSuccess(''), 3000);
    } catch (error: any) {
      console.error('Errore fetch report:', error);
      setReport(null);
      setReportError(
        error?.response?.data?.error || 'Impossibile generare il report. Riprova più tardi'
      );
    } finally {
      setReportLoading(false);
    }
  }

  const segmentPoints: [number, number][][] = useMemo(() => {
    if (!report) {
      return [];
    }
    return (report.segments ?? [])
      .filter((segment) => segment.length > 0)
      .map((segment) =>
        segment
          .filter((position) => Number.isFinite(position.lat) && Number.isFinite(position.lon))
          .map((position) => [position.lat, position.lon] as [number, number])
      )
      .filter((segment) => segment.length > 0);
  }, [report]);

  const allPoints: [number, number][] = useMemo(() => {
    // Il punto di last_known_position deve sempre entrare nel calcolo dei
    // bounds, non solo quando non c'è nessun altro punto: altrimenti il
    // marker dell'ultima posizione viene disegnato ma resta fuori
    // dall'inquadratura della mappa (mai visibile senza scrollare/zoomare
    // manualmente).
    const points = [...segmentPoints.flat()];

    if (
      report?.last_known_position &&
      Number.isFinite(report.last_known_position.lat) &&
      Number.isFinite(report.last_known_position.lon)
    ) {
      points.push([report.last_known_position.lat, report.last_known_position.lon]);
    }

    return points;
  }, [segmentPoints, report]);

  const firstPosition: Position | null = useMemo(() => {
    if (!report) {
      return null;
    }
    const firstSegment = (report.segments ?? []).find((segment) => segment.length > 0);
    return firstSegment?.[0] ?? null;
  }, [report]);

  const lastPosition: Position | null = useMemo(() => {
    return report?.last_known_position ?? null;
  }, [report]);

  const hasMovement = segmentPoints.length > 0;

  const hasDistinctLastPosition =
    firstPosition &&
    lastPosition &&
    (firstPosition.lat !== lastPosition.lat ||
      firstPosition.lon !== lastPosition.lon ||
      firstPosition.recorded_at !== lastPosition.recorded_at);

  const mapCenter: [number, number] = allPoints.length > 0 ? allPoints[0] : DEFAULT_POS;

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
          <p className="text-muted">Analizza tragitto, velocità e tempi di attività di un singolo utente</p>
        </motion.div>

        <div className="grid grid-cols-1 lg:grid-cols-[380px_1fr] gap-6 items-start">
          <GlassCard variant="hover" delay={0.1}>
            <div className="flex items-center gap-3 mb-6">
              <div className="w-9 h-9 rounded-lg bg-white/5 border border-white/10 flex items-center justify-center">
                <BarChart3 size={18} className="text-neutral-300" />
              </div>
              <h2 className="text-lg font-semibold">Richiedi report</h2>
            </div>

            <div className="space-y-5">
              <div ref={dropdownRef} className="relative">
                <label className="text-xs text-muted mb-1 block">Utente</label>

                <div className="relative">
                  <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted pointer-events-none" />
                  <input
                    type="text"
                    placeholder="Cerca nome, cognome o email..."
                    value={search}
                    onChange={(event) => handleSearchChange(event.target.value)}
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
                        <p className="text-xs text-muted text-center py-3">Nessun utente trovato</p>
                      ) : (
                        filteredUsers.map((user) => (
                          <button
                            key={user.id}
                            type="button"
                            onClick={() => handleSelectUser(user)}
                            className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors flex items-center gap-2 ${
                              selectedUser?.id === user.id
                                ? 'bg-white/10 text-white'
                                : 'hover:bg-white/[0.05] text-neutral-200'
                            }`}
                          >
                            <User size={14} className="text-muted flex-shrink-0" />
                            <div className="min-w-0">
                              <p className="truncate">{getUserDisplayName(user)}</p>
                              {user.name && user.surname && (
                                <p className="text-[11px] text-muted truncate">{user.email}</p>
                              )}
                            </div>
                          </button>
                        ))
                      )}
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>

              <div>
                <label className="text-xs text-muted mb-2 block">Granularità</label>
                <div className="grid grid-cols-3 gap-2">
                  <PeriodChip label="Giorno" active={period === 'day'} onClick={() => setPeriod('day')} />
                  <PeriodChip label="Settimana" active={period === 'week'} onClick={() => setPeriod('week')} />
                  <PeriodChip label="Mese" active={period === 'month'} onClick={() => setPeriod('month')} />
                </div>
              </div>

              <AnimatePresence>
                {reportError && (
                  <motion.div
                    className="flex items-center gap-2 text-sm text-danger bg-danger/10 border border-danger/20 rounded-lg px-3 py-2"
                    initial={{ opacity: 0, y: -5 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                  >
                    <CircleAlert size={14} />
                    <span>{reportError}</span>
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
                    <span>{reportSuccess}</span>
                  </motion.div>
                )}
              </AnimatePresence>

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

          <GlassCard variant="hover" delay={0.2}>
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
                  url={`https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png${
                    MAP_API_KEY ? `?key=${MAP_API_KEY}` : ''
                  }`}
                />

                {segmentPoints.map(
                  (points, index) =>
                    points.length >= 2 && (
                      <Polyline
                        key={`segment-${index}`}
                        positions={points}
                        pathOptions={{
                          color: SEGMENT_COLORS[index % SEGMENT_COLORS.length],
                          weight: 3,
                          opacity: 0.85,
                        }}
                      />
                    )
                )}

                {firstPosition && (
                  <Marker position={[firstPosition.lat, firstPosition.lon]} icon={startIcon}>
                    <Popup>
                      <div className="text-neutral-900">
                        <p className="font-semibold">Partenza</p>
                        <p className="text-xs">{formatTimestamp(firstPosition.recorded_at)}</p>
                        <p className="text-xs">
                          Lat: {firstPosition.lat.toFixed(5)} · Lon: {firstPosition.lon.toFixed(5)}
                        </p>
                      </div>
                    </Popup>
                  </Marker>
                )}

                {hasDistinctLastPosition && (
                  <Marker position={[lastPosition!.lat, lastPosition!.lon]} icon={endIcon}>
                    <Popup>
                      <div className="text-neutral-900">
                        <p className="font-semibold">Ultima posizione</p>
                        <p className="text-xs">{formatTimestamp(lastPosition!.recorded_at)}</p>
                        <p className="text-xs">
                          Lat: {lastPosition!.lat.toFixed(5)} · Lon: {lastPosition!.lon.toFixed(5)}
                        </p>
                      </div>
                    </Popup>
                  </Marker>
                )}

                {!hasMovement && report?.last_known_position && (
                  <Marker
                    position={[report.last_known_position.lat, report.last_known_position.lon]}
                    icon={endIcon}
                  >
                    <Popup>
                      <div className="text-neutral-900">
                        <p className="font-semibold">Ultima posizione</p>
                        <p className="text-xs">Nessun movimento nel periodo</p>
                        <p className="text-xs">{formatTimestamp(report.last_known_position.recorded_at)}</p>
                        <p className="text-xs">
                          Lat: {report.last_known_position.lat.toFixed(5)} · Lon:{' '}
                          {report.last_known_position.lon.toFixed(5)}
                        </p>
                      </div>
                    </Popup>
                  </Marker>
                )}

                {/* NB: qui, se non c'è né movimento né last_known_position, in origine
                    si mostrava un marker con "baseIcon" — icona mai definita in questa
                    versione del file, quindi il pallino di default non compare.
                    Lasciato invariato di proposito. */}

                <MapFitter points={allPoints} />
              </MapContainer>
            </div>

            <div className="flex flex-wrap items-center gap-5 mt-4 text-xs text-muted">
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

function PeriodChip({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
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
      <div className={`w-9 h-9 rounded-lg flex items-center justify-center mb-3 ${color}`}>{icon}</div>
      <p className="text-xs text-muted mb-1">{label}</p>
      <p className="text-xl font-semibold">{value}</p>
    </div>
  );
}
