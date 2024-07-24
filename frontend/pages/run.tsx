// import localforage from 'localforage';
import Head from "next/head";
import { useRouter } from "next/router";
import { connect } from "react-redux";
import { SyntheticEvent, useEffect, useMemo, useRef, useState } from "react";
import { throttle } from "lodash";
import {
  ClipboardCopyIcon,
  ExclamationCircleIcon,
  XIcon,
} from "@heroicons/react/solid";

import CollapsibleText from "components/collapsibleText";
import ResizeableText from "components/resizeableText";
import LanguageSelector from "components/languageSelector";
import Footer from "components/footer";
import Navbar from "components/navbar";
import Options from "components/options";
import Notification from "components/notification";
import { ArgvList, parseList } from "components/argvList";
import * as API from "lib/api";
import { load, save } from "lib/urls";
import { DECODERS, ENCODERS } from "lib/encoding";
import stringLength from "lib/stringLength";
import codeToMarkdown from "lib/codeToMarkdown";
import concatUint8Array from "lib/concatUint8Array";

const NEWLINE = "\n".charCodeAt(0);

const EMPTY_BUFFER = new Uint8Array([]);

const SIGNALS: Record<number, string> = {
  1: "SIGHUP",
  2: "SIGINT",
  3: "SIGQUIT",
  4: "SIGILL",
  5: "SIGTRAP",
  6: "SIGABRT",
  7: "SIGBUS",
  8: "SIGFPE",
  9: "SIGKILL",
  10: "SIGUSR1",
  11: "SIGSEGV",
  12: "SIGUSR2",
  13: "SIGPIPE",
  14: "SIGALRM",
  15: "SIGTERM",
  // 16 unused
  17: "SIGCHLD",
  18: "SIGCONT",
  19: "SIGSTOP",
  20: "SIGTSTP",
  21: "SIGTTIN",
  22: "SIGTTOU",
  23: "SIGURG",
  24: "SIGXCPU",
  25: "SIGXFSZ",
  26: "SIGVTALRM",
  27: "SIGPROF",
  28: "SIGWINCH",
  29: "SIGIO",
  30: "SIGPWR",
  31: "SIGSYS",
};

const statusToString = (
  type: "exited" | "killed" | "core_dumped" | "unknown",
  value: number,
) => {
  switch (type) {
    case "exited":
      return `Exited with status code ${value}`;
    case "killed":
      return `Killed by ${SIGNALS[value] || "unknown signal"}`;
    case "core_dumped":
      return `Killed by ${SIGNALS[value] || "unknown signal"} and dumped core`;
    default:
      return "Unknown status";
  }
};

const pluralise = (
  string: string,
  n: number,
) => (n === 1 ? string : `${string}s`);

