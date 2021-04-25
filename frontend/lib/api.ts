import * as msgpack from '@msgpack/msgpack';

const BASE_URL = 'https://ato.pxeger.com';

interface RunAPIResponse {
  stdout: Uint8Array;
  stderr: Uint8Array;
  status_type: 'exited' | 'killed' | 'core_dumped' | 'unknown';
  status_value: number;
  timed_out: boolean;
  real: number;
  kernel: number;
  user: number;
  max_mem: number;
  unshared: number;
  shared: number;
  waits: number;
  preemptions: number;
  major_page_faults: number;
  minor_page_faults: number;
  swaps: number;
  signals_recv: number;
  input_ops: number;
  output_ops: number;
  socket_recv: number;
  socket_sent: number;
}

async function run({
  language,
  input,
  code,
}) {
  const response = await fetch(`${BASE_URL}/api/v0/execute`, {
    method: 'POST',
    body: msgpack.encode({
      language,
      code,
      input,
      options: [],
      arguments: [],
    }),
  });
  if (!response.ok || !response.body) {
    throw new Error(await response.text());
  }
  return await msgpack.decodeAsync(response.body) as APIResponse;
}

async function getMetadata() {
  const response = await fetch(`${BASE_URL}/api/v0/metadata`, { method: 'GET' });
  if (!response.ok || !response.body) {
    throw new Error(await response.text);
  }
  return await msgpack.decodeAsync(response.body) as Record<string, string>[];
}

export { RunAPIResponse, run, getMetadata };
