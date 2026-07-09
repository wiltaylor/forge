/** Metadata for one exported component, as listed in a remote manifest. */
export interface RemoteComponentMeta {
  /** Logical name used by <Remote name=...> lookups. */
  name: string;
  /** Custom element tag the bundle registers (prefix with app name + major version). */
  tag: string;
  /** Bundle file (relative to the manifest endpoint) that registers the tag. */
  file: string;
  /** Property names settable on the element. */
  props?: string[];
  /** CustomEvent names the element dispatches. */
  events?: string[];
  version?: string;
  hash?: string;
}

/** The manifest served at /api/components (inside the {ok,data} envelope). */
export interface RemoteManifest {
  app: string;
  components: RemoteComponentMeta[];
}
