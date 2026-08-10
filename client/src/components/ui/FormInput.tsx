import { motion } from 'framer-motion';
import { ReactNode } from 'react';

interface FormInputProps {
  type?: string;
  placeholder: string;
  icon: ReactNode;
  value: string;
  onChange: (value: string) => void;
  delay?: number;
}

export default function FormInput({ 
  type = 'text', 
  placeholder, 
  icon, 
  value, 
  onChange,
  delay = 0 
}: FormInputProps) {
  return (
    <motion.div
      className="relative"
      initial={{ opacity: 0, x: -20 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.5, delay }}
    >
      <div className="absolute left-4 top-1/2 -translate-y-1/2 text-muted">
        {icon}
      </div>
      <input
        type={type}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="glass-input w-full py-3.5 pl-12 pr-4 text-sm outline-none"
      />
    </motion.div>
  );
}