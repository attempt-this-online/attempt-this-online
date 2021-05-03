import { useState, useRef, useEffect } from 'react';

export default function ResizeableText(
  {
    value,
    onChange = _ => undefined,
    disabled,
    id = undefined,
    onKeyDown = _ => undefined,
    // there seems to be no proper way to get it to resize itself when opening/closing
    open
  }: {
    value: string,
    onChange?: (e: any) => void,
    disabled: boolean,
    id?: string,
    onKeyDown?: (e: any) => void,
    open: boolean,
  },
) {
  const dummy = useRef<any>(null);
  const [height, setHeight] = useState(24);
  const handleChange = (event: any) => {
    onChange(event);
    if (dummy.current) {
      dummy.current.value = event.target.value;
      setHeight(dummy.current.scrollHeight);
    }
  };
  useEffect(() => {
    if (dummy.current) {
      dummy.current.value = value;
      setHeight(dummy.current.scrollHeight);
    }
  }, [dummy, value, open]);
  return (
    <>
      <textarea ref={dummy} className="block w-full px-2 rounded font-mono text-base h-0 opacity-0" aria-hidden disabled />
      <textarea
        id={id}
        value={value}
        disabled={disabled}
        onChange={handleChange}
        onKeyDown={onKeyDown}
        style={{ height: `max(${height + 16}px, 6rem)` }}
        className="block w-full my-4 p-2 rounded bg-gray-100 dark:bg-gray-800 font-mono text-base resize-none cursor-text focus:outline-none focus:ring min-h-6"
      />
    </>
  );
}
