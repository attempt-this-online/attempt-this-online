import PropTypes from 'prop-types';
import * as React from 'react';
import 'tailwindcss/tailwind.css';

function MyApp({ Component, pageProps }: { Component: React.ComponentType, pageProps: object }) {
  // eslint-disable-next-line react/jsx-props-no-spreading
  return <Component {...pageProps} />;
}
MyApp.propTypes = {
  Component: PropTypes.elementType,
  pageProps: PropTypes.object,
};
export default MyApp;
