/* Remote bundle entry — registers every exported widget as a custom element.
   Component CSS ships inside the bundle via ?inline; token CSS deliberately
   does NOT (it inherits from the host document, so host themes apply). */
import uiCss from '@forge/ui/styles.css?inline';
import chartsCss from '@forge/charts/styles.css?inline';
import { defineRemoteElement } from '@forge/remote';
import { StatusCard } from './StatusCard';
import { MetricsPanel } from './MetricsPanel';

const css = `${uiCss}\n${chartsCss}`;

defineRemoteElement('forge-rw-status-card', StatusCard, {
  props: ['title', 'status', 'message'],
  events: ['refresh'],
  css,
});

defineRemoteElement('forge-rw-metrics-panel', MetricsPanel, {
  props: ['title', 'series', 'unit'],
  events: ['select'],
  css,
});
