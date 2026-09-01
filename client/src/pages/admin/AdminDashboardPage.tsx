import { useState, useEffect, useCallback, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useNavigate } from 'react-router-dom';
import { Users, Activity, Radio, BarChart3, ChevronDown, Check, Eye, Settings2 } from 'lucide-react';
import Navbar from '../../components/layout/Navbar';
import AnimatedBackground from '../../components/ui/AnimatedBackground';
import GlassCard from '../../components/ui/GlassCard';
import FleetMapView from '../../components/dashboard/FleetMapView';
import type { FleetUserTrack } from '../../components/dashboard/FleetMapView';
import { api } from '../../lib/api';

interface DashboardStats {
  totalUsers: number;
  activeUsers: number;
  movingUsers: number;
}

interface UserItem {
  id: number;
  name?: string;
  surname?: string;
  username?: string;
  state: string;
}

const TRACK_COLORS = ['#38bdf8', '#34d399', '#fbbf24', '#f472b6', '#a78bfa'];
const MAX_TRACKED_USERS = 4;
const DEFAULT_POLL_INTERVAL_SECS = 10;
const MIN_POLL_INTERVAL_SECS = 5;
const MAX_POLL_INTERVAL_SECS = 300;

function shuffleArray<T>(array: T[]): T[] {
  const arr = [...array];
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

export default function AdminDashboardPage() {
  const navigate = useNavigate();
  const [stats, setStats] = useState<DashboardStats>({
    totalUsers: 0,
    activeUsers: 0,
    movingUsers: 0,
  });
  const [allUsers, setAllUsers] = useState<UserItem[]>([]);
  const [selectedUserIds, setSelectedUserIds] = useState<Set<number>>(new Set());
  const [tracks, setTracks] = useState<FleetUserTrack[]>([]);
  const [mapLoading, setMapLoading] = useState(true);
  const [showDropdown, setShowDropdown] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [pollIntervalSecs, setPollIntervalSecs] = useState(DEFAULT_POLL_INTERVAL_SECS);
  const [tempPollInterval, setTempPollInterval] = useState(DEFAULT_POLL_INTERVAL_SECS.toString());
  const dropdownRef = useRef<HTMLDivElement>(null);
  const settingsRef = useRef<HTMLDivElement>(null);

  const fetchStats = useCallback(async () => {
    try {
      const usersRes = await api.get('/api/users?limit=100&offset=0');

      const users: UserItem[] = usersRes.data.users || [];

      const active = users.filter((u) => u.state !== 'disconnected').length;
      const moving = users.filter((u) => u.state === 'moving').length;

      setStats({
        totalUsers: users.length,
        activeUsers: active,
        movingUsers: moving,
      });

      setAllUsers(users);

      if (selectedUserIds.size === 0) {
        const activeUsers = users.filter((u) => u.state !== 'disconnected');
        const shuffled = shuffleArray(activeUsers);
        const initial = shuffled.slice(0, Math.min(3, shuffled.length));
        setSelectedUserIds(new Set(initial.map((u) => u.id)));
      }

      return users;
    } catch (e) {
      console.error('Errore fetch stats:', e);
      return [];
    }
  }, [selectedUserIds.size]);

  const fetchFleetTracks = useCallback(async (users: UserItem[]) => {
    const selected = users.filter((u) => selectedUserIds.has(u.id));

    if (selected.length === 0) {
      setTracks([]);
      setMapLoading(false);
      return;
    }

    setMapLoading(true);
    try {
      const results = await Promise.all(
        selected.map((u) =>
          api
            .get(`/api/admin/users/${u.id}/positions`)
            .then((res) => ({ user: u, trajectory: res.data || [] }))
            .catch(() => ({ user: u, trajectory: [] }))
        )
      );

      const newTracks: FleetUserTrack[] = results.map((r, i) => ({
        id: r.user.id,
        label: `${r.user.name ?? ''} ${r.user.surname ?? ''}`.trim() || `Utente #${r.user.id}`,
        color: TRACK_COLORS[i % TRACK_COLORS.length],
        trajectory: r.trajectory,
        state: r.user.state,
      }));

      setTracks(newTracks);
    } catch (e) {
      console.error('Errore fetch percorsi flotta:', e);
    } finally {
      setMapLoading(false);
    }
  }, [selectedUserIds]);

  const refreshAll = useCallback(async () => {
    const users = await fetchStats();
    await fetchFleetTracks(users);
  }, [fetchStats, fetchFleetTracks]);

  useEffect(() => {
    refreshAll();
    const interval = setInterval(refreshAll, pollIntervalSecs * 1000);
    return () => clearInterval(interval);
  }, [refreshAll, pollIntervalSecs]);

  useEffect(() => {
    const onClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
      }
      if (settingsRef.current && !settingsRef.current.contains(e.target as Node)) {
        setShowSettings(false);
      }
    };
    document.addEventListener('mousedown', onClickOutside);
    return () => document.removeEventListener('mousedown', onClickOutside);
  }, []);

  const toggleUser = (userId: number) => {
    setSelectedUserIds((prev) => {
      const next = new Set(prev);
      if (next.has(userId)) {
        next.delete(userId);
      } else if (next.size < MAX_TRACKED_USERS) {
        next.add(userId);
      }
      return next;
    });
  };

  const handleIntervalChange = () => {
    const val = parseInt(tempPollInterval, 10);
    if (!isNaN(val) && val >= MIN_POLL_INTERVAL_SECS && val <= MAX_POLL_INTERVAL_SECS) {
      setPollIntervalSecs(val);
      setShowSettings(false);
    }
  };

  const getUserLabel = (u: UserItem) => {
    return `${u.name ?? ''} ${u.surname ?? ''}`.trim() || u.username || `Utente #${u.id}`;
  };

  const activeUsersForTracking = allUsers.filter((u) => u.state !== 'disconnected');
  const selectedCount = selectedUserIds.size;

  const cards = [
    {
      title: 'Utenti Totali',
      value: stats.totalUsers,
      icon: <Users size={18} />,
      color: 'text-blue-400',
      bg: 'bg-blue-400/10',
      border: 'border-blue-400/20',
      path: '/admin/users',
    },
    {
      title: 'In Movimento',
      value: stats.movingUsers,
      icon: <Activity size={18} />,
      color: 'text-emerald-400',
      bg: 'bg-emerald-400/10',
      border: 'border-emerald-400/20',
      path: '/admin/users',
    },
    {
      title: 'Attivi Ora',
      value: stats.activeUsers,
      icon: <Radio size={18} />,
      color: 'text-amber-400',
      bg: 'bg-amber-400/10',
      border: 'border-amber-400/20',
      path: '/admin/users',
    },
    {
      title: 'Report',
      value: '→',
      icon: <BarChart3 size={18} />,
      color: 'text-rose-400',
      bg: 'bg-rose-400/10',
      border: 'border-rose-400/20',
      path: '/admin/reports',
    },
  ];

  return (
    <div className="h-screen overflow-hidden relative flex flex-col">
      <AnimatedBackground />
      <Navbar />

      <div className="relative z-10 flex-1 min-h-0 pt-20 pb-6 px-6 flex flex-col gap-4">
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4 }}
          className="flex items-center justify-between flex-shrink-0"
        >
          <div>
            <h1 className="text-2xl font-bold">Dashboard Admin</h1>
            <p className="text-sm text-muted">Panoramica della flotta in tempo reale</p>
          </div>

          <div className="flex items-center gap-2">
            {/* Selettore utenti da tracciare */}
            <div ref={dropdownRef} className="relative">
              <button
                onClick={() => setShowDropdown(!showDropdown)}
                className="flex items-center gap-2 px-4 py-2 rounded-xl bg-white/5 border border-white/10 hover:bg-white/10 transition-colors text-sm"
              >
                <Eye size={16} />
                <span>Tracciamento</span>
                <span className={`text-xs px-2 py-0.5 rounded-full ${selectedCount >= MAX_TRACKED_USERS ? 'bg-amber-400/20 text-amber-400' : 'bg-emerald-400/20 text-emerald-400'}`}>
                  {selectedCount}/{MAX_TRACKED_USERS}
                </span>
                <ChevronDown size={14} className={`transition-transform ${showDropdown ? 'rotate-180' : ''}`} />
              </button>

              <AnimatePresence>
                {showDropdown && (
                  <motion.div
                    className="absolute right-0 top-full mt-2 w-72 max-h-80 overflow-y-auto bg-[#1a1a2e] border border-white/10 rounded-xl p-2 z-[9999] shadow-2xl"
                    initial={{ opacity: 0, y: -8, scale: 0.95 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, y: -8, scale: 0.95 }}
                    transition={{ duration: 0.15 }}
                  >
                    {activeUsersForTracking.length === 0 ? (
                      <p className="text-xs text-muted text-center py-3">Nessun utente attivo</p>
                    ) : (
                      <>
                        <p className="text-xs text-muted px-2 py-1 mb-1">
                          Seleziona fino a {MAX_TRACKED_USERS} utenti
                        </p>
                        {activeUsersForTracking.map((u) => {
                          const isSelected = selectedUserIds.has(u.id);
                          const isDisabled = !isSelected && selectedCount >= MAX_TRACKED_USERS;
                          return (
                            <button
                              key={u.id}
                              type="button"
                              onClick={() => !isDisabled && toggleUser(u.id)}
                              disabled={isDisabled}
                              className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors flex items-center gap-2 ${
                                isSelected
                                  ? 'bg-white/10 text-white'
                                  : isDisabled
                                    ? 'opacity-40 cursor-not-allowed text-neutral-500'
                                    : 'hover:bg-white/[0.05] text-neutral-200'
                              }`}
                            >
                              <div className={`w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 ${
                                isSelected ? 'bg-emerald-400 border-emerald-400' : 'border-white/20'
                              }`}>
                                {isSelected && <Check size={12} className="text-black" />}
                              </div>
                              <span className="truncate">{getUserLabel(u)}</span>
                              <span className={`ml-auto text-[10px] px-1.5 py-0.5 rounded-full ${
                                u.state === 'moving'
                                  ? 'bg-emerald-400/20 text-emerald-400'
                                  : 'bg-amber-400/20 text-amber-400'
                              }`}>
                                {u.state === 'moving' ? 'Mov' : 'Fermo'}
                              </span>
                            </button>
                          );
                        })}
                      </>
                    )}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            {/* Impostazioni polling */}
            <div ref={settingsRef} className="relative">
              <button
                onClick={() => setShowSettings(!showSettings)}
                className="flex items-center gap-2 px-3 py-2 rounded-xl bg-white/5 border border-white/10 hover:bg-white/10 transition-colors text-sm"
                title="Impostazioni aggiornamento"
              >
                <Settings2 size={16} />
                <span className="text-xs text-muted">{pollIntervalSecs}s</span>
              </button>

              <AnimatePresence>
                {showSettings && (
                  <motion.div
                    className="absolute right-0 top-full mt-2 w-64 bg-[#1a1a2e] border border-white/10 rounded-xl p-4 z-[9999] shadow-2xl"
                    initial={{ opacity: 0, y: -8, scale: 0.95 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, y: -8, scale: 0.95 }}
                    transition={{ duration: 0.15 }}
                  >
                    <p className="text-sm font-medium mb-3">Aggiornamento ogni</p>
                    <div className="flex items-center gap-2">
                      <input
                        type="number"
                        min={MIN_POLL_INTERVAL_SECS}
                        max={MAX_POLL_INTERVAL_SECS}
                        value={tempPollInterval}
                        onChange={(e) => setTempPollInterval(e.target.value)}
                        className="glass-input w-20 text-center !py-1.5"
                      />
                      <span className="text-sm text-muted">secondi</span>
                    </div>
                    <p className="text-[10px] text-muted mt-2">
                      Min: {MIN_POLL_INTERVAL_SECS}s — Max: {MAX_POLL_INTERVAL_SECS}s
                    </p>
                    <button
                      onClick={handleIntervalChange}
                      className="btn-primary w-full mt-3 text-sm py-1.5"
                    >
                      Applica
                    </button>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>
        </motion.div>

        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 flex-shrink-0">
          {cards.map((card, i) => (
            <GlassCard
              key={card.title}
              variant="interactive"
              delay={i * 0.05}
              className="cursor-pointer group p-4"
            >
              <div
                onClick={() => navigate(card.path)}
                className="flex items-center gap-3"
              >
                <div
                  className={`w-9 h-9 rounded-lg ${card.bg} ${card.color} flex items-center justify-center border ${card.border} flex-shrink-0`}
                >
                  {card.icon}
                </div>
                <div className="min-w-0">
                  <p className="text-xl font-bold leading-tight">{card.value}</p>
                  <p className="text-xs text-muted truncate">{card.title}</p>
                </div>
              </div>
            </GlassCard>
          ))}
        </div>

        <div className="flex-1 min-h-0 relative">
          <FleetMapView tracks={tracks} loading={mapLoading} />
          
          {/* Overlay "Nessun utente disponibile" */}
          {!mapLoading && tracks.length === 0 && (
            <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
              <div className="bg-[#1a1a2e]/90 border border-white/10 rounded-2xl px-8 py-6 text-center">
                <Eye size={32} className="text-neutral-500 mx-auto mb-3" />
                <p className="text-lg font-medium text-neutral-300">Nessun utente disponibile</p>
                <p className="text-sm text-muted mt-1">
                  {activeUsersForTracking.length === 0
                    ? 'Nessun utente attivo nella flotta'
                    : 'Seleziona almeno un utente dal menu Tracciamento'}
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}