export default function concatUint8Array(a: Uint8Array, b: Uint8Array): Uint8Array {
  const ret = new Uint8Array(a.length + b.length);
  ret.set(a);
  ret.set(b, a.length);
  return ret;
}
