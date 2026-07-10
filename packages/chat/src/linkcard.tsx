import { Show, createResource } from 'solid-js';
import { Skeleton } from '@forge/ui';
import type { LinkMeta, LinkResolver } from './types';
import { GlobeSvg } from './internal/icons';

/* Metadata can't be fetched client-side (CORS) — pass `meta`, or a
   server-backed `resolve`. Unresolvable URLs degrade to a plain anchor. */
const metaCache = new Map<string, Promise<LinkMeta | null>>();

export interface LinkCardProps {
  url: string;
  meta?: LinkMeta;
  resolve?: LinkResolver;
}

export function LinkCard(props: LinkCardProps) {
  const [fetched] = createResource(
    () => (!props.meta && props.resolve ? props.url : null),
    (url) => {
      let p = metaCache.get(url);
      if (!p) {
        p = props.resolve!(url).catch(() => null);
        metaCache.set(url, p);
      }
      return p;
    },
  );
  const meta = () => props.meta ?? fetched();
  const loading = () => !props.meta && !!props.resolve && fetched.loading;
  const domain = () => {
    const m = meta();
    if (m?.domain) return m.domain;
    try {
      return new URL(props.url).hostname;
    } catch {
      return props.url;
    }
  };

  return (
    <Show
      when={meta() || loading()}
      fallback={
        <a class="fchat-linkplain" href={props.url} target="_blank" rel="noopener noreferrer">
          {props.url}
        </a>
      }
    >
      <a class="fchat-linkcard" href={props.url} target="_blank" rel="noopener noreferrer">
        <Show
          when={!loading()}
          fallback={
            <span class="fchat-linkcard-text" aria-hidden="true">
              <Skeleton width="90px" height="10px" />
              <Skeleton width="180px" height="12px" />
              <Skeleton width="220px" height="10px" />
            </span>
          }
        >
          <span class="fchat-linkcard-text">
            <span class="fchat-linkcard-domain">
              <Show when={meta()?.icon} fallback={<GlobeSvg />}>
                <img src={meta()!.icon} alt="" />
              </Show>
              {domain()}
            </span>
            <Show when={meta()?.title}>
              <span class="fchat-linkcard-title">{meta()!.title}</span>
            </Show>
            <Show when={meta()?.description}>
              <span class="fchat-linkcard-desc">{meta()!.description}</span>
            </Show>
          </span>
          <Show when={meta()?.image}>
            <img class="fchat-linkcard-thumb" src={meta()!.image} alt="" loading="lazy" />
          </Show>
        </Show>
      </a>
    </Show>
  );
}
