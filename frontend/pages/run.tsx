import * as msgpack from '@msgpack/msgpack';
import { AdjustmentsIcon, HomeIcon } from '@heroicons/react/outline';
import Head from 'next/head';
import Link from 'next/link';
import { SyntheticEvent, useState } from 'react';

import CollapsibleText from 'components/collapsibleText';
import Footer from 'components/footer';
import Notification from 'components/notification';

const BASE_URL = 'https://ato.pxeger.com';

interface APIResponse {
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

export default function Run() {
  const [code, setCode] = useState('');
  const [input, setInput] = useState('');
  const [output, setOutput] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [notifications, setNotifications] = useState([]);
  const dismissNotification = (target) => {
    setNotifications(notifications.filter(({ id }) => id !== target));
  };
  const notify = (text) => {
    setNotifications([{ id: Math.random(), text }, ...notifications]);
  };
  const submit = async (event: SyntheticEvent) => {
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
    if (!response.ok || !response.body) {
      notify('An error occured');
      console.error(await response.text);
      setSubmitting(false);
      return;
    }
    let data;
    try {
      data = await msgpack.decodeAsync(response.body) as APIResponse;
    } catch (e) {
      notify('An error occured');
      console.error(e);
      setSubmitting(false);
      return;
    }
    const stdout = new TextDecoder().decode(data.stdout);
    // const stderr = new TextDecoder().decode(data.stderr);
    // console.log(data);
    // console.log(stderr);
    setOutput(stdout);
    setSubmitting(false);
  };
  return (
    <>
      <Head>
        <title>Run &ndash; Attempt This Online</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white relative flex flex-col">
        <nav className="flex bg-gray-100 dark:bg-gray-800 w-full px-4 py-2 mb-4 justify-between">
          <Link href="/">
            <a className="p-2 transition hover:bg-gray-300 dark:hover:bg-gray-600 rounded-full">
              <HomeIcon className="w-6 h-6" />
            </a>
          </Link>
          <h2 className="hidden sm:block font-bold text-xl my-auto">Attempt This Online</h2>
          <Link href="/preferences">
            <a className="p-2 transition hover:bg-gray-300 dark:hover:bg-gray-600 rounded-full">
              <AdjustmentsIcon className="w-6 h-6" />
            </a>
          </Link>
        </nav>
        <div className="flex-grow relative">
          <div className="absolute top-0 z-20 w-full flex flex-col">
            {notifications.map(
              (notification) => (
                <Notification
                  key={notification.id}
                  onClick={() => dismissNotification(notification.id)}
                >
                  {notification.text}
                </Notification>
              ),
            )}
          </div>
          <main className="mb-3 mx-4 -mt-4 md:container md:mx-auto">
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
        </div>
        <Footer />
      </div>
    </>
  );
}
