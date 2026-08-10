import { motion } from 'framer-motion';

export default function DiagonalDivider() {
  return (
    <motion.div 
      className="relative flex flex-col items-center justify-center h-80 w-20"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.8, delay: 0.3 }}
    >
      {/* Linea diagonale superiore */}
      <motion.div
        className="w-px h-28 bg-gradient-to-b from-transparent to-white/20"
        style={{ transform: 'rotate(20deg)' }}
        initial={{ scaleY: 0 }}
        animate={{ scaleY: 1 }}
        transition={{ duration: 0.6, delay: 0.5 }}
      />
      
      {/* Testo "oppure" */}
      <motion.span
        className="my-4 text-[10px] text-neutral-500 uppercase tracking-[0.2em] font-medium"
        initial={{ opacity: 0, scale: 0.8 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.5, delay: 0.9 }}
      >
        oppure
      </motion.span>
      
      {/* Linea diagonale inferiore */}
      <motion.div
        className="w-px h-28 bg-gradient-to-t from-transparent to-white/20"
        style={{ transform: 'rotate(20deg)' }}
        initial={{ scaleY: 0 }}
        animate={{ scaleY: 1 }}
        transition={{ duration: 0.6, delay: 0.7 }}
      />
    </motion.div>
  );
}