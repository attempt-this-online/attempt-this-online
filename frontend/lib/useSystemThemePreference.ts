// Inspired by https://github.com/neo4j/neo4j-browser/blob/6b1a9d96755383e0e7b1f209e1c10f73c3b198e5/src/browser/hooks/useDetectColorScheme.ts
import { useState, useEffect } from 'react';

export default function useSystemThemePreference() {
  const [systemThemePreference, setSystemThemePreference] = useState(null);

  useEffect(() => {
    if (!window.matchMedia) {
      // no browser support
      return null;
    }

    const darkListener = event => event && event.matches && setSystemThemePreference('dark');
    const darkQuery = window.matchMedia('(prefers-color-scheme: dark)');
    darkQuery.addListener(darkListener);
    darkListener(darkQuery);
    const lightListener = event => event && event.matches && setSystemThemePreference('light');
    const lightQuery = window.matchMedia('(prefers-color-scheme: light)');
    lightQuery.addListener(lightListener);
    lightListener(lightQuery);

    // cleanup
    return () => {
      darkQuery.removeListener(darkListener);
      lightQuery.removeListener(lightListener);
    };
  }, []);
  return systemThemePreference;
}
