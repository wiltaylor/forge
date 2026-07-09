/* Port type → token colour (from the Forge port-colour table in tokens.md). */
export const PORT_COLORS: Record<string, string> = {
  trigger: 'var(--fg-0)',
  string: 'var(--success)',
  number: 'var(--info)',
  boolean: 'var(--danger)',
  object: 'var(--accent)',
  array: 'var(--warning)',
  any: 'var(--fg-3)',
};

/** Built-in port types — arbitrary strings fall back to the `any` colour. */
export type PortType = 'trigger' | 'string' | 'number' | 'boolean' | 'object' | 'array' | 'any';

export const portColor = (type: string | undefined): string => PORT_COLORS[type ?? ''] ?? PORT_COLORS['any']!;

/** A point on the graph canvas, in pixels. */
export interface Point {
  x: number;
  y: number;
}

/* ---------------- Elbow routing -------------------------------------------- */
const STUB = 16;   // min straight run out of / into a port
const BEND_R = 6;  // elbow corner radius

/* Orthogonal (Manhattan) polyline: a exits rightward, b enters leftward. */
function elbowPoints(a: Point, b: Point): Point[] {
  if (b.x - a.x >= 2 * STUB) {
    if (a.y === b.y) return [a, b];
    const mx = (a.x + b.x) / 2;
    return [a, { x: mx, y: a.y }, { x: mx, y: b.y }, b];
  }
  const my = (a.y + b.y) / 2;  // backward: detour around via stubs
  return [
    a,
    { x: a.x + STUB, y: a.y },
    { x: a.x + STUB, y: my },
    { x: b.x - STUB, y: my },
    { x: b.x - STUB, y: b.y },
    b,
  ];
}

const dist = (p: Point, q: Point) => Math.abs(p.x - q.x) + Math.abs(p.y - q.y);  // axis-aligned
function towards(c: Point, p: Point, r: number): Point {
  const dx = Math.sign(p.x - c.x), dy = Math.sign(p.y - c.y);
  return { x: c.x + dx * r, y: c.y + dy * r };
}

function roundedPath(pts: Point[], r = BEND_R): string {
  let d = `M ${pts[0]!.x} ${pts[0]!.y}`;
  for (let i = 1; i < pts.length - 1; i++) {
    const p = pts[i - 1]!, c = pts[i]!, n = pts[i + 1]!;
    const rr = Math.min(r, dist(p, c) / 2, dist(c, n) / 2);
    const inPt = towards(c, p, rr), outPt = towards(c, n, rr);
    d += ` L ${inPt.x} ${inPt.y} Q ${c.x} ${c.y} ${outPt.x} ${outPt.y}`;
  }
  const last = pts[pts.length - 1]!;
  return `${d} L ${last.x} ${last.y}`;
}

export const edgePath = (a: Point, b: Point): string => roundedPath(elbowPoints(a, b));
