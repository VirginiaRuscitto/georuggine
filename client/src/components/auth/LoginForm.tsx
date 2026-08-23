import { useState } from 'react';
import { motion } from 'framer-motion';
import { Mail, Lock, ArrowRight } from 'lucide-react';
import FormInput from '../ui/FormInput';
import { api } from '../../lib/api';
import { useAuth } from '../../context/AuthContext';
import { useNavigate } from 'react-router-dom';

export default function LoginForm() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const navigate = useNavigate();
  const { login } = useAuth();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');

    try {
      const response = await api.post('/api/login', { email, password });
      const isAdmin = login(response.data.token);
      navigate(isAdmin ? '/admin' : '/');
    } catch (err: any) {
      setError(err.response?.data?.error || 'Errore durante il login');
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-5">
      <motion.h2 
        className="text-2xl font-semibold text-center mb-8"
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
      >
        Accedi
      </motion.h2>

      <FormInput
        type="email"
        placeholder="Email"
        icon={<Mail size={18} />}
        value={email}
        onChange={setEmail}
        delay={0.1}
      />

      <FormInput
        type="password"
        placeholder="Password"
        icon={<Lock size={18} />}
        value={password}
        onChange={setPassword}
        delay={0.2}
      />

      <motion.div
        className="flex items-center justify-center text-xs"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.3 }}
      >
        <span className="text-muted">Inserisci le tue credenziali per accedere</span>
      </motion.div>

      {error && (
        <motion.p 
          className="text-danger text-sm text-center"
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
        >
          {error}
        </motion.p>
      )}

      <motion.button
        type="submit"
        disabled={loading}
        className="btn-primary w-full flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
      >
        {loading ? 'Caricamento...' : (
          <>
            Login
            <ArrowRight size={18} />
          </>
        )}
      </motion.button>
    </form>
  );
}