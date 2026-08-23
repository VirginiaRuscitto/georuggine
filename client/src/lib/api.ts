import axios from 'axios';

const API_URL = import.meta.env.VITE_API_URL || 'https://127.0.0.1:3001';

export const api = axios.create({
  baseURL: API_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Evento globale per errori
export function showError(message: string) {
  window.dispatchEvent(new CustomEvent('app-error', { detail: message }));
}

api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem('token');
      window.location.href = '/login';
    } else {
      const msg = error.response?.data?.error || error.message || 'Errore di connessione';
      showError(msg);
    }
    return Promise.reject(error);
  }
);