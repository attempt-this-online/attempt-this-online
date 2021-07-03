// import localforage from 'localforage';
import Head from 'next/head';
import { useRouter } from 'next/router';
import { connect } from 'react-redux';
import {
  SyntheticEvent, useMemo, useRef, useState, useEffect,
} from 'react';
import { escape, throttle } from 'lodash';

import CollapsibleText from 'components/collapsibleText';
import ResizeableText from 'components/resizeableText';
import Footer from 'components/footer';
import Navbar from 'components/navbar';
import Notification from 'components/notification';
import { ArgvList, parseList } from 'components/argvList';
import * as API from 'lib/api';
import { save, load } from 'lib/urls';

const ENCODERS: Record<string, ((s: string) => Uint8Array)> = {
  'utf-8': s => new TextEncoder().encode(s),
};

const DECODERS: Record<string, ((b: Uint8Array) => string)> = {
  'utf-8': b => new TextDecoder().decode(b),
};

const NEWLINE = '\n'.charCodeAt(0);

const SIGNALS: Record<number, string> = {
  1: 'SIGHUP',
  2: 'SIGINT',
  3: 'SIGQUIT',
  4: 'SIGILL',
  5: 'SIGTRAP',
  6: 'SIGABRT',
  7: 'SIGBUS',
  8: 'SIGFPE',
  9: 'SIGKILL',
  10: 'SIGUSR1',
  11: 'SIGSEGV',
  12: 'SIGUSR2',
  13: 'SIGPIPE',
  14: 'SIGALRM',
  15: 'SIGTERM',
  // 16 unused
  17: 'SIGCHLD',
  18: 'SIGCONT',
  19: 'SIGSTOP',
  20: 'SIGTSTP',
  21: 'SIGTTIN',
  22: 'SIGTTOU',
  23: 'SIGURG',
  24: 'SIGXCPU',
  25: 'SIGXFSZ',
  26: 'SIGVTALRM',
  27: 'SIGPROF',
  28: 'SIGWINCH',
  29: 'SIGIO',
  30: 'SIGPWR',
  31: 'SIGSYS',
};

const statusToString = (type: 'exited' | 'killed' | 'core_dumped' | 'unknown', value: number) => {
  switch (type) {
    case 'exited':
      return `Exited with status code ${value}`;
    case 'killed':
      return `Killed by ${SIGNALS[value] || 'unknown signal'}`;
    case 'core_dumped':
      return `Killed by ${SIGNALS[value] || 'unknown signal'} and dumped core`;
    default:
      return 'Unknown status';
  }
};

const pluralise = (string: string, n: number) => (n === 1 ? string : `${string}s`);

