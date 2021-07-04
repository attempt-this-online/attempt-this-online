import { XIcon } from '@heroicons/react/solid';

export default function Notification(
  { children, onClick }: { children: React.ReactNode, onClick: () => void },
) {
  return (
    <div className="rounded p-3 bg-gray-200 dark:bg-gray-700 shadow-md mx-auto mb-4 flex max-w-full w-80 pointer-events-auto">
      <div className="flex-grow">
        {children}
      </div>
      <button type="button" className="transition hover:bg-gray-300 dark:hover:bg-gray-600 rounded-full focus:outline-none px-1" onClick={onClick}>
        <XIcon className="h-4 w-4 my-auto" />
      </button>
    </div>
  );
}
