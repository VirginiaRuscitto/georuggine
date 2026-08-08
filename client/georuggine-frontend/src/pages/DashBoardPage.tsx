import { useState, useEffect } from 'react';
import Navbar from '../components/layout/Navbar';
import UserSidebar from '../components/dashboard/UserSidebar';
import MapView from '../components/dashboard/MapView';
import { useAuth } from '../context/AuthContext';
import { api } from '../lib/api';
import type { User } from '../types';

export default function DashboardPage() {
  const [user, setUser] = useState<User | null>(null);
  const [position, setPosition] = useState<{ lat: number; lon: number } | null>(null);
  const [state, setState] = useState<'disconnected' | 'stopped' | 'moving'>('stopped');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchUser = async () => {
      try {
        const token = localStorage.getItem('token');
        console.log('Token trovato:', token ? 'Sì' : 'No');
        
        const res = await api.get('/api/me');
        console.log('User data:', res.data);
        setUser(res.data);
      } catch (e: any) {
        console.error('Errore fetch user:', e.response?.status, e.response?.data || e.message);
      } finally {
        setLoading(false);
      }
    };

    fetchUser();
    
    // Simula posizione per test
    setPosition({ lat: 45.4642, lon: 9.1900 });
    setState('moving');
  }, []);

  // Simula aggiornamento posizione ogni 30s
  useEffect(() => {
    const interval = setInterval(() => {
      setPosition(prev => {
        if (!prev) return prev;
        return {
          lat: prev.lat + (Math.random() - 0.5) * 0.001,
          lon: prev.lon + (Math.random() - 0.5) * 0.001,
        };
      });
    }, 30000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-background">
      <Navbar activePage="home" />
      
      <div className="pt-20 h-screen flex gap-6 px-6 pb-6">
        {loading ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="w-8 h-8 border-2 border-white/20 border-t-white rounded-full animate-spin" />
          </div>
        ) : (
          <>
            <UserSidebar
              user={user}
              position={position}
              state={state}
            />
            <MapView position={position} />
          </>
        )}
      </div>
    </div>
  );
}