function _Run({ languages }: { languages: Record<string, Record<string, any>> }) {
  const router = useRouter();

  const [language, setLanguage] = useState('python');
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

  const [statusType, setStatusType] = useState<'exited' | 'killed' | 'core_dumped' | 'unknown' | null>(null);
  const [statusValue, setStatusValue] = useState<number | null>(null);
  const [timing, setTiming] = useState('');
  const [timingOpen, setTimingOpen] = useState(false);

  const [[optionsString, options], setOptions] = useState<[string, string[] | null]>(['', []]);
  const [[argsString, programArguments], setProgramArguments] = useState<[string, string[] | null]>(['', []]);

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

    let data;

    try {
      data = await API.run({
        language,
        input: inputBytes,
        code: combined,
        options: options!,
        programArguments: programArguments!,
      });
    } catch (e) {
      console.error(e);
      notify('An error occurred; see the console for details');
      setSubmitting(false);
      return;
    }

    setStdout(DECODERS[stdoutEncoding](data.stdout));
    setStderr(DECODERS[stderrEncoding](data.stderr));

    setStatusType(data.status_type);
    setStatusValue(data.status_value);

    setTiming(
      `
      Real time: ${data.real / 1e9} s
      Kernel time: ${data.kernel / 1e9} s
      User time: ${data.kernel / 1e9} s
      Maximum lifetime memory usage: ${data.max_mem} KiB
      Waits (volunatry context switches): ${data.waits}
      Preemptions (involuntary context switches): ${data.preemptions}
      Minor page faults: ${data.minor_page_faults}
      Major page faults: ${data.major_page_faults}
      Input operations: ${data.input_ops}
      Output operations: ${data.output_ops}
      `.trim().split('\n').map(s => s.trim()).join('\n'),
    );

    if (data.timed_out) {
      notify('The program ran for over 60 seconds and timed out');
    }

    setSubmitting(false);
  };

  const [shouldLoad, setShouldLoad] = useState(true);
  useEffect(() => {
    if (!shouldLoad || !router.isReady) {
      return;
    }
    const loadedData = load(router.query);
    if (loadedData !== null) {
      setLanguage(loadedData.language);
      setOptions([loadedData.options, parseList(loadedData.options)]);
      setHeader(loadedData.header);
      setHeaderEncoding(loadedData.headerEncoding);
      setCode(loadedData.code);
      setCodeEncoding(loadedData.codeEncoding);
      setFooter(loadedData.footer);
      setFooterEncoding(loadedData.footerEncoding);
      setProgramArguments([loadedData.programArguments, parseList(loadedData.programArguments)]);
      setInput(loadedData.input);
      setInputEncoding(loadedData.inputEncoding);
      // further updates the router.query should not trigger updates
      setShouldLoad(false);
    }
  }, [router, shouldLoad]);

  // combination of useRef and useMemo: useRef ignores its argument after the second call, but the
  // argument still has to be computed which is a bit of a waste. useMemo here is not being used to
  // maintain the same stored throttle function (that's the job of useMemo), but as an optimisation
  // only.
  const updateURL = useRef(useMemo(
    // we throttle calls for 2 reasons:
    // - saving might take a while due to several layers of encoding and compression;
    //   we don't want it to block the main thread too much
    // - some browsers error on too many frequent history pushState/replaceState calls
    () => throttle(
      // the debounced function cannot have any closure variable dependencies because otherwise the
      // function would have to be recreated every render and the debouncing wouldn't work properly,
      // so what would otherwise be closure variables are passed as arguments upon call.
      (router2, data) => {
        router2.replace(`/run?${save(data)}`);
      },
      200, // milliseconds
    ),
    [],
  ));

  const byteLength = ENCODERS[codeEncoding](code).length;

  // save data in URL on data change
  useEffect(
    () => {
      // TODO: fix jank with not including router in useEffect dependencies!
      // For now I've got the right combination such that the URL never flickers, but it's fragile
      // and I don't really understand why it would or wouldn't work.
      if (!router.isReady) {
        return;
      }
      updateURL.current(router, {
        language,
        options: optionsString,
        header,
        headerEncoding,
        code,
        codeEncoding,
        footer,
        footerEncoding,
        programArguments: argsString,
        input,
        inputEncoding,
      });
    },
    // update when these values change:
    [
      language,
      header,
      headerEncoding,
      code,
      codeEncoding,
      footer,
      footerEncoding,
      input,
      inputEncoding,
    ],
  );

  const copyCGCCPost = () => {
    if (!language) {
      notify('Please select a language first!');
      return;
    }
    // avoid double notifications
    if (notifications.find(n => n.text === 'Copied to clipboard!') === undefined) {
      notify('Copied to clipboard!');
    }
    const url = save({
      language,
      options: optionsString,
      header,
      headerEncoding,
      code,
      codeEncoding,
      footer,
      footerEncoding,
      programArguments: argsString,
      input,
      inputEncoding,
    });
    navigator.clipboard.writeText(`# ${languages[language].name}, ${byteLength} ${pluralise('byte', byteLength)}

<pre><code>${escape(code)}</code></pre>

[Attempt This Online!](https://ato.pxeger.com/run?${url})`);
  };

  const keyDownHandler = (e: any) => {
    if (!submitting && language && options !== null && programArguments !== null && e.ctrlKey && !e.shiftKey && !e.metaKey && !e.altKey && e.key === 'Enter') {
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
              <div className="lg:flex flex-wrap lg:flex-nowrap items-center mt-4 pb-1">
                <div className="flex sm:block lg:flex-grow">
                  <label className="my-auto" htmlFor="languageSelector">Language:</label>
                  <select
                    id="languageSelector"
                    className="appearance-none ml-2 p-2 flex-grow sm:w-80 rounded bg-gray-100 hover:bg-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 transition cursor-pointer ATO_select focus:outline-none focus:ring"
                    value={language || ''}
                    onChange={e => setLanguage(e.target.value)}
                  >
                    {Object.keys(languages ?? {}).map(l => (
                      <option value={l} key={l}>{languages[l].name}</option>
                    ))}
                  </select>
                </div>
                <div className="mt-3 lg:mt-0 flex justify-between">
                  <code className="my-auto mr-4 font-mono bg-gray-200 dark:bg-gray-800 px-2 py-px rounded">
                    {code.length}
                    {' '}
                    {pluralise('char', code.length)}
                    {', '}
                    {byteLength}
                    {' '}
                    {pluralise('byte', byteLength)}
                  </code>
                  <button
                    type="button"
                    onClick={copyCGCCPost}
                    className="rounded px-4 py-2 bg-blue-500 text-white flex focus:outline-none focus:ring disabled:cursor-not-allowed"
                  >
                    CGCC Post
                  </button>
                </div>
              </div>
              <div className="pt-3 pb-1">
                <ArgvList
                  id="options"
                  state={[optionsString, options]}
                  setState={setOptions}
                  keyDownHandler={keyDownHandler}
                >
                  Options:
                </ArgvList>
              </div>
              <CollapsibleText
                value={header}
                onChange={e => setHeader(e.target.value)}
                encoding={headerEncoding}
                onEncodingChange={e => setHeaderEncoding(e.target.value)}
                id="header"
                onKeyDown={keyDownHandler}
              >
                Header
              </CollapsibleText>
              <CollapsibleText
                value={code}
                onChange={e => setCode(e.target.value)}
                encoding={codeEncoding}
                onEncodingChange={e => setCodeEncoding(e.target.value)}
                id="code"
                onKeyDown={keyDownHandler}
              >
                Code
              </CollapsibleText>
              <CollapsibleText
                value={footer}
                onChange={e => setFooter(e.target.value)}
                encoding={footerEncoding}
                onEncodingChange={e => setFooterEncoding(e.target.value)}
                id="footer"
                onKeyDown={keyDownHandler}
              >
                Footer
              </CollapsibleText>
              <div className="pb-1 -mt-2">
                <ArgvList
                  id="programArguments"
                  state={[argsString, programArguments]}
                  setState={setProgramArguments}
                  keyDownHandler={keyDownHandler}
                >
                  Arguments:
                </ArgvList>
              </div>
              <CollapsibleText
                value={input}
                onChange={e => setInput(e.target.value)}
                encoding={inputEncoding}
                onEncodingChange={e => setInputEncoding(e.target.value)}
                id="input"
                onKeyDown={keyDownHandler}
              >
                Input
              </CollapsibleText>
              <div className="flex mb-6 items-center">
                <button
                  type="submit"
                  className="rounded px-4 py-2 bg-blue-500 text-white flex focus:outline-none focus:ring disabled:cursor-not-allowed"
                  onKeyDown={keyDownHandler}
                  disabled={submitting || !language || options === null || programArguments === null}
                >
                  <span>Execute</span>
                  {submitting && (
                  /* this SVG is taken from https://git.io/JYHot, under the MIT licence https://git.io/JYHoh */
                  <svg className="animate-spin my-auto -mr-1 ml-3 h-5 w-5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                  </svg>
                  )}
                </button>
                {statusType && (
                <p className="ml-4">
                  {statusToString(statusType, statusValue!)}
                </p>
                )}
              </div>
            </form>
            <CollapsibleText
              value={stdout}
              onChange={e => setStdout(e.target.value)}
              encoding={stdoutEncoding}
              onEncodingChange={e => setStdoutEncoding(e.target.value)}
              id="stdout"
              onKeyDown={keyDownHandler}
              disabled
            >
              <code>stdout</code>
              {' '}
              output
            </CollapsibleText>
            <CollapsibleText
              value={stderr}
              onChange={e => setStderr(e.target.value)}
              encoding={stderrEncoding}
              onEncodingChange={e => setStderrEncoding(e.target.value)}
              id="stderr"
              onKeyDown={keyDownHandler}
              disabled
            >
              <code>stderr</code>
              {' '}
              output
            </CollapsibleText>
            <details open={timingOpen} className="my-6">
              <summary className="cursor-pointer focus-within:ring rounded pl-2 hover:bg-gray-200 dark:hover:bg-gray-700 transition py-1 -mt-3 -mb-1">
                <button
                  type="button"
                  onClick={() => { setTimingOpen(!timingOpen); }}
                  className="select-none focus:outline-none"
                >
                  Timing details
                </button>
              </summary>
              <ResizeableText
                disabled
                value={timing}
                open={timingOpen}
              />
            </details>
          </main>
        </div>
        <Footer />
      </div>
    </>
  );
}

const Run = connect((state: any) => ({ languages: state.metadata }))(_Run);
export default Run;
