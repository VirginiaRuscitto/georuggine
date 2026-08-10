import { useNavigate, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Home, MessageSquare, LogOut, LayoutDashboard, Users, BarChart3 } from 'lucide-react';
import { useAuth } from '../../context/AuthContext';

export default function Navbar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { logout, isAdmin } = useAuth();

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const isActive = (path: string) => location.pathname === path;

  return (
    <motion.nav
      className="fixed top-0 left-0 right-0 z-50 glass border-b border-white/[0.06]"
      initial={{ y: -60 }}
      animate={{ y: 0 }}
      transition={{ duration: 0.5 }}
    >
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
        {/* Logo */}
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-neutral-800 border border-white/10 flex items-center justify-center p-1">
            <img src="/logo_wb.png" alt="GeoRuggine" className="w-full h-full object-contain" />
          </div>
          <span className="font-bold text-lg tracking-tight">GeoRuggine</span>
        </div>

        {/* Nav Links */}
        <div className="flex items-center gap-1">
          {isAdmin ? (
            <>
              <NavButton
                icon={<LayoutDashboard size={18} />}
                label="Dashboard"
                active={isActive('/admin')}
                onClick={() => navigate('/admin')}
              />
              <NavButton
                icon={<Users size={18} />}
                label="Utenti"
                active={isActive('/admin/users')}
                onClick={() => navigate('/admin/users')}
              />
              <NavButton
                icon={<MessageSquare size={18} />}
                label="Messaggi"
                active={isActive('/admin/messages')}
                onClick={() => navigate('/admin/messages')}
              />
              <NavButton
                icon={<BarChart3 size={18} />}
                label="Report"
                active={isActive('/admin/reports')}
                onClick={() => navigate('/admin/reports')}
              />
            </>
          ) : (
            <>
              <NavButton
                icon={<Home size={18} />}
                label="Home"
                active={isActive('/')}
                onClick={() => navigate('/')}
              />
              <NavButton
                icon={<MessageSquare size={18} />}
                label="Messaggi"
                active={isActive('/messages')}
                onClick={() => navigate('/messages')}
              />
            </>
          )}
        </div>

        {/* Logout */}
        <motion.button
          onClick={handleLogout}
          className="flex items-center gap-2 px-4 py-2 rounded-lg text-sm text-muted hover:text-white hover:bg-white/5 transition-colors"
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
        >
          <LogOut size={16} />
          <span>Logout</span>
        </motion.button>
      </div>
    </motion.nav>
  );
}

function NavButton({ icon, label, active, onClick }: {
  icon: React.ReactNode;
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <motion.button
      onClick={onClick}
      className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm transition-colors ${
        active
          ? 'bg-white/10 text-white'
          : 'text-muted hover:text-white hover:bg-white/5'
      }`}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
    >
      {icon}
      <span>{label}</span>
    </motion.button>
  );
}