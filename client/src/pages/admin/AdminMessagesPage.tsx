import { useState, useEffect, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Megaphone, Search, User } from 'lucide-react';
import Navbar from '../../components/layout/Navbar';
import ChatWindow from '../../components/messages/ChatWindow';
import AnimatedBackground from '../../components/ui/AnimatedBackground';
import GlassCard from '../../components/ui/GlassCard';
import { api } from '../../lib/api';
import { useAuth } from '../../context/AuthContext';

interface Message {
  id: number;
  sender: 'me' | 'other';
  content: string;
  timestamp: string;
  sentAt: string;
}

interface ApiMessage {
  id: number;
  sender_id: number | null;
  recipient_id: number | null;
  content: string;
  sent_at: string;
}

interface UserItem {
  id: number;
  name: string;
  surname: string;
  email: string;
  state: string;
  is_admin: boolean;
}

interface UsersResponse {
  users: UserItem[];
  tot_pages: number;
}

interface LastMessageInfo {
  content: string;
  sentAt: string;
}

const BROADCAST_ID = 0;

function getDisplayName(user: UserItem): string {
  const fullName = `${user.name ?? ''} ${user.surname ?? ''}`.trim();
  return fullName || user.email;
}

function getStateColorClass(state: string): string {
  const s = (state ?? '').toLowerCase();
  if (s === 'moving' || s === 'inmovimento' || s === 'in_movimento') return 'bg-emerald-400';
  if (s === 'stopped' || s === 'fermo') return 'bg-amber-400';
  return 'bg-neutral-600';
}

