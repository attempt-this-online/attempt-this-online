import { useState, ReactNode } from 'react';

function CollapsibleText({
  state: [value, setValue], id, disabled = false, children,
}: {
  state: [string, (value: string) => void],
  id: string,
  disabled?: boolean,
  children: ReactNode,
}) {
  const [open, setOpen] = useState(true);
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
        onChange={event => setValue(event.target.value)}
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
