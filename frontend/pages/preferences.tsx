import { ArrowLeftIcon } from '@heroicons/react/outline';
import Head from 'next/head';
import Link from 'next/link';
import localForage from 'localforage';
import { useEffect, useState } from 'react';

import useSystemThemePreference from 'lib/useSystemThemePreference';
import { useStore } from 'lib/store';
import Footer from 'components/footer';

export default function Preferences() {
  const store = useStore();
  const [theme, setTheme] = useState('system');
  const [submitting, setSubmitting] = useState(false);
  const systemThemePreference = useSystemThemePreference();
  const handle = (setState) => (event) => setState(event.target.value);
  const onSubmit = async (event) => {
    event.preventDefault();
    setSubmitting(true);
    store.dispatch({ type: 'setTheme', theme });
    await localForage.setItem('ATO_theme', theme);
    setSubmitting(false);
  };
  useEffect(() => localForage.getItem('ATO_theme').then((v) => setTheme(v || 'system')), []);
  return (
    <>
      <Head>
        <title>Prefences &ndash; Attempt This Online</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white pt-8 relative flex flex-col">
        <main className="mb-3 mx-4 md:container md:mx-auto flex-grow">
          <header className="flex">
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
          <form onSubmit={onSubmit}>
            <select className="text-black block" value={theme} onChange={(e) => setTheme(e.target.value)}>
              { systemThemePreference !== null && (
              <option value="system">
                System Default (
                { systemThemePreference }
                )
              </option>
              ) }
              <option value="dark">Dark</option>
              <option value="light">Light</option>
            </select>
            <button type="submit" className="block rounded px-4 py-2 bg-blue-500 text-white">Save</button>
          </form>
        </main>
        <Footer />
      </div>
    </>
  );
}
