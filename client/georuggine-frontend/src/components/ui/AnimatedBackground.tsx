import { motion, useMotionValue, useSpring } from 'framer-motion';
import { useEffect, useMemo } from 'react';

interface FloatingCube {
  id: number;
  size: number;
  startX: number;
  startY: number;
  duration: number;
  delay: number;
  blur: number;
  opacity: number;
}

function generateCubes(count: number): FloatingCube[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i,
    size: Math.random() * 100 + 30,
    startX: Math.random() * window.innerWidth,
    startY: Math.random() * window.innerHeight,
    duration: Math.random() * 25 + 20,
    delay: Math.random() * 15,
    blur: Math.random() * 4 + 2,
    opacity: Math.random() * 0.03 + 0.01,
  }));
}

function Cube({ cube }: { cube: FloatingCube }) {
  return (
    <motion.div
      className="absolute pointer-events-none"
      style={{
        width: cube.size,
        height: cube.size,
        left: cube.startX,
        top: cube.startY,
        filter: `blur(${cube.blur}px)`,
        opacity: cube.opacity,
        marginLeft: -cube.size / 2,
        marginTop: -cube.size / 2,
      }}
      animate={{
        x: [
          0,
          (Math.random() - 0.5) * 300,
          (Math.random() - 0.5) * 300,
          (Math.random() - 0.5) * 200,
          0,
        ],
        y: [
          0,
          (Math.random() - 0.5) * 200,
          (Math.random() - 0.5) * 300,
          (Math.random() - 0.5) * 200,
          0,
        ],
        rotateX: [0, 180, 360, 540, 720],
        rotateY: [0, 90, 270, 450, 720],
        rotateZ: [0, 45, 135, 225, 360],
      }}
      transition={{
        duration: cube.duration,
        delay: cube.delay,
        repeat: Infinity,
        ease: 'linear',
      }}
    >
      {/* Faccia frontale */}
      <div
        className="absolute inset-0 border border-white/[0.08]"
        style={{
          background: 'linear-gradient(135deg, rgba(255,255,255,0.06) 0%, rgba(255,255,255,0.01) 100%)',
          transform: `translateZ(${cube.size * 0.3}px)`,
        }}
      />
      {/* Faccia posteriore */}
      <div
        className="absolute inset-0 border border-white/[0.08]"
        style={{
          background: 'linear-gradient(135deg, rgba(255,255,255,0.04) 0%, rgba(255,255,255,0.005) 100%)',
          transform: `translateZ(-${cube.size * 0.3}px) rotateY(180deg)`,
        }}
      />
      {/* Faccia destra */}
      <div
        className="absolute inset-0 border border-white/[0.08]"
        style={{
          background: 'linear-gradient(135deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.01) 100%)',
          transform: `rotateY(90deg) translateZ(${cube.size * 0.3}px)`,
        }}
      />
      {/* Faccia sinistra */}
      <div
        className="absolute inset-0 border border-white/[0.08]"
        style={{
          background: 'linear-gradient(135deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.01) 100%)',
          transform: `rotateY(-90deg) translateZ(${cube.size * 0.3}px)`,
        }}
      />
      {/* Faccia superiore */}
      <div
        className="absolute inset-0 border border-white/[0.08]"
        style={{
          background: 'linear-gradient(135deg, rgba(255,255,255,0.07) 0%, rgba(255,255,255,0.02) 100%)',
          transform: `rotateX(90deg) translateZ(${cube.size * 0.3}px)`,
        }}
      />
      {/* Faccia inferiore */}
      <div
        className="absolute inset-0 border border-white/[0.08]"
        style={{
          background: 'linear-gradient(135deg, rgba(255,255,255,0.03) 0%, rgba(255,255,255,0.005) 100%)',
          transform: `rotateX(-90deg) translateZ(${cube.size * 0.3}px)`,
        }}
      />
    </motion.div>
  );
}

export default function AnimatedBackground() {
  const mouseX = useMotionValue(0);
  const mouseY = useMotionValue(0);

  const springX = useSpring(mouseX, { stiffness: 50, damping: 30 });
  const springY = useSpring(mouseY, { stiffness: 50, damping: 30 });

  const cubes = useMemo(() => generateCubes(20), []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      const x = (e.clientX / window.innerWidth - 0.5) * 100;
      const y = (e.clientY / window.innerHeight - 0.5) * 100;
      mouseX.set(x);
      mouseY.set(y);
    };

    window.addEventListener('mousemove', handleMouseMove);
    return () => window.removeEventListener('mousemove', handleMouseMove);
  }, [mouseX, mouseY]);

  return (
    <div className="fixed inset-0 overflow-hidden pointer-events-none">
      {/* Orb che segue il mouse */}
      <motion.div
        className="absolute w-[500px] h-[500px] rounded-full"
        style={{
          background: 'radial-gradient(circle, rgba(255,255,255,0.05) 0%, transparent 70%)',
          filter: 'blur(100px)',
          x: springX,
          y: springY,
          top: '30%',
          left: '40%',
        }}
      />

      {/* Cubi 3D fluttuanti distribuiti ovunque */}
      {cubes.map((cube) => (
        <Cube key={cube.id} cube={cube} />
      ))}

      {/* Griglia sottile */}
      <div
        className="absolute inset-0 opacity-[0.012]"
        style={{
          backgroundImage: `
            linear-gradient(rgba(255,255,255,0.1) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255,255,255,0.1) 1px, transparent 1px)
          `,
          backgroundSize: '120px 120px',
        }}
      />
    </div>
  );
}