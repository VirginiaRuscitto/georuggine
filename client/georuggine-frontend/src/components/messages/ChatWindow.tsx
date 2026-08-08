import { useState, useRef, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Send, Shield } from 'lucide-react';

interface Message {
  id: number;
  sender: 'me' | 'admin';
  content: string;
  timestamp: string;
}

interface ChatWindowProps {
  chatId: number | null;
  messages: Message[];
  onSendMessage: (content: string) => void;
}

export default function ChatWindow({ chatId, messages, onSendMessage }: ChatWindowProps) {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;
    onSendMessage(input.trim());
    setInput('');
  };

  if (!chatId) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted">
        <div className="text-center">
          <Shield size={48} className="mx-auto mb-4 opacity-20" />
          <p>Seleziona una conversazione</p>
        </div>
      </div>
    );
  }

  return (
    <motion.div
      className="flex-1 flex flex-col glass-card"
      initial={{ opacity: 0, x: 20 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.4 }}
    >
      {/* Header */}
      <div className="p-4 border-b border-white/[0.06] flex items-center gap-3">
        <div className="w-10 h-10 rounded-full bg-accent/10 border border-white/10 flex items-center justify-center">
          <Shield size={18} className="text-neutral-300" />
        </div>
        <div>
          <p className="font-medium text-sm">Admin</p>
          <p className="text-xs text-emerald-400">Online</p>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        <AnimatePresence>
          {messages.map((msg) => (
            <motion.div
              key={msg.id}
              className={`flex ${msg.sender === 'me' ? 'justify-end' : 'justify-start'}`}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
            >
              <div
                className={`max-w-[70%] px-4 py-2.5 rounded-2xl text-sm ${
                  msg.sender === 'me'
                    ? 'bg-white text-background rounded-br-md'
                    : 'bg-white/[0.05] text-foreground rounded-bl-md border border-white/[0.06]'
                }`}
              >
                <p>{msg.content}</p>
                <p className={`text-[10px] mt-1 ${msg.sender === 'me' ? 'text-neutral-500' : 'text-muted'}`}>
                  {msg.timestamp}
                </p>
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <form onSubmit={handleSubmit} className="p-4 border-t border-white/[0.06]">
        <div className="flex items-center gap-3">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Scrivi un messaggio..."
            className="flex-1 glass-input py-3 px-4 !pl-4"
          />
          <motion.button
            type="submit"
            disabled={!input.trim()}
            className="w-11 h-11 rounded-xl bg-white text-background flex items-center justify-center disabled:opacity-30 transition-opacity"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            <Send size={18} />
          </motion.button>
        </div>
      </form>
    </motion.div>
  );
}