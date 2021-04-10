import * as msgpack from '@msgpack/msgpack';
import Head from 'next/head';
import { SyntheticEvent, useState } from 'react';

import CollapsibleText from 'components/collapsibleText';
import Footer from 'components/footer';
import Navbar from 'components/navbar';
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
  const [header, setHeader] = useState('');
  const [code, setCode] = useState('');
  const [footer, setFooter] = useState('');
  const [input, setInput] = useState('');
  const [stdout, setStdout] = useState('');
  const [stderr, setStderr] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [notifications, setNotifications] = useState<{ id: number, text: string }[]>([]);
  const dismissNotification = (target: number) => {
    setNotifications(notifications.filter(({ id }) => id !== target));
  };
  const notify = (text: string) => {
    setNotifications([{ id: Math.random(), text }, ...notifications]);
  };
  const submit = async (event: SyntheticEvent) => {
    event.preventDefault();
    setSubmitting(true);
    const codeBytes = new TextEncoder().encode((header && `${header}\n`) + code + (footer && `\n${footer}`));
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
    setStdout(new TextDecoder().decode(data.stdout));
    setStderr(new TextDecoder().decode(data.stderr));
    if (data.timed_out) {
      notify('The program ran for over 60 seconds and timed out');
    }
    setSubmitting(false);
  };

  const keyDownHandler = (e: any) => {
    if (e.ctrlKey && !e.shiftKey && !e.metaKey && !e.altKey && e.key === 'Enter') {
      submit(e);
    }
  };
  return (
    <>
      <Head>
        <title>Run &ndash; Attempt This Online</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white relative flex flex-col">
        <Navbar />
        <div className="flex-grow relative">
          <div className="absolute top-0 z-20 w-full flex flex-col">
            {notifications.map(
              notification => (
                <Notification
                  key={notification.id}
                  onClick={() => dismissNotification(notification.id)}
                >
                  {notification.text}
                </Notification>
              ),
            )}
          </div>
          <main className="mb-3 px-4 -mt-4 md:container md:mx-auto">
            <form onSubmit={submit}>
              <CollapsibleText state={[header, setHeader]} id="header" onKeyDown={keyDownHandler}>
                Header
              </CollapsibleText>
              <CollapsibleText state={[code, setCode]} id="code" onKeyDown={keyDownHandler}>
                Code
              </CollapsibleText>
              <CollapsibleText state={[footer, setFooter]} id="footer" onKeyDown={keyDownHandler}>
                Footer
              </CollapsibleText>
              <CollapsibleText state={[input, setInput]} id="input" onKeyDown={keyDownHandler}>
                Input
              </CollapsibleText>
              <button type="submit" className="mt-6 rounded px-4 py-2 bg-blue-500 text-white flex focus:outline-none focus:ring" onKeyDown={keyDownHandler}>
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
            <CollapsibleText state={[stdout, setStdout]} id="stdout" disabled onKeyDown={keyDownHandler}>
              <code>stdout</code>
              {' '}
              output
            </CollapsibleText>
            <CollapsibleText state={[stderr, setStderr]} id="stderr" disabled onKeyDown={keyDownHandler}>
              <code>stderr</code>
              {' '}
              output
            </CollapsibleText>
          </main>
        </div>
        <Footer />
      </div>
    </>
  );
}
