import { createClient } from '@forge/client';

/* One client for the whole app — same-origin (Vite proxies /api in dev). */
export const api = createClient();
