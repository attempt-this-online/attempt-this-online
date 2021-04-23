// copied from https://github.com/vercel/next.js/blob/5eafc1e92a5effa24bb6ac3c1d9e23065a16496b/examples/with-redux/store.js
import { useMemo } from 'react';
import { Store, createStore, applyMiddleware } from 'redux';
import { composeWithDevTools } from 'redux-devtools-extension';

let store: Store | undefined;

const initialState = {
  lastUpdate: 0,
  light: false,
  count: 0,
};

const reducer = (state = initialState, action: any) => {
  switch (action.type) {
    case 'setTheme':
      return {
        ...state,
        theme: action.theme!,
      };
    case 'setFontLigaturesEnabled':
      return {
        ...state,
        fontLigaturesEnabled: action.fontLigaturesEnabled!,
      }
    default:
      return state;
  }
};

function initStore(preloadedState = initialState) {
  return createStore(
    reducer,
    preloadedState,
    composeWithDevTools(applyMiddleware()),
  );
}

export const initializeStore = (preloadedState: typeof initialState) => {
  let newStore = store ?? initStore(preloadedState);

  // After navigating to a page with an initial Redux state, merge that state
  // with the current state in the store, and create a new store
  if (preloadedState && store) {
    newStore = initStore({
      ...store.getState(),
      ...preloadedState,
    });
    // Reset the current store
    store = undefined;
  }

  // For SSG and SSR always create a new store
  if (typeof window === 'undefined') {
    return newStore;
  }
  // Create the store once in the client
  if (!store) {
    store = newStore;
  }

  return newStore;
};

export function useStore(state: any) {
  return useMemo(() => initializeStore(state), [state]);
}
