import { useState, useEffect, useRef, useCallback } from 'react';
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

const POLL_INTERVAL_MS = 30_000;

export default function DashboardPage() {
  const [user, setUser] = useState<User | null>(null);
  const [trajectory, setTrajectory] = useState<TrajectoryPoint[]>([]);
  const [state, setState] = useState<'disconnected' | 'stopped' | 'moving'>('disconnected');
  const [loading, setLoading] = useState(true);
  const wasOnlineRef = useRef(false);

  const fetchUser = useCallback(async () => {
    try {
      const res = await api.get('/api/me');
      setUser(res.data);
      setState(res.data.state ?? 'disconnected');
    } catch (e: any) {
      console.error('Errore fetch user:', e.response?.status, e.response?.data || e.message);
    }
  }, []);

  const fetchOwnTrajectory = useCallback(async () => {
    try {
      const res = await api.get<TrajectoryPoint[]>('/api/me/positions', { params: { period: 'day' } });
      setTrajectory(res.data || []);
    } catch (e: any) {
      console.error('Errore fetch tragitto:', e.response?.status, e.response?.data || e.message);
    }
  }, []);

  useEffect(() => {
    (async () => {
      await fetchUser();
      setLoading(false);
    })();
  }, [fetchUser]);

  // Gestione sessione corrente:
  // - utente disconnesso -> la mappa non mostra nulla, niente fetch della tratta
  // - utente online (transizione da disconnected -> stopped/moving) -> si azzera
  //   la traiettoria precedente e si ricomincia a tracciare
  // - utente già online -> si ricarica la tratta corrente della sessione
  useEffect(() => {
    if (state === 'disconnected') {
      wasOnlineRef.current = false;
      setTrajectory([]);
      return;
    }

    if (!wasOnlineRef.current) {
      // prima osservazione di una sessione online: riparti da zero
      wasOnlineRef.current = true;
      setTrajectory([]);
    }

    fetchOwnTrajectory();
  }, [state, fetchOwnTrajectory]);

  // Polling utente: ogni 30s ricarica lo stato
  useEffect(() => {
    const interval = setInterval(() => {
      fetchUser();
    }, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [fetchUser]);

  // Polling traiettoria: mentre l'utente è online, ricarica la tratta ogni 30s
  useEffect(() => {
    if (state === 'disconnected') return;

    fetchOwnTrajectory(); // fetch immediato
    const interval = setInterval(() => {
      fetchOwnTrajectory();
    }, POLL_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [state, fetchOwnTrajectory]);

  const position = trajectory.length > 0
    ? { lat: trajectory[trajectory.length - 1].lat, lon: trajectory[trajectory.length - 1].lon }
    : null;

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
            <UserSidebar user={user} position={position} state={state} />
            <MapView position={position} trajectory={trajectory} />
          </>
        )}
      </div>
    </div>
  );
}