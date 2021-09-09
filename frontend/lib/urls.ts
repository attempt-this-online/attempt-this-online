import { toUint8Array, fromUint8Array } from 'js-base64';
import * as msgpack from '@msgpack/msgpack';
import { deflateRaw, inflateRaw } from 'pako';

const SCHEMA = 1;

function save(data: any): string {
  const compactedData = [
    data.language,
    data.options,
    data.header,
    data.headerEncoding,
    data.code,
    data.codeEncoding,
    data.footer,
    data.footerEncoding,
    data.programArguments,
    data.input,
    data.inputEncoding,
  ];
  const msgpacked = msgpack.encode(compactedData);
  const compressed = deflateRaw(msgpacked);
  const b64encoded = fromUint8Array(compressed, true); // true means URL-safe
  const params = new URLSearchParams();
  params.set(SCHEMA.toString(), b64encoded);
  return params.toString();
}

function load1(b64encoded: string): any {
  const compressed = toUint8Array(b64encoded);
  const msgpacked = inflateRaw(compressed);
  const compactedData = msgpack.decode(msgpacked) as any;
  if (compactedData.length !== 11) {
    console.error('invalid URL data (expected array of length 11)', compactedData);
    return null;
  }
  const [
    language,
    options,
    header,
    headerEncoding,
    code,
    codeEncoding,
    footer,
    footerEncoding,
    programArguments,
    input,
    inputEncoding,
  ] = compactedData;
  return {
    language,
    options,
    header,
    headerEncoding,
    code,
    codeEncoding,
    footer,
    footerEncoding,
    programArguments,
    input,
    inputEncoding,
  };
}

function load0(b64encoded: string): any {
  const compressed = toUint8Array(b64encoded);
  const msgpacked = inflateRaw(compressed);
  const compactedData = msgpack.decode(msgpacked) as any;
  if (compactedData.length !== 9) {
    console.error('invalid URL data (expected array of length 9)', compactedData);
    return null;
  }
  const [
    language,
    header,
    headerEncoding,
    code,
    codeEncoding,
    footer,
    footerEncoding,
    input,
    inputEncoding,
  ] = compactedData;
  return {
    language,
    options: '',
    header,
    headerEncoding,
    code,
    codeEncoding,
    footer,
    footerEncoding,
    programArguments: '',
    input,
    inputEncoding,
  };
}

function load(query: any): any {
  const latestVersion = Object.keys(query).reduce((acc, key) => {
    const value = parseInt(key, 10);
    if (isNaN(value) || value.toString() !== key) {
      // invalid integer
      return acc;
    }
    if (value < 0) {
      console.warn('invalid integer key in URL', key);
      return acc;
    } if (value <= SCHEMA && value > acc) {
      // valid version and maximum so far
      return value;
    } if (value === acc) {
      console.warn('duplicate save keys in URL; first value will be used', key);
      return acc;
    }
    return acc;
  }, -1);
  switch (latestVersion) {
    case 0:
      return load0(query[0]);
    case 1:
      return load1(query[1]);
    default:
      console.warn('missing or invalid save key in URL');
      return null;
  }
}

export { save, load };
