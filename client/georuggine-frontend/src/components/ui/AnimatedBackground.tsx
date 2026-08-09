import { motion } from 'framer-motion';

export default function AnimatedBackground() {
  return (
    <div className="fixed inset-0 overflow-hidden pointer-events-none bg-background">
      {/* Base: leggero gradiente verticale invece di nero piatto */}
      <div
        className="absolute inset-0"
        style={{
          background: 'linear-gradient(180deg, #0a0a0c 0%, #050505 45%, #030303 100%)',
        }}
      />

      {/* Lama di luce in alto a sinistra — nessun filter:blur, morbidezza data dal gradiente stesso */}
      <motion.div
        className="absolute -top-1/4 -left-1/4 w-[90vw] h-[70vh]"
        style={{
          background: 'radial-gradient(ellipse, rgba(180,190,255,0.09) 0%, rgba(180,190,255,0.03) 35%, transparent 65%)',
        }}
        animate={{
          x: [0, 60, -20, 0],
          y: [0, 40, -30, 0],
        }}
        transition={{ duration: 50, repeat: Infinity, ease: 'easeInOut' }}
      />

      {/* Lama di luce in basso a destra — tono caldo */}
      <motion.div
        className="absolute -bottom-1/4 -right-1/4 w-[80vw] h-[65vh]"
        style={{
          background: 'radial-gradient(ellipse, rgba(255,210,180,0.06) 0%, rgba(255,210,180,0.02) 35%, transparent 65%)',
        }}
        animate={{
          x: [0, -50, 20, 0],
          y: [0, -30, 40, 0],
        }}
        transition={{ duration: 60, repeat: Infinity, ease: 'easeInOut' }}
      />

      {/* Glow centrale statico, per dare profondità dietro al contenuto */}
      <div
        className="absolute inset-0"
        style={{
          background: 'radial-gradient(1000px 500px at 50% 20%, rgba(255,255,255,0.05), transparent 65%)',
        }}
      />

      {/* Vignette ai bordi */}
      <div
        className="absolute inset-0"
        style={{
          background: 'radial-gradient(120% 120% at 50% 50%, transparent 50%, rgba(0,0,0,0.45) 100%)',
        }}
      />

      {/* Griglia, leggermente più visibile */}
      <div
        className="absolute inset-0 opacity-[0.03]"
        style={{
          backgroundImage: `
            linear-gradient(rgba(255,255,255,0.15) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255,255,255,0.15) 1px, transparent 1px)
          `,
          backgroundSize: '120px 120px',
        }}
      />
    </div>
  );
}