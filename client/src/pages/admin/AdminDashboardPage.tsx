import { useState, useEffect, useCallback } from 'react';
import { motion } from 'framer-motion';
import { useNavigate } from 'react-router-dom';
import { Users, Activity, MessageSquare, Radio } from 'lucide-react';
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
  totalMessages: number;
}

// Palette distinta per identificare i percorsi sulla mappa
const TRACK_COLORS = ['#38bdf8', '#34d399', '#fbbf24', '#f472b6', '#a78bfa'];

// Quanti percorsi mostrare in mappa contemporaneamente
const MAX_TRACKED_USERS = 3;

export default function AdminDashboardPage() {
  const navigate = useNavigate();
  const [stats, setStats] = useState<DashboardStats>({
    totalUsers: 0,
    activeUsers: 0,
    movingUsers: 0,
    totalMessages: 0,
  });
  const [tracks, setTracks] = useState<FleetUserTrack[]>([]);
  const [mapLoading, setMapLoading] = useState(true);

  const fetchStats = useCallback(async () => {
    try {
      const usersRes = await api.get('/api/users');
      const users = usersRes.data || [];
      const active = users.filter((u: any) => u.state !== 'disconnected').length;
      const moving = users.filter((u: any) => u.state === 'moving').length;

      setStats({
        totalUsers: users.length,
        activeUsers: active,
        movingUsers: moving,
        totalMessages: 0, // TODO: endpoint conteggio
      });

      return users as any[];
    } catch (e) {
      console.error('Errore fetch stats:', e);
      return [];
    }
  }, []);

  // Seleziona fino a MAX_TRACKED_USERS utenti da tracciare, dando priorità
  // a chi è in movimento, poi a chi è comunque attivo.
  const pickUsersToTrack = (users: any[]) => {
    const moving = users.filter((u) => u.state === 'moving');
    const stopped = users.filter((u) => u.state === 'stopped');
    return [...moving, ...stopped].slice(0, MAX_TRACKED_USERS);
  };

  const fetchFleetTracks = useCallback(async (users: any[]) => {
    const selected = pickUsersToTrack(users);

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
            .get('/api/report', { params: { user_id: u.id, period: 'day' } })
            .then((res) => ({ user: u, trajectory: res.data.trajectory || [] }))
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
  }, []);

  const refreshAll = useCallback(async () => {
    const users = await fetchStats();
    await fetchFleetTracks(users);
  }, [fetchStats, fetchFleetTracks]);

  useEffect(() => {
    refreshAll();
    const interval = setInterval(refreshAll, 30_000);
    return () => clearInterval(interval);
  }, [refreshAll]);

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
      title: 'Messaggi Totali',
      value: stats.totalMessages,
      icon: <MessageSquare size={18} />,
      color: 'text-purple-400',
      bg: 'bg-purple-400/10',
      border: 'border-purple-400/20',
      path: '/admin/messages',
    },
  ];

  return (
    <div className="h-screen overflow-hidden relative flex flex-col">
      <AnimatedBackground />
      <Navbar />

      <div className="relative z-10 flex-1 min-h-0 pt-20 pb-6 px-6 flex flex-col gap-4">
        {/* Header compatto */}
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
        </motion.div>

        {/* Stats compatte */}
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

        {/* Mappa flotta: occupa tutto lo spazio rimanente, nessuno scroll */}
        <div className="flex-1 min-h-0">
          <FleetMapView tracks={tracks} loading={mapLoading} />
        </div>
      </div>
    </div>
  );
}