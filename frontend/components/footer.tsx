import Link from 'next/link';

// eslint-disable-next-line react/require-default-props
export default function Footer({ noLegalLink = false }: { noLegalLink?: boolean }) {
  return (
    <footer className="px-4 py-2 bg-black bg-opacity-5 dark:bg-opacity-20 w-100 flex flex-col text-center md:flex-row">
      <div className="order-2 md:order-1 md:flex md:flex-grow md:justify-center md:w-0">
        <a href="https://github.com/pxeger/attempt_this_online#licence" className="mr-auto">
          ©
          {' '}
          {new Date().getUTCFullYear()}
          {' '}
          Patrick Reader and contributors
        </a>
      </div>
      <div className="order-3 md:order-2 md:flex md:flex-grow md:justify-center md:w-0">
        { noLegalLink ? null : <Link href="/legal"><a className="underline text-blue-500">Legal</a></Link>}
      </div>
      <div className="order-1 md:order-3 md:flex md:flex-grow md:justify-center md:w-0">
        <div className="ml-auto md:max-w-qu truncate">
          <a href="https://github.com/pxeger/attempt_this_online">
            Version:
            {' '}
            <code>{buildId}</code>
          </a>
        </div>
      </div>
    </footer>
  );
}
