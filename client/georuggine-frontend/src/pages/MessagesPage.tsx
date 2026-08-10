import { useState, useEffect, useCallback } from 'react';
import { Megaphone } from 'lucide-react';
import Navbar from '../components/layout/Navbar';
import ChatSidebar from '../components/messages/ChatSidebar';
import ChatWindow from '../components/messages/ChatWindow';
import AnimatedBackground from '../components/ui/AnimatedBackground';
import { api } from '../lib/api';
import { useAuth } from '../context/AuthContext';

interface Message {
  id: number;
  sender: 'me' | 'other';
  content: string;
  timestamp: string;
}

interface ApiMessage {
  id: number;
  sender_id: number | null;
  recipient_id: number | null;
  content: string;
  sent_at: string;
}

const BROADCAST_CHAT_ID = 0;
const ADMIN_CHAT_ID = 1;

export default function MessagesPage() {
  const { userId } = useAuth();
  const [adminId, setAdminId] = useState<number | null>(null);
  const [selectedChatId, setSelectedChatId] = useState<number>(ADMIN_CHAT_ID);
  const [adminMessages, setAdminMessages] = useState<Message[]>([]);
  const [broadcastMessages, setBroadcastMessages] = useState<Message[]>([]);
  const [chats, setChats] = useState([
    {
      id: BROADCAST_CHAT_ID,
      name: 'Broadcast',
      lastMessage: 'Nessun messaggio',
      time: '',
      unread: 0,
      icon: <Megaphone size={20} className="text-neutral-300" />,
    },
    {
      id: ADMIN_CHAT_ID,
      name: 'Admin',
      lastMessage: 'Nessun messaggio',
      time: '',
      unread: 0,
    },
  ]);

  // PASSO 1: Trova l'ID dell'admin
  useEffect(() => {
    const findAdmin = async () => {
      try {
        const res = await api.get('/api/users');
        const admin = (res.data || []).find((u: any) => u.is_admin === true);
        if (admin) {
          setAdminId(admin.id);
        } else {
          console.error('Nessun admin trovato!');
        }
      } catch (e) {
        console.error('Errore ricerca admin:', e);
      }
    };
    findAdmin();
  }, []);

  // Carica la conversazione con l'admin
  const fetchAdminConversation = useCallback(async () => {
    if (!userId || !adminId) return;
    try {
      const res = await api.get(`/api/messages?with=${adminId}&limit=50`);
      const apiMessages: ApiMessage[] = res.data || [];

      const serverMessages: Message[] = apiMessages.map((msg) => {
        const isMe = msg.sender_id === userId;
        return {
          id: msg.id,
          sender: isMe ? 'me' : 'other',
          content: msg.content,
          timestamp: new Date(msg.sent_at).toLocaleTimeString('it-IT', {
            hour: '2-digit',
            minute: '2-digit',
          }),
        };
      }).reverse();

      setAdminMessages(serverMessages);

      const last = serverMessages[serverMessages.length - 1];
      if (last) {
        setChats((prev) =>
          prev.map((c) =>
            c.id === ADMIN_CHAT_ID
              ? { ...c, lastMessage: last.content, time: last.timestamp }
              : c
          )
        );
      }
    } catch (e: any) {
      console.error('Errore fetch admin conversation:', e.response?.status, e.response?.data);
    }
  }, [userId, adminId]);

  // Carica i messaggi broadcast
  const fetchBroadcasts = useCallback(async () => {
    try {
      const res = await api.get('/api/messages?limit=50');
      const apiMessages: ApiMessage[] = (res.data || []).filter(
        (m) => m.sender_id === null && m.recipient_id === null
      );

      const serverMessages: Message[] = apiMessages.map((msg) => ({
        id: msg.id,
        sender: 'other' as const,
        content: msg.content,
        timestamp: new Date(msg.sent_at).toLocaleTimeString('it-IT', {
          hour: '2-digit',
          minute: '2-digit',
        }),
      })).reverse();

      setBroadcastMessages(serverMessages);

      const last = serverMessages[serverMessages.length - 1];
      if (last) {
        setChats((prev) =>
          prev.map((c) =>
            c.id === BROADCAST_CHAT_ID
              ? { ...c, lastMessage: last.content, time: last.timestamp }
              : c
          )
        );
      }
    } catch (e: any) {
      console.error('Errore fetch broadcasts:', e.response?.status, e.response?.data);
    }
  }, []);

  useEffect(() => {
    fetchAdminConversation();
    fetchBroadcasts();
    const interval = setInterval(() => {
      fetchAdminConversation();
      fetchBroadcasts();
    }, 5000);
    return () => clearInterval(interval);
  }, [fetchAdminConversation, fetchBroadcasts]);

  const handleSendMessage = async (content: string) => {
    if (!userId || !adminId) return;
    try {
      await api.post('/api/messages/direct', {
        recipient_id: adminId,
        content,
      });
      await fetchAdminConversation();
    } catch (e: any) {
      console.error('Errore invio:', e.response?.status, e.response?.data);
    }
  };

  if (!adminId) {
    return (
      <div className="min-h-screen relative flex items-center justify-center">
        <AnimatedBackground />
        <div className="relative z-10 text-muted">Connessione alla chat...</div>
      </div>
    );
  }

  const isBroadcast = selectedChatId === BROADCAST_CHAT_ID;

  return (
    <div className="min-h-screen relative">
      <AnimatedBackground />
      <Navbar />
      <div className="relative z-10 pt-20 h-screen flex gap-6 px-6 pb-6">
        <ChatSidebar
          chats={chats}
          activeChat={selectedChatId}
          onSelectChat={setSelectedChatId}
        />
        {isBroadcast ? (
          <ChatWindow
            chatId={BROADCAST_CHAT_ID}
            messages={broadcastMessages}
            onSendMessage={() => {}}
            title="Broadcast"
            icon={<Megaphone size={18} className="text-neutral-300" />}
            status="Canale ufficiale"
            readOnly
          />
        ) : (
          <ChatWindow
            chatId={ADMIN_CHAT_ID}
            messages={adminMessages}
            onSendMessage={handleSendMessage}
          />
        )}
      </div>
    </div>
  );
}