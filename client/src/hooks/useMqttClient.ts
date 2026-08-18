import { useEffect, useRef, useCallback, useState } from 'react';
import mqtt from 'mqtt/dist/mqtt.esm';

const MQTT_WS_URL = 'wss://broker.emqx.io:8084/mqtt';

export function useMqttClient() {
  const clientRef = useRef<mqtt.MqttClient | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const clientId = `georuggine_web_${Math.random().toString(16).slice(2, 8)}`;

    const client = mqtt.connect(MQTT_WS_URL, {
      clientId,
      clean: true,
      reconnectPeriod: 5000,
      connectTimeout: 10_000,
      keepalive: 60,
    });

    clientRef.current = client;

    client.on('connect', () => {
      console.log('MQTT connesso');
      setConnected(true);
    });

    client.on('error', (err) => {
      console.error('Errore MQTT:', err.message);
    });

    client.on('offline', () => {
      console.warn('MQTT offline');
      setConnected(false);
    });

    client.on('close', () => {
      console.warn('MQTT disconnesso');
      setConnected(false);
    });

    return () => {
      client.end(true);
      clientRef.current = null;
    };
  }, []);

  const publish = useCallback(
    async (topic: string, payload: object): Promise<boolean> => {
      const client = clientRef.current;
      if (!client || !client.connected) {
        console.error('MQTT non connesso, impossibile pubblicare');
        return false;
      }
      return new Promise((resolve) => {
        client.publish(
          topic,
          JSON.stringify(payload),
          { qos: 1 },
          (err) => {
            if (err) {
              console.error('Publish error:', err);
              resolve(false);
            } else {
              resolve(true);
            }
          }
        );
      });
    },
    []
  );

  return { publish, connected };
}