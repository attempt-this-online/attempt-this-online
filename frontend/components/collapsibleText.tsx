import {
  useState, useEffect, ReactNode,
} from 'react';

import ResizeableText from 'components/resizeableText';

function CollapsibleText({
  value,
  setValue,
  encoding,
  onEncodingChange,
  id,
  readOnly = false,
  children,
  onKeyDown,
  dummy,
}: {
  value: string,
  setValue?: (event: any) => void,
  encoding: string,
  onEncodingChange: (event: any) => void,
  id: string,
  readOnly?: boolean,
  children: ReactNode,
  onKeyDown: (event: any) => void,
  dummy: any,
}) {
  const [open, setOpen] = useState(true);
  const [modified, setModified] = useState(false);

  useEffect(() => {
    if (modified) {
      // don't open when already manually modified
      return;
    }
    if (id === 'header' || id === 'footer' || id === 'input') {
      // close by default when empty
      setOpen(!!value);
    }
  }, [modified, id, value]);

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
        <ResizeableText
          id={id}
          value={value}
          setValue={event => { setModified(true); setValue?.(event); }}
          onKeyDown={onKeyDown}
          readOnly={readOnly}
          dummy={dummy}
        />
        <div className="absolute top-0 right-0">
          <label htmlFor={`encodingSelect:${id}`}>
            Encoding:
          </label>
          {' '}
          <select
            value={encoding}
            onChange={onEncodingChange}
            id={`encodingSelect:${id}`}
            className="appearance-none ml-1 p-1 rounded bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 transition cursor-pointer ATO_select focus:outline-none focus:ring"
          >
            <option value="utf-8">UTF-8</option>
            <option value="base64">Base64</option>
            {id === 'code' ? (<option value="sbcs">SBCS</option>) : undefined}
          </select>
        </div>
      </details>
    </div>
  );
}

CollapsibleText.defaultProps = {
  readOnly: false,
};

export default CollapsibleText;
