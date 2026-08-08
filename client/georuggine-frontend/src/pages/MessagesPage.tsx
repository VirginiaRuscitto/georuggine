import { useState, useEffect, useCallback } from 'react';
import Navbar from '../components/layout/Navbar';
import ChatSidebar from '../components/messages/ChatSidebar';
import ChatWindow from '../components/messages/ChatWindow';
import { api } from '../lib/api';

interface Message {
  id: number;
  sender: 'me' | 'admin';
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

export default function MessagesPage() {
  const [activeChat, setActiveChat] = useState<number | null>(1);
  const [messages, setMessages] = useState<Message[]>([]);
  const [chats, setChats] = useState([
    {
      id: 1,
      name: 'Admin',
      lastMessage: 'Nessun messaggio',
      time: '',
      unread: 0,
    },
  ]);
  const [loading, setLoading] = useState(true);

  const fetchMessages = useCallback(async () => {
    try {
      
      const res = await api.get('/api/messages/conversation?with=1&limit=50');
      const apiMessages: ApiMessage[] = res.data;

      if (!apiMessages || apiMessages.length === 0) {
        setLoading(false);
        return;
      }

      const serverMessages: Message[] = apiMessages.map((msg) => {
        const sender: 'me' | 'admin' = msg.sender_id === null ? 'admin' : 'me';
        return {
          id: msg.id,
          sender,
          content: msg.content,
          timestamp: new Date(msg.sent_at).toLocaleTimeString('it-IT', {
            hour: '2-digit',
            minute: '2-digit',
          }),
        };
      }).reverse();
      setMessages(serverMessages);

      const last = serverMessages[serverMessages.length - 1];
      if (last) {
        setChats((prev) =>
          prev.map((c) =>
            c.id === 1
              ? { ...c, lastMessage: last.content, time: last.timestamp, unread: 0 }
              : c
          )
        );
      }
    } catch (e: any) {
      console.error('=== ERRORE FETCH ===');
      console.error('Status:', e.response?.status);
      console.error('Data:', e.response?.data);
      console.error('Message:', e.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchMessages();
    const interval = setInterval(() => {
      fetchMessages();
    }, 5000);
    return () => clearInterval(interval);
  }, [fetchMessages]);

  const handleSendMessage = async (content: string) => {
    
    const tempId = Date.now();
    const newMsg: Message = {
      id: tempId,
      sender: 'me',
      content,
      timestamp: new Date().toLocaleTimeString('it-IT', {
        hour: '2-digit',
        minute: '2-digit',
      }),
    };

    setMessages((prev) => [...prev, newMsg]);
    setChats((prev) =>
      prev.map((c) =>
        c.id === activeChat
          ? { ...c, lastMessage: content, time: newMsg.timestamp }
          : c
      )
    );

    try {
      fetchMessages();
    } catch (e: any) {
      console.error('=== ERRORE INVIO ===');
      console.error('Status:', e.response?.status);
      console.error('Data:', e.response?.data);
      console.error('Message:', e.message);
    }
  };

  return (
    <div className="min-h-screen bg-background">
      <Navbar activePage="messages" />

      <div className="pt-20 h-screen flex gap-6 px-6 pb-6">
        <ChatSidebar
          chats={chats}
          activeChat={activeChat}
          onSelectChat={setActiveChat}
        />
        <ChatWindow
          chatId={activeChat}
          messages={messages}
          onSendMessage={handleSendMessage}
        />
      </div>
    </div>
  );
}