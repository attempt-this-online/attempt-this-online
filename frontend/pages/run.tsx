import * as msgpack from '@msgpack/msgpack';
import Head from 'next/head';
import { useState } from 'react';

import CollapsibleText from 'components/collapsibleText';
import Footer from 'components/footer';

const BASE_URL = 'https://ato.pxeger.com';

export default function Run() {
  const [code, setCode] = useState('');
  const [input, setInput] = useState('');
  const [output, setOutput] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const submit = async (event) => {
    event.preventDefault();
    setSubmitting(true);
    const codeBytes = new TextEncoder().encode(code);
    const inputBytes = new TextEncoder().encode(input);
    const response = await fetch(`${BASE_URL}/api/v0/execute`, {
      method: 'POST',
      body: msgpack.encode({
        language: 'python',
        code: codeBytes,
        input: inputBytes,
        options: [],
        arguments: [],
      }),
    });
    const data = await msgpack.decodeAsync(response.body);
    data.stdout = new TextDecoder().decode(data.stdout);
    data.stderr = new TextDecoder().decode(data.stderr);
    setOutput(JSON.stringify(data, null, 2));
    setSubmitting(false);
  };
  return (
    <>
      <Head>
        <title>Run &ndash; Attempt This Online</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white py-8 relative">
        <main className="mb-3 mx-4 -mt-4 md:container md:mx-auto text-lg">
          <form onSubmit={submit}>
            <CollapsibleText state={[code, setCode]} id="code" text="Code" />
            <CollapsibleText state={[input, setInput]} id="input" text="Input" />
            <button type="submit" className="mt-6 rounded px-4 py-2 bg-blue-500 text-white flex">
              <span>Execute</span>
              {submitting && (
              /* this SVG is taken from https://git.io/JYHot, under the MIT licence https://git.io/JYHoh */
              <svg className="animate-spin my-auto -mr-1 ml-3 h-5 w-5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              )}
            </button>
          </form>
          <CollapsibleText state={[output, setOutput]} id="output" text="Output" disabled />
        </main>
        <Footer />
      </div>
    </>
  );
}
