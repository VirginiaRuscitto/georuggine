import { createContext, useContext, useState, useEffect, ReactNode } from 'react';

interface AuthContextType {
  isAuthenticated: boolean;
  isAdmin: boolean;
  userId: number | null;
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
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const token = localStorage.getItem('token');
    if (token) {
      const payload = readTokenPayload(token);
      if (payload) {
        setIsAdmin(payload.is_admin || false);
        setUserId(payload.user_id ?? payload.id ?? payload.sub ?? null);
        setIsAuthenticated(true);
      } else {
        localStorage.removeItem('token');
      }
    }
    setLoading(false);
  }, []);

  const login = (token: string): boolean => {
    localStorage.setItem('token', token);
    const payload = readTokenPayload(token);
    const admin = payload?.is_admin || false;
    setIsAdmin(admin);
    setUserId(payload?.user_id ?? payload?.id ?? payload?.sub ?? null);
    setIsAuthenticated(true);
    return admin;
  };

  const logout = () => {
    localStorage.removeItem('token');
    setIsAuthenticated(false);
    setIsAdmin(false);
    setUserId(null);
  };

  return (
    <AuthContext.Provider value={{ isAuthenticated, isAdmin, userId, login, logout, loading }}>
      {children}
    </AuthContext.Provider>
  );
}

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) throw new Error('useAuth must be used within AuthProvider');
  return context;
};
