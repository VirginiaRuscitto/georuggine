import { useState, useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider, useAuth } from './context/AuthContext';
import AuthPage from './pages/AuthPage';
import DashboardPage from './pages/DashboardPage';
import MessagesPage from './pages/MessagesPage';
import AdminDashboardPage from './pages/admin/AdminDashboardPage';
import AdminUsersPage from './pages/admin/AdminUsersPage';
import AdminMessagesPage from './pages/admin/AdminMessagesPage';
import { motion, AnimatePresence } from 'framer-motion';
import { AlertCircle, X } from 'lucide-react';

function ErrorBanner({ message, onClose }: { message: string; onClose: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onClose, 5000);
    return () => clearTimeout(timer);
  }, [onClose]);

  return (
    <motion.div
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      className="fixed top-20 left-1/2 -translate-x-1/2 z-[100] bg-danger/90 backdrop-blur-sm text-white px-4 py-3 rounded-xl shadow-lg flex items-center gap-3 max-w-md"
    >
      <AlertCircle size={18} />
      <p className="text-sm flex-1">{message}</p>
      <button onClick={onClose} className="hover:opacity-70">
        <X size={16} />
      </button>
    </motion.div>
  );
}

function ProtectedRoute({ children, adminOnly = false }: { children: React.ReactNode; adminOnly?: boolean }) {
  const { isAuthenticated, isAdmin, loading } = useAuth();

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-background/80">
        <div className="w-8 h-8 border-2 border-white/20 border-t-white rounded-full animate-spin" />
      </div>
    );
  }

  if (!isAuthenticated) return <Navigate to="/login" replace />;
  if (adminOnly && !isAdmin) return <Navigate to="/" replace />;

  return <>{children}</>;
}

function App() {
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const handler = (e: Event) => {
      const msg = (e as CustomEvent).detail;
      setError(msg);
    };
    window.addEventListener('app-error', handler);
    return () => window.removeEventListener('app-error', handler);
  }, []);

  return (
    <AuthProvider>
      <BrowserRouter>
        <AnimatePresence>
          {error && <ErrorBanner message={error} onClose={() => setError(null)} />}
        </AnimatePresence>
        <Routes>
          <Route path="/login" element={<AuthPage />} />

          {/* User Routes */}
          <Route path="/" element={<ProtectedRoute><DashboardPage /></ProtectedRoute>} />
          <Route path="/messages" element={<ProtectedRoute><MessagesPage /></ProtectedRoute>} />

          {/* Admin Routes */}
          <Route path="/admin" element={<ProtectedRoute adminOnly><AdminDashboardPage /></ProtectedRoute>} />
          <Route path="/admin/users" element={<ProtectedRoute adminOnly><AdminUsersPage /></ProtectedRoute>} />
          <Route path="/admin/reports" element={
            <ProtectedRoute adminOnly>
              <div className="pt-20 text-center">Reports (prossimamente)</div>
            </ProtectedRoute>
          } />
          <Route path="/admin/messages" element={<ProtectedRoute adminOnly><AdminMessagesPage /></ProtectedRoute>} />
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  );
}

export default App;