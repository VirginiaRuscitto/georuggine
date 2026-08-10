import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { useNavigate } from 'react-router-dom';
import { Users, Activity, MessageSquare, BarChart3, ArrowRight } from 'lucide-react';
import Navbar from '../../components/layout/Navbar';
import AnimatedBackground from '../../components/ui/AnimatedBackground';
import GlassCard from '../../components/ui/GlassCard';
import { api } from '../../lib/api';

interface DashboardStats {
  totalUsers: number;
  activeUsers: number;
  movingUsers: number;
  totalMessages: number;
}

export default function AdminDashboardPage() {
  const navigate = useNavigate();
  const [stats, setStats] = useState<DashboardStats>({
    totalUsers: 0,
    activeUsers: 0,
    movingUsers: 0,
    totalMessages: 0,
  });
  const [recentUsers, setRecentUsers] = useState<any[]>([]);

  useEffect(() => {
    fetchStats();
  }, []);

  const fetchStats = async () => {
    try {
      const usersRes = await api.get('/api/users');
      const messagesRes = await api.get('/api/messages?limit=1');

      const users = usersRes.data || [];
      const active = users.filter((u: any) => u.state !== 'disconnected').length;
      const moving = users.filter((u: any) => u.state === 'moving').length;

      setStats({
        totalUsers: users.length,
        activeUsers: active,
        movingUsers: moving,
        totalMessages: 0, // TODO: endpoint conteggio
      });
      setRecentUsers(users.slice(0, 5));
    } catch (e) {
      console.error('Errore fetch stats:', e);
    }
  };

  const cards = [
    {
      title: 'Utenti Totali',
      value: stats.totalUsers,
      icon: <Users size={20} />,
      color: 'text-blue-400',
      bg: 'bg-blue-400/10',
      border: 'border-blue-400/20',
      path: '/admin/users',
    },
    {
      title: 'In Movimento',
      value: stats.movingUsers,
      icon: <Activity size={20} />,
      color: 'text-emerald-400',
      bg: 'bg-emerald-400/10',
      border: 'border-emerald-400/20',
      path: '/admin/users',
    },
    {
      title: 'Attivi Ora',
      value: stats.activeUsers,
      icon: <Activity size={20} />,
      color: 'text-amber-400',
      bg: 'bg-amber-400/10',
      border: 'border-amber-400/20',
      path: '/admin/users',
    },
    {
      title: 'Messaggi Totali',
      value: stats.totalMessages,
      icon: <MessageSquare size={20} />,
      color: 'text-purple-400',
      bg: 'bg-purple-400/10',
      border: 'border-purple-400/20',
      path: '/admin/messages',
    },
  ];

  return (
    <div className="min-h-screen relative">
      <AnimatedBackground />
      <Navbar />

      <div className="relative z-10 pt-24 pb-12 px-6 max-w-7xl mx-auto">
        {/* Header */}
        <motion.div
          className="mb-10"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          <h1 className="text-3xl font-bold mb-2">Dashboard Admin</h1>
          <p className="text-muted">Panoramica della flotta e del sistema</p>
        </motion.div>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-10">
          {cards.map((card, i) => (
            <GlassCard
              key={card.title}
              variant="interactive"
              delay={i * 0.1}
              className="cursor-pointer group p-6"
            >
              <div onClick={() => navigate(card.path)}>
                <div className="flex items-start justify-between mb-4">
                  <div
                    className={`w-10 h-10 rounded-lg ${card.bg} ${card.color} flex items-center justify-center border ${card.border}`}
                  >
                    {card.icon}
                  </div>
                  <ArrowRight
                    size={16}
                    className="text-muted group-hover:text-white group-hover:translate-x-1 transition-all"
                  />
                </div>
                <p className="text-3xl font-bold mb-1">{card.value}</p>
                <p className="text-sm text-muted">{card.title}</p>
              </div>
            </GlassCard>
          ))}
        </div>

        {/* Sezioni Rapide */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Utenti Recenti */}
          <GlassCard variant="hover" delay={0.4} className="p-6">
            <div className="flex items-center justify-between mb-6">
              <h2 className="text-lg font-semibold">Utenti Recenti</h2>
              <button
                onClick={() => navigate('/admin/users')}
                className="text-xs text-muted hover:text-white transition-colors flex items-center gap-1"
              >
                Vedi tutti <ArrowRight size={12} />
              </button>
            </div>

            <div className="space-y-3">
              {recentUsers.length === 0 ? (
                <p className="text-sm text-muted text-center py-8">Nessun utente registrato</p>
              ) : (
                recentUsers.map((user: any) => (
                  <div
                    key={user.id}
                    className="flex items-center gap-3 p-3 rounded-lg bg-white/[0.02] border border-white/[0.04] hover:bg-white/[0.05] hover:border-white/[0.08] transition-all"
                  >
                    <div className="w-8 h-8 rounded-full bg-white/5 flex items-center justify-center text-xs font-bold">
                      {user.username?.charAt(0)?.toUpperCase() || 'U'}
                    </div>
                    <div className="flex-1">
                      <p className="text-sm font-medium">{user.username}</p>
                      <p className="text-xs text-muted">ID: {user.id}</p>
                    </div>
                    <div
                      className={`w-2 h-2 rounded-full ${
                        user.state === 'moving'
                          ? 'bg-emerald-400'
                          : user.state === 'stopped'
                          ? 'bg-amber-400'
                          : 'bg-neutral-600'
                      }`}
                    />
                  </div>
                ))
              )}
            </div>
          </GlassCard>

          {/* Accesso Rapido Reports */}
          <GlassCard variant="hover" delay={0.5} className="p-6">
            <div className="flex items-center justify-between mb-6">
              <h2 className="text-lg font-semibold">Report Rapido</h2>
              <BarChart3 size={18} className="text-muted" />
            </div>

            <div className="space-y-3">
              <QuickReportButton
                label="Report Giornaliero"
                period="Oggi"
                onClick={() => navigate('/admin/reports')}
              />
              <QuickReportButton
                label="Report Settimanale"
                period="Questa settimana"
                onClick={() => navigate('/admin/reports')}
              />
              <QuickReportButton
                label="Report Mensile"
                period="Questo mese"
                onClick={() => navigate('/admin/reports')}
              />
            </div>
          </GlassCard>
        </div>
      </div>
    </div>
  );
}

function QuickReportButton({
  label,
  period,
  onClick,
}: {
  label: string;
  period: string;
  onClick: () => void;
}) {
  return (
    <motion.button
      onClick={onClick}
      className="w-full p-4 rounded-xl bg-white/[0.02] border border-white/[0.04] hover:bg-white/[0.05] hover:border-white/[0.08] transition-all text-left group"
      whileHover={{ x: 4 }}
      whileTap={{ scale: 0.99 }}
    >
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium group-hover:text-white transition-colors">{label}</p>
          <p className="text-xs text-muted">{period}</p>
        </div>
        <ArrowRight
          size={16}
          className="text-muted group-hover:text-white group-hover:translate-x-1 transition-all"
        />
      </div>
    </motion.button>
  );
}
