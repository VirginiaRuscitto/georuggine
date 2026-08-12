import { useState, useEffect, useCallback, useRef } from 'react';
import { Megaphone } from 'lucide-react';
import Navbar from '../components/layout/Navbar';
import ChatSidebar from '../components/messages/ChatSidebar';
import ChatWindow from '../components/messages/ChatWindow';
import AnimatedBackground from '../components/ui/AnimatedBackground';
import { api } from '../lib/api';
import { useAuth } from '../context/AuthContext';
import { useMqttClient } from '../hooks/useMqttClient';

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

const BROADCAST_CHAT_ID = 0;
const ADMIN_CHAT_ID = 1;
// dopo l'invio, un refetch veloce a botta quasi sicura; se il messaggio non c'è ancora
// (broker/rete lenti), ci pensa comunque il prossimo poll periodico
const POST_SEND_REFETCH_MS = 200;

function formatTime(iso: string) {
  return new Date(iso).toLocaleTimeString('it-IT', { hour: '2-digit', minute: '2-digit' });
}

export default function MessagesPage() {
  const { userId, token } = useAuth();
  const { publish } = useMqttClient();
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

  const fetchMessagesRef = useRef<() => Promise<void>>(async () => {});

  const fetchMessages = useCallback(async () => {
    if (!userId) return;
    try {
      const res = await api.get('/api/messages?limit=50');
      const apiMessages: ApiMessage[] = res.data || [];

      const direct: Message[] = [];
      const broadcast: Message[] = [];

      for (const msg of apiMessages) {
        const isBroadcast = msg.sender_id === null && msg.recipient_id === null;
        const mapped: Message = {
          id: msg.id,
          sender: isBroadcast ? 'other' : msg.sender_id === userId ? 'me' : 'other',
          content: msg.content,
          timestamp: formatTime(msg.sent_at),
          sentAt: msg.sent_at,
        };
        (isBroadcast ? broadcast : direct).push(mapped);
      }

      setAdminMessages(direct);
      setBroadcastMessages(broadcast);

      setChats((prev) =>
        prev.map((c) => {
          const list = c.id === BROADCAST_CHAT_ID ? broadcast : direct;
          const last = list[list.length - 1];
          return last ? { ...c, lastMessage: last.content, time: last.timestamp } : c;
        })
      );
    } catch (e: any) {
      console.error('Errore fetch messaggi:', e.response?.status, e.response?.data);
    }
  }, [userId]);

  fetchMessagesRef.current = fetchMessages;

  useEffect(() => {
    fetchMessages();
    const interval = setInterval(fetchMessages, 5000);
    return () => clearInterval(interval);
  }, [fetchMessages]);

  const handleSendMessage = async (content: string) => {
    if (!userId || !token) {
      console.error('Impossibile inviare: utente o token mancante');
      return;
    }
    const ok = await publish(`georuggine/client/${userId}/message`, { token, content });
    if (!ok) {
      console.error('Invio messaggio MQTT fallito');
      return;
    }
    setTimeout(() => fetchMessagesRef.current(), POST_SEND_REFETCH_MS);
  };

  const isBroadcast = selectedChatId === BROADCAST_CHAT_ID;

  return (
    <div className="min-h-screen relative">
      <AnimatedBackground />
      <Navbar />
      <div className="relative z-10 pt-20 h-screen flex gap-6 px-6 pb-6">
        <ChatSidebar chats={chats} activeChat={selectedChatId} onSelectChat={setSelectedChatId} />
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
          <ChatWindow chatId={ADMIN_CHAT_ID} messages={adminMessages} onSendMessage={handleSendMessage} />
        )}
      </div>
    </div>
  );
}