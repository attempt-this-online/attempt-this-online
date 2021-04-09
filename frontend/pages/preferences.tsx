import { ArrowLeftIcon } from '@heroicons/react/outline';
import Head from 'next/head';
import Link from 'next/link';
import localForage from 'localforage';
import { useState } from 'react';
import { useDispatch } from 'react-redux';

import useSystemThemePreference from 'lib/useSystemThemePreference';
import Footer from 'components/footer';

export default function Preferences() {
  const dispatch = useDispatch();
  const [theme, setTheme] = useState('system');
  const systemThemePreference = useSystemThemePreference();
  const handleThemeChange = async (event: any) => {
    event.preventDefault();
    setTheme(event.target.value);
    dispatch({ type: 'setTheme', theme: event.target.value });
    await localForage.setItem('ATO_theme', event.target.value);
  };
  return (
    <>
      <Head>
        <title>Prefences &ndash; Attempt This Online</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white pt-8 relative flex flex-col">
        <main className="mb-3 px-4 md:container md:mx-auto flex-grow">
          <header className="flex mb-2">
            <Link href="/run">
              <a className="my-auto p-2 transition hover:bg-gray-300 dark:hover:bg-gray-600 rounded-full">
                <ArrowLeftIcon className="w-8 h-8 inline" />
              </a>
            </Link>
            <h1 className="flex-grow my-auto text-4xl md:text-center font-bold">
              Preferences
            </h1>
            <div className="w-12 h-12 inline" />
          </header>
          <p className="mb-3">
            Attempt This Online stores your preferences locally in your browser, and they are never
            shared with anyone.
          </p>
          <fieldset className="border border-black dark:border-white rounded pt-2 pb-4 px-4">
            <legend className="px-2">Appearance</legend>
            <div className="flex">
              {/* eslint-disable-next-line jsx-a11y/label-has-associated-control */}
              <label htmlFor="themeSelector" className="my-auto mr-2">Theme:</label>
              <select
                id="themeSelector"
                className="appearance-none p-2 rounded bg-gray-100 hover:bg-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 transition cursor-pointer ATO_select focus:outline-none focus:ring"
                value={theme}
                onChange={handleThemeChange}
              >
                { systemThemePreference !== null && (
                <option value="system">
                  System default (
                  { systemThemePreference }
                  )
                </option>
                ) }
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </div>
          </fieldset>
        </main>
        <Footer />
      </div>
    </>
  );
}
