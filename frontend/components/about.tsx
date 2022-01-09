import localforage from 'localforage';
import Head from 'next/head';
import Link from 'next/link';
import { useRouter } from 'next/router';
import { useEffect } from 'react';

import Footer from 'components/footer';

export default function About({ enableRedirect }: { enableRedirect: boolean }) {
  const router = useRouter();
  useEffect(() => {
    localforage.getItem('ATO_greeted').then(greeted => {
      if (!greeted) {
        localforage.setItem('ATO_greeted', true);
      }
      // if already used the site, and not explicitly on /about, send them to the run page
      if (enableRedirect && greeted) {
        router.replace('/run');
      }
    });
  }, []);
  return (
    <>
      <Head>
        <title>Attempt This Online</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white pt-8 relative flex flex-col">
        <main className="mb-3 px-4 md:container md:mx-auto flex-grow">
          <h1 className="mb-3 -mt-3 md:pt-3 text-4xl md:text-center font-bold">Attempt This Online</h1>
          <p className="my-4 text-justify">
            Attempt This Online is an online sandbox environment for running code in an ever-growing
            list of programming languages, both practical and recreational. ATO was originally
            conceived as a replacement for the increasingly out-of-date
            {' '}
            <a href="https://tio.run" className="text-blue-500 underline">Try It Online</a>
            {' '}
            service.
          </p>
          <p className="my-4 text-justify">
            To get started, click the button below to run some code.
          </p>
          <p className="my-4 text-center sm:text-left">
            <Link href="/run">
              <a>
                <button type="button" className="rounded px-4 py-2 bg-blue-500 text-white focus:outline-none focus:ring transition">
                  Run
                </button>
              </a>
            </Link>
          </p>
          <p className="my-4 text-justify">
            By using this website, you agree to be bound by our
            {' '}
            <Link href="/legal">
              <a className="underline text-blue-500">
                Privacy Policy and Terms of Use
              </a>
            </Link>
            .
          </p>
          <h2 className="mt-6 text-2xl font-bold">Why ATO?</h2>
          <ul className="list-disc ml-6 my-4">
            <li className="text-justify">It&apos;s completely free of charge</li>
            <li className="text-justify">
              The software is
              {' '}
              <a href="https://github.com/attempt-this-online/attempt-this-online" className="text-blue-500 underline">open-source</a>
              {' '}
              (available under the copyleft
              {' '}
              <a href="https://github.com/attempt-this-online/attempt-this-online/blob/main/LICENCE.txt" className="text-blue-500 underline">GNU Affero General Public License 3.0</a>
              )
            </li>
            <li className="text-justify">We don&apos;t advertise or use any tracking technologies</li>
            <li className="text-justify">Regularly maintained: new languages and features are added by request all the time</li>
            <li className="text-justify">
              The interface is customisable (see the
              {' '}
              <Link href="/preferences">
                <a className="underline text-blue-500">Preferences</a>
              </Link>
              {' '}
              page)
            </li>
          </ul>
          <h2 className="mt-6 text-2xl font-bold">Give Feedback</h2>
          <p className="my-4 text-justify">
            If you have a feature suggestion, bug report, or request for a new or updated language,
            open an issue in the
            {' '}
            <a href="https://github.com/attempt-this-online/attempt-this-online/issues/new/choose" className="text-blue-500 underline">GitHub repository</a>
            .
            Feel free to implement it yourself and
            {' '}
            <a href="https://github.com/attempt-this-online/attempt-this-online/compare" className="text-blue-500 underline">submit a pull request</a>
            !
          </p>
          <p className="my-4 text-justify">
            You can also discuss ATO in the dedicated
            {' '}
            <a href="https://chat.stackexchange.com/rooms/122645/attempt-this-online" className="text-blue-500 underline">Stack Exchange chatroom</a>
            .
          </p>
        </main>
        <Footer />
      </div>
    </>
  );
}
