import { useState, useRef, useEffect, ReactNode } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Send, Shield } from 'lucide-react';
import GlassCard from '../ui/GlassCard';

interface Message {
  id: number;
  sender: 'me' | 'other';
  content: string;
  timestamp: string; // "HH:MM", per la bolla
  sentAt: string;     // ISO completo, per raggruppare per giorno
}

interface ChatWindowProps {
  chatId: number | null;
  messages: Message[];
  onSendMessage: (content: string) => void;
  title?: string;
  icon?: ReactNode;
  status?: string;
  readOnly?: boolean;
}

function sameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function dayLabel(iso: string): string {
  const d = new Date(iso);
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);

  if (sameDay(d, today)) return 'Oggi';
  if (sameDay(d, yesterday)) return 'Ieri';

  return d.toLocaleDateString('it-IT', {
    day: 'numeric',
    month: 'long',
    year: d.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
  });
}

type ListItem =
  | { type: 'separator'; label: string; key: string }
  | { type: 'message'; msg: Message };

function buildListItems(messages: Message[]): ListItem[] {
  const items: ListItem[] = [];
  let lastLabel: string | null = null;

  for (const msg of messages) {
    const label = dayLabel(msg.sentAt);
    if (label !== lastLabel) {
      items.push({ type: 'separator', label, key: `sep-${label}` });
      lastLabel = label;
    }
    items.push({ type: 'message', msg });
  }

  return items;
}

export default function ChatWindow({
  chatId,
  messages,
  onSendMessage,
  title = 'Admin',
  icon,
  status = 'Online',
  readOnly = false,
}: ChatWindowProps) {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

    const [userScrolled, setUserScrolled] = useState(false);
  const messagesContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!userScrolled) {
      scrollToBottom();
    }
  }, [messages, userScrolled]);

  const handleScroll = () => {
    const container = messagesContainerRef.current;
    if (!container) return;
    const isNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 50;
    setUserScrolled(!isNearBottom);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;
    onSendMessage(input.trim());
    setInput('');
  };

  if (chatId === null) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted">
        <div className="text-center">
          {icon || <Shield size={48} className="mx-auto mb-4 opacity-20" />}
          <p>Seleziona una conversazione</p>
        </div>
      </div>
    );
  }

  const items = buildListItems(messages);

  return (
    <GlassCard
      variant="subtle"
      noEnter
      className="flex-1 flex flex-col !p-0 overflow-hidden"
    >
      <motion.div
        initial={{ opacity: 0, x: 16 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 0.4 }}
        className="flex flex-col h-full"
      >
        {/* Header */}
        <div className="p-4 border-b border-white/[0.06] flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-accent/10 border border-white/10 flex items-center justify-center">
            {icon || <Shield size={18} className="text-neutral-300" />}
          </div>
          <div>
            <p className="font-medium text-sm">{title}</p>
            <p className="text-xs text-emerald-400">{status}</p>
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3" onScroll={handleScroll} ref={messagesContainerRef}>
          <AnimatePresence initial={false}>
            {items.map((item) =>
              item.type === 'separator' ? (
                <div key={item.key} className="flex justify-center py-2">
                  <span className="text-[11px] uppercase tracking-wide text-muted bg-white/[0.04] border border-white/[0.06] rounded-full px-3 py-1">
                    {item.label}
                  </span>
                </div>
              ) : (
                <motion.div
                  key={item.msg.id}
                  className={`flex ${item.msg.sender === 'me' ? 'justify-end' : 'justify-start'}`}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0 }}
                >
                  <div
                    className={`max-w-[70%] px-4 py-2.5 rounded-2xl text-sm ${
                      item.msg.sender === 'me'
                        ? 'bg-white text-background rounded-br-md'
                        : 'bg-white/[0.05] text-foreground rounded-bl-md border border-white/[0.06]'
                    }`}
                  >
                    <p>{item.msg.content}</p>
                    <p
                      className={`text-[10px] mt-1 ${
                        item.msg.sender === 'me' ? 'text-neutral-500' : 'text-muted'
                      }`}
                    >
                      {item.msg.timestamp}
                    </p>
                  </div>
                </motion.div>
              )
            )}
          </AnimatePresence>
          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        {readOnly ? (
          <div className="p-4 border-t border-white/[0.06] text-center text-xs text-muted">
            Chat in sola lettura
          </div>
        ) : (
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
        )}
      </motion.div>
    </GlassCard>
  );
}