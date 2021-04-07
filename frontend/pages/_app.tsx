import 'tailwindcss/tailwind.css';
import PropTypes from 'prop-types';
import * as React from 'react';
import { Provider, connect } from 'react-redux';

import useSystemThemePreference from 'lib/useSystemThemePreference';
import { useStore } from 'lib/store';

const ThemeWrapper = connect((state) => ({ theme: state.theme }))(({ Component, pageProps, theme }: { Component: React.ComponentType, theme: ('light' | 'dark' | 'system'), pageProps: object }) => {
  const systemThemePreference = useSystemThemePreference();
  return (
    <div
      className={
        (theme === 'light' || (theme === 'system' && systemThemePreference === 'light'))
          ? null : 'dark'
    }
    >
      {/* eslint-disable-next-line react/jsx-props-no-spreading */}
      <Component {...pageProps} />
    </div>
  );
});

function MyApp({ Component, pageProps }: { Component: React.ComponentType, pageProps: object }) {
  const store = useStore(pageProps.initialReduxState);
  return (
    <Provider store={store}>
      <ThemeWrapper Component={Component} pageProps={pageProps} />
    </Provider>
  );
}

MyApp.propTypes = {
  Component: PropTypes.elementType.isRequired,
  // eslint-disable-next-line react/forbid-prop-types
  pageProps: PropTypes.object.isRequired,
};

export default MyApp;
