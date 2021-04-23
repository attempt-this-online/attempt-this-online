import localforage from 'localforage';
import { debounce } from 'lodash';
import {
  useEffect, useMemo, useState, ReactNode,
} from 'react';

// milliseconds
const DEBOUNCE = 100;

function CollapsibleText({
  state: [value, setValue],
  encodingState: [encoding, setEncoding],
  id,
  disabled = false,
  children,
  onKeyDown,
}: {
  state: [string, (value: string) => void],
  encodingState: [string, (value: string) => void],
  id: string,
  disabled?: boolean,
  children: ReactNode,
  onKeyDown: (event: any) => void,
}) {
  const [open, setOpen] = useState(true);
  // don't recreate the debouncer on every render
  const save = useMemo(
    () => debounce( // don't save too quickly
      async v => {
        await localforage.setItem(`ATO_saved_${id}`, v);
      },
      DEBOUNCE,
    ),
    [id],
  );
  // restore saved code
  useEffect(() => {
    localforage.getItem(`ATO_saved_${id}`)
      .then((v: string) => { setValue(v || value); });
    localforage.getItem(`ATO_encoding_${id}`)
      .then((e: string) => { setEncoding(e || encoding); });
  }, [id]);
  const handleChange = (event: any) => {
    setValue(event.target.value);
    save(event.target.value);
  };
  const handleChangeEncoding = (event: any) => {
    setEncoding(event.target.value);
    localforage.setItem(`ATO_encoding_${id}`, event.target.value);
  };
  return (
    <div className="relative">
      <details open={open} className="my-6">
        <summary className="cursor-pointer focus-within:ring rounded pl-2 hover:bg-gray-200 dark:hover:bg-gray-700 transition py-1 -mt-3 -mb-1">
          <label htmlFor={`textarea:${id}`}>
            <button
              type="button"
              onClick={() => { setOpen(!open); }}
              className="select-none focus:outline-none"
            >
              {children}
            </button>
          </label>
        </summary>
        <textarea
          id={`textarea:${id}`}
          value={value}
          disabled={disabled}
          onChange={handleChange}
          onKeyDown={onKeyDown}
          className="block w-full my-4 p-2 rounded bg-gray-100 dark:bg-gray-800 font-mono text-base resize-y cursor-text focus:outline-none focus:ring min-h-6"
        />
        <div className="absolute top-0 right-0">
          <label htmlFor={`encodingSelect:${id}`}>
            Encoding:
          </label>
          {' '}
          <select
            value={encoding}
            onChange={handleChangeEncoding}
            id={`encodingSelect:${id}`}
            className="appearance-none ml-1 p-1 rounded bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 transition cursor-pointer ATO_select focus:outline-none focus:ring"
          >
            <option value="utf-8">UTF-8</option>
          </select>
        </div>
      </details>
    </div>
  );
}

CollapsibleText.defaultProps = {
  disabled: false,
};

export default CollapsibleText;
