import { motion } from 'framer-motion';
import AnimatedBackground from '../components/ui/AnimatedBackground';
import GlassCard from '../components/ui/GlassCard';
import DiagonalDivider from '../components/ui/DiagonalDivider';
import LoginForm from '../components/auth/LoginForm';
import RegisterForm from '../components/auth/RegisterForm';
import { MapPin } from 'lucide-react';

export default function AuthPage() {
  return (
    <div className="relative min-h-screen flex items-center justify-center p-4">
      <AnimatedBackground />
      
      {/* Logo/Brand */}
      <motion.div
        className="absolute top-8 left-1/2 -translate-x-1/2 flex items-center gap-4"
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
      >
        <div className="w-12 h-12 rounded-xl bg-white/10 border border-white/10 flex items-center justify-center p-1.5">
          <img 
            src="/logo_wb.png" 
            alt="GeoRuggine" 
            className="w-full h-full object-contain"
          />
        </div>
        <span className="text-2xl font-bold tracking-tight text-white">GeoRuggine</span>
      </motion.div>

      {/* Container principale - gap aumentato */}
      <div className="relative z-10 flex items-center justify-center gap-16 max-w-6xl w-full">
        {/* Login Card */}
        <GlassCard className="w-full max-w-sm">
          <LoginForm />
        </GlassCard>

        {/* Divisore diagonale */}
        <DiagonalDivider />

        {/* Register Card */}
        <GlassCard className="w-full max-w-sm">
          <RegisterForm />
        </GlassCard>
      </div>

      {/* Footer */}
      <motion.p
        className="absolute bottom-6 text-xs text-neutral-600"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1 }}
      >
        GeoRuggine — Sistema di geolocalizzazione per flotte
      </motion.p>
    </div>
  );
}