import * as msgpack from '@msgpack/msgpack';
import localforage from 'localforage';
import Head from 'next/head';
import { SyntheticEvent, useState, useEffect } from 'react';

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

const ENCODERS: Record<string, ((s: string) => Uint8Array)> = {
  'utf-8': s => new TextEncoder().encode(s),
};

const DECODERS: Record<string, ((b: Uint8Array) => string)> = {
  'utf-8': b => new TextDecoder().decode(b),
};

const NEWLINE = '\n'.charCodeAt(0);

export default function Run() {
  const [language, setLanguage] = useState('');
  const [header, setHeader] = useState('');
  const [headerEncoding, setHeaderEncoding] = useState('utf-8');
  const [code, setCode] = useState('');
  const [codeEncoding, setCodeEncoding] = useState('utf-8');
  const [footer, setFooter] = useState('');
  const [footerEncoding, setFooterEncoding] = useState('utf-8');
  const [input, setInput] = useState('');
  const [inputEncoding, setInputEncoding] = useState('utf-8');
  const [stdout, setStdout] = useState('');
  const [stdoutEncoding, setStdoutEncoding] = useState('utf-8');
  const [stderr, setStderr] = useState('');
  const [stderrEncoding, setStderrEncoding] = useState('utf-8');

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

    const headerBytes = ENCODERS[headerEncoding](header);
    const footerBytes = ENCODERS[footerEncoding](footer);
    const codeBytes = ENCODERS[codeEncoding](code);
    const combined = new Uint8Array([
      ...headerBytes,
      ...(headerBytes.length === 0 ? [] : [NEWLINE]),
      ...codeBytes,
      ...(footerBytes.length === 0 ? [] : [NEWLINE]),
      ...footerBytes,
    ]);
    const inputBytes = ENCODERS[inputEncoding](input);

    const response = await fetch(`${BASE_URL}/api/v0/execute`, {
      method: 'POST',
      body: msgpack.encode({
        language,
        code: combined,
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

    setStdout(DECODERS[stdoutEncoding](data.stdout));
    setStderr(DECODERS[stderrEncoding](data.stderr));

    if (data.timed_out) {
      notify('The program ran for over 60 seconds and timed out');
    }
    setSubmitting(false);
  };

  useEffect(() => {
    localforage.getItem('ATO_saved_language').then((l: any) => l && setLanguage(l));
  }, []);

  const languageChangeHandler = (e: any) => {
    setLanguage(e.target.value);
    localforage.setItem('ATO_saved_language', e.target.value);
  };

  const keyDownHandler = (e: any) => {
    if (!submitting && language && e.ctrlKey && !e.shiftKey && !e.metaKey && !e.altKey && e.key === 'Enter') {
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
              <div className="flex items-center mt-4 pb-1">
                <label htmlFor="languageSelector">Language:</label>
                <select
                  id="languageSelector"
                  className="appearance-none ml-2 p-2 w-80 rounded bg-gray-100 hover:bg-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 transition cursor-pointer ATO_select focus:outline-none focus:ring"
                  value={language}
                  onChange={languageChangeHandler}
                >
                  <option value=""></option>
                  <option value="python">Python</option>
                </select>
              </div>
              <CollapsibleText state={[header, setHeader]} encodingState={[headerEncoding, setHeaderEncoding]} id="header" onKeyDown={keyDownHandler}>
                Header
              </CollapsibleText>
              <CollapsibleText state={[code, setCode]} encodingState={[codeEncoding, setCodeEncoding]} id="code" onKeyDown={keyDownHandler}>
                Code
              </CollapsibleText>
              <CollapsibleText state={[footer, setFooter]} encodingState={[footerEncoding, setFooterEncoding]} id="footer" onKeyDown={keyDownHandler}>
                Footer
              </CollapsibleText>
              <CollapsibleText state={[input, setInput]} encodingState={[inputEncoding, setInputEncoding]} id="input" onKeyDown={keyDownHandler}>
                Input
              </CollapsibleText>
              <button type="submit" className="mb-6 rounded px-4 py-2 bg-blue-500 text-white flex focus:outline-none focus:ring disabled:cursor-not-allowed" onKeyDown={keyDownHandler} disabled={submitting || !language}>
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
            <CollapsibleText state={[stdout, setStdout]} encodingState={[stdoutEncoding, setStdoutEncoding]} id="stdout" disabled onKeyDown={keyDownHandler}>
              <code>stdout</code>
              {' '}
              output
            </CollapsibleText>
            <CollapsibleText state={[stderr, setStderr]} encodingState={[stderrEncoding, setStderrEncoding]} id="stderr" disabled onKeyDown={keyDownHandler}>
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
