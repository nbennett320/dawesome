/** @type {import('ts-jest/dist/types').InitialOptionsTsJest} */
// eslint-disable-next-line import/no-anonymous-default-export
export default {
  preset: 'ts-jest/presets/default-esm',
  testEnvironment: 'jest-environment-jsdom',
  setupFilesAfterEnv: ['./jest.setup.ts'],
  roots: ['<rootDir>/src'],
  collectCoverageFrom: ['src/**/*.{js,jsx,ts,tsx}', '!src/**/*.d.ts'],
  transform: {
    '^.+\\.tsx?$': 'ts-jest',
  },
  testMatch: ['<rootDir>/src/**/__tests__/**/*.{js,jsx,ts,tsx}', '<rootDir>/src/**/*.{spec,test}.{js,jsx,ts,tsx}'],
  extensionsToTreatAsEsm: ['.ts', '.tsx', '.jsx'],
  globals: {
    'ts-jest': {
      isolatedModules: true,
      babelConfig: true,
    },
  },
  modulePaths: [],
  moduleNameMapper: {
    '\\.(css|less|scss|sass)$': 'jest-transform-css',
  },
  moduleFileExtensions: ['js', 'jsx', 'ts', 'tsx', 'yaml', 'yml'],
  resetMocks: true,
}
