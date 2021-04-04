module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  parserOptions: { project: './tsconfig.json' },
  plugins: [
    '@typescript-eslint',
  ],
  extends: [
    'airbnb-typescript',
  ],
  rules: {
    // https://github.com/vercel/next.js/issues/5533
    'jsx-a11y/anchor-is-valid': 'off',
    // if you don't use these rules you're dangerous and must be locked up
    'comma-dangle': ['error', 'always-multiline'],
    curly: ['error', 'all'],
    'space-before-function-paren': ['error', {anonymous: 'always', named: 'never', asyncArrow: 'always'}],
    'react/react-in-jsx-scope': 'off',

  }
}
