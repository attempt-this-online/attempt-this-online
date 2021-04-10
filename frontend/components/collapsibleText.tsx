import localforage from 'localforage';
import { debounce } from 'lodash';
import {
  useEffect, useMemo, useState, ReactNode,
} from 'react';

// milliseconds
const DEBOUNCE = 100;

function CollapsibleText({
  state: [value, setValue], id, disabled = false, children, onKeyDown,
}: {
  state: [string, (value: string) => void],
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
      .then((v: string) => { setValue(v || ''); });
  }, [id]);
  const handleChange = (event: any) => {
    setValue(event.target.value);
    save(event.target.value);
  };
  return (
    <details open={open} className="my-4">
      <summary className="cursor-pointer">
        <label htmlFor={id}>
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
        id={id}
        value={value}
        disabled={disabled}
        onChange={handleChange}
        onKeyDown={onKeyDown}
        className="block w-full my-4 p-2 rounded bg-gray-100 dark:bg-gray-800 font-mono text-base resize-y cursor-text focus:outline-none focus:ring min-h-6"
      />
    </details>
  );
}

CollapsibleText.defaultProps = {
  disabled: false,
};

export default CollapsibleText;
