import { SearchIcon, XIcon } from '@heroicons/react/solid';
import { useRef, useState, useEffect } from 'react';
import { MetadataItem } from 'lib/api';

export default function LanguageSelector({
  languages, language, callback
}: {
  languages: Record<string, MetadataItem>,
  language: string | null,
  callback: (id: string | null) => void,
}) {
  const searchBox = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    searchBox.current?.focus?.();
  }, []);
  const [search, setSearch] = useState('');
  const onSearchChange = (event: any) => {
    setSearch(event.target.value);
  };
  const searchFilter = (value: string) => value.toLowerCase().includes(search.toLowerCase());
  const results = (
    languages ?
      Object.entries(languages)
      .filter(([_, { name }]) => searchFilter(name))
      .sort(([_, { name: a }], [_2, { name: b }]) => a.localeCompare(b))
    : null
  );
  return (
    <>
      <div
        className="absolute bg-black bg-opacity-40 left-0 right-0 -top-4 bottom-0 z-40"
        onClick={
          (event: any) => {
            if (language !== null) {
                callback(null);
            }
          }
        }
      ></div>
      <div
        className="fixed flex items-center justify-center overflow-auto z-50 left-0 right-0 top-0 bottom-0 pointer-events-none"
        onKeyUp={(event: any) => {
          if (event.key === 'Escape' && language !== null) {
            callback(null);
          } else if (event.key === 'Enter' && results?.length && search) {
            const [id, _metadata] = results[0];
            callback(id);
          }
        }}
      >
        <div
          className="bg-gray-100 dark:bg-gray-900 rounded shadow-2xl w-full sm:w-10/12 mx-5 sm:mx-10 h-3/5 p-4 flex flex-col pointer-events-auto"
          style={{ maxWidth: '50em' }}
        >
          <div className="relative">
            <h3 className="text-xl font-bold text-center">Select language</h3>
            {language !== null && (
              <button
                type="button"
                onClick={() => callback(null)}
                className="absolute right-0 top-0 bottom-0 rounded-full bg-transparent hover:bg-gray-200 dark:hover:bg-gray-800 transition flex"
              >
                <XIcon className="h-5 w-5 inline-block mx-1 my-auto" />
              </button>
            )}
          </div>
          <label
            className="rounded mt-4 flex bg-gray-200 dark:bg-gray-800 focus-within:ring mx-2 transition"
            role="search"
          >
            <input
              className="appearance-none p-2 w-full focus:outline-none bg-transparent"
              placeholder="Search"
              value={search}
              onChange={onSearchChange}
              ref={searchBox}
            />
            <SearchIcon className="opacity-50 mr-2 my-auto h-6 w-6 inline-block" />
          </label>
          <div className="grow overflow-y-auto mt-4 mb-2 mx-2 bg-gray-200 dark:bg-gray-800 rounded p-2" role="listbox">
            {results && results.length > 0 ? results
              .map(([id, { name }]) => (
                <button
                  type="button"
                  key={id}
                  className="p-2 rounded flex w-full text-left cursor-pointer hover:bg-gray-300 dark:hover:bg-gray-700 transition"
                  onClick={() => callback(id)}
                  role="option"
                  aria-selected={id === language}
                >
                  <span className="grow">{name}</span>
                  {id === language ? <span className="bg-gray-300 dark:bg-gray-700 -mx-2 -my-1 py-1 px-2 rounded">Selected</span> : null}
                </button>
              )) : <p className="p-2 text-center">{results === null ? 'Loading...' : 'No results'}</p>}
          </div>
        </div>
      </div>
    </>
  );
};
