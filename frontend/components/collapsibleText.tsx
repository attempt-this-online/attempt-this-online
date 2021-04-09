import localforage from 'localforage';
import { debounce } from 'lodash';
import { useEffect, useMemo, useState, ReactNode } from 'react';

// milliseconds
const DEBOUNCE = 100;

function CollapsibleText({
  state: [value, setValue], id, disabled = false, children,
}: {
  state: [string, (value: string) => void],
  id: string,
  disabled?: boolean,
  children: ReactNode,
}) {
  const [open, setOpen] = useState(true);
  // don't recreate the debouncer on every render
  const save = useMemo(
    () =>
      // don't save too quickly
      debounce(
        async v => {
          await localforage.setItem('ATO_saved_' + id, v); 
        },
        DEBOUNCE),
    []
  );
  // restore saved code
  useEffect(() => {
    localforage.getItem('ATO_saved_' + id)
      .then(v => { setValue(v || ''); });
  }, []);
  const handleChange = event => {
    setValue(event.target.value);
    save(event.target.value);
  }
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
        className="block w-full my-4 p-2 rounded bg-gray-100 dark:bg-gray-800 font-mono text-base resize-y cursor-text focus:outline-none focus:ring"
        style={{ minHeight: '6rem' }}
      />
    </details>
  );
}

CollapsibleText.defaultProps = {
  disabled: false,
};

export default CollapsibleText;
