export type UserState = 'disconnected' | 'stopped' | 'moving';

export interface User {
  id: number;
  name: string;
  surname: string;
  email: string;
  created_at: string;
  is_admin: boolean;
  state: UserState;
}