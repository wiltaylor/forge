/* Binary rect frame decoding for the desktop viewer (no DOM — unit-testable).

   Frame layout (LE): u8 version=1, u8 encoding, u16 x, u16 y, u16 w, u16 h,
   then the payload. Encodings: 0 = raw RGBA (w*h*4 bytes), 1 = raw deflate
   (RFC 1951) of the RGBA payload, 2 = baseline JPEG (painted via
   createImageBitmap in the viewer, not decoded here). The server only emits
   encodings advertised in the `connect` frame. */
import { inflateSync } from 'fflate';

export const ENC_RAW = 0;
export const ENC_DEFLATE = 1;
export const ENC_JPEG = 2;
/** Advertised in the `connect` frame, as wire header-byte values. */
export const SUPPORTED_ENCODINGS = [ENC_RAW, ENC_DEFLATE, ENC_JPEG];

const HEADER_LEN = 10;

export interface RectFrame {
  encoding: number;
  x: number;
  y: number;
  w: number;
  h: number;
  /** View over the message buffer — no copy. */
  payload: Uint8Array<ArrayBuffer>;
}

/** Parse + validate the rect frame header. null = drop the frame (unknown
    version or encoding, zero-size rect, missing or mis-sized payload). */
export function parseRect(buf: ArrayBuffer): RectFrame | null {
  if (buf.byteLength < HEADER_LEN) return null;
  const dv = new DataView(buf);
  if (dv.getUint8(0) !== 1) return null; /* unknown frame version */
  const encoding = dv.getUint8(1);
  if (encoding !== ENC_RAW && encoding !== ENC_DEFLATE && encoding !== ENC_JPEG) return null;
  const x = dv.getUint16(2, true);
  const y = dv.getUint16(4, true);
  const w = dv.getUint16(6, true);
  const h = dv.getUint16(8, true);
  if (!w || !h || buf.byteLength === HEADER_LEN) return null;
  if (encoding === ENC_RAW && buf.byteLength !== HEADER_LEN + w * h * 4) return null;
  return { encoding, x, y, w, h, payload: new Uint8Array(buf, HEADER_LEN) };
}

/** Raw/deflate payload → RGBA pixels of exactly w*h*4 bytes; null = drop.
    JPEG rects are not handled here — the viewer paints them as bitmaps. */
export function decodeRgba(rect: RectFrame): Uint8ClampedArray<ArrayBuffer> | null {
  const size = rect.w * rect.h * 4;
  if (rect.encoding === ENC_RAW) {
    /* parseRect validated the length; view, don't copy. */
    return new Uint8ClampedArray(rect.payload.buffer, rect.payload.byteOffset, size);
  }
  if (rect.encoding !== ENC_DEFLATE) return null;
  let out: Uint8Array;
  try {
    out = inflateSync(rect.payload);
  } catch {
    return null;
  }
  if (out.length !== size) return null;
  /* fflate's types predate generic buffers; it always allocates a plain
     ArrayBuffer. */
  return new Uint8ClampedArray(out.buffer as ArrayBuffer, out.byteOffset, size);
}

/* ---------------- ordered paint queue ----------------------------------------
   JPEG rects decode asynchronously (createImageBitmap) while raw/deflate rects
   decode synchronously — but rects MUST paint in arrival order (later rects
   overwrite earlier ones). The queue serializes paints on a promise chain;
   decode work can still run ahead of the chain. `bump()` invalidates every
   job enqueued before it (disconnect, or a resize that cleared the canvas);
   long-running jobs get a `stale()` probe to re-check after internal awaits. */

export interface PaintQueue {
  enqueue(job: (stale: () => boolean) => void | Promise<void>): void;
  /** Invalidate all previously enqueued jobs. */
  bump(): void;
}

export function createPaintQueue(): PaintQueue {
  let chain: Promise<unknown> = Promise.resolve();
  let epoch = 0;
  return {
    enqueue(job) {
      const e = epoch;
      const stale = () => e !== epoch;
      chain = chain
        .then(() => (stale() ? undefined : job(stale)))
        .catch(() => undefined); /* a failed paint must not stall the chain */
    },
    bump() {
      epoch += 1;
    },
  };
}
