import CollapsibleText from 'components/collapsibleText';
import { ArgvList } from 'components/argvList';

const advancedModeExampleUrl = 'http://localhost:3000/run?1=TY_NSsNAFIX3eYrDWCURk1goRYgVfAI3gmApMp0fM5DOrclEArVv4qKb-kYu-jZOMhG9i5nLOd-9Z-bzKF5ehTh9b96R3z8-5IKk-usyEXkX6ROvqv8qUkLz1vJaRZpqGLttnT-xm2bZ9HpfQFIEX9Q67ywmsRIlgU0GkOEDWR7GkwEzGsultwPOsMB8htWqgCuVHYi-wg4vjdGoiRxI97BpMC4fcG0iSVYdjq3T6c3p-cxYUbX-Z7eNk4ay8i4y1mHDjY0T7MIjvNAVQ9sIbnXMziW7wkWXBHFbe-JX7XCJ3tiHhDHoMN5fa96o-ewH';

function Options({
  optionsString,
  options,
  setOptions,
  keyDownHandler,
  isAdvanced,
  setIsAdvanced,
  customRunner,
  setCustomRunner,
  customRunnerEncoding,
  setCustomRunnerEncoding,
  dummy,
}: {
  optionsString: string,
  options: string[] | null,
  setOptions: (options: [string, string[] | null]) => void,
  keyDownHandler: (event: any) => void,
  isAdvanced: boolean,
  setIsAdvanced: (isAdvanced: boolean) => void,
  customRunner: string,
  setCustomRunner: (customRunner: string) => void,
  customRunnerEncoding: string,
  setCustomRunnerEncoding: (customRunnerEncoding: string) => void,
  dummy: any,
}) {
  if (isAdvanced) {
    return (
      <fieldset className="mt-3 border-gray-200 dark:border-gray-700 border-2 rounded-2xl">
        <legend className="mx-4 px-2">
          Advanced mode
          <button
            type="button"
            onClick={() => setIsAdvanced(false)}
            className="rounded ml-3 mr-1 -my-1 py-1 px-3 bg-gray-300 dark:bg-gray-700 dark:text-white hover:bg-red-300 dark:hover:bg-red-500 focus:outline-none focus:ring transition"
          >
            Exit
          </button>
        </legend>
        <p className="mt-3 px-4 text-justify">
          In advanced mode, your code is not run directly.
          Instead, you write a custom runner script in Bash to invoke the compiler and run your code.
          This allows you maximum flexibility in how many times the code is run, with what options and inputs, and what you do with the outputs.
          {' '}
          <a href={advancedModeExampleUrl} target="_blank" className="text-blue-500 underline">
            See an example
            {/* icon from heroicons.com, MIT licensed */}
            {/* TODO: once #130 is fixed, remove this! */}
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" className="inline ml-1 w-4 h-4">
              <path d="M6.22 8.72a.75.75 0 0 0 1.06 1.06l5.22-5.22v1.69a.75.75 0 0 0 1.5 0v-3.5a.75.75 0 0 0-.75-.75h-3.5a.75.75 0 0 0 0 1.5h1.69L6.22 8.72Z" />
              <path d="M3.5 6.75c0-.69.56-1.25 1.25-1.25H7A.75.75 0 0 0 7 4H4.75A2.75 2.75 0 0 0 2 6.75v4.5A2.75 2.75 0 0 0 4.75 14h4.5A2.75 2.75 0 0 0 12 11.25V9a.75.75 0 0 0-1.5 0v2.25c0 .69-.56 1.25-1.25 1.25h-4.5c-.69 0-1.25-.56-1.25-1.25v-4.5Z" />
            </svg>
          </a>
        </p>
        <div className="px-4 -mb-1.5 -mt-4">
          <CollapsibleText
            value={customRunner}
            setValue={setCustomRunner}
            encoding={customRunnerEncoding}
            onEncodingChange={e => setCustomRunnerEncoding(e.target.value)}
            id="customRunner"
            onKeyDown={keyDownHandler}
            dummy={dummy}
          >
            Custom runner
          </CollapsibleText>
        </div>
      </fieldset>
    );
  }
  return (
    <div className="pt-3 pb-1 flex">
      <div className="flex-grow">
        <ArgvList
          id="options"
          state={[optionsString, options]}
          setState={setOptions}
          keyDownHandler={keyDownHandler}
        >
          Options:
        </ArgvList>
      </div>
      <button
        type="button"
        onClick={() => setIsAdvanced(true)}
        className="rounded ml-4 px-3 bg-gray-300 dark:bg-gray-700 dark:text-white hover:bg-gray-400 dark:hover:bg-gray-600 focus:outline-none focus:ring transition"
      >
        Advanced
      </button>
    </div>
  );
}

export default Options;
