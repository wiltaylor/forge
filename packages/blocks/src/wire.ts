/* The helper types the block union's fields are made of — the TypeScript form
   of the structs and enums beside `BlockKind` in crates/forge-blocks/src/schema.rs.

   The union itself is generated (`types.gen.ts`, from the kind registry); these
   are not. A struct has no per-kind policy to generate, and the registry names
   them rather than restating them. A shape change here is a change to the frozen
   interchange contract and must land on the Rust side too — see that crate's
   tests/schema.rs for the literal fixtures. */
import type { Block } from './types.gen';

export type ListStyle = 'bullet' | 'number' | 'todo';
export type AdmonitionTone = 'info' | 'success' | 'warning' | 'danger';

export interface BlockColumn {
  ratio: number;
  blocks: Block[];
}

export interface ChartSeries {
  name: string;
  values: number[];
}

/** A labelled point annotation on a line chart; `category` indexes into the
    chart's `categories`. */
export interface ChartPoint {
  label: string;
  category: number;
  value: number;
}

export interface PieSlice {
  label: string;
  value: number;
}

export type DiagramDirection = 'right' | 'down';
export type DiagramNodeKind = 'process' | 'decision' | 'terminator' | 'node';
export type DiagramEdgeKind = 'solid' | 'dashed';

export interface DiagramNode {
  id: string;
  kind: DiagramNodeKind;
  text: string;
}

export interface DiagramEdge {
  from: string;
  to: string;
  label?: string;
  kind?: DiagramEdgeKind;
}

export type ParticipantKind = 'box' | 'actor' | 'external';
export type MessageKind = 'sync' | 'async' | 'reply';

export interface SeqParticipant {
  id: string;
  name?: string;
  kind?: ParticipantKind;
}

export interface SeqMessage {
  from: string;
  to: string;
  text?: string;
  kind?: MessageKind;
}

/** A note anchored under message index `at`. */
export interface SeqNote {
  at: number;
  text: string;
}

export interface StateNode {
  id: string;
  name?: string;
  initial?: boolean;
  final?: boolean;
}

export interface StateTransition {
  from: string;
  to: string;
  trigger?: string;
  guard?: string;
}

/** `key` is the row's stable identifier (wdoc uses it as an edge target). */
export interface NodeTableRow {
  key?: string;
  md: string;
}

export interface TreeNode {
  title: string;
  icon?: string;
  children?: TreeNode[];
}

export type TimelineDirection = 'horizontal' | 'vertical';
export type TimelineSide = 'near' | 'far';

/** `from`/`to`/`on` are ISO-8601 date strings. */
export interface TimelinePhase {
  label: string;
  from: string;
  to: string;
}

export interface TimelineItem {
  label: string;
  on: string;
  side?: TimelineSide;
}
