import { createContext, useContext, useState, useEffect, ReactNode } from 'react';

interface AuthContextType {
  isAuthenticated: boolean;
  isAdmin: boolean;
  userId: number | null;
  token: string | null;
  login: (token: string) => boolean;
  logout: () => void;
  loading: boolean;
}

const AuthContext = createContext<AuthContextType | null>(null);

function readTokenPayload(token: string | null): { is_admin?: boolean; user_id?: number; id?: number; sub?: number } | null {
  if (!token) return null;
  try {
    return JSON.parse(atob(token.split('.')[1]));
  } catch {
    return null;
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isAdmin, setIsAdmin] = useState(false);
  const [userId, setUserId] = useState<number | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const stored = localStorage.getItem('token');
    if (stored) {
      const payload = readTokenPayload(stored);
      if (payload) {
        setIsAdmin(payload.is_admin || false);
        setUserId(payload.user_id ?? payload.id ?? payload.sub ?? null);
        setToken(stored);
        setIsAuthenticated(true);
      } else {
        localStorage.removeItem('token');
      }
    }
    setLoading(false);
  }, []);

  const login = (newToken: string): boolean => {
    localStorage.setItem('token', newToken);
    const payload = readTokenPayload(newToken);
    const admin = payload?.is_admin || false;
    setIsAdmin(admin);
    setUserId(payload?.user_id ?? payload?.id ?? payload?.sub ?? null);
    setToken(newToken);
    setIsAuthenticated(true);
    return admin;
  };

  const logout = () => {
    localStorage.removeItem('token');
    setIsAuthenticated(false);
    setIsAdmin(false);
    setUserId(null);
    setToken(null);
  };

  return (
    <AuthContext.Provider value={{ isAuthenticated, isAdmin, userId, token, login, logout, loading }}>
      {children}
    </AuthContext.Provider>
  );
}

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) throw new Error('useAuth must be used within AuthProvider');
  return context;
};