import { motion } from 'framer-motion';
import { MessageSquare, Shield } from 'lucide-react';

interface Chat {
  id: number;
  name: string;
  lastMessage: string;
  time: string;
  unread: number;
}

interface ChatSidebarProps {
  chats: Chat[];
  activeChat: number | null;
  onSelectChat: (id: number) => void;
}

export default function ChatSidebar({ chats, activeChat, onSelectChat }: ChatSidebarProps) {
  return (
    <motion.div
      className="w-80 h-full glass-card flex flex-col border-r border-white/[0.06]"
      initial={{ opacity: 0, x: -20 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.4 }}
    >
      {/* Header */}
      <div className="p-5 border-b border-white/[0.06]">
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <MessageSquare size={20} className="text-neutral-400" />
          Messaggi
        </h2>
      </div>

      {/* Lista chat */}
      <div className="flex-1 overflow-y-auto">
        {chats.map((chat) => (
          <motion.button
            key={chat.id}
            onClick={() => onSelectChat(chat.id)}
            className={`w-full p-4 flex items-center gap-3 text-left transition-colors border-b border-white/[0.03] ${
              activeChat === chat.id
                ? 'bg-white/[0.05]'
                : 'hover:bg-white/[0.02]'
            }`}
            whileHover={{ x: 2 }}
            whileTap={{ scale: 0.99 }}
          >
            {/* Avatar */}
            <div className="w-12 h-12 rounded-full bg-accent/10 border border-white/10 flex items-center justify-center flex-shrink-0">
              <Shield size={20} className="text-neutral-300" />
            </div>

            {/* Info */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-0.5">
                <span className="font-medium text-sm truncate">{chat.name}</span>
                <span className="text-xs text-muted">{chat.time}</span>
              </div>
              <p className="text-xs text-muted truncate">{chat.lastMessage}</p>
            </div>

            {/* Unread badge */}
            {chat.unread > 0 && (
              <div className="w-5 h-5 rounded-full bg-white text-background text-xs font-bold flex items-center justify-center flex-shrink-0">
                {chat.unread}
              </div>
            )}
          </motion.button>
        ))}
      </div>
    </motion.div>
  );
}