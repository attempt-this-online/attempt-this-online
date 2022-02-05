import Link from 'next/link';

// eslint-disable-next-line react/require-default-props
export default function Footer({ noLegalLink = false }: { noLegalLink?: boolean }) {
  return (
    <footer className="px-4 py-2 bg-black bg-opacity-5 dark:bg-opacity-20 w-100 flex flex-col text-center md:flex-row">
      <div className="md:order-1 md:flex md:grow md:justify-center md:w-0">
        <a href="https://github.com/attempt-this-online/attempt-this-online#licence" className="mr-auto">
          ©
          {' '}
          {new Date().getUTCFullYear()}
          {' '}
          Patrick Reader and contributors
        </a>
      </div>
      <div className="md:order-2 md:flex md:justify-center md:w-0">
        { noLegalLink ? null : <Link href="/legal"><a className="underline text-blue-500">Legal</a></Link>}
      </div>
      <div className="md:order-3 md:flex md:grow md:justify-center md:w-0">
        <div className="ml-auto truncate">
          <a href="https://github.com/attempt-this-online/attempt-this-online">
            Version:
            {' '}
            <code>{buildId}</code>
          </a>
        </div>
      </div>
    </footer>
  );
}
