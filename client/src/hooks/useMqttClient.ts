import { useEffect, useRef, useCallback } from 'react';
import mqtt from 'mqtt';

const MQTT_WS_URL = 'wss://broker.emqx.io:8084/mqtt';

export function useMqttClient() {
  const clientRef = useRef<ReturnType<typeof mqtt.connect> | null>(null);

  useEffect(() => {
    const client = mqtt.connect(MQTT_WS_URL, {
      clientId: `georuggine_web_${Math.random().toString(16).slice(2)}`,
      clean: true,
      reconnectPeriod: 2000,
    });

    client.on('connect', () => console.log('MQTT connesso'));
    client.on('error', (err) => console.error('Errore MQTT:', err));

    clientRef.current = client;

    return () => {
      client.end(true);
      clientRef.current = null;
    };
  }, []);

  const publish = useCallback((topic: string, payload: object) => {
    return new Promise<boolean>((resolve) => {
      const client = clientRef.current;
      if (!client || !client.connected) {
        console.error('MQTT non connesso, impossibile pubblicare');
        resolve(false);
        return;
      }
      client.publish(topic, JSON.stringify(payload), { qos: 1 }, (err) => {
        resolve(!err);
      });
    });
  }, []);

  return { publish };
}