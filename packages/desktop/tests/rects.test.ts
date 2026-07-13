import { describe, it, expect } from 'vitest';
import { deflateSync } from 'fflate';
import {
  parseRect,
  decodeRgba,
  createPaintQueue,
  ENC_RAW,
  ENC_DEFLATE,
  ENC_JPEG,
} from '../src/rects';

/** Build a wire rect frame: 10-byte LE header + payload. */
function frame(encoding: number, w: number, h: number, payload: Uint8Array, version = 1) {
  const buf = new Uint8Array(10 + payload.length);
  const dv = new DataView(buf.buffer);
  dv.setUint8(0, version);
  dv.setUint8(1, encoding);
  dv.setUint16(2, 7, true); /* x */
  dv.setUint16(4, 9, true); /* y */
  dv.setUint16(6, w, true);
  dv.setUint16(8, h, true);
  buf.set(payload, 10);
  return buf.buffer;
}

const rawPixels = (n: number) => Uint8Array.from({ length: n * 4 }, (_, i) => (i * 37) % 256);

describe('parseRect', () => {
  it('parses a valid raw frame', () => {
    const rect = parseRect(frame(ENC_RAW, 2, 3, rawPixels(6)));
    expect(rect).toMatchObject({ encoding: ENC_RAW, x: 7, y: 9, w: 2, h: 3 });
    expect(rect!.payload.length).toBe(24);
  });

  it('drops unknown versions, encodings, and empty rects', () => {
    expect(parseRect(frame(ENC_RAW, 2, 3, rawPixels(6), 2))).toBeNull();
    expect(parseRect(frame(3, 2, 3, rawPixels(6)))).toBeNull();
    expect(parseRect(frame(ENC_RAW, 0, 3, new Uint8Array(0)))).toBeNull();
    expect(parseRect(frame(ENC_DEFLATE, 2, 3, new Uint8Array(0)))).toBeNull();
  });

  it('drops truncated headers and raw length mismatches', () => {
    expect(parseRect(new ArrayBuffer(9))).toBeNull();
    expect(parseRect(frame(ENC_RAW, 2, 3, rawPixels(5)))).toBeNull();
    expect(parseRect(frame(ENC_RAW, 2, 3, rawPixels(7)))).toBeNull();
  });
});

describe('decodeRgba', () => {
  it('views raw payloads without copying', () => {
    const rect = parseRect(frame(ENC_RAW, 2, 3, rawPixels(6)))!;
    const rgba = decodeRgba(rect)!;
    expect(rgba).toEqual(new Uint8ClampedArray(rawPixels(6)));
    expect(rgba.buffer).toBe(rect.payload.buffer);
  });

  it('inflates deflate payloads', () => {
    const pixels = rawPixels(6);
    const rect = parseRect(frame(ENC_DEFLATE, 2, 3, deflateSync(pixels)))!;
    expect(decodeRgba(rect)).toEqual(new Uint8ClampedArray(pixels));
  });

  it('rejects wrong inflated sizes and corrupt streams', () => {
    const short = parseRect(frame(ENC_DEFLATE, 2, 3, deflateSync(rawPixels(5))))!;
    expect(decodeRgba(short)).toBeNull();
    const corrupt = parseRect(frame(ENC_DEFLATE, 2, 3, rawPixels(2)))!;
    expect(decodeRgba(corrupt)).toBeNull();
  });

  it('leaves JPEG payloads to the bitmap path', () => {
    const rect = parseRect(frame(ENC_JPEG, 2, 3, rawPixels(2)))!;
    expect(decodeRgba(rect)).toBeNull();
  });
});

describe('createPaintQueue', () => {
  it('paints in enqueue order even when an early job is slow', async () => {
    const queue = createPaintQueue();
    const order: string[] = [];
    queue.enqueue(async () => {
      await new Promise((r) => setTimeout(r, 20));
      order.push('slow');
    });
    queue.enqueue(() => {
      order.push('fast');
    });
    await new Promise((r) => setTimeout(r, 50));
    expect(order).toEqual(['slow', 'fast']);
  });

  it('skips jobs enqueued before a bump', async () => {
    const queue = createPaintQueue();
    const order: string[] = [];
    queue.enqueue(() => {
      order.push('stale');
    });
    queue.bump();
    queue.enqueue(() => {
      order.push('fresh');
    });
    await new Promise((r) => setTimeout(r, 10));
    expect(order).toEqual(['fresh']);
  });

  it('reports staleness across a job-internal await', async () => {
    const queue = createPaintQueue();
    let sawStale: boolean | null = null;
    let release!: () => void;
    const gate = new Promise<void>((r) => { release = r; });
    queue.enqueue(async (stale) => {
      await gate;
      sawStale = stale();
    });
    await new Promise((r) => setTimeout(r, 0));
    queue.bump(); /* while the job is parked on its internal await */
    release();
    await new Promise((r) => setTimeout(r, 10));
    expect(sawStale).toBe(true);
  });

  it('keeps the chain alive after a failed job', async () => {
    const queue = createPaintQueue();
    const order: string[] = [];
    queue.enqueue(() => {
      throw new Error('paint failed');
    });
    queue.enqueue(() => {
      order.push('after-failure');
    });
    await new Promise((r) => setTimeout(r, 10));
    expect(order).toEqual(['after-failure']);
  });
});
