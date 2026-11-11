import js from '@eslint/js'
import tseslint from 'typescript-eslint'
import react from 'eslint-plugin-react'
import reactHooks from 'eslint-plugin-react-hooks'
import jsxA11y from 'eslint-plugin-jsx-a11y'
import importPlugin from 'eslint-plugin-import'
import prettier from 'eslint-config-prettier'
import globals from 'globals'

export default tseslint.config(
  {
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/coverage/**',
      'engine/**',
      'context/**',
      'public/**',
      'eslint.config.mjs',
      'vite.config.mts',
      'vitest.setup.ts',
      'postcss.config.js',
      'tailwind.config.js',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  react.configs.flat.recommended,
  react.configs.flat['jsx-runtime'],
  jsxA11y.flatConfigs.recommended,
  importPlugin.flatConfigs.recommended,
  importPlugin.flatConfigs.typescript,
  {
    // disable comments for airbnb rules that are no longer on are left alone
    linterOptions: { reportUnusedDisableDirectives: 'off' },
  },
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: {
      parserOptions: {
        project: './tsconfig.json',
        tsconfigRootDir: import.meta.dirname,
      },
      globals: { ...globals.browser },
    },
    plugins: {
      'react-hooks': reactHooks,
    },
    settings: {
      react: { version: 'detect' },
      'import/resolver': {
        typescript: { project: './tsconfig.json' },
        node: true,
      },
    },
    rules: {
      // the hooks rules (rules-of-hooks, exhaustive-deps), without the
      // React Compiler rules the plugin's newer presets add
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',

      // carried over from the airbnb config this replaces
      'import/no-extraneous-dependencies': ['error', { packageDir: [import.meta.dirname] }],
      'import/order': ['error', { groups: [['builtin', 'external', 'internal']] }],
      'import/first': 'error',
      'import/no-duplicates': 'error',
      'import/newline-after-import': 'error',
      'no-restricted-syntax': [
        'error',
        { selector: 'ForInStatement', message: 'for..in loops iterate over the prototype chain; use Object.keys/entries' },
        { selector: 'LabeledStatement', message: 'labels are a form of goto' },
        { selector: 'WithStatement', message: '`with` is disallowed' },
      ],
      'no-var': 'error',
      'prefer-const': 'error',
      eqeqeq: ['error', 'always', { null: 'ignore' }],
      'no-console': 'warn',
      'arrow-body-style': 'warn',
      '@typescript-eslint/no-unused-vars': ['warn', { ignoreRestSiblings: true, caughtErrors: 'none' }],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-empty-object-type': 'off',
      'react/prop-types': 'off',
      // files import React though the jsx runtime doesn't need it; count it as used
      'react/jsx-uses-react': 'error',
    },
  },
  prettier,
)
