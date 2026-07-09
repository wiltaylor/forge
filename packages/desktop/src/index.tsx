/* Forge remote desktop — shared RGBA-rect canvas viewer for VNC and RDP
   (the forge-server backend decodes the protocol; the wire is identical).
   Import '@forge/tokens/tokens.css' then '@forge/desktop/styles.css' at app
   entry.

   Exports: VncViewer, RdpViewer (thin port-defaulted wrappers), DesktopViewer
   (+ DesktopApi ref). */
import { mergeProps } from 'solid-js';
import { DesktopViewer } from './viewer';
import type { DesktopViewerProps } from './viewer';

export { DesktopViewer } from './viewer';
export type { DesktopApi, DesktopViewerProps, DesktopStatus } from './viewer';
export type { WidgetTransport } from './transport';

export function VncViewer(props: DesktopViewerProps) {
  return <DesktopViewer {...mergeProps({ port: 5900 }, props)} />;
}

export function RdpViewer(props: DesktopViewerProps) {
  return <DesktopViewer {...mergeProps({ port: 3389 }, props)} />;
}
