import { ExclamationCircleIcon } from '@heroicons/react/solid';

const parseList = (list: string) => {
  if (list === '') {
    // convenience
    return [];
  }
  let data;
  try {
    data = JSON.parse(list);
  } catch (e) {
    console.error('invalid JSON', e);
    return null;
  }
  const output: string[] | null = [];
  if (!(typeof data === 'object' && data.constructor === Array)) {
    console.error('items must be an array', data);
    return null;
  }
  // for (const item of data) {
  for (let i = 0; i < data.length; i += 1) {
    const item = data[i];
    if (typeof item === 'object') {
      console.error('items must not be objects', item);
      return null;
    }
    output.push(item.toString());
  }
  return output;
};

function ArgvList({
  state, setState, keyDownHandler, children, id,
}: {
  state: [string, string[] | null],
  setState: (state: [string, string[] | null]) => void,
  keyDownHandler: (event: any) => void,
  children: any,
  id: string
}) {
  const [stringState, parsedData] = state;
  const onKeyDown = (event: any) => {
    if (event.key === 'Enter' && !event.ctrlKey) {
      // in <input> elements, Enter submits the form by default
      // but we only want to submit with Ctrl+Enter
      event.preventDefault();
    }
    keyDownHandler(event);
  };
  return (
    <div className="flex">
      <label htmlFor={id} className="my-auto">
        {children}
      </label>
      <div className="flex ml-3 flex-grow rounded bg-gray-100 dark:bg-gray-800 focus-within:ring transition">
        <input
          type="text"
          placeholder="[]"
          id={id}
          value={stringState}
          className="p-2 w-full rounded bg-transparent font-mono focus:outline-none"
          onChange={e => {
            const parsed = parseList(e.target.value);
            setState([e.target.value, parsed]);
          }}
          onKeyDown={onKeyDown}
        />
        {parsedData === null ? (
          <ExclamationCircleIcon
            className="text-red-500 mx-2 my-auto h-6 w-6 inline-block"
          />
        ) : undefined}
      </div>
    </div>
  );
}

export { ArgvList, parseList };
