import 'tailwindcss/tailwind.css';
import localForage from 'localforage';
import PropTypes from 'prop-types';
import * as React from 'react';
import { Provider, connect, useDispatch } from 'react-redux';

import useSystemThemePreference from 'lib/useSystemThemePreference';
import { useStore } from 'lib/store';
import 'styles/ATO.css';

const ThemeWrapper = connect((state: any) => ({ theme: state.theme }))(({ Component, pageProps, theme }: { Component: React.ComponentType, theme: ('light' | 'dark' | 'system'), pageProps: any }) => {
  const dispatch = useDispatch();
  React.useEffect((async () => {
    const storedTheme = await localForage.getItem('ATO_theme');
    if (storedTheme) {
      dispatch({ type: 'setTheme', theme: storedTheme });
    }
  }) as (() => void), []);
  const systemThemePreference = useSystemThemePreference();
  return (
    <div
      className={
        (theme === 'light' || (theme === 'system' && systemThemePreference === 'light'))
          ? undefined : 'dark'
    }
    >
      {/* eslint-disable-next-line react/jsx-props-no-spreading */}
      <Component {...pageProps} />
    </div>
  );
});

function MyApp({ Component, pageProps }: { Component: React.ComponentType, pageProps: any }) {
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
