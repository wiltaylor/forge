/* Private inline SVGs — chrome icons that keep @forge/chat usable without an
   icon library. Consumer-provided icons come in via `icon` props. */

export const ChevronRightSvg = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M9 6l6 6-6 6" />
  </svg>
);

export const ArrowDownSvg = () => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <line x1="12" y1="4" x2="12" y2="20" /><path d="M6 14l6 6 6-6" />
  </svg>
);

export const SendSvg = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M21 3L10 14" /><path d="M21 3l-7 18-4-7-7-4 18-7z" />
  </svg>
);

export const FileSvg = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
    <path d="M14 3v5h5" />
  </svg>
);

export const GlobeSvg = () => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" aria-hidden="true">
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18" /><path d="M12 3a13.5 13.5 0 0 1 0 18 13.5 13.5 0 0 1 0-18z" />
  </svg>
);
