import { createSignal, onMount } from 'solid-js';
import { PageHead, Card } from '@forge/ui';
import { BlockEditor, emptyDocument } from '@forge/blocks';
import { api } from '../api';

/* The block editor is plain SolidJS + CSS — it works in Tauri unchanged.
   The document persists through the doc store over IPC (putDebounced →
   <app_data_dir>/data/page.json), proving blocks ride the frozen contract. */
export default function BlocksTab() {
  const [doc, setDoc] = createSignal(emptyDocument());

  onMount(async () => {
    const saved = await api.data.get('page');
    if (saved?.version) setDoc(saved);
  });

  const onChange = (next) => {
    setDoc(next);
    api.data.putDebounced('page', next);
  };

  return (
    <>
      <PageHead
        title="Page blocks"
        sub="Notion-style block editing over the Tauri doc store — same @forge/blocks package as the web gallery."
      />
      <Card title="Page" padded>
        <BlockEditor document={doc()} onChange={onChange} placeholder="Type '/' for blocks" />
      </Card>
    </>
  );
}
