/* Playpen data API client — the only fetch layer the app should use.
   Documents live at /api/data/<name>; custom actions at /api/actions/<name>. */

export async function loadDoc(name) {
  const res = await fetch(`/api/data/${name}`);
  if (!res.ok) return null;
  return (await res.json()).data;
}

export async function saveDoc(name, data) {
  await fetch(`/api/data/${name}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
}

const timers = {};
export function saveDocDebounced(name, data, ms = 500) {
  clearTimeout(timers[name]);
  timers[name] = setTimeout(() => saveDoc(name, data), ms);
}

export async function deleteDoc(name) {
  await fetch(`/api/data/${name}`, { method: 'DELETE' });
}

export async function listDocs() {
  const res = await fetch('/api/data');
  return (await res.json()).data;
}

export async function callAction(name, payload = {}) {
  const res = await fetch(`/api/actions/${name}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  if (!res.ok) throw new Error(`action ${name} failed: ${res.status}`);
  return (await res.json()).data;
}
