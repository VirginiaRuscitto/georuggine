import { motion, type HTMLMotionProps } from 'framer-motion';
import { ReactNode } from 'react';

type Variant = 'default' | 'hover' | 'interactive' | 'subtle';

interface GlassCardProps {
  children: ReactNode;
  className?: string;
  delay?: number;
  /** default: card statica. hover: lift al passaggio. interactive: cursore + lift, per card cliccabili. subtle: minimal, per sidebar. */
  variant?: Variant;
  /** disabilita l'animazione di ingresso (es. dentro liste che hanno già la propria) */
  noEnter?: boolean;
}

const VARIANT_CLASS: Record<Variant, string> = {
  default: 'glass-card',
  hover: 'glass-card glass-card-hover',
  interactive: 'glass-card glass-card-interactive',
  subtle: 'glass-card glass-card-subtle',
};

export default function GlassCard({
  children,
  className = '',
  delay = 0,
  variant = 'default',
  noEnter = false,
}: GlassCardProps) {
  const motionProps: HTMLMotionProps<'div'> = noEnter
    ? {}
    : {
        initial: { opacity: 0, y: 24 },
        animate: { opacity: 1, y: 0 },
        transition: {
          duration: 0.6,
          delay,
          ease: [0.22, 1, 0.36, 1],
        },
      };

  return (
    <motion.div className={`${VARIANT_CLASS[variant]} ${className}`} {...motionProps}>
      {children}
    </motion.div>
  );
}