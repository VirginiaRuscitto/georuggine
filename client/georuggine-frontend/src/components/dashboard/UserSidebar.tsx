import { motion } from 'framer-motion';
import { User, Navigation, Activity, Clock } from 'lucide-react';
import type { User as UserType } from '../../types';

interface UserSidebarProps {
  user: UserType | null;
  position: { lat: number; lon: number } | null;
  state: 'disconnected' | 'stopped' | 'moving';
}

export default function UserSidebar({ user, position, state }: UserSidebarProps) {
  const stateConfig = {
    disconnected: { color: 'text-neutral-500', bg: 'bg-neutral-500/10', label: 'Offline' },
    stopped: { color: 'text-amber-400', bg: 'bg-amber-400/10', label: 'Fermo' },
    moving: { color: 'text-emerald-400', bg: 'bg-emerald-400/10', label: 'In movimento' },
  };

  const config = stateConfig[state];

  return (
    <motion.div
      className="w-[30%] min-w-[320px] h-full glass-card flex flex-col"
      initial={{ opacity: 0, x: -30 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.5 }}
    >
      {/* Header */}
      <div className="flex items-center gap-3 mb-6">
        <div className="w-12 h-12 rounded-full bg-white/5 border border-white/10 flex items-center justify-center">
          <User size={20} className="text-neutral-400" />
        </div>
        <div>
          <h2 className="font-semibold text-lg">{user?.name} {user?.surname}</h2>
          <p className="text-xs text-muted">{user?.email}</p>
        </div>
      </div>

      {/* Stato */}
      <div className="mb-6">
        <h3 className="text-xs uppercase tracking-wider text-muted mb-3">Stato attuale</h3>
        <div className={`flex items-center gap-3 p-4 rounded-xl ${config.bg} border border-white/5`}>
          <Activity size={20} className={config.color} />
          <div>
            <p className={`font-medium ${config.color}`}>{config.label}</p>
            <p className="text-xs text-muted mt-0.5">Aggiornato ora</p>
          </div>
        </div>
      </div>

      {/* Posizione */}
      <div className="mb-6">
        <h3 className="text-xs uppercase tracking-wider text-muted mb-3">Posizione</h3>
        <div className="p-4 rounded-xl bg-white/[0.02] border border-white/5 space-y-3">
          <div className="flex items-center gap-3">
            <Navigation size={16} className="text-neutral-400" />
            <div>
              <p className="text-xs text-muted">Latitudine</p>
              <p className="font-mono text-sm">{position?.lat.toFixed(6) ?? '--'}</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Navigation size={16} className="text-neutral-400 rotate-90" />
            <div>
              <p className="text-xs text-muted">Longitudine</p>
              <p className="font-mono text-sm">{position?.lon.toFixed(6) ?? '--'}</p>
            </div>
          </div>
        </div>
      </div>

      {/* Ultimo aggiornamento */}
      <div className="mt-auto">
        <div className="flex items-center gap-2 text-xs text-muted">
          <Clock size={14} />
          <span>Ultimo aggiornamento: {new Date().toLocaleTimeString('it-IT')}</span>
        </div>
      </div>
    </motion.div>
  );
}