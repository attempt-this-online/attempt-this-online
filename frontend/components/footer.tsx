export default function Footer() {
  return (
    <footer className="px-4 py-2 bg-black bg-opacity-5 dark:bg-opacity-20 absolute bottom-0 left-0 right-0 flex justify-between">
      <span>
        ©
        {new Date().getUTCFullYear()}
        {' '}
        Patrick Reader and contributors
      </span>
      <a href="https://github.com/pxeger/attempt_this_online">{buildId}</a>
    </footer>
  );
}
