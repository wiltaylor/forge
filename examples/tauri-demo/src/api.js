/* The whole "backend integration": one import swap versus the web client
   (@forge/client's createClient({ baseUrl }) → @forge/tauri's createClient()).
   Everything else — data, actions, events, widgets — keeps the same API. */
import { createClient } from '@forge/tauri';

export const api = createClient();
