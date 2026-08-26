import { useState } from 'react';
import { motion } from 'framer-motion';
import { User, Mail, Lock, UserPlus } from 'lucide-react';
import FormInput from '../ui/FormInput';
import { api } from '../../lib/api';
import { useAuth } from '../../context/AuthContext';

export default function RegisterForm() {
  const [name, setName] = useState('');
  const [surname, setSurname] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const { login } = useAuth();

    const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    setSuccess('');

    try {
      const response = await api.post('/api/register', {
        name,
        surname,
        email,
        password,
      });
      login(response.data.token);
      setSuccess('Registrazione completata!');
      setName('');
      setSurname('');
      setEmail('');
      setPassword('');
    } catch (err: any) {
      setError(err.response?.data?.error || 'Errore durante la registrazione');
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <motion.h2 
        className="text-2xl font-semibold text-center mb-8"
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
      >
        Registrati
      </motion.h2>

      <div className="grid grid-cols-2 gap-4">
        <FormInput
          placeholder="Nome"
          icon={<User size={18} />}
          value={name}
          onChange={setName}
          delay={0.1}
        />
        <FormInput
          placeholder="Cognome"
          icon={<User size={18} />}
          value={surname}
          onChange={setSurname}
          delay={0.15}
        />
      </div>

      <FormInput
        type="email"
        placeholder="Email"
        icon={<Mail size={18} />}
        value={email}
        onChange={setEmail}
        delay={0.2}
      />

      <FormInput
        type="password"
        placeholder="Password (min 12 caratteri, maiuscola, simbolo)"
        icon={<Lock size={18} />}
        value={password}
        onChange={setPassword}
        delay={0.25}
      />

      {error && (
        <motion.p 
          className="text-danger text-sm text-center"
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
        >
          {error}
        </motion.p>
      )}

      {success && (
        <motion.p 
          className="text-success text-sm text-center"
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
        >
          {success}
        </motion.p>
      )}

      <motion.button
        type="submit"
        disabled={loading}
        className="btn-secondary w-full flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
      >
        {loading ? 'Caricamento...' : (
          <>
            Crea account
            <UserPlus size={18} />
          </>
        )}
      </motion.button>
    </form>
  );
}