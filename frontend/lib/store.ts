// copied from https://github.com/vercel/next.js/blob/5eafc1e92a5effa24bb6ac3c1d9e23065a16496b/examples/with-redux/store.js
import { useMemo } from 'react';
import { Store, createStore, applyMiddleware } from 'redux';
import { composeWithDevTools } from 'redux-devtools-extension';
import localForage from 'localforage';

let store: Store | undefined;

const initialState = {
  // TODO(pxeger): these are part of the Redux example, but are they necessary for something?
  lastUpdate: 0,
  light: false,
  count: 0,

  fullWidthMode: false,
  theme: 'system',
  fontLigaturesEnabled: true,
  bigTextBoxes: true,
  tabBehaviour: 'insert',
};

const reducer = (state = initialState, action: any) => {
  switch (action.type) {
    case 'setTheme':
      localForage.setItem('ATO_theme', action.theme!);
      return {
        ...state,
        theme: action.theme!,
      };
    case 'setFontLigaturesEnabled':
      localForage.setItem('ATO_font_ligatures', action.fontLigaturesEnabled!);
      return {
        ...state,
        fontLigaturesEnabled: action.fontLigaturesEnabled!,
      };
    case 'setLanguagesMetadata':
      return {
        ...state,
        metadata: action.metadata!,
      };
    case 'setFullWidthMode':
      localForage.setItem('ATO_full_width_mode', action.fullWidthMode!);
      return {
        ...state,
        fullWidthMode: action.fullWidthMode!,
      };
    case 'setBigTextBoxes':
      localForage.setItem('ATO_big_text_boxes', action.bigTextBoxes!);
      return {
        ...state,
        bigTextBoxes: action.bigTextBoxes!,
      };
    case 'setTabBehaviour':
      localForage.setItem('ATO_tab_behaviour', action.tabBehaviour!);
      return {
        ...state,
        tabBehaviour: action.tabBehaviour!,
      };
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
