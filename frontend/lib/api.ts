import * as msgpack from '@msgpack/msgpack';

const BASE_URL = process.env.NEXT_PUBLIC_ATO_BASE_URL || '';
const CLOSE_NORMAL = 1000;

interface RunAPIResponseDone {
  stdout_truncated: boolean;
  stderr_truncated: boolean;
  status_type: 'exited' | 'killed' | 'core_dumped' | 'unknown';
  status_value: number;
  timed_out: boolean;
  real: number;
  kernel: number;
  user: number;
  max_mem: number;
  waits: number;
  preemptions: number;
  major_page_faults: number;
  minor_page_faults: number;
  input_ops: number;
  output_ops: number;
}

type RunAPIResponse
  = { Done: RunAPIResponseDone }
  | { Stdout: Uint8Array }
  | { Stderr: Uint8Array };

interface MetadataItem {
  name: string;
  image: string;
  version: string;
  se_class: string;
  sbcs: string;
  url: string;
}

type APIMessages = Promise<[RunAPIResponse, APIMessages | null]>;

function handleAPIConnection(ws: WebSocket): APIMessages {
  let resolve: (value: [RunAPIResponse, APIMessages | null]) => void;
  let reject: (reason: any) => void;
  const newP = () => new Promise<[RunAPIResponse, APIMessages | null]>((resolve1, reject1) => {
    resolve = resolve1;
    reject = reject1;
  });
  const p = newP();
  ws.onmessage = async message => {
    try {
      const response = await msgpack.decodeAsync(message.data.stream()) as RunAPIResponse;
      if ('Done' in response) {
        ws.close(CLOSE_NORMAL);
        resolve([response, null]);
      } else {
        resolve([response, newP()]);
      }
    } catch (e) {
      reject(e);
    }
  };
  // setup error handling
  ws.onerror = e => { console.error('websocket error:', e); };
  ws.onclose = e => {
    if (e.code !== CLOSE_NORMAL) {
      const msg = `websocket connection unexpectedly closed with code ${e.code}:\n${e.reason}`;
      reject(msg);
    }
  };
  return p;
}

async function runWs({
  language,
  input,
  code,
  options,
  programArguments,
  timeout,
}: {
  language: string,
  input: Uint8Array,
  code: Uint8Array,
  options: string[],
  programArguments: string[],
  timeout: number,
}): Promise<[() => void, APIMessages]> {
  const url = new URL(`${BASE_URL}/api/v0/ws/execute`, document.baseURI);
  if (url.protocol === 'http:') {
    url.protocol = 'ws:';
  } else if (url.protocol === 'https:') {
    url.protocol = 'wss:';
  } else {
    throw new Error(`invalid URL protocol: ${url}`);
  }
  const ws = new WebSocket(url);
  const handler = handleAPIConnection(ws);
  // wait for connection
  await new Promise(resolve => {
    ws.onopen = () => resolve(ws);
  });

  ws.send(msgpack.encode({
    language,
    code,
    input,
    options,
    arguments: programArguments,
    timeout,
  }));

  function kill() {
    ws.send(msgpack.encode('Kill'));
  }

  return [kill, handler];
}

async function getMetadata() {
  const response = await fetch('/languages.json', { method: 'GET' });
  if (!response.ok || !response.body) {
    throw new Error(await response.text());
  }
  return await response.json() as Record<string, MetadataItem>;
}

export {
  BASE_URL, getMetadata, runWs,
};
export type { RunAPIResponse, MetadataItem };