export default function AdminMessagesPage() {
  const { userId } = useAuth();
  const [users, setUsers] = useState<UserItem[]>([]);
  const [selectedUserId, setSelectedUserId] = useState<number | null>(null);
  const [broadcastSelected, setBroadcastSelected] = useState(false);
  const [messages, setMessages] = useState<Message[]>([]);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);
  const [lastMessageByUser, setLastMessageByUser] = useState<Record<number, LastMessageInfo>>({});

  // Fetch utenti al mount
  useEffect(() => {
    const fetchUsers = async () => {
      try {
        const res = await api.get<UsersResponse>('/api/users', {
          params: {
            is_admin: false,
            limit: 100,
            offset: 0,
          },
        });
        setUsers(res.data.users || []);
      } catch (e: any) {
        console.error('Errore fetch users:', e.response?.status, e.response?.data);
        setUsers([]);
      } finally {
        setLoading(false);
      }
    };

    fetchUsers();
  }, [userId]);

  // Fetch conversazione 1:1
  const fetchConversation = useCallback(async () => {
    if (!selectedUserId || !userId) return;
    try {
      const res = await api.get(`/api/messages?with=${selectedUserId}&limit=50`);
      const apiMessages: ApiMessage[] = res.data || [];

      const formatted: Message[] = apiMessages.map((msg) => {
        const sender: 'me' | 'other' = msg.sender_id === null ? 'me' : 'other';
        return {
          id: msg.id,
          sender,
          content: msg.content,
          timestamp: new Date(msg.sent_at).toLocaleTimeString('it-IT', {
            hour: '2-digit',
            minute: '2-digit',
          }),
          sentAt: msg.sent_at,
        };
      });

      setMessages(formatted);
    } catch (e: any) {
      console.error('Errore fetch messaggi:', e.response?.status, e.response?.data);
    }
  }, [selectedUserId, userId]);

  // Ultimo messaggio per utente (per ordinare la sidebar)
  const fetchLastMessages = useCallback(async () => {
    try {
      const res = await api.get('/api/messages?limit=200&all_direct=true');
      const apiMessages: ApiMessage[] = res.data || [];

      const map: Record<number, LastMessageInfo> = {};
      for (const msg of apiMessages) {
        const isBroadcast = msg.sender_id === null && msg.recipient_id === null;
        if (isBroadcast) continue;

        const otherUserId = msg.sender_id !== null ? msg.sender_id : msg.recipient_id;
        if (otherUserId === null || otherUserId === undefined) continue;

        const existing = map[otherUserId];
        if (!existing || new Date(msg.sent_at).getTime() > new Date(existing.sentAt).getTime()) {
          map[otherUserId] = { content: msg.content, sentAt: msg.sent_at };
        }
      }

      setLastMessageByUser(map);
    } catch (e: any) {
      console.error('Errore fetch ultimi messaggi:', e.response?.status, e.response?.data);
    }
  }, []);

  useEffect(() => {
    fetchLastMessages();
    const interval = setInterval(fetchLastMessages, 5000);
    return () => clearInterval(interval);
  }, [fetchLastMessages]);

  // Fetch broadcast
  const fetchBroadcasts = useCallback(async () => {
    try {
      const res = await api.get('/api/messages?limit=50');
      const apiMessages: ApiMessage[] = (res.data || []).filter(
        (m: ApiMessage) => m.sender_id === null && m.recipient_id === null
      );

      const formatted: Message[] = apiMessages.map((msg) => ({
        id: msg.id,
        sender: 'other' as const,
        content: msg.content,
        timestamp: new Date(msg.sent_at).toLocaleTimeString('it-IT', {
          hour: '2-digit',
          minute: '2-digit',
        }),
        sentAt: msg.sent_at,
      }));

      setMessages(formatted);
    } catch (e: any) {
      console.error('Errore fetch broadcasts:', e.response?.status, e.response?.data);
    }
  }, []);

  useEffect(() => {
    if (selectedUserId) {
      fetchConversation();
      const interval = setInterval(fetchConversation, 3000);
      return () => clearInterval(interval);
    } else if (broadcastSelected) {
      fetchBroadcasts();
      const interval = setInterval(fetchBroadcasts, 3000);
      return () => clearInterval(interval);
    }
  }, [fetchConversation, fetchBroadcasts, selectedUserId, broadcastSelected]);

  const handleSendDirect = async (content: string) => {
    if (!selectedUserId || !userId) return;
    try {
      await api.post('/api/messages/direct', {
        recipient_id: selectedUserId,
        content,
      });
      await fetchConversation();
      await fetchLastMessages(); // <-- AGGIUNTO: aggiorna subito l'ordine in sidebar
    } catch (e: any) {
      console.error('Errore invio diretto:', e.response?.status, e.response?.data);
    }
  };

  const handleSendBroadcast = async (content: string) => {
    try {
      await api.post('/api/broadcast', { content });
      await fetchBroadcasts();
      await fetchLastMessages(); // <-- AGGIUNTO: coerenza con la sidebar
    } catch (e: any) {
      console.error('Errore invio broadcast:', e.response?.status, e.response?.data);
    }
  };

  const handleSelectUser = (id: number) => {
    setSelectedUserId(id);
    setBroadcastSelected(false);
  };

  const handleSelectBroadcast = () => {
    setSelectedUserId(null);
    setBroadcastSelected(true);
  };

  const filteredUsers = users
    .filter((u) => getDisplayName(u).toLowerCase().includes(search.toLowerCase()))
    .sort((a, b) => {
      const aTime = lastMessageByUser[a.id]?.sentAt;
      const bTime = lastMessageByUser[b.id]?.sentAt;

      // Entrambi hanno scritto: più recente primo
      if (aTime && bTime) return new Date(bTime).getTime() - new Date(aTime).getTime();
      // Solo a ha scritto: a prima
      if (aTime) return -1;
      // Solo b ha scritto: b prima
      if (bTime) return 1;
      // Nessuno ha scritto: ordine alfabetico
      return getDisplayName(a).localeCompare(getDisplayName(b));
    });

  const selectedUser = users.find((u) => u.id === selectedUserId);

  return (
    <div className="min-h-screen relative">
      <AnimatedBackground />
      <Navbar />
      <div className="relative z-10 pt-20 h-screen flex gap-6 px-6 pb-6">
        {/* Sidebar utenti */}
        <GlassCard variant="subtle" noEnter className="w-80 h-full flex flex-col !p-0 overflow-hidden">
          <div className="p-5 border-b border-white/[0.06]">
            <h2 className="text-lg font-semibold mb-3">Conversazioni</h2>
            <div className="relative">
              <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
              <input
                type="text"
                placeholder="Cerca utente..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="glass-input w-full py-2.5 pl-10 pr-4 !text-sm"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto">
            {/* Riga Broadcast in cima */}
            <motion.button
              onClick={handleSelectBroadcast}
              className={`w-full p-4 flex items-center gap-3 text-left transition-colors border-b border-white/[0.03] ${
                broadcastSelected ? 'bg-white/[0.05]' : 'hover:bg-white/[0.02]'
              }`}
              whileTap={{ scale: 0.99 }}
            >
              <div className="w-10 h-10 rounded-full bg-white/5 border border-white/10 flex items-center justify-center flex-shrink-0">
                <Megaphone size={18} className="text-neutral-400" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">Broadcast</p>
                <p className="text-xs text-muted truncate">Tutti gli utenti</p>
              </div>
            </motion.button>

            {loading ? (
              <div className="p-8 text-center text-sm text-muted">Caricamento...</div>
            ) : filteredUsers.length === 0 ? (
              <div className="p-8 text-center text-sm text-muted">
                {users.length === 0 ? 'Nessun utente disponibile' : 'Nessun risultato'}
              </div>
            ) : (
              filteredUsers.map((user) => (
                <motion.button
                  key={user.id}
                  onClick={() => handleSelectUser(user.id)}
                  className={`w-full p-4 flex items-center gap-3 text-left transition-colors border-b border-white/[0.03] ${
                    selectedUserId === user.id ? 'bg-white/[0.05]' : 'hover:bg-white/[0.02]'
                  }`}
                  whileTap={{ scale: 0.99 }}
                >
                  <div className="w-10 h-10 rounded-full bg-white/5 border border-white/10 flex items-center justify-center flex-shrink-0">
                    <User size={18} className="text-neutral-400" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{getDisplayName(user)}</p>
                    <p className="text-xs text-muted truncate">
                      {lastMessageByUser[user.id]?.content ?? `ID: ${user.id}`}
                    </p>
                  </div>
                  <div className="flex flex-col items-end gap-1 flex-shrink-0">
                    {lastMessageByUser[user.id] && (
                      <span className="text-[10px] text-muted">
                        {new Date(lastMessageByUser[user.id].sentAt).toLocaleTimeString('it-IT', {
                          hour: '2-digit',
                          minute: '2-digit',
                        })}
                      </span>
                    )}
                    <div className={`w-2 h-2 rounded-full ${getStateColorClass(user.state)}`} />
                  </div>
                </motion.button>
              ))
            )}
          </div>
        </GlassCard>

        {broadcastSelected ? (
          <ChatWindow
            chatId={BROADCAST_ID}
            messages={messages}
            onSendMessage={handleSendBroadcast}
            title="Broadcast"
            icon={<Megaphone size={18} className="text-neutral-300" />}
            status="Canale ufficiale"
          />
        ) : (
          <ChatWindow
            chatId={selectedUserId}
            messages={messages}
            onSendMessage={handleSendDirect}
            title={selectedUser ? getDisplayName(selectedUser) : 'Seleziona un utente'}
            icon={<User size={18} className="text-neutral-300" />}
            status={selectedUser?.state || 'Offline'}
          />
        )}
      </div>
    </div>
  );
}