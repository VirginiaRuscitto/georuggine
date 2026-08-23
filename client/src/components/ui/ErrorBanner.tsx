import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { AlertCircle, X } from 'lucide-react';

interface ErrorBannerProps {
  message: string;
  onClose: () => void;
}

export default function ErrorBanner({ message, onClose }: ErrorBannerProps) {
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