// Inspired by https://github.com/neo4j/neo4j-browser/blob/6b1a9d96755383e0e7b1f209e1c10f73c3b198e5/src/browser/hooks/useDetectColorScheme.ts
import { useState, useEffect } from 'react';

export default function useSystemThemePreference() {
  const [systemThemePreference, setSystemThemePreference] = useState<'dark' | 'light' | null>(null);

  useEffect(() => {
    if (!window.matchMedia) {
      // no browser support
      setSystemThemePreference(null);
      return;
    }

    const darkListener = (event: any) => event && event.matches && setSystemThemePreference('dark');
    const darkQuery = window.matchMedia('(prefers-color-scheme: dark)');
    darkQuery.addListener(darkListener);
    darkListener(darkQuery);
    const lightListener = (event: any) => event && event.matches && setSystemThemePreference('light');
    const lightQuery = window.matchMedia('(prefers-color-scheme: light)');
    lightQuery.addListener(lightListener);
    lightListener(lightQuery);

    // cleanup
    // eslint-disable-next-line consistent-return
    return () => {
      darkQuery.removeListener(darkListener);
      lightQuery.removeListener(lightListener);
    };
  }, []);
  return systemThemePreference;
}
