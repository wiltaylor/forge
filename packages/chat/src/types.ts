import type { JSX } from 'solid-js';
import type { IconComponent, Option, StatusTone } from '@forge/ui';

/* ---------------- Participants ---------------------------------------------- */
export interface ChatParticipant {
  id: string;
  name: string;
  /** Avatar image URL — initials fallback when absent. */
  avatar?: string;
  /** Presence dot on the avatar. */
  status?: StatusTone;
}

/* ---------------- Link metadata ---------------------------------------------- */
/* Metadata cannot be fetched client-side (CORS) — supply it, or give ChatView /
   LinkCard a server-backed resolver. */
export interface LinkMeta {
  url: string;
  title?: string;
  description?: string;
  /** Preview image URL. */
  image?: string;
  /** Favicon URL. */
  icon?: string;
  /** Shown above the title; derived from `url` when absent. */
  domain?: string;
}

export type LinkResolver = (url: string) => Promise<LinkMeta | null>;

/* ---------------- Tool calls -------------------------------------------------- */
export interface ChatToolCallData {
  id?: string;
  /** Tool name, rendered in mono (e.g. "read_file"). */
  name: string;
  status: 'running' | 'success' | 'error';
  /** One-liner shown beside the name while collapsed. */
  summary?: JSX.Element;
  /** String renders as a code block; JSX renders as-is. */
  args?: string | JSX.Element;
  result?: string | JSX.Element;
  defaultOpen?: boolean;
  /** Nested sub-calls. */
  children?: ChatToolCallData[];
}

/* ---------------- Interactive prompts ----------------------------------------- */
export type ChatPromptControl =
  | { type: 'buttons'; options: Option<string>[] }
  | { type: 'radio'; options: Option<string>[] }
  | { type: 'checkbox'; options: Option<string>[] }
  | { type: 'select'; options: Option<string>[]; placeholder?: string };

export interface ChatPromptData {
  id: string;
  question: JSX.Element;
  control: ChatPromptControl;
  /** Present ⇒ answered: controls disable and the choice is highlighted. */
  answer?: string | string[];
  onAnswer?: (value: string | string[]) => void;
  /** Submit button label for radio/checkbox/select (default "Submit"). */
  submitLabel?: string;
}

/* ---------------- Message blocks ---------------------------------------------- */
export type ChatBlock =
  | { kind: 'text'; text: string; markdown?: boolean }
  | { kind: 'image'; src: string; alt?: string; width?: number; height?: number; href?: string }
  | { kind: 'video'; src: string; poster?: string; width?: number; height?: number }
  | { kind: 'file'; name: string; size?: number; href?: string; icon?: IconComponent }
  | { kind: 'link'; url: string; meta?: LinkMeta }
  | { kind: 'tool'; tool: ChatToolCallData }
  | { kind: 'prompt'; prompt: ChatPromptData }
  | { kind: 'custom'; render: () => JSX.Element };

/* ---------------- Transcript items --------------------------------------------- */
export interface ChatMessageData {
  type?: 'message';
  id: string;
  /** ChatParticipant.id */
  author: string;
  at?: string | number | Date;
  /** Shorthand for blocks: [{ kind: 'text', text }]. */
  text?: string;
  blocks?: ChatBlock[];
  /** Not yet delivered — rendered dimmed. */
  pending?: boolean;
  /** Delivery failure — danger border + caption. */
  error?: string;
}

export interface ChatEventData {
  type: 'event';
  id: string;
  text: JSX.Element;
  at?: string | number | Date;
}

export interface ChatDividerData {
  type: 'divider';
  id: string;
  label: JSX.Element;
}

export type ChatItem = ChatMessageData | ChatEventData | ChatDividerData;
