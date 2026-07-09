/* Forge terminal — xterm.js over a per-connection WebSocket (/api/term).
   Import '@forge/tokens/tokens.css' then '@forge/term/styles.css' at app entry.

   Exports: Terminal (+ TerminalApi ref), readTermTheme/watchTheme. */

export { Terminal } from './terminal';
export type { TerminalApi, TerminalProps, TerminalStatus } from './terminal';
export { readTermTheme, watchTheme } from './theme';
