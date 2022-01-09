import { Base64 } from 'js-base64';

const ENCODERS: Record<string, ((s: string) => Uint8Array)> = {
  'utf-8': s => new TextEncoder().encode(s),
  // sbcs is still just UTF-8, but with different byte counting (switching to a different code page,
  // if necessary, is done on the backend)
  sbcs: s => new TextEncoder().encode(s),
  base64: s => Base64.toUint8Array(s),
};

const DECODERS: Record<string, ((b: Uint8Array) => string)> = {
  'utf-8': b => new TextDecoder().decode(b),
  base64: b => Base64.fromUint8Array(b),
};

export { ENCODERS, DECODERS };
