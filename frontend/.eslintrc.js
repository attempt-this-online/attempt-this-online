module.exports = {
  root: true,
  "extends": "next/core-web-vitals",
  rules: {
    // https://github.com/vercel/next.js/issues/5533
    "jsx-a11y/anchor-is-valid": "off",
    "no-console": ["error", { allow: ["warn", "error"] }],
    // if you don't use these rules you're dangerous and must be locked up
    "comma-dangle": ["error", "always-multiline"],
    curly: ["error", "all"],
    "space-before-function-paren": ["error", {
      anonymous: "always",
      named: "never",
      asyncArrow: "always",
    }],
    "react/react-in-jsx-scope": "off",
    "arrow-parens": ["error", "as-needed"],
    "react/no-unescaped-entities": "off",
  },
};
