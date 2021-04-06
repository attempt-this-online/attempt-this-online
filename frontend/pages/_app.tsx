import 'tailwindcss/tailwind.css';
import PropTypes from 'prop-types';
import * as React from 'react';
import { Provider } from 'react-redux';

import { useStore } from 'lib/store';

function MyApp({ Component, pageProps }: { Component: React.ComponentType, pageProps: object }) {
  const store = useStore(pageProps.initialReduxState);
  return (
    <Provider store={store}>
      {/* eslint-disable-next-line react/jsx-props-no-spreading */}
      <Component {...pageProps} />
    </Provider>
  );
}
MyApp.propTypes = {
  Component: PropTypes.elementType.isRequired,
  // eslint-disable-next-line react/forbid-prop-types
  pageProps: PropTypes.object.isRequired,
};
export default MyApp;
