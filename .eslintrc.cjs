module.exports = {
  root: true,
  extends: ['eslint:recommended', 'airbnb', 'airbnb-typescript', 'plugin:@typescript-eslint/recommended', 'prettier'],
  plugins: ['prettier', '@typescript-eslint', 'react-hooks'],
  parser: '@typescript-eslint/parser',
  parserOptions: {
    project: 'tsconfig.json',
    tsconfigRootDir: __dirname,
    sourceType: 'module',
  },
  ignorePatterns: [
    'jest.config.ts',
    'jest.setup.ts',
    '.eslintrc.cjs',
    'vite.config.ts',
    'postcss.config.js',
    'tailwind.config.js',
    'backend/**',
    'pnpm-lock.yaml',
  ],
  settings: {
    'import/resolver': {
      node: {
        paths: ['src'],
      },
    },
  },
  rules: {
    'import/no-extraneous-dependencies': [
      'error',
      {
        packageDir: [__dirname],
      },
    ],
    '@typescript-eslint/naming-convention': 'off',
    '@typescript-eslint/no-empty-interface': 'off',
    'arrow-body-style': 'warn',
    'import/no-cycle': 'off',
    'import/prefer-default-export': 'off',
    'no-param-reassign': 'off',
    'react/button-has-type': 'off',
    'react/destructuring-assignment': 'off',
    'react/function-component-definition': 'off',
    'react/jsx-props-no-spreading': 'off',
    'react/require-default-props': 'off',
    '@typescript-eslint/no-loss-of-precision': 'off',
    'no-plusplus': 'off'
  },
}
