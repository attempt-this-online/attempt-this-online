import Link from 'next/link';
import { AdjustmentsIcon, HomeIcon } from '@heroicons/react/outline';

export default function Navbar() {
  return (
    <nav className="flex bg-gray-100 dark:bg-gray-800 w-full px-4 py-2 mb-4 justify-between">
      <Link href="/">
        <a className="p-2 transition hover:bg-gray-300 dark:hover:bg-gray-600 rounded-full focus:ring">
          <HomeIcon className="w-6 h-6" />
        </a>
      </Link>
      <h2 className="hidden sm:block font-bold text-xl my-auto">Attempt This Online</h2>
      <Link href="/preferences">
        <a className="p-2 transition hover:bg-gray-300 dark:hover:bg-gray-600 rounded-full focus:ring">
          <AdjustmentsIcon className="w-6 h-6" />
        </a>
      </Link>
    </nav>
  );
}
