import { readFileSync } from 'node:fs';
import antfu from '@antfu/eslint-config';
import prettierConfig from 'eslint-config-prettier';

const prettierOptions = JSON.parse(
  readFileSync(new URL('./.prettierrc', import.meta.url), 'utf8'),
);

export default antfu(
  {
    type: 'app',
    vue: true,
    typescript: {
      tsconfigPath: './tsconfig.json',
    },
    stylistic: false,
    formatters: {
      css: true,
      html: true,
      prettierOptions,
    },
    ignores: [
      '**/out/**',
      '**/dist/**',
    ],
  },
  {
    files: ['**/*.ts'],
    ignores: ['**/*.d.ts'],
    rules: {
      'ts/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      'ts/no-explicit-any': 'error',
      'ts/consistent-type-imports': [
        'error',
        {
          prefer: 'type-imports',
          fixStyle: 'inline-type-imports',
        },
      ],
      'ts/no-floating-promises': 'error',
      'ts/no-misused-promises': 'error',
      'ts/await-thenable': 'error',
      'ts/no-require-imports': 'warn',
      'ts/no-import-type-side-effects': 'error',
      'ts/no-namespace': 'error',
      'no-console': [
        'warn',
        {
          allow: ['warn', 'error'],
        },
      ],
      'no-restricted-syntax': [
        'error',
        {
          selector: 'ImportNamespaceSpecifier',
        },
      ],
      'prefer-const': 'error',
      'no-var': 'error',
      'object-shorthand': ['error', 'always'],
      'prefer-template': 'error',
    },
  },
  {
    files: ['e2e/**/*.ts'],
    rules: {
      'ts/no-explicit-any': 'off',
      'ts/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
    },
  },
  {
    files: ['**/*.vue'],
    rules: {
      'vue/multi-word-component-names': 'off',
      'vue/no-v-html': 'warn',
      'vue/component-api-style': ['error', ['script-setup']],
      'vue/define-macros-order': [
        'error',
        {
          order: ['defineProps', 'defineEmits'],
        },
      ],
      'vue/block-order': [
        'error',
        {
          order: ['script', 'template', 'style'],
        },
      ],
      'vue/html-self-closing': [
        'error',
        {
          html: {
            void: 'always',
            normal: 'always',
            component: 'always',
          },
        },
      ],
      'vue/max-attributes-per-line': [
        'error',
        {
          singleline: 1,
          multiline: 1,
        },
      ],
      'vue/html-indent': ['error', 2],
      'vue/script-indent': [
        'error',
        2,
        {
          baseIndent: 1,
          switchCase: 1,
        },
      ],
      'vue/mustache-interpolation-spacing': ['error', 'always'],
      'vue/no-unused-refs': 'error',
      'vue/no-useless-mustaches': 'error',
      'vue/padding-line-between-blocks': ['error', 'always'],
      'ts/no-unsafe-assignment': 'off',
      'ts/no-unsafe-member-access': 'off',
      'ts/no-unsafe-call': 'off',
      'ts/no-unsafe-return': 'off',
      'ts/no-unsafe-argument': 'off',
      'ts/no-explicit-any': 'off',
    },
  },
  prettierConfig,
  {
    rules: {
      semi: ['error', 'always'],
    },
  },
);
