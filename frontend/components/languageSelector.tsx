import { SearchIcon, XIcon } from '@heroicons/react/solid';
import { useState } from 'react';
import { MetadataItem } from 'lib/api';

export default function LanguageSelector({ languages, setLanguage, language, setLanguageSelectorOpen }: {
    languages: Record<string, MetadataItem>,
    setLanguage: (language: string) => void,
    language: string | null,
    setLanguageSelectorOpen: (state: boolean) => void 
}) {
  const [search, setSearch] = useState('');
  const onSearchChange = (event: any) => {
    setSearch(event.target.value);
  };
  const searchFilter = (value: string) => value.toLowerCase().includes(search.toLowerCase());
  const results = languages ? Object.entries(languages).filter(([_, {name}]) => searchFilter(name)) : null;
  return (
    <div
      id="foo"
      className="fixed flex items-center justify-center overflow-auto z-50 bg-black bg-opacity-40 left-0 right-0 top-0 bottom-0"
    >
      <div
        className="bg-gray-100 dark:bg-gray-900 rounded shadow-2xl w-full sm:w-10/12 mx-5 sm:mx-10 h-3/5 p-4 flex flex-col"
        style={{ maxWidth: '50em' }}
      >
        <div className="relative">
          <h3 className="text-xl font-bold text-center">Select language</h3>
          <button
            onClick={() => { setLanguageSelectorOpen(false); }}
            className="absolute right-0 top-0 bottom-0 rounded-full bg-transparent hover:bg-gray-200 dark:hover:bg-gray-800 flex"
          >
            <XIcon className="h-5 w-5 inline-block mx-1 my-auto" />
          </button>
        </div>
        <label
          className="rounded mt-4 flex bg-gray-200 dark:bg-gray-800 focus-within:ring mx-2"
          role="search"
        >
          <input
            className="appearance-none p-2 w-full focus:outline-none bg-transparent"
            placeholder="Search"
            value={search}
            onChange={onSearchChange}
          />
          <SearchIcon className="opacity-50 mr-2 my-auto h-6 w-6 inline-block" />
        </label>
        <div className="flex-grow overflow-y-auto mt-4 mb-2 mx-2 bg-gray-200 dark:bg-gray-800 rounded p-2" role="list">
          {results && results.length > 0 ? results
            .sort(([_, {name: a}], [_2, {name: b}]) => a < b ? -1 : a > b ? 1 : 0)
            .map(([id, {name}]) => (
              <div
                key={id}
                className="p-2 rounded flex cursor-pointer hover:bg-gray-300 dark:hover:bg-gray-700"
                onClick={() => { setLanguage(id); setLanguageSelectorOpen(false); }}
                role="listitem"
              >
                <span className="flex-grow">{name}</span>
                {id === language ? <span className="bg-gray-300 dark:bg-gray-700 -mx-2 -my-1 py-1 px-2 rounded">Selected</span> : null}
              </div>
            )
            ) : <p className="p-2 text-center">{results === null ? 'Loading...' : 'No results'}</p>}
        </div>
      </div>
    </div>
  );
};
