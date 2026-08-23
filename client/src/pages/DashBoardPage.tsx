import { useState, useEffect, useCallback } from 'react';
import Navbar from '../components/layout/Navbar';
import UserSidebar from '../components/dashboard/UserSidebar';
import MapView from '../components/dashboard/MapView';
import AnimatedBackground from '../components/ui/AnimatedBackground';
import { useAuth } from '../context/AuthContext';
import { api } from '../lib/api';
import type { User } from '../types';

interface TrajectoryPoint {
  lat: number;
  lon: number;
  recorded_at: string;
}

interface RouteReport {
  user_id: number;
  trajectory: TrajectoryPoint[];
}

const POLL_INTERVAL_MS = 30_000; // stesso intervallo con cui i dispositivi inviano la posizione

export default function DashboardPage() {
  const [user, setUser] = useState<User | null>(null);
  const [lastUpdate, setLastUpdate] = useState<string | null>(null);
  const [trajectory, setTrajectory] = useState<TrajectoryPoint[]>([]);
  const [position, setPosition] = useState<{ lat: number; lon: number } | null>(null);
  const [state, setState] = useState<'disconnected' | 'stopped' | 'moving'>('disconnected');
  const [loading, setLoading] = useState(true);

  const fetchUser = useCallback(async () => {
    try {
      const res = await api.get('/api/me');
      setUser(res.data);
    } catch (e: any) {
      console.error('Errore fetch user:', e.response?.status, e.response?.data || e.message);
    }
  }, []);

  // Il proprio percorso giornaliero (tragitto reale, calcolato dal server dalle posizioni MQTT)
  const fetchOwnTrajectory = useCallback(async () => {
    try {
      const res = await api.get<RouteReport>('/api/me/report', { params: { period: 'day' } });
      const points = res.data.trajectory || [];
      setTrajectory(points);
      if (points.length > 0) {
        const last = points[points.length - 1];
        setPosition({ lat: last.lat, lon: last.lon });
        setLastUpdate(last.recorded_at);
      }
    } catch (e: any) {
      console.error('Errore fetch tragitto:', e.response?.status, e.response?.data || e.message);
    }
  }, []);

  // Il proprio stato (fermo/in movimento/disconnesso) arriva dalla lista utenti,
  // che include lo stato calcolato lato server dalle sessioni attive.
  const fetchOwnState = useCallback(async () => {
    try {
      const res = await api.get('/api/users');
      const users: any[] = res.data || [];
      const me = users.find((u) => u.id === user?.id);
      if (me) setState(me.state);
    } catch (e: any) {
      console.error('Errore fetch stato utente:', e.response?.status, e.response?.data || e.message);
    }
  }, [user?.id]);

  useEffect(() => {
    (async () => {
      await fetchUser();
      await fetchOwnTrajectory();
      setLoading(false);
    })();
  }, [fetchUser, fetchOwnTrajectory]);

  useEffect(() => {
    if (!user) return;
    fetchOwnState();
  }, [user, fetchOwnState]);

  // Poll periodico: nuovo tragitto + nuovo stato ogni 30s
  useEffect(() => {
    const interval = setInterval(() => {
      fetchOwnTrajectory();
      fetchOwnState();
    }, POLL_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [fetchOwnTrajectory, fetchOwnState]);

  return (
    <div className="min-h-screen relative">
      <AnimatedBackground />
      <Navbar />

      <div className="relative z-10 pt-20 h-screen flex gap-6 px-6 pb-6">
        {loading ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="w-8 h-8 border-2 border-white/20 border-t-white rounded-full animate-spin" />
          </div>
        ) : (
          <>
            <UserSidebar user={user} position={position} state={state} lastUpdate={lastUpdate} />
            <MapView position={position} trajectory={trajectory} />
          </>
        )}
      </div>
    </div>
  );
}