import * as msgpack from '@msgpack/msgpack';

const BASE_URL = process.env.NEXT_PUBLIC_ATO_BASE_URL || '';
const CLOSE_NORMAL = 1000;

interface RunAPIResponse {
  stdout: Uint8Array;
  // stdout_truncated: boolean;
  stderr: Uint8Array;
  // stderr_truncated: boolean;
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

interface MetadataItem {
  name: string;
  image: string;
  version: string;
  se_class: string;
  sbcs: string;
  url: string;
}

async function run({
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
}) {
  const response = await fetch(`${BASE_URL}/api/v0/execute`, {
    method: 'POST',
    body: msgpack.encode({
      language,
      code,
      input,
      options,
      arguments: programArguments,
      timeout,
    }),
  });
  if (!response.ok || !response.body) {
    throw new Error(await response.text());
  }
  return await msgpack.decodeAsync(response.body) as RunAPIResponse;
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
}): Promise<[() => void, Promise<RunAPIResponse>]> {
  const url = new URL(`${BASE_URL}/api/v0/ws/execute`, document.baseURI);
  if (url.protocol === 'http:') {
    url.protocol = 'ws:';
  } else if (url.protocol === 'https:') {
    url.protocol = 'wss:';
  } else {
    throw new Error(`invalid URL protocol: ${url}`);
  }
  const ws = new WebSocket(url);
  // wait for connection
  await new Promise(resolve => {
    ws.onopen = () => resolve(ws);
  });

  const p_message = new Promise<RunAPIResponse>((resolve, reject) => {
    ws.onmessage = async message => {
      ws.close(CLOSE_NORMAL);
      resolve(await msgpack.decodeAsync(message.data.stream()) as RunAPIResponse);
    };
    // setup error handling
    ws.onerror = e => { console.error('websocket error:', e); };
    ws.onclose = e => {
      if (e.code !== CLOSE_NORMAL) {
        const msg = `websocket connection unexpectedly closed with code ${e.code}:\n${e.reason}`;
        console.error(msg);
        reject(msg);
      }
    };
  });

  ws.send(msgpack.encode({
    language,
    code,
    input,
    options,
    arguments: programArguments,
    timeout,
  }));

  async function kill() {
    ws.send(msgpack.encode('Kill'));
  }

  return [kill, p_message];
}

async function getMetadata() {
  const response = await fetch(`/languages.json`, { method: 'GET' });
  if (!response.ok || !response.body) {
    throw new Error(await response.text());
  }
  return await response.json() as Record<string, MetadataItem>;
}

export {
  BASE_URL, run, getMetadata, runWs,
};
export type { RunAPIResponse, MetadataItem };