function _Run(
  { languages, fullWidthMode }: {
    languages: Record<string, API.MetadataItem>;
    fullWidthMode: boolean;
  },
) {
  const router = useRouter();

  const codeBox = useRef<any>(null);

  let [language, setLanguage] = useState<string | null>(null);
  const [header, setHeader] = useState("");
  const [headerEncoding, setHeaderEncoding] = useState("utf-8");
  const [code, setCode] = useState("");
  // default code encoding depends on language selected
  const [codeEncoding, setCodeEncoding] = useState<string | null>(null);
  const [footer, setFooter] = useState("");
  const [footerEncoding, setFooterEncoding] = useState("utf-8");
  const [input, setInput] = useState("");
  const [inputEncoding, setInputEncoding] = useState("utf-8");
  const [stdout, setStdout] = useState(EMPTY_BUFFER);
  const [stdoutEncoding, setStdoutEncoding] = useState("utf-8");
  const [stderr, setStderr] = useState(EMPTY_BUFFER);
  const [stderrEncoding, setStderrEncoding] = useState("utf-8");

  const [statusType, setStatusType] = useState<
    "exited" | "killed" | "core_dumped" | "unknown" | null
  >(null);
  const [statusValue, setStatusValue] = useState<number | null>(null);
  const [timing, setTiming] = useState("");
  const [timingOpen, setTimingOpen] = useState(false);

  const [languageSelectorOpen, setLanguageSelectorOpen] = useState(false);

  const [[optionsString, options], setOptions] = useState<
    [string, string[] | null]
  >(["", []]);
  const [[argsString, programArguments], setProgramArguments] = useState<
    [string, string[] | null]
  >(["", []]);

  const [isAdvanced, setIsAdvanced] = useState(false);
  const [customRunner, setCustomRunner] = useState("");
  const [customRunnerEncoding, setCustomRunnerEncoding] = useState("utf-8");

  const [submitting, setSubmitting] = useState(false);
  const [notifications, setNotifications] = useState<
    { id: number; text: string }[]
  >([]);
  const dismissNotification = (target: number) => {
    setNotifications(notifications.filter(({ id }) => id !== target));
  };
  const notify = (text: string) => {
    // avoid double notifications
    if (notifications.find((n) => n.text === text) === undefined) {
      setNotifications([{ id: Math.random(), text }, ...notifications]);
    }
  };

  // when language changes, set encoding to sbcs if it uses it by default
  useEffect(() => {
    if (!languages) {
      // not loaded yet
      return;
    }
    if (!language) {
      // not chosen yet
      return;
    }
    setCodeEncoding(languages[language].sbcs ? "sbcs" : "utf-8");
  }, [language, languages]);

  const [killCallback, setKillCallback] = useState<(() => void) | null>(null);

  const submit = async (event: SyntheticEvent) => {
    event.preventDefault();
    setSubmitting(true);

    const headerBytes = ENCODERS[headerEncoding](header);
    const footerBytes = ENCODERS[footerEncoding](footer);
    const codeBytes = ENCODERS[codeEncoding!](code);
    const combined = new Uint8Array([
      ...headerBytes,
      ...(headerBytes.length === 0 ? [] : [NEWLINE]),
      ...codeBytes,
      ...(footerBytes.length === 0 ? [] : [NEWLINE]),
      ...footerBytes,
    ]);
    const inputBytes = ENCODERS[inputEncoding](input);

    let killCallback1: () => void;
    let pData;

    try {
      const payload = {
        language: language!,
        input: inputBytes,
        code: combined,
        options: options!,
        programArguments: programArguments!,
        timeout: 60, // TODO
        customRunner: isAdvanced
          ? ENCODERS[customRunnerEncoding](customRunner)
          : null,
      };
      [killCallback1, pData] = await API.runWs(payload);
    } catch (e) {
      console.error(e);
      notify("An error occurred; see the console for details");
      setSubmitting(false);
      return;
    }

    setKillCallback(() => killCallback1);

    setStdout(EMPTY_BUFFER);
    setStderr(EMPTY_BUFFER);

    do {
      let data: API.RunAPIResponse;
      try {
        [data, pData] = await pData;
      } catch (e) {
        setKillCallback(null);
        console.error(e);
        notify("An error occurred; see the console for details");
        setSubmitting(false);
        return;
      }
      if ("Done" in data) {
        setKillCallback(null);

        setStatusType(data.Done.status_type);
        setStatusValue(data.Done.status_value);

        setTiming(
          `
          Real time: ${data.Done.real / 1e9} s
          Kernel time: ${data.Done.kernel / 1e9} s
          User time: ${data.Done.user / 1e9} s
          Maximum lifetime memory usage: ${data.Done.max_mem} KiB
          Waits (voluntary context switches): ${data.Done.waits}
          Preemptions (involuntary context switches): ${data.Done.preemptions}
          Minor page faults: ${data.Done.minor_page_faults}
          Major page faults: ${data.Done.major_page_faults}
          Input operations: ${data.Done.input_ops}
          Output operations: ${data.Done.output_ops}
          `.trim().split("\n").map((s) => s.trim()).join("\n"),
        );

        if (data.Done.timed_out) {
          notify("The program ran for over 60 seconds and timed out");
        }
        if (data.Done.stdout_truncated) {
          notify("stdout exceeded 128KiB and was truncated");
        }
        if (data.Done.stderr_truncated) {
          notify("stderr exceeded 128KiB and was truncated");
        }
      }
      if ("Stdout" in data) {
        const output = data.Stdout;
        setStdout((current) => concatUint8Array(current, output));
      }
      if ("Stderr" in data) {
        const output = data.Stderr;
        setStderr((current) => concatUint8Array(current, output));
      }
    } while (pData);

    setSubmitting(false);
  };

  const encodedStdout = useMemo(() => DECODERS[stdoutEncoding](stdout), [
    stdout,
    stdoutEncoding,
  ]);
  const encodedStderr = useMemo(() => DECODERS[stderrEncoding](stderr), [
    stderr,
    stderrEncoding,
  ]);

  const [shouldLoad, setShouldLoad] = useState(true);
  useEffect(() => {
    if (!shouldLoad || !router.isReady) {
      return;
    }
    const loadedData = load(router.query);
    if (loadedData === null) {
      // TODO: if loaded data is invalid rather than simply not present, don't open the selector?
      const loadedLanguage = router.query.L || router.query.l;
      if (loadedLanguage) {
        setLanguage(loadedLanguage as string);
        codeBox?.current?.focus();
      } else {
        setLanguageSelectorOpen(true);
      }
    } else {
      const loadedLanguage = router.query.L || router.query.l ||
        loadedData.language;
      setLanguage(loadedLanguage);
      if (loadedData.isAdvanced) {
        setCustomRunner(loadedData.customRunner);
        setCustomRunnerEncoding(loadedData.customRunnerEncoding);
        setIsAdvanced(true);
      } else {
        setOptions([loadedData.options, parseList(loadedData.options)]);
        setIsAdvanced(false);
      }
      setHeader(loadedData.header);
      setHeaderEncoding(loadedData.headerEncoding);
      setCode(loadedData.code);
      if (loadedData.codeEncoding !== null) {
        // avoid overwriting codeEncoding while it is half way through being set automatically
        setCodeEncoding(loadedData.codeEncoding);
      }
      setFooter(loadedData.footer);
      setFooterEncoding(loadedData.footerEncoding);
      setProgramArguments([
        loadedData.programArguments,
        parseList(loadedData.programArguments),
      ]);
      setInput(loadedData.input);
      setInputEncoding(loadedData.inputEncoding);
      // further updates the router.query should not trigger updates
      setShouldLoad(false);
      codeBox?.current?.focus();
    }
  }, [router, shouldLoad, codeBox]);

  if (languages && language && !languages[language]) {
    alert(`Unknown language:\n${language}`);
    setLanguage(null);
    setLanguageSelectorOpen(true);
    language = null;
  }

  // combination of useRef and useMemo: useRef ignores its argument after the second call, but the
  // argument still has to be computed which is a bit of a waste. useMemo here is not being used to
  // maintain the same stored throttle function (that's the job of useMemo), but as an optimisation
  // only.
  const updateURL = useRef(useMemo(
    // we throttle calls for 2 reasons:
    // - saving might take a while due to several layers of encoding and compression;
    //   we don't want it to block the main thread too much
    // - some browsers error on too many frequent history pushState/replaceState calls
    () =>
      throttle(
        // the debounced function cannot have any closure variable dependencies because otherwise the
        // function would have to be recreated every render and the debouncing wouldn't work properly,
        // so what would otherwise be closure variables are passed as arguments upon call.
        (router2, data) => {
          router2.replace(`/run?${save(data)}`, null, { scroll: false });
        },
        200, // milliseconds
      ),
    [],
  ));

  const charLength = stringLength(code);

  let byteLength: number = 0;
  if (codeEncoding !== null) {
    if (codeEncoding === "sbcs") {
      byteLength = stringLength(code);
    } else {
      byteLength = ENCODERS[codeEncoding](code).length;
    }
  }

  // save data in URL on data change
  useEffect(
    () => {
      if (!router.isReady) {
        return;
      }
      if (!language) {
        // not chosen
        return;
      }
      updateURL.current(router, {
        language,
        options: optionsString,
        isAdvanced,
        customRunner,
        customRunnerEncoding,
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
      optionsString,
      isAdvanced,
      customRunner,
      customRunnerEncoding,
      header,
      headerEncoding,
      code,
      codeEncoding,
      footer,
      footerEncoding,
      argsString,
      input,
      inputEncoding,
    ],
  );

  const getCurrentURL = () => {
    const saveCode = save({
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
    const baseUrl = window.location.host + window.location.pathname;
    return `https://${baseUrl}?${saveCode}`;
  };

  const [clipboardCopyModalOpen, setClipboardCopyModalOpen] = useState(false);
  const clipboardCopyModal = useRef<any>(null);
  const clipboardCopyButton = useRef<any>(null);

  const copyCGCCPost = () => {
    if (!language) {
      notify("Please select a language first!");
      return;
    }
    setClipboardCopyModalOpen(false);
    let title: string;
    if (languages[language].url) {
      title = `[${languages[language].name}](${languages[language].url})`;
    } else {
      title = languages[language].name;
    }

    const markdownCode = codeToMarkdown(code, languages[language].se_class);
    navigator.clipboard.writeText(
      `# ${title}, ${byteLength} ${pluralise("byte", byteLength)}

${markdownCode}

[Attempt This Online!](${getCurrentURL()})`,
    );

    notify("Copied to clipboard!");
  };

  const copyCMC = () => {
    if (!language) {
      notify("Please select a language first!");
      return;
    }
    setClipboardCopyModalOpen(false);
    navigator.clipboard.writeText(
      `${languages[language].name}, ${byteLength} ${
        pluralise("byte", byteLength)
      }:` +
        ` [\`${code}\`](${getCurrentURL()})`,
    );

    notify("Copied to clipboard!");
  };

  const readyToSubmit = !submitting && language && codeEncoding &&
    options !== null && programArguments !== null;

  const keyDownHandler = (e: any) => {
    if (
      readyToSubmit && e.ctrlKey && !e.shiftKey && !e.metaKey && !e.altKey &&
      e.key === "Enter"
    ) {
      submit(e);
    } else if (
      e.ctrlKey && !e.shiftKey && !e.metaKey && !e.altKey && e.key === "h"
    ) {
      e.preventDefault();
      setLanguageSelectorOpen(true);
    }
  };

  const dummy = useRef<any>(null);

  const pageTitle = `${
    language && languages ? languages[language].name : "Run"
  } – Attempt This Online`;
  return (
    <>
      <Head>
        <title>{pageTitle}</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white relative flex flex-col">
        <Navbar />
        <div className="grow relative">
          {languageSelectorOpen
            ? (
              <LanguageSelector
                {...{ language, languages }}
                callback={(id: string) => {
                  if (id !== null) {
                    setLanguage(id);
                  }
                  setLanguageSelectorOpen(false);
                  codeBox?.current?.focus();
                }}
              />
            )
            : null}
          <div className="sticky h-0 top-4 z-20 pointer-events-none">
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
          <main
            className={`mb-3 px-4 -mt-4${
              fullWidthMode ? "" : " md:container md:mx-auto"
            }`}
          >
            <form onSubmit={submit}>
              <div className="flex flex-wrap justify-between mt-4 pb-1 gap-y-2">
                <div className="mr-4 my-auto" style={{ flexGrow: 99999 }}>
                  <span className="my-auto">
                    <span>
                      Language:
                      {" "}
                    </span>
                    {languages && language && (
                      <a
                        className="mx-2 text-blue-500 underline"
                        href={languages[language].url}
                      >
                        {languages[language].name}
                      </a>
                    )}
                  </span>
                  <button
                    type="button"
                    onClick={() => {
                      setLanguageSelectorOpen(true);
                    }}
                    className="ml-1 rounded px-2 py-1 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 focus:outline-none focus:ring hover:bg-gray-300 transition"
                  >
                    Change
                  </button>
                </div>
                <div className="flex grow justify-between relative">
                  <code className="my-auto mr-4 font-mono bg-gray-200 dark:bg-gray-800 px-2 py-px rounded">
                    {charLength} {pluralise("char", charLength)}
                    {", "}
                    {byteLength} {pluralise("byte", byteLength)}
                  </code>
                  <button
                    type="button"
                    onClick={() => {
                      setClipboardCopyModalOpen(!clipboardCopyModalOpen);
                    }}
                    onBlur={(event: any) => {
                      if (
                        clipboardCopyModal.current &&
                        !clipboardCopyModal.current.contains(
                          event.relatedTarget,
                        )
                      ) {
                        setClipboardCopyModalOpen(false);
                      }
                    }}
                    ref={clipboardCopyButton}
                    className="rounded p-2 bg-blue-500 hover:bg-blue-400 text-white focus:outline-none focus:ring transition"
                  >
                    <ClipboardCopyIcon className="w-6 h-6" />
                  </button>
                  {clipboardCopyModalOpen && (
                    <fieldset
                      className="absolute top-14 right-0 bg-gray-200 dark:bg-gray-800 p-4 rounded ring-blue-500 ring-opacity-40 ring shadow-lg z-40 flex flex-col w-max"
                      onBlur={(event: any) => {
                        if (
                          clipboardCopyButton.current &&
                          !clipboardCopyButton.current.contains(
                            event.relatedTarget,
                          )
                        ) {
                          setClipboardCopyModalOpen(false);
                        }
                      }}
                      ref={clipboardCopyModal}
                      tabIndex={-1 /* https://stackoverflow.com/q/42764494 */}
                    >
                      <h3 className="-mt-1 mb-2 flex relative">
                        <legend>Copy to clipboard</legend>
                        <button
                          type="button"
                          onClick={() => {
                            setClipboardCopyModalOpen(false);
                          }}
                          className="absolute top-0 -right-1 bottom-0 p-1 rounded-full bg-transparent hover:bg-gray-300 dark:hover:bg-gray-700 transition flex"
                        >
                          <XIcon className="h-4 w-4" />
                        </button>
                      </h3>
                      <div className="m-auto flex">
                        <button
                          type="button"
                          onClick={copyCMC}
                          className="mr-4 rounded px-4 py-2 bg-gray-300 dark:bg-gray-700 dark:text-white hover:bg-gray-400 dark:hover:bg-gray-600 focus:outline-none focus:ring transition"
                        >
                          <abbr title="Chat Mini Challenge">CMC</abbr>
                        </button>
                        <button
                          type="button"
                          onClick={copyCGCCPost}
                          className="rounded px-4 py-2 bg-gray-300 dark:bg-gray-700 dark:text-white hover:bg-gray-400 dark:hover:bg-gray-600 focus:outline-none focus:ring transition"
                        >
                          <abbr title="Code Golf and Coding Challenges (Stack Exchange)">
                            CGCC
                          </abbr>{" "}
                          Post
                        </button>
                      </div>
                    </fieldset>
                  )}
                </div>
              </div>
              <Options
                optionsString={optionsString}
                options={options}
                setOptions={setOptions}
                keyDownHandler={keyDownHandler}
                isAdvanced={isAdvanced}
                setIsAdvanced={setIsAdvanced}
                customRunner={customRunner}
                setCustomRunner={setCustomRunner}
                customRunnerEncoding={customRunnerEncoding}
                setCustomRunnerEncoding={setCustomRunnerEncoding}
                dummy={dummy}
              />
              <CollapsibleText
                value={header}
                setValue={setHeader}
                encoding={headerEncoding}
                onEncodingChange={(e) => setHeaderEncoding(e.target.value)}
                id="header"
                onKeyDown={keyDownHandler}
                dummy={dummy}
              >
                Header
              </CollapsibleText>
              <CollapsibleText
                value={code}
                setValue={setCode}
                encoding={codeEncoding ?? "utf-8"}
                onEncodingChange={(e) => setCodeEncoding(e.target.value)}
                id="code"
                onKeyDown={keyDownHandler}
                dummy={dummy}
                ref={codeBox}
              >
                Code
              </CollapsibleText>
              <CollapsibleText
                value={footer}
                setValue={setFooter}
                encoding={footerEncoding}
                onEncodingChange={(e) => setFooterEncoding(e.target.value)}
                id="footer"
                onKeyDown={keyDownHandler}
                dummy={dummy}
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
                setValue={setInput}
                encoding={inputEncoding}
                onEncodingChange={(e) => setInputEncoding(e.target.value)}
                id="input"
                onKeyDown={keyDownHandler}
                dummy={dummy}
              >
                Input
              </CollapsibleText>
              <div className="flex mb-6 items-center">
                <button
                  type="submit"
                  className="rounded px-4 py-2 bg-blue-500 hover:bg-blue-400 text-white flex focus:outline-none focus:ring disabled:bg-gray-200 disabled:text-black dark:disabled:bg-gray-700 dark:disabled:text-white disabled:cursor-not-allowed transition"
                  onKeyDown={keyDownHandler}
                  disabled={!readyToSubmit}
                >
                  <span>Execute</span>
                  {submitting && (
                    /* this SVG is taken from https://git.io/JYHot, under the MIT licence https://git.io/JYHoh */
                    <svg
                      className="animate-spin my-auto -mr-1 ml-3 h-5 w-5"
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                    >
                      <circle
                        className="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        strokeWidth="4"
                      />
                      <path
                        className="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                      />
                    </svg>
                  )}
                  {!readyToSubmit && !submitting && (
                    <ExclamationCircleIcon className="text-red-500 h-6 w-6 my-auto -mr-2 ml-1" />
                  )}
                </button>
                {submitting && (
                  <button
                    type="button"
                    className="rounded ml-4 px-4 py-2 bg-red-500 hover:bg-red-500 text-white focus:outline-none ring-red-300/50 focus:ring disabled:bg-gray-200 disabled:text-black dark:disabled:bg-gray-700 dark:disabled:text-white disabled:cursor-not-allowed transition"
                    disabled={!killCallback}
                    onClick={() => killCallback?.()}
                  >
                    Kill
                  </button>
                )}
                {statusType && (
                  <p className="ml-4">
                    {statusToString(statusType, statusValue!)}
                  </p>
                )}
              </div>
            </form>
            <CollapsibleText
              value={encodedStdout}
              encoding={stdoutEncoding}
              onEncodingChange={(e) => setStdoutEncoding(e.target.value)}
              id="stdout"
              onKeyDown={keyDownHandler}
              readOnly
              dummy={dummy}
            >
              <code>stdout</code> output
            </CollapsibleText>
            <CollapsibleText
              value={encodedStderr}
              encoding={stderrEncoding}
              onEncodingChange={(e) => setStderrEncoding(e.target.value)}
              id="stderr"
              onKeyDown={keyDownHandler}
              readOnly
              dummy={dummy}
            >
              <code>stderr</code> output
            </CollapsibleText>
            <details open={timingOpen} className="my-6">
              <summary
                className="cursor-pointer focus-within:ring rounded pl-2 hover:bg-gray-200 dark:hover:bg-gray-700 transition py-1 -mt-3 -mb-1"
                tabIndex={-1}
              >
                <button
                  type="button"
                  onClick={() => {
                    setTimingOpen(!timingOpen);
                  }}
                  className="select-none focus:outline-none"
                >
                  Timing details
                </button>
              </summary>
              <ResizeableText
                readOnly
                value={timing}
                dummy={dummy}
              />
            </details>
            <textarea
              ref={dummy}
              className="block w-full px-2 rounded font-mono text-base h-0 opacity-0"
              aria-hidden
              disabled
            />
          </main>
        </div>
        <Footer />
      </div>
    </>
  );
}

const Run = connect((state: any) => ({
  fullWidthMode: state.fullWidthMode,
  languages: state.metadata,
}))(_Run);
export default Run;
