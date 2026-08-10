import { useState, useEffect, useMemo, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Search,
  SlidersHorizontal,
  Trash2,
  User,
  UserPlus,
  CircleAlert,
  CircleCheck,
  Plus,
  Shield,
} from 'lucide-react';
import Navbar from '../../components/layout/Navbar';
import AnimatedBackground from '../../components/ui/AnimatedBackground';
import GlassCard from '../../components/ui/GlassCard';
import { api } from '../../lib/api';
import { useAuth } from '../../context/AuthContext';

interface UserItem {
  id: number;
  username: string;
  state: string;
  is_admin: boolean;
}

interface NewUserForm {
  name: string;
  surname: string;
  email: string;
  password: string;
  confirmPassword: string;
  isAdmin: boolean;
}

const EMPTY_FORM: NewUserForm = {
  name: '',
  surname: '',
  email: '',
  password: '',
  confirmPassword: '',
  isAdmin: false,
};

export default function AdminUsersPage() {
  const { userId } = useAuth();
  const [users, setUsers] = useState<UserItem[]>([]);
  const [search, setSearch] = useState('');
  const [showFilters, setShowFilters] = useState(false);
  const [adminFilter, setAdminFilter] = useState<'all' | 'admin' | 'non-admin'>('all');
  const [stateFilter, setStateFilter] = useState<'all' | 'moving' | 'stopped' | 'disconnected'>('all');
  const [loading, setLoading] = useState(true);
  const [form, setForm] = useState<NewUserForm>(EMPTY_FORM);
  const [formError, setFormError] = useState('');
  const [formSuccess, setFormSuccess] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const filterRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetchUsers();
  }, []);

  useEffect(() => {
    const onClickOutside = (e: MouseEvent) => {
      if (filterRef.current && !filterRef.current.contains(e.target as Node)) {
        setShowFilters(false);
      }
    };
    document.addEventListener('mousedown', onClickOutside);
    return () => document.removeEventListener('mousedown', onClickOutside);
  }, []);

  const fetchUsers = async () => {
    try {
      const res = await api.get('/api/users');
      setUsers(res.data || []);
    } catch (e) {
      console.error('Errore fetch users:', e);
    } finally {
      setLoading(false);
    }
  };

  const filteredUsers = useMemo(() => {
    const term = search.toLowerCase();
    return users.filter((u) => {
      if (term && !(u.username?.toLowerCase() || '').includes(term)) return false;
      if (adminFilter === 'admin' && !u.is_admin) return false;
      if (adminFilter === 'non-admin' && u.is_admin) return false;
      if (stateFilter !== 'all' && u.state !== stateFilter) return false;
      return true;
    });
  }, [users, search, adminFilter, stateFilter]);

  const handleToggleAdmin = async (u: UserItem) => {
    if (u.id === userId) {
      alert('Non puoi modificare il tuo stesso account');
      return;
    }
    // Aggiornamento ottimistico
    const previousValue = u.is_admin;
    setUsers((prev) => prev.map((x) => (x.id === u.id ? { ...x, is_admin: !x.is_admin } : x)));
    try {
      const res = await api.put(`/api/admin/users/${u.id}/admin`, { is_admin: !u.is_admin });
      console.log(`Toggle admin OK per utente ${u.id}, status:`, res.status);
    } catch (e: any) {
      console.error('Errore toggle admin:', {
        status: e.response?.status,
        data: e.response?.data,
        message: e.message,
      });
      // Rollback
      setUsers((prev) =>
        prev.map((x) => (x.id === u.id ? { ...x, is_admin: previousValue } : x))
      );
      const serverMsg = e.response?.data?.error || e.response?.data?.message;
      const status = e.response?.status;
      const detail = status ? ` [HTTP ${status}]` : '';
      alert(`Impossibile aggiornare l'utente${detail}${serverMsg ? `: ${serverMsg}` : ''}`);
    }
  };

  const handleDelete = async (u: UserItem) => {
    if (u.id === userId) {
      alert('Non puoi eliminare il tuo stesso account');
      return;
    }
    if (!window.confirm(`Eliminare l'utente ${u.username}?`)) return;
    const previous = users;
    setUsers((prev) => prev.filter((x) => x.id !== u.id));
    try {
      await api.delete(`/api/admin/users/${u.id}`);
    } catch (e: any) {
      console.error('Errore delete user:', e.response?.data || e.message);
      setUsers(previous);
      alert(e.response?.data?.error || 'Impossibile eliminare l\'utente');
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError('');
    setFormSuccess('');

    if (!form.name.trim() || !form.surname.trim()) {
      setFormError('Inserisci nome e cognome');
      return;
    }
    if (!form.email.trim()) {
      setFormError('Inserisci una email');
      return;
    }
    if (form.password.length < 12 || !/[A-Z]/.test(form.password) || !/[^a-zA-Z0-9]/.test(form.password)) {
      setFormError('La password deve avere almeno 12 caratteri, una maiuscola e un simbolo');
      return;
    }
    if (form.password !== form.confirmPassword) {
      setFormError('Le password non coincidono');
      return;
    }

    setSubmitting(true);
    try {
      await api.post('/api/admin/register', {
        name: form.name,
        surname: form.surname,
        email: form.email,
        password: form.password,
        is_admin: form.isAdmin,
      });
      setFormSuccess('Utente creato con successo');
      setForm(EMPTY_FORM);
      await fetchUsers();
      setTimeout(() => setFormSuccess(''), 3000);
    } catch (e: any) {
      setFormError(e.response?.data?.error || 'Errore durante la registrazione');
    } finally {
      setSubmitting(false);
    }
  };

  const hasActiveFilters = adminFilter !== 'all' || stateFilter !== 'all';

  return (
    <div className="min-h-screen relative">
      <AnimatedBackground />
      <Navbar />

      <div className="relative z-10 pt-24 pb-12 px-6 max-w-7xl mx-auto">
        <motion.div
          className="mb-8"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          <h1 className="text-3xl font-bold mb-2">Gestione Utenti</h1>
          <p className="text-muted">Aggiungi, promuovi o rimuovi gli utenti del sistema</p>
        </motion.div>

        <div className="grid grid-cols-1 lg:grid-cols-[380px_1fr] gap-6 items-start">
          {/* Form registrazione */}
          <GlassCard variant="hover" delay={0.1}>
            <div className="flex items-center gap-3 mb-6">
              <div className="w-9 h-9 rounded-lg bg-white/5 border border-white/10 flex items-center justify-center">
                <UserPlus size={18} className="text-neutral-300" />
              </div>
              <h2 className="text-lg font-semibold">Registra un nuovo utente</h2>
            </div>

            <form onSubmit={handleSubmit} className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <FormField
                  label="Nome"
                  value={form.name}
                  onChange={(v) => setForm({ ...form, name: v })}
                />
                <FormField
                  label="Cognome"
                  value={form.surname}
                  onChange={(v) => setForm({ ...form, surname: v })}
                />
              </div>
              <FormField
                label="Mail"
                type="email"
                value={form.email}
                onChange={(v) => setForm({ ...form, email: v })}
              />
              <FormField
                label="Pass"
                type="password"
                value={form.password}
                onChange={(v) => setForm({ ...form, password: v })}
              />
              <FormField
                label="Pass"
                type="password"
                value={form.confirmPassword}
                onChange={(v) => setForm({ ...form, confirmPassword: v })}
              />

              <label className="flex items-center gap-2 text-sm cursor-pointer select-none pt-1">
                <input
                  type="checkbox"
                  checked={form.isAdmin}
                  onChange={(e) => setForm({ ...form, isAdmin: e.target.checked })}
                  className="w-4 h-4 rounded border-white/20 bg-white/5 cursor-pointer accent-white"
                />
                <span>IsAdmin?</span>
              </label>

              <AnimatePresence>
                {formError && (
                  <motion.div
                    className="flex items-center gap-2 text-sm text-danger bg-danger/10 border border-danger/20 rounded-lg px-3 py-2"
                    initial={{ opacity: 0, y: -5 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                  >
                    <CircleAlert size={14} />
                    {formError}
                  </motion.div>
                )}
                {formSuccess && (
                  <motion.div
                    className="flex items-center gap-2 text-sm text-success bg-success/10 border border-success/20 rounded-lg px-3 py-2"
                    initial={{ opacity: 0, y: -5 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                  >
                    <CircleCheck size={14} />
                    {formSuccess}
                  </motion.div>
                )}
              </AnimatePresence>

              <motion.button
                type="submit"
                disabled={submitting}
                className="btn-primary w-full flex items-center justify-center gap-2 disabled:opacity-50"
                whileHover={{ scale: 1.01 }}
                whileTap={{ scale: 0.99 }}
              >
                {submitting ? (
                  'Creazione...'
                ) : (
                  <>
                    <Plus size={16} />
                    Crea utente
                  </>
                )}
              </motion.button>
            </form>
          </GlassCard>

          {/* Lista utenti */}
          <GlassCard variant="hover" delay={0.2}>
            <div className="flex items-center justify-center mb-6">
              <h2 className="text-base font-semibold text-muted tracking-wide">Users list</h2>
            </div>

            {/* Search + filters */}
            <div className="flex flex-wrap gap-3 mb-6">
              <div className="relative flex-1 min-w-[200px]">
                <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
                <input
                  type="text"
                  placeholder="Search"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  className="glass-input w-full !py-2.5 !pl-10 !pr-4 !text-sm"
                />
              </div>

              <div className="relative" ref={filterRef}>
                <motion.button
                  type="button"
                  onClick={() => setShowFilters((v) => !v)}
                  className={`flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm border transition-colors ${
                    hasActiveFilters
                      ? 'bg-white/10 border-white/20 text-white'
                      : 'bg-white/[0.03] border-white/[0.06] text-muted hover:text-white hover:bg-white/[0.05]'
                  }`}
                  whileTap={{ scale: 0.97 }}
                >
                  <SlidersHorizontal size={14} />
                  Filters admin/state
                </motion.button>

                <AnimatePresence>
                  {showFilters && (
                    <motion.div
                      className="absolute right-0 top-full mt-2 w-64 glass-card glass-card-subtle !p-4 z-20"
                      initial={{ opacity: 0, y: -8 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: -8 }}
                      transition={{ duration: 0.15 }}
                    >
                      <div className="mb-3">
                        <p className="text-xs uppercase tracking-wider text-muted mb-2">Ruolo</p>
                        <div className="space-y-1.5">
                          <FilterOption
                            label="Tutti"
                            checked={adminFilter === 'all'}
                            onChange={() => setAdminFilter('all')}
                          />
                          <FilterOption
                            label="Solo admin"
                            checked={adminFilter === 'admin'}
                            onChange={() => setAdminFilter('admin')}
                          />
                          <FilterOption
                            label="Solo non admin"
                            checked={adminFilter === 'non-admin'}
                            onChange={() => setAdminFilter('non-admin')}
                          />
                        </div>
                      </div>
                      <div>
                        <p className="text-xs uppercase tracking-wider text-muted mb-2">Stato</p>
                        <div className="space-y-1.5">
                          <FilterOption
                            label="Tutti"
                            checked={stateFilter === 'all'}
                            onChange={() => setStateFilter('all')}
                          />
                          <FilterOption
                            label="Moving"
                            checked={stateFilter === 'moving'}
                            onChange={() => setStateFilter('moving')}
                          />
                          <FilterOption
                            label="Stopped"
                            checked={stateFilter === 'stopped'}
                            onChange={() => setStateFilter('stopped')}
                          />
                          <FilterOption
                            label="Disconnected"
                            checked={stateFilter === 'disconnected'}
                            onChange={() => setStateFilter('disconnected')}
                          />
                        </div>
                      </div>
                      {hasActiveFilters && (
                        <button
                          type="button"
                          onClick={() => {
                            setAdminFilter('all');
                            setStateFilter('all');
                          }}
                          className="mt-3 text-xs text-muted hover:text-white transition-colors w-full text-left"
                        >
                          Reset filtri
                        </button>
                      )}
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
            </div>

            {/* Lista */}
            <div className="space-y-2 max-h-[60vh] overflow-y-auto pr-1">
              {loading ? (
                <div className="py-12 text-center text-sm text-muted">Caricamento...</div>
              ) : filteredUsers.length === 0 ? (
                <div className="py-12 text-center text-sm text-muted">
                  {users.length === 0 ? 'Nessun utente registrato' : 'Nessun risultato'}
                </div>
              ) : (
                filteredUsers.map((u, i) => (
                  <UserRow
                    key={u.id}
                    user={u}
                    index={i}
                    isSelf={u.id === userId}
                    onToggleAdmin={() => handleToggleAdmin(u)}
                    onDelete={() => handleDelete(u)}
                  />
                ))
              )}
            </div>
          </GlassCard>
        </div>
      </div>
    </div>
  );
}

function FormField({
  label,
  value,
  onChange,
  type = 'text',
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
}) {
  return (
    <div>
      <label className="text-xs text-muted mb-1 block">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="glass-input !py-2.5 !pl-4 !pr-4 !text-sm"
      />
    </div>
  );
}

function FilterOption({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: () => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm cursor-pointer select-none hover:text-white transition-colors">
      <input
        type="radio"
        checked={checked}
        onChange={onChange}
        className="w-3.5 h-3.5 accent-white"
      />
      <span>{label}</span>
    </label>
  );
}

function UserRow({
  user,
  index,
  isSelf,
  onToggleAdmin,
  onDelete,
}: {
  user: UserItem;
  index: number;
  isSelf: boolean;
  onToggleAdmin: () => void;
  onDelete: () => void;
}) {
  const stateColor =
    user.state === 'moving'
      ? 'bg-emerald-400'
      : user.state === 'stopped'
      ? 'bg-amber-400'
      : 'bg-neutral-600';

  return (
    <motion.div
      className="grid grid-cols-[44px_minmax(0,1fr)_140px_140px_88px] items-center gap-3 p-3 rounded-lg bg-white/[0.02] border border-white/[0.04] hover:bg-white/[0.04] transition-colors"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, delay: Math.min(index * 0.03, 0.3) }}
    >
      {/* Avatar */}
      <div className="w-11 h-11 rounded-full bg-white/5 border border-white/10 flex items-center justify-center">
        <User size={18} className="text-neutral-400" />
      </div>

      {/* Nome + badge admin */}
      <div className="min-w-0 flex items-center gap-2">
        <p className="text-sm font-medium truncate">{user.username}</p>
        {user.is_admin && (
          <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/30 text-amber-400 text-[10px] font-semibold uppercase tracking-wide flex-shrink-0">
            <Shield size={10} />
            Admin
          </span>
        )}
        {isSelf && <span className="text-[10px] text-muted flex-shrink-0">(tu)</span>}
      </div>

      {/* Toggle admin */}
      <div className="flex items-center gap-2 justify-start">
        <button
          type="button"
          onClick={onToggleAdmin}
          disabled={isSelf}
          aria-pressed={user.is_admin}
          aria-label={user.is_admin ? 'Rimuovi admin' : 'Rendi admin'}
          className={`relative inline-flex w-11 h-6 flex-shrink-0 rounded-full border transition-colors ${
            user.is_admin
              ? 'bg-amber-500/80 border-amber-500'
              : 'bg-white/[0.05] border-white/[0.1]'
          } ${isSelf ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer hover:brightness-110'}`}
        >
          <span
            className={`pointer-events-none absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform ${
              user.is_admin ? 'translate-x-5' : 'translate-x-0'
            }`}
          />
        </button>
      </div>

      {/* Stato */}
      <div className="flex items-center gap-2 text-xs text-muted">
        <span
          className={`w-2 h-2 rounded-full flex-shrink-0 ${stateColor}`}
          aria-label={`state ${user.state}`}
        />
        <span className="capitalize truncate">{user.state}</span>
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-2">
        <motion.button
          type="button"
          onClick={() => alert(`Richieste per ${user.username}: funzione non ancora implementata`)}
          className="w-8 h-8 rounded-md bg-blue-500/10 border border-blue-500/30 text-blue-400 flex items-center justify-center hover:bg-blue-500/20 transition-colors text-xs font-bold"
          whileTap={{ scale: 0.92 }}
          title="Richieste"
        >
          R
        </motion.button>
        <motion.button
          type="button"
          onClick={onDelete}
          disabled={isSelf}
          className={`w-8 h-8 rounded-md bg-danger/10 border border-danger/30 text-danger flex items-center justify-center hover:bg-danger/20 transition-colors ${
            isSelf ? 'opacity-40 cursor-not-allowed' : ''
          }`}
          whileTap={{ scale: 0.92 }}
          title="Elimina utente"
        >
          <Trash2 size={14} />
        </motion.button>
      </div>
    </motion.div>
  );
}
