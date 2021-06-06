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
  waits: number;
  preemptions: number;
  major_page_faults: number;
  minor_page_faults: number;
  input_ops: number;
  output_ops: number;
}

async function run({
  language,
  input,
  code,
  options,
  programArguments,
}: {
  language: string,
  input: Uint8Array,
  code: Uint8Array,
  options: string[],
  programArguments: string[],
}) {
  const response = await fetch(`${BASE_URL}/api/v0/execute`, {
    method: 'POST',
    body: msgpack.encode({
      language,
      code,
      input,
      options,
      arguments: programArguments,
    }),
  });
  if (!response.ok || !response.body) {
    throw new Error(await response.text());
  }
  return await msgpack.decodeAsync(response.body) as RunAPIResponse;
}

async function getMetadata() {
  const response = await fetch(`${BASE_URL}/api/v0/metadata`, { method: 'GET' });
  if (!response.ok || !response.body) {
    throw new Error(await response.text());
  }
  return await msgpack.decodeAsync(response.body) as Record<string, string>[];
}

export { BASE_URL, run, getMetadata };
export type { RunAPIResponse };
