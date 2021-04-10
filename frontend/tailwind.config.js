const defaultTheme = require('tailwindcss/defaultTheme');

module.exports = {
  purge: ['./pages/**/*.{js,ts,jsx,tsx}', './components/**/*.{js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        mono: ['Fira Code', ...defaultTheme.fontFamily.sans],
      },
      maxWidth: {
        qu: '25vw',
      },
      minHeight: {
        6: '6rem',
      },
    },
  },
  variants: {
    extend: {
      outline: ['dark'],
    },
  },
  plugins: [],
};
