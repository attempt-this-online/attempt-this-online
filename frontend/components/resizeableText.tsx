import { useState, useEffect } from 'react';
import { useSelector } from 'react-redux';

export default function ResizeableText(
  {
    value,
    onChange = _ => undefined,
    readOnly,
    id = undefined,
    onKeyDown = _ => undefined,
    dummy,
  }: {
    value: string,
    onChange?: (e: any) => void,
    readOnly: boolean,
    id?: string,
    onKeyDown?: (e: any) => void,
    dummy: any,
  },
) {
  const [height, setHeight] = useState(24);
  const bigTextBoxes = useSelector((state: any) => state.bigTextBoxes);
  const handleChange = (event: any) => {
    onChange(event);
    if (dummy.current) {
      dummy.current.value = event.target.value;
      setHeight(dummy.current.scrollHeight);
      dummy.current.value = '';
    }
  };
  const resize = () => {
    if (dummy.current) {
      dummy.current.value = value;
      setHeight(dummy.current.scrollHeight);
      dummy.current.value = '';
    }
  };
  useEffect(resize, [value]);
  return (
    <>
      <textarea
        id={id}
        value={value}
        readOnly={readOnly}
        onChange={handleChange}
        onKeyDown={onKeyDown}
        style={{ height: `max(${height + 2 * 8}px, ${(bigTextBoxes ? 3 : 1) * 24 + 2 * 8}px)` }}
        className="block w-full my-4 p-2 rounded bg-gray-100 dark:bg-gray-800 font-mono text-base resize-none cursor-text focus:outline-none focus:ring transition"
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="none"
        spellCheck={false}
      />
    </>
  );
}
